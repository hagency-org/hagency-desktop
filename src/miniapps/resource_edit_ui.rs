//! Load canonical readback before enabling the existing native consent footer.
use super::*;
use crate::host::hagency::resource_edit::ResourceEditConsent;

/// Native panel words for a monthly-maximum amendment: title, purpose,
/// consequence and confirmation. Only the maximum changes.
pub(super) const AMENDMENT_WORDS: (&str, &str, &str, &str) = (
    "Change the monthly maximum",
    "Hagency will recheck this pool's current maximum and this month's commitments before saving. Nothing is sent to the server.",
    "This records a new monthly maximum on this device. The pool's definition, its server admission and its running Agents stay as they are. The per-request limit is changed separately.",
    "Save monthly maximum",
);

/// The English catalog key shown when the review could not be loaded.
fn failure_text(code: Option<&str>) -> &'static str {
    match code {
        None => "Reading current resource configuration…",
        Some("management_resource_edit_changed") => "The resource changed. Close this review and refresh its details. Saved requests remain available for recovery.",
        Some("management_resource_edit_not_available") => "This resource cannot be edited locally. Close this review and refresh to see the current reason.",
        Some("management_ceiling_amendment_not_available") => "This resource's monthly maximum cannot be changed here. Close this review and refresh to see the current reason.",
        Some("management_ceiling_clear_refused") => "The monthly maximum of an offered pool cannot be cleared. Choose a whole number of tokens instead.",
        Some("management_ceiling_unchanged") => "The monthly maximum already has this value. Nothing was changed.",
        Some("management_ceiling_below_commitments") => "The monthly maximum cannot be lower than this month's published capacity and Agent holds. Close this review and choose a higher value.",
        Some("management_ceiling_usage_unsettled") => "Some Agent usage is still being reported, so the monthly maximum cannot be lowered yet. You can still raise it. Keep this device online, then try again.",
        Some(_) => "The current configuration could not be verified. Close this review, refresh, or recover your original saved request.",
    }
}

pub(super) struct ResourceEditPrompt {
    account: String,
    expires: Instant,
    receiver: Option<Receiver<Result<ResourceEditConsent, String>>>,
    consent: Option<ResourceEditConsent>,
    failure: Option<String>,
}
impl ResourceEditPrompt {
    pub(super) fn begin(consent: ResourceEditConsent) -> Self {
        let account = consent.account().to_owned();
        let (tx, receiver) = mpsc::sync_channel(1);
        crate::sliding_sync::spawn_async_task(async move {
            let result = consent.load().await;
            let _ = tx.try_send(result);
            SignalToUI::set_ui_signal();
        });
        Self {
            account,
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
    pub(super) fn details(&self) -> String {
        if let Some(consent) = &self.consent {
            return consent.details();
        }
        crate::i18n::tr(failure_text(self.failure.as_deref())).into()
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
        for english in [title, purpose, consequence, confirm] {
            assert_ne!(text_for(Language::Chinese, english), english, "{english}");
        }
        assert_eq!(text_for(Language::Chinese, title), "修改每月总上限");
        assert!(consequence.contains("running Agents stay as they are"));
        assert!(consequence.contains("per-request limit is changed separately"));
        let generic = failure_text(Some("management_unavailable"));
        for code in [
            "management_ceiling_amendment_not_available",
            "management_ceiling_clear_refused",
            "management_ceiling_unchanged",
            "management_ceiling_below_commitments",
            "management_ceiling_usage_unsettled",
        ] {
            let english = failure_text(Some(code));
            assert_ne!(english, generic, "{code} has its own reason");
            assert_ne!(text_for(Language::Chinese, english), english, "{code} is translated");
        }
        assert!(failure_text(Some("management_ceiling_below_commitments")).contains("this month's published capacity"));
        assert_eq!(
            text_for(Language::Chinese, failure_text(Some("management_ceiling_usage_unsettled"))),
            "部分 Agent 用量仍在上报中，暂时不能调低每月总上限，但仍可调高。请保持本设备在线，然后重试。"
        );
    }
}
