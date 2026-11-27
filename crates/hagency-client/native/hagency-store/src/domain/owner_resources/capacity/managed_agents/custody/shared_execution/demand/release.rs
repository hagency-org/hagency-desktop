//! ADR-193 amendment (retry v2): how a claimed demand attempt's stop reaches
//! the server, decided once beside its stop. The stop of an attempt whose
//! event opted in is sent as the demand's next retry request instead of the
//! plain result upload, whether or not its turn began: the server decides,
//! by its record of the turn's start, the one line there is. Work whose turn
//! it never recorded it gives back and retries in place; begun work stays
//! bound. Only when the server shows that retry does the binding here move
//! into a retained release row (`release`, beside the retry's intent), so
//! the retry's work can bind. Nothing here starts a process, a call or a turn.
use super::*;

/// How a claimed attempt's stop reaches the server.
#[derive(Clone, Debug)]
pub enum SharedAttemptStopRoute {
    /// The plain attempt result upload.
    Plain,
    /// Retry `retry` of `offer`'s event, carrying the stop receipt.
    Retry { offer: SharedDemandOffer, retry: u64 },
}

/// This installation holds the server's word that `work_id`'s turn began:
/// the answer that recorded its start (`start_confirmed`), an issued call, an
/// observed reply, a work observation or a reply delivery. The server's record
/// of the start decides; this only refuses a release that contradicts what
/// the server already said here. Migration 117 checks the same facts.
fn started(db: &Connection, work_id: &str) -> Result<bool, Error> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_work_events WHERE work_id=?1 AND kind IN('start_confirmed','call_issued','reply_observed')) OR EXISTS(SELECT 1 FROM owner_shared_work_observations WHERE work_id=?1) OR EXISTS(SELECT 1 FROM owner_shared_reply_deliveries WHERE work_id=?1)",
        [work_id],
        |r| r.get(0),
    )?)
}

/// Work given back is never bound to its demand again: work released here,
/// or work of a process that a retry of its demand names as its predecessor
/// (the server shows a retry only after it gave that process's work back, so
/// a `bound` answer read before then is history).
pub(super) fn handed_back(
    db: &Connection,
    binding: &SharedDemandBinding,
) -> Result<bool, Error> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_demand_released_bindings WHERE work_id=?1) OR EXISTS(SELECT 1 FROM owner_shared_start_intents WHERE id IN('sdr1_'||?2,'sdr2_'||?2,'sdr3_'||?2) AND predecessor_digest=?3)",
        params![binding.work_id, binding.demand_id, binding.runtime_command_digest],
        |r| r.get(0),
    )?)
}

/// The event's opt-in stands: its original start was saved with `reoffers`
/// and none of its retries was first prepared without it (`lapse`). Only
/// then does a stop ask for a retry: the server gives back only the work of
/// a start stored with `reoffers`, and after a lapse every start of the
/// event goes out without it.
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

/// Whether start `n` of the saved event says `reoffers`, read from rows that
/// never change, so that every send of a start has its first bytes (the
/// server refuses other bytes for the same start): the original start as it
/// was saved, a retry as its event's opt-in stood when its intent was saved,
/// that is with no lapse below the intent's ledger revision.
pub(super) fn said_reoffers(db: &Connection, saved: &SharedDemandIntent, n: u64) -> Result<bool, Error> {
    if !saved.input.as_ref().is_some_and(|i| i.reoffers) {
        return Ok(false);
    }
    if n == 0 {
        return Ok(true);
    }
    let intent: u64 = db
        .query_row(
            "SELECT ledger_revision FROM owner_shared_start_intents WHERE id=?1",
            [retry::start_id(&saved.offer.demand_id, n)?],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(Error::Conflict)?;
    Ok(!db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_demand_reoffer_lapses WHERE demand_id=?1 AND ledger_revision<?2)",
        params![saved.offer.demand_id, intent],
        |r| r.get::<_, bool>(0),
    )?)
}

/// Saved, before its intent and in the same transaction, when a retry of
/// `demand_id` is first prepared without `reoffers` although its event opted
/// in (the server that showed the retry no longer lists retry v2): from then
/// on no stop of the event asks for a retry, and that retry and every later
/// start of the event go out without it.
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

/// The demand this stopped attempt was started for (`id`, by its original
/// start or its retry `k`) and the retry its stop asks for, when the stop
/// may ask for one. Whether its turn began is not asked here: the server
/// answers that by its record of the turn's start.
fn retryable(
    db: &Connection,
    profile: &str,
    command: &SharedAttemptCommand,
    digest: &str,
    (id, k): (String, u64),
) -> Result<Option<(SharedDemandOffer, u64)>, Error> {
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
    // No other start holds this attempt's place as predecessor. A start the
    // server never made, of an event it ended, holds none (schema 114), also
    // for a route decided at the stop's first upload, after such a start.
    let competing: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_shared_start_intents WHERE child_id=?1 AND predecessor_digest=?2 AND NOT EXISTS(SELECT 1 FROM owner_shared_start_releases r WHERE r.start_intent_id=owner_shared_start_intents.id)) OR EXISTS(SELECT 1 FROM owner_shared_budget_start_intents WHERE child_id=?1 AND predecessor_digest=?2)",
        params![child, digest],
        |r| r.get(0),
    )?;
    if terminal || competing {
        return Ok(None);
    }
    // The demand's local work: none, or this attempt's own, begun or not.
    let binding: Option<(String, String)> = db
        .query_row(
            "SELECT work_id,binding_payload FROM owner_shared_demand_bindings WHERE demand_id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((work, payload)) = &binding {
        let b: SharedDemandBinding = serde_json::from_str(payload)?;
        if b.runtime_command_id != command.command_id
            || b.runtime_command_digest != digest
            || &b.work_id != work
        {
            return Ok(None);
        }
    }
    Ok(Some((saved.offer, n)))
}

/// Decide once how `command`'s stop reaches the server, beside its stopped
/// observation `observation_id`; a saved route is returned unchanged. Only
/// what is saved here decides, and nothing moves: the server answers the
/// retry request by its record of the turn's start, and the binding here
/// moves only when it shows the retry (`release`). An attempt that was not
/// started for a demand has nothing to decide and saves nothing.
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
    let Some((offer, n)) = eligible else {
        let rev = revision(tx)?;
        tx.execute(
            "INSERT INTO owner_shared_stop_routes VALUES(?1,?2,?3,?4,'plain',NULL,NULL,?5)",
            params![command.command_id, child, profile, observation_id, rev],
        )?;
        return Ok(SharedAttemptStopRoute::Plain);
    };
    let rev = revision(tx)?;
    tx.execute(
        "INSERT INTO owner_shared_stop_routes VALUES(?1,?2,?3,?4,'retry',?5,?6,?7)",
        params![command.command_id, child, profile, observation_id, offer.demand_id, n, rev],
    )?;
    Ok(SharedAttemptStopRoute::Retry { offer, retry: n })
}

/// The server showed a retry of `demand_id`'s event after `head`, the
/// stopped process whose stop asked for it: it gave that process's work back
/// (it opens such a retry only for work whose turn it never recorded). Beside
/// the retry's intent, saved first in the same transaction, the binding here,
/// if any, moves byte for byte into a retained release row, so that the
/// retry's work can bind; the released work is never bound or admitted
/// again. A binding of other work means the event is finished here. One this
/// installation holds the server's word of a begun turn for (`started`)
/// contradicts the server and is kept.
pub(super) fn release(tx: &Connection, demand_id: &str, head: &str) -> Result<(), Error> {
    let row: Option<(String, String, String, u64)> = tx
        .query_row(
            "SELECT work_id,binding_payload,binding_digest,ledger_revision FROM owner_shared_demand_bindings WHERE demand_id=?1",
            [demand_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((work, payload, binding_digest, ledger)) = row else {
        return Ok(());
    };
    let b: SharedDemandBinding = serde_json::from_str(&payload)?;
    if b.runtime_command_id != head || b.work_id != work {
        return Err(Error::LocalAuthority);
    }
    if started(tx, &work)? {
        return Err(Error::Conflict);
    }
    bounded_row(tx, "owner_shared_demand_released_bindings", "work_id", &work, 100_000)?;
    let rev = revision(tx)?;
    tx.execute(
        "INSERT INTO owner_shared_demand_released_bindings VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![work, demand_id, payload, binding_digest, ledger, rev, graphs::now_ms()?],
    )?;
    tx.execute(
        "DELETE FROM owner_shared_demand_bindings WHERE demand_id=?1 AND work_id=?2",
        params![demand_id, work],
    )?;
    Ok(())
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
