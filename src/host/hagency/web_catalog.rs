//! Original web catalogue controls, bound to this native Matrix profile.
use super::{parse, value, ManagementPermission, NativeManagement, Value};
use ::hagency::native_owner::{WebCatalogPrepare, WebCatalogQuery};
use serde::Deserialize;

pub(super) const GET: &str = "hagency.web_catalog.get";
pub(super) const OFFERS: &str = "hagency.web_catalog.offers";
pub(super) const TRUST: &str = "hagency.web_catalog.trust";
pub(super) const PREPARE: &str = "hagency.web_catalog.prepare";
pub(super) const REVIEW: &str = "hagency.web_catalog.command.get";
pub(super) const APPLY: &str = "hagency.web_catalog.command.apply";
pub(super) const CANCEL: &str = "hagency.web_catalog.command.cancel";
pub(super) fn permissions(service: &str) -> &'static [ManagementPermission] {
    use ManagementPermission::*;
    match service {
        GET | OFFERS | TRUST => &[ReadWebCatalog],
        PREPARE | REVIEW | APPLY | CANCEL => &[ReadWebCatalog, ManageWebCatalog, ReadIntents],
        _ => &[],
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Saved { command_id: String }
pub(super) enum Call {
    Get(WebCatalogQuery), Offers(WebCatalogQuery), Trust(WebCatalogQuery),
    Prepare(WebCatalogPrepare), Review(String), Apply(String), Cancel(String),
}
pub(super) fn call(service: &str, args: Value) -> Result<Option<Call>, String> {
    Ok(Some(match service {
        GET | OFFERS | TRUST => {
            let query: WebCatalogQuery = parse(args)?;
            query.validate().map_err(|_| "invalid_management_arguments".to_owned())?;
            match service { GET => Call::Get(query), OFFERS => Call::Offers(query), _ => Call::Trust(query) }
        }
        PREPARE => {
            let input: WebCatalogPrepare = parse(args)?;
            input.validate().map_err(|_| "invalid_management_arguments".to_owned())?;
            Call::Prepare(input)
        }
        REVIEW | APPLY | CANCEL => {
            let saved: Saved = parse(args)?;
            let suffix = saved.command_id.strip_prefix("webcatalog_")
                .ok_or_else(|| "invalid_management_arguments".to_owned())?;
            if suffix.len() != 32 || !suffix.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
                return Err("invalid_management_arguments".into());
            }
            match service { REVIEW => Call::Review(saved.command_id), CANCEL => Call::Cancel(saved.command_id), _ => Call::Apply(saved.command_id) }
        }
        _ => return Ok(None),
    }))
}
impl Call {
    pub(super) fn mutates(&self) -> bool { matches!(self, Self::Prepare(_) | Self::Apply(_) | Self::Cancel(_)) }
    pub(super) async fn execute(self, manager: &NativeManagement) -> Result<Value, String> {
        match self {
            Self::Get(q) => value(manager.web_catalog(q).await.map_err(|e| e.code)?),
            Self::Offers(q) => value(manager.web_offers(q.resource_id).await.map_err(|e| e.code)?),
            Self::Trust(q) => value(manager.web_trust(q).await.map_err(|e| e.code)?),
            Self::Prepare(q) => value(manager.prepare_web_catalog(q).await.map_err(|e| e.code)?),
            Self::Review(id) => value(manager.review_web_catalog_command(id).await.map_err(|e| e.code)?),
            Self::Apply(id) => value(manager.apply_saved_web_catalog(id).await.map_err(|e| e.code)?),
            Self::Cancel(id) => value(manager.cancel_saved_web_catalog(id).await.map_err(|e| e.code)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_original_saved_commands_can_be_applied() {
        let id = "webcatalog_0123456789abcdef0123456789abcdef";
        assert!(matches!(call(APPLY, json!({"commandId":id})), Ok(Some(Call::Apply(_)))));
        assert!(matches!(call(CANCEL, json!({"commandId":id})), Ok(Some(Call::Cancel(_)))));
        for field in ["resourceId", "owner", "profile", "change", "path", "accessToken"] {
            let mut input = json!({"commandId":id});
            input[field] = json!("replacement");
            assert!(call(APPLY, input.clone()).is_err());
            assert!(call(CANCEL, input).is_err());
        }
        assert!(call(APPLY, json!({"commandId":"templatecmd_0123456789abcdef0123456789abcdef"})).is_err());
        assert!(!permissions(GET).contains(&ManagementPermission::ManageWebCatalog));
        for service in [GET, OFFERS, TRUST, PREPARE, REVIEW, APPLY, CANCEL] {
            assert!(octosense_app_contract::hagency::SERVICES.contains(&service));
            assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&service));
            assert!(octosense_app_contract::hagency::words(service).is_some());
        }
    }
}
