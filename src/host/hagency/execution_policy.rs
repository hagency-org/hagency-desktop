//! Typed resource-owner policy services. No caller-supplied identity or path.
use super::{parse, Value};
use ::hagency::native_owner::{EditExecutionPolicy, ExecutionPolicyListQuery, ExecutionPolicyQuery};

pub(super) const LIST: &str = "hagency.execution_policy.list";
pub(super) const GET: &str = "hagency.execution_policy.get";
pub(super) const EDIT: &str = "hagency.execution_policy.edit";
pub(super) enum Call {
    List(ExecutionPolicyListQuery),
    Get(ExecutionPolicyQuery),
    Edit(EditExecutionPolicy),
}
pub(super) fn call(service: &str, args: Value) -> Result<Option<Call>, String> {
    let invalid = |_| "invalid_management_arguments".to_owned();
    Ok(Some(match service {
        LIST => {
            let query: ExecutionPolicyListQuery = parse(args)?;
            query.validate().map_err(invalid)?;
            Call::List(query)
        }
        GET => {
            let query: ExecutionPolicyQuery = parse(args)?;
            query.validate().map_err(invalid)?;
            Call::Get(query)
        }
        EDIT => {
            let input: EditExecutionPolicy = parse(args)?;
            input.validate().map_err(invalid)?;
            Call::Edit(input)
        }
        _ => return Ok(None),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn execution_policy_services_require_exact_scope_and_explicit_capability() {
        for (service, args) in [
            (LIST, json!({"resourceId":"resource"})),
            (GET, json!({"resourceId":"resource","engagementId":"agent"})),
            (EDIT, json!({"resourceId":"resource","engagementId":"agent","commandId":"original","expectedRevision":"a".repeat(64),"executionPolicy":{"yolo":true}})),
        ] {
            assert!(call(service, args.clone()).unwrap().is_some());
            for field in ["owner", "profile", "token", "path", "server", "mayWrite", "approvalPolicy"] {
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
    fn execution_policy_edit_preserves_boolean_and_reviewed_revision() {
        let base = json!({"resourceId":"resource","engagementId":"agent","commandId":"original","expectedRevision":"a".repeat(64),"executionPolicy":{"yolo":false}});
        let Some(Call::Edit(input)) = call(EDIT, base.clone()).unwrap() else {panic!("expected edit")};
        assert!(!input.execution_policy.yolo);
        assert_eq!(input.command_id,"original");
        assert_eq!(input.expected_revision,"a".repeat(64));
        for (field,value) in [("executionPolicy",json!({"yolo":"true"})),("expectedRevision",json!("stale")),("engagementId",json!("../other"))] {
            let mut invalid=base.clone();invalid[field]=value;
            assert!(call(EDIT,invalid).is_err());
        }
    }
}
