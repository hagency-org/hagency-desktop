//! Closed graph-history reads; no raw document or dispatch capability.
use super::*;
pub(super) const SERVICES: &[&str] = &[
    "hagency.graph_history.archives",
    "hagency.graph_history.list",
    "hagency.graph_history.get",
    "hagency.graph_history.node",
];
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

pub(super) async fn execute(
    service: &str,
    args: Value,
    manager: &NativeManagement,
) -> Result<Value, String> {
    #[cfg(feature = "resource-adoption")]
    {
        use ::hagency::native_owner::{
            GraphHistoryQuery, GraphHistoryDetailQuery, GraphHistoryNodeQuery,
        };
        match service {
            "hagency.graph_history.archives" => {
                let _: Empty = parse(args)?;
                value(manager.graph_history_archives().await.map_err(|e| e.code)?)
            }
            "hagency.graph_history.list" => {
                let q: GraphHistoryQuery = parse(args)?;
                q.validate().map_err(|_| "invalid_management_arguments")?;
                value(manager.graph_history_page(q).await.map_err(|e| e.code)?)
            }
            "hagency.graph_history.get" => {
                let q: GraphHistoryDetailQuery = parse(args)?;
                q.validate().map_err(|_| "invalid_management_arguments")?;
                value(manager.graph_history_detail(q).await.map_err(|e| e.code)?)
            }
            "hagency.graph_history.node" => {
                let q: GraphHistoryNodeQuery = parse(args)?;
                q.validate().map_err(|_| "invalid_management_arguments")?;
                value(manager.graph_history_node(q).await.map_err(|e| e.code)?)
            }
            _ => Err("invalid_management_arguments".into()),
        }
    }
    #[cfg(not(feature = "resource-adoption"))]
    {
        let _ = manager;
        if service != "hagency.graph_history.archives" {
            return Err("graph_history_unavailable".into());
        }
        let _: Empty = parse(args)?;
        Ok(json!({"supported":false,"archives":[],"readOnly":true,"identityMapped":false}))
    }
}
