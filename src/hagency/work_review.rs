//! Read-only review of unknown work. No command here can send, start or release usage.
use ::hagency::native_owner::{ManagementCommand, ManagementPackage, ManagementPermission};
use makepad_widgets::*;
use matrix_sdk::ruma::{OwnedEventId, OwnedRoomId};
use serde_json::Value;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OriginalWork {
    pub session: crate::matrix_context::SessionPin,
    pub room: OwnedRoomId,
    pub event: OwnedEventId,
    pub agent: String,
}
#[derive(Clone, Debug)]
pub(crate) enum WorkReviewAction {
    Open(OriginalWork),
    Close,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Query {
    Groups(Option<String>),
    Records { id: String, after: Option<String> },
}
impl Query {
    fn command(&self) -> ManagementCommand {
        match self {
            Self::Groups(after) => ManagementCommand::AgentWorkList {
                project_id: None,
                after: after.clone(),
                limit: Some(16),
                include_pool_agents: true,
            },
            Self::Records { id, after } => ManagementCommand::AgentWork {
                id: id.clone(),
                after: after.clone(),
                limit: Some(16),
            },
        }
    }
}
#[derive(Debug)]
struct Page {
    rows: Vec<Value>,
    next: Option<String>,
}
fn page(query: &Query, value: &Value) -> Result<Page, String> {
    let group = matches!(query, Query::Groups(_));
    if let Query::Records { id, .. } = query {
        if value["requestId"] != *id {
            return Err("History response does not match the selected request.".into());
        }
    }
    let source = &value[if group { "requests" } else { "work" }];
    let rows = source["items"]
        .as_array()
        .filter(|r| r.len() <= 16)
        .ok_or("History response could not be verified.")?
        .clone();
    let next = match &source["nextAfter"] {
        Value::Null => None,
        Value::String(s) if s.len() <= 128 => Some(s.clone()),
        _ => return Err("History response could not be verified.".into()),
    };
    Ok(Page { rows, next })
}
fn matches_origin(
    agent: &str,
    room: &str,
    id: &str,
    assignment: &Value,
    membership: &Value,
) -> bool {
    let a = &assignment["assignment"];
    let m = &membership["membership"];
    a["requestId"] == id
        && a["agentId"] == agent
        && a["id"].as_str().is_some_and(|id| !id.is_empty())
        && m["requestId"] == id
        && m["agentId"] == agent
        && m["roomId"] == room
        && m["assignmentId"] == a["id"]
}
async fn read(query: Query, context: &OriginalWork) -> Result<Page, String> {
    if !context.session.is_current() {
        return Err("History session changed. Reopen the original record.".into());
    }
    let service = super::backend::Service::for_current()?;
    let manager = service
        .management(ManagementPackage {
            digest: blake3::hash(b"hagency-desktop-original-work-read-v1")
                .to_hex()
                .to_string(),
            instance: 1,
            account_epoch: 1,
            permissions: vec![
                ManagementPermission::ReadAgentWork,
                ManagementPermission::ReadAgentStatus,
            ],
        })
        .await?;
    struct Retire(std::sync::Arc<::hagency::daemon::bridge::ManagementClient>);
    impl Drop for Retire {
        fn drop(&mut self) {
            self.0.retire();
        }
    }
    let manager = Retire(manager);
    if !context.session.is_current() {
        return Err("History session changed. Reopen the original record.".into());
    }
    if let Query::Records { id, .. } = &query {
        let assignment = manager
            .0
            .central(ManagementCommand::AgentAssignment { id: id.clone() })
            .await
            .map_err(|_| {
                "Original records could not be read. Check access and connection, then refresh."
                    .to_owned()
            })?;
        if !context.session.is_current() {
            return Err("History session changed. Reopen the original record.".into());
        }
        let membership = manager
            .0
            .central(ManagementCommand::AgentMembership { id: id.clone() })
            .await
            .map_err(|_| {
                "Original records could not be read. Check access and connection, then refresh."
                    .to_owned()
            })?;
        if !matches_origin(
            &context.agent,
            context.room.as_str(),
            id,
            &assignment,
            &membership,
        ) {
            return Err("This request does not match the original Agent and room. Choose another Agent or check the execution device.".into());
        }
    }
    if !context.session.is_current() {
        return Err("History session changed. Reopen the original record.".into());
    }

    let value = manager.0.central(query.command()).await.map_err(|_| {
        "Original records could not be read. Check access and connection, then refresh.".to_owned()
    })?;
    page(&query, &value)
}
fn records(rows: &[Value], event: &str) -> String {
    let mut output = Vec::new();
    for row in rows.iter().filter(|row| row["eventId"] == event) {
        let text = |field: &str| row[field].as_str().unwrap_or("Not confirmed");
        let state = |field: &str| {
            crate::i18n::tr(match row[field].as_str() {
                Some("unknown") => "Unknown",
                Some("queued") => "Queued",
                Some("acknowledged") => "Received by executor",
                Some("started") => "Start recorded",
                Some("completed") => "Response completed",
                Some("failed") => "Agent run failed.",
                Some("cancelled") => "Cancelled",
                Some("sent") => "Sent",
                Some("pending") => "Pending",
                Some("issued") => "Delivery requested",
                Some("blocked") => "Blocked",
                _ => "Not confirmed",
            })
            .to_owned()
        };
        let mut lines = vec![
            crate::i18n::format("Original task: {id}", &[("id", text("id").into())]),
            crate::i18n::format("Execution: {id}", &[("id", text("executionId").into())]),
            crate::i18n::format("Recorded work state: {state}", &[("state", state("state"))]),
            crate::i18n::format(
                "Reply delivery state: {state}",
                &[("state", state("replyState"))],
            ),
        ];
        if row["usage"].is_object() {
            let usage = &row["usage"];
            let amount = |key: &str| {
                usage[key]
                    .as_i64()
                    .filter(|n| *n >= 0)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| crate::i18n::tr("Not confirmed").into())
            };
            lines.push(crate::i18n::format("Known consumed tokens: {known}. Retained accounting total: {retained}. Usage final: {final}.",&[("known",amount("knownConsumedTokens")),("retained",amount("retainedTokens")),("final",crate::i18n::tr(if usage["complete"]==true{"Yes"}else{"No"}).into())]));
        } else {
            lines.push(
                crate::i18n::tr("No usage receipt is available. Do not assume the cost is zero.")
                    .into(),
            );
        }
        output.push(lines.join("\n"));
    }
    if output.is_empty() {
        crate::i18n::tr("No matching original message on this page. Check further pages or the execution device; this does not prove nothing happened.").into()
    } else {
        output.join("\n\n")
    }
}
#[derive(Debug)]
struct ReadResult {
    context: OriginalWork,
    nonce: u64,
    query: Query,
    result: Result<Page, String>,
}
script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    mod.widgets.WorkReview = #(WorkReview::register_widget(vm)) {
        width: Fill {max: 680} height: Fill {max: 680} flow: Down padding: 16 spacing: 12
        show_bg: true draw_bg.color: mod.widgets.RINX_SURFACE
        View {width: Fill height: Fit flow: Right spacing: 12
            Label {width: Fill height: Fit flow: Flow.Right{wrap:true} text:#(crate::i18n::tr("Review original task records")) i18n_text:"Review original task records" draw_text.color:mod.widgets.RINX_INK}
            close := Button {text:#(crate::i18n::tr("Close")) i18n_text:"Close"}
        }
        Label {width: Fill height: Fit flow: Flow.Right{wrap:true} text:#(crate::i18n::tr("Unknown is not failed. Check tool receipts, files and external systems on the execution device before starting a separate request.")) i18n_text:"Unknown is not failed. Check tool receipts, files and external systems on the execution device before starting a separate request." draw_text.color:mod.widgets.RINX_MUTED}
        ScrollYView {width: Fill height: Fill flow: Down spacing: 12
            source := Label {width: Fill height: Fit flow: Flow.Right{wrap:true} draw_text.color:mod.widgets.RINX_MUTED}
            status := Label {width: Fill height: Fit flow: Flow.Right{wrap:true} draw_text.color:mod.widgets.RINX_MUTED}
            groups := DropDown {width: Fill labels:[]}
            show_group := Button {width: Fill text:#(crate::i18n::tr("Read selected Agent history")) i18n_text:"Read selected Agent history"}
            entries := Label {width: Fill height: Fit flow: Flow.Right{wrap:true} draw_text.color:mod.widgets.RINX_INK}
        }
        View {width: Fill height: Fit flow: Flow.Right{wrap:true} spacing: 8
            source_message := Button {text:#(crate::i18n::tr("Open original message")) i18n_text:"Open original message"}
            refresh := Button {text:#(crate::i18n::tr("Refresh status")) i18n_text:"Refresh status"}
            next := Button {text:#(crate::i18n::tr("Next page")) i18n_text:"Next page"}
            back := Button {text:#(crate::i18n::tr("Choose another Agent")) i18n_text:"Choose another Agent"}
        }
    }
}
#[derive(Script, ScriptHook, Widget)]
pub struct WorkReview {
    #[deref]
    view: View,
    #[rust]
    context: Option<OriginalWork>,
    #[rust]
    query: Option<Query>,
    #[rust]
    groups: Vec<Value>,
    #[rust]
    groups_after: Option<String>,
    #[rust]
    groups_next: Option<String>,
    #[rust]
    rows: Vec<Value>,
    #[rust]
    next: Option<String>,
    #[rust]
    selected: usize,
    #[rust]
    loading: bool,
    #[rust]
    nonce: u64,
    #[rust]
    error: String,
    #[rust]
    task: Option<tokio::task::JoinHandle<()>>,
}
impl WorkReview {
    fn cancel(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.nonce = self.nonce.wrapping_add(1);
        self.loading = false;
    }
    fn render(&mut self, cx: &mut Cx) {
        let groups = matches!(self.query, Some(Query::Groups(_)));
        for id in [ids!(groups), ids!(show_group)] {
            self.view.widget(cx, id).set_visible(cx, groups);
        }
        self.view.widget(cx, ids!(back)).set_visible(cx, !groups);
        let labels = self
            .groups
            .iter()
            .map(|r| {
                format!(
                    "{} · {}",
                    r["agentName"].as_str().unwrap_or("Agent"),
                    r["projectName"]
                        .as_str()
                        .or(r["poolName"].as_str())
                        .unwrap_or("")
                )
            })
            .collect::<Vec<_>>();
        self.view.drop_down(cx, ids!(groups)).set_labels(cx, labels);
        self.view
            .drop_down(cx, ids!(groups))
            .set_selected_item(cx, self.selected.min(self.groups.len().saturating_sub(1)));
        let status = if self.loading {
            crate::i18n::tr("Checking…")
        } else if !self.error.is_empty() {
            crate::i18n::tr(&self.error)
        } else {
            crate::i18n::tr(
                "Records are read-only. Missing records do not mean there were no side effects or charges.",
            )
        };
        self.view.label(cx, ids!(status)).set_text(cx, status);
        if let Some(context) = &self.context {
            self.view.label(cx, ids!(source)).set_text(
                cx,
                &format!("{}\n{}\n{}", context.room, context.event, context.agent),
            );
            let entries = if groups {
                String::new()
            } else {
                records(&self.rows, context.event.as_str())
            };
            self.view.label(cx, ids!(entries)).set_text(cx, &entries);
        }
        for id in [ids!(refresh), ids!(back), ids!(source_message)] {
            self.view.button(cx, id).set_enabled(cx, !self.loading);
        }
        self.view
            .button(cx, ids!(show_group))
            .set_enabled(cx, !self.loading && !self.groups.is_empty());
        self.view
            .button(cx, ids!(next))
            .set_enabled(cx, !self.loading && self.next.is_some());
        self.view.redraw(cx);
    }
    fn load(&mut self, cx: &mut Cx, query: Query) {
        let Some(context) = self.context.clone().filter(|c| c.session.is_current()) else {
            return;
        };
        self.cancel();
        self.loading = true;
        self.error.clear();
        self.rows.clear();
        self.next = None;
        if let Query::Groups(after) = &query {
            self.groups.clear();
            self.groups_after = after.clone();
        }
        self.query = Some(query.clone());
        let nonce = self.nonce;
        self.render(cx);
        self.task = Some(crate::account_session::spawn(async move {
            let result=tokio::time::timeout(std::time::Duration::from_secs(30),read(query.clone(),&context)).await.unwrap_or_else(|_|Err("Original records could not be read. Check access and connection, then refresh.".into()));
            Cx::post_action(ReadResult {
                context,
                nonce,
                query,
                result,
            });
        }));
    }
}
impl Drop for WorkReview {
    fn drop(&mut self) {
        self.cancel();
    }
}
impl Widget for WorkReview {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self
            .context
            .as_ref()
            .is_some_and(|c| !c.session.is_current())
        {
            self.cancel();
            self.context = None;
            self.groups.clear();
            self.rows.clear();
            self.view.label(cx, ids!(entries)).set_text(cx, "");
            self.view.label(cx, ids!(source)).set_text(cx, "");
            cx.action(WorkReviewAction::Close);
            return;
        }
        if let Event::Actions(actions) = event {
            for action in actions {
                if let Some(done) = action.downcast_ref::<ReadResult>()
                    && self.loading
                    && done.context.session.is_current()
                    && self.context.as_ref() == Some(&done.context)
                    && done.nonce == self.nonce
                    && self.query.as_ref() == Some(&done.query)
                {
                    self.loading = false;
                    self.task.take();
                    match &done.result {
                        Ok(page) => {
                            self.next = page.next.clone();
                            match &done.query {
                                Query::Groups(_) => {
                                    self.groups = page.rows.clone();
                                    self.groups_next = page.next.clone();
                                    self.selected = 0;
                                }
                                _ => self.rows = page.rows.clone(),
                            }
                        }
                        Err(error) => {
                            self.error = if error
                                == "This request does not match the original Agent and room. Choose another Agent or check the execution device."
                            {
                                error.clone()
                            } else {
                                "Original records could not be read. Check access and connection, then refresh.".into()
                            }
                        }
                    }
                    self.render(cx);
                }
            }
            if self.view.button(cx, ids!(close)).clicked(actions) {
                cx.action(WorkReviewAction::Close);
            }
            if self.loading {
                return;
            }
            if let Some(index) = self.view.drop_down(cx, ids!(groups)).selected(actions) {
                self.selected = index;
            }
            if self.view.button(cx, ids!(show_group)).clicked(actions)
                && let Some(id) = self
                    .groups
                    .get(self.selected)
                    .and_then(|r| r["requestId"].as_str())
                    .map(str::to_owned)
            {
                self.load(cx, Query::Records { id, after: None });
            }
            if self.view.button(cx, ids!(refresh)).clicked(actions)
                && let Some(query) = self.query.clone()
            {
                self.load(cx, query);
            }
            if self.view.button(cx, ids!(back)).clicked(actions) {
                self.query = Some(Query::Groups(self.groups_after.clone()));
                self.next = self.groups_next.clone();
                self.rows.clear();
                self.render(cx);
            }
            if self.view.button(cx, ids!(next)).clicked(actions)
                && let (Some(after), Some(query)) = (self.next.clone(), self.query.clone())
            {
                let next = match query {
                    Query::Groups(_) => Query::Groups(Some(after)),
                    Query::Records { id, .. } => Query::Records {
                        id,
                        after: Some(after),
                    },
                };
                self.load(cx, next);
            }
            if self.view.button(cx, ids!(source_message)).clicked(actions)
                && let Some(context) = &self.context
                && context.session.is_current()
            {
                crate::matrix_links::open(
                    &context
                        .room
                        .matrix_to_event_uri(context.event.clone())
                        .to_string(),
                );
                cx.action(WorkReviewAction::Close);
            }
        }
        self.view.handle_event(cx, event, scope);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
impl WorkReviewRef {
    pub(crate) fn open(&self, cx: &mut Cx, context: OriginalWork) {
        if let Some(mut this) = self.borrow_mut() {
            this.cancel();
            this.context = Some(context);
            this.groups.clear();
            this.groups_next = None;
            this.groups_after = None;
            this.selected = 0;
            this.load(cx, Query::Groups(None));
        }
    }
    pub(crate) fn clear(&self, cx: &mut Cx) {
        if let Some(mut this) = self.borrow_mut() {
            this.cancel();
            this.context = None;
            this.groups.clear();
            this.rows.clear();
            for id in [ids!(source), ids!(entries), ids!(status)] {
                this.view.label(cx, id).set_text(cx, "");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn every_review_query_is_read_only_and_keeps_its_original_request_and_cursor() {
        let groups = Query::Groups(Some("original-cursor".into())).command();

        assert!(
            matches!(groups,ManagementCommand::AgentWorkList{after:Some(ref cursor),limit:Some(16),include_pool_agents:true,..} if cursor=="original-cursor")
        );
        let records = Query::Records {
            id: "request-original".into(),
            after: Some("work-original".into()),
        }
        .command();

        assert!(
            matches!(records,ManagementCommand::AgentWork{id,after:Some(cursor),limit:Some(16)} if id=="request-original" && cursor=="work-original")
        );
    }
    #[test]
    fn original_agent_room_request_and_assignment_must_all_match_before_reading_history() {
        let a =
            json!({"assignment":{"id":"assignment-a","requestId":"request-a","agentId":"agent-a"}});
        let m = json!({"membership":{"assignmentId":"assignment-a","requestId":"request-a","agentId":"agent-a","roomId":"!a:test"}});
        assert!(matches_origin("agent-a", "!a:test", "request-a", &a, &m));
        for (agent, room, id) in [
            ("agent-b", "!a:test", "request-a"),
            ("agent-a", "!b:test", "request-a"),
            ("agent-a", "!a:test", "request-b"),
        ] {
            assert!(!matches_origin(agent, room, id, &a, &m));
        }
        let mut wrong = m.clone();
        wrong["membership"]["assignmentId"] = json!("other");
        assert!(!matches_origin(
            "agent-a",
            "!a:test",
            "request-a",
            &a,
            &wrong
        ));
    }
    #[test]
    fn history_identity_shape_and_allocation_are_checked_before_display() {
        let query = Query::Records {
            id: "original".into(),
            after: None,
        };
        assert!(
            page(
                &query,
                &json!({"requestId":"other","work":{"items":[],"nextAfter":null}})
            )
            .is_err()
        );
        assert!(page(&query,&json!({"requestId":"original","work":{"items":vec![json!({});17],"nextAfter":null}})).is_err());
        assert!(
            page(
                &query,
                &json!({"requestId":"original","work":{"items":[],"nextAfter":"x".repeat(129)}})
            )
            .is_err()
        );
        assert!(
            page(
                &query,
                &json!({"requestId":"original","work":{"items":[],"nextAfter":null}})
            )
            .is_ok()
        );
    }
    #[test]
    fn unknown_with_a_completed_external_effect_and_lost_reply_never_becomes_failed_or_free() {
        // A retained original result, not a command to run the tool again.
        let rows = vec![
            json!({"id":"original-work","executionId":"original-execution","eventId":"$original","state":"unknown","replyState":"unknown","usage":{"knownConsumedTokens":20,"retainedTokens":80,"complete":false}}),
        ];
        let text = records(&rows, "$original");
        for required in ["original-work", "original-execution", "unknown", "20", "80"] {
            assert!(text.to_lowercase().contains(required), "{text}");
        }
        assert!(!text.to_lowercase().contains("send again"));
        assert!(!text.contains("failed"));
        assert!(records(&rows, "$other").contains("does not prove nothing happened"));
        let missing = records(
            &[json!({"eventId":"$original","state":"unknown"})],
            "$original",
        );
        assert!(missing.contains("Do not assume the cost is zero"));
    }
    fn fixture() -> (Cx, WorkReview) {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let review = cx.with_vm(|vm| {
            crate::theme::tests::install(vm, crate::theme::Selection::default());
            crate::app::register_widgets(vm);
            vm.bx.captured_errors = Some(Vec::new());
            let value = script_eval!(vm,{mod.widgets.WorkReview {}});
            let review = WorkReview::script_from_value(vm, value);
            assert!(vm.take_errors().is_empty());
            review
        });
        (cx, review)
    }
    #[test]
    fn history_view_has_only_read_navigation_and_no_automatic_task_creation() {
        let (mut cx, mut view) = fixture();
        view.query = Some(Query::Groups(None));
        view.render(&mut cx);
        assert!(
            !view
                .view
                .button(&mut cx, ids!(show_group))
                .borrow()
                .unwrap()
                .enabled()
        );
        assert!(
            !view
                .view
                .button(&mut cx, ids!(next))
                .borrow()
                .unwrap()
                .enabled()
        );
        for id in [ids!(retry), ids!(send), ids!(approve), ids!(start)] {
            assert!(view.view.widget(&mut cx, id).is_empty());
        }
        let close = view.view.button(&mut cx, ids!(close)).widget_uid();
        let clicked = cx.capture_actions(|cx| {
            cx.widget_action(close, ButtonAction::Clicked(KeyModifiers::default()))
        });
        let actions = cx.capture_actions(|cx| {
            view.handle_event(cx, &Event::Actions(clicked), &mut Scope::empty())
        });
        assert!(actions.iter().any(|a| matches!(
            a.downcast_ref::<WorkReviewAction>(),
            Some(WorkReviewAction::Close)
        )));
        assert!(view.task.is_none());
    }
}
