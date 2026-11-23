use super::*;
use crate::home::room_screen::{RoomScreenRef, RoomScreenWidgetRefExt};
use makepad_widgets::*;
use serde_json::{json, Value};
use wiremock::{
    matchers::{method, path_regex},
    Mock, MockServer, ResponseTemplate,
};

// These fixtures exercise the process-wide Matrix request queue and Makepad action sender.
fn isolated() -> bool {
    const CHILD: &str = "HAGENCY_ROOM_RESTORE_TEST";
    let thread = std::thread::current();
    let name = thread.name().unwrap();
    if std::env::var(CHILD).as_deref() == Ok(name) {
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(CHILD, name)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

struct Fixture {
    cx: Cx,
    screen: RoomScreenRef,
    client: Client,
    server: MockServer,
}

impl Fixture {
    fn new() -> Self {
        let root = std::rc::Rc::new(std::cell::RefCell::new(WidgetRef::empty()));
        let events = root.clone();
        let mut cx = Cx::new(Box::new(move |cx, event| {
            events.borrow().handle_event(cx, event, &mut Scope::empty());
        }));
        let screen = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            vm.bx.captured_errors = Some(Vec::new());
            let template = script_eval!(vm, { mod.widgets.RoomScreen {} });
            let widget = WidgetRef::script_from_value(vm, template);
            widget.register_as_ui_root(vm);
            assert!(vm.take_errors().is_empty());
            *root.borrow_mut() = widget.clone();
            widget.as_room_screen()
        });
        let (client, server) = backend_runtime().block_on(async {
            let server = MockServer::start().await;
            let client = Client::builder()
                .homeserver_url(server.uri())
                .sliding_sync_version_builder(VersionBuilder::Native)
                .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_11])
                .build()
                .await
                .unwrap();
            client.matrix_auth().restore_session(serde_json::from_value(json!({
                "access_token":"fixture", "user_id":"@alice:example.org", "device_id":"RESTORE"
            })).unwrap(), Default::default()).await.unwrap();
            (client, server)
        });
        test_set_current_client(Some(client.clone()));
        Self {
            cx,
            screen,
            client,
            server,
        }
    }

    fn open(&mut self, id: &OwnedRoomId) {
        self.screen.set_displayed_room(
            &mut self.cx,
            &crate::utils::RoomNameId::empty(id.clone()),
            None,
        );
        assert!(
            self.screen
                .view(&self.cx, ids!(restore_status_spinner))
                .visible()
        );
    }

    fn expire(&mut self) {
        self.screen.expire_room_restore(&mut self.cx);
        for _ in 0..3 {
            self.screen
                .handle_event(&mut self.cx, &Event::Signal, &mut Scope::empty());
        }
        assert!(
            !self
                .screen
                .view(&self.cx, ids!(restore_status_spinner))
                .visible()
        );
        assert!(self.screen.button(&self.cx, ids!(restore_retry)).visible());
    }

    fn pump_until(&mut self, description: &str, mut ready: impl FnMut(&Self) -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(25);
        loop {
            self.cx.handle_action_receiver();
            if ready(self) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Timed out waiting for {description}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn expect_terminal(&mut self, message: &str) {
        self.pump_until(message, |f| {
            f.screen.label(&f.cx, ids!(restore_status_label)).text() == crate::i18n::tr(message)
        });
        assert!(
            !self
                .screen
                .view(&self.cx, ids!(restore_status_spinner))
                .visible()
        );
        assert!(!self.screen.button(&self.cx, ids!(restore_retry)).visible());
        assert!(self.screen.button(&self.cx, ids!(restore_close)).visible());
    }

    async fn sync_membership(&self, id: &OwnedRoomId, membership: &str) {
        let room = json!({"initial":true, "required_state":[
            {"type":"m.room.create","state_key":"","event_id":"$create","sender":"@owner:example.org","origin_server_ts":1,"content":{"creator":"@owner:example.org","room_version":"10"}},
            {"type":"m.room.member","state_key":"@alice:example.org","event_id":"$member","sender":"@owner:example.org","origin_server_ts":2,"content":{"membership":membership}}
        ]});
        let template =
            json!({"pos":"initial","lists":{"all_rooms":{"count":1}},"rooms":{(id.as_str()):room}});
        // Echo the request transaction ID as a real sliding-sync server does.
        Mock::given(method("POST"))
            .and(path_regex(".*/sync$"))
            .respond_with(move |req: &wiremock::Request| {
                let request = req.body_json::<Value>().unwrap();
                let mut body = template.clone();
                if let Some(txn) = request.get("txn_id") {
                    body["txn_id"] = txn.clone();
                }
                ResponseTemplate::new(200).set_body_json(body)
            })
            .expect(1)
            .mount(&self.server)
            .await;
        let service = Arc::new(RoomListService::new(self.client.clone()).await.unwrap());
        // The sidebar consumes its regular filtered SDK room list. The membership
        // subscriber sees first-sync left rooms even when that list omits them.
        let _room_list = crate::account_session::spawn(room_list_service_loop(service.clone()));
        let _membership = watch_room_membership(&self.client);
        tokio::time::timeout(Duration::from_secs(15), Box::pin(service.sync()).next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}

#[test]
fn slow_sync_opens_the_restored_timeline_through_the_matrix_worker() {
    if !isolated() {
        return;
    }
    let mut f = Fixture::new();
    let id = "!waiting:example.org".try_into().unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    REQUEST_SENDER.lock().unwrap().replace(tx);
    let (login, _) = tokio::sync::mpsc::channel(1);
    let (account, _) = tokio::sync::mpsc::unbounded_channel();
    let worker = backend_runtime().spawn(matrix_worker_task(rx, login, account));

    f.open(&id);
    f.expire();
    backend_runtime().block_on(f.sync_membership(&id, "join"));
    f.pump_until("the restored tab to take the worker-built timeline", |_| {
        let rooms = ALL_JOINED_ROOMS.lock().unwrap();
        rooms
            .get(&id)
            .and_then(|r| r.main_timeline.as_ref())
            .is_some_and(|t| {
                t.timeline_singleton_endpoints.is_none()
                    && t.request_sender.borrow().is_timeline_open
            })
    });
    assert!(
        f.screen
            .label(&f.cx, ids!(restore_status_label))
            .text()
            .is_empty()
    );
    f.screen.hide_displayed_room(&mut f.cx);
    assert!(
        ALL_JOINED_ROOMS.lock().unwrap()[&id]
            .main_timeline
            .is_none()
    );
    worker.abort();
}

fn first_sync_terminal(membership: &str, message: &str) {
    let mut f = Fixture::new();
    let id = "!old:example.org".try_into().unwrap();
    f.open(&id);
    assert!(f.client.get_room(&id).is_none());
    backend_runtime().block_on(f.sync_membership(&id, membership));
    f.expect_terminal(message);
    assert!(!ALL_JOINED_ROOMS.lock().unwrap().contains_key(&id));
}

#[test]
fn first_sync_left_room_stops_waiting() {
    if isolated() {
        first_sync_terminal("leave", "You are no longer a member of this room.");
    }
}

#[test]
fn first_sync_banned_room_stops_waiting() {
    if isolated() {
        first_sync_terminal("ban", "You have been banned from this room.");
    }
}

fn slow_error(code: u16, errcode: &str, message: &str) {
    let mut f = Fixture::new();
    backend_runtime().block_on(
        Mock::given(method("GET"))
            .and(path_regex(".*/state/m[.]room[.]create/?$"))
            .respond_with(
                ResponseTemplate::new(code)
                    .set_body_json(json!({"errcode":errcode,"error":"unavailable"}))
                    .set_delay(Duration::from_secs(11)),
            )
            .expect(1)
            .mount(&f.server),
    );
    f.open(&"!missing:example.org".try_into().unwrap());
    f.expire();
    f.expect_terminal(message);
}

#[test]
fn forbidden_response_after_spinner_deadline_shows_access_error() {
    if isolated() {
        slow_error(403, "M_FORBIDDEN", "You do not have access to this room.");
    }
}

#[test]
fn not_found_response_after_spinner_deadline_shows_missing_room() {
    if isolated() {
        slow_error(404, "M_NOT_FOUND", "The server could not find this room.");
    }
}
