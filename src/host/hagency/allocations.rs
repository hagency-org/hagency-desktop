//! Human allocation review/decision only. Native execution is never a grant here.
use super::{ManagementCommand, ManagementPermission, Value, parse};
use serde::Deserialize;
pub(super) const LIST: &str = "hagency.allocations.list";
pub(super) const GET: &str = "hagency.allocations.get";
pub(super) const DECIDE: &str = "hagency.allocations.decide";
pub(super) const RELEASE: &str = "hagency.allocations.release";
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct List {
    project_id: String,
    after: Option<String>,
    limit: Option<u16>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decide {
    command_id: String,
    request_id: String,
    expected_revision: i64,
    approve: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Release {
    command_id: String,
    allocation_id: String,
    expected_revision: i64,
}
pub(super) fn permissions(service: &str) -> &'static [ManagementPermission] {
    use ManagementPermission::*;
    match service {
        LIST | GET => &[ReadAllocations],
        DECIDE => &[DecideAllocations],
        RELEASE => &[ReleaseAllocations],
        _ => &[],
    }
}
pub(super) fn command(service: &str, args: Value) -> Result<Option<ManagementCommand>, String> {
    Ok(Some(match service {
        LIST => {
            let a: List = parse(args)?;
            if a.limit.is_some_and(|n| !(1..=16).contains(&n)) {
                return Err("invalid_management_arguments".into());
            }
            ManagementCommand::Allocations {
                project_id: a.project_id,
                after: a.after,
                limit: a.limit,
            }
        }
        GET => {
            let a: super::Id = parse(args)?;
            ManagementCommand::Allocation { id: a.id }
        }
        DECIDE => {
            let a: Decide = parse(args)?;
            ManagementCommand::AllocationDecide {
                command_id: a.command_id,
                request_id: a.request_id,
                expected_revision: a.expected_revision,
                approve: a.approve,
            }
        }
        RELEASE => {
            let a: Release = parse(args)?;
            ManagementCommand::AllocationRelease {
                command_id: a.command_id,
                allocation_id: a.allocation_id,
                expected_revision: a.expected_revision,
            }
        }
        _ => return Ok(None),
    }))
}
pub(super) fn display_reply(service: &str, mut value: Value) -> Result<Value, String> {
    if service != LIST && service != GET {
        return Ok(value);
    }
    fn label(value: &Value) -> Result<Value, String> {
        let at = value
            .as_i64()
            .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
            .ok_or_else(|| "management_allocation_reply_invalid".to_owned())?;
        Ok(Value::String(
            at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        ))
    }
    fn detail(d: &mut Value) -> Result<(), String> {
        d["allocation"]["expiresAtUtc"] = label(&d["allocation"]["expiresAtMs"])?;
        for (field, name) in [
            ("periodStartMs", "periodStartUtc"),
            ("periodEndMs", "periodEndUtc"),
            ("delegationExpiresAtMs", "delegationExpiresAtUtc"),
        ] {
            d["grant"][name] = label(&d["grant"][field])?;
        }
        Ok(())
    }
    value["observedAtUtc"] = label(&value["observedAtMs"])?;
    if service == GET {
        detail(&mut value["detail"])?;
    } else {
        for d in value["allocations"]["items"]
            .as_array_mut()
            .ok_or("management_allocation_reply_invalid")?
        {
            detail(d)?;
        }
    }
    Ok(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn human_allocation_services_have_no_executor_grant_or_forged_authority() {
        assert_eq!(permissions(LIST), &[ManagementPermission::ReadAllocations]);
        assert_eq!(
            permissions(DECIDE),
            &[ManagementPermission::DecideAllocations]
        );
        assert_eq!(
            permissions(RELEASE),
            &[ManagementPermission::ReleaseAllocations]
        );
        assert!(matches!(command(DECIDE,json!({"commandId":"original","requestId":"request","expectedRevision":3,"approve":true})).unwrap(),Some(ManagementCommand::AllocationDecide{expected_revision:3,..})));
        for extra in [
            "token",
            "grantReceipt",
            "ownerMxid",
            "resourceId",
            "periodKey",
            "requestedTokens",
            "execute",
            "url",
        ] {
            let mut value = json!({"commandId":"original","requestId":"request","expectedRevision":3,"approve":true});
            value[extra] = json!("injected");
            assert!(command(DECIDE, value).is_err());
        }
        for limit in [0, 17] {
            assert!(command(LIST, json!({"projectId":"p","limit":limit})).is_err());
        }
        let view=display_reply(GET,json!({"observedAtMs":1000,"detail":{"allocation":{"expiresAtMs":2000},"grant":{"periodStartMs":1000,"periodEndMs":2000,"delegationExpiresAtMs":3000,"nativeAvailableTokens":null}}})).unwrap();
        assert_eq!(
            view["detail"]["grant"]["periodStartUtc"],
            "1970-01-01 00:00:01 UTC"
        );
        assert!(view["detail"]["grant"]["nativeAvailableTokens"].is_null());
    }
}
