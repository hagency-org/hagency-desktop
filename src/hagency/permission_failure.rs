//! Permission reads retain transport status; raw server text is never UI copy.
#[derive(Clone, Debug)]
pub(crate) struct PermissionFailure {
    pub status: Option<u16>,
    pub code: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Recovery {
    SignIn,
    Access,
    Service,
    Unsupported,
    Transient,
    Retired,
    Manual,
}
impl From<String> for PermissionFailure {
    fn from(code: String) -> Self {
        Self { status: None, code }
    }
}
impl From<&str> for PermissionFailure {
    fn from(code: &str) -> Self {
        code.to_owned().into()
    }
}
impl From<::hagency::native_owner::NativeError> for PermissionFailure {
    fn from(error: ::hagency::native_owner::NativeError) -> Self {
        Self {
            status: Some(error.status),
            code: error.code,
        }
    }
}
impl PermissionFailure {
    pub fn recovery(&self) -> Recovery {
        use Recovery::*;
        // Specific local/session codes precede generic HTTP status. In particular,
        // an expired management handle is not an expired Matrix login.
        match self.code.as_str() {
            "matrix_account_changed" | "matrix_account_changing" | "matrix_account_mismatch" => {
                return Retired;
            }
            "matrix_sign_in_required"
            | "matrix_authorization_required"
            | "owner_authorization_required" => return SignIn,
            "owner_daemon_unavailable"
            | "daemon_bridge_unavailable"
            | "owner_daemon_hosting_required"
            | "owner_daemon_management_required"
            | "owner_source_adoption_required"
            | "native_owner_restart_required"
            | "native_resume_revoked" => return Service,
            "owner_daemon_platform_unsupported" | "shared_execution_unsupported" => {
                return Unsupported;
            }
            "matrix_authorization_unavailable"
            | "permission_query_timeout"
            | "management_scope_changed"
            | "daemon_bridge_lease_expired" => return Transient,
            _ => {}
        }
        match self.status {
            Some(401) => SignIn,
            Some(403) => Access,
            Some(404 | 405 | 501) => Unsupported,
            Some(408 | 429 | 500 | 502..=599) => Transient,
            _ => Manual,
        }
    }
    pub fn message(&self) -> &'static str {
        match self.recovery() {
            Recovery::SignIn => "Sign in again to check permission status.",
            Recovery::Access => {
                "Permission status is not available to this account. Ask the resource owner to check access."
            }
            Recovery::Service => {
                "The local Agent service is unavailable. Open My Agents to check its setup and connection."
            }
            Recovery::Unsupported => {
                "This server or local service does not support permission status. Check its version in My Agents."
            }
            Recovery::Transient => {
                "Permission status is temporarily unavailable. Check your connection; status will retry automatically."
            }
            Recovery::Retired => {
                "The account or session changed. Reopen this conversation after signing in."
            }
            Recovery::Manual => {
                "Permission status could not be confirmed. Refresh status or check My Agents."
            }
        }
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;
    #[test]
    fn dynamic_recovery_messages_have_both_catalog_translations() {
        let en: serde_json::Value =
            serde_json::from_str(include_str!("../../resources/i18n/en.json")).unwrap();
        let zh: serde_json::Value =
            serde_json::from_str(include_str!("../../resources/i18n/zh-CN.json")).unwrap();
        for (status, code) in [
            (401, "error"),
            (403, "error"),
            (404, "error"),
            (503, "error"),
            (503, "owner_daemon_unavailable"),
            (409, "matrix_account_changed"),
            (400, "error"),
        ] {
            let failure = PermissionFailure {
                status: Some(status),
                code: code.into(),
            };
            let key = failure.message();
            assert_eq!(en[key], key);
            assert!(zh[key].as_str().is_some_and(|v| !v.is_empty() && v != key));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_status_and_stable_local_codes_choose_the_recovery_path() {
        for (status, expected) in [
            (401, Recovery::SignIn),
            (403, Recovery::Access),
            (404, Recovery::Unsupported),
            (405, Recovery::Unsupported),
            (501, Recovery::Unsupported),
            (408, Recovery::Transient),
            (429, Recovery::Transient),
            (500, Recovery::Transient),
            (503, Recovery::Transient),
            (400, Recovery::Manual),
            (409, Recovery::Manual),
        ] {
            let failure = PermissionFailure::from(::hagency::native_owner::NativeError {
                status,
                code: "server_error".into(),
            });
            assert_eq!(failure.status, Some(status));
            assert_eq!(failure.recovery(), expected, "{status}");
        }
        for (code, expected) in [
            ("owner_daemon_unavailable", Recovery::Service),
            ("matrix_authorization_required", Recovery::SignIn),
            ("matrix_authorization_unavailable", Recovery::Transient),
            ("permission_query_timeout", Recovery::Transient),
            ("matrix_account_changed", Recovery::Retired),
            ("management_scope_changed", Recovery::Transient),
        ] {
            assert_eq!(PermissionFailure::from(code).recovery(), expected, "{code}");
        }
        // A temporary native handle expiring must not send the user to Matrix login.
        assert_eq!(
            PermissionFailure {
                status: Some(401),
                code: "management_scope_changed".into()
            }
            .recovery(),
            Recovery::Transient
        );
        assert_eq!(
            PermissionFailure {
                status: Some(503),
                code: "owner_daemon_unavailable".into()
            }
            .recovery(),
            Recovery::Service
        );
    }
    #[test]
    fn arbitrary_server_errors_are_not_rendered_or_automatically_retried() {
        let failure =
            PermissionFailure::from("https://server.invalid/?access_token=secret\nserver trace");
        assert_eq!(failure.recovery(), Recovery::Manual);
        assert!(!failure.message().contains("secret"));
        assert!(!failure.message().contains("server.invalid"));
    }
}
