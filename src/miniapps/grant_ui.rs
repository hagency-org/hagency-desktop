//! Trusted native owner grant screen. The package cannot render or acknowledge
//! this confirmation; its reply contains only public progress and original IDs.
use super::*;
use crate::host::hagency::grants::{GrantChoices, GrantFlow, GrantResult};
use ::hagency::native_owner::{
    OwnerGrantProgress, OwnerGrantReview, OwnerGrantState, OwnerGrantPublicationReview,
    OwnerGrantPublicationProgress, OwnerGrantPublicationState, OwnerGrantCancellationReview,
    OwnerGrantSummary, OwnerGrantForm,
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
fn form_choices_valid(form: &OwnerGrantForm, draft: &GrantChoices) -> bool {
    draft.clone().parse().is_ok_and(|choices| {
        form.context.resource_total_tokens.is_some()
            && form.context.remaining_tokens.is_some_and(|n| n>=choices.allocated_tokens)
            && choices.delegation_expires_at_ms<=publication_period_end().timestamp_millis() as u64
    })
}
/// A script proposal only seeds the native tokens field; the owner can edit it
/// and the committed value is always read back from that widget.
fn initial_draft(proposal: Option<u64>) -> GrantChoices {
    GrantChoices {
        tokens: proposal.map(|tokens| tokens.to_string()).unwrap_or_default(),
        ..GrantChoices::default()
    }
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
/// Whether a phase offers the native primary action. A proposal never changes
/// this; in particular a published grant is only shown, never replaced here.
fn primary_available(
    phase: &Phase,
    form: Option<&OwnerGrantForm>,
    draft: &GrantChoices,
    has_original: bool,
    can_choose_new_limits: bool,
) -> bool {
    match phase {
        Phase::Choose => form.is_some_and(|f| form_choices_valid(f, draft)),
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
/// The server has accepted a grant for this pool's current admission. It
/// accepts only one per admission, even after revocation or expiry, so a
/// proposal is never applied here and nothing is revoked to make room.
fn published_grant(phase: &Phase) -> bool {
    matches!(phase, Phase::PublicationProgress(p) if matches!(p.state, OwnerGrantPublicationState::ServerObserved))
}
/// The pool's current per-request limit: the tokens of a grant the server last
/// reported as active. Reserved, rejected, revoked or expired grants set none.
fn live_grant_tokens(phase: &Phase) -> Option<u64> {
    match phase {
        Phase::PublicationProgress(p)
            if published_grant(phase) && p.server_state.as_deref() == Some("active") =>
        {
            Some(p.summary.allocated_tokens)
        }
        _ => None,
    }
}
/// What the mini app learns when the panel closes. A proposal still pending
/// beside this admission's published grant was never applied: say why.
fn close_reply(
    phase: &Phase,
    proposal: Option<u64>,
    in_flight: bool,
    original: Option<&str>,
) -> Result<String, String> {
    if proposal.is_some() && published_grant(phase) {
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
/// Native text for a script's proposed per-request limit: the current limit,
/// the amount this screen would commit, and why a phase does not use it.
fn proposal_notice(phase: &Phase, proposal: Option<u64>, draft_tokens: &str) -> Option<String> {
    let proposal = proposal?;
    let (new, note) = match phase {
        Phase::Loading => return None,
        // The form commits only what its tokens field holds now.
        Phase::Choose => (draft_tokens.trim().parse::<u64>().ok(), None),
        phase if published_grant(phase) => (
            Some(proposal),
            Some("This pool already has a published grant for its current admission, and the server accepts only one grant per admission, so the proposed limit is not applied. Nothing was revoked or changed."),
        ),
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
    let current = live_grant_tokens(phase)
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
/// Only exact reviewed native codes may enter diagnostics. A prefix match would
/// allow arbitrary provider errors, paths or credentials to escape into logs.
fn grant_failure_code(code: &str) -> &'static str {
    match code {
        "management_access_required" => "management_access_required",
        "management_admission_receipts_required" => "management_admission_receipts_required",
        "management_features_invalid" => "management_features_invalid",
        "owner_grant_prior_intent_required" => "owner_grant_prior_intent_required",
        "owner_grant_new_admission_required" => "owner_grant_new_admission_required",
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
    fn can_primary(&self) -> bool {
        self.actionable()
            && primary_available(
                &self.phase,
                self.form.as_ref(),
                &self.draft,
                self.original.is_some(),
                self.flow.can_choose_new_limits(),
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
        self.view.view(cx, ids!(app_content)).set_visible(cx, false);
        self.view.view(cx, ids!(approval)).set_visible(cx, false);
        self.view
            .label(cx, ids!(grant_confirmation.scroll.owner))
            .set_text(cx, prompt.flow.account());
        let choosing = matches!(prompt.phase, Phase::Choose);
        if choosing {
            for (id,days) in [(ids!(grant_confirmation.scroll.choices.expiry_presets.day),Some(1)),(ids!(grant_confirmation.scroll.choices.expiry_presets.week),Some(7)),(ids!(grant_confirmation.scroll.choices.expiry_presets.month),None)] {
                self.view.button(cx,id).set_disabled(cx,!publication_preset_available(days));
            }
            if let Some(form)=&prompt.form {
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
        match &prompt.phase {
            Phase::Loading=>status=tr(if prompt.submit_publication {"Publishing resource pool…"} else {"Loading publication status…"}).into(),
            Phase::Choose=>status=tr(if prompt.form.as_ref().is_some_and(|f| f.context.remaining_tokens.is_some_and(|n|n>0)) {
                "Set who can apply, the token budget and expiry. Publish once to make this capacity available on the server."
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
            },
            Phase::Progress(p)=>{
                if matches!(p.state,OwnerGrantState::Recorded) { primary="Publish resource pool"; }
                if matches!(p.state,OwnerGrantState::Declined) { primary="Choose new limits"; }
                details=cancellation_details("",p.summary.clone());
                status=tr(match p.state {OwnerGrantState::Recorded=>"Capacity is reserved. Publish these saved limits to make the pool available on the server.",
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
                details.push_str(&format!("\n\n{}\n{}",tr("Expires (UTC)"),utc(p.summary.delegation_expires_at_ms.min(p.summary.period_end_ms))));
                status=tr(match p.state {
                    OwnerGrantPublicationState::OutcomeUnknown=>"Publication is not confirmed. Recover the original command. This action does not change the local reservation.",
                    OwnerGrantPublicationState::Rejected=>"This original publication was rejected. The operation did not change the local reservation or create a replacement.",
                    OwnerGrantPublicationState::ServerObserved=>"The server state below is the latest observation. Recovery does not renew or reactivate this grant.",
                }).into();
                if let Some(server)=&p.server_state {
                    details.push_str(&format!("\n\n{}\n{}\n\n{}\n{}",tr("Server grant state"),tr(match server.as_str(){"active"=>"Active","revoked"=>"Revoked","expired"=>"Expired",_=>"Unavailable"}),tr("Observed at"),p.observed_at_ms.map(utc).unwrap_or_default()));
                }
            },
            Phase::Failed(code)=>status=tr(failure_message(code)).into(),
        }
        if !prompt.flow.active() {
            status =
                tr("The account or session changed. Open a fresh review after signing in.").into();
        }
        self.view
            .label(cx, ids!(grant_confirmation.scroll.status))
            .set_text(cx, &status);
        let notice = proposal_notice(&prompt.phase, prompt.proposal, &prompt.draft.tokens);
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
                    fill_form_defaults(&mut prompt.draft, &form);
                    prompt.draft.coordinator=prompt.flow.account().into();
                    prompt.form=Some(form);
                    Phase::Choose
                }
                Ok(GrantResult::Review(review)) => {
                    prompt.original = Some(review.summary().intent_id);
                    prompt.cancelling = false;
                    if prompt.submit_publication && prompt.form.as_ref().is_some_and(|f| !same_publication_resource(f,&review)) {
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
        if !matches!(prompt.phase,Phase::Choose) || !publication_preset_available(days){return}
        prompt.draft.expires_utc=publication_expiry(days);
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
        let notice = proposal_notice(&prompt.phase, prompt.proposal, &prompt.draft.tokens);
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
        match std::mem::replace(&mut prompt.phase, Phase::Loading) {
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
                prompt.submit();
                self.grant_job(async move { flow.prepare(choices).await });
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
        self.grant_job(async move {
            match id {
                Some(id) if publishing => flow.publication_review(id).await,
                Some(id) => flow.recover(id).await,
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
        let reply=close_reply(&prompt.phase,prompt.proposal,prompt.receiver.is_some(),prompt.original.as_deref());
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
            intent_id: "original".into(), owner_mxid: "@owner:test".into(),
            contribution_id: "pool_1".into(), resource_id: "resource".into(),
            coordinator_mxid: "@owner:test".into(), allocated_tokens: tokens,
            eligible_managers: vec!["@manager:test".into()], allow_self_approval: false,
            delegation_expires_at_ms: 1793491200000, period_key: "2026-10".into(),
            period_start_ms: 1790812800000, period_end_ms: 1793491200000,
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
            summary: summary(2_000_000), state,
            closure: closed.then_some(::hagency::native_owner::OwnerGrantClosure {
                retained_tokens: 0, known_consumed_tokens: 0, unresolved_tokens: 0, observed_at_ms: 1,
            }),
        })
    }
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
                proposal_notice(&Phase::Choose, Some(3_000_000), field).as_deref(),
                Some(format!("Per-request limit: None active → {shown}").as_str())
            );
        }
        assert!(proposal_notice(&Phase::Choose, None, "3000000").is_none());
        assert!(proposal_notice(&Phase::Loading, Some(3_000_000), "").is_none());
        let declined = proposal_notice(&progress(OwnerGrantState::Declined, false), Some(3_000_000), "").unwrap();
        assert_eq!(declined, "Per-request limit: None active → 3 M\nThe proposed limit is filled in when you choose new limits.");
        let closed = proposal_notice(&progress(OwnerGrantState::Recorded, true), Some(3_000_000), "").unwrap();
        assert!(closed.ends_with("filled in when you choose new limits."), "{closed}");
        let failed = proposal_notice(&Phase::Failed("owner_grant_prior_intent_required".into()), Some(3_000_000), "").unwrap();
        assert!(failed.ends_with("\nThe proposed limit was not applied."), "{failed}");
    }
    #[test]
    fn published_grant_is_shown_read_only_and_never_replaced() {
        const PUBLISHED: &str = "This pool already has a published grant for its current admission, and the server accepts only one grant per admission, so the proposed limit is not applied. Nothing was revoked or changed.";
        let live = publication(OwnerGrantPublicationState::ServerObserved, Some("active"));
        assert_eq!(live_grant_tokens(&live), Some(2_000_000));
        assert_eq!(
            proposal_notice(&live, Some(3_000_000), "").unwrap(),
            format!("Per-request limit: 2 M → 3 M\n{PUBLISHED}")
        );
        // Revoked or expired grants still used this admission: no current limit,
        // and the proposal still cannot become a second grant.
        for server in ["revoked", "expired"] {
            let used = publication(OwnerGrantPublicationState::ServerObserved, Some(server));
            assert_eq!(live_grant_tokens(&used), None);
            assert_eq!(
                proposal_notice(&used, Some(3_000_000), "").unwrap(),
                format!("Per-request limit: None active → 3 M\n{PUBLISHED}")
            );
        }
        // No proposal, applicant or selector flag offers a commit beside a
        // published grant: nothing is revoked, reserved or published from here.
        let form = pool_form(10_000_000);
        let mut draft = initial_draft(Some(3_000_000));
        fill_form_defaults(&mut draft, &form);
        draft.coordinator = "@owner:test".into();
        draft.managers = "@manager:test".into();
        assert!(primary_available(&Phase::Choose, Some(&form), &draft, false, true));
        for server in ["active", "revoked", "expired"] {
            let phase = publication(OwnerGrantPublicationState::ServerObserved, Some(server));
            for has_original in [false, true] {
                for can_choose_new_limits in [false, true] {
                    assert!(!primary_available(&phase, Some(&form), &draft, has_original, can_choose_new_limits));
                }
            }
        }
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
            assert_eq!(live_grant_tokens(&phase), None);
            assert_eq!(proposal_notice(&phase, Some(3_000_000), "").unwrap(), EXISTING);
        }
        // An unattempted original reserved nothing; cancelling it reopens the form.
        for state in [OwnerGrantState::ReviewRequired, OwnerGrantState::ReviewExpired] {
            assert_eq!(
                proposal_notice(&progress(state, false), Some(3_000_000), "").unwrap(),
                format!("Per-request limit: None active → 3 M\n{PENDING_NOTE}")
            );
        }
        // A proposal adds or removes no action on an existing original.
        let draft = initial_draft(Some(3_000_000));
        assert!(primary_available(&progress(OwnerGrantState::Recorded, false), None, &draft, true, true));
        assert!(!primary_available(&progress(OwnerGrantState::OutcomeUnknown, false), None, &draft, true, true));
    }
    #[test]
    fn closing_beside_a_published_grant_names_why_the_proposal_was_not_applied() {
        for server in ["active", "revoked", "expired"] {
            let phase = publication(OwnerGrantPublicationState::ServerObserved, Some(server));
            assert_eq!(
                close_reply(&phase, Some(3_000_000), false, Some("original")),
                Err("pool_grant_already_published".into())
            );
            // Without a proposal the reply stays the public publication status.
            let reply: serde_json::Value =
                serde_json::from_str(&close_reply(&phase, None, false, Some("original")).unwrap()).unwrap();
            assert_eq!(reply["state"], "server_observed");
            assert_eq!(reply["serverState"], server);
            assert_eq!(reply["summary"]["allocatedTokens"], 2_000_000);
        }
        // Other outcomes keep their public progress so the app can recover them.
        let reserved: serde_json::Value = serde_json::from_str(
            &close_reply(&progress(OwnerGrantState::Recorded, false), Some(3_000_000), false, Some("original")).unwrap(),
        ).unwrap();
        assert_eq!(reserved["state"], "recorded");
        assert_eq!(reserved["summary"]["allocatedTokens"], 2_000_000);
        let unknown = close_reply(&publication(OwnerGrantPublicationState::OutcomeUnknown, None), Some(3_000_000), false, Some("original"));
        assert!(unknown.unwrap().contains("\"state\":\"outcome_unknown\""));
        // Closing the form itself applies nothing and names no grant.
        let parsed = |reply: Result<String, String>| serde_json::from_str::<serde_json::Value>(&reply.unwrap()).unwrap();
        assert_eq!(
            parsed(close_reply(&Phase::Choose, Some(3_000_000), false, None)),
            json!({"state":"review_closed","intentId":null})
        );
        assert_eq!(
            parsed(close_reply(&Phase::Loading, Some(3_000_000), true, None)),
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
                "每次申请上限：2 M → 3 M\n此资源池在当前准入下已发布过授权，而服务器每次准入只接受一个授权，因此提议的上限未生效。没有撤销或更改任何授权。"
            } else {
                "Per-request limit: 2 M → 3 M\nThis pool already has a published grant for its current admission, and the server accepts only one grant per admission, so the proposed limit is not applied. Nothing was revoked or changed."
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
