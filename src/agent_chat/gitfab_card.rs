//! The GitFab task line under a workflow message. The Hagency server marks each GitFab stage message
//! and status notice with `org.hagency.gitfab_stage`; this card reads that metadata (never the message
//! text) and shows which task, stage and state the message belongs to.
use makepad_widgets::*;
use serde_json::Value;

use crate::i18n::Language;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    mod.widgets.GitfabStageCard = #(GitfabStageCard::register_widget(vm)) {
        ..mod.widgets.RoundedView
        visible: false width: Fill height: Fit flow: Down
        margin: Inset{top: 6 right: 10} padding: Inset{left: 10 right: 10 top: 6 bottom: 6}
        draw_bg +: {color: (mod.widgets.RBX_BG_SURFACE) border_radius: 4}
        gitfab_line := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: (mod.widgets.RBX_FG_SECONDARY)}
    }
}

/// The content key the server puts on GitFab messages.
pub const META_KEY: &str = "org.hagency.gitfab_stage";

fn stage_name(language: Language, stage: &str) -> String {
    let english = match stage {
        "spec" => "Spec",
        "plan" => "Plan",
        "execute" => "Execute",
        "test" => "Test",
        "review" => "Review",
        "release" => "Release",
        other => return other.to_owned(),
    };
    crate::i18n::text_for(language, english).to_owned()
}

/// The line for one message's GitFab metadata, or `None` for messages that need none (command
/// replies) and for metadata this client does not understand.
pub fn line_in(language: Language, meta: &Value) -> Option<String> {
    if meta["version"] != 1 {
        return None;
    }
    let task = meta["taskId"].as_str()?.to_owned();
    let int = |v: &Value| v.as_u64().map(|n| n.to_string());
    let fmt = |english: &str, args: &[(&str, String)]| {
        let mut all = vec![("task", task.clone())];
        all.extend(args.iter().cloned());
        crate::i18n::format_in(language, english, &all)
    };
    match meta["kind"].as_str() {
        None => {
            let stage = stage_name(language, meta["stage"].as_str()?);
            let round = int(&meta["round"])?;
            if meta["git"] == true {
                return Some(fmt("GitFab · {task} · {stage}, round {round} · Git platform", &[("stage", stage), ("round", round)]));
            }
            let attempt = int(&meta["attempt"])?;
            Some(fmt(
                "GitFab · {task} · {stage}, round {round}, attempt {attempt}",
                &[("stage", stage), ("round", round), ("attempt", attempt)],
            ))
        }
        Some("status") => {
            let state = &meta["state"];
            match state["kind"].as_str()? {
                "awaiting_spec_approval" => Some(fmt(
                    "GitFab · {task} · waiting for Spec version {revision} to be confirmed",
                    &[("revision", int(&state["specRevision"])?)],
                )),
                "manual_intervention" => Some(fmt(
                    "GitFab · {task} · manual intervention at {stage}, round {round} ({reason})",
                    &[
                        ("stage", stage_name(language, state["slot"]["stage"].as_str()?)),
                        ("round", int(&state["slot"]["round"])?),
                        ("reason", state["reason"].as_str()?.to_owned()),
                    ],
                )),
                "completed" => Some(fmt("GitFab · {task} · completed", &[])),
                "failed" => Some(fmt(
                    "GitFab · {task} · ended ({reason})",
                    &[("reason", state["reason"].as_str()?.to_owned())],
                )),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The line for an event, only when it comes from the Hagency service account of the viewer's own
/// server, which posts GitFab stage messages and status notices. Anyone in the Room can put the
/// same metadata on a message, so a card from any other sender would let members fake a stage or a
/// "completed" run.
fn line_from(
    language: Language,
    sender: &matrix_sdk::ruma::UserId,
    viewer: Option<&matrix_sdk::ruma::UserId>,
    meta: Option<Value>,
) -> Option<String> {
    let viewer = viewer?;
    if !crate::home::mobile_chat_info::is_hagency_service(sender, viewer) {
        return None;
    }
    line_in(language, &meta?)
}

/// The GitFab metadata of an event's original content.
fn meta_of(event: &matrix_sdk_ui::timeline::EventTimelineItem) -> Option<Value> {
    let content = event
        .original_json()?
        .get_field::<Value>("content")
        .ok()
        .flatten()?;
    content.get(META_KEY).cloned()
}

#[derive(Script, ScriptHook, Widget)]
pub struct GitfabStageCard {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
}
impl Widget for GitfabStageCard {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
impl GitfabStageCardRef {
    /// Shows the line for `event`, or hides the card: a recycled message may have shown another's.
    pub fn populate(&self, cx: &mut Cx, event: &matrix_sdk_ui::timeline::EventTimelineItem) {
        let Some(mut inner) = self.borrow_mut() else { return };
        let viewer = crate::sliding_sync::current_user_id();
        let line = line_from(crate::i18n::language(), event.sender(), viewer.as_deref(), meta_of(event));
        inner.view.set_visible(cx, line.is_some());
        inner
            .view
            .label(cx, ids!(gitfab_line))
            .set_text(cx, line.as_deref().unwrap_or_default());
        inner.view.redraw(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use Language::{Chinese, English};

    #[test]
    fn stage_messages_name_their_task_stage_round_and_attempt() {
        let meta = json!({"version":1,"runId":"r","taskId":"DEMO-02-task-1","stage":"plan","round":0,"attempt":2,"specRevision":1});
        assert_eq!(line_in(English, &meta).unwrap(), "GitFab · DEMO-02-task-1 · Plan, round 0, attempt 2");
        assert_eq!(line_in(Chinese, &meta).unwrap(), "GitFab · DEMO-02-task-1 · 计划阶段 · 第 0 轮第 2 次");
        let git = json!({"version":1,"runId":"r","taskId":"T","stage":"execute","round":1,"git":true});
        assert_eq!(line_in(Chinese, &git).unwrap(), "GitFab · T · 执行阶段 · 第 1 轮 · Git 平台");
    }

    #[test]
    fn status_notices_show_where_the_run_waits_or_how_it_ended() {
        let status = |state: Value| json!({"version":1,"kind":"status","runId":"r","taskId":"T","state":state});
        let manual = status(json!({"kind":"manual_intervention","slot":{"stage":"review","round":2,"attempt":1},"specRevision":1,"reason":"rework_limit"}));
        assert_eq!(line_in(English, &manual).unwrap(), "GitFab · T · manual intervention at Review, round 2 (rework_limit)");
        assert_eq!(line_in(Chinese, &manual).unwrap(), "GitFab · T · 评审阶段第 2 轮转人工介入（rework_limit）");
        assert_eq!(line_in(Chinese, &status(json!({"kind":"awaiting_spec_approval","specRevision":3}))).unwrap(), "GitFab · T · 等待确认 Spec 第 3 版");
        assert_eq!(line_in(Chinese, &status(json!({"kind":"completed"}))).unwrap(), "GitFab · T · 已完成");
        assert_eq!(line_in(English, &status(json!({"kind":"failed","reason":"stage_failed"}))).unwrap(), "GitFab · T · ended (stage_failed)");
    }

    #[test]
    fn only_the_hagency_service_of_the_viewers_server_shows_a_card() {
        use matrix_sdk::ruma::user_id;
        let meta = || Some(json!({"version": 1, "kind": "status", "runId": "r", "taskId": "T-1", "state": {"kind": "completed"}}));
        let viewer = user_id!("@alice:river.example");
        assert!(line_from(English, user_id!("@_hagency_service:river.example"), Some(viewer), meta()).is_some());
        for sender in [
            user_id!("@bob:river.example"),
            user_id!("@_hagency_service:evil.example"),
            user_id!("@_hagency_agt_01abc:river.example"),
        ] {
            assert_eq!(line_from(English, sender, Some(viewer), meta()), None, "{sender}");
        }
        assert_eq!(line_from(English, user_id!("@_hagency_service:river.example"), None, meta()), None, "no viewer, no card");
    }

    #[test]
    fn other_or_unknown_metadata_shows_nothing() {
        assert_eq!(line_in(English, &json!({"version":1,"kind":"command_reply","commandEventId":"$e"})), None);
        assert_eq!(line_in(English, &json!({"version":2,"taskId":"T","stage":"plan","round":0,"attempt":1})), None);
        assert_eq!(line_in(English, &json!({"version":1,"stage":"plan"})), None);
        assert_eq!(line_in(English, &json!({"version":1,"kind":"status","taskId":"T","state":{"kind":"running"}})), None);
    }
}
