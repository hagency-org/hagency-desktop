//! Native owner-budget handoff. No script receives ConsentGrants, a registry,
//! provider identity, private receipt, or caller-selected original command IDs.
use super::{Host, Lease, parse};
use ::hagency::native_owner::{
    OwnerGrantAccess, OwnerGrantChoices, OwnerGrantProgress, OwnerGrantReview,
    OwnerGrantReviewResult, OwnerGrantPublicationReview, OwnerGrantPublicationResult,
    OwnerGrantPublicationProgress, OwnerGrantCancellationReview, OwnerGrantCancellationResult,
    OwnerGrantState,
};
use serde::Deserialize;
use serde_json::Value;

pub(crate) const REVIEW_GRANT: &str = "hagency.grants.review";

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Selector {
    contribution_id: Option<String>,
    intent_id: Option<String>,
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
        {
            return Err("invalid_management_arguments".into());
        }
        Ok(value)
    }
}

/// Strings read from native widgets only. This is intentionally not Deserialize.
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
            || amount > 9_007_199_254_740_991
            || managers.is_empty()
            || managers.len() > 64
        {
            return Err("invalid_grant_budget".into());
        }
        let coordinator = self.coordinator.trim().to_owned();
        if std::iter::once(&coordinator).chain(&managers).any(|id| {
            id.len() > 255
                || id.chars().any(|c| c.is_whitespace() || c.is_control())
                || !id.split_once(':').is_some_and(|(name, server)| {
                    name.starts_with('@') && name.len() > 1 && !server.is_empty()
                })
        }) {
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
    Choose,
    Review(Box<OwnerGrantReview>),
    Progress(Box<OwnerGrantProgress>),
    PublicationReview(Box<OwnerGrantPublicationReview>),
    PublicationProgress(Box<OwnerGrantPublicationProgress>),
    CancellationReview(Box<OwnerGrantCancellationReview>),
}
// Journal order is creation order. Cancelled or canonically closed originals
// remain in history without hiding a pending, unknown or active original.
fn existing_progress(records: Vec<OwnerGrantProgress>) -> Result<Option<OwnerGrantProgress>, String> {
    let mut pending = None;
    let mut declined = None;
    for record in records {
        if matches!(record.state, OwnerGrantState::Declined)
            || (matches!(record.state, OwnerGrantState::Recorded) && record.closure.is_some()) {
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
}
impl GrantFlow {
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
            if let Some(progress) = existing_progress(records)? {
                return self.observe_progress(progress).await;
            }
            Ok(GrantResult::Choose)
        } else {
            self.recover(self.selector.intent_id.clone().ok_or("owner_grant_original_intent_required")?).await
        }
    }
    pub fn can_choose_new_limits(&self) -> bool {
        self.selector.contribution_id.is_some()
    }
    /// Closed grants require a currently approved, distinct admission before
    /// reopening choices. Prepare and the atomic writer repeat their checks.
    pub async fn choose_new_limits(&self) -> Result<GrantResult, String> {
        if !self.can_choose_new_limits() {
            return Err("owner_grant_original_intent_required".into());
        }
        match self.open().await? {
            GrantResult::Progress(p) if matches!(p.state, OwnerGrantState::Declined) => Ok(GrantResult::Choose),
            GrantResult::Progress(p) if matches!(p.state, OwnerGrantState::Recorded) && p.closure.is_some() => {
                self.access.check_new_admission(self.selector.contribution_id.clone()
                    .ok_or("owner_grant_original_intent_required")?).await.map_err(|e| e.code)?;
                self.check()?;
                Ok(GrantResult::Choose)
            }
            result => Ok(result),
        }
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
            summary: ::hagency::native_owner::OwnerGrantSummary {
                intent_id: id.into(), owner_mxid: "@owner:test".into(),
                contribution_id: "contribution".into(), resource_id: "resource".into(),
                coordinator_mxid: "@coordinator:test".into(), allocated_tokens: 700,
                eligible_managers: vec!["@manager:test".into()], allow_self_approval: false,
                delegation_expires_at_ms: 1793491200000, period_key: "2026-10".into(),
                period_start_ms: 1790812800000, period_end_ms: 1793491200000,
            },
        }
    }
    #[test]
    fn closed_originals_never_hide_pending_unknown_or_recorded_effects() {
        for state in [OwnerGrantState::ReviewRequired, OwnerGrantState::ReviewExpired,
            OwnerGrantState::OutcomeUnknown, OwnerGrantState::Recorded] {
            let result = existing_progress(vec![progress("closed-before", OwnerGrantState::Declined),
                progress("original", state), progress("closed-after", OwnerGrantState::Declined)]).unwrap().unwrap();
            assert_eq!(result.summary.intent_id, "original");
            assert!(!matches!(result.state, OwnerGrantState::Declined));
        }
        assert!(existing_progress(vec![progress("one", OwnerGrantState::OutcomeUnknown),
            progress("two", OwnerGrantState::ReviewRequired)]).is_err());
    }
    #[test]
    fn cancellation_history_keeps_latest_closed_original_for_explicit_new_choices() {
        let result = existing_progress(vec![progress("first", OwnerGrantState::Declined),
            progress("second", OwnerGrantState::Declined)]).unwrap().unwrap();
        assert_eq!(result.summary.intent_id, "second");
        assert!(matches!(result.state, OwnerGrantState::Declined));
        assert!(existing_progress(vec![]).unwrap().is_none());
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
                settled("other_closed")]).unwrap().unwrap();
            assert_eq!(p.summary.intent_id, "current");
            assert!(p.closure.is_none());
        }
        let p = existing_progress(vec![settled("first"), settled("last")]).unwrap().unwrap();
        assert_eq!(p.summary.intent_id, "last");
        assert_eq!(p.closure.unwrap().retained_tokens, 200);
    }
    #[test]
    fn settlement_metadata_cannot_hide_an_unknown_effect() {
        let mut unknown = settled("unknown");
        unknown.state = OwnerGrantState::OutcomeUnknown;
        assert!(existing_progress(vec![unknown, progress("current", OwnerGrantState::Recorded)]).is_err());
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
