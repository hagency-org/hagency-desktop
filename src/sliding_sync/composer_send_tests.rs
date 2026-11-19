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
    let Ok(mode) = std::env::var(CHILD) else {
        use std::process::{Command, Stdio};
        let mut modes = vec!["plain", "encrypted", "blocked"];
        if cfg!(target_os = "macos") {
            modes.extend(["dev-startup", "dev-disconnected"]);
        }
        if cfg!(feature = "tsp") {
            modes.push("signed");
        }
        for mode in modes {
            let profile = tempfile::tempdir().unwrap();
            let delivered = profile.path().join("delivered");
            let mut command = Command::new(std::env::current_exe().unwrap());
            if mode.starts_with("dev-") {
                command.env("MAKEPAD_DEV_LAUNCH_DIR", profile.path());
            }
            // The loopback server is test infrastructure and has its own HTTP request logger.
            let filter = if mode == "blocked" || mode.starts_with("dev-") {
                "info"
            } else {
                "trace,wiremock=off"
            };
            let mut child = command.args(["--exact", "sliding_sync::composer_send_tests::click_and_return_post_one_event_each_without_logging_the_body", "--nocapture"])
                .env(CHILD, mode)
                .env("RUST_LOG", filter)
                .env("HAGENCY_DESKTOP_DATA_DIR", profile.path())
                .stdout(Stdio::piped()).stderr(Stdio::piped())
                .spawn().unwrap();
            if mode == "blocked" || mode.starts_with("dev-") {
                // Model the development launcher losing both readers. Start draining
                // only after the homeserver has received both events.
                let deadline = Instant::now() + Duration::from_secs(60);
                while !delivered.exists() && child.try_wait().unwrap().is_none() {
                    if mode == "dev-disconnected"
                        && child.stdout.is_some()
                        && profile.path().join("ready").exists()
                    {
                        drop(child.stdout.take());
                        drop(child.stderr.take());
                        std::fs::write(profile.path().join("disconnected"), b"ok").unwrap();
                    }
                    if Instant::now() >= deadline {
                        child.kill().unwrap();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            let output = child.wait_with_output().unwrap();
            let logs = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                output.status.success(),
                "{mode}: {}",
                logs.lines()
                    .rev()
                    .take(35)
                    .map(|line| line.chars().take(300).collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            assert!(delivered.exists());
            if mode != "blocked" && !mode.starts_with("dev-") {
                assert!(
                    logs.contains("Queued message for"),
                    "send diagnostic was not captured"
                );
            }
            for marker in [CLICK, RETURN] {
                assert!(!logs.contains(marker));
                let bytes = marker.as_bytes();
                // A serialized event or signed envelope can start the body at
                // any byte offset within a Base64 group.
                for offset in 0..3 {
                    let tail = &bytes[offset..];
                    let groups = tail[..tail.len() / 3 * 3].to_vec();
                    let encoded = ruma::serde::Base64::<ruma::serde::base64::Standard>::new(groups);
                    assert!(!logs.contains(&encoded.encode()));
                }
                let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                assert!(!logs.contains(&hex) && !logs.contains(&hex.to_uppercase()));
                for bytes in [
                    format!("{:X?}", marker.as_bytes()),
                    format!("{:?}", marker.as_bytes()),
                ] {
                    assert!(!logs.contains(&bytes[1..bytes.len() - 1]));
                }
            }
        }
        return;
    };

    #[cfg(target_os = "macos")]
    if mode.starts_with("dev-") {
        use std::io::Write;
        crate::macos_dev_output::install();
        if mode == "dev-disconnected" {
            let dir =
                std::path::PathBuf::from(std::env::var_os("HAGENCY_DESKTOP_DATA_DIR").unwrap());
            std::fs::write(dir.join("ready"), b"ok").unwrap();
            while !dir.join("disconnected").exists() {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        // Synchronous writes before UI/tracing initialization, with neither
        // external pipe being read until the homeserver sees both messages.
        let padding = vec![b' '; 2 * 1024 * 1024];
        std::io::stdout().write_all(&padding).unwrap();
        std::io::stderr().write_all(&padding).unwrap();
    }
    Cx::init_log();
    crate::app::init_tracing();
    let encrypted = mode == "encrypted" || mode.starts_with("dev-");
    let send_path = if encrypted {
        "/send/m.room.encrypted/"
    } else {
        "/send/m.room.message/"
    };
    let id = ruma::room_id!("!composer:example.org").to_owned();
    let rt = backend_runtime();
    #[cfg(feature = "tsp")]
    let signing_vid = (mode == "signed").then(|| {
        use tsp_sdk::{AskarSecureStorage, AsyncSecureStore, OwnedVid, SecureStorage, VerifiedVid};
        let vid = OwnedVid::new_did_peer("https://example.invalid".parse().unwrap());
        let db = AsyncSecureStore::new();
        db.add_private_vid(vid.clone(), None).unwrap();
        let profile = std::env::var("HAGENCY_DESKTOP_DATA_DIR").unwrap();
        let vault = rt
            .block_on(AskarSecureStorage::new(
                &format!("sqlite://{profile}/wallet"),
                b"test",
            ))
            .unwrap();
        let mut state = crate::tsp::tsp_state_ref().lock().unwrap();
        state.current_local_vid = Some(vid.identifier().into());
        state.current_wallet = Some(crate::tsp::OpenedTspWallet {
            db,
            vault,
            metadata: Default::default(),
        });
        vid
    });
    let server = rt.block_on(async {
        let server = MockServer::start().await;
        let (client, room) =
            crate::room_encryption::tests::recorded_unencrypted(&server, id.as_str()).await;
        let state = if encrypted {
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"algorithm":"m.megolm.v1.aes-sha2"}))
        } else {
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"errcode":"M_NOT_FOUND","error":"No encryption state"}))
        };
        Mock::given(method("GET")).and(path_regex(".*/state/m.room.encryption/?$")).respond_with(state).mount(&server).await;
        if encrypted {
            Mock::given(method("GET")).and(path_regex(".*/members$"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"chunk":[{
                    "type":"m.room.member","state_key":"@alice:example.org","sender":"@alice:example.org",
                    "event_id":"$alice","origin_server_ts":1,"content":{"membership":"join"}
                }]}))).mount(&server).await;
            Mock::given(method("POST")).and(path_regex(".*/keys/query"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"device_keys":{"@alice:example.org":{}}}))).mount(&server).await;
            Mock::given(method("PUT")).and(path_regex(".*/send/m.room.encrypted/.*"))
                .respond_with(|req: &wiremock::Request| {
                    let content: serde_json::Value = req.body_json().unwrap();
                    assert_eq!(content["algorithm"], "m.megolm.v1.aes-sha2");
                    assert!(content["ciphertext"].as_str().is_some_and(|s| !s.is_empty()));
                    let data = content.to_string();
                    assert!(!data.contains(CLICK) && !data.contains(RETURN));
                    let txn = req.url.path_segments().unwrap().next_back().unwrap();
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"event_id":format!("$event-{txn}")}))
                }).expect(2).mount(&server).await;
        } else {
            for (body, event) in [(CLICK, "$click"), (RETURN, "$return")] {
                Mock::given(method("PUT")).and(path_regex(".*/send/m.room.message/.*"))
                    .and(wiremock::matchers::body_partial_json(serde_json::json!({"body":body})))
                    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"event_id":event})))
                    .expect(1).mount(&server).await;
            }
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
    let ui = cx.with_vm(|vm| {
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
    #[cfg(feature = "tsp")]
    if signing_vid.is_some() {
        ui.check_box(&mut cx, ids!(tsp_sign_checkbox))
            .set_active(&mut cx, true, Animate::No);
    }
    let input = ui.mentionable_text_input(&mut cx, ids!(mentionable_text_input));
    let send = ui.button(&mut cx, ids!(send_message_button));
    let pending = ui.label(&mut cx, ids!(pending_send_notice));
    if mode == "blocked" {
        use std::io::Write;
        let (locked, ready) = std::sync::mpsc::channel();
        let err_locked = locked.clone();
        std::thread::spawn(move || {
            let mut stdout = std::io::stdout().lock();
            locked.send(()).unwrap();
            stdout.write_all(&vec![b' '; 2 * 1024 * 1024]).unwrap();
        });
        std::thread::spawn(move || {
            let mut stderr = std::io::stderr().lock();
            err_locked.send(()).unwrap();
            stderr.write_all(&vec![b' '; 2 * 1024 * 1024]).unwrap();
        });
        ready.recv().unwrap();
        ready.recv().unwrap();
    }

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
                .filter(|r| r.url.path().contains(send_path))
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
    let requests = rt.block_on(server.received_requests()).unwrap();
    let sends: Vec<_> = requests
        .iter()
        .filter(|req| req.url.path().contains(send_path))
        .collect();
    assert_eq!(sends.len(), 2);
    assert_ne!(
        sends[0].url.path(),
        sends[1].url.path(),
        "distinct transaction IDs"
    );
    #[cfg(feature = "tsp")]
    if let Some(vid) = signing_vid {
        for request in &sends {
            let content = request.body_json::<serde_json::Value>().unwrap();
            let signature: ruma::serde::Base64 =
                serde_json::from_value(content["org.robius.tsp_signature"].clone()).unwrap();
            let mut envelope = signature.into_inner();
            let (payload, _) = tsp_sdk::crypto::verify(&vid, &mut envelope).unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(payload).unwrap()["body"],
                content["body"]
            );
        }
    }
    if encrypted {
        assert!(
            !requests
                .iter()
                .any(|req| req.url.path().contains("/send/m.room.message/"))
        );
    }
    let profile = std::path::PathBuf::from(std::env::var_os("HAGENCY_DESKTOP_DATA_DIR").unwrap());
    std::fs::write(profile.join("delivered"), b"ok").unwrap();
    worker.abort();
}
