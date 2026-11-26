//! Permission cards read through authenticated Hagency management, never chat text.
use makepad_widgets::*;
use serde_json::Value;
use std::time::{Duration, Instant};
use super::permission_retry::PermissionRetry;
use crate::hagency::backend::Service;
use crate::hagency::permission_failure::{PermissionFailure, Recovery};

/// The account session a read belongs to. An answer from another one is dropped.
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
struct PermissionResult {scope:PermissionScope,identity:String,room:String,thread:String,nonce:u64,value:Result<Value,PermissionFailure>}
#[derive(Default)]
struct RoomAllowances {
    supported: bool,
    requested: bool,
    cursor: String,
    next: Option<String>,
    checked: Option<std::time::Instant>,
    text: String,
}
impl RoomAllowances {
    fn request(&mut self) -> Option<String> {
        self.requested=self.supported && self.checked.is_none_or(|t| t.elapsed()>=std::time::Duration::from_secs(30));
        self.requested.then(|| self.cursor.clone())
    }
    fn receive(&mut self, value:&Value) {
        if value["roomAllowanceSupported"]!=true { *self=Self::default(); return; }
        self.supported=true;
        if !self.requested { return; }
        self.requested=false;
        self.checked=Some(std::time::Instant::now());
        let page=&value["roomAllowancePage"];
        self.next=page["next"].as_str().filter(|s| !s.is_empty() && s.len()<=255).map(str::to_owned);
        let rows=page["items"].as_array().into_iter().flatten();
        self.text=rows.filter_map(|r|{
            let agent=r["agentMxid"].as_str().filter(|id| id.starts_with('@') && id.len()<=255 && !id.chars().any(char::is_control))?;
            let msg=match r["state"].as_str()? {
                "exhausted"=>"{agent}: allowance used up.",
                "usage_unresolved"=>"{agent}: waiting for usage to settle.",
                "renewal_waiting_capacity"=>"{agent}: waiting for pool capacity to renew.",
                "unavailable"=>"{agent}: allowance currently unavailable.",
                _=>return None,
            };
            Some(crate::i18n::format(msg,&[("agent",agent.to_owned())]))
        }).take(8).collect::<Vec<_>>().join("\n");
        if self.next.is_none() && self.text.is_empty() { self.cursor.clear(); }
    }
    fn turn_page(&mut self) {
        self.cursor=self.next.take().unwrap_or_default();
        self.text.clear();
        self.checked=None;
    }
}
fn execution_row<'a>(value:&'a Value,room:&str,thread:&str)->Option<&'a Value> {
    value["executions"].as_array()?.iter().find(|r|
        r["roomId"]==room && (thread.is_empty() || r["threadRoot"]==thread))
}
/// Whether the server that wrote this row describes each Agent of a message:
/// it names the Agent and says whether the message can be sent again.
fn per_agent(row:&Value)->bool {row["canSendAgain"].is_boolean()}
/// The newest message of this room or thread has one row for each Agent it
/// was addressed to. A server that does not describe each Agent is read as
/// it always was: by its first row.
fn execution_rows<'a>(value:&'a Value,room:&str,thread:&str)->Vec<&'a Value> {
    let Some(first)=execution_row(value,room,thread) else {return Vec::new()};
    let Some(event)=first["eventId"].as_str().filter(|_| per_agent(first)) else {return vec![first]};
    value["executions"].as_array().into_iter().flatten().filter(|r|
        r["roomId"]==room && (thread.is_empty() || r["threadRoot"]==thread) && r["eventId"]==event).collect()
}
/// The Agent a row is about, when the server names it.
fn row_agent(row:&Value)->Option<&str> {
    row["agentMxid"].as_str().filter(|id| id.starts_with('@') && id.len()<=255 && !id.chars().any(char::is_control))
}
/// The message ended for this Agent and was not processed. A message that
/// is still waiting (for the owner's service, for allowance) and a run whose
/// outcome is unknown are not such an end.
fn row_unprocessed(row:&Value)->bool {
    match row["state"].as_str() {
        Some("cancelled"|"expired")=>true,
        // The Agent's process changed under the message, or every start
        // failed: the message is lost to it. A server that does not describe
        // each Agent keeps the text this state always had.
        Some("blocked")=>per_agent(row) && row["reason"]=="head_changed",
        _=>false,
    }
}
/// One line of the notice: what happened, and for which Agent when the line
/// would otherwise not say.
#[derive(Clone,Debug,Default,PartialEq)]
struct ExecutionLine {
    /// The Agent's Matrix ID.
    agent:Option<String>,
    /// The Agent's name in this room, once it is known.
    name:Option<String>,
    text:&'static str,
}
impl ExecutionLine {
    fn shown(&self)->String {
        use crate::i18n::{format,tr};
        match self.name.as_deref().or(self.agent.as_deref()) {
            Some(agent) if self.text==UNPROCESSED=>format(UNPROCESSED_BY,&[("agent",agent.to_owned())]),
            Some(agent)=>format("{agent}: {status}",&[("agent",agent.to_owned()),("status",tr(self.text).to_owned())]),
            None=>tr(self.text).to_owned(),
        }
    }
}
const UNPROCESSED:&str="This message was not processed; it will not retry automatically.";
const UNPROCESSED_BY:&str="{agent} did not process this message. It will not retry automatically.";
/// The sender's way out of a message that ended without a turn: the same
/// message, sent again to the Agents that did not process it. The server
/// decides for which Agents that is: it knows that nothing of the message
/// reached a model or a tool, that the reader sent it, and that the Agent is
/// in the room and not stopped.
#[derive(Clone,Debug,PartialEq)]
struct Resend {
    event:String,
    /// The thread the message was sent in, when it was.
    thread:Option<String>,
    agents:Vec<String>,
}
fn execution_resend(value:&Value,room:&str,thread:&str)->Option<Resend> {
    let rows=execution_rows(value,room,thread);
    let event=rows.first()?["eventId"].as_str()?;
    let agents=rows.iter().filter(|r| r["canSendAgain"]==true)
        .filter_map(|r| row_agent(r)).map(str::to_owned).collect::<Vec<_>>();
    if agents.is_empty() {return None;}
    let root=rows[0]["threadRoot"].as_str().filter(|root| *root!=event);
    Some(Resend {event:event.to_owned(),thread:root.map(str::to_owned),agents})
}
/// Whether this client has already sent `event` of `room` again.
fn sent_again(room:&str,event:&str)->bool {
    use matrix_sdk::ruma::{EventId,RoomId};
    matches!((<&RoomId>::try_from(room),<&EventId>::try_from(event)),(Ok(room),Ok(event)) if crate::sliding_sync::sent_again(room,event))
}
fn pool_request(row: &Value) -> Option<(String, String)> {
    if row["reason"] != "budget_blocked" { return None; }
    let link = &row["resourceRecovery"];
    let id = link["requestId"].as_str()?;
    let role = link["role"].as_str()?;
    if id.is_empty() || id.len() > 256 || !matches!(role, "manager" | "owner") { return None; }
    Some((id.to_owned(), role.to_owned()))
}
fn execution_busy(value:&Value,room:&str,thread:&str)->bool {
    execution_rows(value,room,thread).into_iter().any(|row|
        row["reason"]!="budget_blocked" && matches!(row["state"].as_str(),Some("started"|"pending"|"queued"|"acknowledged"|"bound"|"reply_pending")))
}
/// What the notice says about the newest message: one line for each Agent
/// that has something to say. An Agent is named when the message went to more
/// than one, and when it did not process the message.
/// `stopped` says whether the run card for a message shows its run stopped.
fn execution_lines(value:&Value,room:&str,thread:&str,stopped:impl Fn(&str)->bool)->Vec<ExecutionLine> {
    let rows=execution_rows(value,room,thread);
    let several=rows.len()>1;
    rows.into_iter().filter_map(|row| {
        let text=row_notice(row,&stopped)?;
        let agent=row_agent(row).filter(|_| several || text==UNPROCESSED).map(str::to_owned);
        Some(ExecutionLine {agent,name:None,text})
    }).collect()
}
#[cfg(test)]
fn execution_notice(value:&Value,room:&str,thread:&str,stopped:impl Fn(&str)->bool)->Option<&'static str> {
    row_notice(execution_row(value,room,thread)?,&stopped)
}
fn row_notice(row:&Value,stopped:&impl Fn(&str)->bool)->Option<&'static str> {
    if row["reason"]=="owner_daemon_update_required" {
        return Some("Message has not started. The resource owner needs to update Hagency and start its service; the same message can then continue.");
    }
    if row["reason"]=="owner_daemon_unavailable" {
        return Some("Message has not started. Waiting for the resource owner's Hagency service to reconnect.");
    }
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
        "unknown"=>Some("Run outcome is unknown. External actions or charges may already exist. Review the original message and task records before starting a separate request."),
        // The Agent records a stopped run as failed; its run card tells them apart.
        "failed" if row["eventId"].as_str().is_some_and(stopped)=>Some("Run stopped."),
        "failed"=>Some("Agent run failed."),
        "blocked" if row_unprocessed(row)=>Some(UNPROCESSED),
        "blocked"=>Some("Message could not start. Check the resource and Agent connection."),
        "paused"=>Some("Agent is paused."),
        "cancelled"|"expired"=>Some(UNPROCESSED),
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
        context := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
        room_allowances := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_INK visible: false}
        allowance_page := Button {text: #(crate::i18n::tr("Other room Agents")) i18n_text: "Other room Agents" visible: false}
        request_details := ScrollYView {width: Fill height: Fit{max: FitBound.Abs(130.0)}
            details := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_INK}
        }
        hint := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
        actions := View {width: Fill height: Fit flow: Down spacing: 4
            once := Button {width: Fill text: #(crate::i18n::tr("Approve once")) i18n_text: "Approve once"}
            session := Button {width: Fill text: #(crate::i18n::tr("Always approve for this session")) i18n_text: "Always approve for this session"}
            deny := Button {width: Fill text: #(crate::i18n::tr("Deny")) i18n_text: "Deny"}
        }
        execution_review := Button {visible:false width:Fill text:#(crate::i18n::tr("Review original task records")) i18n_text:"Review original task records"}
        execution_resend := Button {visible:false width:Fill text:#(crate::i18n::tr("Send again")) i18n_text:"Send again"}
        recovery := View {width: Fill height: Fit flow: Down spacing: 6 visible: false
            recovery_hint := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
            last_checked := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
            refresh := Button {width: Fill text: #(crate::i18n::tr("Refresh status")) i18n_text: "Refresh status"}
            recover := Button {width: Fill text: #(crate::i18n::tr("My Agents")) i18n_text: "My Agents"}
        }
        allowance := Button {text: #(crate::i18n::tr("View original pool request")) i18n_text: "View original pool request" visible: false}
        next := Button {text: #(crate::i18n::tr("Next request")) i18n_text: "Next request" visible: false}
    }
}
/// What the notice shows, worked out from the room's requests and from this
/// client's own decisions (shared with the card on the message itself).
#[derive(Debug,Default,PartialEq)]
struct Display {
    visible:bool,
    /// The spinner beside the title: the Agent works, or a decision is on its way.
    waiting:bool,
    title:String,
    /// Which request this is, after an earlier one; empty hides it.
    context:String,
    /// The command, file or address the request is about; empty hides it.
    details:String,
    hint:String,
    once:bool,
    session:bool,
    deny:bool,
    next:bool,
}
#[derive(Script,ScriptHook,Widget)]
pub struct SharedPermissionNotice {
    #[deref] view:View,
    #[rust] identity:String,
    #[rust] room:String,
    #[rust] thread:String,
    #[rust] error:String,
    /// Why the last read failed, kept with its transport status to choose the way out.
    #[rust] failure:Option<PermissionFailure>,
    /// When the next automatic read is due. Failed reads back off or pause.
    #[rust] retry:PermissionRetry,
    /// The account session this room was bound under.
    #[rust] account_scope:PermissionScope,
    /// The read in flight, stopped when the notice is hidden or bound again.
    #[rust] task:Option<tokio::task::JoinHandle<()>>,
    #[rust] nonce:u64,
    /// A management read is in flight. It never blocks the buttons: a click
    /// during a read is sent at once.
    #[rust] loading:bool,
    #[rust] timer:Timer,
    #[rust] active:bool,
    #[rust] execution_thread_filter:bool,
    /// This room or thread has reported Agent work during this visit.
    #[rust] tracked:bool,
    #[rust] rows:Vec<Value>,
    #[rust] selected:usize,
    #[rust] room_allowances:RoomAllowances,
    #[rust] original_request:Option<(String,String)>,
    /// The run's row of the last read, for the review of a run whose outcome is unknown.
    #[rust] execution_source:Option<Value>,
    #[rust] execution_lines:Vec<ExecutionLine>,
    /// The newest message ended without a turn for some Agents and its sender
    /// is looking: what "Send again" would send.
    #[rust] resend:Option<Resend>,
    #[rust] execution_busy:bool,
    #[rust] waiting:bool,
    /// Request ids shown in this room or thread, in order, to number them.
    #[rust] seen:Vec<String>,
    /// The last request this client decided here, and how.
    #[rust] last_decision:Option<(String,String)>,
}
impl SharedPermissionNotice {
    /// Send the newest message again to the Agents that did not process it.
    /// It goes out as a new message of the same account, so it is taken in
    /// and counted like any other; the first one stays as it ended.
    fn send_again(&mut self,cx:&mut Cx,resend:Resend) {
        use matrix_sdk::ruma::{OwnedEventId,OwnedRoomId,OwnedUserId};
        let parsed=(||{
            let room=OwnedRoomId::try_from(self.room.as_str()).ok()?;
            let event=OwnedEventId::try_from(resend.event.as_str()).ok()?;
            let thread=match resend.thread.as_deref() {
                Some(root)=>Some(OwnedEventId::try_from(root).ok()?),
                None=>None,
            };
            let agents=resend.agents.iter().map(|a|OwnedUserId::try_from(a.as_str()).ok()).collect::<Option<Vec<_>>>()?;
            Some((room,event,thread,agents))
        })();
        let Some((room,event,thread,agents))=parsed else {
            crate::shared::popup_list::enqueue_popup_notification(crate::i18n::tr("Could not send the message again."),crate::shared::popup_list::PopupKind::Error,Some(5.0));
            self.resend=Some(resend);
            return;
        };
        // One click sends once, also after the room was left and opened again.
        crate::sliding_sync::note_sent_again(&room,&event);
        crate::sliding_sync::submit_async_request(crate::sliding_sync::MatrixRequest::SendAgainToAgents {room_id:room,event_id:event,thread_root:thread,agents});
        self.render(cx);
        // The new message has rows of its own: read them soon.
        if self.active {self.retry.sooner(Instant::now()+Duration::from_millis(500));self.arm_timer(cx);}
    }
    /// Shows the spinner while the Agent works or a decision is on its way.
    /// The spinner updates its retained draw call without re-running the draw
    /// of the timeline that contains it.
    fn sync_animation(&mut self,cx:&mut Cx) {
        self.view.widget(cx,ids!(activity_spinner)).set_visible(cx,self.active && self.waiting);
    }
    /// This room or thread has something that depends on the status.
    fn concerned(&self)->bool {
        self.tracked || self.request_waiting()
    }
    /// A request is waiting in this room or thread: one read here, or one the
    /// account-wide list knows.
    fn request_waiting(&self)->bool {
        !self.rows.is_empty()
            || !self.identity.is_empty()
                && crate::hagency::pending_approvals::has_pending_in_scope(&self.identity,&self.room,&self.thread)
    }
    /// A failed read is shown only where something depends on it: this room or
    /// thread reported Agent work during this visit, or a request is waiting.
    /// A server without Agent management is mentioned only for a waiting request.
    fn shows_error(&self)->bool {
        !self.error.is_empty() && match self.failure.as_ref().map(PermissionFailure::recovery) {
            Some(Recovery::ManagementNotConfigured)=>self.request_waiting(),
            _=>self.concerned(),
        }
    }
    fn display(&mut self)->Display {
        use crate::hagency::pending_approvals::{self as approvals,LocalDecision};
        use crate::i18n::{format,tr};
        let shows_error=self.shows_error();
        let Some(row)=self.rows.get(self.selected) else {
            // No request: the run's state, and what was last decided here.
            let hint=match &self.last_decision {
                Some((_,decision)) if self.execution_busy && self.error.is_empty() =>
                    format("Last request: {result}. The Agent is continuing.",&[("result",tr(approvals::decided_label(decision)).to_owned())]),
                _=>String::new(),
            };
            return Display {
                visible:!self.execution_lines.is_empty() || shows_error,
                waiting:self.execution_busy && self.error.is_empty(),
                title:if shows_error{tr(&self.error).to_owned()}
                    else{self.execution_lines.iter().map(ExecutionLine::shown).collect::<Vec<_>>().join("\n")},
                hint,
                ..Display::default()
            };
        };
        let a=&row["approval"];
        let id=a["id"].as_str().unwrap_or_default().to_owned();
        if !self.seen.contains(&id) {self.seen.push(id.clone());}
        let number=self.seen.iter().position(|seen|*seen==id).map_or(1,|i|i+1);
        let local=approvals::local_decision(&id);
        let (asked,details)=approvals::describe(row);
        let recorded=row["decision"].as_str();
        let open=recorded.is_none() && matches!(local,LocalDecision::None|LocalDecision::Failed(..));
        let can=open && row["canDecide"]==true && row["encryptedPending"]!=true && self.error.is_empty();
        // Once answered, the title says so at once; the Agent's next request
        // then replaces it and is marked as a new one.
        let title=match (&local,recorded) {
            (LocalDecision::Sending(decision),_)=>format("Sending: {decision}…",&[("decision",tr(approvals::decision_label(decision)).to_owned())]),
            (LocalDecision::Sent(decision),_)=>tr(approvals::sent_status(decision)).to_owned(),
            (_,Some(decision))=>tr(approvals::sent_status(decision)).to_owned(),
            _=>asked,
        };
        let mut context=Vec::new();
        match self.last_decision.as_ref().filter(|(previous,_)|*previous!=id) {
            Some((_,decision))=>context.push(format("Previous request: {result}. This is a new request ({n}).",
                &[("result",tr(approvals::decided_label(decision)).to_owned()),("n",number.to_string())])),
            None if number>1=>context.push(format("Request {n}",&[("n",number.to_string())])),
            None=>{},
        }
        // When the owner decides for someone else, say who asked.
        if let Some(requester)=a["requesterMxid"].as_str() && a["deciderMxid"].as_str()!=Some(requester) {
            context.push(format("Requested by {requester}",&[("requester",requester.to_owned())]));
        }
        let hint=if shows_error {tr(&self.error).to_owned()}
            else if matches!(local,LocalDecision::Failed(..)) {tr("Could not send the decision. Try again.").to_owned()}
            else if !open {String::new()}
            else if row["encryptedPending"]==true {tr("Approval becomes available after the encrypted details arrive.").to_owned()}
            else if row["canDecide"]!=true {format("Waiting for resource owner {owner}",&[("owner",a["ownerMxid"].as_str().unwrap_or("").to_owned())])}
            else if matches!(a["request"]["method"].as_str(),Some("claude/can_use_tool"|"octos/can_use_tool")) {tr("Each approval covers this one tool call.").to_owned()}
            else {tr("Session approval covers this Codex session only. It ends when this run stops.").to_owned()};
        Display {
            visible:true,
            waiting:!open && !matches!(local,LocalDecision::Failed(..)),
            title,
            context:context.join("\n"),
            details,
            hint,
            once:can,
            session:can && a["request"]["sessionAllowed"]==true,
            deny:can,
            next:self.rows.len()>1,
        }
    }
    /// The way out of a failed read: when the next automatic read is due, a
    /// manual refresh, and where to go when waiting cannot fix it.
    fn render_recovery(&mut self,cx:&mut Cx) {
        use crate::i18n::{format,tr};
        let recovery=self.failure.as_ref().map(PermissionFailure::recovery);
        self.view.widget(cx,ids!(recovery)).set_visible(cx,self.shows_error());
        self.view.button(cx,ids!(refresh)).set_enabled(cx,self.active && !self.loading);
        let destination=matches!(recovery,Some(Recovery::SignIn|Recovery::Access|Recovery::Service|Recovery::Unsupported|Recovery::Manual));
        self.view.widget(cx,ids!(recover)).set_visible(cx,destination);
        self.view.button(cx,ids!(recover)).set_enabled(cx,self.active && !self.loading);
        self.view.button(cx,ids!(recover)).set_text(cx,tr(if recovery==Some(Recovery::SignIn){"Sign in again"}else{"My Agents"}));
        let hint=if self.loading {tr("Checking…").to_owned()}
            else if let Some(at)=self.retry.next {
                let seconds=at.saturating_duration_since(Instant::now()).as_secs_f64().ceil().max(1.0);
                format("Automatic status retry after {seconds} seconds.",&[("seconds",seconds.to_string())])
            }
            else {tr("Automatic retries are paused. Refresh status after addressing the problem.").to_owned()};
        self.view.label(cx,ids!(recovery_hint)).set_text(cx,&hint);
        let checked=self.retry.last_success.map(|at|chrono::DateTime::<chrono::Utc>::from(at).format("%Y-%m-%d %H:%M:%S").to_string());
        self.view.widget(cx,ids!(last_checked)).set_visible(cx,checked.is_some());
        if let Some(checked)=checked {
            self.view.label(cx,ids!(last_checked)).set_text(cx,&std::format!("{}: {}",tr("Last checked (UTC)"),checked));
        }
    }
    /// Look up the room names of the Agents the lines are about. A name not
    /// yet known is asked for, and the next render has it.
    fn name_agents(&mut self,cx:&mut Cx) {
        use crate::profile::user_profile_cache::get_user_display_name_for_room;
        let Ok(room)=matrix_sdk::ruma::OwnedRoomId::try_from(self.room.as_str()) else {return};
        for line in &mut self.execution_lines {
            let Some(Ok(agent))=line.agent.as_deref().map(matrix_sdk::ruma::OwnedUserId::try_from) else {continue};
            if let Some(name)=get_user_display_name_for_room(cx,agent,Some(&room),true).into_option().filter(|n|!n.trim().is_empty()) {
                line.name=Some(name);
            }
        }
    }
    fn render(&mut self,cx:&mut Cx) {
        self.name_agents(cx);
        let shown=self.display();
        self.waiting=shown.waiting;
        self.sync_animation(cx);
        self.render_recovery(cx);
        let review=self.execution_source.as_ref().is_some_and(|r|matches!(r["state"].as_str(),Some("unknown"|"reply_unknown")) && r["eventId"].as_str().is_some() && r["agentId"].as_str().is_some());
        self.view.widget(cx,ids!(execution_review)).set_visible(cx,review);
        self.view.button(cx,ids!(execution_review)).set_enabled(cx,self.active && !self.loading);
        // Offered with the run's state only: a waiting request or a failed
        // read has the notice, and the state it would act on is not shown.
        let resend=self.resend.is_some() && self.rows.is_empty() && !self.shows_error();
        self.view.widget(cx,ids!(execution_resend)).set_visible(cx,resend);
        self.view.button(cx,ids!(execution_resend)).set_enabled(cx,self.active);
        let notices=&self.room_allowances;
        let has_page=notices.next.is_some() || !notices.cursor.is_empty();
        self.view.set_visible(cx,shown.visible || !notices.text.is_empty() || has_page);
        self.view.label(cx,ids!(room_allowances)).set_text(cx,&notices.text);
        self.view.widget(cx,ids!(room_allowances)).set_visible(cx,!notices.text.is_empty());
        self.view.widget(cx,ids!(allowance_page)).set_visible(cx,has_page);
        self.view.label(cx,ids!(title)).set_text(cx,&shown.title);
        for (id,text) in [(ids!(context),&shown.context),(ids!(details),&shown.details),(ids!(hint),&shown.hint)] {
            self.view.label(cx,id).set_text(cx,text);
            self.view.widget(cx,id).set_visible(cx,!text.is_empty());
        }
        self.view.widget(cx,ids!(request_details)).set_visible(cx,!shown.details.is_empty());
        // Buttons that cannot be used are hidden, not merely disabled: the two
        // look alike here.
        for (id,offered) in [(ids!(once),shown.once),(ids!(session),shown.session),(ids!(deny),shown.deny)] {
            let button=self.view.button(cx,id);
            if offered && !self.view.widget(cx,id).visible() {button.reset_hover(cx);}
            button.set_visible(cx,offered);
            button.set_enabled(cx,offered);
        }
        self.view.widget(cx,ids!(actions)).set_visible(cx,shown.once || shown.session || shown.deny);
        self.view.widget(cx,ids!(allowance)).set_visible(cx,self.original_request.is_some() && self.rows.is_empty() && self.error.is_empty());
        self.view.widget(cx,ids!(next)).set_visible(cx,shown.next);
        self.view.redraw(cx);
    }
    /// Take in one read of the room's requests and the run's state. `stopped`
    /// says whether a message's run card shows its run stopped.
    fn read_status(&mut self,value:&Value,stopped:impl Fn(&str)->bool) {
        self.error.clear();
        self.failure=None;
        if self.active {self.room_allowances.receive(value);}
        // The review is of the run whose outcome is unknown, and the request
        // for allowance of the Agent that lacks it, whichever Agent's it is.
        let rows=execution_rows(value,&self.room,&self.thread);
        self.original_request=rows.iter().find_map(|r|pool_request(r));
        self.execution_source=rows.iter().find(|r|matches!(r["state"].as_str(),Some("unknown"|"reply_unknown")))
            .or(rows.first()).map(|r|(*r).clone());
        self.execution_thread_filter = value["executionThreadFilter"] == true;
        self.execution_busy=execution_busy(value,&self.room,&self.thread);
        // The run the last decision belonged to has ended: a later run must
        // not show it as its own last or previous request.
        if !self.execution_busy {self.last_decision=None;}
        let mut lines=execution_lines(value,&self.room,&self.thread,stopped);
        // A name found earlier stays until the next look-up replaces it.
        for line in &mut lines {
            line.name=self.execution_lines.iter().find(|old|old.agent.is_some() && old.agent==line.agent).and_then(|old|old.name.clone());
        }
        self.execution_lines=lines;
        self.resend=execution_resend(value,&self.room,&self.thread).filter(|r|!sent_again(&self.room,&r.event));
        self.rows=value["approvals"].as_array().cloned().unwrap_or_default().into_iter().filter(|r|r["approval"]["roomId"]==self.room && (self.thread.is_empty() || r["approval"]["threadRoot"]==self.thread)).take(16).collect();
        self.tracked |= execution_row(value,&self.room,&self.thread).is_some()
            || !self.rows.is_empty() || !self.room_allowances.text.is_empty();
        if self.selected>=self.rows.len(){self.selected=0;}
    }
    /// A read failed: what depended on a fresh answer is forgotten, the reason
    /// is kept. Whether it is shown is decided by `shows_error`.
    fn read_failed(&mut self,error:PermissionFailure) {
        self.room_allowances=RoomAllowances::default();
        self.execution_thread_filter=false;
        self.error=error.message().into();
        self.failure=Some(error);
    }
    /// Start the timer for the next automatic read, if one is due.
    fn arm_timer(&mut self,cx:&mut Cx) {
        cx.stop_timer(self.timer);
        self.timer=Timer::default();
        if self.active && let Some(at)=self.retry.next {
            self.timer=cx.start_timeout(at.saturating_duration_since(Instant::now()).as_secs_f64().max(0.05));
        }
    }
    fn failed(&mut self,cx:&mut Cx,error:PermissionFailure) {
        self.loading=false;
        self.retry.failed(error.recovery(),Instant::now(),rand::random::<f64>());
        self.read_failed(error);
        self.render(cx);
        self.arm_timer(cx);
    }
    /// Stop the timer and the read in flight. A read that was cut off is due
    /// again at once; its late answer is dropped.
    fn interrupt(&mut self,cx:&mut Cx) {
        cx.stop_timer(self.timer);
        self.timer=Timer::default();
        self.nonce=self.nonce.wrapping_add(1);
        if let Some(task)=self.task.take(){task.abort();}
        if self.loading{self.retry.interrupted(Instant::now());}
        self.loading=false;
    }

    /// Read the room's requests and the run's state again. An automatic read
    /// waits until it is due; `manual` (a button) reads now.
    fn poll(&mut self,cx:&mut Cx,manual:bool) {
        if self.loading || !self.active || self.room.is_empty() || !self.account_scope.is_current(){return;}
        if !manual && !self.retry.due(Instant::now()) {
            if self.timer.is_empty(){self.arm_timer(cx);}
            return;
        }
        let service=match Service::for_current() {
            Ok(service)=>service,
            Err(code)=>{
                // Nothing here depends on the service yet: wait for it without
                // a warning and read once it is there.
                if !manual && !self.concerned(){return;}
                let error=if crate::sliding_sync::token_expired(){PermissionFailure::from("matrix_authorization_required")}else{code.into()};
                self.failed(cx,error);
                return;
            },
        };
        cx.stop_timer(self.timer);
        self.timer=Timer::default();
        self.identity=service.identity();
        self.loading=true;
        self.retry.begin();
        self.nonce=self.nonce.wrapping_add(1);
        // The recovery rows say that a check is running.
        if manual || self.shows_error(){self.render(cx);}
        let identity=self.identity.clone();let room=self.room.clone();let thread=self.thread.clone();let nonce=self.nonce;
        let scope=self.account_scope;
        let room_allowance_after=self.room_allowances.request();
        let execution_thread = (self.execution_thread_filter && !thread.is_empty()).then(|| thread.clone());
        self.task=Some(crate::account_session::spawn(async move {
            let value=tokio::time::timeout(Duration::from_secs(30),
                service.read_shared_approvals(Some(room.clone()),execution_thread,room_allowance_after,manual))
                .await.unwrap_or_else(|_|Err(PermissionFailure::from("permission_query_timeout")));
            Cx::post_action(PermissionResult{scope,identity,room,thread,nonce,value});
        }));
    }
    /// Take in the answer of the read in flight. An answer for another account
    /// session, room, thread or read is dropped.
    fn accept_result(&mut self,cx:&mut Cx,done:&PermissionResult) {
        if !self.active || !self.loading || !self.account_scope.is_current() || done.scope!=self.account_scope
            || done.identity!=self.identity || done.room!=self.room || done.thread!=self.thread || done.nonce!=self.nonce {return;}
        self.task.take();
        self.loading=false;
        match &done.value {
            // A missing or malformed list is not a fresh empty one.
            Ok(value) if value["approvals"].is_array()=>{
                self.retry.success(Instant::now());
                let room=self.room.clone();
                self.read_status(value,|event|crate::hagency::run_controls::stopped(&room,event));
                // The cards on the messages read the same requests.
                if crate::hagency::pending_approvals::merge_room(&self.identity,&self.room,value,self.active){
                    cx.action(crate::hagency::pending_approvals::PendingApprovalsChanged);
                }
                self.render(cx);
                self.arm_timer(cx);
            },
            Ok(_)=>self.failed(cx,"invalid_permission_status".into()),
            Err(error)=>self.failed(cx,error.clone()),
        }
    }
    fn set_active_state(&mut self,cx:&mut Cx,active:bool) {
        if self.active!=active {
            self.active=active;
            if !active {self.interrupt(cx);}
            else if self.retry.due(Instant::now()) {self.poll(cx,false);}
            else {self.arm_timer(cx);}
            self.sync_animation(cx);
        }
        if !active {self.room_allowances=RoomAllowances::default();self.render(cx);}
    }
    /// Send the decision for the request on screen, through the same store as
    /// the card on the message, so both show it.
    fn decide(&mut self,cx:&mut Cx,decision:&str) {
        if !self.active || !self.error.is_empty() {return;}
        let Some(row)=self.rows.get(self.selected) else {return};
        let id=row["approval"]["id"].as_str().unwrap_or_default().to_owned();
        if crate::hagency::pending_approvals::decide(row,decision) {
            self.last_decision=Some((id,decision.to_owned()));
            cx.action(crate::hagency::pending_approvals::PendingApprovalsChanged);
            self.render(cx);
        }
    }
}
impl Drop for SharedPermissionNotice {
    fn drop(&mut self) {
        if let Some(task)=self.task.take(){task.abort();}
    }
}
impl Widget for SharedPermissionNotice {
    fn handle_event(&mut self,cx:&mut Cx,event:&Event,scope:&mut Scope) {
        use crate::hagency::pending_approvals::{PendingApprovalDecided,PendingApprovalsChanged};
        if !self.account_scope.is_current(){self.interrupt(cx);self.execution_busy=false;self.waiting=false;self.sync_animation(cx);self.room_allowances=RoomAllowances::default();self.view.set_visible(cx,false);self.rows.clear();return;}
        if self.timer.is_event(event).is_some(){self.timer=Timer::default();self.poll(cx,false);}
        if let Event::Actions(actions)=event {
            for action in actions {
                if let Some(done)=action.downcast_ref::<PermissionResult>() {self.accept_result(cx,done);}
                // A decision from either card: show its outcome now, and read
                // again soon for the Agent's next request.
                if let Some(decided)=action.downcast_ref::<PendingApprovalDecided>()
                    && (self.rows.iter().any(|r|r["approval"]["id"]==decided.id.as_str()) || self.last_decision.as_ref().is_some_and(|(id,_)|*id==decided.id)) {
                    self.render(cx);
                    // A failed read keeps its own schedule.
                    if self.active && !self.loading {self.retry.sooner(Instant::now()+Duration::from_millis(500));self.arm_timer(cx);}
                }
                if action.downcast_ref::<PendingApprovalsChanged>().is_some(){self.render(cx);}
                // Nothing went out: read again whether the message can still be sent again.
                if let Some(failed)=action.downcast_ref::<crate::sliding_sync::SendAgainFailed>()
                    && failed.room_id.as_str()==self.room && self.active && !self.loading {
                    self.retry.sooner(Instant::now());self.arm_timer(cx);
                }
            }
            for (id,decision) in [(ids!(once),"approve_once"),(ids!(session),"approve_session"),(ids!(deny),"deny")] {
                if self.view.button(cx,id).clicked(actions){self.decide(cx,decision);}
            }
            if self.view.button(cx,ids!(refresh)).clicked(actions){self.poll(cx,true);}
            if self.view.button(cx,ids!(execution_review)).clicked(actions) && !self.loading
                && let (Some(row),Some(client))=(self.execution_source.as_ref(),crate::sliding_sync::get_client())
                && let (Ok(room),Some(event),Some(agent),Some(session))=(self.room.parse(),row["eventId"].as_str().and_then(|e|e.parse().ok()),row["agentId"].as_str(),crate::matrix_context::SessionPin::capture(&client))
                && session.is_current() {
                cx.action(crate::hagency::work_review::WorkReviewAction::Open(crate::hagency::work_review::OriginalWork{session,room,event,agent:agent.into()}));
            }
            if self.view.button(cx,ids!(execution_resend)).clicked(actions) && self.active && self.rows.is_empty() && !self.shows_error()
                && let Some(resend)=self.resend.take() {
                self.send_again(cx,resend);
            }
            if self.view.button(cx,ids!(recover)).clicked(actions) && !self.loading {
                match self.failure.as_ref().map(PermissionFailure::recovery) {
                    Some(Recovery::SignIn)=>cx.action(crate::accounts::AccountAction::Reauthenticate),
                    Some(Recovery::Access|Recovery::Service|Recovery::Unsupported|Recovery::Manual)=>cx.action(crate::hagency::ui::HagencyAction::OpenAgents),
                    _=>{},
                }
            }
            if self.view.button(cx,ids!(allowance_page)).clicked(actions) && self.active && !self.loading {
                self.room_allowances.turn_page();self.render(cx);self.poll(cx,true);
            }
            if self.view.button(cx,ids!(allowance)).clicked(actions) && self.active && self.error.is_empty() && self.rows.is_empty()
                && let Some((request_id,role))=&self.original_request
                && let Some(account)=crate::sliding_sync::current_user_id() {
                cx.action(crate::miniapps::MiniAppsAction::ReviewPoolRequest {
                    owner:account.to_string(), request_id:request_id.clone(), role:role.clone(),
                });
            }
            if self.view.button(cx,ids!(next)).clicked(actions) && !self.rows.is_empty(){self.selected=(self.selected+1)%self.rows.len();self.render(cx);}
        }
        self.view.handle_event(cx,event,scope);
    }
    fn draw_walk(&mut self,cx:&mut Cx2d,scope:&mut Scope,walk:Walk)->DrawStep{self.view.draw_walk(cx,scope,walk)}
}
impl SharedPermissionNoticeRef {
    pub fn set_active(&self,cx:&mut Cx,active:bool){if let Some(mut this)=self.borrow_mut(){this.set_active_state(cx,active);}}
    pub fn set_room(&self,cx:&mut Cx,room:&str,thread:&str){
        let Some(mut this)=self.borrow_mut()else{return};
        let account=PermissionScope::capture();
        if this.account_scope!=account || this.room!=room || this.thread!=thread {
            this.interrupt(cx);this.execution_busy=false;this.waiting=false;this.sync_animation(cx);this.room_allowances=RoomAllowances::default();this.original_request=None;this.execution_source=None;this.execution_thread_filter=false;this.tracked=false;this.identity.clear();this.account_scope=account;this.room=room.into();this.thread=thread.into();this.error.clear();this.failure=None;this.retry=PermissionRetry::default();this.execution_lines.clear();this.resend=None;this.rows.clear();this.selected=0;this.seen.clear();this.last_decision=None;this.active=true;this.view.set_visible(cx,false);this.poll(cx,false);
        }else if this.active && !this.loading {this.poll(cx,false);}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn permission_read_failures_follow_work_in_the_selected_room() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let widget=notice(&mut cx);
        let mut n=widget.borrow_mut().unwrap();
        n.room="!room".into();
        n.read_status(&json!({"executions":[{"roomId":"!other","state":"started"}]}), |_|false);
        n.read_failed("permission_query_timeout".into());
        assert!(!n.display().visible);
        n.read_status(&json!({"executions":[{"roomId":"!room","state":"started"}]}), |_|false);
        n.read_failed("permission_query_timeout".into());
        assert!(!n.error.is_empty() && n.display().visible);
        n.read_status(&json!({"approvals":[]}), |_|false);
        assert!(n.error.is_empty());
    }

    #[test]
    fn room_allowance_updates_are_optional_scoped_and_replaced_on_refresh() {
        let mut state=super::RoomAllowances::default();
        assert!(state.request().is_none());
        state.receive(&serde_json::json!({"roomAllowanceSupported":true}));
        assert_eq!(state.request().as_deref(),Some(""));
        state.receive(&serde_json::json!({"roomAllowanceSupported":true,"roomAllowancePage":{
            "items":[{"agentMxid":"@agent:example","state":"exhausted"}],"next":"next_page"}}));
        assert!(state.text.contains("@agent:example"));
        assert!(state.request().is_none());
        state.turn_page();
        assert_eq!(state.request().as_deref(),Some("next_page"));
        state.receive(&serde_json::json!({"roomAllowanceSupported":true,"roomAllowancePage":{"items":[],"next":null}}));
        assert!(state.text.is_empty() && state.cursor.is_empty());
        state.receive(&serde_json::json!({}));
        assert!(!state.supported && state.next.is_none());
    }
    #[test]
    fn original_pool_request_requires_a_current_block_and_a_known_role() {
        let mut row = serde_json::json!({"reason":"budget_blocked","resourceRecovery":{"requestId":"request_1","role":"manager"}});
        assert_eq!(super::pool_request(&row), Some(("request_1".into(), "manager".into())));
        row["resourceRecovery"]["role"] = "admin".into();
        assert!(super::pool_request(&row).is_none());
        row["resourceRecovery"]["role"] = "owner".into();
        row["reason"] = serde_json::Value::Null;
        assert!(super::pool_request(&row).is_none());
    }

    use super::*;
    use serde_json::json;
    #[test]
    fn shared_protocol_blocked_notice_keeps_the_original_message_and_stops_activity() {
        for (reason, explanation) in [
            ("owner_daemon_update_required", "update Hagency and start its service"),
            ("owner_daemon_unavailable", "service to reconnect"),
        ] {
            let value = json!({"executions":[
                {"roomId":"!other","threadRoot":"$thread","state":"started"},
                {"roomId":"!room","threadRoot":"$thread","eventId":"$original","state":"blocked","reason":reason}
            ]});
            let notice = execution_notice(&value, "!room", "$thread", |_| false).unwrap();
            assert!(notice.starts_with("Message has not started."));
            assert!(notice.contains(explanation));
            assert!(!notice.contains("Send the message again"));
            assert!(!execution_busy(&value, "!room", "$thread"));
            assert!(execution_notice(&value, "!room", "$unrelated", |_| false).is_none());
            assert!(!execution_busy(&value, "!room", "$unrelated"));
        }
        // The same retained event can become eligible when its daemon returns.
        let current = json!({"executions":[{"roomId":"!room","threadRoot":"$thread","eventId":"$original","state":"pending","reason":null}]});
        assert_eq!(execution_notice(&current, "!room", "$thread", |_| false), Some("Message queued; waiting for Agent."));
        assert!(execution_busy(&current, "!room", "$thread"));
    }

    #[test]
    fn shared_execution_notice_tracks_latest_message_and_exact_room_thread() {
        let v=json!({"executions":[
            {"roomId":"!room","threadRoot":"$new","state":"blocked","reason":"budget_blocked"},
            {"roomId":"!room","threadRoot":"$old","state":"unknown"},
            {"roomId":"!room","threadRoot":"$done","state":"replied"}
        ]});
        let never=|_:&str|false;
        assert!(execution_notice(&v,"!room","",never).unwrap().contains("unsettled calls"));
        assert!(execution_notice(&v,"!room","$old",never).unwrap().contains("unknown"));
        assert!(execution_notice(&v,"!room","$done",never).is_none());
        assert!(execution_notice(&v,"!room","$foreign",never).is_none());
        assert!(execution_notice(&v,"!foreign","",never).is_none());
        for (state, expected) in [("usage_unresolved","previous call"),("exhausted","used")] {
            let v=json!({"executions":[{"roomId":"!room","state":"blocked","reason":"budget_blocked","budgetState":state}]});
            assert!(execution_notice(&v,"!room","",never).unwrap().contains(expected));
            assert!(!execution_busy(&v,"!room",""));
        }
    }

    /// The Agent records a stopped run as failed. When the run card for that
    /// message shows it stopped, the person who sent it reads that, not a failure.
    #[test]
    fn a_stopped_run_reads_as_stopped_not_failed() {
        let v=json!({"executions":[{"roomId":"!room","threadRoot":"$thread","eventId":"$message","state":"failed"}]});
        assert_eq!(execution_notice(&v,"!room","$thread",|event|event=="$message"),Some("Run stopped."));
        assert_eq!(execution_notice(&v,"!room","$thread",|event|event=="$other"),Some("Agent run failed."));
        // Without its message, a row cannot be matched to a run card.
        let v=json!({"executions":[{"roomId":"!room","threadRoot":"$thread","state":"failed"}]});
        assert_eq!(execution_notice(&v,"!room","$thread",|_|true),Some("Agent run failed."));
        // Other states keep their own text.
        let v=json!({"executions":[{"roomId":"!room","eventId":"$message","state":"unknown"}]});
        assert!(execution_notice(&v,"!room","",|_|true).unwrap().contains("unknown"));
    }

    /// The last decision belongs to its run. Once that run ends, a later run
    /// shows neither its "last request" hint nor "previous request" context.
    #[test]
    fn the_last_decision_is_forgotten_when_its_run_ends() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let widget=notice(&mut cx);
        let mut n=widget.borrow_mut().unwrap();
        (n.active,n.room)=(true,"!room".into());
        let working=json!({"executions":[{"roomId":"!room","state":"started"}],"approvals":[]});
        n.read_status(&working,|_|false);
        n.last_decision=Some(("sap_1".into(),"approve_once".into()));
        n.read_status(&working,|_|false);
        assert_eq!(n.display().hint,"Last request: Approved once. The Agent is continuing.");
        n.read_status(&json!({"executions":[{"roomId":"!room","state":"replied"}],"approvals":[]}),|_|false);
        assert!(n.last_decision.is_none());
        n.read_status(&working,|_|false);
        let next=n.display();
        assert_eq!(next.title,"Agent is working…");
        assert!(next.hint.is_empty(),"a new run shows no earlier decision: {}",next.hint);
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

    #[test]
    fn unknown_guidance_requires_original_record_review_instead_of_resending() {
        let value=json!({"executions":[{"roomId":"!original:test","eventId":"$original","agentId":"original-agent","state":"unknown"}]});
        let text=execution_notice(&value,"!original:test","",|_|false).unwrap();
        assert!(text.contains("External actions or charges may already exist"));
        assert!(text.contains("original message and task records"));assert!(!text.contains("Send the message again"));
        assert!(!execution_busy(&value,"!original:test",""));
    }

    /// How many timers `f` starts: `Cx` hands out timer ids in sequence.
    pub(super) fn timers_started(cx:&mut Cx,f:impl FnOnce(&mut Cx))->u64 {
        let probe=|cx:&mut Cx|{let t=cx.start_timeout(60.0);cx.stop_timer(t);t.0};
        let before=probe(cx);f(cx);probe(cx)-before-1
    }

    fn notice(cx:&mut Cx)->SharedPermissionNoticeRef {
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            vm.bx.captured_errors=Some(Vec::new());
            let value=script_eval!(vm,{mod.widgets.SharedPermissionNotice {}});
            let widget=WidgetRef::script_from_value(vm,value);
            widget.register_as_ui_root(vm);
            assert!(vm.take_errors().is_empty());
            widget.as_shared_permission_notice()
        })
    }

    /// A row as a server that describes each Agent writes it.
    fn row(event:&str,agent:&str,state:&str,reason:Option<&str>,again:bool)->Value {
        json!({"roomId":"!room:x","eventId":event,"threadRoot":event,"agentMxid":agent,"agentId":agent,"state":state,"reason":reason,"canSendAgain":again})
    }
    /// The same row as a server before that writes it.
    fn old_row(event:&str,state:&str,reason:Option<&str>)->Value {
        json!({"roomId":"!room:x","eventId":event,"threadRoot":event,"agentId":"agt_1","state":state,"reason":reason})
    }
    #[test]
    fn a_message_to_two_agents_says_which_one_did_not_process_it() {
        let value=json!({"executions":[
            row("$two","@first:x","cancelled",Some("never_started"),true),
            row("$two","@second:x","replied",None,false),
            row("$older","@second:x","cancelled",Some("never_started"),true),
        ]});
        let lines=execution_lines(&value,"!room:x","",|_|false);
        // The Agent that answered has nothing to say; the older message is not this one.
        assert_eq!(lines,vec![ExecutionLine{agent:Some("@first:x".into()),name:None,text:UNPROCESSED}]);
        assert!(lines[0].shown().starts_with("@first:x did not process this message."));
        assert!(!execution_busy(&value,"!room:x",""));
        assert_eq!(execution_resend(&value,"!room:x",""),
            Some(Resend{event:"$two".into(),thread:None,agents:vec!["@first:x".into()]}));
        // A name found for the room replaces the Matrix ID.
        let named=ExecutionLine{name:Some("First".into()),..lines[0].clone()};
        assert!(named.shown().starts_with("First did not process this message."));
    }
    #[test]
    fn a_message_is_busy_while_any_of_its_agents_works_and_each_line_is_named() {
        let value=json!({"executions":[
            row("$two","@first:x","cancelled",Some("never_started"),true),
            row("$two","@second:x","started",None,false),
        ]});
        assert!(execution_busy(&value,"!room:x",""));
        let shown=execution_lines(&value,"!room:x","",|_|false).iter().map(ExecutionLine::shown).collect::<Vec<_>>();
        assert_eq!(shown.len(),2);
        assert!(shown[0].starts_with("@first:x did not process this message."));
        assert_eq!(shown[1],"@second:x: Agent is working…");
        // The one that did not process it can be asked again while the other works.
        assert_eq!(execution_resend(&value,"!room:x","").unwrap().agents,vec!["@first:x".to_owned()]);
    }
    #[test]
    fn send_again_is_offered_where_the_server_says_so_and_names_the_agent() {
        let offered=|r:Value|execution_resend(&json!({"executions":[r]}),"!room:x","");
        // The server decides: the same state is offered to its sender and not
        // to the resource's owner, and not while the Agent is stopped.
        for (state,reason) in [("cancelled",Some("never_started")),("expired",None),("blocked",Some("head_changed"))] {
            assert!(offered(row("$e","@a:x",state,reason,true)).is_some(),"{state} {reason:?}");
            assert!(offered(row("$e","@a:x",state,reason,false)).is_none(),"{state} {reason:?}: not offered");
        }
        // Nothing is worked out here from a state alone.
        for (state,reason) in [("cancelled",None),("cancelled",Some("explicit_stop")),("unknown",None),("failed",None),("blocked",None)] {
            assert!(offered(row("$e","@a:x",state,reason,false)).is_none(),"{state} {reason:?}");
        }
        // Without an Agent to address there is nothing to send.
        let mut unnamed=row("$e","@a:x","expired",None,true);
        unnamed["agentMxid"]=Value::Null;
        assert!(offered(unnamed).is_none());
        // A message of a thread goes back to its thread.
        let mut threaded=row("$reply","@a:x","expired",None,true);
        threaded["threadRoot"]=json!("$root");
        assert_eq!(offered(threaded).unwrap().thread.as_deref(),Some("$root"));
    }
    #[test]
    fn a_server_that_does_not_describe_each_agent_is_read_as_before() {
        let text=|rows:Vec<Value>|execution_lines(&json!({"executions":rows}),"!room:x","",|_|false);
        // One unnamed line, from the first row, with the text each state always had.
        assert_eq!(text(vec![old_row("$e","cancelled",None)]),vec![ExecutionLine{agent:None,name:None,text:UNPROCESSED}]);
        assert_eq!(text(vec![old_row("$e","expired",Some("event_expired"))]),vec![ExecutionLine{agent:None,name:None,text:UNPROCESSED}]);
        assert_eq!(text(vec![old_row("$e","blocked",Some("head_changed"))]),
            vec![ExecutionLine{agent:None,name:None,text:"Message could not start. Check the resource and Agent connection."}]);
        let two=vec![old_row("$e","failed",None),old_row("$e","started",None)];
        assert_eq!(text(two.clone()),vec![ExecutionLine{agent:None,name:None,text:"Agent run failed."}]);
        // The second row neither spins nor is offered.
        let value=json!({"executions":two});
        assert!(!execution_busy(&value,"!room:x",""));
        assert_eq!(execution_rows(&value,"!room:x","").len(),1);
        assert!(execution_resend(&json!({"executions":[old_row("$e","cancelled",None)]}),"!room:x","").is_none());
        // A server that describes each Agent says the same state by name.
        assert_eq!(text(vec![row("$e","@a:x","blocked",Some("head_changed"),false)]),
            vec![ExecutionLine{agent:Some("@a:x".into()),name:None,text:UNPROCESSED}]);
    }
    #[test]
    fn the_allowance_request_is_found_on_whichever_agent_lacks_allowance() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let widget=notice(&mut cx);
        let mut n=widget.borrow_mut().unwrap();
        n.room="!room:x".into();
        let mut blocked=row("$e","@second:x","blocked",Some("budget_blocked"),false);
        blocked["resourceRecovery"]=json!({"requestId":"req_1","role":"owner"});
        n.read_status(&json!({"executions":[row("$e","@first:x","replied",None,false),blocked]}),|_|false);
        assert_eq!(n.original_request,Some(("req_1".into(),"owner".into())));
    }
    #[test]
    fn send_again_shows_with_the_run_state_and_sends_one_message_for_one_click() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let widget=notice(&mut cx);
        let mut n=widget.borrow_mut().unwrap();
        n.active=true;n.room="!room:x".into();
        let value=json!({"executions":[row("$one_click","@first:x","cancelled",Some("never_started"),true)]});
        n.read_status(&value,|_|false);
        n.render(&mut cx);
        assert!(n.view.widget(&mut cx,ids!(execution_resend)).visible());
        // A request that waits for an answer has the notice to itself.
        n.rows.push(json!({"approval":{},"decision":null}));
        n.render(&mut cx);
        assert!(!n.view.widget(&mut cx,ids!(execution_resend)).visible());
        n.rows.clear();
        let resend=n.resend.take().unwrap();
        n.send_again(&mut cx,resend);
        assert!(sent_again("!room:x","$one_click"));
        assert!(!n.view.widget(&mut cx,ids!(execution_resend)).visible());
        // The same state read again does not offer the same message twice,
        // and neither does the room when it is left and opened again.
        n.read_status(&value,|_|false);
        n.render(&mut cx);
        assert!(n.resend.is_none() && !n.view.widget(&mut cx,ids!(execution_resend)).visible());
        drop(n);
        widget.set_room(&mut cx,"!other:x","");
        widget.set_room(&mut cx,"!room:x","");
        let mut n=widget.borrow_mut().unwrap();
        n.read_status(&value,|_|false);
        assert!(n.resend.is_none());
        // A later message that ends the same way is offered on its own.
        n.read_status(&json!({"executions":[row("$one_click_later","@first:x","expired",None,true)]}),|_|false);
        assert_eq!(n.resend.as_ref().map(|r|r.event.as_str()),Some("$one_click_later"));
    }

    #[test]
    fn native_activity_spinner_shows_only_while_busy_and_starts_no_redraw_clock() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let widget=notice(&mut cx);
        let mut inner=widget.borrow_mut().unwrap();
        // (step, change, card shown, spinner shown)
        let steps:[(&str,fn(&mut SharedPermissionNotice),bool,bool);7]=[
            ("agent working",|n|{n.active=true;n.execution_busy=true;n.execution_lines=vec![ExecutionLine{text:"Agent is working…",..Default::default()}];},true,true),
            ("permission requested",|n|n.rows.push(json!({"approval":{},"decision":null})),true,false),
            ("request answered",|n|n.rows.clear(),true,true),
            ("offline",|n|n.error="offline".into(),true,false),
            ("room deselected",|n|{n.error.clear();n.active=false;},true,false),
            ("room selected again",|n|n.active=true,true,true),
            ("agent finished",|n|{n.execution_busy=false;n.execution_lines.clear();},false,false),
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

    /// The person who sent the message approves an Agent's tool calls one after
    /// another. Each click must show at once, and the next request must not
    /// look like the one just answered.
    #[test]
    fn a_click_shows_its_decision_at_once_and_the_next_request_reads_as_new() {
        use crate::hagency::pending_approvals::{forget_identity,record_for_test,LocalDecision,TEST_TURN};
        let _turn=TEST_TURN.lock().unwrap_or_else(|e|e.into_inner());
        forget_identity();
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let widget=notice(&mut cx);
        let mut n=widget.borrow_mut().unwrap();
        let request=|id:&str,command:&str| json!({"approval":{"id":id,"agentName":"Claude","requesterMxid":"@e:x","deciderMxid":"@e:x",
            "request":{"method":"claude/can_use_tool","params":{"toolName":"Bash","input":{"command":command,"description":"d"}},"sessionAllowed":false}},
            "digest":"d","decision":null,"canDecide":true});
        n.active=true;n.execution_busy=true;n.execution_lines=vec![ExecutionLine{text:"Agent is working…",..Default::default()}];
        n.rows=vec![request("sap_1","ls -la")];
        let first=n.display();
        assert_eq!((first.title.as_str(),first.details.as_str()),("Claude wants to use Bash","ls -la"));
        assert!(first.once && first.deny && !first.session,"a Claude tool call offers no session approval");
        assert!(first.context.is_empty() && !first.waiting);
        assert_eq!(first.hint,"Each approval covers this one tool call.");
        // A read in flight never takes the buttons away.
        n.loading=true;
        assert!(n.display().once);
        n.loading=false;
        // The click: the card says it is sending and offers no buttons.
        n.last_decision=Some(("sap_1".into(),"approve_once".into()));
        record_for_test("sap_1",LocalDecision::Sending("approve_once".into()));
        let sending=n.display();
        assert_eq!(sending.title,"Sending: Approve once…");
        assert!(!sending.once && !sending.deny && sending.waiting && sending.hint.is_empty());
        record_for_test("sap_1",LocalDecision::Sent("approve_once".into()));
        assert_eq!(n.display().title,"Approved once. Waiting for the Agent to continue.");
        // Used: the Agent runs the tool.
        n.rows.clear();
        let between=n.display();
        assert_eq!(between.title,"Agent is working…");
        assert_eq!(between.hint,"Last request: Approved once. The Agent is continuing.");
        assert!(between.waiting);
        // Its next request is a new one, and says so.
        n.rows=vec![request("sap_2","find . -type f")];
        let second=n.display();
        assert_eq!((second.title.as_str(),second.details.as_str()),("Claude wants to use Bash","find . -type f"));
        assert_eq!(second.context,"Previous request: Approved once. This is a new request (2).");
        assert!(second.once && second.deny && !second.waiting);
        // A send that failed offers the buttons again.
        record_for_test("sap_2",LocalDecision::Failed("deny".into(),"approval_expired".into()));
        let failed=n.display();
        assert!(failed.once && failed.deny && !failed.waiting);
        assert_eq!(failed.hint,"Could not send the decision. Try again.");
        // A request decided on another device shows that, without buttons.
        let mut elsewhere=request("sap_3","pwd");
        elsewhere["decision"]=json!("deny");
        n.rows=vec![elsewhere];
        let decided=n.display();
        assert_eq!(decided.title,"Denied. The Agent will not run this call.");
        assert!(!decided.once && !decided.deny);
        // The owner deciding for someone else sees who asked.
        let mut owners=request("sap_4","pwd");
        owners["approval"]["deciderMxid"]=Value::Null;
        owners["approval"]["requesterMxid"]=json!("@other:x");
        n.rows=vec![owners];
        assert!(n.display().context.ends_with("Requested by @other:x"));
        drop(n);
        forget_identity();
    }
}

#[cfg(test)]
#[path = "permission_recovery_tests.rs"]
mod recovery_tests;
