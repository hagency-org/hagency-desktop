//! Closed native review; the package supplies a proposal, never the old values,
//! eligibility, provider identity, filesystem paths or confirmation authority.
//!
//! The same service amends the monthly maximum of a resource that was offered,
//! published or retains commitments, when `ceilingAmendment.allowed` in its
//! detail. The request then changes nothing else and names the reviewed
//! sequence: `{resourceId, expectedRevision, expectedCeilingSequence,
//! profile:{kind:"preserve"}, ceiling:{kind:"monthly", tokens}}`. The native
//! review shows old → new; the definition, its revision, server admission,
//! running Agents and the per-request limit stay unchanged.
use super::*;
use ::hagency::native_owner::{
    CeilingAmendmentReview, CeilingChange, EditLocalResource, LocalResourceEditReview,
    ProfileChange,
};
use crate::i18n::Language;

pub(crate) const EDIT: &str = "hagency.resources.edit";

pub(super) fn policy_text(yolo: bool) -> String {
    crate::i18n::tr(if yolo { "Unrestricted" } else { "Standard permissions" }).into()
}

fn reasoning_text(value: Option<&str>) -> String {
    let key = match value {
        None => "Default",
        Some("none") => "None",
        Some("minimal") => "Minimal",
        Some("low") => "Low",
        Some("medium") => "Medium",
        Some("high") => "High",
        Some("xhigh") => "Extra high",
        Some("max") => "Maximum",
        Some("ultra") => "Ultra",
        Some(other) => return other.to_owned(),
    };
    crate::i18n::tr(key).into()
}

/// "Monthly maximum (offered pool): old → new" in `language`.
fn amendment_change(language: Language, previous: u64, tokens: u64) -> String {
    crate::i18n::format_in(
        language,
        "Monthly maximum (offered pool): {current} → {new}",
        &[
            ("current", crate::utils::format_token_count(previous)),
            ("new", crate::utils::format_token_count(tokens)),
        ],
    )
}

/// The native confirmation for a monthly-maximum amendment: only the maximum
/// changes; the definition, admission, Agents and per-request limit do not.
pub(crate) fn amendment_details(language: Language, review: &CeilingAmendmentReview) -> String {
    let text = |english| crate::i18n::text_for(language, english);
    let per_request = match review.per_request_tokens {
        Some(limit) => crate::i18n::format_in(
            language,
            "The per-request limit stays {limit} until you change it separately.",
            &[("limit", crate::utils::format_token_count(limit))],
        ),
        None => text("Per-request limits do not change.").to_owned(),
    };
    format!(
        "{}\n\n{}\n{}\n{}\n{}",
        review.name.as_deref().unwrap_or(&review.resource_id),
        amendment_change(language, review.previous_tokens, review.tokens),
        text("The pool's definition and its server admission do not change."),
        text("Running Agents keep running."),
        per_request,
    )
}

pub(crate) struct ResourceEditConsent {
    host: Host,
    lease: Lease,
    input: EditLocalResource,
    expires: Instant,
    review: Option<LocalResourceEditReview>,
}

pub(super) fn parse_edit(args: Value) -> Result<EditLocalResource, String> {
    let input: EditLocalResource = parse(args)?;
    input
        .validate()
        .map_err(|_| "invalid_management_arguments".to_owned())?;
    Ok(input)
}

impl Host {
    pub fn resource_edit_consent(
        &self,
        lease: Lease,
        args: Value,
    ) -> Result<ResourceEditConsent, String> {
        let input = parse_edit(args)?;
        let account = self.check(&lease)?;
        lease.authorize(&account, EDIT, None)?;
        Ok(ResourceEditConsent {
            host: self.clone(),
            lease,
            input,
            expires: Instant::now() + Duration::from_secs(300),
            review: None,
        })
    }
}

impl ResourceEditConsent {
    pub fn account(&self) -> &str {
        &self.lease.identity().account
    }
    pub fn expired(&self) -> bool {
        Instant::now() >= self.expires
    }
    pub fn recovering(&self) -> bool {
        matches!(self.review, Some(LocalResourceEditReview::Recorded(_)))
    }
    /// The review is an append-only monthly-maximum amendment, not an edit.
    pub fn amending(&self) -> bool {
        matches!(self.review, Some(LocalResourceEditReview::CeilingAmendment(_)))
    }
    fn authorize(&self) -> Result<(), String> {
        if self.expired() {
            return Err("provider_confirmation_expired".into());
        }
        let account = self.host.check(&self.lease)?;
        self.lease.authorize(&account, EDIT, None)
    }
    pub async fn load(mut self) -> Result<Self, String> {
        self.authorize()?;
        let manager = self.host.manager(&self.lease).await?;
        self.authorize()?;
        let review = manager
            .review_local_resource_edit(&self.input)
            .await
            .map_err(|e| e.code)?;
        self.authorize()?;
        self.review = Some(review);
        Ok(self)
    }
    pub fn details(&self) -> String {
        let tr = crate::i18n::tr;
        match &self.review {
            Some(LocalResourceEditReview::Recorded(result)) => {
                let saved = format!(
                    "{}\n{}: {}\n{}: {}",
                    self.input.resource_id,
                    tr("Original request"),
                    self.input.command_id,
                    tr("Saved revision"),
                    result.revision
                );
                match &result.ceiling_amendment {
                    Some(amendment) => format!(
                        "{saved}\n{}",
                        amendment_change(
                            crate::i18n::language(),
                            amendment.previous_tokens,
                            amendment.tokens
                        )
                    ),
                    None => saved,
                }
            }
            Some(LocalResourceEditReview::CeilingAmendment(review)) => {
                amendment_details(crate::i18n::language(), review)
            }
            Some(LocalResourceEditReview::Editable(detail)) => {
                let old = &detail.resource;
                let (model, reasoning) = match &self.input.profile {
                    ProfileChange::Preserve {} => (&old.model, &old.reasoning),
                    ProfileChange::Select { model, reasoning } => (model, reasoning),
                };
                let ceiling = |value: Option<u64>| {
                    value
                        .map(crate::utils::format_token_count)
                        .unwrap_or_else(|| tr("Unknown").into())
                };
                let previous = detail
                    .ceiling
                    .as_ref()
                    .and_then(|v| v.tokens)
                    .map(u64::from);
                let next = match &self.input.ceiling {
                    CeilingChange::Preserve {} => previous,
                    CeilingChange::Clear {} => None,
                    CeilingChange::Monthly { tokens } => Some(u64::from(*tokens)),
                };
                format!(
                    "{}\n\n{}\n{} · {}\n{}: {}\n{}: {}\n\n{}\n{} · {}\n{}: {}\n{}: {}\n\n{}",
                    old.id,
                    tr("Current configuration"),
                    old.model,
                    reasoning_text(old.reasoning.as_deref()),
                    tr("Monthly resource token limit"),
                    ceiling(previous),
                    tr("Default permissions for new agents"),
                    policy_text(detail.execution_policy.yolo),
                    tr("Proposed configuration"),
                    model,
                    reasoning_text(reasoning.as_deref()),
                    tr("Monthly resource token limit"),
                    ceiling(next),
                    tr("Default permissions for new agents"),
                    policy_text(self.input.execution_policy.unwrap_or(detail.execution_policy).yolo),
                    tr("Existing agents keep their current permissions.")
                )
            }
            None => tr("Reading current resource configuration…").into(),
        }
    }
    pub async fn accept(self) -> Result<Value, String> {
        self.authorize()?;
        if self.review.is_none() {
            return Err("resource_edit_review_required".into());
        }
        let manager = self.host.manager(&self.lease).await?;
        self.authorize()?;
        let result = manager
            .edit_local_resource(self.input, self.expires)
            .await
            .map_err(|e| e.code)
            .and_then(value);
        self.host
            .check(&self.lease)
            .map_err(|_| "management_outcome_unknown".to_owned())?;
        self.lease
            .authorize(&self.lease.identity().account, EDIT, None)
            .map_err(|_| "management_outcome_unknown".to_owned())?;
        rinx_miniapp_core::bounded_reply(result?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ceiling_amendment_review_says_only_the_monthly_maximum_changes_in_both_languages() {
        let review = CeilingAmendmentReview {
            resource_id: "resource_1".into(),
            name: Some("写作池".into()),
            revision: "a".repeat(64),
            sequence: 2,
            previous_tokens: 3_000_000,
            tokens: 5_000_000,
            per_request_tokens: Some(1_000_000),
        };
        assert_eq!(
            amendment_details(Language::English, &review),
            "写作池\n\nMonthly maximum (offered pool): 3 M → 5 M\nThe pool's definition and its server admission do not change.\nRunning Agents keep running.\nThe per-request limit stays 1 M until you change it separately."
        );
        assert_eq!(
            amendment_details(Language::Chinese, &review),
            "写作池\n\n每月总上限（已贡献的资源池）：3 M → 5 M\n资源池的定义及其服务器准入保持不变。\n正在运行的 Agent 会继续运行。\n每次申请上限仍为 1 M，如需调整请另行修改。"
        );
        let unnamed = CeilingAmendmentReview {
            name: None,
            per_request_tokens: None,
            ..review
        };
        let english = amendment_details(Language::English, &unnamed);
        assert!(english.starts_with("resource_1\n\n"), "{english}");
        assert!(english.ends_with("Per-request limits do not change."), "{english}");
        assert!(amendment_details(Language::Chinese, &unnamed).ends_with("每次申请上限保持不变。"));
    }
    #[test]
    fn ceiling_amendment_input_is_ceiling_only_and_names_its_reviewed_sequence() {
        let amend = json!({"commandId":"original","resourceId":"resource_1","expectedRevision":"a".repeat(64),"profile":{"kind":"preserve"},"ceiling":{"kind":"monthly","tokens":5_000_000u64},"expectedCeilingSequence":2});
        let parsed = parse_edit(amend.clone()).unwrap();
        assert_eq!(parsed.expected_ceiling_sequence, Some(2));
        assert_eq!(serde_json::to_value(&parsed).unwrap(), amend);
        for (field, value) in [
            ("profile", json!({"kind":"select","model":"gpt-5.6-sol","reasoning":"high"})),
            ("executionPolicy", json!({"yolo":true})),
            ("ceiling", json!({"kind":"preserve"})),
            ("ceiling", json!({"kind":"monthly","tokens":0})),
            ("expectedCeilingSequence", json!(-1)),
            ("expectedCeilingSequence", json!(9_007_199_254_740_992u64)),
        ] {
            let mut invalid = amend.clone();
            invalid[field] = value;
            assert!(parse_edit(invalid).is_err(), "{field}");
        }
        // A clear stays well-formed: the native review refuses it with its reason.
        let mut clear = amend.clone();
        clear["ceiling"] = json!({"kind":"clear"});
        assert!(parse_edit(clear).is_ok());
        for key in ["previousTokens", "perRequestTokens", "minimumTokens", "sequence", "allowed"] {
            let mut injected = amend.clone();
            injected[key] = json!(1);
            assert!(parse_edit(injected).is_err(), "{key}");
        }
    }
    #[test]
    fn resource_edit_service_has_closed_input_and_only_its_required_permissions() {
        let valid = json!({"commandId":"original","resourceId":"resource_1","expectedRevision":"a".repeat(64),"profile":ProfileChange::Preserve {},"ceiling":CeilingChange::Monthly { tokens:450.try_into().unwrap() }});
        assert_eq!(serde_json::to_value(parse_edit(valid.clone()).unwrap()).unwrap(), valid);
        for yolo in [false, true] {
            let mut configured = valid.clone();
            configured["executionPolicy"] = json!({"yolo":yolo});
            assert_eq!(parse_edit(configured).unwrap().execution_policy.unwrap().yolo, yolo);
        }
        for policy in [json!({"yolo":"true"}), json!({"yolo":true,"approved":true}), json!({})] {
            let mut invalid = valid.clone(); invalid["executionPolicy"] = policy;
            assert!(parse_edit(invalid).is_err());
        }
        for key in [
            "create",
            "owner",
            "provider",
            "seatId",
            "path",
            "url",
            "published",
            "eligible",
            "approved",
            "currentModel",
            "confirmedUntil",
        ] {
            let mut injected = valid.clone();
            injected[key] = json!("forged");
            assert!(parse_edit(injected).is_err(), "{key}");
        }
        let authority = rinx_miniapp_core::SessionAuthority::default();
        let lease = authority.issue(
            InstanceId {
                app: octosense_app_contract::hagency::APP_ID.into(),
                account: "@owner:test".into(),
                room: None,
                generation: 1,
            },
            [EDIT.to_owned()].into_iter().collect(),
            Default::default(),
            Instant::now() + Duration::from_secs(60),
        );
        assert_eq!(
            permissions(&lease),
            vec![
                ManagementPermission::ReadResources,
                ManagementPermission::ConfigureResources
            ]
        );
        assert!(octosense_app_contract::hagency::SERVICES.contains(&EDIT));
        assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&EDIT));
        assert!(octosense_app_contract::hagency::words(EDIT).is_some());
    }
}
