//! Permission cards read through authenticated Hagency management, never chat text.
use super::permission_retry::PermissionRetry;
use crate::hagency::backend::Service;
use crate::hagency::permission_failure::{PermissionFailure, Recovery};
use ::hagency::native_owner::ManagementCommand;
use makepad_widgets::*;
use serde_json::Value;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PermissionScope {
    epoch: u64,
    revision: u64,
}
impl PermissionScope {
    fn capture() -> Self {
        Self {
            epoch: crate::account_session::epoch(),
            revision: crate::sliding_sync::client_revision(),
        }
    }
    fn is_current(self) -> bool {
        self == Self::capture()
            && !crate::account_session::is_closing()
            && !crate::sliding_sync::account_changing()
            && !crate::logout::logout_state_machine::is_logout_in_progress()
    }
}

#[derive(Debug)]
struct PermissionResult {
    scope: PermissionScope,
    identity: String,
    room: String,
    thread: String,
    nonce: u64,
    deciding: bool,
    value: Result<Value, PermissionFailure>,
}
fn execution_row<'a>(value:&'a Value,room:&str,thread:&str)->Option<&'a Value> {
    value["executions"].as_array()?.iter().find(|r|
        r["roomId"]==room && (thread.is_empty() || r["threadRoot"]==thread))
}
fn execution_busy(value:&Value,room:&str,thread:&str)->bool {
    execution_row(value,room,thread).is_some_and(|row|
        row["reason"]!="budget_blocked" && matches!(row["state"].as_str(),Some("started"|"pending"|"queued"|"acknowledged"|"bound"|"reply_pending")))
}
fn execution_notice(value:&Value,room:&str,thread:&str)->Option<&'static str> {
    let row=execution_row(value,room,thread)?;
    if row["reason"]=="budget_blocked" {
        return Some(match row["budgetState"].as_str() {
            Some("usage_unresolved") => "The previous call has not been settled. Its reserved allowance is unavailable until usage is confirmed.",
            Some("exhausted") => "The Agent's allowance has been used. Check its next renewal period with the resource owner.",
            _ => "No allowance is currently available for this message. Check the Agent's allowance and unsettled calls.",
        });
    }
    if row["reason"]=="usage_unrepresentable" {return Some("The previous call has not been settled. Its reserved allowance is unavailable until usage is confirmed.");}
    match row["state"].as_str()? {
        "started"=>Some("Agent is working…"),
        "pending"|"queued"|"acknowledged"|"bound"=>Some("Message queued; waiting for Agent."),
        "unknown"=>Some("Run interrupted; no final answer confirmed. Send the message again if you still need it."),
        "failed"=>Some("Agent run failed."),
        "blocked"=>Some("Message could not start. Check the resource and Agent connection."),
        "paused"=>Some("Agent is paused."),
        "cancelled"|"expired"=>Some("This message was not processed; it will not retry automatically."),
        "reply_pending"=>Some("Answer generated; waiting for delivery."),
        "reply_unknown"=>Some("Answer delivery could not be confirmed."),
        _=>None,
    }
}
script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    mod.widgets.SharedPermissionNotice = #(SharedPermissionNotice::register_widget(vm)) {
        ..mod.widgets.View
        width: Fill height: Fit flow: Down spacing: 8 padding: 12
        visible: false show_bg: true draw_bg.color: mod.widgets.RINX_HOVER
        status := View {width: Fill height: Fit flow: Right spacing: 8 align: Align{y: 0.5}
            activity_spinner := PacedLoadingSpinner {width: 20 height: 20 visible: false draw_bg +: {color: mod.widgets.RINX_ACCENT stroke_width: 2.0}}
            title := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_INK}
        }
        context := Label {width: Fill height: Fit draw_text.color: mod.widgets.RINX_MUTED}
        request_details := ScrollYView {width: Fill height: 130
            details := Label {width: Fill height: Fit draw_text.color: mod.widgets.RINX_INK}
        }
        hint := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
        actions := View {width: Fill height: Fit flow: Down spacing: 4
            once := Button {width: Fill text: #(crate::i18n::tr("Approve once")) i18n_text: "Approve once"}
            session := Button {width: Fill text: #(crate::i18n::tr("Always approve for this session")) i18n_text: "Always approve for this session"}
            deny := Button {width: Fill text: #(crate::i18n::tr("Deny")) i18n_text: "Deny"}
        }
        recovery := View {width: Fill height: Fit flow: Down spacing: 6 visible: false
            recovery_hint := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
            last_checked := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
            refresh := Button {width: Fill text: #(crate::i18n::tr("Refresh status")) i18n_text: "Refresh status"}
            recover := Button {width: Fill text: #(crate::i18n::tr("My Agents")) i18n_text: "My Agents"}
        }
        next := Button {text: #(crate::i18n::tr("Next request")) i18n_text: "Next request" visible: false}
    }
}
#[derive(Script, ScriptHook, Widget)]
pub struct SharedPermissionNotice {
    #[deref]
    view: View,
    #[rust]
    identity: String,
    #[rust]
    room: String,
    #[rust]
    thread: String,
    #[rust]
    error: String,
    #[rust]
    failure: Option<PermissionFailure>,
    #[rust]
    retry: PermissionRetry,
    #[rust]
    account_scope: PermissionScope,
    #[rust]
    task: Option<tokio::task::JoinHandle<()>>,
    #[rust]
    decision_unconfirmed: bool,
    #[rust]
    deciding: bool,
    #[rust]
    nonce: u64,
    #[rust]
    loading: bool,
    #[rust]
    timer: Timer,
    #[rust]
    active: bool,
    #[rust]
    rows: Vec<Value>,
    #[rust]
    selected: usize,
    #[rust]
    execution_text: String,
    #[rust]
    execution_busy: bool,
}
impl SharedPermissionNotice {
    /// Shows the spinner while the Agent works. The spinner updates its retained
    /// draw call without re-running the draw of the timeline that contains it.
    fn sync_animation(&mut self, cx: &mut Cx) {
        let animate =
            self.active && self.execution_busy && self.rows.is_empty() && self.error.is_empty();
        self.view
            .widget(cx, ids!(activity_spinner))
            .set_visible(cx, animate);
    }
    fn render(&mut self, cx: &mut Cx) {
        self.sync_animation(cx);
        let failed = !self.error.is_empty();
        self.view.widget(cx, ids!(recovery)).set_visible(cx, failed);
        let recovery = self.failure.as_ref().map(PermissionFailure::recovery);
        self.view
            .button(cx, ids!(refresh))
            .set_enabled(cx, self.active && !self.loading);
        let destination = matches!(
            recovery,
            Some(
                Recovery::SignIn
                    | Recovery::Access
                    | Recovery::Service
                    | Recovery::Unsupported
                    | Recovery::Manual
            )
        );
        self.view
            .widget(cx, ids!(recover))
            .set_visible(cx, destination);
        self.view
            .button(cx, ids!(recover))
            .set_enabled(cx, self.active && !self.loading);
        self.view.button(cx, ids!(recover)).set_text(
            cx,
            crate::i18n::tr(if recovery == Some(Recovery::SignIn) {
                "Sign in again"
            } else {
                "My Agents"
            }),
        );
        let hint = if self.decision_unconfirmed {
            crate::i18n::tr("The decision was not confirmed. Only status will be refreshed; the decision will not be sent again automatically.").to_owned()
        } else if self.loading {
            crate::i18n::tr("Checking…").to_owned()
        } else if let Some(at) = self.retry.next {
            crate::i18n::format(
                "Automatic status retry after {seconds} seconds.",
                &[(
                    "seconds",
                    at.saturating_duration_since(Instant::now())
                        .as_secs_f64()
                        .ceil()
                        .max(1.0)
                        .to_string(),
                )],
            )
        } else {
            crate::i18n::tr(
                "Automatic retries are paused. Refresh status after addressing the problem.",
            )
            .to_owned()
        };
        self.view.label(cx, ids!(recovery_hint)).set_text(cx, &hint);
        let checked = self.retry.last_success.map(|at| {
            chrono::DateTime::<chrono::Utc>::from(at)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        });
        self.view
            .widget(cx, ids!(last_checked))
            .set_visible(cx, checked.is_some());
        if let Some(checked) = checked {
            self.view.label(cx, ids!(last_checked)).set_text(
                cx,
                &format!("{}: {}", crate::i18n::tr("Last checked (UTC)"), checked),
            );
        }
        let has_request=self.rows.get(self.selected).is_some();
        for id in [ids!(context),ids!(request_details),ids!(hint),ids!(actions)] {
            self.view.widget(cx,id).set_visible(cx,has_request);
        }
        let Some(row)=self.rows.get(self.selected) else {
            self.view.set_visible(cx,!self.execution_text.is_empty() || !self.error.is_empty());
            self.view.widget(cx,ids!(next)).set_visible(cx,false);
            self.view.label(cx,ids!(title)).set_text(cx,if self.error.is_empty(){crate::i18n::tr(&self.execution_text)}else{crate::i18n::tr(&self.error)});
            self.view.redraw(cx);return;
        };
        let a=&row["approval"];
        let pending=row["decision"].is_null();let can=row["canDecide"]==true && pending && !self.loading && self.error.is_empty();
        self.view.set_visible(cx,true);
        self.view.label(cx,ids!(title)).set_text(cx,&crate::i18n::format("{agent} · Permission request", &[("agent",a["agentName"].as_str().unwrap_or("Agent").to_owned())]));
        self.view.label(cx,ids!(context)).set_text(cx,&format!("{} · {}",a["requesterMxid"].as_str().unwrap_or(""),a["eventId"].as_str().unwrap_or("")));
        self.view.label(cx,ids!(details)).set_text(cx,&if row["encryptedPending"]==true {crate::i18n::tr("Waiting for encrypted permission details…").into()}else{serde_json::to_string_pretty(&a["request"]).unwrap_or_default()});
        let hint=if !pending {crate::i18n::tr("Decision sent; waiting for Agent").to_owned()}
        else if row["encryptedPending"]==true {crate::i18n::tr("Approval becomes available after the encrypted details arrive.").into()}
        else if row["canDecide"]!=true {crate::i18n::format("Waiting for resource owner {owner}", &[("owner",a["ownerMxid"].as_str().unwrap_or("").to_owned())])}
        else {crate::i18n::tr("Session approval covers this Codex session only. It ends when this run stops.").into()};
        self.view.label(cx,ids!(hint)).set_text(cx,if self.error.is_empty(){&hint}else{crate::i18n::tr(&self.error)});
        self.view.button(cx,ids!(once)).set_enabled(cx,can);
        self.view.button(cx,ids!(session)).set_enabled(cx,can && a["request"]["sessionAllowed"]==true);
        self.view.button(cx,ids!(deny)).set_enabled(cx,can);
        self.view.widget(cx,ids!(next)).set_visible(cx,self.rows.len()>1);
        self.view.redraw(cx);
    }
    fn arm_timer(&mut self, cx: &mut Cx) {
        cx.stop_timer(self.timer);
        self.timer = Timer::default();
        if self.active
            && let Some(at) = self.retry.next
        {
            self.timer = cx.start_timeout(
                at.saturating_duration_since(Instant::now())
                    .as_secs_f64()
                    .max(0.05),
            );
        }
    }
    fn failed(&mut self, cx: &mut Cx, error: PermissionFailure) {
        self.loading = false;
        self.error = error.message().into();
        self.retry
            .failed(error.recovery(), Instant::now(), rand::random::<f64>());
        self.failure = Some(error);
        self.render(cx);
        self.arm_timer(cx);
    }
    fn interrupt(&mut self, cx: &mut Cx) {
        cx.stop_timer(self.timer);
        self.timer = Timer::default();
        self.nonce = self.nonce.wrapping_add(1);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        if self.loading {
            self.retry.interrupted(Instant::now());
            if self.deciding {
                self.decision_unconfirmed = true;
                self.error="The decision was not confirmed. Only status will be refreshed; the decision will not be sent again automatically.".into();
            }
        }
        self.deciding = false;
        self.loading = false;
    }
    fn request(&mut self, cx: &mut Cx, decision: Option<&str>, manual: bool) {
        if self.loading || !self.active || self.room.is_empty() || !self.account_scope.is_current()
        {
            return;
        }
        if decision.is_none() && !manual && !self.retry.due(Instant::now()) {
            if self.timer.is_empty() {
                self.arm_timer(cx);
            }
            return;
        }
        let command = if let Some(decision) = decision {
            if !self.error.is_empty()
                || !matches!(decision, "approve_once" | "approve_session" | "deny")
            {
                return;
            }
            let Some(row) = self.rows.get(self.selected) else {
                return;
            };
            if row["canDecide"] != true
                || !row["decision"].is_null()
                || (decision == "approve_session"
                    && row["approval"]["request"]["sessionAllowed"] != true)
            {
                return;
            }
            let (Some(id), Some(digest)) = (row["approval"]["id"].as_str(), row["digest"].as_str())
            else {
                return;
            };
            ManagementCommand::SharedApprovalDecide {
                id: id.into(),
                digest: digest.into(),
                decision: decision.into(),
            }
        } else {
            ManagementCommand::SharedApprovals {
                room_id: Some(self.room.clone()),
            }
        };
        cx.stop_timer(self.timer);
        self.timer = Timer::default();
        let service = match Service::for_current() {
            Ok(service) => service,
            Err(code) => {
                self.failed(
                    cx,
                    if crate::sliding_sync::token_expired() {
                        PermissionFailure::from("matrix_authorization_required")
                    } else {
                        code.into()
                    },
                );
                return;
            }
        };
        self.identity = service.identity();
        self.loading = true;
        self.retry.begin();
        self.nonce = self.nonce.wrapping_add(1);
        self.render(cx);
        let identity = self.identity.clone();
        let room = self.room.clone();
        let thread = self.thread.clone();
        let nonce = self.nonce;
        let scope = self.account_scope;
        let deciding = decision.is_some();
        self.deciding = deciding;
        self.task = Some(crate::account_session::spawn(async move {
            let value = tokio::time::timeout(Duration::from_secs(30), async {
                let result = service.shared_permissions_detailed(command).await?;
                if result.get("approvals").is_some() {
                    Ok(result)
                } else {
                    service
                        .shared_permissions_detailed(ManagementCommand::SharedApprovals {
                            room_id: Some(room.clone()),
                        })
                        .await
                }
            })
            .await
            .unwrap_or_else(|_| Err(PermissionFailure::from("permission_query_timeout")));
            Cx::post_action(PermissionResult {
                scope,
                identity,
                room,
                thread,
                nonce,
                deciding,
                value,
            });
        }));
    }
    fn accept_result(&mut self, cx: &mut Cx, done: &PermissionResult) {
        if !self.active
            || !self.loading
            || !self.account_scope.is_current()
            || done.scope != self.account_scope
            || done.identity != self.identity
            || done.room != self.room
            || done.thread != self.thread
            || done.nonce != self.nonce
        {
            return;
        }
        self.task.take();
        self.loading = false;
        self.deciding = false;
        match &done.value {
            Ok(value) => {
                // Missing/malformed status cannot masquerade as a fresh empty list.
                let Some(rows) = value["approvals"].as_array() else {
                    self.failed(cx, "invalid_permission_status".into());
                    return;
                };
                self.error.clear();
                self.failure = None;
                self.decision_unconfirmed = false;
                self.retry.success(Instant::now());
                self.execution_busy = execution_busy(value, &self.room, &self.thread);
                self.execution_text = execution_notice(value, &self.room, &self.thread)
                    .unwrap_or_default()
                    .into();
                self.rows = rows
                    .iter()
                    .filter(|r| {
                        r["approval"]["roomId"] == self.room
                            && (self.thread.is_empty()
                                || r["approval"]["threadRoot"] == self.thread)
                    })
                    .take(16)
                    .cloned()
                    .collect();
                if self.selected >= self.rows.len() {
                    self.selected = 0;
                }
                self.render(cx);
                self.arm_timer(cx);
            }
            Err(error) => {
                self.decision_unconfirmed |= done.deciding;
                self.failed(cx, error.clone());
            }
        }
    }
    fn set_active_state(&mut self, cx: &mut Cx, active: bool) {
        if self.active == active {
            return;
        }
        self.active = active;
        if !active {
            self.interrupt(cx);
        } else if self.retry.due(Instant::now()) {
            self.request(cx, None, false);
        } else {
            self.arm_timer(cx);
        }
        self.sync_animation(cx);
    }
}
impl Drop for SharedPermissionNotice {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
impl Widget for SharedPermissionNotice {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if !self.account_scope.is_current() {
            self.interrupt(cx);
            self.execution_busy = false;
            self.sync_animation(cx);
            self.view.set_visible(cx, false);
            self.rows.clear();
            return;
        }
        if !self.active {
            return;
        }
        if self.timer.is_event(event).is_some() {
            self.timer = Timer::default();
            self.request(cx, None, false);
        }
        if let Event::Actions(actions) = event {
            for action in actions {
                if let Some(done) = action.downcast_ref::<PermissionResult>() {
                    self.accept_result(cx, done);
                }
            }
            if self.view.button(cx, ids!(refresh)).clicked(actions) {
                self.request(cx, None, true);
            }
            if self.view.button(cx, ids!(recover)).clicked(actions) && !self.loading {
                match self.failure.as_ref().map(PermissionFailure::recovery) {
                    Some(Recovery::SignIn) => {
                        cx.action(crate::accounts::AccountAction::Reauthenticate)
                    }
                    Some(
                        Recovery::Access
                        | Recovery::Service
                        | Recovery::Unsupported
                        | Recovery::Manual,
                    ) => cx.action(crate::hagency::ui::HagencyAction::OpenAgents),
                    _ => {}
                }
            }
            for (id, decision) in [
                (ids!(once), "approve_once"),
                (ids!(session), "approve_session"),
                (ids!(deny), "deny"),
            ] {
                if self.view.button(cx, id).clicked(actions) {
                    self.request(cx, Some(decision), true);
                }
            }
            if self.view.button(cx, ids!(next)).clicked(actions)
                && !self.loading
                && !self.rows.is_empty()
            {
                self.selected = (self.selected + 1) % self.rows.len();
                self.render(cx);
            }
        }
        self.view.handle_event(cx, event, scope);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
impl SharedPermissionNoticeRef {
    pub fn set_active(&self, cx: &mut Cx, active: bool) {
        if let Some(mut this) = self.borrow_mut() {
            this.set_active_state(cx, active);
        }
    }
    pub fn set_room(&self, cx: &mut Cx, room: &str, thread: &str) {
        let Some(mut this) = self.borrow_mut() else {
            return;
        };
        let account = PermissionScope::capture();
        if this.account_scope != account || this.room != room || this.thread != thread {
            this.interrupt(cx);
            this.execution_busy = false;
            this.identity.clear();
            this.account_scope = account;
            this.room = room.into();
            this.thread = thread.into();
            this.error.clear();
            this.failure = None;
            this.retry = PermissionRetry::default();
            this.decision_unconfirmed = false;
            this.execution_text.clear();
            this.rows.clear();
            this.selected = 0;
            this.active = true;
            this.sync_animation(cx);
            this.view.set_visible(cx, false);
            this.request(cx, None, false);
        } else if this.active && !this.loading {
            this.request(cx, None, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn shared_execution_notice_tracks_latest_message_and_exact_room_thread() {
        let v=json!({"executions":[
            {"roomId":"!room","threadRoot":"$new","state":"blocked","reason":"budget_blocked"},
            {"roomId":"!room","threadRoot":"$old","state":"unknown"},
            {"roomId":"!room","threadRoot":"$done","state":"replied"}
        ]});
        assert!(execution_notice(&v,"!room","").unwrap().contains("unsettled calls"));
        assert!(execution_notice(&v,"!room","$old").unwrap().contains("interrupted"));
        assert!(execution_notice(&v,"!room","$done").is_none());
        assert!(execution_notice(&v,"!room","$foreign").is_none());
        assert!(execution_notice(&v,"!foreign","").is_none());
        for (state, expected) in [("usage_unresolved","previous call"),("exhausted","used")] {
            let v=json!({"executions":[{"roomId":"!room","state":"blocked","reason":"budget_blocked","budgetState":state}]});
            assert!(execution_notice(&v,"!room","").unwrap().contains(expected));
            assert!(!execution_busy(&v,"!room",""));
        }
    }

    #[test]
    fn shared_activity_animation_stops_on_terminal_permission_and_scope_changes() {
        for state in ["pending","queued","bound","started","reply_pending"] {
            let v=json!({"executions":[{"roomId":"!room","threadRoot":"$thread","state":state}]});
            assert!(execution_busy(&v,"!room","$thread"));
            assert!(!execution_busy(&v,"!room","$other"));
            assert!(!execution_busy(&v,"!other",""));
        }
        for state in ["unknown","failed","blocked","paused","expired","cancelled","replied","reply_unknown"] {
            let v=json!({"executions":[{"roomId":"!room","threadRoot":"$thread","state":state}]});
            assert!(!execution_busy(&v,"!room","$thread"));
        }
        assert!(!execution_busy(&json!({"executions":[{"roomId":"!room","state":"pending","reason":"budget_blocked"}]}),"!room",""));
        // Latest delivered reply must not resurrect an older interrupted run.
        assert!(!execution_busy(&json!({"executions":[{"roomId":"!room","state":"replied"},{"roomId":"!room","state":"started"}]}),"!room",""));
    }

    /// How many timers `f` starts: `Cx` hands out timer ids in sequence.
    pub(super) fn timers_started(cx:&mut Cx,f:impl FnOnce(&mut Cx))->u64 {
        let probe=|cx:&mut Cx|{let t=cx.start_timeout(60.0);cx.stop_timer(t);t.0};
        let before=probe(cx);f(cx);probe(cx)-before-1
    }

    #[test]
    fn native_activity_spinner_shows_only_while_busy_and_starts_no_redraw_clock() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let widget=cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            vm.bx.captured_errors=Some(Vec::new());
            let value=script_eval!(vm,{mod.widgets.SharedPermissionNotice {}});
            let widget=WidgetRef::script_from_value(vm,value);
            widget.register_as_ui_root(vm);
            assert!(vm.take_errors().is_empty());
            widget.as_shared_permission_notice()
        });
        let mut inner=widget.borrow_mut().unwrap();
        // (step, change, card shown, spinner shown)
        let steps:[(&str,fn(&mut SharedPermissionNotice),bool,bool);7]=[
            ("agent working",|n|{n.active=true;n.execution_busy=true;n.execution_text="Agent is working…".into();},true,true),
            ("permission requested",|n|n.rows.push(json!({"approval":{},"decision":null})),true,false),
            ("request answered",|n|n.rows.clear(),true,true),
            ("offline",|n|n.error="offline".into(),true,false),
            ("room deselected",|n|{n.error.clear();n.active=false;},true,false),
            ("room selected again",|n|n.active=true,true,true),
            ("agent finished",|n|{n.execution_busy=false;n.execution_text.clear();},false,false),
        ];
        for (step,change,card,spinner) in steps {
            change(&mut inner);
            let started=timers_started(&mut cx,|cx|inner.render(cx));
            assert_eq!(inner.view.visible(),card,"{step}: card");
            assert_eq!(inner.view.widget(&mut cx,ids!(activity_spinner)).visible(),spinner,"{step}: spinner");
            // The notice is the timeline's last row. Rendering the status must
            // not start a clock that redraws every visible message on each tick.
            assert_eq!(started,0,"{step}: the notice must not start its own redraw clock");
        }
    }
}

#[cfg(test)]
#[path = "permission_recovery_tests.rs"]
mod recovery_tests;
