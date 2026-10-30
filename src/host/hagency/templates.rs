//! Closed access to retained per-resource agent definitions.
use super::{parse, value, ManagementPermission, NativeManagement, Value};
use ::hagency::native_owner::{TemplateCreateInput, TemplateUpdateInput, TemplateDeleteInput};
use serde::Deserialize;

pub(super) const LIST: &str = "hagency.templates.list";
pub(super) const GET: &str = "hagency.templates.get";
pub(super) const CREATE: &str = "hagency.templates.create";
pub(super) const UPDATE: &str = "hagency.templates.update";
pub(super) const DELETE: &str = "hagency.templates.delete";
pub(super) const REVIEW: &str = "hagency.templates.command.get";
pub(super) const APPLY: &str = "hagency.templates.command.apply";
pub(super) const CANCEL: &str = "hagency.templates.command.cancel";

pub(super) fn permissions(service: &str) -> &'static [ManagementPermission] {
    use ManagementPermission::*;
    match service {
        LIST | GET => &[ReadResourceTemplates],
        CREATE | UPDATE | DELETE => &[ReadResourceTemplates, ManageResourceTemplates],
        REVIEW | APPLY => &[ReadResourceTemplates, ReadIntents],
        // Withdrawing writes the original command's receipt, like apply, so it
        // also needs the explicit write grant the original change required.
        CANCEL => &[ReadResourceTemplates, ManageResourceTemplates, ReadIntents],
        _ => &[],
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Query {
    resource_id: String,
    after: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Detail {
    resource_id: String,
    id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Saved {
    command_id: String,
}

fn invalid() -> String {
    "invalid_management_arguments".into()
}
fn exact_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err(invalid());
    }
    Ok(())
}

pub(super) enum Call {
    List(String, Option<String>),
    Get(String, String),
    Create(TemplateCreateInput),
    Update(TemplateUpdateInput),
    Delete(TemplateDeleteInput),
    Review(String),
    Apply(String),
    Cancel(String),
}
pub(super) fn call(service: &str, args: Value) -> Result<Option<Call>, String> {
    Ok(Some(match service {
        LIST => {
            let q: Query = parse(args)?;
            exact_id(&q.resource_id)?;
            if let Some(after) = &q.after {
                exact_id(after)?;
            }
            Call::List(q.resource_id, q.after)
        }
        GET => {
            let q: Detail = parse(args)?;
            exact_id(&q.resource_id)?;
            exact_id(&q.id)?;
            Call::Get(q.resource_id, q.id)
        }
        CREATE => {
            let input: TemplateCreateInput = parse(args)?;
            input.validate().map_err(|_| invalid())?;
            Call::Create(input)
        }
        UPDATE => {
            let input: TemplateUpdateInput = parse(args)?;
            input.validate().map_err(|_| invalid())?;
            Call::Update(input)
        }
        DELETE => {
            let input: TemplateDeleteInput = parse(args)?;
            input.validate().map_err(|_| invalid())?;
            Call::Delete(input)
        }
        REVIEW | APPLY | CANCEL => {
            let saved: Saved = parse(args)?;
            if !saved.command_id.starts_with("templatecmd_")
                || saved.command_id.len() != "templatecmd_".len() + 32
                || !saved.command_id["templatecmd_".len()..]
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err(invalid());
            }
            match service {
                REVIEW => Call::Review(saved.command_id),
                CANCEL => Call::Cancel(saved.command_id),
                _ => Call::Apply(saved.command_id),
            }
        }
        _ => return Ok(None),
    }))
}
impl Call {
    pub(super) fn mutates(&self) -> bool {
        !matches!(self, Self::List(..) | Self::Get(..) | Self::Review(..))
    }
    pub(super) async fn execute(self, manager: &NativeManagement) -> Result<Value, String> {
        match self {
            Self::List(resource, after) => value(
                manager
                    .resource_templates(resource, after)
                    .await
                    .map_err(|e| e.code)?,
            ),
            Self::Get(resource, id) => value(
                manager
                    .resource_template(resource, id)
                    .await
                    .map_err(|e| e.code)?,
            ),
            Self::Create(input) => value(
                manager
                    .prepare_template_create(input)
                    .await
                    .map_err(|e| e.code)?,
            ),
            Self::Update(input) => value(
                manager
                    .prepare_template_update(input)
                    .await
                    .map_err(|e| e.code)?,
            ),
            Self::Delete(input) => value(
                manager
                    .prepare_template_delete(input)
                    .await
                    .map_err(|e| e.code)?,
            ),
            Self::Review(id) => value(
                manager
                    .review_template_command(id)
                    .await
                    .map_err(|e| e.code)?,
            ),
            Self::Apply(id) => value(manager.apply_saved_template(id).await.map_err(|e| e.code)?),
            Self::Cancel(id) => value(
                manager
                    .cancel_saved_template(id)
                    .await
                    .map_err(|e| e.code)?,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn template_services_reject_ambient_identity_and_only_apply_original_ids() {
        assert!(call(LIST, json!({"resourceId":"resource_a","after":null})).is_ok());
        assert!(call(GET, json!({"resourceId":"resource_a","id":"rad_original"})).is_ok());
        for field in ["owner", "profile", "path", "url", "device", "canEdit"] {
            let mut input = json!({"resourceId":"resource_a"});
            input[field] = json!("forged");
            assert!(call(LIST, input).is_err());
        }
        assert!(call(APPLY, json!({"commandId":"original","name":"replacement"})).is_err());
        let original = "templatecmd_0123456789abcdef0123456789abcdef";
        assert!(matches!(
            call(APPLY, json!({"commandId":original})),
            Ok(Some(Call::Apply(id))) if id == original
        ));
        assert!(call(APPLY, json!({"commandId":original,"resourceId":"replacement"})).is_err());
        assert!(call(REVIEW, json!({"commandId":"graphcmd_0123456789abcdef0123456789abcdef"})).is_err());
        assert!(matches!(
            call(CANCEL, json!({"commandId":original})),
            Ok(Some(Call::Cancel(id))) if id == original
        ));
        for field in ["resourceId", "owner", "profile", "change", "path", "accessToken"] {
            let mut input = json!({"commandId":original});
            input[field] = json!("replacement");
            assert!(call(CANCEL, input).is_err(), "{field}");
        }
        assert!(call(CANCEL, json!({"commandId":"webcatalog_0123456789abcdef0123456789abcdef"})).is_err());
        assert!(call(CANCEL, json!({"commandId":"templatecmd_0123"})).is_err());
        assert!(matches!(call(CANCEL, json!({"commandId":original})), Ok(Some(c)) if c.mutates()));
        assert!(permissions(CANCEL).contains(&ManagementPermission::ManageResourceTemplates));
        assert!(call(GET, json!({"resourceId":"../outside","id":"rad_a"})).is_err());
        assert!(!permissions(LIST).contains(&ManagementPermission::ManageResourceTemplates));
        assert!(permissions(CREATE).contains(&ManagementPermission::ManageResourceTemplates));
        for service in [LIST, GET, CREATE, UPDATE, DELETE, REVIEW, APPLY, CANCEL] {
            assert!(octosense_app_contract::hagency::SERVICES.contains(&service));
            assert!(octosense_app_contract::KNOWN_CAPABILITIES.contains(&service));
            assert!(octosense_app_contract::hagency::words(service).is_some());
        }
    }
}
