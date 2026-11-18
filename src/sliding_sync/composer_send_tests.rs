use super::*;
use crate::{
    home::rooms_list::RoomsListRef, room::room_input_bar::RoomInputBarWidgetRefExt,
    shared::mentionable_text_input::MentionableTextInputWidgetRefExt,
};
use makepad_widgets::*;
use std::time::{Duration, Instant};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path_regex},
};

#[test]
fn click_and_return_post_one_event_each_without_logging_the_body() {
    const CLICK: &str = "synthetic-click-private-147";
    const RETURN: &str = "synthetic-return-private-147";
    const CHILD: &str = "RINX_TEST_COMPOSER_CHILD";

    // The request worker and the log handler are process globals. Capture this
    // instance's diagnostic streams separately from other tests.
    if std::env::var_os(CHILD).is_none() {
        let profile = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "sliding_sync::composer_send_tests::click_and_return_post_one_event_each_without_logging_the_body", "--nocapture"])
            .env(CHILD, "1")
            .env("HAGENCY_DESKTOP_DATA_DIR", profile.path())
            .output().unwrap();
        let logs = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{logs}");
        assert!(logs.contains("1 passed"));
        assert!(
            logs.contains("Queued message for"),
            "send diagnostic was not captured"
        );
        assert!(!logs.contains(CLICK) && !logs.contains(RETURN));
        return;
    }

    let id = ruma::room_id!("!composer:example.org").to_owned();
    let rt = backend_runtime();
    let server = rt.block_on(async {
        let server = MockServer::start().await;
        let (client, room) =
            crate::room_encryption::tests::recorded_unencrypted(&server, id.as_str()).await;
        Mock::given(method("GET"))
            .and(path_regex(".*/state/m.room.encryption/?$"))
            .respond_with(ResponseTemplate::new(404).set_body_json(
                serde_json::json!({"errcode":"M_NOT_FOUND","error":"No encryption state"}),
            ))
            .mount(&server)
            .await;
        for (body, event) in [(CLICK, "$click"), (RETURN, "$return")] {
            Mock::given(method("PUT"))
                .and(path_regex(".*/send/m.room.message/.*"))
                .and(wiremock::matchers::body_partial_json(
                    serde_json::json!({"body":body}),
                ))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"event_id":event})),
                )
                .expect(1)
                .mount(&server)
                .await;
        }
        test_set_current_client(Some(client));
        ALL_JOINED_ROOMS.lock().unwrap().insert(
            id.clone(),
            JoinedRoomDetails {
                room_id: id.clone(),
                room,
                main_timeline: None,
                evicted_senders: HashMap::new(),
                thread_timelines: HashMap::new(),
                pending_thread_timelines: HashSet::new(),
                typing_notice_subscriber: None,
                pinned_events_subscriber: None,
                identity_resets_subscriber: None,
                auxiliary_tasks: Vec::new(),
            },
        );
        server
    });
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    *REQUEST_SENDER.lock().unwrap() = Some(tx);
    let mut cx = Cx::new(Box::new(|_, _| {}));
    cx.set_global(RoomsListRef::default());
    let mut ui = cx.with_vm(|vm| {
        makepad_widgets::script_mod(vm);
        crate::app::register_widgets(vm);
        let template = script_eval!(vm, {mod.widgets.RoomInputBar {}});
        WidgetRef::script_from_value(vm, template)
    });
    ui.as_room_input_bar().set_room_context(
        &mut cx,
        ui.widget_uid(),
        TimelineKind::MainRoom { room_id: id },
        None,
    );
    let input = ui.mentionable_text_input(&mut cx, ids!(mentionable_text_input));
    let send = ui.button(&mut cx, ids!(send_message_button));
    let pending = ui.label(&mut cx, ids!(pending_send_notice));
    for (body, click) in [(CLICK, true), (RETURN, false)] {
        input.set_text(&mut cx, body);
        let actions = cx.capture_actions(|cx| {
            if click {
                cx.widget_action(send.widget_uid(), ButtonAction::Clicked(Default::default()));
            } else {
                cx.widget_action(
                    input.text_input_ref().widget_uid(),
                    TextInputAction::Returned(body.into(), Default::default()),
                );
            }
        });
        ui.handle_event(&mut cx, &Event::Actions(actions), &mut Scope::empty());
        assert!(input.text().is_empty());
        assert!(pending.visible());
    }
    let (login, _) = tokio::sync::mpsc::channel(1);
    let (account, _) = tokio::sync::mpsc::unbounded_channel();
    let worker = rt.spawn(matrix_worker_task(rx, login, account));
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        ui.handle_event(&mut cx, &Event::Signal, &mut Scope::empty());
        let requests = rt.block_on(server.received_requests()).unwrap();
        if !pending.visible()
            && requests
                .iter()
                .filter(|r| r.url.path().contains("/send/m.room.message/"))
                .count()
                >= 2
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for composer sends"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    rt.block_on(server.verify());
    worker.abort();
}
