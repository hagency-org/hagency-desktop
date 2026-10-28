//! Exact assignment Run/Stop settings. The native layer owns authority,
//! durable commands and the resource owner's local Stop fence.
use super::{parse, ManagementCommand, ManagementPermission, Value};
use ::hagency::native_owner::AgentRunMode;
use serde::Deserialize;
pub(super) const READINESS: &str = "hagency.agents.control.readiness";
pub(super) const SET: &str = "hagency.agents.control.set";
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Selection {
    assignment_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Set {
    command_id: String,
    assignment_id: String,
    expected_revision: i64,
    mode: AgentRunMode,
}
fn key(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("invalid_management_arguments".into());
    }
    Ok(())
}
pub(super) fn permissions(service: &str) -> &'static [ManagementPermission] {
    match service {
        READINESS => &[ManagementPermission::ReadAgentRunControl],
        SET => &[ManagementPermission::ChangeAgentRunControl],
        _ => &[],
    }
}
pub(super) fn command(service: &str, args: Value) -> Result<Option<ManagementCommand>, String> {
    Ok(Some(match service {
        READINESS => {
            let a: Selection = parse(args)?;
            key(&a.assignment_id)?;
            ManagementCommand::AgentRunReadiness {
                assignment_id: a.assignment_id,
            }
        }
        SET => {
            let a: Set = parse(args)?;
            key(&a.assignment_id)?;
            key(&a.command_id)?;
            if !(1..=9_007_199_254_740_991).contains(&a.expected_revision) {
                return Err("invalid_management_arguments".into());
            }
            ManagementCommand::AgentRunControl {
                command_id: a.command_id,
                assignment_id: a.assignment_id,
                expected_revision: a.expected_revision,
                mode: a.mode,
            }
        }
        _ => return Ok(None),
    }))
}
pub(super) fn display_reply(service: &str, mut value: Value) -> Result<Value, String> {
    #[cfg(not(feature = "shared-intake"))]
    if service == "hagency.session" {
        if let Some(features) = value.get_mut("features").and_then(Value::as_array_mut) {
            features.retain(|feature| feature.as_str() != Some("shared-run-control-v1"));
        }
        return Ok(value);
    }
    if service != READINESS && service != SET {
        return Ok(value);
    }
    let at = value["observedAtMs"]
        .as_i64()
        .filter(|n| (1..=9_007_199_254_740_991).contains(n))
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .ok_or_else(|| "management_agent_control_invalid".to_owned())?;
    value["observedAtUtc"] = Value::String(at.format("%Y-%m-%d %H:%M:%S UTC").to_string());
    Ok(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[cfg(not(feature = "shared-intake"))]
    #[test]
    fn agent_run_control_disabled_build_does_not_offer_unavailable_controls() {
        let reply = display_reply(
            "hagency.session",
            json!({"features":["agent-admission-v1","shared-run-control-v1"]}),
        )
        .unwrap();
        assert_eq!(reply["features"], json!(["agent-admission-v1"]));
    }
    #[test]
    fn agent_run_control_host_preserves_original_command_and_cas() {
        let command = command(SET,json!({"commandId":"original_command","assignmentId":"original_assignment","expectedRevision":7,"mode":"paused"})).unwrap().unwrap();
        let value = serde_json::to_value(command).unwrap();
        assert_eq!(value["command_id"], "original_command");
        assert_eq!(value["expected_revision"], 7);
        assert_eq!(value["mode"], "paused");
    }
    #[test]
    fn agent_run_control_host_rejects_authority_paths_new_modes_and_inexact_revisions() {
        let input = json!({"commandId":"original","assignmentId":"assignment","expectedRevision":7,"mode":"running"});
        for extra in ["owner", "token", "url", "deviceId", "generation", "canRun"] {
            let mut invalid = input.clone();
            invalid[extra] = json!("forged");
            assert!(command(SET, invalid).is_err());
        }
        for revision in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!(9_007_199_254_740_992_i64),
        ] {
            let mut invalid = input.clone();
            invalid["expectedRevision"] = revision;
            assert!(command(SET, invalid).is_err());
        }
        let mut invalid = input;
        invalid["mode"] = json!("manual");
        assert!(command(SET, invalid).is_err());
        assert!(command(READINESS, json!({"assignmentId":"../control"})).is_err());
    }
    #[test]
    fn agent_run_control_host_read_and_write_have_distinct_permissions() {
        assert_eq!(
            permissions(READINESS),
            &[ManagementPermission::ReadAgentRunControl]
        );
        assert_eq!(
            permissions(SET),
            &[ManagementPermission::ChangeAgentRunControl]
        );
        assert!(command(READINESS, json!({"assignmentId":"a","mode":"running"})).is_err());
    }
    #[test]
    fn agent_run_control_host_formats_observation_without_inventing_online_state() {
        let reply = display_reply(
            READINESS,
            json!({"observedAtMs":1000,"readiness":{"control":{"mode":"paused"}}}),
        )
        .unwrap();
        assert_eq!(reply["observedAtUtc"], "1970-01-01 00:00:01 UTC");
        assert!(reply.get("online").is_none());
        assert!(display_reply(READINESS, json!({"observedAtMs":null})).is_err());
    }
}
