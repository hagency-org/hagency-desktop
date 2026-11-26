//! ADR-193 amendment (retry v2): how a claimed demand attempt's stop reaches
//! the server, decided once beside its stop. A stop that provably never began
//! its demand's work hands that work back: in the same transaction the binding
//! moves into a retained release row, so the demand waits again in its FIFO
//! place, and the stop is then sent as the demand's next retry request instead
//! of the plain result upload. Nothing here starts a process, a call or a turn.
use super::*;

/// How a claimed attempt's stop reaches the server.
#[derive(Clone, Debug)]
pub enum SharedAttemptStopRoute {
    /// The plain attempt result upload.
    Plain,
    /// Retry `retry` of `offer`'s event, carrying the stop receipt.
    Retry { offer: SharedDemandOffer, retry: u64 },
}

/// Work that is provably unbegun: no start, call, tool or reply event, no work
/// observation, no reply delivery and no call row of any state. Migration 116
/// checks the same facts before a binding may be released.
fn unbegun(db: &Connection, work_id: &str) -> Result<bool, Error> {
    Ok(db.query_row(
        "SELECT NOT EXISTS(SELECT 1 FROM owner_shared_work_events WHERE work_id=?1) AND NOT EXISTS(SELECT 1 FROM owner_shared_work_observations WHERE work_id=?1) AND NOT EXISTS(SELECT 1 FROM owner_shared_reply_deliveries WHERE work_id=?1) AND NOT EXISTS(SELECT 1 FROM owner_shared_successor_call_pins WHERE work_id=?1) AND NOT EXISTS(SELECT 1 FROM owner_managed_calls c JOIN owner_shared_work_inbox w ON w.execution_id=c.execution_id AND w.child_id=c.child_id WHERE w.id=?1)",
        [work_id],
        |r| r.get(0),
    )?)
}

/// Work its stopped attempt handed back is never bound to its demand again.
pub(super) fn handed_back(
    db: &Connection,
    binding: &SharedDemandBinding,
) -> Result<bool, Error> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_demand_released_bindings WHERE work_id=?1) OR EXISTS(SELECT 1 FROM owner_shared_stop_routes WHERE attempt_id=?2 AND demand_id=?3 AND route='retry')",
        params![binding.work_id, binding.runtime_command_id, binding.demand_id],
        |r| r.get(0),
    )?)
}

/// The event's starts opt in to the server giving its message back: its
/// original start was saved with `reoffers` (its retries share it) and no
/// start of it went out without (`lapse`). Only such a start's work is ever
/// handed back here: the server gives back no other.
pub(super) fn reoffering(db: &Connection, saved: &SharedDemandIntent) -> Result<bool, Error> {
    if !saved.input.as_ref().is_some_and(|i| i.reoffers) {
        return Ok(false);
    }
    Ok(!db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_demand_reoffer_lapses WHERE demand_id=?1)",
        [&saved.offer.demand_id],
        |r| r.get::<_, bool>(0),
    )?)
}

/// Saved before a start of `demand_id` goes out without `reoffers` although
/// its event opted in (the server no longer lists retry v2): from then on
/// none of the event's work is handed back here.
pub(super) fn lapse(tx: &Connection, demand_id: &str) -> Result<(), Error> {
    let saved: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_demand_reoffer_lapses WHERE demand_id=?1)",
        [demand_id],
        |r| r.get(0),
    )?;
    if saved {
        return Ok(());
    }
    bounded_row(tx, "owner_shared_demand_reoffer_lapses", "demand_id", demand_id, 100_000)?;
    let rev = revision(tx)?;
    tx.execute(
        "INSERT INTO owner_shared_demand_reoffer_lapses VALUES(?1,?2)",
        params![demand_id, rev],
    )?;
    Ok(())
}

/// The decided route of `attempt_id`, if any.
fn saved(
    db: &Connection,
    profile: &str,
    child: &str,
    attempt_id: &str,
) -> Result<Option<SharedAttemptStopRoute>, Error> {
    let row: Option<(String, String, String, Option<String>, Option<u64>)> = db
        .query_row(
            "SELECT child_id,profile,route,demand_id,retry FROM owner_shared_stop_routes WHERE attempt_id=?1",
            [attempt_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((saved_child, saved_profile, route, demand, retry)) = row else {
        return Ok(None);
    };
    if saved_child != child || saved_profile != profile {
        return Err(Error::Conflict);
    }
    match (route.as_str(), demand, retry) {
        ("plain", None, None) => Ok(Some(SharedAttemptStopRoute::Plain)),
        ("retry", Some(id), Some(n)) => {
            let saved = journal::load(db, profile, child, &id)?.ok_or(Error::Conflict)?;
            Ok(Some(SharedAttemptStopRoute::Retry {
                offer: saved.offer,
                retry: n,
            }))
        }
        _ => Err(Error::Conflict),
    }
}

/// A binding row, copied exactly.
type BindingRow = (String, String, String, String, u64);

/// The demand this stopped attempt was started for (`id`, by its original
/// start or its retry `k`), the retry its stop asks for and the binding it
/// hands back, when the stop may ask for a retry.
fn retryable(
    db: &Connection,
    profile: &str,
    command: &SharedAttemptCommand,
    digest: &str,
    (id, k): (String, u64),
) -> Result<Option<(SharedDemandOffer, u64, Option<BindingRow>)>, Error> {
    let child = &command.agent_reservation_id;
    let n = k + 1;
    if n > MAX_DEMAND_RETRIES + 1 {
        return Ok(None);
    }
    let Some(saved) = journal::load(db, profile, child, &id)? else {
        return Ok(None);
    };
    // The server gives back only the work of a start that opted in.
    if !reoffering(db, &saved)? {
        return Ok(None);
    }
    // The demand's own start under its current Run revision: a Stop or a new
    // Run revision leaves the event to the plain route.
    match guard::check_attempt_control(db, profile, command) {
        Ok(()) => {}
        Err(Error::LocalAuthority) => return Ok(None),
        Err(e) => return Err(e),
    }
    let terminal: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_budget_demand_terminals WHERE demand_id=?1)",
        [&id],
        |r| r.get(0),
    )?;
    // No other start holds this attempt's place as predecessor.
    let competing: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_start_intents WHERE child_id=?1 AND predecessor_digest=?2) OR EXISTS(SELECT 1 FROM owner_shared_budget_start_intents WHERE child_id=?1 AND predecessor_digest=?2)",
        params![child, digest],
        |r| r.get(0),
    )?;
    if terminal || competing {
        return Ok(None);
    }
    // The demand's local work: none, or this attempt's own unbegun work.
    let binding: Option<BindingRow> = db
        .query_row(
            "SELECT demand_id,work_id,binding_payload,binding_digest,ledger_revision FROM owner_shared_demand_bindings WHERE demand_id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    if let Some((_, work, payload, _, _)) = &binding {
        let b: SharedDemandBinding = serde_json::from_str(payload)?;
        if b.runtime_command_id != command.command_id
            || b.runtime_command_digest != digest
            || &b.work_id != work
            || !unbegun(db, work)?
        {
            return Ok(None);
        }
    }
    Ok(Some((saved.offer, n, binding)))
}

/// Decide once how `command`'s stop reaches the server, beside its stopped
/// observation `observation_id`; a saved route is returned unchanged. Only
/// what is saved here decides: the server gives back exactly the work of a
/// start it stored with `reoffers`, so nothing is asked of it. An attempt
/// that was not started for a demand has nothing to decide and saves nothing.
pub(in super::super) fn decide(
    tx: &Connection,
    profile: &str,
    command: &SharedAttemptCommand,
    observation_id: &str,
) -> Result<SharedAttemptStopRoute, Error> {
    let child = &command.agent_reservation_id;
    if let Some(route) = saved(tx, profile, child, &command.command_id)? {
        return Ok(route);
    }
    // Started for a demand: its original start or one of its retries.
    let found = if cfg!(feature = "shared-intake") {
        retry::demand_for_start(tx, &command.start_intent_id)?
    } else {
        None
    };
    let Some(found) = found else {
        return Ok(SharedAttemptStopRoute::Plain);
    };
    let digest = canonical::digest(&serde_json::to_value(command)?)?;
    let eligible = retryable(tx, profile, command, &digest, found)?;
    bounded_row(tx, "owner_shared_stop_routes", "attempt_id", &command.command_id, 100_000)?;
    let Some((offer, n, binding)) = eligible else {
        let rev = revision(tx)?;
        tx.execute(
            "INSERT INTO owner_shared_stop_routes VALUES(?1,?2,?3,?4,'plain',NULL,NULL,NULL,?5)",
            params![command.command_id, child, profile, observation_id, rev],
        )?;
        return Ok(SharedAttemptStopRoute::Plain);
    };
    let released = if let Some((demand, work, payload, binding_digest, ledger)) = binding {
        bounded_row(tx, "owner_shared_demand_released_bindings", "work_id", &work, 100_000)?;
        let rev = revision(tx)?;
        tx.execute(
            "INSERT INTO owner_shared_demand_released_bindings VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![work, demand, payload, binding_digest, ledger, rev, graphs::now_ms()?],
        )?;
        tx.execute(
            "DELETE FROM owner_shared_demand_bindings WHERE demand_id=?1 AND work_id=?2",
            params![demand, work],
        )?;
        Some(work)
    } else {
        None
    };
    let rev = revision(tx)?;
    tx.execute(
        "INSERT INTO owner_shared_stop_routes VALUES(?1,?2,?3,?4,'retry',?5,?6,?7,?8)",
        params![command.command_id, child, profile, observation_id, offer.demand_id, n, released, rev],
    )?;
    Ok(SharedAttemptStopRoute::Retry { offer, retry: n })
}

impl OwnerResourceRegistry {
    /// The stop route of a claimed attempt, as it was decided beside its
    /// stop. A stop saved without one (by an earlier build, or when deciding
    /// failed there) is decided now, once. `None` for an attempt without a
    /// stop on this installation. One store call: a caller that sends the
    /// stop again repeats only its exchange, never this.
    pub async fn shared_attempt_stop_route(
        &self,
        authority: ManagedCleanupAuthority,
        command: SharedAttemptCommand,
    ) -> Result<Option<SharedAttemptStopRoute>, Error> {
        authority.check(&self.0)?;
        command.validate(&authority.enrollment.profile.account)?;
        if authority.child != command.agent_reservation_id {
            return Err(Error::LocalAuthority);
        }
        let profile = authority.enrollment.profile.clone();
        let expires = authority.deadline();
        let registry = self.0.clone();
        let result = self.0.store.call(32 * 1024, move |db| {
            authority.check(&registry)?;
            let tx = db.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            authority::cleanup(&tx, &authority)?;
            let p = authority.enrollment.profile.digest.clone();
            let digest = canonical::digest(&serde_json::to_value(&command)?)?;
            let claim: Option<(String, String)> = tx
                .query_row(
                    "SELECT profile,command_digest FROM owner_shared_successor_attempts WHERE id=?1 AND child_id=?2",
                    params![command.command_id, authority.child],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let Some((saved_profile, saved_digest)) = claim else {
                authority.check(&registry)?;
                return Ok(None);
            };
            if saved_profile != p || saved_digest != digest {
                return Err(Error::Conflict);
            }
            if let Some(route) = saved(&tx, &p, &authority.child, &command.command_id)? {
                authority.check(&registry)?;
                return Ok(Some(route));
            }
            let stop: Option<String> = tx
                .query_row(
                    "SELECT id FROM owner_shared_successor_observations WHERE attempt_id=?1 AND json_extract(payload,'$.state')='stopped' ORDER BY ledger_revision LIMIT 1",
                    [&command.command_id],
                    |r| r.get(0),
                )
                .optional()?;
            let Some(stop) = stop else {
                authority.check(&registry)?;
                return Ok(None);
            };
            let route = decide(&tx, &p, &command, &stop)?;
            authority.check(&registry)?;
            tx.commit()?;
            authority.check(&registry).map_err(|_| Error::OutcomeUnknown)?;
            Ok(Some(route))
        }).await?;
        after(&profile, &self.0, expires)?;
        Ok(result)
    }
}

#[cfg(all(test, feature = "shared-intake"))]
impl OwnerResourceRegistry {
    /// Test view of the binding gate that every admission step of `work`
    /// passes (`work::exact_work`): receive, call preparation, start, issue.
    pub(crate) async fn shared_work_control_for_test(
        &self,
        child: String,
        work: SharedWorkItem,
    ) -> Result<(), Error> {
        self.0
            .store
            .call(16 * 1024, move |db| check_work_control(&db.db, &child, &work))
            .await
    }
}
