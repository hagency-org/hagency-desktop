use super::{parse, Id, ManagementCommand, ManagementPermission, Value};
use ::hagency::native_owner::PoolAgentInput;
use serde::Deserialize;
pub(super) const LIST: &str = "hagency.pools.list";
pub(super) const AGENTS: &str = "hagency.pools.agents.list";
pub(super) const GET: &str = "hagency.pools.agents.get";
pub(super) const REQUEST: &str = "hagency.pools.agents.request";
pub(super) const DECIDE: &str = "hagency.pools.agents.decide";
pub(super) const STOP: &str = "hagency.pools.agents.stop";
pub(super) const ROOM: &str = "hagency.pools.agents.room";
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct List {
    after: Option<String>,
    capability: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Agents {
    after: Option<String>,
    pool_id: Option<String>,
    #[serde(default)]
    needs_my_action: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    command_id: String,
    pool_id: String,
    request: PoolAgentInput,
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
struct Stop {
    command_id: String,
    request_id: String,
    expected_revision: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Room {
    pub command_id: String,
    pub name: String,
}
pub(super) fn permissions(service: &str) -> &'static [ManagementPermission] {
    use ManagementPermission::*;
    match service {
        LIST => &[ReadCatalog],
        AGENTS | GET => &[ReadAgentRequests],
        REQUEST | ROOM => &[RequestAgents],
        DECIDE => &[DecideAgentRequests],
        STOP => &[CancelAgentRequests],
        _ => &[],
    }
}
pub(super) fn command(service: &str, args: Value) -> Result<Option<ManagementCommand>, String> {
    Ok(Some(match service {
        LIST => {
            let a: List = parse(args)?;
            ManagementCommand::Pools {
                after: a.after,
                capability: a.capability,
            }
        }
        AGENTS => {
            let a: Agents = parse(args)?;
            ManagementCommand::PoolAgents {
                after: a.after,
                pool_id: a.pool_id,
                needs_my_action: a.needs_my_action,
            }
        }
        GET => {
            let a: Id = parse(args)?;
            ManagementCommand::PoolAgent { id: a.id }
        }
        REQUEST => {
            let a: Request = parse(args)?;
            a.request.validate().map_err(|e| e.code)?;
            ManagementCommand::PoolAgentRequest {
                command_id: a.command_id,
                pool_id: a.pool_id,
                request: a.request,
            }
        }
        DECIDE => {
            let a: Decide = parse(args)?;
            ManagementCommand::PoolAgentDecide {
                command_id: a.command_id,
                request_id: a.request_id,
                expected_revision: a.expected_revision,
                approve: a.approve,
            }
        }
        STOP => {
            let a: Stop = parse(args)?;
            ManagementCommand::PoolAgentStop {
                command_id: a.command_id,
                request_id: a.request_id,
                expected_revision: a.expected_revision,
            }
        }
        _ => return Ok(None),
    }))
}
