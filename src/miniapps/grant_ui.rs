//! Trusted native owner grant screen. The package cannot render or acknowledge
//! this confirmation; its reply contains only public progress and original IDs.
use super::*;
use crate::host::hagency::grants::{
    GrantChoices, GrantFlow, GrantResult, Succession, month_ended, now_ms,
};
use ::hagency::native_owner::{
    OwnerGrantProgress, OwnerGrantReview, OwnerGrantState, OwnerGrantPublicationReview,
    OwnerGrantPublicationProgress, OwnerGrantPublicationState, OwnerGrantCancellationReview,
    OwnerGrantSummary, OwnerGrantForm, OwnerGrantSuccessionForm, OwnerGrantSuccessionKind,
};

pub(super) struct GrantPrompt {
    flow: GrantFlow,
    reply: (usize, u64),
    phase: Phase,
    original: Option<String>,
    receiver: Option<Receiver<Result<GrantResult, String>>>,
    draft: GrantChoices,
    publishing: bool,
    cancelling: bool,
    form: Option<OwnerGrantForm>,
    /// One explicit owner submission covers reservation and publication of
    /// these same terms. Recovery never sets this flag.
    submit_publication: bool,
    /// The script's proposed tokens, shown beside the current amount until the
    /// owner submits terms natively. Only the native tokens field commits.
    proposal: Option<u64>,
    /// The owner asked for this month's limits after a grant's month ended.
    regrant: bool,
    /// While choosing an amendment or renewal: the native successor facts.
    /// `form` then stays empty; the Choose phase reads one or the other.
    successor: Option<OwnerGrantSuccessionForm>,
}
enum Phase {
    Loading,
    Choose,
    Review(Box<OwnerGrantReview>),
    Progress(OwnerGrantProgress),
    PublicationReview(Box<OwnerGrantPublicationReview>),
    PublicationProgress(OwnerGrantPublicationProgress),
    CancellationReview(Box<OwnerGrantCancellationReview>),
    Failed(String),
}
fn utc(ms: u64) -> String {
    i64::try_from(ms)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|date| date.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| crate::i18n::tr("Unavailable").into())
}
fn amount(value: Option<u64>) -> String {
    value
        .map(|n| format!("{} M", crate::utils::token_millions(n)))
        .unwrap_or_else(|| crate::i18n::tr("Unavailable").into())
}
fn publication_period_end() -> chrono::DateTime<chrono::Utc> {
    use chrono::Datelike;
    let first = chrono::Utc::now().date_naive().with_day(1).unwrap();
    (first + chrono::Duration::days(32)).with_day(1).unwrap().and_hms_opt(0,0,0).unwrap().and_utc()
}
fn publication_expiry(days: Option<i64>) -> String {
    let end = publication_period_end();
    days.map(|days| (chrono::Utc::now()+chrono::Duration::days(days)).min(end)).unwrap_or(end)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
fn publication_preset_available(days: Option<i64>) -> bool {
    days.is_none_or(|n| chrono::Utc::now()+chrono::Duration::days(n)<=publication_period_end())
}
/// What the native choose form may commit, from a new grant's form or from
/// an amendment or renewal form. The writer re-checks every bound natively.
#[derive(Clone, Debug, PartialEq)]
struct Bounds {
    /// Most tokens the grant may hold; None while capacity is unknown.
    remaining: Option<u64>,
    total_known: bool,
    /// Fewest tokens: 1, or what Agents carried into a successor already hold.
    minimum: u64,
    /// Applicants a successor must keep: they can be added, never removed.
    keep_managers: Vec<String>,
    /// The delegation must end after this instant (a renewal's month start)
    expiry_after_ms: u64,
    /// and no later than the end of the month being granted.
    expiry_until_ms: u64,
}
fn root_bounds(form: &OwnerGrantForm) -> Bounds {
    Bounds {
        remaining: form.context.remaining_tokens,
        total_known: form.context.resource_total_tokens.is_some(),
        minimum: 1,
        keep_managers: Vec::new(),
        expiry_after_ms: 0,
        expiry_until_ms: publication_period_end().timestamp_millis() as u64,
    }
}
fn renewal(form: &OwnerGrantSuccessionForm) -> bool {
    form.kind == OwnerGrantSuccessionKind::Renewal
}
fn successor_bounds(form: &OwnerGrantSuccessionForm) -> Bounds {
    Bounds {
        remaining: form.context.remaining_tokens,
        total_known: form.context.resource_total_tokens.is_some(),
        minimum: form.floor.retained_tokens.max(1),
        keep_managers: form.predecessor.eligible_managers.clone(),
        expiry_after_ms: if renewal(form) { form.period_start_ms } else { 0 },
        expiry_until_ms: form.period_end_ms,
    }
}
fn choices_valid(bounds: &Bounds, draft: &GrantChoices) -> bool {
    draft.clone().parse().is_ok_and(|choices| {
        bounds.total_known
            && bounds.remaining.is_some_and(|n| n >= choices.allocated_tokens)
            && choices.allocated_tokens >= bounds.minimum
            && bounds
                .keep_managers
                .iter()
                .all(|manager| choices.eligible_managers.contains(manager))
            && choices.delegation_expires_at_ms > bounds.expiry_after_ms
            && choices.delegation_expires_at_ms <= bounds.expiry_until_ms
    })
}
fn form_choices_valid(form: &OwnerGrantForm, draft: &GrantChoices) -> bool {
    choices_valid(&root_bounds(form), draft)
}
fn rfc3339(ms: u64) -> String {
    i64::try_from(ms)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|date| date.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}
/// The expiry an amendment or renewal preset fills in, if it is inside the
/// month being granted: `days` from now (capped at its end), or its end.
fn successor_preset(form: &OwnerGrantSuccessionForm, days: Option<i64>, now: u64) -> Option<String> {
    let bounds = successor_bounds(form);
    let value = match days {
        Some(days) => {
            let at = now.saturating_add(u64::try_from(days).ok()?.saturating_mul(86_400_000));
            if at > bounds.expiry_until_ms {
                return None;
            }
            at
        }
        None => bounds.expiry_until_ms,
    };
    (value > bounds.expiry_after_ms.max(now)).then(|| rfc3339(value))
}
/// A script proposal only seeds the native tokens field; the owner can edit it
/// and the committed value is always read back from that widget.
fn initial_draft(proposal: Option<u64>) -> GrantChoices {
    GrantChoices {
        tokens: proposal.map(|tokens| tokens.to_string()).unwrap_or_default(),
        ..GrantChoices::default()
    }
}
/// Defaults for an amendment or renewal: the current limit unless a proposal
/// filled the field, the current applicants plus any typed ones
/// (applicants are never removed), and the current delegation for an
/// amendment or the end of the new month for a renewal.
fn fill_successor_defaults(draft: &mut GrantChoices, form: &OwnerGrantSuccessionForm, now: u64) {
    if draft.tokens.is_empty() {
        draft.tokens = form.predecessor.allocated_tokens.to_string();
    }
    let mut managers: Vec<String> = form
        .predecessor
        .eligible_managers
        .iter()
        .cloned()
        .chain(draft.managers.split_whitespace().map(str::to_owned))
        .collect();
    managers.sort();
    managers.dedup();
    draft.managers = managers.join(" ");
    let current = form.predecessor.delegation_expires_at_ms;
    draft.expires_utc = if !renewal(form) && current > now && current <= form.period_end_ms {
        rfc3339(current)
    } else {
        rfc3339(form.period_end_ms)
    };
}
/// Defaults for fields that are still empty when the form opens. A proposal
/// is kept exactly, even above the available capacity: the form then shows it
/// and stays unconfirmable instead of silently lowering it.
fn fill_form_defaults(draft: &mut GrantChoices, form: &OwnerGrantForm) {
    if draft.tokens.is_empty() {
        draft.tokens = form.context.remaining_tokens.unwrap_or(0).min(1_000_000).to_string();
    }
    if draft.expires_utc.is_empty() {
        draft.expires_utc = publication_expiry(None);
    }
}
/// Whether a phase offers the native primary action. A proposal never commits
/// anything here: beside a published grant it can only open the native
/// amendment form (`amendable`), and only when the server supports it.
/// A grant whose UTC month ended can be followed by this month's grant.
fn primary_available(
    phase: &Phase,
    bounds: Option<&Bounds>,
    draft: &GrantChoices,
    has_original: bool,
    can_choose_new_limits: bool,
    amendable: bool,
    now: u64,
) -> bool {
    if ended_grant(phase, now) {
        return can_choose_new_limits;
    }
    if published_grant(phase, now) {
        return amendable && can_choose_new_limits && live_grant_tokens(phase, now).is_some();
    }
    match phase {
        Phase::Choose => bounds.is_some_and(|b| choices_valid(b, draft)),
        Phase::Review(review) => !review.expired() && review.capacity_available(),
        Phase::Progress(progress)
            if progress.closure.is_some() && matches!(progress.state, OwnerGrantState::Recorded) =>
        {
            can_choose_new_limits
        }
        Phase::Progress(progress) => {
            matches!(progress.state, OwnerGrantState::ReviewRequired | OwnerGrantState::Recorded)
                || (matches!(progress.state, OwnerGrantState::Declined) && can_choose_new_limits)
        }
        Phase::PublicationReview(review) => !review.expired(),
        Phase::CancellationReview(review) => !review.expired(),
        Phase::Failed(code) => {
            has_original
                && matches!(
                    code.as_str(),
                    "owner_grant_review_changed"
                        | "owner_grant_original_review_expired"
                        | "owner_grant_publication_review_expired"
                )
        }
        _ => false,
    }
}
/// A recorded grant whose UTC month has ended. The pool has no capacity from
/// it any more, and this month's grant may follow it on the same admission.
fn ended_grant(phase: &Phase, now: u64) -> bool {
    match phase {
        Phase::PublicationProgress(p) => month_ended(&p.summary, now),
        Phase::Progress(p) => {
            matches!(p.state, OwnerGrantState::Recorded)
                && p.closure.is_none()
                && month_ended(&p.summary, now)
        }
        _ => false,
    }
}
/// The server has accepted a grant for this pool's current admission and
/// month. It accepts only one per admission each month, even after
/// revocation, so a proposal is never applied here and nothing is revoked to
/// make room. Once that month has ended the grant no longer blocks a proposal.
fn published_grant(phase: &Phase, now: u64) -> bool {
    matches!(phase, Phase::PublicationProgress(p) if matches!(p.state, OwnerGrantPublicationState::ServerObserved))
        && !ended_grant(phase, now)
}
/// The pool's current per-request limit: the tokens of a grant the server last
/// reported as active whose month has started. Reserved, rejected, revoked,
/// superseded, expired or scheduled grants set none.
fn live_grant_tokens(phase: &Phase, now: u64) -> Option<u64> {
    match phase {
        Phase::PublicationProgress(p)
            if published_grant(phase, now)
                && p.server_state.as_deref() == Some("active")
                && p.summary.period_start_ms <= now =>
        {
            Some(p.summary.allocated_tokens)
        }
        _ => None,
    }
}
/// What the mini app learns when the panel closes. A proposal beside this
/// month's published grant on a server that cannot amend it was never
/// applied: say why. Otherwise the reply is the public progress shown.
fn close_reply(
    phase: &Phase,
    proposal: Option<u64>,
    in_flight: bool,
    original: Option<&str>,
    succession_supported: bool,
    now: u64,
) -> Result<String, String> {
    if proposal.is_some() && published_grant(phase, now) && !succession_supported {
        return Err("pool_grant_already_published".into());
    }
    match phase {
        Phase::Progress(p) => serde_json::to_string(p).map_err(|_| "management_outcome_unknown".to_owned()),
        Phase::PublicationProgress(p) => serde_json::to_string(p).map_err(|_| "management_outcome_unknown".to_owned()),
        _ => Ok(json!({"state":if in_flight {"outcome_unknown"} else {"review_closed"},"intentId":original}).to_string()),
    }
}
/// An unattempted original reserves nothing; cancelling it reopens new limits.
const PENDING_NOTE: &str = "The proposed limit is not applied to this pending reservation. To use it, cancel the pending reservation, then choose new limits.";
/// One grant per admission per UTC month, even after revocation, on a server
/// without succession support.
const PUBLISHED_NOTE: &str = "This pool already has a published grant for this month on its current admission, and the server accepts one grant per admission each month, so the proposed limit is not applied. Nothing was revoked or changed.";
/// Beside a published grant on a server that supports amendments.
const AMENDABLE_NOTE: &str = "Not applied yet. Choose Change per-request limit to review it. Running Agents keep their budgets.";
/// Beside a revoked, superseded or expired grant this month.
const NOT_AMENDABLE_NOTE: &str = "The proposed limit is not applied: this grant is no longer active, so it cannot be changed.";
/// Beside next month's scheduled renewal (Q7: cancel, amend, renew again).
const SCHEDULED_NOTE: &str = "The proposed limit is not applied: next month's renewal is already scheduled. To change this month's limit, cancel the renewal, change the limit, then renew again.";
/// A published grant whose month has not started: a scheduled renewal.
fn scheduled_grant(phase: &Phase, now: u64) -> bool {
    matches!(phase, Phase::PublicationProgress(p) if p.summary.period_start_ms > now)
        && published_grant(phase, now)
}
/// The native successor terms shown above the form while choosing: the
/// limit change, the Agents that keep running and their floor, and what the
/// new limit means for pending requests.
fn successor_notice(form: &OwnerGrantSuccessionForm, draft_tokens: &str) -> String {
    let new = amount(draft_tokens.trim().parse::<u64>().ok());
    let current = amount(Some(form.predecessor.allocated_tokens));
    let held = amount(Some(form.floor.retained_tokens));
    let count = form.floor.children.to_string();
    let lines = if renewal(form) {
        vec![
            crate::i18n::format(
                "Renew for {month}: per-request limit {current} → {new}",
                &[("month", form.period_key.clone()), ("current", current), ("new", new)],
            ),
            crate::i18n::format(
                "Agents that continue: {count}. They hold {held} (minimum).",
                &[("count", count), ("held", held)],
            ),
            crate::i18n::format("Takes effect {start}.", &[("start", utc(form.period_start_ms))]),
        ]
    } else {
        vec![
            crate::i18n::format(
                "Per-request limit: {current} → {new} (applies now, until {end})",
                &[("current", current), ("new", new.clone()), ("end", utc(form.period_end_ms))],
            ),
            crate::i18n::format(
                "Agents that keep running: {count}. The limit cannot go below {held}, which Agents already hold this month.",
                &[("count", count), ("held", held)],
            ),
            crate::i18n::format(
                "Pending requests above {new} cannot be approved.",
                &[("new", new)],
            ),
        ]
    };
    lines.join("\n")
}
/// Shown for a grant whose month ended: the pool is closed until the owner
/// publishes this month's grant, which needs last month's Agents settled.
const ENDED_STATUS: &str = "This grant's UTC month has ended, so the pool has no capacity. Choose this month's limits to publish it again. Last month's Agents must first stop and report their final usage, which happens only while this device is online after the month ends (00:00 UTC).";
/// This month's form cannot show capacity while last month's usage is unknown.
const REGRANT_UNKNOWN_STATUS: &str = "Available capacity is unknown until last month's Agents stop and report their final usage. Keep this device online, then refresh.";
/// Native text for a script's proposed per-request limit: the current limit,
/// the amount this screen would commit, and why a phase does not use it.
fn proposal_notice(
    phase: &Phase,
    proposal: Option<u64>,
    draft_tokens: &str,
    succession_supported: bool,
    now: u64,
) -> Option<String> {
    let proposal = proposal?;
    let (new, note) = match phase {
        Phase::Loading => return None,
        // The form commits only what its tokens field holds now.
        Phase::Choose => (draft_tokens.trim().parse::<u64>().ok(), None),
        phase if ended_grant(phase, now) => (
            Some(proposal),
            Some("The proposed limit is filled in when you choose this month's limits."),
        ),
        phase if published_grant(phase, now) && !succession_supported => {
            (Some(proposal), Some(PUBLISHED_NOTE))
        }
        phase if scheduled_grant(phase, now) => (Some(proposal), Some(SCHEDULED_NOTE)),
        phase if published_grant(phase, now) && live_grant_tokens(phase, now).is_some() => {
            (Some(proposal), Some(AMENDABLE_NOTE))
        }
        phase if published_grant(phase, now) => (Some(proposal), Some(NOT_AMENDABLE_NOTE)),
        Phase::Progress(p)
            if matches!(p.state, OwnerGrantState::Declined)
                || (matches!(p.state, OwnerGrantState::Recorded) && p.closure.is_some()) =>
        {
            (Some(proposal), Some("The proposed limit is filled in when you choose new limits."))
        }
        Phase::Progress(p)
            if matches!(p.state, OwnerGrantState::ReviewRequired | OwnerGrantState::ReviewExpired) =>
        {
            (Some(proposal), Some(PENDING_NOTE))
        }
        // Reviews of an existing, unattempted original; submitting the form
        // clears the proposal before its own review appears.
        Phase::Review(_) | Phase::CancellationReview(_) => (Some(proposal), Some(PENDING_NOTE)),
        Phase::Failed(_) => (Some(proposal), Some("The proposed limit was not applied.")),
        _ => (
            Some(proposal),
            Some("The proposed limit is not applied to this existing grant. It keeps the amounts shown below."),
        ),
    };
    let current = live_grant_tokens(phase, now)
        .map(|tokens| amount(Some(tokens)))
        .unwrap_or_else(|| crate::i18n::tr("None active").into());
    let mut text = crate::i18n::format(
        "Per-request limit: {current} → {new}",
        &[("current", current), ("new", amount(new))],
    );
    if let Some(note) = note {
        text.push('\n');
        text.push_str(crate::i18n::tr(note));
    }
    Some(text)
}
fn same_publication_resource(form: &OwnerGrantForm, review: &OwnerGrantReview) -> bool {
    let c=review.context();let s=review.summary();
    form.contribution_id==s.contribution_id && form.resource_id==s.resource_id
        && form.context.server_origin==c.server_origin && form.context.profile_id==c.profile_id
        && form.context.resource_revision==c.resource_revision
        && form.context.framework==c.framework && form.context.model==c.model
}
/// The successor review continues one owner submission only if it is the
/// change of the same resource, kind and predecessor the owner was shown.
fn same_successor_resource(form: &OwnerGrantSuccessionForm, review: &OwnerGrantReview) -> bool {
    let c=review.context();let s=review.summary();
    form.contribution_id==s.contribution_id && form.resource_id==s.resource_id
        && form.context.server_origin==c.server_origin && form.context.profile_id==c.profile_id
        && form.context.resource_revision==c.resource_revision
        && form.context.framework==c.framework && form.context.model==c.model
        && s.succession.as_ref().is_some_and(|succession| succession.kind==form.kind
            && succession.predecessor_grant_id==form.predecessor.grant_id)
}
/// Only exact reviewed native codes may enter diagnostics. A prefix match would
/// allow arbitrary provider errors, paths or credentials to escape into logs.
fn grant_failure_code(code: &str) -> &'static str {
    match code {
        "management_access_required" => "management_access_required",
        "management_admission_receipts_required" => "management_admission_receipts_required",
        "management_features_invalid" => "management_features_invalid",
        "owner_grant_prior_intent_required" => "owner_grant_prior_intent_required",
        "owner_grant_new_admission_required" => "owner_grant_new_admission_required",
        "management_monthly_regrant_required" => "management_monthly_regrant_required",
        "management_backing_changed" => "management_backing_changed",
        "management_capacity" => "management_capacity",
        "management_drain_outcome_unknown" => "management_drain_outcome_unknown",
        "management_drain_unavailable" => "management_drain_unavailable",
        "management_enrollment_changed" => "management_enrollment_changed",
        "management_enrollment_history_full" => "management_enrollment_history_full",
        "management_enrollment_history_required" => "management_enrollment_history_required",
        "management_model_unqualified" => "management_model_unqualified",
        "management_not_found" => "management_not_found",
        "management_outcome_unknown" => "management_outcome_unknown",
        "management_profile_mismatch" => "management_profile_mismatch",
        "management_resource_in_use" => "management_resource_in_use",
        "management_revision_changed" => "management_revision_changed",
        "management_revision_conflict" => "management_revision_conflict",
        "management_scope_changed" => "management_scope_changed",
        "management_state_unavailable" => "management_state_unavailable",
        "management_storage_failed" => "management_storage_failed",
        "management_unavailable" => "management_unavailable",
        "management_permission_required" => "management_permission_required",
        "management_session_required" => "management_session_required",
        "management_command_conflict" => "management_command_conflict",
        "management_receipt_capacity" => "management_receipt_capacity",
        "native_grant_not_publishable" => "native_grant_not_publishable",
        "native_grant_receipt_required" => "native_grant_receipt_required",
        "native_owner_state_unavailable" => "native_owner_state_unavailable",
        "native_random_unavailable" => "native_random_unavailable",
        "owner_authorization_required" => "owner_authorization_required",
        "owner_grant_already_attempted" => "owner_grant_already_attempted",
        "owner_grant_binding_changed" => "owner_grant_binding_changed",
        "owner_grant_capacity_unavailable" => "owner_grant_capacity_unavailable",
        "owner_grant_history_full" => "owner_grant_history_full",
        "owner_grant_history_unavailable" => "owner_grant_history_unavailable",
        "owner_grant_intent_missing" => "owner_grant_intent_missing",
        "owner_grant_original_intent_required" => "owner_grant_original_intent_required",
        "owner_grant_original_review_expired" => "owner_grant_original_review_expired",
        "owner_grant_publication_confirmation_required" => {
            "owner_grant_publication_confirmation_required"
        }
        "owner_grant_publication_history_unavailable" => {
            "owner_grant_publication_history_unavailable"
        }
        "owner_grant_publication_review_changed" => "owner_grant_publication_review_changed",
        "owner_grant_publication_review_expired" => "owner_grant_publication_review_expired",
        "owner_grant_review_changed" => "owner_grant_review_changed",
        "owner_source_adoption_required" => "owner_source_adoption_required",
        "provider_account_changed" => "provider_account_changed",
        "provider_closing" => "provider_closing",
        "provider_directory_unavailable" => "provider_directory_unavailable",
        "provider_identity_timeout" => "provider_identity_timeout",
        "provider_identity_unavailable" => "provider_identity_unavailable",
        "provider_profile_mismatch" => "provider_profile_mismatch",
        "local_codex_account_not_signed_in" => "local_codex_account_not_signed_in",
        "local_provider_not_signed_in" => "local_provider_not_signed_in",
        "contribution_not_admitted" => "contribution_not_admitted",
        "contribution_executor_changed" => "contribution_executor_changed",
        "contribution_resource_changed" => "contribution_resource_changed",
        "allocation_enrollment_changed" => "allocation_enrollment_changed",
        "invalid_grant_budget" => "invalid_grant_budget",
        "invalid_grant_expiry" => "invalid_grant_expiry",
        "invalid_grant_manager" => "invalid_grant_manager",
        "management_grant_succession_required" => "management_grant_succession_required",
        "pool_grant_succession_unavailable" => "pool_grant_succession_unavailable",
        "owner_grant_predecessor_required" => "owner_grant_predecessor_required",
        "owner_grant_succession_pending" => "owner_grant_succession_pending",
        "owner_grant_successor_exists" => "owner_grant_successor_exists",
        "owner_grant_renewal_window_closed" => "owner_grant_renewal_window_closed",
        "owner_grant_below_floor" => "owner_grant_below_floor",
        "owner_grant_manager_removed" => "owner_grant_manager_removed",
        "grant_predecessor_not_effective" => "grant_predecessor_not_effective",
        "grant_successor_exists" => "grant_successor_exists",
        "grant_successor_below_reserved" => "grant_successor_below_reserved",
        "grant_succession_window_closed" => "grant_succession_window_closed",
        "invalid_grant_succession" => "invalid_grant_succession",
        _ => "owner_grant_unclassified_failure",
    }
}
fn failure_message(code: &str) -> &'static str {
    match code {
        "owner_grant_original_review_expired" | "owner_grant_publication_review_expired" => {
            "This review expired. Review the original intent again."
        }
        "native_grant_not_publishable" => {
            "The local reservation is no longer eligible for publication. Its original receipt remains available for recovery."
        }
        "owner_grant_publication_history_unavailable" => {
            "Original publication history is incomplete. No replacement command can be created; the original local receipt remains available."
        }
        "owner_grant_publication_review_changed" => {
            "The server contribution changed. The original publication cannot silently adopt a new revision."
        }
        "owner_grant_review_changed" => {
            "This review changed or expired. Check the original intent again."
        }
        "management_enrollment_changed" | "contribution_not_admitted" => {
            "This resource admission changed. Check the original intent; an unattempted reservation can still be cancelled."
        }
        "owner_grant_prior_intent_required" => {
            "Resolve previous grant reviews and close earlier capacity grants before requesting a new grant."
        }
        "management_admission_receipts_required" => {
            "The server is not accepting grants for this admission. Check with its administrator."
        }
        "management_monthly_regrant_required" => {
            "This server does not accept a new month's grant on the same admission yet. Ask its administrator to update the server."
        }
        "management_features_invalid" => {
            "Server support for this grant could not be verified. Refresh the connection and try again."
        }
        "management_scope_changed" | "management_access_required" => {
            "The account or session changed. Open a fresh review after signing in."
        }
        "invalid_grant_expiry" => "Enter an explicit UTC expiry, for example 2027-01-01T00:00:00Z.",
        "owner_grant_new_admission_required" => {
            "Request a new resource admission and wait for approval before reserving again."
        }
        "owner_grant_capacity_unavailable" => {
            "Available capacity is unknown or insufficient. This review cannot be confirmed."
        }
        "invalid_grant_budget" => "Enter a positive token amount and at least one Agent applicant.",
        "invalid_grant_manager" => {
            "Enter complete Matrix IDs for the Agent applicants."
        }
        "management_grant_succession_required" | "pool_grant_succession_unavailable" => {
            "This server cannot change or renew a published pool yet. Ask its administrator to update the server."
        }
        "owner_grant_predecessor_required" | "grant_predecessor_not_effective" => {
            "This pool has no active published limit to change. Publish the pool first."
        }
        "owner_grant_succession_pending" => {
            "A change to this pool is not finished. Publish, recover or cancel it first."
        }
        "owner_grant_successor_exists" | "grant_successor_exists" => {
            "Next month's renewal is already scheduled. Cancel it, change this month's limit, then renew again."
        }
        "owner_grant_renewal_window_closed" | "grant_succession_window_closed" => {
            "Next month's limit can be scheduled only during the last 7 days of this month (UTC)."
        }
        "owner_grant_below_floor" | "grant_successor_below_reserved" => {
            "The new limit is below what running Agents already hold. Choose at least the minimum shown."
        }
        "owner_grant_manager_removed" => {
            "Applicants can be added but not removed while Agents may be running. Keep every current applicant."
        }
        _ => {
            "The result is not confirmed. Recover the original intent before trying another allocation."
        }
    }
}
impl GrantPrompt {
    fn actionable(&self) -> bool {
        self.receiver.is_none() && self.flow.active()
    }
    /// The owner submits the terms shown natively for reservation and
    /// publication; a script proposal plays no further part in this window.
    fn submit(&mut self) {
        self.submit_publication = true;
        self.proposal = None;
    }
    /// The bounds of whichever native form the Choose phase shows.
    fn bounds(&self) -> Option<Bounds> {
        match (&self.successor, &self.form) {
            (Some(form), _) => Some(successor_bounds(form)),
            (None, Some(form)) => Some(root_bounds(form)),
            _ => None,
        }
    }
    /// A proposal beside this month's published grant may open the native
    /// amendment form, but only once the server is known to support it.
    fn amendable(&self) -> bool {
        self.proposal.is_some() && self.flow.succession_supported() == Some(true)
    }
    fn can_primary(&self) -> bool {
        self.actionable()
            && primary_available(
                &self.phase,
                self.bounds().as_ref(),
                &self.draft,
                self.original.is_some(),
                self.flow.can_choose_new_limits(),
                self.amendable(),
                now_ms(),
            )
    }
    fn can_recover(&self) -> bool {
        self.actionable()
            && !matches!(self.phase, Phase::Loading)
            && !matches!(&self.phase, Phase::Review(review) if review.capacity_available())
            && !matches!(&self.phase, Phase::PublicationReview(review) if !review.expired())
            && !matches!(&self.phase, Phase::CancellationReview(review) if !review.expired())
    }
    fn can_cancel_intent(&self) -> bool {
        self.actionable()
            && self.original.is_some()
            && !self.publishing
            && !self.cancelling
            && match &self.phase {
                Phase::Review(_) | Phase::Failed(_) => true,
                Phase::Progress(p) => matches!(
                    p.state,
                    OwnerGrantState::ReviewRequired | OwnerGrantState::ReviewExpired
                ),
                _ => false,
            }
    }
}
/// The panel's own argument for the native successor kind.
fn succession_arg(kind: OwnerGrantSuccessionKind) -> Succession {
    match kind {
        OwnerGrantSuccessionKind::Renewal => Succession::Renewal,
        _ => Succession::Amendment,
    }
}
/// Review rows that say what a successor changes and what it carries.
fn succession_details(
    s: &::hagency::native_owner::OwnerGrantSuccessionSummary,
    allocated_tokens: u64,
) -> String {
    let tr = crate::i18n::tr;
    [
        (
            "Kind of change",
            tr(match s.kind {
                OwnerGrantSuccessionKind::Renewal => "Renewal for next month",
                _ => "New per-request limit this month",
            })
            .to_owned(),
        ),
        (
            "Per-request limit",
            format!(
                "{} → {}",
                amount(Some(s.predecessor_allocated_tokens)),
                amount(Some(allocated_tokens))
            ),
        ),
        ("Running Agents carried", s.carried_children.to_string()),
        ("Held by running Agents (minimum)", amount(Some(s.carried_retained_tokens))),
    ]
    .into_iter()
    .map(|(label, value)| format!("{}\n{}", tr(label), value))
    .collect::<Vec<_>>()
    .join("\n\n")
}
fn cancellation_details(server: &str, s: OwnerGrantSummary) -> String {
    let tr = crate::i18n::tr;
    [
        ("Matrix server", server.to_owned()),
        ("Resource", s.resource_id),
        ("Tokens to reserve", format!("{} M", crate::utils::token_millions(s.allocated_tokens))),
        (
            "Accounting period (UTC)",
            format!(
                "{}\n{} → {}",
                s.period_key,
                utc(s.period_start_ms),
                utc(s.period_end_ms)
            ),
        ),
        ("Recorded coordinator", s.coordinator_mxid),
        ("Agent applicants", s.eligible_managers.join("\n")),
        ("Original intent", s.intent_id),
    ]
    .into_iter()
    .map(|(label, value)| format!("{}\n{}", tr(label), value))
    .collect::<Vec<_>>()
    .join("\n\n")
}
impl MiniAppsPanel {
    pub(super) fn begin_grant_confirmation(
        &mut self,
        cx: &mut Cx,
        args: Value,
        reply: (usize, u64),
    ) -> Result<(), String> {
        if self.execution_confirmation.is_some()
            || self.grant_confirmation.is_some()
            || self.adoption_confirmation.is_some()
            || self.provider_confirmation.is_some()
            || self.account_login.is_some()
            || self.reauthentication.is_some()
            || self.project_room_reply.is_some()
            || self.pending.len() >= 16
        {
            return Err("provider_confirmation_busy".into());
        }
        let host = self.hagency.as_ref().ok_or("management_session_required")?;
        let flow = host.grant_handoff(
            self.lease.clone().ok_or("management_session_required")?,
            args,
        )?;
        let proposal = flow.proposed_tokens();
        self.grant_confirmation = Some(GrantPrompt {
            flow: flow.clone(),
            reply,
            phase: Phase::Loading,
            original: None,
            receiver: None,
            draft: initial_draft(proposal),
            publishing: false,
            cancelling: false,
            form: None,
            submit_publication: false,
            proposal,
            regrant: false,
            successor: None,
        });
        self.grant_job(async move { flow.open().await });
        self.present_grant_confirmation(cx);
        cx.set_key_focus(
            self.view
                .button(cx, ids!(grant_confirmation.buttons.cancel))
                .area(),
        );
        Ok(())
    }
    fn grant_job(
        &mut self,
        task: impl std::future::Future<Output = Result<GrantResult, String>> + Send + 'static,
    ) {
        let Some(prompt) = &mut self.grant_confirmation else {
            return;
        };
        let (tx, receiver) = mpsc::sync_channel(1);
        prompt.receiver = Some(receiver);
        prompt.phase = Phase::Loading;
        crate::sliding_sync::spawn_async_task(async move {
            let result = task.await;
            if let Err(code) = &result {
                let diagnostic = grant_failure_code(code);
                makepad_widgets::warning!("Native owner grant operation failed: {diagnostic}");
            }
            let _ = tx.try_send(result);
            SignalToUI::set_ui_signal();
        });
    }
    pub(super) fn present_grant_confirmation(&mut self, cx: &mut Cx) {
        let visible = self.grant_confirmation.is_some();
        self.view
            .view(cx, ids!(grant_confirmation))
            .set_visible(cx, visible);
        let Some(prompt) = &self.grant_confirmation else {
            return;
        };
        let now = now_ms();
        self.view.view(cx, ids!(app_content)).set_visible(cx, false);
        self.view.view(cx, ids!(approval)).set_visible(cx, false);
        self.view
            .label(cx, ids!(grant_confirmation.scroll.owner))
            .set_text(cx, prompt.flow.account());
        let choosing = matches!(prompt.phase, Phase::Choose);
        if choosing {
            for (id,days) in [(ids!(grant_confirmation.scroll.choices.expiry_presets.day),Some(1)),(ids!(grant_confirmation.scroll.choices.expiry_presets.week),Some(7)),(ids!(grant_confirmation.scroll.choices.expiry_presets.month),None)] {
                let available=match &prompt.successor {
                    Some(form)=>successor_preset(form,days,now).is_some(),
                    None=>publication_preset_available(days),
                };
                self.view.button(cx,id).set_disabled(cx,!available);
            }
            let widget=self.view.widget(cx,ids!(grant_confirmation.scroll.choices.tokens));
            if let Some(mut editor)=widget.borrow_mut::<crate::miniapps::design::MiniAppNumberInput>() {
                editor.set_minimum(cx,prompt.bounds().map_or(1,|b|b.minimum));
            }
            if let Some(form)=&prompt.successor {
                let widget=self.view.widget(cx,ids!(grant_confirmation.scroll.choices.tokens));
                if let Some(mut editor)=widget.borrow_mut::<crate::miniapps::design::MiniAppNumberInput>() {
                    editor.set_maximum(cx,form.context.remaining_tokens.unwrap_or(0));
                }
                let available=if renewal(form) {
                    crate::i18n::format("Available in {month}",&[("month",form.period_key.clone())])
                } else {crate::i18n::tr("Available for this month's limit").into()};
                self.view.label(cx,ids!(grant_confirmation.scroll.choices.capacity)).set_text(cx,
                    &format!("{} · {}\n{}\n{}: {} M",form.label,form.context.model,form.context.server_origin,
                        available,form.context.remaining_tokens.map(crate::utils::token_millions)
                            .unwrap_or_else(||crate::i18n::tr("Unavailable").into())));
            }
            if let Some(form)=prompt.form.as_ref().filter(|_|prompt.successor.is_none()) {
                let widget=self.view.widget(cx,ids!(grant_confirmation.scroll.choices.tokens));
                if let Some(mut editor)=widget.borrow_mut::<crate::miniapps::design::MiniAppNumberInput>() {
                    editor.set_maximum(cx,form.context.remaining_tokens.unwrap_or(0));
                }
                self.view.label(cx,ids!(grant_confirmation.scroll.choices.capacity)).set_text(cx,
                    &format!("{} · {}\n{}\n{}: {} M",form.label,form.context.model,form.context.server_origin,
                        crate::i18n::tr("Available to publish"),form.context.remaining_tokens.map(crate::utils::token_millions)
                            .unwrap_or_else(||crate::i18n::tr("Unavailable").into())));
            }
            for (id, value) in [
                (
                    ids!(grant_confirmation.scroll.choices.managers),
                    &prompt.draft.managers,
                ),
                (
                    ids!(grant_confirmation.scroll.choices.tokens),
                    &prompt.draft.tokens,
                ),
                (
                    ids!(grant_confirmation.scroll.choices.expires),
                    &prompt.draft.expires_utc,
                ),
            ] {
                self.view.widget(cx, id).set_text(cx, value);
            }

        }

        self.view
            .view(cx, ids!(grant_confirmation.scroll.choices))
            .set_visible(cx, choosing);
        self.view
            .view(cx, ids!(grant_confirmation.scroll.details))
            .set_visible(cx, !choosing);
        let mut status: String;
        let mut details = String::new();
        let mut primary = if prompt.cancelling {"Review cancellation"} else {"Publish resource pool"};
        let tr = crate::i18n::tr;
        let succession_supported = prompt.flow.succession_supported() == Some(true);
        let mut notice = proposal_notice(&prompt.phase, prompt.proposal, &prompt.draft.tokens, succession_supported, now);
        match &prompt.phase {
            Phase::Loading=>status=tr(if prompt.submit_publication {"Publishing resource pool…"} else {"Loading publication status…"}).into(),
            Phase::Choose if prompt.successor.is_some()=>{
                let form=prompt.successor.as_ref().unwrap();
                primary=if renewal(form) {"Publish renewal"} else {"Publish new limit"};
                notice=Some(successor_notice(form,&prompt.draft.tokens));
                status=if form.context.remaining_tokens.is_none() || form.context.resource_total_tokens.is_none() {
                    tr("Available capacity is unknown or insufficient. This review cannot be confirmed.").into()
                } else if renewal(form) {
                    tr("Choose next month's limit and applicants. Running Agents continue into the new month with the budgets they have; applicants can be added but not removed.").into()
                } else {
                    tr("Choose this month's new per-request limit and applicants. It applies as soon as the server accepts it. Running Agents keep their budgets; applicants can be added but not removed.").into()
                };
            }
            Phase::Choose=>status=tr(if prompt.form.as_ref().is_some_and(|f| f.context.remaining_tokens.is_some_and(|n|n>0)) {
                "Set who can apply, the token budget and expiry. Publish once to make this capacity available on the server."
            } else if prompt.regrant && prompt.form.as_ref().is_some_and(|f| f.context.remaining_tokens.is_none()) {
                REGRANT_UNKNOWN_STATUS
            } else {"Available capacity is unknown or insufficient. This review cannot be confirmed."}).into(),
            Phase::Review(review)=> {
                primary="Publish resource pool";
                let s=review.summary();let c=review.context();
                let rows=[("Matrix server",c.server_origin.clone()),("Native profile",c.profile_id.clone()),
                    ("Resource",format!("{} · {}\n{}",c.framework,c.model,s.resource_id)),
                    ("Remaining / total tokens",format!("{} / {}",amount(c.remaining_tokens),amount(c.resource_total_tokens))),
                    ("Shared account limit",amount(c.shared_account_total_tokens)),
                    ("Tokens to reserve",format!("{} M", crate::utils::token_millions(s.allocated_tokens))),
                    ("Accounting period (UTC)",format!("{}\n{} → {}",s.period_key,utc(s.period_start_ms),utc(s.period_end_ms))),
                    ("Recorded coordinator",s.coordinator_mxid.clone()),("Agent applicants",s.eligible_managers.join("\n")),
                    ("Delegation expires (UTC)",utc(s.delegation_expires_at_ms)),("Original intent",s.intent_id.clone()),
                    ("Observed at",utc(c.observed_at_ms)),("Resource revision",c.resource_revision.clone())];
                details=rows.into_iter().map(|(label,value)|format!("{}\n{}",tr(label),value)).collect::<Vec<_>>().join("\n\n");
                status=tr(if review.capacity_available() {"Publish these saved limits to the server. Capacity is checked automatically during publication."} else {"Available capacity is unknown or insufficient. This review cannot be confirmed."}).into();
                if let Some(succession)=&s.succession {
                    details=format!("{}\n\n{}",succession_details(succession,s.allocated_tokens),details);
                    if review.capacity_available() {
                        status=tr(match succession.kind {
                            OwnerGrantSuccessionKind::Renewal=>"Publish this renewal. It takes effect when the new month starts; running Agents continue.",
                            _=>"Publish this change. The new limit applies as soon as the server accepts it; running Agents keep their budgets.",
                        }).into();
                    }
                }
            },
            Phase::Progress(p)=>{
                if matches!(p.state,OwnerGrantState::Recorded) { primary="Publish resource pool"; }
                if matches!(p.state,OwnerGrantState::Declined) { primary="Choose new limits"; }
                details=cancellation_details("",p.summary.clone());
                if let Some(succession)=&p.summary.succession {
                    details=format!("{}\n\n{}",succession_details(succession,p.summary.allocated_tokens),details);
                }
                status=tr(match p.state {OwnerGrantState::Recorded if p.summary.succession.is_some()=>"This change is saved on this device. Publish it to apply it on the server.",
                    OwnerGrantState::Recorded=>"Capacity is reserved. Publish these saved limits to make the pool available on the server.",
                    OwnerGrantState::ReviewRequired=>"The original intent is waiting for a fresh review.",
                    OwnerGrantState::OutcomeUnknown=>"The outcome is unknown. Recover this original intent; do not create a replacement.",
                    OwnerGrantState::Declined=>"This pending reservation was closed without reserving capacity. Its original record is kept.",OwnerGrantState::ReviewExpired=>"The original accounting period ended. This intent cannot move to a new month."}).into();
                if let Some(closure) = &p.closure {
                    primary="Review new admission";
                    status=tr("Previous grant closed. Retained amounts remain committed; a new grant requires a separately approved admission.").into();
                    let rows=[("Retained tokens",format!("{} M", crate::utils::token_millions(closure.retained_tokens))),
                        ("Confirmed usage",format!("{} M", crate::utils::token_millions(closure.known_consumed_tokens))),
                        ("Unresolved tokens",format!("{} M", crate::utils::token_millions(closure.unresolved_tokens))),
                        ("Accounting period (UTC)",format!("{}\n{} → {}",p.summary.period_key,utc(p.summary.period_start_ms),utc(p.summary.period_end_ms))),
                        ("Original intent",p.summary.intent_id.clone()),("Observed at",utc(closure.observed_at_ms))];
                    details=rows.into_iter().map(|(label,value)|format!("{}\n{}",tr(label),value)).collect::<Vec<_>>().join("\n\n");
                }
            },
            Phase::CancellationReview(review)=>{
                primary="Cancel pending reservation";
                details=cancellation_details(review.server_origin(),review.summary());
                status=tr("Cancel this unattempted reservation? This closes only the pending local intent. The resource, server admission and previous grants stay unchanged. Closing this review keeps the intent pending.").into();
            },
            Phase::PublicationReview(review)=>{
                primary="Publish resource pool";
                let s=review.summary();
                let rows=[("Matrix server",review.server_origin().to_owned()),
                    ("Resource",s.resource_id), ("Tokens reserved",format!("{} M", crate::utils::token_millions(s.allocated_tokens))),
                    ("Accounting period (UTC)",format!("{}\n{} → {}",s.period_key,utc(s.period_start_ms),utc(s.period_end_ms))),
                    ("Recorded coordinator",s.coordinator_mxid),("Agent applicants",s.eligible_managers.join("\n")),
                    ("Delegation expires (UTC)",utc(s.delegation_expires_at_ms)),
                    ("Original intent",s.intent_id),
                    ("Contribution revision",review.contribution_revision().to_string()),
                    ("Observed at",utc(review.observed_at_ms()))];
                details=rows.into_iter().map(|(label,value)|format!("{}\n{}",tr(label),value)).collect::<Vec<_>>().join("\n\n");
                status=tr("Publish this existing reservation to the shown server with these owner and manager limits. This does not reserve more tokens or start an agent.").into();
            },
            Phase::PublicationProgress(p)=>{
                details=cancellation_details("",p.summary.clone());
                if let Some(succession)=&p.summary.succession {
                    details=format!("{}\n\n{}",succession_details(succession,p.summary.allocated_tokens),details);
                }
                details.push_str(&format!("\n\n{}\n{}",tr("Expires (UTC)"),utc(p.summary.delegation_expires_at_ms.min(p.summary.period_end_ms))));
                status=tr(match p.state {
                    OwnerGrantPublicationState::OutcomeUnknown=>"Publication is not confirmed. Recover the original command. This action does not change the local reservation.",
                    OwnerGrantPublicationState::Rejected=>"This original publication was rejected. The operation did not change the local reservation or create a replacement.",
                    OwnerGrantPublicationState::ServerObserved=>"The server state below is the latest observation. Recovery does not renew or reactivate this grant.",
                }).into();
                if let Some(server)=&p.server_state {
                    details.push_str(&format!("\n\n{}\n{}\n\n{}\n{}",tr("Server grant state"),tr(match server.as_str(){"active" if p.summary.period_start_ms>now=>"Scheduled","active"=>"Active","revoked"=>"Revoked","expired"=>"Expired","superseded"=>"Replaced by a newer limit",_=>"Unavailable"}),tr("Observed at"),p.observed_at_ms.map(utc).unwrap_or_default()));
                }
            },
            Phase::Failed(code)=>status=tr(failure_message(code)).into(),
        }
        if ended_grant(&prompt.phase, now) {
            primary = "Choose this month's limits";
            status = tr(ENDED_STATUS).into();
        }
        if published_grant(&prompt.phase, now) && prompt.amendable() && live_grant_tokens(&prompt.phase, now).is_some() {
            primary = "Change per-request limit";
        }
        if !prompt.flow.active() {
            status =
                tr("The account or session changed. Open a fresh review after signing in.").into();
        }
        self.view
            .label(cx, ids!(grant_confirmation.scroll.status))
            .set_text(cx, &status);
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.change))
            .set_visible(cx, notice.is_some());
        self.view
            .label(cx, ids!(grant_confirmation.scroll.change))
            .set_text(cx, notice.as_deref().unwrap_or_default());
        self.view
            .label(cx, ids!(grant_confirmation.scroll.details.copy))
            .set_text(cx, &details);
        self.view
            .button(cx, ids!(grant_confirmation.buttons.primary))
            .set_text(cx, tr(primary));
        self.view
            .widget(cx, ids!(grant_confirmation.buttons.primary))
            .set_visible(cx, choosing || prompt.can_primary());
        self.view.button(cx, ids!(grant_confirmation.buttons.primary)).set_disabled(cx,!prompt.can_primary());
        self.view
            .button(cx, ids!(grant_confirmation.buttons.recover))
            .set_text(
                cx,
                tr(if prompt.original.is_some() {
                    "Recover original"
                } else {
                    "Refresh"
                }),
            );
        self.view
            .widget(cx, ids!(grant_confirmation.buttons.recover))
            .set_visible(cx, prompt.can_recover());
        self.view
            .widget(cx, ids!(grant_confirmation.buttons.cancel_intent))
            .set_visible(cx, prompt.can_cancel_intent());
        self.request_confirmation_redraw(cx);
    }
    pub(super) fn poll_grant_confirmation(&mut self, cx: &mut Cx) {
        let Some(prompt) = &mut self.grant_confirmation else {
            return;
        };
        let result = prompt
            .receiver
            .as_ref()
            .and_then(|receiver| match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("management_outcome_unknown".into()))
                }
                Err(_) => None,
            });
        let mut changed = false;
        if let Some(result) = result {
            prompt.receiver = None;
            prompt.phase = match result {
                Ok(GrantResult::Choose(form)) => {
                    prompt.original = None;
                    prompt.publishing = false;
                    prompt.cancelling = false;
                    prompt.submit_publication = false;
                    prompt.successor = None;
                    fill_form_defaults(&mut prompt.draft, &form);
                    prompt.draft.coordinator=prompt.flow.account().into();
                    prompt.form=Some(form);
                    Phase::Choose
                }
                Ok(GrantResult::ChooseSuccessor(form)) => {
                    prompt.original = None;
                    prompt.publishing = false;
                    prompt.cancelling = false;
                    prompt.submit_publication = false;
                    fill_successor_defaults(&mut prompt.draft, &form, now_ms());
                    prompt.draft.coordinator=prompt.flow.account().into();
                    prompt.form=None;
                    prompt.successor=Some(*form);
                    Phase::Choose
                }
                Ok(GrantResult::Review(review)) => {
                    prompt.original = Some(review.summary().intent_id);
                    prompt.cancelling = false;
                    let same=match (&prompt.successor,&prompt.form) {
                        (Some(form),_)=>same_successor_resource(form,&review),
                        (None,Some(form))=>same_publication_resource(form,&review),
                        _=>true,
                    };
                    if prompt.submit_publication && !same {
                        prompt.submit_publication=false;
                    }
                    Phase::Review(review)
                }
                Ok(GrantResult::Progress(progress)) => {
                    prompt.original = Some(progress.summary.intent_id.clone());
                    prompt.cancelling = false;
                    Phase::Progress(*progress)
                }
                Ok(GrantResult::CancellationReview(review)) => {
                    prompt.original = Some(review.summary().intent_id);
                    prompt.cancelling = true;
                    Phase::CancellationReview(review)
                }
                Ok(GrantResult::PublicationReview(review)) => {
                    prompt.original = Some(review.summary().intent_id);
                    prompt.publishing = true;
                    Phase::PublicationReview(review)
                }
                Ok(GrantResult::PublicationProgress(progress)) => {
                    prompt.original = Some(progress.summary.intent_id.clone());
                    prompt.publishing = true;
                    Phase::PublicationProgress(*progress)
                }
                Err(code) => Phase::Failed(code),
            };
            changed = true;
        }
        if matches!(&prompt.phase,Phase::Review(review) if review.expired()) {
            prompt.phase = Phase::Failed("owner_grant_original_review_expired".into());
            changed = true;
        }
        if matches!(&prompt.phase,Phase::PublicationReview(review) if review.expired()) {
            prompt.phase = Phase::Failed("owner_grant_publication_review_expired".into());
            changed = true;
        }
        if matches!(&prompt.phase,Phase::CancellationReview(review) if review.expired()) {
            prompt.phase = Phase::Failed("owner_grant_review_changed".into());
            changed = true;
        }
        if changed {
            self.continue_grant_publication();
            self.present_grant_confirmation(cx);
        }
    }
    /// Advance only the exact native reviews covered by this window's owner
    /// submission. Unknown outcomes, rejection, closing, and reopen all stop it.
    fn continue_grant_publication(&mut self) {
        let Some(prompt)=&mut self.grant_confirmation else{return};
        if !prompt.submit_publication || !prompt.actionable(){return}
        let flow=prompt.flow.clone();
        match std::mem::replace(&mut prompt.phase,Phase::Loading) {
            Phase::Review(review) if !review.expired() && review.capacity_available()=> {
                self.grant_job(async move {flow.confirm(*review).await});
            }
            Phase::Progress(progress) if matches!(progress.state,OwnerGrantState::Recorded) && progress.closure.is_none()=> {
                prompt.publishing=true;
                self.grant_job(async move {flow.publication_review(progress.summary.intent_id).await});
            }
            Phase::PublicationReview(review) if !review.expired()=> {
                self.grant_job(async move {flow.publication_confirm(*review).await});
            }
            phase=> {prompt.phase=phase;prompt.submit_publication=false;}
        }
    }
    pub(super) fn grant_expiry_preset(&mut self,cx:&mut Cx,days:Option<i64>){
        self.capture_grant_choices(cx);
        let Some(prompt)=&mut self.grant_confirmation else{return};
        if !matches!(prompt.phase,Phase::Choose){return}
        if let Some(form)=&prompt.successor {
            let Some(value)=successor_preset(form,days,now_ms()) else{return};
            prompt.draft.expires_utc=value;
        } else {
            if !publication_preset_available(days){return}
            prompt.draft.expires_utc=publication_expiry(days);
        }
        self.present_grant_confirmation(cx);
    }
    pub(super) fn capture_grant_choices(&mut self, cx: &mut Cx) {
        let Some(prompt) = &mut self.grant_confirmation else {
            return;
        };
        if !matches!(prompt.phase, Phase::Choose) {
            return;
        }
        prompt.draft = GrantChoices {
            coordinator: prompt.flow.account().to_owned(),
            managers: self
                .view
                .text_input(cx, ids!(grant_confirmation.scroll.choices.managers))
                .text(),
            tokens: self
                .view
                .widget(cx, ids!(grant_confirmation.scroll.choices.tokens))
                .text(),
            expires_utc: self
                .view
                .text_input(cx, ids!(grant_confirmation.scroll.choices.expires))
                .text(),
            self_approval: false,
        };
        // Keep the shown new amount equal to what the tokens field would commit.
        let notice = match &prompt.successor {
            Some(form) => Some(successor_notice(form, &prompt.draft.tokens)),
            None => proposal_notice(
                &prompt.phase,
                prompt.proposal,
                &prompt.draft.tokens,
                prompt.flow.succession_supported() == Some(true),
                now_ms(),
            ),
        };
        self.view
            .label(cx, ids!(grant_confirmation.scroll.change))
            .set_text(cx, notice.as_deref().unwrap_or_default());
        self.view.button(cx,ids!(grant_confirmation.buttons.primary)).set_disabled(cx,!prompt.can_primary());
    }
    pub(super) fn grant_primary(&mut self, cx: &mut Cx) {
        self.capture_grant_choices(cx);
        self.poll_grant_confirmation(cx);
        let Some(prompt) = &mut self.grant_confirmation else {
            return;
        };
        if !prompt.can_primary() {
            return;
        }
        let flow = prompt.flow.clone();
        let now = now_ms();
        match std::mem::replace(&mut prompt.phase, Phase::Loading) {
            // Before any other action on a recorded grant: its month ended, so
            // the only step is this month's grant, never publishing the old one.
            phase if ended_grant(&phase, now) => {
                prompt.regrant = true;
                self.grant_job(async move { flow.choose_new_limits().await });
            }
            // A proposal beside this month's active grant opens the native
            // amendment form; nothing is reserved or published until the
            // owner submits that form.
            phase
                if published_grant(&phase, now)
                    && prompt.amendable()
                    && live_grant_tokens(&phase, now).is_some() =>
            {
                self.grant_job(async move { flow.choose_successor(Succession::Amendment).await });
            }
            Phase::Choose => {
                let choices = prompt.draft.clone();
                if let Err(code) = choices.clone().parse() {
                    if let Some(prompt) = &mut self.grant_confirmation {
                        prompt.phase = Phase::Choose;
                    }
                    self.present_grant_confirmation(cx);
                    self.view
                        .label(cx, ids!(grant_confirmation.scroll.status))
                        .set_text(cx, crate::i18n::tr(failure_message(&code)));
                    return;
                }
                // Validate native input before consuming the one possible prepare.
                let kind = prompt.successor.as_ref().map(|form| succession_arg(form.kind));
                prompt.submit();
                match kind {
                    Some(kind) => {
                        self.grant_job(async move { flow.prepare_successor(kind, choices).await })
                    }
                    None => self.grant_job(async move { flow.prepare(choices).await }),
                }
            }
            Phase::Review(review) => {
                prompt.submit();
                self.grant_job(async move { flow.confirm(*review).await });
            }
            Phase::PublicationReview(review) => {
                prompt.submit();
                self.grant_job(async move { flow.publication_confirm(*review).await })
            }
            Phase::CancellationReview(review) => {
                self.grant_job(async move { flow.cancellation_confirm(*review).await })
            }
            Phase::Progress(progress) if matches!(progress.state, OwnerGrantState::Declined) => {
                self.grant_job(async move { flow.choose_new_limits().await });
            }
            Phase::Progress(progress)
                if matches!(progress.state, OwnerGrantState::Recorded)
                    && progress.closure.is_some() =>
            {
                self.grant_job(async move { flow.choose_new_limits().await });
            }
            Phase::Progress(progress) if matches!(progress.state, OwnerGrantState::Recorded) => {
                prompt.publishing = true;
                prompt.submit();
                self.grant_job(
                    async move { flow.publication_review(progress.summary.intent_id).await },
                );
            }
            _ => {
                let id = prompt.original.clone();
                let publishing = prompt.publishing;
                let cancelling = prompt.cancelling;
                self.grant_job(async move {
                    if cancelling {
                        flow.cancellation_review(id.ok_or("owner_grant_original_intent_required")?)
                            .await
                    } else if publishing {
                        flow.publication_review(id.ok_or("owner_grant_original_intent_required")?)
                            .await
                    } else {
                        flow.review(id).await
                    }
                });
            }
        }
        self.present_grant_confirmation(cx);
    }
    pub(super) fn grant_cancel_intent(&mut self, cx: &mut Cx) {
        let Some(prompt) = &mut self.grant_confirmation else {
            return;
        };
        if !prompt.can_cancel_intent() {
            return;
        }
        let Some(id) = prompt.original.clone() else {
            return;
        };
        let flow = prompt.flow.clone();
        prompt.cancelling = true;
        self.grant_job(async move { flow.cancellation_review(id).await });
        self.present_grant_confirmation(cx);
    }
    pub(super) fn grant_recover(&mut self, cx: &mut Cx) {
        let Some(prompt) = &self.grant_confirmation else {
            return;
        };
        if !prompt.can_recover() {
            return;
        }
        let flow = prompt.flow.clone();
        let id = prompt.original.clone();
        let publishing = prompt.publishing;
        // This month's form refreshes its capacity, e.g. once last month's
        // Agents have reported their final usage; a successor form refreshes
        // its headroom and carried floor.
        let regrant = prompt.regrant;
        let successor = prompt.successor.as_ref().map(|form| succession_arg(form.kind));
        self.grant_job(async move {
            match id {
                Some(id) if publishing => flow.publication_review(id).await,
                Some(id) => flow.recover(id).await,
                None if successor.is_some() => {
                    flow.choose_successor(successor.unwrap_or(Succession::Amendment)).await
                }
                None if regrant => flow.choose_new_limits().await,
                None => flow.open().await,
            }
        });
        self.present_grant_confirmation(cx);
    }
    pub(super) fn close_grant_confirmation(&mut self, cx: &mut Cx) {
        let Some(prompt) = self.grant_confirmation.take() else {
            return;
        };
        // Close never mutates the private journal. An in-flight effect keeps
        // its original native custody even when the screen/reply disappears.
        let reply = close_reply(
            &prompt.phase,
            prompt.proposal,
            prompt.receiver.is_some(),
            prompt.original.as_deref(),
            prompt.flow.succession_supported() == Some(true),
            now_ms(),
        );
        splash_host_respond(
            cx,
            prompt.reply.0,
            prompt.reply.1,
            reply.as_ref().map(String::as_str).map_err(String::as_str),
        );
        self.present_grant_confirmation(cx);
        self.show_approval(cx);
        self.view.view(cx, ids!(app_content)).set_visible(cx, true);
        self.reply_redraw = cx.new_next_frame();
        self.view.redraw(cx);
    }
    pub(super) fn grant_key(&mut self, cx: &mut Cx, event: &Event) -> bool {
        let Some(prompt) = &self.grant_confirmation else {
            return false;
        };
        let Event::KeyDown(key) = event else {
            return false;
        };
        if key.key_code == KeyCode::Escape {
            self.close_grant_confirmation(cx);
            return true;
        }
        let cancel = self
            .view
            .button(cx, ids!(grant_confirmation.buttons.cancel))
            .area();
        let primary = self
            .view
            .button(cx, ids!(grant_confirmation.buttons.primary))
            .area();
        let recover = self
            .view
            .button(cx, ids!(grant_confirmation.buttons.recover))
            .area();
        let cancel_intent = self
            .view
            .button(cx, ids!(grant_confirmation.buttons.cancel_intent))
            .area();
        let mut targets = Vec::new();
        if matches!(prompt.phase, Phase::Choose) {
            targets.push(self.view.text_input(cx, ids!(grant_confirmation.scroll.choices.managers)).area());
            if let Some(slider)=self.view.widget(cx,ids!(grant_confirmation.scroll.choices.tokens.slider_wrap.amount_slider)).borrow::<crate::miniapps::design::MiniAppAmountSlider>() {targets.push(slider.focus_area());}
            targets.push(self.view.text_input(cx, ids!(grant_confirmation.scroll.choices.tokens.input)).area());
            for id in [ids!(grant_confirmation.scroll.choices.expiry_presets.day),ids!(grant_confirmation.scroll.choices.expiry_presets.week),ids!(grant_confirmation.scroll.choices.expiry_presets.month)] {
                let button=self.view.button(cx,id);if !button.disabled(cx){targets.push(button.area());}
            }
            targets.push(self.view.text_input(cx, ids!(grant_confirmation.scroll.choices.expires)).area());
        }
        targets.push(cancel);
        if prompt.can_primary() {
            targets.push(primary);
        }
        if prompt.can_recover() {
            targets.push(recover);
        }
        if prompt.can_cancel_intent() {
            targets.push(cancel_intent);
        }
        if key.key_code == KeyCode::Tab {
            let next = focus_next(
                targets.iter().position(|area| cx.has_key_focus(*area)),
                targets.len(),
                key.modifiers.shift,
            );
            cx.set_key_focus(targets[next]);
            return true;
        }
        if matches!(key.key_code, KeyCode::ReturnKey | KeyCode::Space) {
            if matches!(prompt.phase,Phase::Choose) {
                for (id,days) in [(ids!(grant_confirmation.scroll.choices.expiry_presets.day),Some(1)),(ids!(grant_confirmation.scroll.choices.expiry_presets.week),Some(7)),(ids!(grant_confirmation.scroll.choices.expiry_presets.month),None)] {
                    if cx.has_key_focus(self.view.button(cx,id).area()){self.grant_expiry_preset(cx,days);return true}
                }
            }
            if cx.has_key_focus(cancel) {
                self.close_grant_confirmation(cx);
                return true;
            }
            if prompt.can_primary() && cx.has_key_focus(primary) {
                self.grant_primary(cx);
                return true;
            }
            if prompt.can_recover() && cx.has_key_focus(recover) {
                self.grant_recover(cx);
                return true;
            }
            if prompt.can_cancel_intent() && cx.has_key_focus(cancel_intent) {
                self.grant_cancel_intent(cx);
                return true;
            }
        }
        false
    }
}
fn focus_next(current: Option<usize>, count: usize, reverse: bool) -> usize {
    match current {
        None => {
            if reverse {
                count - 1
            } else {
                0
            }
        }
        Some(index) => {
            if reverse {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_cycle_keeps_footer_reachable_both_directions() {
        assert_eq!(focus_next(Some(3), 6, false), 4);
        assert_eq!(focus_next(Some(5), 6, false), 0);
        assert_eq!(focus_next(Some(0), 6, true), 5);
        assert_eq!(focus_next(None, 2, true), 1);
    }
    #[test]
    fn period_display_is_explicit_utc_and_unknown_is_not_zero() {
        assert_eq!(utc(1790812800000), "2026-10-01 00:00 UTC");
        assert_ne!(amount(None), "0");
        assert_eq!(amount(Some(0)), "0 M");
    }
    #[test]
    fn publication_form_requires_known_capacity_and_expiry_in_current_period() {
        let mut form=OwnerGrantForm {contribution_id:"c".into(),resource_id:"r".into(),label:"Pool".into(),
            context: ::hagency::native_owner::OwnerGrantReviewContext {
                server_origin:"https://matrix.test".into(),profile_id:"profile".into(),resource_revision:"revision".into(),
                framework:"codex".into(),model:"model".into(),remaining_tokens:Some(1_000_001),
                resource_total_tokens:Some(2_000_000),shared_account_total_tokens:Some(9_000_000),observed_at_ms:1,
            }};
        let mut draft=GrantChoices {coordinator:"@owner:test".into(),managers:"@manager:test".into(),tokens:"1000001".into(),expires_utc:publication_expiry(None),self_approval:false};
        assert!(form_choices_valid(&form,&draft));
        draft.tokens="1000002".into();assert!(!form_choices_valid(&form,&draft));
        draft.tokens="1000001".into();
        form.context.remaining_tokens=None;assert!(!form_choices_valid(&form,&draft));
        form.context.remaining_tokens=Some(0);assert!(!form_choices_valid(&form,&draft));
        form.context.remaining_tokens=Some(1_000_001);
        draft.expires_utc=(publication_period_end()+chrono::Duration::seconds(1)).to_rfc3339();
        assert!(!form_choices_valid(&form,&draft));
        for days in [Some(1),Some(7),None] {
            draft.expires_utc=publication_expiry(days);
            assert!(form_choices_valid(&form,&draft));
        }
        draft.managers.clear();assert!(!form_choices_valid(&form,&draft));
    }
    fn pool_form(remaining: u64) -> OwnerGrantForm {
        OwnerGrantForm {contribution_id:"pool_1".into(),resource_id:"r".into(),label:"Pool".into(),
            context: ::hagency::native_owner::OwnerGrantReviewContext {
                server_origin:"https://matrix.test".into(),profile_id:"profile".into(),resource_revision:"revision".into(),
                framework:"codex".into(),model:"model".into(),remaining_tokens:Some(remaining),
                resource_total_tokens:Some(20_000_000),shared_account_total_tokens:None,observed_at_ms:1,
            }}
    }
    fn summary(tokens: u64) -> OwnerGrantSummary {
        OwnerGrantSummary {
            intent_id: "original".into(), grant_id: "grant_original".into(), owner_mxid: "@owner:test".into(),
            contribution_id: "pool_1".into(), resource_id: "resource".into(),
            coordinator_mxid: "@owner:test".into(), allocated_tokens: tokens,
            eligible_managers: vec!["@manager:test".into()], allow_self_approval: false,
            delegation_expires_at_ms: 1793491200000, period_key: "2026-10".into(),
            period_start_ms: 1790812800000, period_end_ms: 1793491200000,
            succession: None,
        }
    }
    fn publication(state: OwnerGrantPublicationState, server: Option<&str>) -> Phase {
        Phase::PublicationProgress(OwnerGrantPublicationProgress {
            summary: summary(2_000_000), state, server_state: server.map(str::to_owned),
            server_revision: server.map(|_| 1), observed_at_ms: server.map(|_| 1),
        })
    }
    fn progress(state: OwnerGrantState, closed: bool) -> Phase {
        Phase::Progress(OwnerGrantProgress {
            summary: summary(2_000_000), state, succession_state: None,
            closure: closed.then_some(::hagency::native_owner::OwnerGrantClosure {
                retained_tokens: 0, known_consumed_tokens: 0, unresolved_tokens: 0, observed_at_ms: 1,
            }),
        })
    }
    /// Inside the fixtures' 2026-10 month, and that month's end (2026-11-01).
    const OCTOBER: u64 = 1_792_000_000_000;
    const NOVEMBER: u64 = 1_793_491_200_000;
    #[test]
    fn proposal_prefills_only_the_native_tokens_field_and_is_never_lowered() {
        let form = pool_form(10_000_000);
        let mut draft = initial_draft(Some(3_000_000));
        assert_eq!(draft.tokens, "3000000");
        fill_form_defaults(&mut draft, &form);
        assert_eq!(draft.tokens, "3000000");
        // Applicants are still typed natively; expiry keeps today's default.
        assert!(draft.managers.is_empty());
        assert_eq!(draft.expires_utc, publication_expiry(None));
        let mut default = initial_draft(None);
        fill_form_defaults(&mut default, &form);
        assert_eq!(default.tokens, "1000000");
        // Above the available capacity the proposal stays visible and unconfirmable.
        let mut above = initial_draft(Some(12_000_000));
        fill_form_defaults(&mut above, &form);
        above.coordinator = "@owner:test".into();
        above.managers = "@manager:test".into();
        assert_eq!(above.tokens, "12000000");
        assert!(!form_choices_valid(&form, &above));
        above.tokens = "3000000".into();
        assert!(form_choices_valid(&form, &above));
        // A reopened form keeps what the owner typed over the proposal.
        let mut edited = initial_draft(Some(3_000_000));
        edited.tokens = "2500000".into();
        fill_form_defaults(&mut edited, &form);
        assert_eq!(edited.tokens, "2500000");
    }
    #[test]
    fn proposal_notice_shows_the_current_and_the_committed_amount() {
        // Only a pool without an issued grant reaches the form; the new amount
        // is whatever the native tokens field holds now.
        for (field, shown) in [("3000000", "3 M"), ("2500000", "2.5 M"), ("2.5", "Unavailable")] {
            assert_eq!(
                proposal_notice(&Phase::Choose, Some(3_000_000), field, false, OCTOBER).as_deref(),
                Some(format!("Per-request limit: None active → {shown}").as_str())
            );
        }
        assert!(proposal_notice(&Phase::Choose, None, "3000000", false, OCTOBER).is_none());
        assert!(proposal_notice(&Phase::Loading, Some(3_000_000), "", false, OCTOBER).is_none());
        let declined = proposal_notice(&progress(OwnerGrantState::Declined, false), Some(3_000_000), "", false, OCTOBER).unwrap();
        assert_eq!(declined, "Per-request limit: None active → 3 M\nThe proposed limit is filled in when you choose new limits.");
        let closed = proposal_notice(&progress(OwnerGrantState::Recorded, true), Some(3_000_000), "", false, OCTOBER).unwrap();
        assert!(closed.ends_with("filled in when you choose new limits."), "{closed}");
        let failed = proposal_notice(&Phase::Failed("owner_grant_prior_intent_required".into()), Some(3_000_000), "", false, OCTOBER).unwrap();
        assert!(failed.ends_with("\nThe proposed limit was not applied."), "{failed}");
    }
    #[test]
    fn published_grant_is_shown_read_only_and_never_replaced_in_its_month() {
        let live = publication(OwnerGrantPublicationState::ServerObserved, Some("active"));
        assert_eq!(live_grant_tokens(&live, OCTOBER), Some(2_000_000));
        assert_eq!(
            proposal_notice(&live, Some(3_000_000), "", false, OCTOBER).unwrap(),
            format!("Per-request limit: 2 M → 3 M\n{PUBLISHED_NOTE}")
        );
        // A grant revoked, or expired by its delegation, within its month still
        // used this month of the admission: no current limit, and the proposal
        // still cannot become a second grant this month.
        for server in ["revoked", "expired"] {
            let used = publication(OwnerGrantPublicationState::ServerObserved, Some(server));
            assert_eq!(live_grant_tokens(&used, OCTOBER), None);
            assert_eq!(
                proposal_notice(&used, Some(3_000_000), "", false, OCTOBER).unwrap(),
                format!("Per-request limit: None active → 3 M\n{PUBLISHED_NOTE}")
            );
        }
        // No proposal, applicant or selector flag offers a commit beside a
        // published grant: nothing is revoked, reserved or published from here.
        let form = pool_form(10_000_000);
        let mut draft = initial_draft(Some(3_000_000));
        fill_form_defaults(&mut draft, &form);
        draft.coordinator = "@owner:test".into();
        draft.managers = "@manager:test".into();
        assert!(primary_available(
            &Phase::Choose,
            Some(&root_bounds(&form)),
            &draft,
            false,
            true,
            false, OCTOBER
        ));
        for server in ["active", "revoked", "expired"] {
            let phase = publication(OwnerGrantPublicationState::ServerObserved, Some(server));
            for has_original in [false, true] {
                for can_choose_new_limits in [false, true] {
                    assert!(!primary_available(
                        &phase,
                        Some(&root_bounds(&form)),
                        &draft,
                        has_original,
                        can_choose_new_limits,
                        false, OCTOBER
                    ));
                }
            }
        }
    }
    #[test]
    fn ended_month_grant_no_longer_blocks_a_proposal_and_offers_this_month() {
        const ENDED_NOTE: &str =
            "The proposed limit is filled in when you choose this month's limits.";
        let form = pool_form(10_000_000);
        let draft = initial_draft(Some(3_000_000));
        // Once its UTC month has ended, a published grant (the server reports
        // it expired) or an unpublished recorded one no longer blocks: the
        // pool has no limit and the panel offers this month's limits instead.
        for phase in [
            publication(OwnerGrantPublicationState::ServerObserved, Some("expired")),
            publication(OwnerGrantPublicationState::ServerObserved, Some("active")),
            publication(OwnerGrantPublicationState::ServerObserved, Some("revoked")),
            publication(OwnerGrantPublicationState::OutcomeUnknown, None),
            publication(OwnerGrantPublicationState::Rejected, None),
            progress(OwnerGrantState::Recorded, false),
        ] {
            assert!(ended_grant(&phase, NOVEMBER) && !ended_grant(&phase, NOVEMBER - 1));
            assert!(!published_grant(&phase, NOVEMBER));
            assert_eq!(live_grant_tokens(&phase, NOVEMBER), None);
            assert_eq!(
                proposal_notice(&phase, Some(3_000_000), "", false, NOVEMBER).unwrap(),
                format!("Per-request limit: None active → 3 M\n{ENDED_NOTE}")
            );
            assert!(primary_available(
                &phase,
                Some(&root_bounds(&form)),
                &draft,
                true,
                true,
                false, NOVEMBER
            ));
            assert!(!primary_available(
                &phase,
                Some(&root_bounds(&form)),
                &draft,
                true,
                false,
                false, NOVEMBER
            ));
            // Closing keeps the public progress, never pool_grant_already_published.
            let reply: serde_json::Value = serde_json::from_str(
                &close_reply(&phase, Some(3_000_000), false, Some("original"), false, NOVEMBER).unwrap(),
            )
            .unwrap();
            assert_eq!(reply["summary"]["periodKey"], "2026-10");
        }
        // Unresolved, declined and closed originals keep their own handling.
        for phase in [
            progress(OwnerGrantState::OutcomeUnknown, false),
            progress(OwnerGrantState::ReviewExpired, false),
            progress(OwnerGrantState::Declined, false),
            progress(OwnerGrantState::Recorded, true),
        ] {
            assert!(!ended_grant(&phase, NOVEMBER));
        }
        assert!(!primary_available(
            &progress(OwnerGrantState::OutcomeUnknown, false),
            None,
            &draft,
            true,
            true,
            false, NOVEMBER
        ));
    }
    #[test]
    fn other_originals_keep_their_amounts_and_their_own_actions() {
        const EXISTING: &str = "Per-request limit: None active → 3 M\nThe proposed limit is not applied to this existing grant. It keeps the amounts shown below.";
        for phase in [
            publication(OwnerGrantPublicationState::Rejected, None),
            publication(OwnerGrantPublicationState::OutcomeUnknown, None),
            progress(OwnerGrantState::Recorded, false),
            progress(OwnerGrantState::OutcomeUnknown, false),
        ] {
            assert_eq!(live_grant_tokens(&phase, OCTOBER), None);
            assert_eq!(
                proposal_notice(&phase, Some(3_000_000), "", false, OCTOBER).unwrap(),
                EXISTING
            );
        }
        // An unattempted original reserved nothing; cancelling it reopens the form.
        for state in [OwnerGrantState::ReviewRequired, OwnerGrantState::ReviewExpired] {
            assert_eq!(
                proposal_notice(&progress(state, false), Some(3_000_000), "", false, OCTOBER).unwrap(),
                format!("Per-request limit: None active → 3 M\n{PENDING_NOTE}")
            );
        }
        // A proposal adds or removes no action on an existing original.
        let draft = initial_draft(Some(3_000_000));
        assert!(primary_available(
            &progress(OwnerGrantState::Recorded, false),
            None,
            &draft,
            true,
            true,
            false, OCTOBER
        ));
        assert!(!primary_available(
            &progress(OwnerGrantState::OutcomeUnknown, false),
            None,
            &draft,
            true,
            true,
            false, OCTOBER
        ));
    }
    /// The real UTC month (start, end): `GrantChoices::parse` reads the real clock.
    fn this_month() -> (u64, u64) {
        use chrono::Datelike;
        let first = chrono::Utc::now().date_naive().with_day(1).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
        (first.timestamp_millis() as u64, publication_period_end().timestamp_millis() as u64)
    }
    fn month_after(start: u64) -> u64 {
        use chrono::Datelike;
        let at = chrono::DateTime::from_timestamp_millis(start as i64).unwrap().date_naive();
        (at + chrono::Duration::days(32)).with_day(1).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis() as u64
    }
    fn successor_form(kind: OwnerGrantSuccessionKind) -> OwnerGrantSuccessionForm {
        let (start, end) = this_month();
        let mut predecessor = summary(2_000_000);
        predecessor.period_start_ms = start;
        predecessor.period_end_ms = end;
        predecessor.delegation_expires_at_ms = end;
        let (key, period_start, period_end) = if kind == OwnerGrantSuccessionKind::Renewal {
            let at = chrono::DateTime::from_timestamp_millis(end as i64).unwrap();
            (at.format("%Y-%m").to_string(), end, month_after(end))
        } else {
            let at = chrono::DateTime::from_timestamp_millis(start as i64).unwrap();
            (at.format("%Y-%m").to_string(), start, end)
        };
        OwnerGrantSuccessionForm {
            kind,
            contribution_id: "pool_1".into(),
            resource_id: "resource".into(),
            label: "Pool".into(),
            context: pool_form(5_000_000).context,
            predecessor,
            floor: ::hagency::native_owner::OwnerGrantFloor { children: 2, retained_tokens: 600_000 },
            period_key: key,
            period_start_ms: period_start,
            period_end_ms: period_end,
            window_opens_at_ms: end.saturating_sub(7 * 86_400_000),
            window_closes_at_ms: end,
        }
    }
    #[test]
    fn an_amendment_keeps_running_agents_and_only_adds_applicants() {
        let form = successor_form(OwnerGrantSuccessionKind::Amendment);
        let bounds = successor_bounds(&form);
        assert_eq!(bounds.minimum, 600_000);
        assert_eq!(bounds.keep_managers, ["@manager:test"]);
        // The proposal fills the tokens field; applicants are added to the
        // current ones; the current delegation is kept.
        let mut draft = initial_draft(Some(800_000));
        draft.managers = "@new:test".into();
        fill_successor_defaults(&mut draft, &form, now_ms());
        assert_eq!(draft.tokens, "800000");
        assert_eq!(draft.managers, "@manager:test @new:test");
        assert_eq!(draft.expires_utc, rfc3339(form.predecessor.delegation_expires_at_ms));
        draft.coordinator = "@owner:test".into();
        assert!(choices_valid(&bounds, &draft));
        // Lowering is allowed down to what running Agents hold, never below.
        for (tokens, valid) in [("600000", true), ("599999", false), ("5000000", true), ("5000001", false)] {
            let mut d = draft.clone();
            d.tokens = tokens.into();
            assert_eq!(choices_valid(&bounds, &d), valid, "{tokens}");
        }
        // Removing a current applicant is never committed from this form.
        let mut removed = draft.clone();
        removed.managers = "@new:test".into();
        assert!(!choices_valid(&bounds, &removed));
        // The delegation cannot leave the month being amended.
        let mut late = draft.clone();
        late.expires_utc = rfc3339(form.period_end_ms + 1000);
        assert!(!choices_valid(&bounds, &late));
        // Without a proposal the field starts at the current limit.
        let mut plain = initial_draft(None);
        fill_successor_defaults(&mut plain, &form, now_ms());
        assert_eq!(plain.tokens, "2000000");
        assert_eq!(plain.managers, "@manager:test");
        // The notice names the change, the carried floor and pending requests.
        let notice = successor_notice(&form, "800000");
        let lines: Vec<&str> = notice.lines().collect();
        assert_eq!(lines.len(), 3, "{notice}");
        assert_eq!(lines[0], format!("Per-request limit: 2 M → {} (applies now, until {})", amount(Some(800_000)), utc(form.period_end_ms)));
        assert_eq!(lines[1], format!("Agents that keep running: 2. The limit cannot go below {}, which Agents already hold this month.", amount(Some(600_000))));
        assert_eq!(lines[2], format!("Pending requests above {} cannot be approved.", amount(Some(800_000))));
        // Unknown capacity keeps the form unconfirmable.
        let mut unknown = form.clone();
        unknown.context.remaining_tokens = None;
        assert!(!choices_valid(&successor_bounds(&unknown), &draft));
    }
    #[test]
    fn a_renewal_starts_next_month_and_its_delegation_reaches_into_it() {
        let form = successor_form(OwnerGrantSuccessionKind::Renewal);
        let bounds = successor_bounds(&form);
        let mut draft = initial_draft(None);
        fill_successor_defaults(&mut draft, &form, now_ms());
        // Q12: a renewal's delegation defaults to the end of the new month.
        assert_eq!(draft.expires_utc, rfc3339(form.period_end_ms));
        assert_eq!(draft.tokens, "2000000");
        draft.coordinator = "@owner:test".into();
        assert!(choices_valid(&bounds, &draft));
        for (expiry, valid) in [
            (form.period_start_ms, false),
            (form.period_start_ms + 1000, true),
            (form.period_end_ms, true),
            (form.period_end_ms + 1000, false),
        ] {
            let mut d = draft.clone();
            d.expires_utc = rfc3339(expiry);
            assert_eq!(choices_valid(&bounds, &d), valid, "{expiry}");
        }
        // Presets only offer expiries inside the new month.
        assert_eq!(successor_preset(&form, None, now_ms()), Some(rfc3339(form.period_end_ms)));
        for days in [Some(1), Some(7)] {
            if let Some(value) = successor_preset(&form, days, now_ms()) {
                let mut d = draft.clone();
                d.expires_utc = value;
                assert!(choices_valid(&bounds, &d));
            }
        }
        let notice = successor_notice(&form, "3000000");
        assert!(notice.starts_with(&format!("Renew for {}: per-request limit 2 M → 3 M\n", form.period_key)), "{notice}");
        assert!(notice.ends_with(&format!("Takes effect {}.", utc(form.period_start_ms))), "{notice}");
    }
    #[test]
    fn a_proposal_beside_an_active_grant_opens_the_amendment_when_the_server_supports_it() {
        let live = publication(OwnerGrantPublicationState::ServerObserved, Some("active"));
        assert_eq!(
            proposal_notice(&live, Some(3_000_000), "", true, OCTOBER).unwrap(),
            format!("Per-request limit: 2 M → 3 M\n{AMENDABLE_NOTE}")
        );
        let draft = initial_draft(Some(3_000_000));
        assert!(primary_available(&live, None, &draft, true, true, true, OCTOBER));
        // Only the native amendment form can commit; without support or a
        // pool selector nothing is offered.
        assert!(!primary_available(&live, None, &draft, true, true, false, OCTOBER));
        assert!(!primary_available(&live, None, &draft, true, false, true, OCTOBER));
        // The reply is the public progress, not a refusal.
        let reply: serde_json::Value = serde_json::from_str(
            &close_reply(&live, Some(3_000_000), false, Some("original"), true, OCTOBER).unwrap(),
        )
        .unwrap();
        assert_eq!(reply["serverState"], "active");
        assert_eq!(reply["summary"]["grantId"], "grant_original");
        // A revoked, superseded or expired grant cannot be amended.
        for server in ["revoked", "superseded", "expired"] {
            let used = publication(OwnerGrantPublicationState::ServerObserved, Some(server));
            assert_eq!(
                proposal_notice(&used, Some(3_000_000), "", true, OCTOBER).unwrap(),
                format!("Per-request limit: None active → 3 M\n{NOT_AMENDABLE_NOTE}")
            );
            assert!(!primary_available(&used, None, &draft, true, true, true, OCTOBER));
        }
    }
    #[test]
    fn a_scheduled_renewal_is_shown_but_never_amended_from_here() {
        let mut p = OwnerGrantPublicationProgress {
            summary: summary(2_000_000),
            state: OwnerGrantPublicationState::ServerObserved,
            server_state: Some("active".into()),
            server_revision: Some(1),
            observed_at_ms: Some(1),
        };
        p.summary.period_key = "2026-11".into();
        p.summary.period_start_ms = NOVEMBER;
        p.summary.period_end_ms = 1_796_083_200_000;
        let scheduled = Phase::PublicationProgress(p);
        assert!(published_grant(&scheduled, OCTOBER) && scheduled_grant(&scheduled, OCTOBER));
        assert_eq!(live_grant_tokens(&scheduled, OCTOBER), None);
        assert_eq!(
            proposal_notice(&scheduled, Some(3_000_000), "", true, OCTOBER).unwrap(),
            format!("Per-request limit: None active → 3 M\n{SCHEDULED_NOTE}")
        );
        let draft = initial_draft(Some(3_000_000));
        assert!(!primary_available(&scheduled, None, &draft, true, true, true, OCTOBER));
        // In its own month it is the active grant and can be amended.
        assert!(!scheduled_grant(&scheduled, NOVEMBER));
        assert_eq!(live_grant_tokens(&scheduled, NOVEMBER), Some(2_000_000));
        assert!(primary_available(&scheduled, None, &draft, true, true, true, NOVEMBER));
    }
    #[test]
    fn closing_beside_a_published_grant_names_why_the_proposal_was_not_applied() {
        for server in ["active", "revoked", "expired"] {
            let phase = publication(OwnerGrantPublicationState::ServerObserved, Some(server));
            assert_eq!(
                close_reply(&phase, Some(3_000_000), false, Some("original"), false, OCTOBER),
                Err("pool_grant_already_published".into())
            );
            // Without a proposal the reply stays the public publication status.
            let reply: serde_json::Value =
                serde_json::from_str(&close_reply(&phase, None, false, Some("original"), false, OCTOBER).unwrap()).unwrap();
            assert_eq!(reply["state"], "server_observed");
            assert_eq!(reply["serverState"], server);
            assert_eq!(reply["summary"]["allocatedTokens"], 2_000_000);
        }
        // Other outcomes keep their public progress so the app can recover them.
        let reserved: serde_json::Value = serde_json::from_str(
            &close_reply(&progress(OwnerGrantState::Recorded, false), Some(3_000_000), false, Some("original"), false, OCTOBER).unwrap(),
        ).unwrap();
        assert_eq!(reserved["state"], "recorded");
        assert_eq!(reserved["summary"]["allocatedTokens"], 2_000_000);
        let unknown = close_reply(
            &publication(OwnerGrantPublicationState::OutcomeUnknown, None),
            Some(3_000_000),
            false,
            Some("original"),
            false,
            OCTOBER,
        );
        assert!(unknown.unwrap().contains("\"state\":\"outcome_unknown\""));
        // Closing the form itself applies nothing and names no grant.
        let parsed = |reply: Result<String, String>| serde_json::from_str::<serde_json::Value>(&reply.unwrap()).unwrap();
        assert_eq!(
            parsed(close_reply(
                &Phase::Choose,
                Some(3_000_000),
                false,
                None,
                false, OCTOBER
            )),
            json!({"state":"review_closed","intentId":null})
        );
        assert_eq!(
            parsed(close_reply(
                &Phase::Loading,
                Some(3_000_000),
                true,
                None,
                false, OCTOBER
            )),
            json!({"state":"outcome_unknown","intentId":null})
        );
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use makepad_widgets::makepad_draw::cx_draw::CxDraw;
    #[test]
    fn native_owner_grant_footer_remains_inside_viewport_with_long_english_and_chinese_review() {
        for (width, height, cn, mode) in [
            (360.0, 640.0, false, 0),
            (360.0, 640.0, true, 0),
            (840.0, 700.0, false, 0),
            (840.0, 700.0, true, 0),
            (360.0, 640.0, false, 1),
            (360.0, 640.0, true, 1),
            (840.0, 700.0, false, 1),
            (840.0, 700.0, true, 1),
            (360.0, 640.0, false, 2),
            (360.0, 640.0, true, 2),
            (840.0, 700.0, false, 2),
            (840.0, 700.0, true, 2),
            (360.0, 640.0, false, 3),
            (360.0, 640.0, true, 3),
            (840.0, 700.0, false, 3),
            (840.0, 700.0, true, 3),
        ] {
            let mut cx = Cx::new(Box::new(|_, _| {}));
            let mut panel = cx.with_vm(|vm| {
                makepad_widgets::script_mod(vm);
                crate::i18n::install(vm);
                makepad_code_editor::script_mod(vm);
                crate::shared::script_mod(vm);
                vm.bx.captured_errors = Some(Vec::new());
                crate::miniapps::script_mod(vm);
                let value =
                    vm.eval(script! {use mod.prelude.widgets.* use mod.widgets.* MiniAppsPanel{}});
                let panel = MiniAppsPanel::script_from_value(vm, value);
                let errors = vm.take_errors();
                assert!(errors.is_empty(), "native grant script errors: {errors:?}");
                panel
            });
            for id in [
                ids!(import_form),
                ids!(app_content),
                ids!(notice),
                ids!(approval),
            ] {
                panel.view.widget(&cx, id).set_visible(&mut cx, false);
            }
            panel
                .view
                .view(&cx, ids!(grant_confirmation))
                .set_visible(&mut cx, true);
            panel
                .view
                .view(&cx, ids!(grant_confirmation.scroll.choices))
                .set_visible(&mut cx, mode==3);
            panel.view.widget(&cx,ids!(grant_confirmation.scroll.details)).set_visible(&mut cx,mode!=3);
            let long = if cn {
                "Agent 申请者\n@申请者:example.org\n原始授权核对信息\n"
            } else {
                "Agent applicant\n@applicant:example.org\nOriginal authorization review\n"
            }
            .repeat(64);
            panel
                .view
                .label(&cx, ids!(grant_confirmation.scroll.details.copy))
                .set_text(&mut cx, &long);
            // The proposed-limit line sits above the form and the details.
            panel.view.widget(&cx, ids!(grant_confirmation.scroll.change)).set_visible(&mut cx, true);
            panel.view.label(&cx, ids!(grant_confirmation.scroll.change)).set_text(&mut cx, if cn {
                "每次申请上限：2 M → 3 M\n此资源池在当前准入下本月已发布过授权，而服务器每次准入每月只接受一个授权，因此提议的上限未生效。没有撤销或更改任何授权。"
            } else {
                "Per-request limit: 2 M → 3 M\nThis pool already has a published grant for this month on its current admission, and the server accepts one grant per admission each month, so the proposed limit is not applied. Nothing was revoked or changed."
            });
            if cn {
                for (id, text) in [
                    (ids!(grant_confirmation.buttons.cancel), "关闭核对"),
                    (ids!(grant_confirmation.buttons.primary), "预留容量"),
                    (ids!(grant_confirmation.buttons.recover), "恢复原始授权"),
                    (
                        ids!(grant_confirmation.buttons.cancel_intent),
                        "核对取消操作",
                    ),
                ] {
                    panel.view.button(&cx, id).set_text(&mut cx, text);
                }
            }
            if mode == 1 {
                panel
                    .view
                    .button(&cx, ids!(grant_confirmation.buttons.primary))
                    .set_text(
                        &mut cx,
                        if cn {
                            "发布到服务器"
                        } else {
                            "Publish to server"
                        },
                    );
            }
            if mode == 2 {
                panel
                    .view
                    .button(&cx, ids!(grant_confirmation.buttons.primary))
                    .set_text(
                        &mut cx,
                        if cn {
                            "取消待确认预留"
                        } else {
                            "Cancel pending reservation"
                        },
                    );
            }
            let pass = DrawPass::new(&mut cx);
            pass.set_size(&mut cx, dvec2(width, height));
            let mut list = DrawList2d::new(&mut cx);
            let event = DrawEvent::default();
            let mut draw = CxDraw::new(&mut cx, &event);
            let mut cx2d = Cx2d::new(&mut draw);
            cx2d.begin_pass(&pass, None);
            list.begin_always(&mut cx2d);
            cx2d.begin_root_turtle(dvec2(width, height), Layout::flow_overlay());
            panel.draw_all(&mut cx2d, &mut Scope::empty());
            cx2d.end_pass_sized_turtle();
            list.end(&mut cx2d);
            cx2d.end_pass(&pass);
            drop(cx2d);
            drop(draw);
            let change=panel.view.widget(&cx,ids!(grant_confirmation.scroll.change)).area().rect(&cx);
            assert!(change.size.x>0.0 && change.size.y>0.0 && change.pos.x+change.size.x<=width+0.5,"{change:?}");
            if mode==3 {
                let slider=panel.view.widget(&cx,ids!(grant_confirmation.scroll.choices.tokens.slider_wrap.amount_slider)).area().rect(&cx);
                let input=panel.view.widget(&cx,ids!(grant_confirmation.scroll.choices.tokens.input)).area().rect(&cx);
                assert!(slider.size.x>=100.0 && slider.size.y>=44.0,"{slider:?}");
                assert!(input.size.x<=111.0 && input.pos.x+input.size.x<=width+0.5,"{input:?}");
                for path in [ids!(grant_confirmation.scroll.choices.tokens.decrease),ids!(grant_confirmation.scroll.choices.tokens.increase)] {
                    assert!(!panel.view.widget(&cx,path).visible());
                }
            }
            for path in [
                ids!(grant_confirmation.buttons.cancel),
                ids!(grant_confirmation.buttons.primary),
                ids!(grant_confirmation.buttons.recover),
                ids!(grant_confirmation.buttons.cancel_intent),
            ] {
                let rect = panel.view.widget(&cx, path).area().rect(&cx);
                assert!(
                    rect.size.x > 0.0
                        && rect.size.y > 0.0
                        && rect.pos.x >= 0.0
                        && rect.pos.y >= 0.0
                        && rect.pos.x + rect.size.x <= width + 0.5
                        && rect.pos.y + rect.size.y <= height + 0.5,
                    "grant control {path:?} clipped at {width}x{height}, cn={cn}: {rect:?}"
                );
            }
        }
    }
}

#[cfg(test)]
mod diagnostic_tests {
    #[test]
    fn grant_diagnostics_only_accept_exact_static_codes() {
        for code in [
            "management_access_required",
            "management_monthly_regrant_required",
            "management_backing_changed",
            "management_capacity",
            "management_drain_outcome_unknown",
            "management_drain_unavailable",
            "management_enrollment_changed",
            "management_enrollment_history_full",
            "management_enrollment_history_required",
            "management_model_unqualified",
            "management_not_found",
            "management_outcome_unknown",
            "management_profile_mismatch",
            "management_resource_in_use",
            "management_revision_changed",
            "management_revision_conflict",
            "management_scope_changed",
            "management_state_unavailable",
            "management_storage_failed",
            "management_unavailable",
            "management_permission_required",
            "management_session_required",
            "management_command_conflict",
            "management_receipt_capacity",
            "native_grant_not_publishable",
            "native_grant_receipt_required",
            "native_owner_state_unavailable",
            "native_random_unavailable",
            "owner_authorization_required",
            "owner_grant_already_attempted",
            "owner_grant_binding_changed",
            "owner_grant_capacity_unavailable",
            "owner_grant_history_full",
            "owner_grant_history_unavailable",
            "owner_grant_intent_missing",
            "owner_grant_original_intent_required",
            "owner_grant_original_review_expired",
            "owner_grant_publication_confirmation_required",
            "owner_grant_publication_history_unavailable",
            "owner_grant_publication_review_changed",
            "owner_grant_publication_review_expired",
            "owner_grant_review_changed",
            "owner_source_adoption_required",
            "provider_account_changed",
            "provider_closing",
            "provider_directory_unavailable",
            "provider_identity_timeout",
            "provider_identity_unavailable",
            "provider_profile_mismatch",
            "local_codex_account_not_signed_in",
            "local_provider_not_signed_in",
            "contribution_not_admitted",
            "contribution_executor_changed",
            "contribution_resource_changed",
            "allocation_enrollment_changed",
            "invalid_grant_budget",
            "invalid_grant_expiry",
            "invalid_grant_manager",
            "management_grant_succession_required",
            "pool_grant_succession_unavailable",
            "owner_grant_predecessor_required",
            "owner_grant_succession_pending",
            "owner_grant_successor_exists",
            "owner_grant_renewal_window_closed",
            "owner_grant_below_floor",
            "owner_grant_manager_removed",
            "grant_predecessor_not_effective",
            "grant_successor_exists",
            "grant_successor_below_reserved",
            "grant_succession_window_closed",
            "invalid_grant_succession",
        ] {
            assert_eq!(super::grant_failure_code(code), code);
        }
        for untrusted in [
            "",
            "/private/profile/provider.json",
            "bearer secret",
            "owner_grant_history_unavailable: secret",
            "provider_identity_unavailable\ncredential=secret",
            "{\"error\":\"owner_authorization_required\",\"token\":\"secret\"}",
        ] {
            assert_eq!(
                super::grant_failure_code(untrusted),
                "owner_grant_unclassified_failure"
            );
        }
    }
}
