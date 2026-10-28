//! Same-account OAuth recovery. No NativeOwner construction, tokens, paths or
//! logout call: only a consumable native confirmation of the current SDK user.
use super::*;
pub(crate) const SERVICE: &str = "hagency.account.reauthenticate";
pub(crate) struct ReauthenticationConsent {
    host: Host,
    lease: Lease,
    expires: Instant,
}
impl ReauthenticationConsent {
    pub(crate) fn account(&self) -> &str {
        &self.lease.identity().account
    }
    pub(crate) fn expired(&self) -> bool {
        Instant::now() >= self.expires
    }
    fn authorize(&self, account: &str) -> Result<(), String> {
        if self.expired() {
            return Err("account_confirmation_expired".into());
        }
        if self.host.0.retired.load(Ordering::Acquire)
            || self.host.0.identity != *self.lease.identity()
            || !self
                .host
                .0
                .reauthentication_required
                .load(Ordering::Acquire)
        {
            return Err("management_scope_changed".into());
        }
        self.lease.authorize(account, SERVICE, None)
    }
    pub(crate) fn accept(self) -> Result<crate::accounts::AccountAction, String> {
        let account = self.host.check(&self.lease)?;
        self.authorize(&account)?;
        Ok(crate::accounts::AccountAction::Reauthenticate)
    }
}
impl Host {
    pub(crate) fn reauthentication_consent(
        &self,
        lease: Lease,
        args: Value,
    ) -> Result<ReauthenticationConsent, String> {
        let _: Empty = parse(args)?;
        let account = self.check(&lease)?;
        let consent = ReauthenticationConsent {
            host: self.clone(),
            lease,
            expires: Instant::now() + Duration::from_secs(60),
        };
        consent.authorize(&account)?;
        Ok(consent)
    }
    pub(super) fn note_reauthentication_error(&self, error: &str) {
        if matches!(error, "native_resume_missing" | "native_resume_revoked") {
            self.0
                .reauthentication_required
                .store(true, Ordering::Release);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Host, rinx_miniapp_core::SessionAuthority, Lease) {
        let authority = rinx_miniapp_core::SessionAuthority::default();
        let identity = InstanceId {
            app: octosense_app_contract::hagency::APP_ID.into(),
            account: "@owner:test".into(),
            room: None,
            generation: 7,
        };
        let lease = authority.issue(
            identity.clone(),
            [SERVICE.into()].into(),
            Default::default(),
            Instant::now() + Duration::from_secs(60),
        );
        let host = Host(Arc::new(Inner {
            digest: "a".repeat(64),
            identity,
            epoch: 0,
            retired: AtomicBool::new(false),
            reauthentication_required: AtomicBool::new(false),
            initialising: Default::default(),
            manager: Default::default(),
            grant_access: Default::default(),
        }));
        (host, authority, lease)
    }
    #[test]
    fn reauthentication_is_closed_scoped_and_requires_original_native_failure() {
        for bad in [
            json!({"account":"@other:test"}),
            json!({"token":"secret"}),
            json!({"origin":"https://other.test"}),
            json!({"approved":true}),
            json!(null),
        ] {
            assert!(parse::<Empty>(bad).is_err());
        }
        assert!(parse::<Empty>(json!({})).is_ok());
        let (host, _authority, lease) = fixture();
        let consent = ReauthenticationConsent {
            host: host.clone(),
            lease,
            expires: Instant::now() + Duration::from_secs(60),
        };
        assert!(consent.authorize("@owner:test").is_err());
        host.note_reauthentication_error("server_unavailable");
        assert!(consent.authorize("@owner:test").is_err());
        host.note_reauthentication_error("native_resume_missing");
        assert!(consent.authorize("@owner:test").is_ok());
        assert!(consent.authorize("@other:test").is_err());
        host.retire();
        assert!(consent.authorize("@owner:test").is_err());
    }
    #[test]
    fn reauthentication_old_package_and_expired_confirmation_stay_denied() {
        let (host, authority, lease) = fixture();
        host.note_reauthentication_error("native_resume_revoked");
        let mut consent = ReauthenticationConsent {
            host,
            lease,
            expires: Instant::now() + Duration::from_secs(60),
        };
        assert!(consent.authorize("@owner:test").is_ok());
        consent.expires = Instant::now();
        assert!(consent.authorize("@owner:test").is_err());
        consent.expires = Instant::now() + Duration::from_secs(60);
        authority.invalidate();
        assert!(consent.authorize("@owner:test").is_err());
    }
}
