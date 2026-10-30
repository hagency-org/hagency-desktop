//! Native owner-budget handoff. No script receives ConsentGrants, a registry,
//! provider identity, private receipt, or caller-selected original command IDs.
use super::{Host, Lease, parse};
use ::hagency::native_owner::{
    OwnerGrantAccess, OwnerGrantChoices, OwnerGrantProgress, OwnerGrantReview,
    OwnerGrantReviewResult, OwnerGrantPublicationReview, OwnerGrantPublicationResult,
    OwnerGrantPublicationProgress, OwnerGrantCancellationReview, OwnerGrantCancellationResult,
    OwnerGrantState, OwnerGrantForm, OwnerGrantSummary, OwnerGrantSuccessionForm,
    OwnerGrantSuccessionKind, OwnerGrantSuccessionState,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

pub(crate) const REVIEW_GRANT: &str = "hagency.grants.review";
/// Largest grant amount, in tokens, that stays an exact JSON integer.
pub(crate) const MAX_GRANT_TOKENS: u64 = 9_007_199_254_740_991;

/// A change to the pool's published grant that a script may ask the native
/// panel to prepare. Only the native form commits it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Succession {
    /// The same UTC month: a new per-request limit from now on.
    Amendment,
    /// Next UTC month, scheduled in the last days of this one.
    Renewal,
}
impl Succession {
    pub(crate) fn native(self) -> OwnerGrantSuccessionKind {
        match self {
            Self::Amendment => OwnerGrantSuccessionKind::Amendment,
            Self::Renewal => OwnerGrantSuccessionKind::Renewal,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Selector {
    contribution_id: Option<String>,
    intent_id: Option<String>,
    /// A script may only propose an amount for a new grant on this pool. It
    /// fills the native tokens field once; prepare never reads it.
    proposed_tokens: Option<u64>,
    /// Open the native form for an amendment or a renewal of the pool's
    /// published grant instead of a new grant.
    succession: Option<Succession>,
}
/// A complete Matrix user ID (`@name:server`), as the native form accepts.
pub(crate) fn matrix_user(id: &str) -> bool {
    id.len() <= 255
        && !id.chars().any(|c| c.is_whitespace() || c.is_control())
        && id.split_once(':').is_some_and(|(name, server)| {
            name.starts_with('@') && name.len() > 1 && !server.is_empty()
        })
}
impl Selector {
    fn parse(value: Value) -> Result<Self, String> {
        let value: Self = parse(value)?;
        let keys: Vec<_> = [&value.contribution_id, &value.intent_id]
            .into_iter()
            .flatten()
            .collect();
        if keys.len() != 1
            || keys.iter().any(|key| {
                key.is_empty()
                    || key.len() > 128
                    || !key
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            })
            || value.proposed_tokens.is_some_and(|tokens| {
                value.contribution_id.is_none() || tokens == 0 || tokens > MAX_GRANT_TOKENS
            })
            || (value.succession.is_some() && value.contribution_id.is_none())
            // A renewal is offered with next month's renewal support.
            || value.succession == Some(Succession::Renewal)
        {
            return Err("invalid_management_arguments".into());
        }
        Ok(value)
    }
}

/// Strings read from native widgets only. This is intentionally not Deserialize.
/// A script proposal can only pre-fill the native tokens field; these values
/// are always read back from the widgets the owner sees before committing.
#[derive(Clone, Default)]
pub(crate) struct GrantChoices {
    pub coordinator: String,
    pub managers: String,
    pub tokens: String,
    pub expires_utc: String,
    pub self_approval: bool,
}
impl GrantChoices {
    pub fn parse(self) -> Result<OwnerGrantChoices, String> {
        let amount = self
            .tokens
            .trim()
            .parse::<u64>()
            .map_err(|_| "invalid_grant_budget")?;
        let until = chrono::DateTime::parse_from_rfc3339(self.expires_utc.trim())
            .map_err(|_| "invalid_grant_expiry")?;
        let now = chrono::Utc::now().timestamp_millis();
        if until.offset().local_minus_utc() != 0
            || until.timestamp_millis() <= now
            || until.timestamp_millis() > now.saturating_add(366 * 86_400_000)
        {
            return Err("invalid_grant_expiry".into());
        }
        let mut managers: Vec<String> = self
            .managers
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        managers.sort();
        managers.dedup();
        if amount == 0
            || amount > MAX_GRANT_TOKENS
            || managers.is_empty()
            || managers.len() > 64
        {
            return Err("invalid_grant_budget".into());
        }
        let coordinator = self.coordinator.trim().to_owned();
        if std::iter::once(&coordinator)
            .chain(&managers)
            .any(|id| !matrix_user(id))
        {
            return Err("invalid_grant_manager".into());
        }
        Ok(OwnerGrantChoices {
            coordinator_mxid: coordinator,
            delegation_expires_at_ms: until.timestamp_millis() as u64,
            allow_self_approval: self.self_approval,
            eligible_managers: managers,
            allocated_tokens: amount,
        })
    }
}

pub(crate) enum GrantResult {
    Choose(OwnerGrantForm),
    /// Terms for an amendment or renewal of the published grant.
    ChooseSuccessor(Box<OwnerGrantSuccessionForm>),
    Review(Box<OwnerGrantReview>),
    Progress(Box<OwnerGrantProgress>),
    PublicationReview(Box<OwnerGrantPublicationReview>),
    PublicationProgress(Box<OwnerGrantPublicationProgress>),
    CancellationReview(Box<OwnerGrantCancellationReview>),
}
pub(crate) fn now_ms() -> u64 {
    u64::try_from(chrono::Utc::now().timestamp_millis()).unwrap_or(0)
}
/// A grant's UTC month has ended. Its receipt stays history; the server and the
/// native ledger accept one new grant for the same admission in a later month.
pub(crate) fn month_ended(summary: &OwnerGrantSummary, now: u64) -> bool {
    summary.period_end_ms <= now
}
/// A successor original that still names its predecessor: anything but a
/// cancelled review or a successor the native ledger rejected or cancelled.
fn live_successor(record: &OwnerGrantProgress) -> bool {
    record.summary.succession.is_some()
        && !matches!(record.state, OwnerGrantState::Declined)
        && !matches!(
            record.succession_state,
            Some(OwnerGrantSuccessionState::Rejected | OwnerGrantSuccessionState::Cancelled)
        )
}
/// A successor that still needs the owner: an unattempted or unknown review,
/// or a recorded change that the server has not been seen to accept.
pub(crate) fn unfinished_successor(record: &OwnerGrantProgress) -> bool {
    live_successor(record)
        && !(matches!(record.state, OwnerGrantState::Recorded)
            && matches!(record.succession_state, Some(OwnerGrantSuccessionState::Effective)))
}
// Journal order is creation order. Cancelled, canonically closed, ended or
// succeeded originals remain in history without hiding a pending, unknown,
// active or scheduled original. A grant is succeeded once a live successor
// names it: an amendment that replaced it, or next month's renewal.
fn existing_progress(
    records: Vec<OwnerGrantProgress>,
    now: u64,
) -> Result<Option<OwnerGrantProgress>, String> {
    let succeeded: std::collections::BTreeSet<String> = records
        .iter()
        .filter(|record| live_successor(record))
        .filter_map(|record| record.summary.succession.as_ref())
        .map(|succession| succession.predecessor_grant_id.clone())
        .collect();
    let mut pending = None;
    let mut declined = None;
    for record in records {
        if matches!(record.state, OwnerGrantState::Declined)
            || (matches!(record.state, OwnerGrantState::Recorded)
                && (record.closure.is_some()
                    || month_ended(&record.summary, now)
                    || succeeded.contains(&record.summary.grant_id)
                    || matches!(
                        record.succession_state,
                        Some(
                            OwnerGrantSuccessionState::Rejected
                                | OwnerGrantSuccessionState::Cancelled
                        )
                    )))
        {
            declined = Some(record);
        } else if pending.replace(record).is_some() {
            return Err("owner_grant_history_unavailable".into());
        }
    }
    Ok(pending.or(declined))
}
/// Only the native panel owns this. The exact host, package lease and native
/// manager stay bound throughout selection, review and asynchronous confirmation.
#[derive(Clone)]
pub(crate) struct GrantFlow {
    host: Host,
    lease: Lease,
    access: OwnerGrantAccess,
    selector: Selector,
    /// Whether the server advertises `pool-grant-succession-v1`: 0 not yet
    /// asked, 1 no, 2 yes. Only gates which native form is offered.
    succession_support: Arc<AtomicU8>,
}
impl GrantFlow {
    async fn form(&self) -> Result<GrantResult, String> {
        self.check()?;
        let form = self.access.publication_form(self.selector.contribution_id.clone()
            .ok_or("owner_grant_original_intent_required")?).await.map_err(|e| e.code)?;
        self.check()?;
        Ok(GrantResult::Choose(form))
    }
    fn check(&self) -> Result<(), String> {
        let account = self.host.check(&self.lease)?;
        self.lease.authorize(&account, REVIEW_GRANT, None)?;
        if self.access.retired() {
            return Err("management_scope_changed".into());
        }
        Ok(())
    }
    pub async fn open(&self) -> Result<GrantResult, String> {
        self.check()?;
        if let Some(contribution) = &self.selector.contribution_id {
            let originals = self.access.list_existing().await.map_err(|e| e.code)?;
            self.check()?;
            let matching = originals
                .into_iter()
                .filter(|s| &s.contribution_id == contribution);
            let mut records = Vec::new();
            for original in matching {
                records.push(self.access.recover(original.intent_id).await.map_err(|e| e.code)?);
                self.check()?;
            }
            let current = existing_progress(records, now_ms())?;
            if let Some(kind) = self.selector.succession {
                // An unfinished change is resolved before another one starts:
                // it is published, recovered or cancelled from its own record.
                if let Some(progress) = current.filter(unfinished_successor) {
                    return self.observe_progress(progress).await;
                }
                return self.successor_form(kind).await;
            }
            if let Some(progress) = current {
                return self.observe_progress(progress).await;
            }
            self.form().await
        } else {
            self.recover(self.selector.intent_id.clone().ok_or("owner_grant_original_intent_required")?).await
        }
    }
    pub fn can_choose_new_limits(&self) -> bool {
        self.selector.contribution_id.is_some()
    }
    /// The script's proposed amount for the native tokens field, if any.
    pub fn proposed_tokens(&self) -> Option<u64> {
        self.selector.proposed_tokens
    }
    /// The change the script asked the panel to prepare, if any.
    pub fn succession(&self) -> Option<Succession> {
        self.selector.succession
    }
    /// Last known server support for amending or renewing a published grant.
    pub fn succession_supported(&self) -> Option<bool> {
        match self.succession_support.load(Ordering::Acquire) {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        }
    }
    async fn refresh_succession_support(&self) -> Result<bool, String> {
        self.check()?;
        let supported = self.access.supports_succession().await.map_err(|e| e.code)?;
        self.check()?;
        self.succession_support
            .store(if supported { 2 } else { 1 }, Ordering::Release);
        Ok(supported)
    }
    async fn successor_form(&self, kind: Succession) -> Result<GrantResult, String> {
        self.check()?;
        let contribution = self
            .selector
            .contribution_id
            .clone()
            .ok_or("owner_grant_original_intent_required")?;
        let form = self
            .access
            .succession_form(contribution, kind.native())
            .await
            .map_err(|e| e.code)?;
        self.check()?;
        Ok(GrantResult::ChooseSuccessor(Box::new(form)))
    }
    /// Open the amendment or renewal form for this pool's published grant.
    /// The native form, review and publication still commit everything.
    pub async fn choose_successor(&self, kind: Succession) -> Result<GrantResult, String> {
        if !self.can_choose_new_limits() {
            return Err("owner_grant_original_intent_required".into());
        }
        self.successor_form(kind).await
    }
    pub async fn prepare_successor(
        &self,
        kind: Succession,
        mut choices: GrantChoices,
    ) -> Result<GrantResult, String> {
        self.check()?;
        choices.coordinator = self.account().to_owned();
        let choices = choices.parse()?;
        let contribution = self
            .selector
            .contribution_id
            .clone()
            .ok_or("owner_grant_original_intent_required")?;
        let review = self
            .access
            .prepare_successor(contribution, kind.native(), choices)
            .await
            .map_err(|e| e.code)?;
        self.check()?;
        Ok(GrantResult::Review(Box::new(review)))
    }
    /// Closed grants require a currently approved, distinct admission before
    /// reopening choices. A grant whose UTC month ended may instead be followed
    /// by this month's grant on the same admission: the same native check
    /// verifies that regrant. Prepare and the atomic writer repeat their checks.
    pub async fn choose_new_limits(&self) -> Result<GrantResult, String> {
        if !self.can_choose_new_limits() {
            return Err("owner_grant_original_intent_required".into());
        }
        let now = now_ms();
        match self.open().await? {
            GrantResult::Progress(p) if matches!(p.state, OwnerGrantState::Declined) => self.form().await,
            GrantResult::Progress(p) if matches!(p.state, OwnerGrantState::Recorded)
                && (p.closure.is_some() || month_ended(&p.summary, now)) => self.new_limits().await,
            GrantResult::PublicationProgress(p) if month_ended(&p.summary, now) => self.new_limits().await,
            result => Ok(result),
        }
    }
    async fn new_limits(&self) -> Result<GrantResult, String> {
        self.access
            .check_new_admission(
                self.selector
                    .contribution_id
                    .clone()
                    .ok_or("owner_grant_original_intent_required")?,
            )
            .await
            .map_err(|e| e.code)?;
        self.check()?;
        self.form().await
    }
    pub fn account(&self) -> &str {
        &self.lease.identity().account
    }
    pub fn active(&self) -> bool {
        self.check().is_ok()
    }
    pub async fn review(&self, id: Option<String>) -> Result<GrantResult, String> {
        self.check()?;
        let id = id
            .or_else(|| self.selector.intent_id.clone())
            .ok_or("owner_grant_original_intent_required")?;
        let result = self.access.review(id).await.map_err(|e| e.code)?;
        self.check()?;
        Ok(match result {
            OwnerGrantReviewResult::Review(review) => GrantResult::Review(review),
            OwnerGrantReviewResult::Progress(progress) => GrantResult::Progress(progress),
        })
    }
    pub async fn prepare(&self, mut choices: GrantChoices) -> Result<GrantResult, String> {
        self.check()?;
        choices.coordinator = self.account().to_owned();
        let choices = choices.parse()?;
        let contribution = self
            .selector
            .contribution_id
            .clone()
            .ok_or("owner_grant_original_intent_required")?;
        let review = self
            .access
            .prepare(contribution, choices)
            .await
            .map_err(|e| e.code)?;
        self.check()?;
        Ok(GrantResult::Review(Box::new(review)))
    }
    pub async fn confirm(&self, review: OwnerGrantReview) -> Result<GrantResult, String> {
        self.check()?;
        if review.expired() {
            return Err("owner_grant_original_review_expired".into());
        }
        let result = self.access.confirm(review).await.map_err(|e| e.code);
        if self.check().is_err() {
            return Err("management_outcome_unknown".into());
        }
        result.map(|progress| GrantResult::Progress(Box::new(progress)))
    }
    pub async fn publication_review(&self, id: String) -> Result<GrantResult, String> {
        self.check()?;
        let result = self
            .access
            .review_publication(id)
            .await
            .map_err(|e| e.code)?;
        self.check()?;
        Ok(match result {
            OwnerGrantPublicationResult::Review(review) => GrantResult::PublicationReview(review),
            OwnerGrantPublicationResult::Progress(progress) => {
                GrantResult::PublicationProgress(progress)
            }
        })
    }
    pub async fn publication_confirm(
        &self,
        review: OwnerGrantPublicationReview,
    ) -> Result<GrantResult, String> {
        self.check()?;
        if review.expired() {
            return Err("owner_grant_publication_review_expired".into());
        }
        let result = self
            .access
            .confirm_publication(review)
            .await
            .map_err(|e| e.code);
        if self.check().is_err() {
            return Err("management_outcome_unknown".into());
        }
        result.map(|p| GrantResult::PublicationProgress(Box::new(p)))
    }
    pub async fn recover(&self, id: String) -> Result<GrantResult, String> {
        self.check()?;
        let result = self.access.recover(id).await.map_err(|e| e.code)?;
        self.check()?;
        self.observe_progress(result).await
    }
    async fn observe_progress(&self, progress: OwnerGrantProgress) -> Result<GrantResult, String> {
        self.check()?;
        // Keep canonical closure and unsettled local outcomes visible. Only a
        // recorded, open reservation can be replaced by its publication status.
        if matches!(progress.state, OwnerGrantState::Recorded) && progress.closure.is_none() {
            let publication = self.access.observe_publication(progress.summary.intent_id.clone())
                .await.map_err(|e| e.code)?;
            self.check()?;
            if let Some(publication) = publication {
                // A published grant with a proposal or a requested change can
                // be amended only if the server advertises succession.
                // A failed probe leaves support unknown, which keeps today's
                // read-only answer; it never hides the published grant.
                if (self.selector.proposed_tokens.is_some() || self.selector.succession.is_some())
                    && self.succession_supported().is_none()
                {
                    let _ = self.refresh_succession_support().await;
                    self.check()?;
                }
                return Ok(GrantResult::PublicationProgress(Box::new(publication)));
            }
        }
        Ok(GrantResult::Progress(Box::new(progress)))
    }
    pub async fn cancellation_review(&self, id: String) -> Result<GrantResult, String> {
        self.check()?;
        let result = self.access.review_cancellation(id).await.map_err(|e| e.code)?;
        self.check()?;
        Ok(match result {
            OwnerGrantCancellationResult::Review(review) => GrantResult::CancellationReview(review),
            OwnerGrantCancellationResult::Progress(progress) => GrantResult::Progress(progress),
        })
    }
    pub async fn cancellation_confirm(&self, review: OwnerGrantCancellationReview) -> Result<GrantResult, String> {
        self.check()?;
        let result = self.access.confirm_cancellation(review).await.map_err(|e| e.code);
        if self.check().is_err() {
            return Err("management_outcome_unknown".into());
        }
        result.map(|p| GrantResult::Progress(Box::new(p)))
    }
}
impl Host {
    pub fn grant_handoff(&self, lease: Lease, args: Value) -> Result<GrantFlow, String> {
        let selector = Selector::parse(args)?;
        let account = self.check(&lease)?;
        lease.authorize(&account, REVIEW_GRANT, None)?;
        // Only the native adoption handoff or authenticated descriptor reload
        // can supply this qualified registry/session. This script selector
        // cannot choose a path or initialize a new resource authority.
        let access = self
            .0
            .grant_access
            .lock()
            .map_err(|_| "management_unavailable")?
            .clone()
            .ok_or("owner_source_adoption_required")?;
        let weak = std::sync::Arc::downgrade(&self.0);
        let gate_lease = lease.clone();
        let access = access
            .restrict_to_host(move || {
                weak.upgrade().is_some_and(|host| {
                    !host.retired.load(std::sync::atomic::Ordering::Acquire)
                        && host.identity == *gate_lease.identity()
                        && gate_lease.check(&host.identity.account).is_ok()
                })
            })
            .map_err(|error| error.code)?;
        let flow = GrantFlow {
            host: self.clone(),
            lease,
            access,
            selector,
            succession_support: Arc::new(AtomicU8::new(0)),
        };
        flow.check()?;
        Ok(flow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn progress(id: &str, state: OwnerGrantState) -> OwnerGrantProgress {
        OwnerGrantProgress {
            state,
            closure: None,
            succession_state: None,
            summary: ::hagency::native_owner::OwnerGrantSummary {
                intent_id: id.into(), grant_id: format!("grant_{id}"), owner_mxid: "@owner:test".into(),
                contribution_id: "contribution".into(), resource_id: "resource".into(),
                coordinator_mxid: "@coordinator:test".into(), allocated_tokens: 700,
                eligible_managers: vec!["@manager:test".into()], allow_self_approval: false,
                delegation_expires_at_ms: 1793491200000, period_key: "2026-10".into(),
                period_start_ms: 1790812800000, period_end_ms: 1793491200000,
                succession: None,
            },
        }
    }
    /// A successor original of `predecessor`, in the given native state.
    fn successor(
        id: &str,
        predecessor: &OwnerGrantProgress,
        kind: OwnerGrantSuccessionKind,
        state: OwnerGrantState,
        native: Option<OwnerGrantSuccessionState>,
    ) -> OwnerGrantProgress {
        let mut p = progress(id, state);
        p.succession_state = native;
        p.summary.succession = Some(::hagency::native_owner::OwnerGrantSuccessionSummary {
            kind,
            predecessor_grant_id: predecessor.summary.grant_id.clone(),
            predecessor_allocated_tokens: predecessor.summary.allocated_tokens,
            carried_children: 2,
            carried_retained_tokens: 300,
        });
        p
    }
    /// Inside the fixtures' 2026-10 month, and that month's end (2026-11-01).
    const OCTOBER: u64 = 1_792_000_000_000;
    const NOVEMBER: u64 = 1_793_491_200_000;
    fn in_month(mut p: OwnerGrantProgress, key: &str, start: u64, end: u64) -> OwnerGrantProgress {
        p.summary.period_key = key.into();
        p.summary.period_start_ms = start;
        p.summary.period_end_ms = end;
        p
    }
    #[test]
    fn closed_originals_never_hide_pending_unknown_or_recorded_effects() {
        for state in [OwnerGrantState::ReviewRequired, OwnerGrantState::ReviewExpired,
            OwnerGrantState::OutcomeUnknown, OwnerGrantState::Recorded] {
            let result = existing_progress(vec![progress("closed-before", OwnerGrantState::Declined),
                progress("original", state), progress("closed-after", OwnerGrantState::Declined)], OCTOBER).unwrap().unwrap();
            assert_eq!(result.summary.intent_id, "original");
            assert!(!matches!(result.state, OwnerGrantState::Declined));
        }
        assert!(existing_progress(vec![progress("one", OwnerGrantState::OutcomeUnknown),
            progress("two", OwnerGrantState::ReviewRequired)], OCTOBER).is_err());
    }
    #[test]
    fn cancellation_history_keeps_latest_closed_original_for_explicit_new_choices() {
        let result = existing_progress(vec![progress("first", OwnerGrantState::Declined),
            progress("second", OwnerGrantState::Declined)], OCTOBER).unwrap().unwrap();
        assert_eq!(result.summary.intent_id, "second");
        assert!(matches!(result.state, OwnerGrantState::Declined));
        assert!(existing_progress(vec![], OCTOBER).unwrap().is_none());
    }
    #[test]
    fn ended_months_are_history_beside_this_month_original() {
        let september = in_month(
            progress("september", OwnerGrantState::Recorded),
            "2026-09",
            1_788_220_800_000,
            1_790_812_800_000,
        );
        let october = progress("october", OwnerGrantState::Recorded);
        assert!(!month_ended(&october.summary, NOVEMBER - 1));
        assert!(month_ended(&october.summary, NOVEMBER));
        // In its own month a recorded grant is the current original.
        let current = existing_progress(vec![september.clone(), october.clone()], OCTOBER)
            .unwrap()
            .unwrap();
        assert_eq!(current.summary.intent_id, "october");
        assert!(
            existing_progress(
                vec![
                    october.clone(),
                    progress("review", OwnerGrantState::ReviewRequired)
                ],
                OCTOBER
            )
            .is_err()
        );
        // Several ended grants are history, never an ambiguous history.
        let ended = existing_progress(vec![september.clone(), october.clone()], NOVEMBER)
            .unwrap()
            .unwrap();
        assert_eq!(ended.summary.intent_id, "october");
        assert!(matches!(ended.state, OwnerGrantState::Recorded));
        // This month's original is current beside them, whatever its state.
        for state in [
            OwnerGrantState::ReviewRequired,
            OwnerGrantState::ReviewExpired,
            OwnerGrantState::OutcomeUnknown,
            OwnerGrantState::Recorded,
        ] {
            let november = in_month(
                progress("november", state),
                "2026-11",
                NOVEMBER,
                1_796_083_200_000,
            );
            let p = existing_progress(vec![september.clone(), november, october.clone()], NOVEMBER)
                .unwrap()
                .unwrap();
            assert_eq!(p.summary.intent_id, "november");
        }
        // An unresolved original is never history, even after its month.
        let unknown = progress("unknown", OwnerGrantState::OutcomeUnknown);
        let p = existing_progress(vec![october.clone(), unknown.clone()], NOVEMBER)
            .unwrap()
            .unwrap();
        assert_eq!(p.summary.intent_id, "unknown");
        assert!(
            existing_progress(
                vec![
                    unknown,
                    in_month(
                        progress("november", OwnerGrantState::ReviewRequired),
                        "2026-11",
                        NOVEMBER,
                        1_796_083_200_000
                    )
                ],
                NOVEMBER
            )
            .is_err()
        );
    }
    fn settled(id: &str) -> OwnerGrantProgress {
        let mut p = progress(id, OwnerGrantState::Recorded);
        p.closure = Some(::hagency::native_owner::OwnerGrantClosure {
            retained_tokens: 200, known_consumed_tokens: 40,
            unresolved_tokens: 160, observed_at_ms: 1792340000000,
        });
        p
    }
    #[test]
    fn settled_rounds_keep_history_without_hiding_the_current_original() {
        for state in [OwnerGrantState::ReviewRequired, OwnerGrantState::ReviewExpired,
            OwnerGrantState::OutcomeUnknown, OwnerGrantState::Recorded] {
            let p = existing_progress(vec![settled("old"), progress("current", state),
                settled("other_closed")], OCTOBER).unwrap().unwrap();
            assert_eq!(p.summary.intent_id, "current");
            assert!(p.closure.is_none());
        }
        let p = existing_progress(vec![settled("first"), settled("last")], OCTOBER)
            .unwrap()
            .unwrap();
        assert_eq!(p.summary.intent_id, "last");
        assert_eq!(p.closure.unwrap().retained_tokens, 200);
    }
    #[test]
    fn settlement_metadata_cannot_hide_an_unknown_effect() {
        let mut unknown = settled("unknown");
        unknown.state = OwnerGrantState::OutcomeUnknown;
        assert!(
            existing_progress(
                vec![unknown, progress("current", OwnerGrantState::Recorded)],
                OCTOBER
            )
            .is_err()
        );
    }
    #[test]
    fn handoff_rejects_authority_fields_and_ambiguous_selectors() {
        for bad in [
            json!({}),
            json!({"contributionId":"a","intentId":"b"}),
            json!({"intentId":"a","allocatedTokens":500}),
            json!({"contributionId":"a","registryPath":"/tmp"}),
            json!({"contributionId":"a","ownerMxid":"@other:test"}),
            json!({"intentId":"../other"}),
        ] {
            assert!(Selector::parse(bad).is_err());
        }
        assert!(Selector::parse(json!({"intentId":"original_1"})).is_ok());
    }
    #[test]
    fn proposed_tokens_are_a_bounded_integer_for_a_pool_only() {
        let proposal = |value: Value| Selector::parse(value).map(|s| s.proposed_tokens);
        assert_eq!(proposal(json!({"contributionId":"pool_1","proposedTokens":1})), Ok(Some(1)));
        assert_eq!(
            proposal(json!({"contributionId":"pool_1","proposedTokens":MAX_GRANT_TOKENS})),
            Ok(Some(MAX_GRANT_TOKENS))
        );
        assert_eq!(proposal(json!({"contributionId":"pool_1"})), Ok(None));
        assert_eq!(proposal(json!({"contributionId":"pool_1","proposedTokens":null})), Ok(None));
        for bad in [
            json!({"proposedTokens":3000000}),
            json!({"intentId":"original_1","proposedTokens":3000000}),
            json!({"contributionId":"pool_1","intentId":"original_1","proposedTokens":3000000}),
            json!({"contributionId":"pool_1","proposedTokens":0}),
            json!({"contributionId":"pool_1","proposedTokens":MAX_GRANT_TOKENS + 1}),
            json!({"contributionId":"pool_1","proposedTokens":-1}),
            json!({"contributionId":"pool_1","proposedTokens":1.5}),
            json!({"contributionId":"pool_1","proposedTokens":"3000000"}),
            json!({"contributionId":"pool_1","proposedTokens":true}),
            json!({"contributionId":"pool_1","proposedTokens":[3000000]}),
            json!({"contributionId":"pool_1","proposedTokens":3000000,"allocatedTokens":3000000}),
        ] {
            assert_eq!(proposal(bad.clone()), Err("invalid_management_arguments".into()), "{bad}");
        }
    }
    #[test]
    fn an_amendment_replaces_the_published_grant_as_the_current_original() {
        use OwnerGrantSuccessionKind::Amendment;
        use OwnerGrantSuccessionState::*;
        let root = progress("root", OwnerGrantState::Recorded);
        // Every live state of a successor makes it the current original:
        // its review, an unknown outcome, a saved or an effective change.
        for (state, native) in [
            (OwnerGrantState::ReviewRequired, None),
            (OwnerGrantState::ReviewExpired, None),
            (OwnerGrantState::OutcomeUnknown, None),
            (OwnerGrantState::Recorded, Some(Pending)),
            (OwnerGrantState::Recorded, Some(Effective)),
        ] {
            let change = successor("change", &root, Amendment, state.clone(), native);
            let current = existing_progress(vec![root.clone(), change], OCTOBER).unwrap().unwrap();
            assert_eq!(current.summary.intent_id, "change");
            assert_eq!(
                unfinished_successor(&current),
                !matches!(native, Some(Effective))
            );
        }
        // A cancelled review or a rejected change leaves the published grant current.
        for (state, native) in [
            (OwnerGrantState::Declined, None),
            (OwnerGrantState::Recorded, Some(Rejected)),
            (OwnerGrantState::Recorded, Some(Cancelled)),
        ] {
            let change = successor("change", &root, Amendment, state, native);
            assert!(!unfinished_successor(&change));
            let current = existing_progress(vec![root.clone(), change.clone()], OCTOBER).unwrap().unwrap();
            assert_eq!(current.summary.intent_id, "root");
            // A later change after a rejected one is current again.
            let again = successor("again", &root, Amendment, OwnerGrantState::Recorded, Some(Pending));
            let current = existing_progress(vec![root.clone(), change, again], OCTOBER).unwrap().unwrap();
            assert_eq!(current.summary.intent_id, "again");
        }
        // A chain of amendments: only its head is current.
        let first = successor("first", &root, Amendment, OwnerGrantState::Recorded, Some(Effective));
        let second = successor("second", &first, Amendment, OwnerGrantState::Recorded, Some(Effective));
        let current = existing_progress(vec![root.clone(), first, second], OCTOBER).unwrap().unwrap();
        assert_eq!(current.summary.intent_id, "second");
        // Two live originals that do not succeed each other stay ambiguous.
        assert!(existing_progress(vec![root.clone(), progress("other", OwnerGrantState::Recorded)], OCTOBER).is_err());
    }
    #[test]
    fn a_scheduled_renewal_is_current_and_survives_the_boundary() {
        let october = progress("october", OwnerGrantState::Recorded);
        let mut renewal = successor(
            "renewal",
            &october,
            OwnerGrantSuccessionKind::Renewal,
            OwnerGrantState::Recorded,
            Some(OwnerGrantSuccessionState::Effective),
        );
        renewal = in_month(renewal, "2026-11", NOVEMBER, 1_796_083_200_000);
        for now in [OCTOBER, NOVEMBER, NOVEMBER + 1] {
            let current = existing_progress(vec![october.clone(), renewal.clone()], now).unwrap().unwrap();
            assert_eq!(current.summary.intent_id, "renewal");
        }
        // Once the renewal's own month ends, it is history like any grant.
        let ended = existing_progress(vec![october, renewal], 1_796_083_200_000).unwrap().unwrap();
        assert!(matches!(ended.state, OwnerGrantState::Recorded));
    }
    #[test]
    fn succession_is_a_bounded_pool_argument() {
        let parsed = |value: Value| Selector::parse(value);
        let s = parsed(json!({"contributionId":"pool_1","proposedTokens":800000,"succession":"amendment"})).unwrap();
        assert_eq!(s.succession, Some(Succession::Amendment));
        assert_eq!(s.proposed_tokens, Some(800000));
        // Scripts cannot request a renewal yet.
        assert!(parsed(json!({"contributionId":"pool_1","succession":"renewal"})).is_err());
        for bad in [
            json!({"intentId":"original_1","succession":"amendment"}),
            json!({"contributionId":"pool_1","succession":"revoke"}),
            json!({"contributionId":"pool_1","succession":"Amendment"}),
            json!({"contributionId":"pool_1","succession":true}),
            json!({"contributionId":"pool_1","succession":"amendment","eligibleManagers":["@a:test"]}),
        ] {
            assert!(parsed(bad.clone()).is_err(), "{bad}");
        }
    }
    #[test]
    fn native_choices_require_explicit_utc_and_bounded_budget() {
        let input = |expiry: &str, tokens: &str| GrantChoices {
            coordinator: "@coordinator:test".into(),
            managers: "@b:test\n@a:test\n@b:test".into(),
            tokens: tokens.into(),
            expires_utc: expiry.into(),
            self_approval: false,
        };
        let future = (chrono::Utc::now() + chrono::Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let parsed = input(&future, "700").parse().unwrap();
        assert_eq!(parsed.eligible_managers, ["@a:test", "@b:test"]);
        assert!(!parsed.allow_self_approval);
        assert!(input("2030-01-01T08:00:00+08:00", "700").parse().is_err());
        assert!(input(&future, "0").parse().is_err());
        assert!(input(&future, "9007199254740992").parse().is_err());
    }
    #[test]
    fn bundle_permission_mapping_never_grants_owner_consent() {
        let authority = rinx_miniapp_core::SessionAuthority::default();
        let lease = authority.issue(
            crate::miniapps::InstanceId {
                app: octosense_app_contract::hagency::APP_ID.into(),
                account: "@owner:test".into(),
                room: None,
                generation: 1,
            },
            [
                REVIEW_GRANT.to_owned(),
                "hagency.onboarding.review".to_owned(),
                "hagency.onboarding.status".to_owned(),
            ]
            .into(),
            Default::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(10),
        );
        assert!(super::super::permissions(&lease).is_empty());
        for capability in [
            REVIEW_GRANT,
            "hagency.onboarding.review",
            "hagency.onboarding.status",
        ] {
            assert!(octosense_app_contract::hagency::SERVICES.contains(&capability));
        }
    }
}
