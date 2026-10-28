//! Trusted native owner grant screen. The package cannot render or acknowledge
//! this confirmation; its reply contains only public progress and original IDs.
use super::*;
use crate::host::hagency::grants::{GrantChoices, GrantFlow, GrantResult};
use ::hagency::native_owner::{
    OwnerGrantProgress, OwnerGrantReview, OwnerGrantState, OwnerGrantPublicationReview,
    OwnerGrantPublicationProgress, OwnerGrantPublicationState, OwnerGrantCancellationReview,
    OwnerGrantSummary,
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
        .map(|n| n.to_string())
        .unwrap_or_else(|| crate::i18n::tr("Unavailable").into())
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
    fn can_primary(&self) -> bool {
        self.actionable()
            && match &self.phase {
                Phase::Choose => true,
                Phase::Review(review) => !review.expired() && review.capacity_available(),
                Phase::Progress(progress)
                    if progress.closure.is_some()
                        && matches!(progress.state, OwnerGrantState::Recorded) =>
                {
                    self.flow.can_choose_new_limits()
                }
                Phase::Progress(progress) => {
                    matches!(
                        progress.state,
                        OwnerGrantState::ReviewRequired | OwnerGrantState::Recorded
                    ) || (matches!(progress.state, OwnerGrantState::Declined)
                        && self.flow.can_choose_new_limits())
                }
                Phase::PublicationReview(review) => !review.expired(),
                Phase::CancellationReview(review) => !review.expired(),
                Phase::Failed(code) => {
                    self.original.is_some()
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
    fn can_recover(&self) -> bool {
        self.actionable()
            && !matches!(self.phase, Phase::Choose | Phase::Loading)
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
        ("Tokens to reserve", s.allocated_tokens.to_string()),
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
        self.grant_confirmation = Some(GrantPrompt {
            flow: flow.clone(),
            reply,
            phase: Phase::Loading,
            original: None,
            receiver: None,
            draft: GrantChoices::default(),
            publishing: false,
            cancelling: false,
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
        let mut primary = if prompt.cancelling {
            "Review cancellation"
        } else if prompt.publishing {
            "Review publication"
        } else {
            "Review capacity"
        };
        let tr = crate::i18n::tr;
        match &prompt.phase {
            Phase::Loading=>status=tr("Checking the original intent…").into(),
            Phase::Choose=>status=tr("Choose limits in this native window. The next step shows the verified resource and available budget.").into(),
            Phase::Review(review)=> {
                primary="Reserve capacity";
                let s=review.summary();let c=review.context();
                let rows=[("Matrix server",c.server_origin.clone()),("Native profile",c.profile_id.clone()),
                    ("Resource",format!("{} · {}\n{}",c.framework,c.model,s.resource_id)),
                    ("Remaining / total tokens",format!("{} / {}",amount(c.remaining_tokens),amount(c.resource_total_tokens))),
                    ("Shared account limit",amount(c.shared_account_total_tokens)),
                    ("Tokens to reserve",s.allocated_tokens.to_string()),
                    ("Accounting period (UTC)",format!("{}\n{} → {}",s.period_key,utc(s.period_start_ms),utc(s.period_end_ms))),
                    ("Recorded coordinator",s.coordinator_mxid.clone()),("Agent applicants",s.eligible_managers.join("\n")),
                    ("Delegation expires (UTC)",utc(s.delegation_expires_at_ms)),("Original intent",s.intent_id.clone()),
                    ("Observed at",utc(c.observed_at_ms)),("Resource revision",c.resource_revision.clone())];
                details=rows.into_iter().map(|(label,value)|format!("{}\n{}",tr(label),value)).collect::<Vec<_>>().join("\n\n");
                status=tr(if review.capacity_available() {"This reserves capacity on this device. Publishing it to the server is a separate step; no agent starts."} else {"Available capacity is unknown or insufficient. This review cannot be confirmed."}).into();
            },
            Phase::Progress(p)=>{
                if matches!(p.state,OwnerGrantState::Recorded) { primary="Review publication"; }
                if matches!(p.state,OwnerGrantState::Declined) { primary="Choose new limits"; }
                details=format!("{}\n{}",tr("Original intent"),p.summary.intent_id);
                status=tr(match p.state {OwnerGrantState::Recorded=>"The original local reservation receipt is recorded. Review current eligibility before publishing.",
                    OwnerGrantState::ReviewRequired=>"The original intent is waiting for a fresh review.",
                    OwnerGrantState::OutcomeUnknown=>"The outcome is unknown. Recover this original intent; do not create a replacement.",
                    OwnerGrantState::Declined=>"This pending reservation was closed without reserving capacity. Its original record is kept.",OwnerGrantState::ReviewExpired=>"The original accounting period ended. This intent cannot move to a new month."}).into();
                if let Some(closure) = &p.closure {
                    primary="Review new admission";
                    status=tr("Previous grant closed. Retained amounts remain committed; a new grant requires a separately approved admission.").into();
                    let rows=[("Retained tokens",closure.retained_tokens.to_string()),
                        ("Confirmed usage",closure.known_consumed_tokens.to_string()),
                        ("Unresolved tokens",closure.unresolved_tokens.to_string()),
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
                primary="Publish to server";
                let s=review.summary();
                let rows=[("Matrix server",review.server_origin().to_owned()),
                    ("Resource",s.resource_id), ("Tokens reserved",s.allocated_tokens.to_string()),
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
                details=format!("{}\n{}",tr("Original intent"),p.summary.intent_id);
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
        self.view
            .label(cx, ids!(grant_confirmation.scroll.details.copy))
            .set_text(cx, &details);
        self.view
            .button(cx, ids!(grant_confirmation.buttons.primary))
            .set_text(cx, tr(primary));
        self.view
            .widget(cx, ids!(grant_confirmation.buttons.primary))
            .set_visible(cx, prompt.can_primary());
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
                Ok(GrantResult::Choose) => {
                    prompt.original = None;
                    prompt.publishing = false;
                    prompt.cancelling = false;
                    Phase::Choose
                }
                Ok(GrantResult::Review(review)) => {
                    prompt.original = Some(review.summary().intent_id);
                    prompt.cancelling = false;
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
            self.present_grant_confirmation(cx);
        }
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
                self.grant_job(async move { flow.prepare(choices).await });
            }
            Phase::Review(review) => self.grant_job(async move { flow.confirm(*review).await }),
            Phase::PublicationReview(review) => {
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
        let reply=match prompt.phase {
            Phase::Progress(p)=>serde_json::to_string(&p).map_err(|_|"management_outcome_unknown".to_owned()),
            Phase::PublicationProgress(p)=>serde_json::to_string(&p).map_err(|_|"management_outcome_unknown".to_owned()),
            _=>Ok(json!({"state":if prompt.receiver.is_some(){"outcome_unknown"}else{"review_closed"},"intentId":prompt.original}).to_string()),
        };
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
            let decrease=self.view.button(cx, ids!(grant_confirmation.scroll.choices.tokens.decrease));
            let increase=self.view.button(cx, ids!(grant_confirmation.scroll.choices.tokens.increase));
            if !decrease.disabled(cx) {targets.push(decrease.area());}
            targets.push(self.view.text_input(cx, ids!(grant_confirmation.scroll.choices.tokens.input)).area());
            if !increase.disabled(cx) {targets.push(increase.area());}
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
        assert_eq!(amount(Some(0)), "0");
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
                .set_visible(&mut cx, false);
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
