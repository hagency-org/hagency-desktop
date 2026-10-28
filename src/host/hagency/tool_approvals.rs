//! Typed, consented tool-status and saved-rule services; no approval verdicts.
use super::{parse, Value};
use ::hagency::native_owner::{RevokeToolGrant, ToolApprovalLookup, ToolApprovalQuery, ToolGrantQuery};

pub(super) const LIST: &str = "hagency.tool_approvals.list";
pub(super) const GET: &str = "hagency.tool_approvals.get";
pub(crate) const OPEN: &str = "hagency.tool_approvals.open";
pub(super) const GRANTS: &str = "hagency.tool_grants.list";
pub(super) const REVOKE: &str = "hagency.tool_grants.revoke";
pub(super) enum Call {
    List(ToolApprovalQuery),
    Get(ToolApprovalLookup),
    Open(ToolApprovalLookup),
    Grants(ToolGrantQuery),
    Revoke(RevokeToolGrant),
}
pub(super) fn call(service: &str, args: Value) -> Result<Option<Call>, String> {
    let invalid = |_| "invalid_management_arguments".to_owned();
    Ok(Some(match service {
        LIST => {
            let q: ToolApprovalQuery = parse(args)?;
            q.validate().map_err(invalid)?;
            Call::List(q)
        }
        GET => {
            let q: ToolApprovalLookup = parse(args)?;
            q.validate().map_err(invalid)?;
            Call::Get(q)
        }
        OPEN => {
            let q: ToolApprovalLookup = parse(args)?;
            q.validate().map_err(invalid)?;
            Call::Open(q)
        }
        GRANTS => {
            let q: ToolGrantQuery = parse(args)?;
            q.validate().map_err(invalid)?;
            Call::Grants(q)
        }
        REVOKE => {
            let q: RevokeToolGrant = parse(args)?;
            q.validate().map_err(invalid)?;
            Call::Revoke(q)
        }
        _ => return Ok(None),
    }))
}

/// Host-resolved navigation only. The native approval inbox revalidates the
/// current account's Matrix markers; opening it never submits a decision.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolApprovalTarget {
    v: u8,
    pub account: ruma::OwnedUserId,
    resource_id: String,
    approval_id: String,
    pub project_room_id: ruma::OwnedRoomId,
}
impl ToolApprovalTarget {
    pub(crate) fn from_reply(value: &Value, account: &str) -> Result<Self, String> {
        let target: Self = serde_json::from_value(value.clone())
            .map_err(|_| "management_invalid_result".to_owned())?;
        if target.v != 1 || target.account.as_str() != account {
            return Err("management_scope_changed".into());
        }
        ToolApprovalLookup {
            resource_id: target.resource_id.clone(),
            approval_id: target.approval_id.clone(),
        }
        .validate()
        .map_err(|_| "management_invalid_result".to_owned())?;
        Ok(target)
    }
}
pub(super) fn destination(
    snapshot: ::hagency::native_owner::ToolApprovalSnapshot,
    account: &str,
) -> Result<Value, String> {
    let room = snapshot
        .approval
        .project_room_id
        .ok_or("tool_approval_room_unavailable")?;
    let value = serde_json::json!({"v":1,"account":account,"resourceId":snapshot.resource_id,
        "approvalId":snapshot.approval.id,"projectRoomId":room});
    ToolApprovalTarget::from_reply(&value, account)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn tool_services_reject_forged_authority_and_approval_verdicts() {
        for (service, args) in [
            (
                LIST,
                json!({"resourceId":"resource","engagementId":"agent"}),
            ),
            (
                GET,
                json!({"resourceId":"resource","approvalId":"approval"}),
            ),
            (
                OPEN,
                json!({"resourceId":"resource","approvalId":"approval"}),
            ),
            (
                GRANTS,
                json!({"resourceId":"resource","engagementId":"agent"}),
            ),
            (
                REVOKE,
                json!({"resourceId":"resource","engagementId":"agent","grantId":"grant","commandId":"original","expectedRevision":"a".repeat(64)}),
            ),
        ] {
            assert!(call(service, args.clone()).unwrap().is_some());
            for field in [
                "owner",
                "profile",
                "token",
                "server",
                "path",
                "choice",
                "allow",
                "params",
                "roomId",
                "projectRoomId",
                "account",
            ] {
                let mut forged = args.clone();
                forged[field] = json!("forged");
                assert!(call(service, forged).is_err(), "{service}: {field}");
            }
            assert!(octosense_app_contract::hagency::SERVICES.contains(&service));
            assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&service));
            assert!(octosense_app_contract::hagency::words(service).is_some());
        }
    }
    #[test]
    fn tool_approval_destination_is_closed_and_account_bound() {
        let value = json!({"v":1,"account":"@owner:matrix.test","resourceId":"resource",
            "approvalId":"approval","projectRoomId":"!project:matrix.test"});
        let target = ToolApprovalTarget::from_reply(&value, "@owner:matrix.test").unwrap();
        assert_eq!(target.project_room_id.as_str(), "!project:matrix.test");
        assert!(ToolApprovalTarget::from_reply(&value, "@other:matrix.test").is_err());
        for field in ["url", "choice", "roomId", "token"] {
            let mut forged = value.clone();
            forged[field] = json!("forged");
            assert!(ToolApprovalTarget::from_reply(&forged, "@owner:matrix.test").is_err());
        }
        for (field, replacement) in [
            ("v", json!(2)),
            ("approvalId", json!("../private")),
            ("projectRoomId", json!("https://matrix.test")),
        ] {
            let mut malformed = value.clone();
            malformed[field] = replacement;
            assert!(ToolApprovalTarget::from_reply(&malformed, "@owner:matrix.test").is_err());
        }
    }
    #[test]
    fn tool_approval_destination_uses_snapshot_room_and_refuses_missing_room() {
        let snapshot = json!({"resourceId":"resource", "observedAtMs":12001,
            "approval":{"id":"approval","state":"decided","choice":"deny","reusableScope":false,
                "expiresAt":12000,"engagementId":"engagement","projectRoomId":"!current:matrix.test"},
            "subject":{"engagementId":"engagement","agent":"Writer","projectId":"project","projectName":"Project Alpha"}});
        let value = destination(
            serde_json::from_value(snapshot.clone()).unwrap(),
            "@owner:matrix.test",
        )
        .unwrap();
        assert_eq!(value["projectRoomId"], "!current:matrix.test");
        assert_eq!(value.as_object().unwrap().len(), 5);
        let mut unavailable = snapshot;
        unavailable["approval"]["projectRoomId"] = Value::Null;
        assert_eq!(
            destination(
                serde_json::from_value(unavailable).unwrap(),
                "@owner:matrix.test"
            )
            .unwrap_err(),
            "tool_approval_room_unavailable"
        );
    }
    #[test]
    fn tool_grant_reads_and_revoke_require_exact_agent_scope_and_review() {
        assert!(call(GRANTS, json!({"resourceId":"resource"})).is_err());
        assert!(call(LIST, json!({"resourceId":"../private"})).is_err());
        assert!(call(REVOKE,json!({"resourceId":"resource","engagementId":"agent","grantId":"grant","commandId":"original","expectedRevision":"old"})).is_err());
        let Some(Call::Revoke(input))=call(REVOKE,json!({"resourceId":"resource","engagementId":"agent","grantId":"grant","commandId":"original","expectedRevision":"a".repeat(64)})).unwrap() else {panic!("expected revoke")};
        assert_eq!(input.command_id, "original");
        assert_eq!(input.grant_id, "grant");
        assert_eq!(input.expected_revision, "a".repeat(64));
    }
}
