//! Load canonical readback before enabling the existing native consent footer.
use super::*;
use crate::host::hagency::resource_edit::ResourceEditConsent;

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
    pub(super) fn details(&self) -> String {
        if let Some(consent) = &self.consent {
            return consent.details();
        }
        crate::i18n::tr(match self.failure.as_deref() {
            None => "Reading current resource configuration…",
            Some("management_resource_edit_changed") => "The resource changed. Close this review and refresh its details. Saved requests remain available for recovery.",
            Some("management_resource_edit_not_available") => "This resource cannot be edited locally. Close this review and refresh to see the current reason.",
            Some(_) => "The current configuration could not be verified. Close this review, refresh, or recover your original saved request.",
        }).into()
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
