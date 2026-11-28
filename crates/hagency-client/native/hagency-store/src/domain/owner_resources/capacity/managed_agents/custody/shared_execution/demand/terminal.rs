//! An authenticated terminal state for an exact saved v2 offer (ADR-193). It
//! releases the offer's FIFO position, and the place after its predecessor
//! of each start of it the server never made: no work, call, start, process
//! or capacity history changes, and its event is never offered or started
//! again. Same-installation cleanup authority suffices, so an offer older
//! than a day can still end; the server's state is the only input that is
//! trusted.
use super::*;

/// The server makes no start of an ended event, and its status names the
/// event's latest start. Each start of the event it never made, and for which
/// no command was issued here, gives its place back (schema 114).
const RELEASE: &str = "INSERT INTO owner_shared_start_releases(start_intent_id,demand_id) SELECT start_intent_id,demand_id FROM owner_shared_start_releasable WHERE demand_id=?1";

/// Budget blocks end an event here only after a retry (ADR-193 amendment):
/// the original start's own block is its saved decision.
const TERMINAL: [&str; 5] = [
    "expired",
    "cancelled",
    "head_changed",
    "budget_blocked",
    "usage_unrepresentable",
];

/// Version-neutral idempotency check over a retained terminal payload.
fn same_terminal(saved: &str, offer: &SharedDemandOffer, state: &str) -> Result<bool, Error> {
    let saved: serde_json::Value = serde_json::from_str(saved)?;
    let old: SharedDemandOffer = serde_json::from_value(saved["offer"].clone())?;
    Ok(old.digest()? == offer.digest()? && saved["state"] == state)
}

/// Both lanes read every saved event, and the status each lane reads names
/// only that lane's starts. The end read first is the event's end, `saved`;
/// the other lane's end is kept beside it (schema 115), so it gives back
/// that lane's starts too. True: this lane's end is to be kept beside
/// `saved`. False: this lane's end is saved already.
pub(in super::super) fn second_terminal(
    tx: &Connection,
    saved: &str,
    plain: bool,
    offer: &SharedDemandOffer,
    state: &str,
) -> Result<bool, Error> {
    let saved_plain = serde_json::from_str::<serde_json::Value>(saved)?.get("version").is_some();
    let own: Option<String> = if saved_plain == plain {
        Some(saved.to_owned())
    } else {
        tx.query_row(
            "SELECT payload FROM owner_shared_demand_second_terminals WHERE demand_id=?1",
            [&offer.demand_id], |r| r.get(0),
        ).optional()?
    };
    match own {
        Some(own) if same_terminal(&own, offer, state)? => Ok(false),
        Some(_) => Err(Error::Conflict),
        None => Ok(true),
    }
}

impl OwnerResourceRegistry {
    pub async fn retain_shared_demand_terminal(
        &self,
        authority: ManagedCleanupAuthority,
        offer: SharedDemandOffer,
        state: String,
        runtime_command_id: Option<String>,
    ) -> Result<(), Error> {
        authority.check(&self.0)?;
        offer.validate()?;
        if !TERMINAL.contains(&state.as_str()) {
            return Err(Error::LocalAuthority);
        }
        if let Some(id) = &runtime_command_id {
            identifier(id, 128)?;
        }
        let profile = authority.enrollment.profile.clone();
        let expires = authority.deadline();
        let registry = self.0.clone();
        self.0.store.call(64 * 1024, move |db| {
            authority.check(&registry)?;
            let tx = db.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            authority::cleanup(&tx, &authority)?;
            let Some(old) = journal::load(&tx, &authority.enrollment.profile.digest, &authority.child, &offer.demand_id)? else {
                authority.check(&registry)?;
                return Ok(());
            };
            if old.offer.digest()? != offer.digest()? {
                return Err(Error::Conflict);
            }
            let saved: Option<String> = tx.query_row(
                "SELECT payload FROM owner_shared_budget_demand_terminals WHERE demand_id=?1",
                [&offer.demand_id], |r| r.get(0),
            ).optional()?;
            // An end the budget lane read first keeps this one beside it.
            let table = match saved.map(|saved| second_terminal(&tx, &saved, true, &offer, &state)) {
                None => "owner_shared_budget_demand_terminals",
                Some(Ok(true)) => "owner_shared_demand_second_terminals",
                Some(saved) => {
                    authority.check(&registry)?;
                    return saved.map(|_| ());
                }
            };
            let value = json!({"version":1,"offer":offer,"demandDigest":offer.digest()?,"state":state,"runtimeCommandId":runtime_command_id});
            let revision = revision(&tx)?;
            tx.execute(
                &format!("INSERT INTO {table} VALUES(?1,?2,?3,?4)"),
                params![offer.demand_id, canonical::encode(&value)?, canonical::digest(&value)?, revision],
            )?;
            tx.execute(RELEASE, [&offer.demand_id])?;
            authority.check(&registry)?;
            tx.commit()?;
            authority.check(&registry).map_err(|_| Error::OutcomeUnknown)
        }).await?;
        after(&profile, &self.0, expires)
    }
}
