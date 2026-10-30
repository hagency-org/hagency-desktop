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
    /// The owner submitted terms here; the result shown is theirs.
    submitted: bool,
    /// Sections the owner opened: the list editor, advanced and details.
    disclosure: Disclosure,
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
/// A month as owners read it: "10 月" in Chinese, "October" in English.
fn month_name(month: u32) -> String {
    const NAMES: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July", "August",
        "September", "October", "November", "December",
    ];
    if crate::i18n::language() == crate::i18n::Language::Chinese {
        format!("{month} 月")
    } else {
        NAMES[(month.clamp(1, 12) - 1) as usize].to_owned()
    }
}
/// The UTC month a grant covers, from its period key ("2026-10").
fn key_month(key: &str) -> Option<String> {
    key.get(5..7)?.parse::<u32>().ok().filter(|m| (1..=12).contains(m)).map(month_name)
}
/// The UTC month that contains `ms`, which is the month a new grant covers.
fn ms_month(ms: u64) -> String {
    use chrono::Datelike;
    let month = i64::try_from(ms)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map_or(1, |date| date.month());
    month_name(month)
}
/// The calendar day a UTC month starts on, as the mini app names it:
/// "11 月 1 日" or "November 1". Months here are UTC months.
fn month_day(ms: u64) -> String {
    let Some(date) = i64::try_from(ms).ok().and_then(chrono::DateTime::from_timestamp_millis) else {
        return crate::i18n::tr("Unavailable").into();
    };
    if crate::i18n::language() == crate::i18n::Language::Chinese {
        date.format("%-m 月 %-d 日").to_string()
    } else {
        date.format("%B %-d").to_string()
    }
}
/// A day and time in the owner's own time zone, for when sharing ends.
fn local_time(ms: u64) -> String {
    let Some(date) = i64::try_from(ms).ok().and_then(chrono::DateTime::from_timestamp_millis) else {
        return crate::i18n::tr("Unavailable").into();
    };
    let date = date.with_timezone(&chrono::Local);
    if crate::i18n::language() == crate::i18n::Language::Chinese {
        date.format("%-m 月 %-d 日 %H:%M").to_string()
    } else {
        date.format("%B %-d, %-I:%M %p").to_string()
    }
}
/// A person on the list as the mini app shows them: "@lisi". Every applicant
/// is on the owner's own server, so the server part only appears in details.
fn person(id: &str) -> &str {
    id.split_once(':').map_or(id, |(name, _)| name)
}
/// Up to three people by name, then a count, for one-line sentences.
fn people_text(ids: &[String]) -> String {
    let separator = if crate::i18n::language() == crate::i18n::Language::Chinese { "、" } else { ", " };
    let names: Vec<&str> = ids.iter().map(|id| person(id)).collect();
    if names.len() <= 3 {
        return names.join(separator);
    }
    crate::i18n::format(
        "{first} and {more} more",
        &[("first", names[..2].join(separator)), ("more", (names.len() - 2).to_string())],
    )
}
/// Matrix IDs from the native applicants field, in the order they are kept.
fn managers_of(text: &str) -> Vec<String> {
    let mut managers: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
    managers.sort();
    managers.dedup();
    managers
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
#[cfg(test)]
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
/// A script proposal only seeds the native tokens field, and its applicants
/// only the applicants field; the owner can edit both and the committed values
/// are always read back from those widgets.
fn initial_draft(proposal: Option<u64>, applicants: &[String]) -> GrantChoices {
    GrantChoices {
        tokens: proposal.map(|tokens| tokens.to_string()).unwrap_or_default(),
        managers: applicants.join(" "),
        ..GrantChoices::default()
    }
}
/// Defaults for an amendment or renewal: the current limit unless a proposal
/// filled the field, the current applicants plus any typed or proposed ones
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
/// An unattempted original reserves nothing; discarding it reopens the form.
const PENDING_NOTE: &str = "An unfinished publish is waiting, so this amount isn’t used yet. Discard that publish first, then choose again.";
/// One grant per admission per UTC month, even after revocation, on a server
/// without succession support.
const PUBLISHED_NOTE: &str = "This pool is already published this month and the server takes one publish a month, so this amount wasn’t applied. Nothing was withdrawn or changed.";
/// Beside a published grant on a server that supports amendments.
const AMENDABLE_NOTE: &str = "Not applied yet. Tap Change Per-Request Limit to continue; running Agents keep what they hold.";
/// Beside a revoked, superseded or expired grant this month.
const NOT_AMENDABLE_NOTE: &str = "This publish no longer applies, so it can’t be changed.";
/// Beside next month's scheduled renewal: this month can no longer change here.
const SCHEDULED_NOTE: &str = "Next month is already published ahead, so this month’s per-request limit can’t change now.";
/// A published grant whose month has not started: a scheduled renewal.
fn scheduled_grant(phase: &Phase, now: u64) -> bool {
    matches!(phase, Phase::PublicationProgress(p) if p.summary.period_start_ms > now)
        && published_grant(phase, now)
}
/// Shown for a grant whose month ended: the pool is closed until the owner
/// publishes this month's grant, which needs last month's Agents settled.
const ENDED_LEAD: &str = "Last month’s capacity has ended, so this pool has none right now.";
const ENDED_STATUS: &str = "Set this month’s capacity to publish again. Last month’s Agents must first stop and report their final use, which needs this computer online after the month ends.";
/// This month's form cannot show capacity while last month's usage is unknown.
const REGRANT_UNKNOWN_STATUS: &str = "How much can be shared isn’t known until last month’s Agents report their final use. Keep this computer online, then refresh.";
const UNKNOWN_CAPACITY_STATUS: &str = "How much can still be shared isn’t known, so this can’t be confirmed now. Keep this computer online, then refresh.";

/// What the window is about, named the way the mini app names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GrantKind {
    /// This month's publish of a pool.
    Publish,
    /// A new per-request limit this month.
    Amend,
    /// People added to this month's list, the limit unchanged.
    AddPeople,
    /// Next month, published ahead.
    Renew,
    /// An existing record opened by itself, such as this month's list.
    Record,
}
/// The panel's collapsed sections the owner opened. All start closed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Disclosure {
    pub people: bool,
    pub advanced: bool,
    pub technical: bool,
}
/// The native facts one rendering is computed from. The flow's facts are
/// copied in, so the shipped panel and its offline preview share this code.
pub(super) struct GrantState<'a> {
    phase: &'a Phase,
    draft: &'a GrantChoices,
    proposal: Option<u64>,
    form: Option<&'a OwnerGrantForm>,
    successor: Option<&'a OwnerGrantSuccessionForm>,
    requested: Option<Succession>,
    applicants: &'a [String],
    account: &'a str,
    original: bool,
    in_flight: bool,
    /// The owner submitted terms in this window, so a result is theirs.
    submitted: bool,
    /// That submission is still moving through its native reviews.
    submitting: bool,
    publishing: bool,
    cancelling: bool,
    regrant: bool,
    active: bool,
    can_choose_new_limits: bool,
    succession_supported: Option<bool>,
    disclosure: Disclosure,
}
impl GrantState<'_> {
    fn actionable(&self) -> bool {
        !self.in_flight && self.active
    }
    /// The bounds of whichever native form the Choose phase shows.
    fn bounds(&self) -> Option<Bounds> {
        match (self.successor, self.form) {
            (Some(form), _) => Some(successor_bounds(form)),
            (None, Some(form)) => Some(root_bounds(form)),
            _ => None,
        }
    }
    /// A proposal beside this month's published grant may open the native
    /// amendment form, but only once the server is known to support it.
    fn amendable(&self) -> bool {
        self.proposal.is_some() && self.succession_supported == Some(true)
    }
    /// An amendment confirms only a real change: a new limit or new people.
    fn changes_something(&self) -> bool {
        match (self.phase, self.successor) {
            (Phase::Choose, Some(form)) if !renewal(form) => {
                self.draft.tokens.trim().parse::<u64>().ok() != Some(form.predecessor.allocated_tokens)
                    || !added_people(form, &managers_of(&self.draft.managers)).is_empty()
            }
            _ => true,
        }
    }
    fn can_primary(&self, now: u64) -> bool {
        self.actionable()
            && primary_available(
                self.phase,
                self.bounds().as_ref(),
                self.draft,
                self.original,
                self.can_choose_new_limits,
                self.amendable(),
                now,
            )
            && self.changes_something()
    }
    fn can_recover(&self) -> bool {
        self.actionable()
            && !matches!(self.phase, Phase::Loading)
            && !matches!(self.phase, Phase::Review(review) if review.capacity_available())
            && !matches!(self.phase, Phase::PublicationReview(review) if !review.expired())
            && !matches!(self.phase, Phase::CancellationReview(review) if !review.expired())
    }
    fn can_cancel_intent(&self) -> bool {
        self.actionable()
            && self.original
            && !self.publishing
            && !self.cancelling
            && match self.phase {
                Phase::Review(_) | Phase::Failed(_) => true,
                Phase::Progress(p) => matches!(
                    p.state,
                    OwnerGrantState::ReviewRequired | OwnerGrantState::ReviewExpired
                ),
                _ => false,
            }
    }
}
/// Everything the window shows, already in the owner's language.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct GrantView {
    pub title: String,
    pub pool: String,
    pub lead: String,
    pub status: String,
    pub notice: Option<String>,
    pub choosing: bool,
    pub people_visible: bool,
    pub people_label: String,
    /// Each person by name, and whether this change adds them.
    pub people: Vec<(String, bool)>,
    pub people_note: String,
    pub people_editable: bool,
    pub people_editing: bool,
    pub editor_hint: String,
    pub quota_visible: bool,
    pub quota_hint: String,
    pub advanced_open: bool,
    pub expiry: String,
    pub expiry_example: String,
    pub technical: String,
    pub technical_open: bool,
    pub primary: String,
    pub primary_visible: bool,
    pub primary_enabled: bool,
    pub cancel: String,
    pub recover: String,
    pub recover_visible: bool,
    pub cancel_intent_visible: bool,
}
fn phase_summary(phase: &Phase) -> Option<OwnerGrantSummary> {
    match phase {
        Phase::Review(review) => Some(review.summary()),
        Phase::Progress(p) => Some(p.summary.clone()),
        Phase::PublicationReview(review) => Some(review.summary()),
        Phase::PublicationProgress(p) => Some(p.summary.clone()),
        Phase::CancellationReview(review) => Some(review.summary()),
        _ => None,
    }
}
/// People on `managers` who are not on the published list being changed.
fn added_people(form: &OwnerGrantSuccessionForm, managers: &[String]) -> Vec<String> {
    managers
        .iter()
        .filter(|id| !form.predecessor.eligible_managers.contains(id))
        .cloned()
        .collect()
}
fn grant_kind(s: &GrantState) -> GrantKind {
    if let Some(form) = s.successor {
        if renewal(form) {
            return GrantKind::Renew;
        }
        let (tokens, managers) = match phase_summary(s.phase) {
            Some(summary) if summary.succession.is_some() => {
                (Some(summary.allocated_tokens), summary.eligible_managers)
            }
            _ => (s.draft.tokens.trim().parse::<u64>().ok(), managers_of(&s.draft.managers)),
        };
        let only_people = tokens == Some(form.predecessor.allocated_tokens)
            && (!added_people(form, &managers).is_empty() || !s.applicants.is_empty());
        return if only_people { GrantKind::AddPeople } else { GrantKind::Amend };
    }
    match s.requested {
        Some(Succession::Renewal) => GrantKind::Renew,
        Some(Succession::Amendment) if !s.applicants.is_empty() => GrantKind::AddPeople,
        Some(Succession::Amendment) => GrantKind::Amend,
        None if !s.can_choose_new_limits => GrantKind::Record,
        None => GrantKind::Publish,
    }
}
/// The month a window is about: the record's, the form's, or this UTC month
/// for a new publish. Unknown for a renewal whose form has not loaded.
fn grant_month(s: &GrantState, summary: Option<&OwnerGrantSummary>, now: u64) -> Option<String> {
    if let Some(summary) = summary {
        return key_month(&summary.period_key);
    }
    if let Some(form) = s.successor {
        return key_month(&form.period_key);
    }
    match s.requested {
        Some(Succession::Renewal) => None,
        _ => Some(ms_month(now)),
    }
}
fn grant_title(kind: GrantKind, s: &GrantState, month: Option<&str>) -> String {
    let tr = crate::i18n::tr;
    let fmt = |key: &str, month: &str| crate::i18n::format(key, &[("month", month.to_owned())]);
    if matches!(s.phase, Phase::CancellationReview(_)) {
        return tr("Discard Unfinished Publish").into();
    }
    match (kind, month) {
        (GrantKind::Publish, Some(month)) => fmt("Publish {month}", month),
        (GrantKind::Publish, None) => tr("Publish This Pool").into(),
        (GrantKind::Amend, _) => tr("Change Per-Request Limit").into(),
        (GrantKind::AddPeople, _) => tr("Add People Who Can Request").into(),
        (GrantKind::Renew, Some(month)) => fmt("Publish {month} Ahead", month),
        (GrantKind::Renew, None) => tr("Publish Next Month Ahead").into(),
        (GrantKind::Record, Some(month)) => fmt("{month} Capacity", month),
        (GrantKind::Record, None) => tr("This Pool’s Capacity").into(),
    }
}
/// The pool and its model, so the owner knows which pool this is.
fn pool_line(s: &GrantState) -> String {
    let (label, model) = match (s.successor, s.form, s.phase) {
        (Some(form), _, _) => (form.label.clone(), form.context.model.clone()),
        (None, Some(form), _) => (form.label.clone(), form.context.model.clone()),
        (None, None, Phase::Review(review)) => (String::new(), review.context().model.clone()),
        _ => (String::new(), String::new()),
    };
    match (label.is_empty(), model.is_empty()) {
        (false, false) if label != model => format!("{label} · {model}"),
        (false, _) => label,
        (true, false) => model,
        (true, true) => String::new(),
    }
}
/// One sentence naming the change from the terms shown: the limit, its
/// previous value for a successor, and the people this change adds.
fn terms_lead(
    kind: GrantKind,
    month: Option<&str>,
    current: Option<u64>,
    tokens: Option<u64>,
    added: &[String],
    requested: &[String],
) -> String {
    let fmt = crate::i18n::format;
    let new = tokens.map_or_else(|| "—".to_owned(), |n| amount(Some(n)));
    let current_text = amount(current);
    let month = month.map(str::to_owned).unwrap_or_else(|| crate::i18n::tr("next month").into());
    match kind {
        GrantKind::Publish | GrantKind::Record => fmt(
            "Publish {month}: up to {amount} per request",
            &[("month", month), ("amount", new)],
        ),
        GrantKind::Amend if tokens == current && added.is_empty() => fmt(
            "The per-request limit is {current}. Choose a new amount or add people.",
            &[("current", current_text)],
        ),
        GrantKind::Amend => {
            let change = fmt(
                "Change the per-request limit from {current} to {new}",
                &[("current", current_text), ("new", new)],
            );
            if added.is_empty() {
                change
            } else {
                fmt(
                    "{change}, and add {people} to this month’s list",
                    &[("change", change), ("people", people_text(added))],
                )
            }
        }
        GrantKind::AddPeople if added.is_empty() && !requested.is_empty() => fmt(
            "{people} are already on this month’s list",
            &[("people", people_text(requested))],
        ),
        GrantKind::AddPeople => fmt(
            "Add {people} to this month’s list",
            &[("people", people_text(added))],
        ),
        GrantKind::Renew => {
            let lead = if tokens == current || current.is_none() {
                fmt(
                    "Publish {month} ahead: up to {amount} per request; the list and monthly maximum carry over",
                    &[("month", month), ("amount", new)],
                )
            } else {
                fmt(
                    "Publish {month} ahead: per-request limit from {current} to {new}; the list and monthly maximum carry over",
                    &[("month", month), ("current", current_text), ("new", new)],
                )
            };
            if added.is_empty() {
                lead
            } else {
                fmt("{lead}, and add {people}", &[("lead", lead), ("people", people_text(added))])
            }
        }
    }
}
/// The first thing wrong with what the native fields hold, said beside them.
/// It mirrors `choices_valid`; the writer still re-checks everything.
fn draft_problem(bounds: &Bounds, draft: &GrantChoices, renewing: Option<&str>, now: u64) -> Option<String> {
    let tr = crate::i18n::tr;
    let fmt = crate::i18n::format;
    let tokens = match draft.tokens.trim().parse::<u64>() {
        Ok(0) | Err(_) => return Some(tr("Enter a per-request limit above 0, for example 0.25.").into()),
        Ok(tokens) => tokens,
    };
    if let Some(remaining) = bounds.remaining.filter(|remaining| tokens > *remaining) {
        return Some(fmt("The per-request limit can be at most {max}.", &[("max", amount(Some(remaining)))]));
    }
    if tokens < bounds.minimum {
        return Some(fmt(
            "It can’t be lower than the {held} running Agents already hold.",
            &[("held", amount(Some(bounds.minimum)))],
        ));
    }
    let managers = managers_of(&draft.managers);
    if managers.is_empty() {
        return Some(tr("Add at least one person who can request.").into());
    }
    if managers.len() > 64 {
        return Some(tr("At most 64 people can be on the list.").into());
    }
    if managers.iter().any(|id| !crate::host::hagency::grants::matrix_user(id)) {
        return Some(tr("Use full account names (@name:server), separated by spaces.").into());
    }
    if !bounds.keep_managers.iter().all(|id| managers.contains(id)) {
        return Some(tr("This month’s list can only grow: keep everyone already on it.").into());
    }
    let expiry = chrono::DateTime::parse_from_rfc3339(draft.expires_utc.trim())
        .ok()
        .filter(|at| at.offset().local_minus_utc() == 0)
        .and_then(|at| u64::try_from(at.timestamp_millis()).ok());
    match expiry {
        None => Some(tr("Choose when sharing ends under Advanced.").into()),
        Some(at) if at <= now.max(bounds.expiry_after_ms) || at > bounds.expiry_until_ms => Some(match renewing {
            Some(month) => fmt("Sharing must end within {month}.", &[("month", month.to_owned())]),
            None => tr("Sharing must end within this month.").into(),
        }),
        Some(_) => None,
    }
}
/// Sentences joined the way each language writes them.
fn sentences(parts: &[String]) -> String {
    let separator = if crate::i18n::language() == crate::i18n::Language::Chinese { "" } else { " " };
    parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(separator)
}
/// Codes refused before anything was written: capacity and the list stay.
fn unchanged_failure(code: &str) -> bool {
    matches!(
        code,
        "invalid_grant_budget"
            | "invalid_grant_expiry"
            | "invalid_grant_manager"
            | "owner_grant_capacity_unavailable"
            | "management_grant_succession_required"
            | "pool_grant_succession_unavailable"
            | "owner_grant_predecessor_required"
            | "grant_predecessor_not_effective"
            | "owner_grant_succession_pending"
            | "owner_grant_successor_exists"
            | "grant_successor_exists"
            | "owner_grant_renewal_window_closed"
            | "grant_succession_window_closed"
            | "owner_grant_below_floor"
            | "grant_successor_below_reserved"
            | "owner_grant_manager_removed"
            | "management_monthly_regrant_required"
            | "management_admission_receipts_required"
            | "owner_grant_prior_intent_required"
            | "owner_grant_new_admission_required"
            | "management_features_invalid"
    )
}
/// The collapsed technical details: servers, accounts, IDs, revisions and
/// UTC periods. None of this is needed to decide, so it starts hidden.
fn technical_rows(s: &GrantState, summary: Option<&OwnerGrantSummary>, now: u64) -> String {
    let mut rows: Vec<(&str, String)> = vec![("Your account", s.account.to_owned())];
    let context = match (s.successor, s.form, s.phase) {
        (_, _, Phase::Review(review)) => Some(review.context().clone()),
        (Some(form), _, _) => Some(form.context.clone()),
        (None, Some(form), _) => Some(form.context.clone()),
        _ => None,
    };
    match s.phase {
        Phase::PublicationReview(review) => {
            rows.push(("Server", review.server_origin().to_owned()));
            rows.push(("Pool revision", review.contribution_revision().to_string()));
            rows.push(("Read at (UTC)", utc(review.observed_at_ms())));
        }
        Phase::CancellationReview(review) => rows.push(("Server", review.server_origin().to_owned())),
        Phase::PublicationProgress(p) => {
            if let Some(server) = &p.server_state {
                let state = match server.as_str() {
                    "active" if p.summary.period_start_ms > now => "Scheduled",
                    "active" => "In effect",
                    "revoked" => "Withdrawn",
                    "expired" => "Expired",
                    "superseded" => "Replaced by a newer change",
                    _ => "Unknown",
                };
                rows.push(("Server status", crate::i18n::tr(state).into()));
            }
            if let Some(at) = p.observed_at_ms {
                rows.push(("Read at (UTC)", utc(at)));
            }
        }
        Phase::Progress(p) => {
            if let Some(closure) = &p.closure {
                rows.push(("Kept by Agents", amount(Some(closure.retained_tokens))));
                rows.push(("Confirmed use", amount(Some(closure.known_consumed_tokens))));
                rows.push(("Not yet confirmed", amount(Some(closure.unresolved_tokens))));
                rows.push(("Read at (UTC)", utc(closure.observed_at_ms)));
            }
        }
        _ => {}
    }
    if let Some(c) = &context {
        rows.push(("Server", c.server_origin.clone()));
        rows.push(("Local profile", c.profile_id.clone()));
        rows.push(("Left / total", format!("{} / {}", amount(c.remaining_tokens), amount(c.resource_total_tokens))));
        if c.shared_account_total_tokens.is_some() {
            rows.push(("Account limit", amount(c.shared_account_total_tokens)));
        }
        rows.push(("Resource revision", c.resource_revision.clone()));
        rows.push(("Read at (UTC)", utc(c.observed_at_ms)));
    }
    match summary {
        Some(summary) => {
            rows.push(("Pool ID", summary.contribution_id.clone()));
            rows.push(("Resource ID", summary.resource_id.clone()));
            rows.push(("Record ID", summary.intent_id.clone()));
            rows.push(("Grant ID", summary.grant_id.clone()));
            rows.push(("Per-request limit", amount(Some(summary.allocated_tokens))));
            rows.push((
                "Month (UTC)",
                format!("{}\n{} → {}", summary.period_key, utc(summary.period_start_ms), utc(summary.period_end_ms)),
            ));
            rows.push(("Approves requests", summary.coordinator_mxid.clone()));
            rows.push(("People who can request", summary.eligible_managers.join("\n")));
            rows.push(("Shared until (UTC)", utc(summary.delegation_expires_at_ms.min(summary.period_end_ms))));
            if let Some(succession) = &summary.succession {
                rows.push((
                    "Kind of change",
                    crate::i18n::tr(match succession.kind {
                        OwnerGrantSuccessionKind::Renewal => "Next month, published ahead",
                        _ => "Changed this month",
                    })
                    .into(),
                ));
                rows.push(("Previous grant ID", succession.predecessor_grant_id.clone()));
                rows.push(("Previous per-request limit", amount(Some(succession.predecessor_allocated_tokens))));
                rows.push(("Running Agents", succession.carried_children.to_string()));
                rows.push(("Held by running Agents", amount(Some(succession.carried_retained_tokens))));
            }
        }
        None => {
            if let Some(form) = s.successor {
                rows.push(("Pool ID", form.contribution_id.clone()));
                rows.push(("Resource ID", form.resource_id.clone()));
                rows.push(("Previous grant ID", form.predecessor.grant_id.clone()));
                rows.push((
                    "Month (UTC)",
                    format!("{}\n{} → {}", form.period_key, utc(form.period_start_ms), utc(form.period_end_ms)),
                ));
                rows.push(("Running Agents", form.floor.children.to_string()));
                rows.push(("Held by running Agents", amount(Some(form.floor.retained_tokens))));
                rows.push(("People who can request", form.predecessor.eligible_managers.join("\n")));
            } else if let Some(form) = s.form {
                rows.push(("Pool ID", form.contribution_id.clone()));
                rows.push(("Resource ID", form.resource_id.clone()));
            }
        }
    }
    rows.into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(label, value)| format!("{}\n{}", crate::i18n::tr(label), value))
        .collect::<Vec<_>>()
        .join("\n\n")
}
/// Everything one rendering of the panel shows. Pure: it reads native state
/// only, so the owner sees the same words in the panel and in its preview.
fn grant_view(s: &GrantState, now: u64) -> GrantView {
    let tr = crate::i18n::tr;
    let fmt = crate::i18n::format;
    let kind = grant_kind(s);
    let summary = phase_summary(s.phase);
    let month = grant_month(s, summary.as_ref(), now);
    let choosing = matches!(s.phase, Phase::Choose);
    let bounds = s.bounds();
    let mut v = GrantView {
        title: grant_title(kind, s, month.as_deref()),
        pool: pool_line(s),
        choosing,
        people_label: tr("People who can request").into(),
        cancel: tr(if choosing || matches!(s.phase, Phase::Review(_) | Phase::PublicationReview(_) | Phase::CancellationReview(_)) || (matches!(s.phase, Phase::Loading) && !s.submitting) {
            "Cancel"
        } else {
            "Close"
        })
        .into(),
        recover: tr(if s.original { "Check Again" } else { "Refresh" }).into(),
        recover_visible: s.can_recover(),
        cancel_intent_visible: s.can_cancel_intent(),
        technical: if matches!(s.phase, Phase::Loading) { String::new() } else { technical_rows(s, summary.as_ref(), now) },
        technical_open: s.disclosure.technical,
        primary_enabled: s.can_primary(now),
        ..GrantView::default()
    };
    let current = s.successor.map(|form| form.predecessor.allocated_tokens);
    let base: Vec<String> = s.successor.map(|form| form.predecessor.eligible_managers.clone()).unwrap_or_default();
    let people = |ids: &[String]| -> Vec<(String, bool)> {
        ids.iter().map(|id| (person(id).to_owned(), s.successor.is_some() && !base.contains(id))).collect()
    };
    let renew_month = month.clone().filter(|_| kind == GrantKind::Renew);
    let add_only_note = match kind {
        GrantKind::Renew => tr("The list carries over; you can add people but not remove them."),
        GrantKind::Amend | GrantKind::AddPeople => tr("You can add people this month but not remove them."),
        _ => "",
    };
    let mut primary = "Publish";
    let mut primary_text: Option<String> = None;
    match s.phase {
        Phase::Loading => {
            v.status = tr(match (s.submitting, kind) {
                (true, GrantKind::Publish | GrantKind::Renew | GrantKind::Record) => "Publishing…",
                (true, _) => "Saving your change…",
                (false, _) => "Reading this pool…",
            })
            .into();
        }
        Phase::Choose => {
            let managers = managers_of(&s.draft.managers);
            let tokens = s.draft.tokens.trim().parse::<u64>().ok();
            let added = s.successor.map(|form| added_people(form, &managers)).unwrap_or_default();
            v.lead = terms_lead(kind, month.as_deref(), current, tokens, &added, s.applicants);
            let capacity_known = bounds.as_ref().is_some_and(|b| b.total_known && b.remaining.is_some());
            v.status = if !capacity_known {
                tr(if s.regrant { REGRANT_UNKNOWN_STATUS } else { UNKNOWN_CAPACITY_STATUS }).into()
            } else if bounds.as_ref().is_some_and(|b| b.remaining == Some(0)) {
                tr("Nothing is left to share this month. Raise the monthly maximum on the pool’s page first.").into()
            } else if let Some(problem) = bounds.as_ref().and_then(|b| draft_problem(b, s.draft, renew_month.as_deref(), now)) {
                problem
            } else {
                match kind {
                    GrantKind::Publish | GrantKind::Record => tr("Once published, people on the list can request it until the end of the month.").into(),
                    GrantKind::Amend if !s.changes_something() => String::new(),
                    GrantKind::Amend => tr("Takes effect once you confirm; running Agents keep what they hold.").into(),
                    GrantKind::AddPeople if added.is_empty() => tr("Nothing needs to change.").into(),
                    GrantKind::AddPeople => tr("Once you confirm, they can request right away.").into(),
                    GrantKind::Renew => fmt(
                        "Starts {date}; running Agents carry on without requesting again.",
                        &[("date", month_day(s.successor.map_or(now, |form| form.period_start_ms)))],
                    ),
                }
            };
            v.people_visible = true;
            v.people = people(&managers);
            v.people_note = if managers.is_empty() {
                tr("No one yet. Add at least one person below.").into()
            } else {
                add_only_note.into()
            };
            v.people_editable = true;
            v.people_editing = s.disclosure.people || managers.is_empty();
            let server = s.account.split_once(':').map_or("server", |(_, server)| server);
            v.editor_hint = fmt(
                "Enter full account names separated by spaces, for example {example}.",
                &[("example", format!("@lisi:{server}"))],
            );
            v.quota_visible = kind != GrantKind::AddPeople || s.disclosure.advanced;
            if let Some(b) = &bounds {
                let mut hint = Vec::new();
                if let Some(remaining) = b.remaining {
                    hint.push(fmt("Up to {max}.", &[("max", amount(Some(remaining)))]));
                }
                if b.minimum > 1 {
                    hint.push(fmt(
                        "Not below the {held} running Agents already hold.",
                        &[("held", amount(Some(b.minimum)))],
                    ));
                }
                if kind == GrantKind::Amend && tokens.is_some_and(|n| current.is_some_and(|c| n < c)) {
                    hint.push(fmt(
                        "Waiting requests above {new} can’t be approved.",
                        &[("new", amount(tokens))],
                    ));
                }
                v.quota_hint = sentences(&hint);
                let expiry = chrono::DateTime::parse_from_rfc3339(s.draft.expires_utc.trim())
                    .ok()
                    .and_then(|at| u64::try_from(at.timestamp_millis()).ok());
                let end_month = ms_month(b.expiry_until_ms.saturating_sub(1));
                v.expiry = match expiry {
                    Some(at) if at == b.expiry_until_ms => fmt(
                        "Sharing lasts until the end of {month}",
                        &[("month", end_month)],
                    ),
                    Some(at) => fmt("Sharing ends {time}", &[("time", local_time(at))]),
                    None => tr("Choose when sharing ends.").into(),
                };
                v.expiry_example = rfc3339(b.expiry_until_ms);
            }
            v.advanced_open = s.disclosure.advanced;
            primary = match kind {
                GrantKind::Publish | GrantKind::Record => "Publish",
                GrantKind::Amend if added.is_empty() => {
                    primary_text = Some(fmt("Change to {amount}", &[("amount", amount(tokens))]));
                    ""
                }
                GrantKind::Amend => "Confirm Changes",
                GrantKind::AddPeople => "Add to This Month’s List",
                GrantKind::Renew => "Publish Ahead",
            };
        }
        Phase::Review(review) => {
            let s2 = review.summary();
            v.lead = summary_lead(kind, month.as_deref(), &s2, &base);
            v.status = tr(if review.capacity_available() { "Not published yet. Publish to finish." } else { UNKNOWN_CAPACITY_STATUS }).into();
            if kind == GrantKind::Renew { primary = "Publish Ahead"; }
        }
        Phase::PublicationReview(review) => {
            let s2 = review.summary();
            v.lead = summary_lead(kind, month.as_deref(), &s2, &base);
            v.status = tr("Not published yet. Publish to finish.").into();
            if kind == GrantKind::Renew { primary = "Publish Ahead"; }
        }
        Phase::Progress(p) => {
            if p.closure.is_some() {
                v.lead = tr("The previous publish has ended.").into();
                v.status = tr("What Agents hold still counts. To publish again, the server administrator must approve this pool again.").into();
                primary = "Publish Again";
            } else {
                match p.state {
                    OwnerGrantState::Recorded => {
                        v.lead = summary_lead(kind, month.as_deref(), &p.summary, &base);
                        v.status = tr("Saved on this computer but not published yet. Publish to finish.").into();
                        if kind == GrantKind::Renew { primary = "Publish Ahead"; }
                    }
                    OwnerGrantState::ReviewRequired => {
                        v.lead = tr("This publish isn’t finished.").into();
                        v.status = tr("Confirm it again, or discard it.").into();
                        primary = "Continue";
                    }
                    OwnerGrantState::OutcomeUnknown => {
                        v.lead = tr("It isn’t certain yet whether this was published.").into();
                        v.status = tr("Tap Check Again; don’t publish it a second time.").into();
                    }
                    OwnerGrantState::Declined => {
                        v.lead = tr("This publish was discarded; no capacity is held.").into();
                        primary = "Choose Again";
                    }
                    OwnerGrantState::ReviewExpired => {
                        v.lead = tr("This publish belongs to a month that has ended, so it can’t continue.").into();
                    }
                }
            }
        }
        Phase::PublicationProgress(p) => {
            let started = p.summary.period_start_ms <= now;
            match (&p.state, p.server_state.as_deref()) {
                (OwnerGrantPublicationState::ServerObserved, Some("active")) if !started => {
                    let month = key_month(&p.summary.period_key).unwrap_or_default();
                    let date = month_day(p.summary.period_start_ms);
                    v.lead = if s.submitted {
                        fmt("Published {month} ahead. It starts {date}.", &[("month", month), ("date", date)])
                    } else {
                        fmt("{month} is published ahead and starts {date}.", &[("month", month), ("date", date)])
                    };
                }
                (OwnerGrantPublicationState::ServerObserved, Some("active")) => {
                    let limit = amount(Some(p.summary.allocated_tokens));
                    let added = s.successor.map(|form| added_people(form, &p.summary.eligible_managers)).unwrap_or_default();
                    v.lead = match (s.submitted, kind) {
                        (true, GrantKind::AddPeople) => fmt(
                            "Added {people} to this month’s list. They can request now.",
                            &[("people", people_text(&added))],
                        ),
                        (true, GrantKind::Amend) => fmt(
                            "Changed. Up to {amount} per request now; running Agents keep what they hold.",
                            &[("amount", limit)],
                        ),
                        (true, _) => fmt("Published. Up to {amount} per request.", &[("amount", limit)]),
                        (false, _) => fmt(
                            "{month} is published: up to {amount} per request.",
                            &[("month", key_month(&p.summary.period_key).unwrap_or_default()), ("amount", limit)],
                        ),
                    };
                }
                (OwnerGrantPublicationState::ServerObserved, Some("revoked")) => {
                    v.lead = tr("This publish was withdrawn and no longer applies.").into();
                }
                (OwnerGrantPublicationState::ServerObserved, Some("expired")) => {
                    v.lead = tr("This publish has expired.").into();
                }
                (OwnerGrantPublicationState::ServerObserved, Some("superseded")) => {
                    v.lead = tr("A newer change replaced this publish.").into();
                }
                (OwnerGrantPublicationState::ServerObserved, _) => {
                    v.lead = tr("The server’s status isn’t known yet.").into();
                }
                (OwnerGrantPublicationState::OutcomeUnknown, _) => {
                    v.lead = tr("It isn’t certain yet whether this was published.").into();
                    v.status = tr("Tap Check Again; don’t publish it a second time.").into();
                }
                (OwnerGrantPublicationState::Rejected, _) => {
                    v.lead = tr("The server didn’t accept this publish.").into();
                    v.status = tr("No capacity is held and nothing changed.").into();
                }
            }
            let done = matches!(p.state, OwnerGrantPublicationState::ServerObserved)
                && p.server_state.as_deref() == Some("active");
            if done && s.submitted {
                v.status = tr("You can close this window.").into();
                v.cancel = tr("Done").into();
                v.recover_visible = false;
            }
        }
        Phase::CancellationReview(_) => {
            v.lead = tr("Discard this unfinished publish?").into();
            v.status = tr("Only this unfinished publish on this computer is discarded; the pool and earlier publishes stay as they are. Closing the window keeps it.").into();
            primary = "Discard This Publish";
        }
        Phase::Failed(code) => {
            v.lead = tr(failure_message(code)).into();
            if unchanged_failure(code) {
                v.status = tr("Capacity and the list were not changed.").into();
            }
        }
    }
    if let Some(summary) = summary.as_ref().filter(|_| !choosing) {
        v.people_visible = !summary.eligible_managers.is_empty();
        v.people = people(&summary.eligible_managers);
        v.people_note = if kind == GrantKind::Record || !s.can_choose_new_limits {
            tr("To add someone, use Who Can Request on the pool’s page.").into()
        } else {
            String::new()
        };
    }
    if ended_grant(s.phase, now) {
        v.title = fmt("Publish {month}", &[("month", ms_month(now))]);
        v.lead = tr(ENDED_LEAD).into();
        v.status = tr(ENDED_STATUS).into();
        primary = "Set This Month’s Capacity";
        primary_text = None;
    }
    if published_grant(s.phase, now) && s.amendable() && live_grant_tokens(s.phase, now).is_some() {
        primary = "Change Per-Request Limit";
        primary_text = None;
    }
    if !s.active {
        v.status = tr("The account or session changed. Sign in again, then open this again.").into();
    }
    // Beside a usable form, Refresh would only reload what is shown; keep it
    // for an unknown or used-up capacity, where reading again can help.
    if choosing && bounds.as_ref().is_some_and(|b| b.total_known && b.remaining.is_some_and(|n| n > 0)) {
        v.recover_visible = false;
    }
    v.notice = if choosing {
        None
    } else {
        proposal_notice(s.phase, s.proposal, &s.draft.tokens, s.succession_supported == Some(true), now)
    };
    v.primary = primary_text.unwrap_or_else(|| tr(primary).into());
    v.primary_visible = choosing || v.primary_enabled;
    v
}
/// The terms a saved or reviewed record holds, as one sentence.
fn summary_lead(kind: GrantKind, month: Option<&str>, summary: &OwnerGrantSummary, base: &[String]) -> String {
    let added: Vec<String> = if base.is_empty() {
        Vec::new()
    } else {
        summary.eligible_managers.iter().filter(|id| !base.contains(id)).cloned().collect()
    };
    let current = summary.succession.as_ref().map(|s| s.predecessor_allocated_tokens);
    terms_lead(kind, month, current, Some(summary.allocated_tokens), &added, &[])
}
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
            Some("This amount is filled in when you set this month’s capacity."),
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
            (Some(proposal), Some("This amount is filled in when you choose again."))
        }
        Phase::Progress(p)
            if matches!(p.state, OwnerGrantState::ReviewRequired | OwnerGrantState::ReviewExpired) =>
        {
            (Some(proposal), Some(PENDING_NOTE))
        }
        // Reviews of an existing, unattempted original; submitting the form
        // clears the proposal before its own review appears.
        Phase::Review(_) | Phase::CancellationReview(_) => (Some(proposal), Some(PENDING_NOTE)),
        Phase::Failed(_) => (Some(proposal), Some("This amount wasn’t applied.")),
        _ => (
            Some(proposal),
            Some("This amount doesn’t change the existing publish; it keeps its own amounts."),
        ),
    };
    // A failure says only that the amount was not used: what is in force
    // is not known from it, so no current amount is claimed.
    if matches!(phase, Phase::Failed(_)) {
        return note.map(|note| crate::i18n::tr(note).to_owned());
    }
    let current = live_grant_tokens(phase, now)
        .map(|tokens| amount(Some(tokens)))
        .unwrap_or_else(|| crate::i18n::tr("Not in effect").into());
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
            "This confirmation expired. Check the record again."
        }
        "native_grant_not_publishable" => {
            "This saved publish can’t be published any more. Its record is kept."
        }
        "owner_grant_publication_history_unavailable" => {
            "This computer’s publish history is incomplete, so nothing new can be created. The saved record is kept."
        }
        "owner_grant_publication_review_changed" => {
            "This pool changed on the server. Close this window and refresh."
        }
        "owner_grant_review_changed" => "This confirmation changed or expired. Check the record again.",
        "management_enrollment_changed" | "contribution_not_admitted" => {
            "The server’s approval of this pool changed. Check the record; an unfinished publish can still be discarded."
        }
        "owner_grant_prior_intent_required" => "Finish or discard earlier publishes first.",
        "management_admission_receipts_required" => {
            "The server isn’t accepting publishes for this pool. Ask its administrator."
        }
        "management_monthly_regrant_required" => {
            "This server can’t take a new month for this pool yet. Ask its administrator to update the server."
        }
        "management_features_invalid" => {
            "Couldn’t confirm what the server supports. Refresh, then try again."
        }
        "management_scope_changed" | "management_access_required" => {
            "The account or session changed. Sign in again, then open this again."
        }
        "invalid_grant_expiry" => "Choose when sharing ends under Advanced.",
        "owner_grant_new_admission_required" => {
            "The server administrator must approve this pool again before it can be published."
        }
        "owner_grant_capacity_unavailable" => {
            "There isn’t enough capacity left, or it isn’t known yet. Nothing was published."
        }
        "invalid_grant_budget" => {
            "Enter a per-request limit above 0 and at least one person who can request."
        }
        "invalid_grant_manager" => "Use full account names (@name:server), separated by spaces.",
        "management_grant_succession_required" | "pool_grant_succession_unavailable" => {
            "This server can’t change a published month or publish ahead yet. Ask its administrator to update the server."
        }
        "owner_grant_predecessor_required" | "grant_predecessor_not_effective" => {
            "This pool has nothing published this month to change. Publish it first."
        }
        "owner_grant_succession_pending" => {
            "An earlier change to this pool isn’t finished. Finish or discard it first."
        }
        "owner_grant_successor_exists" | "grant_successor_exists" => {
            "Next month is already published ahead, so this month can’t change now."
        }
        "owner_grant_renewal_window_closed" | "grant_succession_window_closed" => {
            "Next month can be published ahead only in the last 7 days of this month."
        }
        "owner_grant_below_floor" | "grant_successor_below_reserved" => {
            "The new limit is lower than what running Agents already hold. Choose a larger amount."
        }
        "owner_grant_manager_removed" => {
            "This month’s list can only grow: keep everyone already on it."
        }
        _ => "The result isn’t confirmed yet. Tap Check Again before trying anything else.",
    }
}
/// Values the native fields are refilled with when the panel is presented.
pub(super) struct GrantInputs {
    managers: String,
    tokens: String,
    expires: String,
    minimum: u64,
    maximum: Option<u64>,
    presets: [bool; 3],
}
impl GrantInputs {
    fn of(s: &GrantState, now: u64) -> Self {
        let bounds = s.bounds();
        let presets = [Some(1), Some(7), None].map(|days| match s.successor {
            Some(form) => successor_preset(form, days, now).is_some(),
            None => publication_preset_available(days),
        });
        Self {
            managers: s.draft.managers.clone(),
            tokens: s.draft.tokens.clone(),
            expires: s.draft.expires_utc.clone(),
            minimum: bounds.as_ref().map_or(1, |b| b.minimum),
            maximum: bounds.as_ref().map(|b| b.remaining.unwrap_or(0)),
            presets,
        }
    }
}
/// The panel's collapsible sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GrantSection {
    People,
    Advanced,
    Technical,
}
impl GrantPrompt {
    /// The native facts this panel shows, with the flow's facts copied in.
    fn state(&self) -> GrantState<'_> {
        GrantState {
            phase: &self.phase,
            draft: &self.draft,
            proposal: self.proposal,
            form: self.form.as_ref(),
            successor: self.successor.as_ref(),
            requested: self.flow.requested_succession(),
            applicants: self.flow.applicants(),
            account: self.flow.account(),
            original: self.original.is_some(),
            in_flight: self.receiver.is_some(),
            submitted: self.submitted,
            submitting: self.submit_publication,
            publishing: self.publishing,
            cancelling: self.cancelling,
            regrant: self.regrant,
            active: self.flow.active(),
            can_choose_new_limits: self.flow.can_choose_new_limits(),
            succession_supported: self.flow.succession_supported(),
            disclosure: self.disclosure,
        }
    }
    fn actionable(&self) -> bool {
        self.state().actionable()
    }
    /// The owner submits the terms shown natively for reservation and
    /// publication; a script proposal plays no further part in this window.
    fn submit(&mut self) {
        self.submit_publication = true;
        self.submitted = true;
        self.proposal = None;
    }
    /// A proposal beside this month's published grant may open the native
    /// amendment form, but only once the server is known to support it.
    fn amendable(&self) -> bool {
        self.state().amendable()
    }
    fn can_primary(&self) -> bool {
        self.state().can_primary(now_ms())
    }
    fn can_recover(&self) -> bool {
        self.state().can_recover()
    }
    fn can_cancel_intent(&self) -> bool {
        self.state().can_cancel_intent()
    }
}
/// The panel's own argument for the native successor kind.
fn succession_arg(kind: OwnerGrantSuccessionKind) -> Succession {
    match kind {
        OwnerGrantSuccessionKind::Renewal => Succession::Renewal,
        _ => Succession::Amendment,
    }
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
            draft: initial_draft(proposal, flow.applicants()),
            publishing: false,
            cancelling: false,
            form: None,
            submit_publication: false,
            proposal,
            regrant: false,
            successor: None,
            submitted: false,
            disclosure: Disclosure::default(),
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
        let state = prompt.state();
        let view = grant_view(&state, now);
        let inputs = GrantInputs::of(&state, now);
        self.apply_grant_view(cx, &view, Some(&inputs));
        self.request_confirmation_redraw(cx);
    }
    /// Write one computed view into the panel. `inputs`, when given, also
    /// refills the native fields; the draft is always read back from them.
    fn apply_grant_view(&mut self, cx: &mut Cx, view: &GrantView, inputs: Option<&GrantInputs>) {
        let tr = crate::i18n::tr;
        let label = |this: &mut Self, cx: &mut Cx, path: &[LiveId], text: &str| {
            this.view.label(cx, path).set_text(cx, text);
            this.view.widget(cx, path).set_visible(cx, !text.is_empty());
        };
        self.view
            .label(cx, ids!(grant_confirmation.scroll.heading.title))
            .set_text(cx, &view.title);
        label(self, cx, ids!(grant_confirmation.scroll.pool), &view.pool);
        label(self, cx, ids!(grant_confirmation.scroll.lead), &view.lead);
        label(self, cx, ids!(grant_confirmation.scroll.status), &view.status);
        label(self, cx, ids!(grant_confirmation.scroll.change), view.notice.as_deref().unwrap_or_default());
        // The people on the list, one checked row each; new ones are marked.
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.people))
            .set_visible(cx, view.people_visible);
        self.view
            .label(cx, ids!(grant_confirmation.scroll.people.people_label))
            .set_text(cx, &view.people_label);
        let rows = [
            ids!(grant_confirmation.scroll.people.list.p0),
            ids!(grant_confirmation.scroll.people.list.p1),
            ids!(grant_confirmation.scroll.people.list.p2),
            ids!(grant_confirmation.scroll.people.list.p3),
            ids!(grant_confirmation.scroll.people.list.p4),
            ids!(grant_confirmation.scroll.people.list.p5),
            ids!(grant_confirmation.scroll.people.list.p6),
            ids!(grant_confirmation.scroll.people.list.p7),
        ];
        let shown = if view.people.len() > rows.len() { rows.len() - 1 } else { view.people.len() };
        for (index, path) in rows.iter().enumerate() {
            let row = self.view.widget(cx, *path);
            row.set_visible(cx, index < shown);
            if let Some((name, added)) = view.people.get(index).filter(|_| index < shown) {
                row.label(cx, ids!(name)).set_text(cx, name);
                row.label(cx, ids!(tag)).set_text(cx, if *added { tr("Newly added") } else { "" });
                row.widget(cx, ids!(tag)).set_visible(cx, *added);
            }
        }
        let more = view.people.len().saturating_sub(shown);
        let more_text = if more > 0 {
            crate::i18n::format("{count} more; see Edit List or Technical Details", &[("count", more.to_string())])
        } else {
            String::new()
        };
        label(self, cx, ids!(grant_confirmation.scroll.people.list.more), &more_text);
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.people.list))
            .set_visible(cx, !view.people.is_empty());
        label(self, cx, ids!(grant_confirmation.scroll.people.note), &view.people_note);
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.people.edit))
            .set_visible(cx, view.people_editable);
        self.view
            .button(cx, ids!(grant_confirmation.scroll.people.edit))
            .set_text(cx, tr(if view.people_editing { "Hide List Editor" } else { "Edit List" }));
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.people.editor))
            .set_visible(cx, view.people_editable && view.people_editing);
        self.view
            .label(cx, ids!(grant_confirmation.scroll.people.editor.editor_hint))
            .set_text(cx, &view.editor_hint);
        // The per-request limit, then the advanced choices and the details.
        self.view
            .view(cx, ids!(grant_confirmation.scroll.choices))
            .set_visible(cx, view.choosing);
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.choices.quota))
            .set_visible(cx, view.quota_visible);
        label(self, cx, ids!(grant_confirmation.scroll.choices.quota.quota_hint), &view.quota_hint);
        self.view
            .button(cx, ids!(grant_confirmation.scroll.disclosures.advanced_toggle))
            .set_text(cx, tr(if view.advanced_open { "Hide Advanced Options" } else { "Advanced Options" }));
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.disclosures.advanced_toggle))
            .set_visible(cx, view.choosing);
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.advanced))
            .set_visible(cx, view.choosing && view.advanced_open);
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.disclosures))
            .set_visible(cx, view.choosing || !view.technical.is_empty());
        self.view
            .label(cx, ids!(grant_confirmation.scroll.advanced.expiry))
            .set_text(cx, &view.expiry);
        self.view
            .text_input(cx, ids!(grant_confirmation.scroll.advanced.expires))
            .set_empty_text(cx, view.expiry_example.clone());
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.disclosures.technical_toggle))
            .set_visible(cx, !view.technical.is_empty());
        self.view
            .button(cx, ids!(grant_confirmation.scroll.disclosures.technical_toggle))
            .set_text(cx, tr(if view.technical_open { "Hide Technical Details" } else { "Technical Details" }));
        self.view
            .widget(cx, ids!(grant_confirmation.scroll.details))
            .set_visible(cx, view.technical_open && !view.technical.is_empty());
        self.view
            .label(cx, ids!(grant_confirmation.scroll.details.copy))
            .set_text(cx, &view.technical);
        if let Some(inputs) = inputs.filter(|_| view.choosing) {
            for (id, available) in [
                (ids!(grant_confirmation.scroll.advanced.expiry_presets.day), inputs.presets[0]),
                (ids!(grant_confirmation.scroll.advanced.expiry_presets.week), inputs.presets[1]),
                (ids!(grant_confirmation.scroll.advanced.expiry_presets.month), inputs.presets[2]),
            ] {
                self.view.button(cx, id).set_disabled(cx, !available);
            }
            let widget = self.view.widget(cx, ids!(grant_confirmation.scroll.choices.quota.tokens));
            if let Some(mut editor) = widget.borrow_mut::<crate::miniapps::design::MiniAppNumberInput>() {
                editor.set_minimum(cx, inputs.minimum);
                if let Some(maximum) = inputs.maximum {
                    editor.set_maximum(cx, maximum);
                }
            }
            self.view
                .widget(cx, ids!(grant_confirmation.scroll.people.editor.managers))
                .set_text(cx, &inputs.managers);
            self.view
                .widget(cx, ids!(grant_confirmation.scroll.choices.quota.tokens))
                .set_text(cx, &inputs.tokens);
            self.view
                .widget(cx, ids!(grant_confirmation.scroll.advanced.expires))
                .set_text(cx, &inputs.expires);
        }
        self.view
            .button(cx, ids!(grant_confirmation.buttons.cancel))
            .set_text(cx, &view.cancel);
        self.view
            .button(cx, ids!(grant_confirmation.buttons.primary))
            .set_text(cx, &view.primary);
        self.view
            .widget(cx, ids!(grant_confirmation.buttons.primary))
            .set_visible(cx, view.primary_visible);
        self.view
            .button(cx, ids!(grant_confirmation.buttons.primary))
            .set_disabled(cx, !view.primary_enabled);
        self.view
            .button(cx, ids!(grant_confirmation.buttons.recover))
            .set_text(cx, &view.recover);
        self.view
            .widget(cx, ids!(grant_confirmation.buttons.recover))
            .set_visible(cx, view.recover_visible);
        self.view
            .button(cx, ids!(grant_confirmation.buttons.cancel_intent))
            .set_text(cx, tr("Discard This Publish…"));
        self.view
            .widget(cx, ids!(grant_confirmation.buttons.cancel_intent))
            .set_visible(cx, view.cancel_intent_visible);
    }
    /// Open or close one section; the draft is read back first.
    pub(super) fn grant_disclose(&mut self, cx: &mut Cx, section: GrantSection) {
        self.capture_grant_choices(cx);
        let Some(prompt) = &mut self.grant_confirmation else {
            return;
        };
        let open = match section {
            GrantSection::People => &mut prompt.disclosure.people,
            GrantSection::Advanced => &mut prompt.disclosure.advanced,
            GrantSection::Technical => &mut prompt.disclosure.technical,
        };
        *open = !*open;
        self.present_grant_confirmation(cx);
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
                    prompt.submitted = false;
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
                    prompt.submitted = false;
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
        let draft = GrantChoices {
            coordinator: prompt.flow.account().to_owned(),
            managers: self
                .view
                .text_input(cx, ids!(grant_confirmation.scroll.people.editor.managers))
                .text(),
            tokens: self
                .view
                .widget(cx, ids!(grant_confirmation.scroll.choices.quota.tokens))
                .text(),
            expires_utc: self
                .view
                .text_input(cx, ids!(grant_confirmation.scroll.advanced.expires))
                .text(),
            self_approval: false,
        };
        let changed = draft.managers != prompt.draft.managers
            || draft.tokens != prompt.draft.tokens
            || draft.expires_utc != prompt.draft.expires_utc;
        prompt.draft = draft;
        // Keep every sentence, the list and the confirm button equal to what
        // the fields would commit. The fields themselves are left untouched.
        if changed {
            let view = grant_view(&prompt.state(), now_ms());
            self.apply_grant_view(cx, &view, None);
            self.request_confirmation_redraw(cx);
        }
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
        let view = grant_view(&prompt.state(), now_ms());
        let (can_primary, can_recover, can_cancel_intent) =
            (view.primary_enabled, view.recover_visible, view.cancel_intent_visible);
        let area = |this: &Self, cx: &mut Cx, path: &[LiveId]| this.view.button(cx, path).area();
        let cancel = area(self, cx, ids!(grant_confirmation.buttons.cancel));
        let primary = area(self, cx, ids!(grant_confirmation.buttons.primary));
        let recover = area(self, cx, ids!(grant_confirmation.buttons.recover));
        let cancel_intent = area(self, cx, ids!(grant_confirmation.buttons.cancel_intent));
        let edit = area(self, cx, ids!(grant_confirmation.scroll.people.edit));
        let advanced = area(self, cx, ids!(grant_confirmation.scroll.disclosures.advanced_toggle));
        let technical = area(self, cx, ids!(grant_confirmation.scroll.disclosures.technical_toggle));
        let presets = [
            (ids!(grant_confirmation.scroll.advanced.expiry_presets.day), Some(1)),
            (ids!(grant_confirmation.scroll.advanced.expiry_presets.week), Some(7)),
            (ids!(grant_confirmation.scroll.advanced.expiry_presets.month), None),
        ];
        let mut targets = Vec::new();
        if view.choosing {
            targets.push(edit);
            if view.people_editing {
                targets.push(self.view.text_input(cx, ids!(grant_confirmation.scroll.people.editor.managers)).area());
            }
            if view.quota_visible {
                if let Some(slider) = self
                    .view
                    .widget(cx, ids!(grant_confirmation.scroll.choices.quota.tokens.slider_wrap.amount_slider))
                    .borrow::<crate::miniapps::design::MiniAppAmountSlider>()
                {
                    targets.push(slider.focus_area());
                }
                targets.push(self.view.text_input(cx, ids!(grant_confirmation.scroll.choices.quota.tokens.input)).area());
            }
            targets.push(advanced);
            if view.advanced_open {
                for (id, _) in presets {
                    let button = self.view.button(cx, id);
                    if !button.disabled(cx) {
                        targets.push(button.area());
                    }
                }
                targets.push(self.view.text_input(cx, ids!(grant_confirmation.scroll.advanced.expires)).area());
            }
        }
        if !view.technical.is_empty() {
            targets.push(technical);
        }
        targets.push(cancel);
        if can_primary {
            targets.push(primary);
        }
        if can_recover {
            targets.push(recover);
        }
        if can_cancel_intent {
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
            if view.choosing {
                if view.advanced_open {
                    for (id, days) in presets {
                        if cx.has_key_focus(self.view.button(cx, id).area()) {
                            self.grant_expiry_preset(cx, days);
                            return true;
                        }
                    }
                }
                for (target, section) in [(edit, GrantSection::People), (advanced, GrantSection::Advanced)] {
                    if cx.has_key_focus(target) {
                        self.grant_disclose(cx, section);
                        cx.set_key_focus(target);
                        return true;
                    }
                }
            }
            if !view.technical.is_empty() && cx.has_key_focus(technical) {
                self.grant_disclose(cx, GrantSection::Technical);
                cx.set_key_focus(technical);
                return true;
            }
            if cx.has_key_focus(cancel) {
                self.close_grant_confirmation(cx);
                return true;
            }
            if can_primary && cx.has_key_focus(primary) {
                self.grant_primary(cx);
                return true;
            }
            if can_recover && cx.has_key_focus(recover) {
                self.grant_recover(cx);
                return true;
            }
            if can_cancel_intent && cx.has_key_focus(cancel_intent) {
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
/// An offline preview of the panel: phases built from fixture JSON and shown
/// through the production view model and layout. There is no flow, so the
/// preview can never reserve, publish or reply to a script.
#[cfg(any(test, feature = "onboarding-test-support"))]
pub(super) struct GrantPreview {
    phase: Phase,
    draft: GrantChoices,
    proposal: Option<u64>,
    form: Option<OwnerGrantForm>,
    successor: Option<OwnerGrantSuccessionForm>,
    requested: Option<Succession>,
    applicants: Vec<String>,
    account: String,
    original: bool,
    in_flight: bool,
    submitted: bool,
    submitting: bool,
    can_choose_new_limits: bool,
    disclosure: Disclosure,
}
#[cfg(any(test, feature = "onboarding-test-support"))]
impl GrantPreview {
    fn state(&self) -> GrantState<'_> {
        GrantState {
            phase: &self.phase,
            draft: &self.draft,
            proposal: self.proposal,
            form: self.form.as_ref(),
            successor: self.successor.as_ref(),
            requested: self.requested,
            applicants: &self.applicants,
            account: &self.account,
            original: self.original,
            in_flight: self.in_flight,
            submitted: self.submitted,
            submitting: self.submitting,
            publishing: false,
            cancelling: false,
            regrant: false,
            active: true,
            can_choose_new_limits: self.can_choose_new_limits,
            succession_supported: Some(true),
            disclosure: self.disclosure,
        }
    }
    /// `state`: publish, amend, add_people, renew, published, record, loading
    /// or failed. Amounts are tokens; people are full Matrix IDs.
    pub(super) fn from_fixture(f: &Value, now: u64) -> Result<Self, String> {
        use chrono::Datelike;
        let text = |key: &str, default: &str| f[key].as_str().unwrap_or(default).to_owned();
        let list = |key: &str| -> Vec<String> {
            f[key].as_array().map(|items| items.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect()).unwrap_or_default()
        };
        let account = text("account", "@reviewer:example.test");
        let pool = text("pool", "pool_original");
        let resource = text("resource", "resource_original");
        let current = f["current"].as_u64().unwrap_or(200_000);
        let managers = list("managers");
        let added = list("added");
        let state = text("state", "publish");
        let kind = match (state.as_str(), f["kind"].as_str()) {
            ("amend" | "add_people", _) | (_, Some("amendment" | "add_people")) => Some(OwnerGrantSuccessionKind::Amendment),
            ("renew", _) | (_, Some("renewal")) => Some(OwnerGrantSuccessionKind::Renewal),
            _ => None,
        };
        let month_start = |ms: u64| -> u64 {
            let at = chrono::DateTime::from_timestamp_millis(ms as i64).unwrap_or_default().date_naive();
            at.with_day(1).and_then(|d| d.and_hms_opt(0, 0, 0)).map_or(0, |d| d.and_utc().timestamp_millis() as u64)
        };
        let start = month_start(now);
        let end = month_start(start + 32 * 86_400_000);
        let next_end = month_start(end + 32 * 86_400_000);
        let key = |ms: u64| chrono::DateTime::from_timestamp_millis(ms as i64).unwrap_or_default().format("%Y-%m").to_string();
        let context = ::hagency::native_owner::OwnerGrantReviewContext {
            server_origin: text("server", "https://matrix.example.test"),
            profile_id: text("profile", "profile_owner"),
            resource_revision: "c".repeat(64),
            framework: "codex".into(),
            model: text("model", "coding-model"),
            remaining_tokens: f["remaining"].as_u64(),
            resource_total_tokens: Some(f["total"].as_u64().unwrap_or(1_000_000)),
            shared_account_total_tokens: f["accountLimit"].as_u64(),
            observed_at_ms: now,
        };
        let summary = |tokens: u64, people: Vec<String>, period: (String, u64, u64), succession: Option<::hagency::native_owner::OwnerGrantSuccessionSummary>| OwnerGrantSummary {
            intent_id: "intent_preview".into(),
            grant_id: if succession.is_some() { "grant_preview_2".into() } else { "grant_preview_1".into() },
            owner_mxid: account.clone(),
            contribution_id: pool.clone(),
            resource_id: resource.clone(),
            coordinator_mxid: account.clone(),
            allocated_tokens: tokens,
            eligible_managers: people,
            allow_self_approval: false,
            delegation_expires_at_ms: period.2,
            period_key: period.0,
            period_start_ms: period.1,
            period_end_ms: period.2,
            succession,
        };
        let this_period = (key(start), start, end);
        let successor = kind.map(|kind| {
            let renewing = kind == OwnerGrantSuccessionKind::Renewal;
            OwnerGrantSuccessionForm {
                kind,
                contribution_id: pool.clone(),
                resource_id: resource.clone(),
                label: text("label", "Research"),
                context: context.clone(),
                predecessor: summary(current, managers.clone(), this_period.clone(), None),
                floor: ::hagency::native_owner::OwnerGrantFloor {
                    children: f["running"].as_u64().unwrap_or(0),
                    retained_tokens: f["held"].as_u64().unwrap_or(0),
                },
                period_key: if renewing { key(end) } else { key(start) },
                period_start_ms: if renewing { end } else { start },
                period_end_ms: if renewing { next_end } else { end },
                window_opens_at_ms: end.saturating_sub(7 * 86_400_000),
                window_closes_at_ms: end,
            }
        });
        let form = kind.is_none().then(|| OwnerGrantForm {
            contribution_id: pool.clone(),
            resource_id: resource.clone(),
            label: text("label", "Research"),
            context: context.clone(),
        });
        // As the mini app asks: a new limit, or the current one with people or
        // a renewal. Submitting in the window clears it, as `submit` does.
        let proposal = match state.as_str() {
            "amend" | "publish" | "loading" => f["proposed"].as_u64(),
            "add_people" | "renew" => Some(current),
            _ => None,
        };
        // A refused change keeps what the owner typed in the fields.
        let typed = if state == "failed" { f["proposed"].as_u64() } else { proposal };
        let mut draft = initial_draft(typed, if kind.is_some() { &added } else { &managers });
        match (&successor, &form) {
            (Some(successor), _) => fill_successor_defaults(&mut draft, successor, now),
            (None, Some(form)) => fill_form_defaults(&mut draft, form),
            _ => {}
        }
        draft.coordinator = account.clone();
        let requested = kind.map(succession_arg);
        let published_tokens = f["proposed"].as_u64().unwrap_or(current);
        let mut people = managers.clone();
        people.extend(added.iter().cloned());
        let people = managers_of(&people.join(" "));
        let succession_summary = |kind| ::hagency::native_owner::OwnerGrantSuccessionSummary {
            kind,
            predecessor_grant_id: "grant_preview_1".into(),
            predecessor_allocated_tokens: current,
            carried_children: f["running"].as_u64().unwrap_or(0),
            carried_retained_tokens: f["held"].as_u64().unwrap_or(0),
        };
        let progress = |summary| {
            Phase::PublicationProgress(OwnerGrantPublicationProgress {
                summary,
                state: OwnerGrantPublicationState::ServerObserved,
                server_state: Some("active".into()),
                server_revision: Some(2),
                observed_at_ms: Some(now),
            })
        };
        let phase = match state.as_str() {
            "publish" | "amend" | "add_people" | "renew" => Phase::Choose,
            "published" => progress(match kind {
                Some(OwnerGrantSuccessionKind::Renewal) => summary(published_tokens, people.clone(), (key(end), end, next_end), Some(succession_summary(OwnerGrantSuccessionKind::Renewal))),
                Some(kind) => summary(published_tokens, people.clone(), this_period.clone(), Some(succession_summary(kind))),
                None => summary(published_tokens, people.clone(), this_period.clone(), None),
            }),
            "record" => progress(summary(current, managers.clone(), this_period.clone(), None)),
            "loading" => Phase::Loading,
            "failed" => Phase::Failed(text("code", "owner_grant_below_floor")),
            other => return Err(format!("unknown preview state {other}")),
        };
        // A record opened by itself, or a window still reading, has no form yet.
        let record = state == "record";
        let formless = record || (state == "loading" && !f["submitting"].as_bool().unwrap_or(false));
        Ok(Self {
            phase,
            draft,
            proposal,
            form: if formless { None } else { form },
            successor: if formless { None } else { successor },
            requested: if record { None } else { requested },
            applicants: if state == "add_people" || f["kind"].as_str() == Some("add_people") { added.clone() } else { Vec::new() },
            account,
            original: matches!(state.as_str(), "published" | "record"),
            in_flight: state == "loading",
            submitted: state == "published",
            submitting: f["submitting"].as_bool().unwrap_or(false),
            can_choose_new_limits: !record,
            disclosure: Disclosure {
                people: f["people"].as_bool().unwrap_or(false),
                advanced: f["advanced"].as_bool().unwrap_or(false),
                technical: f["technical"].as_bool().unwrap_or(false),
            },
        })
    }
}
#[cfg(any(test, feature = "onboarding-test-support"))]
impl MiniAppsPanel {
    /// Show one preview phase in the real panel. Nothing here can be confirmed.
    pub(super) fn preview_grant(&mut self, cx: &mut Cx, fixture: &Value) -> Result<(), String> {
        let now = now_ms();
        let preview = GrantPreview::from_fixture(fixture, now)?;
        let state = preview.state();
        let view = grant_view(&state, now);
        let inputs = GrantInputs::of(&state, now);
        self.view.view(cx, ids!(grant_confirmation)).set_visible(cx, true);
        self.apply_grant_view(cx, &view, Some(&inputs));
        Ok(())
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
        let mut draft = initial_draft(Some(3_000_000), &[]);
        assert_eq!(draft.tokens, "3000000");
        fill_form_defaults(&mut draft, &form);
        assert_eq!(draft.tokens, "3000000");
        // Applicants are still typed natively; expiry keeps today's default.
        assert!(draft.managers.is_empty());
        assert_eq!(draft.expires_utc, publication_expiry(None));
        let mut default = initial_draft(None, &[]);
        fill_form_defaults(&mut default, &form);
        assert_eq!(default.tokens, "1000000");
        // Above the available capacity the proposal stays visible and unconfirmable.
        let mut above = initial_draft(Some(12_000_000), &[]);
        fill_form_defaults(&mut above, &form);
        above.coordinator = "@owner:test".into();
        above.managers = "@manager:test".into();
        assert_eq!(above.tokens, "12000000");
        assert!(!form_choices_valid(&form, &above));
        above.tokens = "3000000".into();
        assert!(form_choices_valid(&form, &above));
        // A reopened form keeps what the owner typed over the proposal.
        let mut edited = initial_draft(Some(3_000_000), &[]);
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
                Some(format!("Per-request limit: Not in effect → {shown}").as_str())
            );
        }
        assert!(proposal_notice(&Phase::Choose, None, "3000000", false, OCTOBER).is_none());
        assert!(proposal_notice(&Phase::Loading, Some(3_000_000), "", false, OCTOBER).is_none());
        let declined = proposal_notice(&progress(OwnerGrantState::Declined, false), Some(3_000_000), "", false, OCTOBER).unwrap();
        assert_eq!(declined, "Per-request limit: Not in effect → 3 M\nThis amount is filled in when you choose again.");
        let closed = proposal_notice(&progress(OwnerGrantState::Recorded, true), Some(3_000_000), "", false, OCTOBER).unwrap();
        assert!(closed.ends_with("filled in when you choose again."), "{closed}");
        let failed = proposal_notice(&Phase::Failed("owner_grant_prior_intent_required".into()), Some(3_000_000), "", false, OCTOBER).unwrap();
        assert_eq!(failed, "This amount wasn’t applied.");
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
                format!("Per-request limit: Not in effect → 3 M\n{PUBLISHED_NOTE}")
            );
        }
        // No proposal, applicant or selector flag offers a commit beside a
        // published grant: nothing is revoked, reserved or published from here.
        let form = pool_form(10_000_000);
        let mut draft = initial_draft(Some(3_000_000), &[]);
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
            "This amount is filled in when you set this month’s capacity.";
        let form = pool_form(10_000_000);
        let draft = initial_draft(Some(3_000_000), &[]);
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
                format!("Per-request limit: Not in effect → 3 M\n{ENDED_NOTE}")
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
        const EXISTING: &str = "Per-request limit: Not in effect → 3 M\nThis amount doesn’t change the existing publish; it keeps its own amounts.";
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
                format!("Per-request limit: Not in effect → 3 M\n{PENDING_NOTE}")
            );
        }
        // A proposal adds or removes no action on an existing original.
        let draft = initial_draft(Some(3_000_000), &[]);
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
        let mut draft = initial_draft(Some(800_000), &["@new:test".to_owned()]);
        assert_eq!(draft.managers, "@new:test");
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
        let mut plain = initial_draft(None, &[]);
        fill_successor_defaults(&mut plain, &form, now_ms());
        assert_eq!(plain.tokens, "2000000");
        assert_eq!(plain.managers, "@manager:test");
        // One sentence names the change and who it adds; the field says the
        // floor running Agents hold and what lowering means for waiting requests.
        let applicants = ["@new:test".to_owned()];
        let view = preview_view(&Phase::Choose, &draft, Some(&form), None, &applicants, Disclosure::default());
        assert_eq!(view.title, "Change Per-Request Limit");
        assert_eq!(view.lead, "Change the per-request limit from 2 M to 0.8 M, and add @new to this month’s list");
        assert_eq!(view.primary, "Confirm Changes");
        assert!(view.primary_enabled);
        assert_eq!(view.people, [("@manager".to_owned(), false), ("@new".to_owned(), true)]);
        assert_eq!(
            view.quota_hint,
            "Up to 5 M. Not below the 0.6 M running Agents already hold. Waiting requests above 0.8 M can’t be approved."
        );
        assert_eq!(view.status, "Takes effect once you confirm; running Agents keep what they hold.");
        // Unknown capacity keeps the form unconfirmable.
        let mut unknown = form.clone();
        unknown.context.remaining_tokens = None;
        assert!(!choices_valid(&successor_bounds(&unknown), &draft));
    }
    #[test]
    fn a_renewal_starts_next_month_and_its_delegation_reaches_into_it() {
        let form = successor_form(OwnerGrantSuccessionKind::Renewal);
        let bounds = successor_bounds(&form);
        let mut draft = initial_draft(None, &[]);
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
        // The window names next month, keeps the list and says when it starts.
        let view = preview_view(&Phase::Choose, &draft, Some(&form), None, &[], Disclosure::default());
        let month = key_month(&form.period_key).unwrap();
        assert_eq!(view.title, format!("Publish {month} Ahead"));
        assert_eq!(view.lead, format!("Publish {month} ahead: up to 2 M per request; the list and monthly maximum carry over"));
        assert_eq!(view.status, format!("Starts {}; running Agents carry on without requesting again.", month_day(form.period_start_ms)));
        assert_eq!(view.primary, "Publish Ahead");
        let mut raised = draft.clone();
        raised.tokens = "3000000".into();
        let view = preview_view(&Phase::Choose, &raised, Some(&form), None, &[], Disclosure::default());
        assert_eq!(view.lead, format!("Publish {month} ahead: per-request limit from 2 M to 3 M; the list and monthly maximum carry over"));
    }
    #[test]
    fn a_proposal_beside_an_active_grant_opens_the_amendment_when_the_server_supports_it() {
        let live = publication(OwnerGrantPublicationState::ServerObserved, Some("active"));
        assert_eq!(
            proposal_notice(&live, Some(3_000_000), "", true, OCTOBER).unwrap(),
            format!("Per-request limit: 2 M → 3 M\n{AMENDABLE_NOTE}")
        );
        let draft = initial_draft(Some(3_000_000), &[]);
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
                format!("Per-request limit: Not in effect → 3 M\n{NOT_AMENDABLE_NOTE}")
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
            format!("Per-request limit: Not in effect → 3 M\n{SCHEDULED_NOTE}")
        );
        let draft = initial_draft(Some(3_000_000), &[]);
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
    /// The view of one native state, with an active flow that supports
    /// successors, as the shipped panel computes it.
    fn preview_view(
        phase: &Phase,
        draft: &GrantChoices,
        successor: Option<&OwnerGrantSuccessionForm>,
        form: Option<&OwnerGrantForm>,
        applicants: &[String],
        disclosure: Disclosure,
    ) -> GrantView {
        let state = GrantState {
            phase,
            draft,
            proposal: None,
            form,
            successor,
            requested: successor.map(|form| succession_arg(form.kind)),
            applicants,
            account: "@owner:test",
            original: false,
            in_flight: false,
            submitted: false,
            submitting: false,
            publishing: false,
            cancelling: false,
            regrant: false,
            active: true,
            can_choose_new_limits: true,
            succession_supported: Some(true),
            disclosure,
        };
        grant_view(&state, now_ms())
    }
    #[test]
    fn adding_people_names_only_them_and_keeps_the_unchanged_limit_out_of_the_way() {
        let form = successor_form(OwnerGrantSuccessionKind::Amendment);
        // As the mini app sends it: the current limit and the people to add.
        let applicants = ["@lisi:test".to_owned()];
        let mut draft = initial_draft(Some(2_000_000), &applicants);
        fill_successor_defaults(&mut draft, &form, now_ms());
        draft.coordinator = "@owner:test".into();
        let view = preview_view(&Phase::Choose, &draft, Some(&form), None, &applicants, Disclosure::default());
        assert_eq!(view.title, "Add People Who Can Request");
        assert_eq!(view.lead, "Add @lisi to this month’s list");
        assert_eq!(view.status, "Once you confirm, they can request right away.");
        assert_eq!(view.people_note, "You can add people this month but not remove them.");
        assert_eq!(view.primary, "Add to This Month’s List");
        assert!(view.primary_enabled);
        // The unchanged limit stays under Advanced Options; nothing says 2 M → 2 M.
        assert!(!view.quota_visible && !view.lead.contains("→"));
        let open = preview_view(&Phase::Choose, &draft, Some(&form), None, &applicants, Disclosure { advanced: true, ..Disclosure::default() });
        assert!(open.quota_visible && open.advanced_open);
        // Someone already on the list: nothing to confirm.
        let already = ["@manager:test".to_owned()];
        let mut same = initial_draft(Some(2_000_000), &already);
        fill_successor_defaults(&mut same, &form, now_ms());
        same.coordinator = "@owner:test".into();
        let view = preview_view(&Phase::Choose, &same, Some(&form), None, &already, Disclosure::default());
        assert_eq!(view.lead, "@manager are already on this month’s list");
        assert!(!view.primary_enabled);
        // Removing someone already on the list is refused beside the list.
        let mut removed = draft.clone();
        removed.managers = "@lisi:test".into();
        let view = preview_view(&Phase::Choose, &removed, Some(&form), None, &applicants, Disclosure::default());
        assert_eq!(view.status, "This month’s list can only grow: keep everyone already on it.");
        assert!(!view.primary_enabled);
    }
    #[test]
    fn an_unchanged_amendment_cannot_be_confirmed() {
        let form = successor_form(OwnerGrantSuccessionKind::Amendment);
        let mut draft = initial_draft(None, &[]);
        fill_successor_defaults(&mut draft, &form, now_ms());
        draft.coordinator = "@owner:test".into();
        assert!(choices_valid(&successor_bounds(&form), &draft));
        let view = preview_view(&Phase::Choose, &draft, Some(&form), None, &[], Disclosure::default());
        assert_eq!(view.lead, "The per-request limit is 2 M. Choose a new amount or add people.");
        assert!(!view.primary_enabled && view.primary_visible);
    }
    #[test]
    fn publishing_names_the_month_and_keeps_ids_and_utc_in_technical_details() {
        let form = pool_form(5_000_000);
        let mut draft = initial_draft(Some(500_000), &["@applicant:test".to_owned()]);
        fill_form_defaults(&mut draft, &form);
        draft.coordinator = "@owner:test".into();
        let view = preview_view(&Phase::Choose, &draft, None, Some(&form), &[], Disclosure::default());
        let month = ms_month(now_ms());
        assert_eq!(view.title, format!("Publish {month}"));
        assert_eq!(view.lead, format!("Publish {month}: up to 0.5 M per request"));
        assert_eq!(view.status, "Once published, people on the list can request it until the end of the month.");
        assert_eq!(view.primary, "Publish");
        assert_eq!(view.people, [("@applicant".to_owned(), false)]);
        assert_eq!(view.expiry, format!("Sharing lasts until the end of {month}"));
        for primary in [&view.title, &view.pool, &view.lead, &view.status, &view.quota_hint, &view.expiry] {
            assert!(!primary.contains("UTC") && !primary.contains("matrix.test") && !primary.contains("pool_1"), "{primary}");
        }
        assert!(view.technical.contains("https://matrix.test") && view.technical.contains("pool_1"));
        assert!(!view.technical_open);
        // Nobody chosen yet: the editor opens and says what is missing.
        let mut empty = draft.clone();
        empty.managers.clear();
        let view = preview_view(&Phase::Choose, &empty, None, Some(&form), &[], Disclosure::default());
        assert!(view.people_editing && !view.primary_enabled);
        assert_eq!(view.status, "Add at least one person who can request.");
    }
    #[test]
    fn a_finished_publish_says_what_now_applies_and_offers_done() {
        let live = publication(OwnerGrantPublicationState::ServerObserved, Some("active"));
        let draft = initial_draft(None, &[]);
        let form = pool_form(5_000_000);
        let state = GrantState {
            phase: &live,
            draft: &draft,
            proposal: None,
            form: Some(&form),
            successor: None,
            requested: None,
            applicants: &[],
            account: "@owner:test",
            original: true,
            in_flight: false,
            submitted: true,
            submitting: false,
            publishing: true,
            cancelling: false,
            regrant: false,
            active: true,
            can_choose_new_limits: true,
            succession_supported: Some(true),
            disclosure: Disclosure::default(),
        };
        let view = grant_view(&state, OCTOBER);
        assert_eq!(view.lead, "Published. Up to 2 M per request.");
        assert_eq!(view.status, "You can close this window.");
        assert_eq!(view.cancel, "Done");
        assert_eq!(view.people, [("@manager".to_owned(), false)]);
    }
    #[test]
    fn every_preview_state_renders_through_the_shipped_view() {
        let now = now_ms();
        for state in ["publish", "amend", "add_people", "renew", "published", "record", "loading", "failed"] {
            let fixture = json!({
                "panel": "grant", "state": state, "label": "Research", "model": "coding-model",
                "account": "@reviewer:example.test", "current": 200000, "proposed": 250000,
                "remaining": 800000, "held": 100000, "running": 1,
                "managers": ["@applicant:example.test", "@designer:example.test"],
                "added": ["@lisi:example.test"], "kind": if state == "failed" {"amendment"} else {""},
            });
            let preview = GrantPreview::from_fixture(&fixture, now).unwrap();
            let view = grant_view(&preview.state(), now);
            assert!(!view.title.is_empty(), "{state}");
            for primary in [&view.title, &view.lead, &view.status] {
                assert!(!primary.contains(":example.test") && !primary.contains("UTC"), "{state}: {primary}");
            }
        }
    }
    /// String literals in Rust source, unescaped as the compiler would.
    fn literals(source: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut chars = source.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '"' {
                continue;
            }
            let mut text = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => match chars.next() {
                        Some('n') => text.push('\n'),
                        Some(other) => text.push(other),
                        None => break,
                    },
                    c => text.push(c),
                }
            }
            out.push(text);
        }
        out
    }
    #[test]
    fn native_pool_windows_use_the_mini_apps_words_in_chinese() {
        use crate::i18n::{Language, text_for};
        const BANNED: [&str; 13] = [
            "每月总上限", "本设备", "智能体", "令牌", "授权意向", "准入", "协调者", "发布额度",
            "M tokens", "申请者", "已贡献", "结算周期", "资源池发布",
        ];
        let cut = |source: &'static str, from: &str, to: &str| -> &'static str {
            let start = source.find(from).unwrap_or(0);
            let end = source[start..].find(to).map_or(source.len(), |n| start + n);
            &source[start..end]
        };
        let sources = [
            cut(include_str!("grant_ui.rs"), "//!", "#[cfg(test)]\nmod tests {"),
            cut(include_str!("resource_edit_ui.rs"), "//!", "#[cfg(test)]"),
            cut(include_str!("../host/hagency/resource_edit.rs"), "//!", "#[cfg(test)]"),
            cut(include_str!("ui.rs"), "grant_confirmation := View", "project_room_picker := View"),
            cut(include_str!("resource_enrollment_ui.rs"), "Self::Create(v) if v.recovering()", "pub(super) async fn accept"),
        ];
        let mut checked = 0;
        for source in sources {
            for english in literals(source) {
                let chinese = text_for(Language::Chinese, &english);
                if chinese == english {
                    continue;
                }
                checked += 1;
                for word in BANNED {
                    assert!(!chinese.contains(word), "{english:?} is shown as {chinese:?}, which says {word}");
                }
            }
        }
        assert!(checked > 150, "only {checked} translated strings were checked");
        // The window titles use the tab's own words.
        for (english, chinese) in [
            ("Publish {month}", "发布 {month}额度"),
            ("Change Per-Request Limit", "修改每次申请上限"),
            ("Add People Who Can Request", "添加可以申请的人"),
            ("Publish {month} Ahead", "提前发布 {month}额度"),
            ("Per-request limit (M)", "每次申请上限（M）"),
            ("Change the monthly maximum", "修改每月上限"),
        ] {
            assert_eq!(text_for(Language::Chinese, english), chinese);
        }
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
            // The list, the advanced choices and every toggle are open at once.
            let open: [&[LiveId]; 5] = [ids!(grant_confirmation.scroll.people),ids!(grant_confirmation.scroll.people.list.p0),ids!(grant_confirmation.scroll.people.list.p1),ids!(grant_confirmation.scroll.people.editor),ids!(grant_confirmation.scroll.advanced)];
            for path in open {
                panel.view.widget(&cx,path).set_visible(&mut cx,true);
            }
            let texts: [(&[LiveId], &str); 3] = [(ids!(grant_confirmation.scroll.people.list.p0.name),"@applicant"),(ids!(grant_confirmation.scroll.people.list.p1.name),"@lisi"),(ids!(grant_confirmation.scroll.lead),if cn {"把每次申请上限从 0.2 M 改为 0.25 M，并把 @lisi 加入本月名单"} else {"Change the per-request limit from 0.2 M to 0.25 M, and add @lisi to this month’s list"})];
            for (path,text) in texts {
                panel.view.label(&cx,path).set_text(&mut cx,text);
            }
            let long = if cn {
                "可以申请的人\n@lisi:example.org\n技术详情\n"
            } else {
                "People who can request\n@applicant:example.org\nTechnical details\n"
            }
            .repeat(64);
            panel
                .view
                .label(&cx, ids!(grant_confirmation.scroll.details.copy))
                .set_text(&mut cx, &long);
            // The proposed-limit line sits above the form and the details.
            panel.view.widget(&cx, ids!(grant_confirmation.scroll.change)).set_visible(&mut cx, true);
            panel.view.label(&cx, ids!(grant_confirmation.scroll.change)).set_text(&mut cx, if cn {
                "每次申请上限：2 M → 3 M\n这个资源池本月已经发布过，服务器每月只接受一次发布，所以这个数没有生效。没有撤回或改动任何东西。"
            } else {
                "Per-request limit: 2 M → 3 M\nThis pool already has a published grant for this month on its current admission, and the server accepts one grant per admission each month, so the proposed limit is not applied. Nothing was revoked or changed."
            });
            if cn {
                for (id, text) in [
                    (ids!(grant_confirmation.buttons.cancel), "取消"),
                    (ids!(grant_confirmation.buttons.primary), "发布"),
                    (ids!(grant_confirmation.buttons.recover), "再查一次"),
                    (
                        ids!(grant_confirmation.buttons.cancel_intent),
                        "放弃这次发布…",
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
                            "提前发布"
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
                            "放弃这次发布"
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
                let slider=panel.view.widget(&cx,ids!(grant_confirmation.scroll.choices.quota.tokens.slider_wrap.amount_slider)).area().rect(&cx);
                let input=panel.view.widget(&cx,ids!(grant_confirmation.scroll.choices.quota.tokens.input)).area().rect(&cx);
                assert!(slider.size.x>=100.0 && slider.size.y>=44.0,"{slider:?}");
                assert!(input.size.x<=111.0 && input.pos.x+input.size.x<=width+0.5,"{input:?}");
                for path in [ids!(grant_confirmation.scroll.choices.quota.tokens.decrease),ids!(grant_confirmation.scroll.choices.quota.tokens.increase)] {
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
