//! Closed read projections for the original management console screens.
use super::{parse, value, ManagementPermission, NativeManagement, Value};
use ::hagency::native_owner::{WebOverviewQuery, WebAccountCapacityQuery, WebAlertsQuery, WebTerminalQuery};
pub(super) const WORKFORCE: &str = "hagency.web.workforce";
pub(super) const CAPABILITY: &str = "hagency.web.capability";
pub(super) const USAGE: &str = "hagency.web.usage";
pub(super) const ACCOUNTS: &str = "hagency.web.accounts";
pub(super) const ALERTS: &str = "hagency.web.alerts";
pub(super) const TERMINAL: &str = "hagency.web.terminal";
pub(super) fn permissions(service: &str) -> &'static [ManagementPermission] {
    use ManagementPermission::*;
    match service {
        WORKFORCE => &[ReadAgentRequests, ReadAgentBudget, ReadAllocations, ReadResources, ReadAccounts],
        CAPABILITY => &[ReadResources, ReadWebCatalog], USAGE => &[ReadResources],
        ACCOUNTS => &[ReadAccounts], ALERTS => &[ReadAlerts], TERMINAL => &[ReadAgentRequests, ReadResources, ReadAccounts],
        _ => &[],
    }
}
pub(super) enum Call {
    Workforce(WebOverviewQuery), Capability(WebOverviewQuery), Usage(WebOverviewQuery),
    Accounts(WebAccountCapacityQuery), Alerts(WebAlertsQuery), Terminal(WebTerminalQuery),
}
pub(super) fn call(service: &str, args: Value) -> Result<Option<Call>, String> {
    let invalid = |_| "invalid_management_arguments".to_owned();
    Ok(Some(match service {
        WORKFORCE | CAPABILITY | USAGE => {
            let query: WebOverviewQuery = parse(args)?;
            query.validate().map_err(invalid)?;
            match service { WORKFORCE => Call::Workforce(query), CAPABILITY => Call::Capability(query), _ => Call::Usage(query) }
        }
        ACCOUNTS => {let q: WebAccountCapacityQuery = parse(args)?; q.validate().map_err(invalid)?; Call::Accounts(q)}
        ALERTS => {let q: WebAlertsQuery = parse(args)?; q.validate().map_err(|_| "invalid_management_arguments".to_owned())?; Call::Alerts(q)}
        TERMINAL => {let q: WebTerminalQuery = parse(args)?; q.validate().map_err(invalid)?; Call::Terminal(q)}
        _ => return Ok(None),
    }))
}
impl Call {
    pub(super) fn mutates(&self) -> bool { false }
    pub(super) async fn execute(self, manager: &NativeManagement) -> Result<Value, String> {
        match self {
            Self::Workforce(q) => manager.web_workforce(q).await.map_err(|e| e.code),
            Self::Capability(q) => manager.web_capability(q).await.map_err(|e| e.code),
            Self::Usage(q) => manager.web_usage(q).await.map_err(|e| e.code),
            Self::Accounts(q) => manager.web_account_capacity(q).await.map_err(|e| e.code),
            Self::Alerts(q) => value(manager.web_alerts(q).await.map_err(|e| e.code)?),
            Self::Terminal(q) => manager.web_terminal(q).await.map_err(|e| e.code),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn overview_queries_reject_identity_and_execution_arguments() {
        for service in [WORKFORCE, CAPABILITY, USAGE, ACCOUNTS, ALERTS] {
            assert!(call(service,json!({})).is_ok());
            for key in ["owner", "profile", "path", "command", "url", "credential"] {
                let mut args = json!({}); args[key] = json!("injected");
                assert!(call(service,args).is_err());
            }
            assert!(octosense_app_contract::hagency::SERVICES.contains(&service));
            assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&service));
        }
        assert!(call(TERMINAL,json!({"id":"agent_request","command":"restart"})).is_err());
        assert!(call(USAGE,json!({"limit":17})).is_err());
        assert!(call(ACCOUNTS,json!({"resourceAfter":"resource_a"})).is_err());
    }
}
