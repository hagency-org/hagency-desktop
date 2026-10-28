//! Typed resource-owner workspace services. No caller-supplied identity or path.
use super::{parse, Value};
use ::hagency::native_owner::{ClearWorkspaceDirty, WorkspaceListQuery, WorkspaceQuery};

pub(super) const LIST: &str = "hagency.workspaces.list";
pub(super) const GET: &str = "hagency.workspaces.get";
pub(super) const CLEAR: &str = "hagency.workspaces.clear_dirty";
pub(super) enum Call {
    List(WorkspaceListQuery),
    Get(WorkspaceQuery),
    Clear(ClearWorkspaceDirty),
}
pub(super) fn call(service: &str, args: Value) -> Result<Option<Call>, String> {
    let invalid = |_| "invalid_management_arguments".to_owned();
    Ok(Some(match service {
        LIST => {
            let query: WorkspaceListQuery = parse(args)?;
            query.validate().map_err(invalid)?;
            Call::List(query)
        }
        GET => {
            let query: WorkspaceQuery = parse(args)?;
            query.validate().map_err(invalid)?;
            Call::Get(query)
        }
        CLEAR => {
            let input: ClearWorkspaceDirty = parse(args)?;
            input.validate().map_err(invalid)?;
            Call::Clear(input)
        }
        _ => return Ok(None),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn workspace_services_reject_forged_authority_and_require_declared_capabilities() {
        for (service, args) in [
            (LIST, json!({"resourceId":"resource"})),
            (GET, json!({"resourceId":"resource","workspaceId":"workspace"})),
            (CLEAR, json!({"resourceId":"resource","workspaceId":"workspace","commandId":"original","expectedRevision":"a".repeat(64)})),
        ] {
            assert!(call(service, args.clone()).unwrap().is_some());
            for field in ["owner", "profile", "token", "path", "server", "force", "dirty", "quarantined"] {
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
    fn workspace_release_preserves_review_and_rejects_arbitrary_paths() {
        let base = json!({"resourceId":"resource","workspaceId":"workspace","commandId":"original","expectedRevision":"a".repeat(64)});
        let Some(Call::Clear(input)) = call(CLEAR, base.clone()).unwrap() else {panic!("expected release")};
        assert_eq!(input.command_id,"original");
        assert_eq!(input.expected_revision,"a".repeat(64));
        for (field,value) in [("workspaceId",json!("../private")),("expectedRevision",json!("stale")),("resourceId",json!("/tmp/owner"))] {
            let mut invalid=base.clone();invalid[field]=value;
            assert!(call(CLEAR,invalid).is_err());
        }
    }
}

