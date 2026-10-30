//! Load canonical readback before enabling the existing native consent footer.
use super::*;
use crate::host::hagency::resource_edit::ResourceEditConsent;

/// Native panel words for a monthly-maximum amendment: title, purpose,
/// consequence and confirmation. The change itself is one sentence in the
/// details card (`amendment_details`), so purpose and consequence stay empty.
pub(super) const AMENDMENT_WORDS: (&str, &str, &str, &str) = (
    "Change the monthly maximum",
    "",
    "",
    "Save monthly maximum",
);

/// The English catalog key shown when the review could not be loaded.
fn failure_text(code: Option<&str>, amendment: bool) -> &'static str {
    match code {
        None if amendment => "Reading this pool…",
        None => "Reading current resource configuration…",
        Some("management_resource_edit_changed") => "This pool just changed, so nothing was saved. Close this window and refresh, then try again.",
        Some("management_resource_edit_not_available") => "This pool can’t be changed here right now. Close this window and refresh to see why.",
        Some("management_ceiling_amendment_not_available") => "This pool’s monthly maximum can’t change right now. Close this window and refresh to see why.",
        Some("management_ceiling_clear_refused") => "A pool already offered to a server can’t drop its monthly maximum. You can lower it instead.",
        Some("management_ceiling_unchanged") => "The monthly maximum already has this value. Nothing was changed.",
        Some("management_ceiling_below_commitments") => "The monthly maximum can’t be lower than what this month has committed: what was used plus what Agents hold. Close this window and choose a larger amount.",
        Some("management_ceiling_usage_unsettled") => "Some Agent use is still being reported, so the monthly maximum can only go up for now. Keep this computer online, then try again.",
        Some(_) => "The current settings couldn’t be confirmed. Close this window and refresh, then try again.",
    }
}

/// Offline review only: the words, details and readiness of the monthly
/// maximum window for a fixture, through the same text as the real window.
#[cfg(any(test, feature = "onboarding-test-support"))]
pub(super) fn preview(fixture: &Value) -> Result<((&'static str, &'static str, &'static str, &'static str), String, bool), String> {
    if let Some(code) = fixture["error"].as_str() {
        return Ok((AMENDMENT_WORDS, crate::i18n::tr(failure_text(Some(code), true)).into(), false));
    }
    let tokens = |key: &str| fixture[key].as_u64().ok_or_else(|| format!("preview field {key} missing"));
    let review = ::hagency::native_owner::CeilingAmendmentReview {
        resource_id: fixture["resource"].as_str().unwrap_or("resource_original").into(),
        name: fixture["label"].as_str().map(str::to_owned),
        revision: "0".repeat(64),
        sequence: 1,
        previous_tokens: tokens("current")?,
        tokens: tokens("proposed")?,
        per_request_tokens: fixture["perRequest"].as_u64(),
    };
    Ok((
        AMENDMENT_WORDS,
        crate::host::hagency::resource_edit::amendment_details(crate::i18n::language(), &review),
        true,
    ))
}

pub(super) struct ResourceEditPrompt {
    account: String,
    /// The request asked for a monthly-maximum amendment.
    amendment: bool,
    expires: Instant,
    receiver: Option<Receiver<Result<ResourceEditConsent, String>>>,
    consent: Option<ResourceEditConsent>,
    failure: Option<String>,
}
impl ResourceEditPrompt {
    pub(super) fn begin(consent: ResourceEditConsent) -> Self {
        let account = consent.account().to_owned();
        let amendment = consent.amendment_requested();
        let (tx, receiver) = mpsc::sync_channel(1);
        crate::sliding_sync::spawn_async_task(async move {
            let result = consent.load().await;
            let _ = tx.try_send(result);
            SignalToUI::set_ui_signal();
        });
        Self {
            account,
            amendment,
            expires: Instant::now() + Duration::from_secs(300),
            receiver: Some(receiver),
            consent: None,
            failure: None,
        }
    }
    pub(super) fn poll(&mut self) -> bool {
        let Some(receiver) = &self.receiver else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err("management_unavailable".into()),
        };
        self.receiver = None;
        match result {
            Ok(consent) => self.consent = Some(consent),
            Err(code) => self.failure = Some(code),
        }
        true
    }
    pub(super) fn account(&self) -> &str {
        &self.account
    }
    pub(super) fn expired(&self) -> bool {
        Instant::now() >= self.expires
            || self
                .consent
                .as_ref()
                .is_some_and(ResourceEditConsent::expired)
    }
    pub(super) fn ready(&self) -> bool {
        self.consent.is_some() && !self.expired()
    }
    pub(super) fn recovering(&self) -> bool {
        self.consent
            .as_ref()
            .is_some_and(ResourceEditConsent::recovering)
    }
    pub(super) fn amending(&self) -> bool {
        self.consent
            .as_ref()
            .is_some_and(ResourceEditConsent::amending)
    }
    /// A monthly-maximum request keeps its own window, even when its review
    /// could not be loaded and the window only says why.
    pub(super) fn amendment_requested(&self) -> bool {
        self.amendment
    }
    pub(super) fn details(&self) -> String {
        if let Some(consent) = &self.consent {
            return consent.details();
        }
        crate::i18n::tr(failure_text(self.failure.as_deref(), self.amendment)).into()
    }
    pub(super) async fn accept(self) -> Result<Value, String> {
        if self.expired() {
            return Err("provider_confirmation_expired".into());
        }
        self.consent
            .ok_or_else(|| "resource_edit_review_required".to_owned())?
            .accept()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::{AMENDMENT_WORDS, failure_text};
    use crate::i18n::{Language, text_for};

    #[test]
    fn ceiling_amendment_panel_and_refusals_are_localized_and_distinct() {
        let (title, purpose, consequence, confirm) = AMENDMENT_WORDS;
        for english in [title, confirm] {
            assert_ne!(text_for(Language::Chinese, english), english, "{english}");
        }
        // The change and what stays the same are one sentence each in the details card.
        assert!(purpose.is_empty() && consequence.is_empty());
        assert_eq!(text_for(Language::Chinese, title), "修改每月上限");
        assert_eq!(text_for(Language::Chinese, confirm), "保存每月上限");
        let generic = failure_text(Some("management_unavailable"), true);
        for code in [
            "management_ceiling_amendment_not_available",
            "management_ceiling_clear_refused",
            "management_ceiling_unchanged",
            "management_ceiling_below_commitments",
            "management_ceiling_usage_unsettled",
        ] {
            let english = failure_text(Some(code), true);
            assert_ne!(english, generic, "{code} has its own reason");
            assert_ne!(text_for(Language::Chinese, english), english, "{code} is translated");
        }
        assert!(failure_text(Some("management_ceiling_below_commitments"), true).contains("what was used plus what Agents hold"));
        assert_eq!(
            text_for(Language::Chinese, failure_text(Some("management_ceiling_usage_unsettled"), true)),
            "部分 Agent 的用量还在上报，所以暂时只能调高、不能调低每月上限。请保持这台电脑在线，稍后再试。"
        );
        assert_eq!(text_for(Language::Chinese, failure_text(None, true)), "正在读取这个资源池…");
    }
}
