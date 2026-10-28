//! Human top-up request/decision services. No native executor receipt or Run authority.
use super::{parse, Id, ManagementCommand, ManagementPermission, Value};
use serde::Deserialize;
pub(super) const GET: &str = "hagency.agents.budget.get";
pub(super) const LIST: &str = "hagency.agents.topups.list";
pub(super) const TOP_UP: &str = "hagency.agents.topups.get";
pub(super) const REQUEST: &str = "hagency.agents.topups.request";
pub(super) const DECIDE: &str = "hagency.agents.topups.decide";
pub(super) const CANCEL: &str = "hagency.agents.topups.cancel";
const MAX: i64 = 9_007_199_254_740_991;
fn key(s: &str) -> Result<(), String> {
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    {
        Err("invalid_management_arguments".into())
    } else {
        Ok(())
    }
}
fn positive(n: i64) -> Result<(), String> {
    if (1..=MAX).contains(&n) {
        Ok(())
    } else {
        Err("invalid_management_arguments".into())
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct List {
    id: String,
    after: Option<String>,
    limit: Option<u16>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    #[serde(default)]
    owner_funded: bool,
    #[serde(default)]
    approve_immediately: bool,
    command_id: String,
    request_id: String,
    expected_request_revision: i64,
    expected_assignment_generation: i64,
    expected_budget_revision: i64,
    expected_budget_digest: String,
    add_tokens: i64,
    purpose: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decide {
    command_id: String,
    top_up_id: String,
    expected_revision: i64,
    expected_authority_revision: i64,
    approve: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cancel {
    command_id: String,
    top_up_id: String,
    expected_revision: i64,
}
pub(super) fn permissions(service: &str) -> &'static [ManagementPermission] {
    use ManagementPermission::*;
    match service {
        GET | LIST | TOP_UP => &[ReadAgentBudget],
        REQUEST => &[RequestAgentTopUps],
        DECIDE => &[DecideAgentTopUps],
        CANCEL => &[CancelAgentTopUps],
        _ => &[],
    }
}
pub(super) fn get_id(service: &str, args: Value) -> Result<Option<String>, String> {
    if service != GET {
        return Ok(None);
    }
    let a: Id = parse(args)?;
    key(&a.id)?;
    Ok(Some(a.id))
}
pub(super) fn command(service: &str, args: Value) -> Result<Option<ManagementCommand>, String> {
    Ok(Some(match service {
        LIST => {
            let a: List = parse(args)?;
            key(&a.id)?;
            if let Some(s) = &a.after {
                key(s)?;
            }
            if a.limit.is_some_and(|n| !(1..=16).contains(&n)) {
                return Err("invalid_management_arguments".into());
            }
            ManagementCommand::AgentTopUps {
                id: a.id,
                after: a.after,
                limit: a.limit,
            }
        }
        TOP_UP => {
            let a: Id = parse(args)?;
            key(&a.id)?;
            ManagementCommand::AgentTopUp { id: a.id }
        }
        REQUEST => {
            let a: Request = parse(args)?;
            key(&a.command_id)?;
            key(&a.request_id)?;
            for n in [
                a.expected_request_revision,
                a.expected_assignment_generation,
                a.expected_budget_revision,
                a.add_tokens,
            ] {
                positive(n)?;
            }
            if (a.approve_immediately && !a.owner_funded) || a.expected_budget_digest.len() != 64
                || !a
                    .expected_budget_digest
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                || a.purpose.trim().is_empty()
                || a.purpose.len() > 2048
                || a.purpose.chars().any(|c| c.is_control() && c != '\n')
            {
                return Err("invalid_management_arguments".into());
            }
            ManagementCommand::AgentTopUpRequest {
                owner_funded: a.owner_funded,
                approve_immediately: a.approve_immediately,
                command_id: a.command_id,
                request_id: a.request_id,
                expected_request_revision: a.expected_request_revision,
                expected_assignment_generation: a.expected_assignment_generation,
                expected_budget_revision: a.expected_budget_revision,
                expected_budget_digest: a.expected_budget_digest,
                add_tokens: a.add_tokens,
                purpose: a.purpose,
            }
        }
        DECIDE => {
            let a: Decide = parse(args)?;
            key(&a.command_id)?;
            key(&a.top_up_id)?;
            positive(a.expected_revision)?;
            positive(a.expected_authority_revision)?;
            ManagementCommand::AgentTopUpDecide {
                command_id: a.command_id,
                top_up_id: a.top_up_id,
                expected_revision: a.expected_revision,
                expected_authority_revision: a.expected_authority_revision,
                approve: a.approve,
            }
        }
        CANCEL => {
            let a: Cancel = parse(args)?;
            key(&a.command_id)?;
            key(&a.top_up_id)?;
            positive(a.expected_revision)?;
            ManagementCommand::AgentTopUpCancel {
                command_id: a.command_id,
                top_up_id: a.top_up_id,
                expected_revision: a.expected_revision,
            }
        }
        _ => return Ok(None),
    }))
}
fn time(v: &Value) -> Result<Value, String> {
    if v.is_null() {
        return Ok(Value::Null);
    }
    let t = v
        .as_i64()
        .filter(|n| (1..=MAX).contains(n))
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .ok_or("management_agent_budget_invalid")?;
    Ok(Value::String(t.format("%Y-%m-%d %H:%M:%S UTC").to_string()))
}
fn row(v: &mut Value) -> Result<(), String> {
    v["createdAtUtc"] = time(&v["createdAtMs"])?;
    v["expiresAtUtc"] = time(&v["expiresAtMs"])?;
    Ok(())
}
pub(super) fn display_reply(service: &str, mut v: Value) -> Result<Value, String> {
    if service == "hagency.intent.retry"
        && matches!(
            v["operation"].as_str(),
            Some("agent_top_up_request" | "agent_top_up_decide" | "agent_top_up_cancel")
        )
    {
        v["data"] = display_reply(TOP_UP, v["data"].take())?;
        return Ok(v);
    }
    if permissions(service).is_empty() {
        return Ok(v);
    }
    v["observedAtUtc"] = time(&v["observedAtMs"])?;
    if service == LIST {
        let rows = v["topUps"]["items"]
            .as_array_mut()
            .ok_or("management_agent_budget_invalid")?;
        for item in rows {
            row(item)?;
        }
    } else if service != GET {
        row(&mut v["topUp"])?;
    }
    Ok(v)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn request() -> Value {
        json!({"commandId":"original","requestId":"request","expectedRequestRevision":3,"expectedAssignmentGeneration":2,"expectedBudgetRevision":7,"expectedBudgetDigest":"a".repeat(64),"addTokens":1024,"purpose":"Continue"})
    }
    #[test]
    fn agent_topup_host_preserves_exact_command_and_all_cas() {
        let v = serde_json::to_value(command(REQUEST, request()).unwrap().unwrap()).unwrap();
        assert_eq!(v["command_id"], "original");
        assert_eq!(v["expected_budget_revision"], 7);
        assert_eq!(v["expected_assignment_generation"], 2);
        assert_eq!(v["expected_request_revision"], 3);
        assert_eq!(v["expected_budget_digest"], "a".repeat(64));
        assert_eq!(v["add_tokens"], 1024);
    }
    #[test]
    fn owner_direct_confirmation_is_explicit_and_requires_owner_funding() {
        let old=serde_json::to_value(command(REQUEST, request()).unwrap().unwrap()).unwrap();
        assert!(old.get("approve_immediately").is_none());
        let mut value=request();value["approveImmediately"]=json!(true);
        assert!(command(REQUEST,value.clone()).is_err());
        value["ownerFunded"]=json!(true);
        let parsed=serde_json::to_value(command(REQUEST,value).unwrap().unwrap()).unwrap();
        assert_eq!(parsed["approve_immediately"],true);
        assert_eq!(parsed["owner_funded"],true);
    }
    #[test]
    fn agent_topup_host_denies_paths_authority_and_inexact_amounts() {
        for key in [
            "url",
            "token",
            "owner",
            "provider",
            "canRequest",
            "executionSupported",
        ] {
            let mut v = request();
            v[key] = json!("forged");
            assert!(command(REQUEST, v).is_err());
        }
        for n in [json!(0), json!(-1), json!(1.5), json!(MAX + 1)] {
            let mut v = request();
            v["addTokens"] = n;
            assert!(command(REQUEST, v).is_err());
        }
        assert!(get_id(GET, json!({"id":"../other"})).is_err());
        assert!(command(LIST, json!({"id":"r","limit":17})).is_err());
    }
    #[test]
    fn agent_topup_host_permissions_are_operation_specific() {
        assert_eq!(permissions(GET), &[ManagementPermission::ReadAgentBudget]);
        assert_eq!(
            permissions(REQUEST),
            &[ManagementPermission::RequestAgentTopUps]
        );
        assert_eq!(
            permissions(DECIDE),
            &[ManagementPermission::DecideAgentTopUps]
        );
        assert_eq!(
            permissions(CANCEL),
            &[ManagementPermission::CancelAgentTopUps]
        );
        assert!(command(DECIDE,json!({"commandId":"c","topUpId":"t","expectedRevision":2,"expectedAuthorityRevision":3,"approve":true,"ownerApproved":true})).is_err());
    }
    #[test]
    fn agent_topup_host_unknown_budget_and_saved_recovery_stay_descriptive() {
        let v = display_reply(
            GET,
            json!({"supported":false,"budget":null,"observedAtMs":null}),
        )
        .unwrap();
        assert!(v["observedAtUtc"].is_null());
        let r=display_reply("hagency.intent.retry",json!({"operation":"agent_top_up_request","data":{"observedAtMs":1000,"topUp":{"createdAtMs":100,"expiresAtMs":200,"nativeState":"unknown"}}})).unwrap();
        assert_eq!(r["data"]["observedAtUtc"], "1970-01-01 00:00:01 UTC");
        assert_eq!(r["data"]["topUp"]["nativeState"], "unknown");
        assert!(r["data"].get("remainingTokens").is_none());
    }
}
