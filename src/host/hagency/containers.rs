//! The owner's choice, made while creating a pool, to run its Claude Agents
//! in containers. The package only asks, with one boolean beside the
//! creation's closed input. Which pool that is, where the choice is kept and
//! what runs the containers are the host's.
use super::*;
use crate::hagency::hosting::{self, CreatedContainers};

/// The request field, and the reply field that says what came of it.
const FIELD: &str = "containers";

/// Whether this computer runs pool Agents in containers at all. The Hosting
/// screen offers its switch on the same computers.
pub(super) const OFFERED: bool = cfg!(unix);

/// Takes the choice out of a creation request, leaving the closed input the
/// native command checks.
pub(super) fn take(args: &mut Value) -> Result<bool, String> {
    match args.as_object_mut().and_then(|object| object.remove(FIELD)) {
        None => Ok(false),
        Some(Value::Bool(chosen)) => Ok(chosen),
        Some(_) => Err("invalid_management_arguments".into()),
    }
}

/// What the native review says about the choice. Both languages are here,
/// like the review's other lines for a Claude pool.
pub(super) fn review_line(language: crate::i18n::Language) -> &'static str {
    if language == crate::i18n::Language::Chinese {
        "运行方式：每个 Agent 在自己的容器中运行；容器未就绪时，Agent 不会启动。"
    } else {
        "Runs in: a container for each Agent. Agents do not start until containers are ready."
    }
}

/// The signed-in account, when it is the one the package's session is for.
fn account(owner: &str) -> Option<hosting::Account> {
    crate::hagency::backend::Service::for_current()
        .and_then(|service| service.account())
        .ok()
        .filter(|account| account.owner == owner)
}

/// Before the creating command runs: keeps the choice with the command, so
/// that it holds however the command's result arrives. An error means the
/// choice cannot be kept, and nothing is created.
pub(super) fn expect(owner: &str, command_id: &str) -> Result<(), String> {
    match account(owner) {
        Some(account) if hosting::expect_containers(&account, command_id) => Ok(()),
        _ => Err("pool_containers_unavailable".into()),
    }
}

/// A result naming the pool `command_id` created, whichever way it arrives
/// (the first reply, a retry, the recovery of a saved request). Where the
/// command asked for containers the pool is chosen at once and preparing
/// starts; the reply then says what came of it.
pub(super) fn created(owner: &str, command_id: &str, reply: &mut Value) {
    let Some(pool) = reply.get("resourceId").and_then(Value::as_str).map(str::to_owned) else {
        return;
    };
    // Without this account's session the command keeps the choice for the
    // next time its result arrives.
    let Some(account) = account(owner) else {
        return;
    };
    let word = match hosting::pool_created(&account, command_id, &pool) {
        CreatedContainers::NotAsked => return,
        CreatedContainers::Preparing => "preparing",
        CreatedContainers::NoRuntime => "no_runtime",
        CreatedContainers::Unsaved => "unsaved",
    };
    if let Some(object) = reply.as_object_mut() {
        object.insert(FIELD.into(), json!(word));
    }
}

/// The first reply of a creating command, which may have asked for
/// containers (`chosen`). A pool that asked and is not listed, for whatever
/// reason, would run in the sandbox: the reply then says so, and the page
/// tells its owner before the pool can be published.
pub(super) fn created_first(owner: &str, command_id: &str, chosen: bool, reply: &mut Value) {
    created(owner, command_id, reply);
    if !chosen || reply.get(FIELD).is_some() {
        return;
    }
    let Some(pool) = reply.get("resourceId").and_then(Value::as_str) else {
        return;
    };
    // Listed already when the result arrived another way first.
    let listed = account(owner).is_some_and(|account| hosting::containers_chosen(&account, pool));
    if !listed && let Some(object) = reply.as_object_mut() {
        object.insert(FIELD.into(), json!("unsaved"));
    }
}

/// The saved-request operations that create a pool.
fn creates_a_pool(operation: &Value) -> bool {
    matches!(operation.as_str(), Some("enroll_account_resource" | "create_resource_configuration"))
}

/// A saved request applied again: `{operation, data}`. A pool it created
/// keeps the container choice made with it.
pub(super) fn recovered(owner: &str, command_id: &str, reply: &mut Value) {
    if reply.get("operation").is_some_and(creates_a_pool)
        && let Some(data) = reply.get_mut("data")
    {
        created(owner, command_id, data);
    }
}

/// A saved request as the pending list reads it, with the reply recorded for
/// it. If the app stopped between creating a pool and listing it, this is
/// where the choice is picked up again: the list is read whenever the
/// package opens, before its owner can publish the pool.
pub(super) fn recorded(owner: &str, command_id: &str, operation: &Value, reply: &Value) {
    if creates_a_pool(operation)
        && reply.get("kind").and_then(Value::as_str) == Some("success")
        && let Some(data) = reply.get("data")
    {
        created(owner, command_id, &mut data.clone());
    }
}

/// `hagency.session` says whether a pool can be created with containers
/// chosen, so the page offers the choice only where the Hosting screen
/// offers each pool's switch. Display only: it authorizes nothing.
pub(super) fn display(service: &str, mut value: Value) -> Value {
    if service != "hagency.session" {
        return value;
    }
    let offered = OFFERED
        && crate::hagency::backend::Service::for_current()
            .and_then(|service| service.account())
            .is_ok_and(|account| hosting::containers_offered(&account));
    session_fields(&mut value, offered);
    value
}
fn session_fields(value: &mut Value, offered: bool) {
    if offered && let Some(object) = value.as_object_mut() {
        object.insert("poolContainers".into(), json!(true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choice_is_one_boolean_beside_the_closed_input() {
        let mut args = json!({"commandId":"c","containers":true,"model":"m"});
        assert_eq!(take(&mut args), Ok(true));
        assert_eq!(args, json!({"commandId":"c","model":"m"}), "the native command never sees it");
        assert_eq!(take(&mut args), Ok(false), "not asking is the sandbox");
        assert_eq!(take(&mut json!({"containers":false})), Ok(false));
        for forged in [json!("true"), json!(1), json!(null), json!({"on":true}), json!([true])] {
            assert!(take(&mut json!({"containers":forged})).is_err());
        }
        assert_eq!(take(&mut json!("not an object")), Ok(false));
    }

    #[test]
    fn the_session_says_where_the_choice_is_offered_and_other_replies_are_left() {
        let mut session = json!({"protocol":"p"});
        session_fields(&mut session, false);
        assert_eq!(session, json!({"protocol":"p"}));
        session_fields(&mut session, true);
        assert_eq!(session, json!({"protocol":"p","poolContainers":true}));
        // No account is signed in here: nothing is offered.
        assert_eq!(display("hagency.session", json!({"protocol":"p"})), json!({"protocol":"p"}));
        assert_eq!(display("hagency.resources.list", json!({"resources":[]})), json!({"resources":[]}));
        for language in [crate::i18n::Language::Chinese, crate::i18n::Language::English] {
            let line = review_line(language);
            assert!(!line.contains('\n') && line.chars().count() < 100, "{line}");
        }
        assert!(review_line(crate::i18n::Language::Chinese).contains("容器"));
        assert!(review_line(crate::i18n::Language::English).contains("container"));
    }

    #[test]
    fn only_a_creation_carries_a_pool_to_list() {
        for operation in ["enroll_account_resource", "create_resource_configuration"] {
            assert!(creates_a_pool(&json!(operation)), "{operation}");
        }
        for operation in [json!("rename_resource"), json!("edit_local_resource"), json!("publish_resource"), json!(null), json!(7)] {
            assert!(!creates_a_pool(&operation), "{operation}");
        }
        // Nothing here is signed in, so nothing is listed and replies stay as
        // they are, whatever they carry.
        let mut reply = json!({"operation":"enroll_account_resource","data":{"resourceId":"resource_1"}});
        let before = reply.clone();
        recovered("@owner:test", "command-1", &mut reply);
        assert_eq!(reply, before);
        recorded("@owner:test", "command-1", &json!("enroll_account_resource"), &json!({"kind":"success","data":{"resourceId":"resource_1"}}));
        recorded("@owner:test", "command-1", &json!("enroll_account_resource"), &json!({"kind":"failure","data":{"status":409,"code":"c"}}));
        recorded("@owner:test", "command-1", &json!("enroll_account_resource"), &Value::Null);
    }

    #[test]
    fn a_reply_without_a_pool_is_left_as_it_is() {
        // No resource was created: nothing is looked up, nothing is added.
        let mut reply = json!({"state":"starting"});
        created("@owner:test", "command-1", &mut reply);
        assert_eq!(reply, json!({"state":"starting"}));
        // Nor is a pool whose owner is not the account signed in.
        let mut reply = json!({"resourceId":"resource_1"});
        created("@owner:test", "command-1", &mut reply);
        assert_eq!(reply, json!({"resourceId":"resource_1"}));
        assert_eq!(expect("@owner:test", "command-1").unwrap_err(), "pool_containers_unavailable");
    }

    #[test]
    fn a_first_reply_says_when_a_chosen_pool_could_not_be_listed() {
        // Nothing is signed in here, so no pool can be listed.
        let mut reply = json!({"resourceId":"resource_1"});
        created_first("@owner:test", "command-1", false, &mut reply);
        assert_eq!(reply, json!({"resourceId":"resource_1"}), "a pool that did not ask is left as it is");
        created_first("@owner:test", "command-1", true, &mut reply);
        assert_eq!(reply, json!({"resourceId":"resource_1","containers":"unsaved"}));
        // What an earlier step already said stays.
        let mut reply = json!({"resourceId":"resource_1","containers":"preparing"});
        created_first("@owner:test", "command-1", true, &mut reply);
        assert_eq!(reply["containers"], "preparing");
        // No pool was created: nothing to say.
        let mut reply = json!({"state":"starting"});
        created_first("@owner:test", "command-1", true, &mut reply);
        assert_eq!(reply, json!({"state":"starting"}));
    }
}
