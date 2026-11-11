use super::*;
use serde_json::json;

fn fixture() -> (Cx, SharedPermissionNoticeRef) {
    let mut cx = Cx::new(Box::new(|_, _| {}));
    let widget = cx.with_vm(|vm| {
        makepad_widgets::script_mod(vm);
        crate::app::register_widgets(vm);
        vm.bx.captured_errors = Some(Vec::new());
        let value = script_eval!(vm,{mod.widgets.SharedPermissionNotice {}});
        let widget = WidgetRef::script_from_value(vm, value);
        widget.register_as_ui_root(vm);
        assert!(vm.take_errors().is_empty());
        widget.as_shared_permission_notice()
    });
    {
        let mut n = widget.borrow_mut().unwrap();
        n.active = true;
        n.account_scope = PermissionScope::capture();
        n.identity = "original-owner".into();
        n.room = "!room".into();
        n.thread = "$thread".into();
        n.nonce = 17;
        n.loading = true;
        n.retry.begin();
    }
    (cx, widget)
}
fn response(
    n: &SharedPermissionNotice,
    value: Result<Value, PermissionFailure>,
) -> PermissionResult {
    PermissionResult {
        scope: n.account_scope,
        identity: n.identity.clone(),
        room: n.room.clone(),
        thread: n.thread.clone(),
        nonce: n.nonce,
        deciding: false,
        value,
    }
}
fn pending() -> Value {
    json!({"approval":{"id":"approval-id","roomId":"!room","threadRoot":"$thread","request":{"sessionAllowed":true}},"digest":"a".repeat(64),"canDecide":true,"decision":null})
}
fn enabled(n: &SharedPermissionNotice, cx: &mut Cx) -> bool {
    n.view.button(cx, ids!(once)).borrow().unwrap().enabled()
}

#[test]
fn failed_status_disables_stale_approvals_and_permanent_errors_never_arm_polling() {
    for status in [401, 403, 404, 405, 501] {
        let (mut cx, widget) = fixture();
        {
            let mut n = widget.borrow_mut().unwrap();
            n.rows = vec![pending()];
            n.retry.last_success = Some(std::time::UNIX_EPOCH + Duration::from_secs(123));
            let done = response(
                &n,
                Err(PermissionFailure {
                    status: Some(status),
                    code: "server_error".into(),
                }),
            );
            n.accept_result(&mut cx, &done);
            assert!(!n.loading);
            assert!(!enabled(&n, &mut cx));
            assert!(n.timer.is_empty());
            assert!(n.retry.next.is_none());
            assert!(n.view.widget(&mut cx, ids!(recovery)).visible());
            assert!(n.view.widget(&mut cx, ids!(last_checked)).visible());
            assert!(!n.error.contains("server_error"));
        }
        // The real rebinding path is called repeatedly by room lifecycle events.
        let timers = tests::timers_started(&mut cx, |cx| {
            for _ in 0..100 {
                widget.set_room(cx, "!room", "$thread");
            }
        });
        assert_eq!(
            timers, 0,
            "{status}: rebinding must not bypass a permanent pause"
        );
        let n = widget.borrow();
        let n = n.as_ref().unwrap();
        assert!(!n.loading);
        assert!(n.task.is_none());
    }
}
#[test]
fn transient_failure_deadline_survives_rebind_hide_reopen_and_resets_on_fresh_success() {
    let (mut cx, widget) = fixture();
    let deadline = {
        let mut n = widget.borrow_mut().unwrap();
        let done = response(
            &n,
            Err(PermissionFailure {
                status: Some(503),
                code: "server_error".into(),
            }),
        );
        n.accept_result(&mut cx, &done);
        n.retry.next.unwrap()
    };
    for _ in 0..100 {
        widget.set_room(&mut cx, "!room", "$thread");
    }
    widget.set_active(&mut cx, false);
    assert!(widget.borrow().unwrap().timer.is_empty());
    widget.set_active(&mut cx, true);
    let mut n = widget.borrow_mut().unwrap();
    assert_eq!(n.retry.next, Some(deadline));
    assert!(!n.loading);
    assert!(n.task.is_none());
    n.loading = true;
    n.retry.begin();
    let done = response(&n, Ok(json!({"approvals":[pending()]})));
    n.accept_result(&mut cx, &done);
    assert!(n.error.is_empty());
    assert!(enabled(&n, &mut cx));
    assert!(n.retry.last_success.is_some());
    assert!(n.retry.next.unwrap() <= Instant::now() + Duration::from_secs(2));
}
#[test]
fn late_status_is_rejected_for_hidden_room_thread_account_sdk_nonce_and_identity() {
    for mismatch in 0..7 {
        let (mut cx, widget) = fixture();
        let mut n = widget.borrow_mut().unwrap();
        let mut done = response(&n, Ok(json!({"approvals":[pending()]})));
        match mismatch {
            0 => n.active = false,
            1 => done.room = "!other".into(),
            2 => done.thread = "$other".into(),
            3 => done.scope.epoch = done.scope.epoch.wrapping_add(1),
            4 => done.scope.revision = done.scope.revision.wrapping_add(1),
            5 => done.nonce += 1,
            _ => done.identity = "other-owner".into(),
        }
        n.accept_result(&mut cx, &done);
        assert!(n.rows.is_empty());
        assert!(n.retry.last_success.is_none());
    }
}
#[test]
fn invalid_status_is_not_fresh_empty_success_and_cannot_reenable_decisions() {
    let (mut cx, widget) = fixture();
    let mut n = widget.borrow_mut().unwrap();
    n.rows = vec![pending()];
    let done = response(&n, Ok(json!({"approvals":"invalid"})));
    n.accept_result(&mut cx, &done);
    assert!(!enabled(&n, &mut cx));
    assert!(n.retry.last_success.is_none());
    assert!(n.timer.is_empty());
}
#[test]
fn recovery_buttons_route_to_account_login_or_my_agents_without_starting_a_run() {
    for (status, sign_in) in [(401, true), (403, false), (404, false), (400, false)] {
        let (mut cx, widget) = fixture();
        let uid = {
            let mut n = widget.borrow_mut().unwrap();
            let done = response(
                &n,
                Err(PermissionFailure {
                    status: Some(status),
                    code: "server_error".into(),
                }),
            );
            n.accept_result(&mut cx, &done);
            n.view.button(&mut cx, ids!(recover)).widget_uid()
        };
        let actions = cx.capture_actions(|cx| {
            cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
        });
        let routed = cx.capture_actions(|cx| {
            widget.handle_event(cx, &Event::Actions(actions), &mut Scope::empty())
        });
        if sign_in {
            assert!(routed.iter().any(|a| matches!(
                a.downcast_ref::<crate::accounts::AccountAction>(),
                Some(crate::accounts::AccountAction::Reauthenticate)
            )));
        } else {
            assert!(routed.iter().any(|a| matches!(
                a.downcast_ref::<crate::hagency::ui::HagencyAction>(),
                Some(crate::hagency::ui::HagencyAction::OpenAgents)
            )));
        }
    }
}
#[tokio::test]
async fn hiding_cancels_the_actual_observer_and_rejects_its_pre_hide_result() {
    let (mut cx, widget) = fixture();
    let (entered, started) = tokio::sync::oneshot::channel();
    let (dropped, stopped) = tokio::sync::oneshot::channel();
    struct Original(Option<tokio::sync::oneshot::Sender<()>>);
    impl Drop for Original {
        fn drop(&mut self) {
            let _ = self.0.take().unwrap().send(());
        }
    }
    let done = {
        let mut n = widget.borrow_mut().unwrap();
        let done = response(&n, Ok(json!({"approvals":[pending()]})));
        n.deciding = true;
        n.task = Some(crate::sliding_sync::backend_runtime().spawn(async move {
            let _original = Original(Some(dropped));
            let _ = entered.send(());
            std::future::pending::<()>().await;
        }));
        done
    };
    started.await.unwrap();
    widget.set_active(&mut cx, false);
    tokio::time::timeout(Duration::from_secs(2), stopped)
        .await
        .unwrap()
        .unwrap();
    let mut n = widget.borrow_mut().unwrap();
    assert!(!n.loading);
    assert!(n.task.is_none());
    assert!(n.timer.is_empty());
    assert!(n.decision_unconfirmed);
    assert!(n.retry.due(Instant::now()));
    n.accept_result(&mut cx, &done);
    assert!(n.rows.is_empty());
    assert!(n.retry.last_success.is_none());
}
#[test]
fn decision_failure_only_schedules_status_checks_and_never_infers_success() {
    let (mut cx, widget) = fixture();
    let mut n = widget.borrow_mut().unwrap();
    n.rows = vec![pending()];
    let mut done = response(&n, Err(PermissionFailure::from("permission_query_timeout")));
    done.deciding = true;
    n.accept_result(&mut cx, &done);
    assert!(n.decision_unconfirmed);
    assert!(!enabled(&n, &mut cx));
    assert!(n.retry.next.is_some());
    assert!(n.retry.last_success.is_none());
    assert!(n.rows[0]["decision"].is_null());
    // All automatic requests pass decision=None; this failed decision is not retained as a command.
    assert!(n.task.is_none());
}

#[test]
fn recovery_actions_fit_narrow_chat_panes_and_stale_cards_remain_disabled() {
    use makepad_widgets::makepad_draw::cx_draw::CxDraw;
    let (mut cx, widget) = fixture();
    let mut n = widget.borrow_mut().unwrap();
    n.rows = vec![pending()];
    let done = response(
        &n,
        Err(PermissionFailure {
            status: Some(403),
            code: "access_denied".into(),
        }),
    );
    n.accept_result(&mut cx, &done);
    for width in [280.0, 360.0, 560.0, 900.0] {
        cx.redraw_all();
        let pass = DrawPass::new(&mut cx);
        pass.set_size(&mut cx, dvec2(width, 1000.0));
        let mut list = DrawList2d::new(&mut cx);
        let event = DrawEvent::default();
        let mut draw = CxDraw::new(&mut cx, &event);
        let mut cx2d = Cx2d::new(&mut draw);
        cx2d.begin_pass(&pass, None);
        list.begin_always(&mut cx2d);
        cx2d.begin_root_turtle(dvec2(width, 1000.0), Layout::flow_down());
        n.draw_walk_all(
            &mut cx2d,
            &mut Scope::empty(),
            Walk {
                width: Size::Fixed(width),
                height: Size::fit(),
                ..Walk::default()
            },
        );
        cx2d.end_pass_sized_turtle();
        list.end(&mut cx2d);
        cx2d.end_pass(&pass);
        drop(cx2d);
        drop(draw);
        for id in [ids!(refresh), ids!(recover), ids!(recovery_hint)] {
            let rect = n.view.widget(&mut cx, id).area().rect(&cx);
            assert!(
                rect.size.x > 0.0 && rect.size.y > 0.0,
                "{width}: {id:?}: {rect:?}"
            );
            assert!(
                rect.pos.x >= 0.0 && rect.pos.x + rect.size.x <= width + 1.0,
                "{width}: {id:?}: {rect:?}"
            );
        }
        assert!(!enabled(&n, &mut cx));
    }
}
