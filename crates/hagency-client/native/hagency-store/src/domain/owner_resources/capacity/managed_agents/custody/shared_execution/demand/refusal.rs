//! A start of a message that the server refused because the Agent has an
//! applied budget (schema 116). The server refuses every start of this lane
//! once the budget is reported, and again every time, since an applied budget
//! stays applied; it looks for a start of the message that it made before it
//! reads the budget, so the refusal also says that it made none. Such a start
//! gives its place back at once and is never sent again, and the budget lane
//! takes up a message whose first start it was.
use super::*;

/// The server's code for a start of this lane of an Agent with an applied
/// budget. It is the only refusal that gives a place back before the
/// message ends: the server's other refusals of a start can change from one
/// try to the next, or name a start it made.
pub const SHARED_BUDGET_EXECUTION_REQUIRED: &str = "shared_budget_execution_required";

/// Whether the server refused this start for the Agent's budget.
pub(in super::super) fn refused(db: &Connection, start_intent_id: &str) -> Result<bool, Error> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_start_refusals WHERE start_intent_id=?1)",
        [start_intent_id],
        |r| r.get(0),
    )?)
}

#[cfg(feature = "shared-budget-execution")]
impl OwnerResourceRegistry {
    /// Keep the server's refusal of `input`, the first start or a retry of
    /// the message `proof` names, for the Agent's applied budget, and give
    /// the start's place back. Only the exact request saved here, for an
    /// Agent this computer has applied a budget to, with no command issued
    /// here for it; the same refusal read again changes nothing.
    pub async fn release_refused_shared_demand_start(
        &self,
        proof: VerifiedSharedDemand,
        original: SharedRuntimeCommand,
        input: SharedDemandStartInput,
        code: String,
    ) -> Result<(), Error> {
        if code != SHARED_BUDGET_EXECUTION_REQUIRED {
            return Err(Error::LocalAuthority);
        }
        proof.check()?;
        original.validate(&proof.enrollment.profile.account)?;
        input.validate_for(&proof.offer)?;
        let p = proof.enrollment.profile.clone();
        let expires = proof.expires;
        let registry = self.0.clone();
        self.0.store.call(64 * 1024, move |db| {
            proof.check()?;
            let p = &proof.enrollment.profile;
            after(p, &registry, expires)?;
            let tx = db.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let e = &proof.enrollment;
            let child = &original.agent_reservation_id;
            let b = authority::original(&tx, e, child, false)?;
            if b.digest != original.reserve_command_digest
                || proof.offer.assignment_id != original.assignment_id
                || original.reserve_command.executor_device_id != e.executor_device_id
                || original.reserve_command.executor_generation != e.executor_generation
            {
                return Err(Error::LocalAuthority);
            }
            // The server reads the budget this computer applied and reported.
            if !super::super::super::budget::execution::applied(&tx, child)? {
                return Err(Error::LocalAuthority);
            }
            let saved = journal::load(&tx, &p.digest, child, &proof.offer.demand_id)?
                .ok_or(Error::LocalAuthority)?;
            if saved.offer.digest()? != proof.offer.digest()? {
                return Err(Error::Conflict);
            }
            let n = input.retry.unwrap_or(0);
            let start = retry::saved_start(&tx, &p.digest, child, &saved, n)?;
            let sent = canonical::digest(&serde_json::to_value(&input)?)?;
            // The first start's whole request is its saved decision; a
            // retry's request is built around its saved start.
            let same = match (&saved.input, n) {
                (Some(first), 0) => canonical::digest(&serde_json::to_value(first)?)? == sent,
                _ => canonical::digest(&serde_json::to_value(&start)?)?
                    == canonical::digest(&serde_json::to_value(&input.start)?)?,
            };
            if !same {
                return Err(Error::Conflict);
            }
            let id = &start.start_intent_id;
            let old: Option<(String, String)> = tx.query_row(
                "SELECT demand_id,input_digest FROM owner_shared_start_refusals WHERE start_intent_id=?1",
                [id], |r| Ok((r.get(0)?, r.get(1)?)),
            ).optional()?;
            match old {
                Some((demand, digest)) => {
                    if demand != proof.offer.demand_id || digest != sent {
                        return Err(Error::Conflict);
                    }
                }
                None => {
                    // A command issued here for it says that the server made it.
                    let issued: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM owner_shared_successor_attempts WHERE start_intent_id=?1) OR EXISTS(SELECT 1 FROM owner_shared_successor_fences WHERE start_intent_id=?1)",
                        [id], |r| r.get(0),
                    )?;
                    if issued {
                        return Err(Error::Conflict);
                    }
                    let revision = revision(&tx)?;
                    tx.execute(
                        "INSERT INTO owner_shared_start_refusals VALUES(?1,?2,?3,?4,?5)",
                        params![id, proof.offer.demand_id, code, sent, revision],
                    )?;
                }
            }
            // Its message may have ended and given the place back already.
            tx.execute(
                "INSERT INTO owner_shared_start_releases(start_intent_id,demand_id) SELECT ?1,?2 WHERE NOT EXISTS(SELECT 1 FROM owner_shared_start_releases WHERE start_intent_id=?1)",
                params![id, proof.offer.demand_id],
            )?;
            proof.check()?;
            tx.commit()?;
            proof.check().map_err(|_| Error::OutcomeUnknown)
        }).await?;
        after(&p, &self.0, expires)
    }
}
