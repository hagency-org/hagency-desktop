//! Closed native review; the package supplies a proposal, never the source values,
//! eligibility, provider identity, filesystem paths or confirmation authority.
use super::*;
use ::hagency::native_owner::{
    CeilingChange, CreateResourceConfiguration, ResourceConfigurationReview, ProfileChange,
};

pub(crate) const CREATE: &str = "hagency.resources.create";

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

pub(crate) struct ResourceCreateConsent {
    host: Host,
    lease: Lease,
    input: CreateResourceConfiguration,
    expires: Instant,
    review: Option<ResourceConfigurationReview>,
}

pub(super) fn parse_create(args: Value) -> Result<CreateResourceConfiguration, String> {
    let input: CreateResourceConfiguration = parse(args)?;
    input
        .validate()
        .map_err(|_| "invalid_management_arguments".to_owned())?;
    Ok(input)
}

impl Host {
    pub fn resource_create_consent(
        &self,
        lease: Lease,
        args: Value,
    ) -> Result<ResourceCreateConsent, String> {
        let input = parse_create(args)?;
        let account = self.check(&lease)?;
        lease.authorize(&account, CREATE, None)?;
        Ok(ResourceCreateConsent {
            host: self.clone(),
            lease,
            input,
            expires: Instant::now() + Duration::from_secs(300),
            review: None,
        })
    }
}

impl ResourceCreateConsent {
    pub fn account(&self) -> &str {
        &self.lease.identity().account
    }
    pub fn expired(&self) -> bool {
        Instant::now() >= self.expires
    }
    pub fn recovering(&self) -> bool {
        matches!(self.review, Some(ResourceConfigurationReview::Recorded(_)))
    }
    fn authorize(&self) -> Result<(), String> {
        if self.expired() {
            return Err("provider_confirmation_expired".into());
        }
        let account = self.host.check(&self.lease)?;
        self.lease.authorize(&account, CREATE, None)
    }
    pub async fn load(mut self) -> Result<Self, String> {
        self.authorize()?;
        let manager = self.host.manager(&self.lease).await?;
        self.authorize()?;
        let review = manager
            .review_resource_configuration(&self.input)
            .await
            .map_err(|e| e.code)?;
        self.authorize()?;
        self.review = Some(review);
        Ok(self)
    }
    pub fn details(&self) -> String {
        let tr = crate::i18n::tr;
        match &self.review {
            Some(ResourceConfigurationReview::Recorded(result)) => format!(
                "{}\n{}: {}\n{}: {}\n{}: {}",
                self.input.source_resource_id,
                tr("New resource"),
                result.resource_id,
                tr("Original request"),
                self.input.command_id,
                tr("Saved revision"),
                result.revision
            ),
            Some(ResourceConfigurationReview::Available(detail)) => {
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
                // The pool by its own name, or its model; IDs stay out of the summary.
                let name = old
                    .local_name
                    .as_ref()
                    .map(|local| local.name.trim())
                    .filter(|name| !name.is_empty())
                    .unwrap_or(old.model.as_str())
                    .to_owned();
                let account_note = old.budget.seat.quota.map(|n| format!("{}: {}", tr("Explicit shared account limit"), ceiling(Some(u64::from(n)))))
                    .unwrap_or_else(|| tr("Each pool uses the monthly maximum set by its owner.").into());
                format!(
                    "{}\n\n{}\n{} · {}\n{}: {}\n{}: {}\n\n{}\n{} · {}\n{}: {}\n{}: {}\n\n{}\n{}",
                    name,
                    tr("Source configuration"),
                    old.model,
                    reasoning_text(old.reasoning.as_deref()),
                    tr("Monthly maximum"),
                    ceiling(previous),
                    tr("Default permissions for new Agents"),
                    super::resource_edit::policy_text(detail.execution_policy.yolo),
                    tr("New configuration"),
                    model,
                    reasoning_text(reasoning.as_deref()),
                    tr("Monthly maximum"),
                    ceiling(next),
                    tr("Default permissions for new Agents"),
                    super::resource_edit::policy_text(self.input.execution_policy.unwrap_or(detail.execution_policy).yolo),
                    account_note,
                    tr("Existing Agents keep their current permissions.")
                )
            }
            None => tr("Reading current resource configuration…").into(),
        }
    }
    pub async fn accept(self) -> Result<Value, String> {
        self.authorize()?;
        if self.review.is_none() {
            return Err("resource_create_review_required".into());
        }
        let manager = self.host.manager(&self.lease).await?;
        self.authorize()?;
        let result = manager
            .create_resource_configuration(self.input, self.expires.min(Instant::now() + Duration::from_secs(30)))
            .await
            .map_err(|e| e.code)
            .and_then(value);
        self.host
            .check(&self.lease)
            .map_err(|_| "management_outcome_unknown".to_owned())?;
        self.lease
            .authorize(&self.lease.identity().account, CREATE, None)
            .map_err(|_| "management_outcome_unknown".to_owned())?;
        rinx_miniapp_core::bounded_reply(result?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resource_create_service_has_closed_input_and_only_its_required_permissions() {
        let valid = json!({"commandId":"original","sourceResourceId":"resource_1","expectedRevision":"a".repeat(64),"profile":ProfileChange::Preserve {},"ceiling":CeilingChange::Monthly { tokens:450.try_into().unwrap() }});
        assert_eq!(serde_json::to_value(parse_create(valid.clone()).unwrap()).unwrap(), valid);
        for yolo in [false, true] {
            let mut configured = valid.clone();
            configured["executionPolicy"] = json!({"yolo":yolo});
            assert_eq!(parse_create(configured).unwrap().execution_policy.unwrap().yolo, yolo);
        }
        for policy in [json!({"yolo":"true"}), json!({"yolo":true,"approved":true}), json!({})] {
            let mut invalid = valid.clone(); invalid["executionPolicy"] = policy;
            assert!(parse_create(invalid).is_err());
        }
        for key in [
            "create",
            "resourceId",
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
            assert!(parse_create(injected).is_err(), "{key}");
        }
        let authority = rinx_miniapp_core::SessionAuthority::default();
        let lease = authority.issue(
            InstanceId {
                app: octosense_app_contract::hagency::APP_ID.into(),
                account: "@owner:test".into(),
                room: None,
                generation: 1,
            },
            [CREATE.to_owned()].into_iter().collect(),
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
        assert!(octosense_app_contract::hagency::SERVICES.contains(&CREATE));
        assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&CREATE));
        assert!(octosense_app_contract::hagency::words(CREATE).is_some());
    }
}
