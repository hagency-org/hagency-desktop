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
    crate::i18n::tr(if yolo { "Full trust (unrestricted)" } else { "Standard permissions" }).into()
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

/// The native confirmation for a monthly-maximum amendment, in the mini
/// app's words: one sentence naming the change, then what stays the same.
/// Only the maximum changes; the list, the Agents and the per-request limit do not.
pub(crate) fn amendment_details(language: Language, review: &CeilingAmendmentReview) -> String {
    let current = crate::utils::format_token_count(review.previous_tokens);
    let new = crate::utils::format_token_count(review.tokens);
    let lead = match review.name.as_deref().map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => crate::i18n::format_in(
            language,
            "Change the monthly maximum of {pool} from {current} to {new}",
            &[("pool", name.to_owned()), ("current", current), ("new", new)],
        ),
        None => crate::i18n::format_in(
            language,
            "Change this pool’s monthly maximum from {current} to {new}",
            &[("current", current), ("new", new)],
        ),
    };
    let unchanged = match review.per_request_tokens {
        Some(limit) => crate::i18n::format_in(
            language,
            "The per-request limit stays {limit}; change it separately if needed. Running Agents aren’t affected.",
            &[("limit", crate::utils::format_token_count(limit))],
        ),
        None => crate::i18n::text_for(
            language,
            "The per-request limit doesn’t change. Running Agents aren’t affected.",
        )
        .to_owned(),
    };
    format!("{lead}\n{unchanged}")
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
    /// The request asked for a monthly-maximum amendment (it names the
    /// reviewed sequence), whether or not its review could be loaded.
    pub fn amendment_requested(&self) -> bool {
        self.input.expected_ceiling_sequence.is_some()
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
                let saved = match &result.ceiling_amendment {
                    Some(amendment) => crate::i18n::format(
                        "The monthly maximum was already saved: {current} → {new}.",
                        &[
                            ("current", crate::utils::format_token_count(amendment.previous_tokens)),
                            ("new", crate::utils::format_token_count(amendment.tokens)),
                        ],
                    ),
                    None => tr("This change was already saved.").into(),
                };
                format!(
                    "{saved}\n\n{}: {}\n{}: {}\n{}: {}",
                    tr("Resource ID"),
                    self.input.resource_id,
                    tr("Record ID"),
                    self.input.command_id,
                    tr("Saved revision"),
                    result.revision
                )
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
                let name = old
                    .local_name
                    .as_ref()
                    .map(|local| local.name.trim())
                    .filter(|name| !name.is_empty())
                    .unwrap_or(old.model.as_str())
                    .to_owned();
                format!(
                    "{}\n\n{}\n{} · {}\n{}: {}\n{}: {}\n\n{}\n{} · {}\n{}: {}\n{}: {}\n\n{}",
                    name,
                    tr("Current configuration"),
                    old.model,
                    reasoning_text(old.reasoning.as_deref()),
                    tr("Monthly maximum"),
                    ceiling(previous),
                    tr("Default permissions for new Agents"),
                    policy_text(detail.execution_policy.yolo),
                    tr("Proposed configuration"),
                    model,
                    reasoning_text(reasoning.as_deref()),
                    tr("Monthly maximum"),
                    ceiling(next),
                    tr("Default permissions for new Agents"),
                    policy_text(self.input.execution_policy.unwrap_or(detail.execution_policy).yolo),
                    tr("Existing Agents keep their current permissions.")
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
            "Change the monthly maximum of 写作池 from 3 M to 5 M\nThe per-request limit stays 1 M; change it separately if needed. Running Agents aren’t affected."
        );
        assert_eq!(
            amendment_details(Language::Chinese, &review),
            "把「写作池」的每月上限从 3 M 改为 5 M\n每次申请上限仍为 1 M，需要时另外修改；正在运行的 Agent 不受影响。"
        );
        let unnamed = CeilingAmendmentReview {
            name: None,
            per_request_tokens: None,
            ..review
        };
        // No ID stands in for a missing name.
        let english = amendment_details(Language::English, &unnamed);
        assert!(english.starts_with("Change this pool’s monthly maximum from 3 M to 5 M\n"), "{english}");
        assert!(!english.contains("resource_1"), "{english}");
        assert!(english.ends_with("The per-request limit doesn’t change. Running Agents aren’t affected."), "{english}");
        assert_eq!(
            amendment_details(Language::Chinese, &unnamed),
            "把这个资源池的每月上限从 3 M 改为 5 M\n每次申请上限不变；正在运行的 Agent 不受影响。"
        );
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
