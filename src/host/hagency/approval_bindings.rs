//! Resource-scoped approval rooms; the host supplies identity and permissions.
use super::{parse, Value};
use ::hagency::native_owner::{ResourceApprovalBindingQuery, RetireResourceApprovalBinding};

pub(super) const LIST: &str = "hagency.approval_bindings.list";
pub(super) const RETIRE: &str = "hagency.approval_bindings.retire";
pub(super) enum Call {
    List(ResourceApprovalBindingQuery),
    Retire(RetireResourceApprovalBinding),
}
pub(super) fn call(service: &str, args: Value) -> Result<Option<Call>, String> {
    let invalid = |_| "invalid_management_arguments".to_owned();
    Ok(Some(match service {
        LIST => {
            let query: ResourceApprovalBindingQuery = parse(args)?;
            query.validate().map_err(invalid)?;
            Call::List(query)
        }
        RETIRE => {
            let input: RetireResourceApprovalBinding = parse(args)?;
            input.validate().map_err(invalid)?;
            Call::Retire(input)
        }
        _ => return Ok(None),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn binding_filters_are_typed_and_cannot_supply_authority() {
        for project in ["project_one", "!project:matrix.test"] {
            let value = json!({"resourceId":"resource", "agent":"agent_one", "project":project});
            let Some(Call::List(query)) = call(LIST, value.clone()).unwrap() else { panic!("expected list"); };
            assert_eq!(query.agent, "agent_one");
            assert_eq!(query.project, project);
            for field in ["owner", "profile", "token", "server"] {
                let mut forged = value.clone();
                forged[field] = json!("forged");
                assert!(call(LIST, forged).is_err());
            }
        }
        for project in ["../private", "!missing-server", "https://matrix.test"] {
            assert!(call(LIST, json!({"resourceId":"resource", "project":project})).is_err());
        }
        assert!(call(LIST, json!({"resourceId":"resource", "agent":"../private"})).is_err());
    }

    #[test]
    fn binding_host_preserves_scope_revision_and_rejects_authority_inputs() {
        let input = json!({"commandId":"original", "resourceId":"resource",
            "engagementId":"engagement", "expectedRevision":"a".repeat(64)});
        assert!(matches!(call(RETIRE, input.clone()).unwrap(), Some(Call::Retire(_))));
        for field in ["owner", "actor", "profile", "roomId", "token", "path", "allow"] {
            let mut forged = input.clone();
            forged[field] = json!("forged");
            assert!(call(RETIRE, forged).is_err(), "{field}");
        }
        assert!(call(LIST, json!({"resourceId":"resource"})).is_ok());
        assert!(call(LIST, json!({"resourceId":"../private"})).is_err());
        let mut stale = input;
        stale["expectedRevision"] = json!(42);
        assert!(call(RETIRE, stale).is_err());
        for capability in [LIST, RETIRE] {
            assert!(octosense_app_contract::hagency::SERVICES.contains(&capability));
            assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&capability));
            assert!(octosense_app_contract::hagency::words(capability).is_some());
        }
    }
}
