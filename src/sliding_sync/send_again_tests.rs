//! A message sent again names the one it was sent for; every place a device
//! of its sender can find the new message stops the first one's offer. The
//! record of what was sent again is one for the process, so each test keeps
//! to a room of its own.
use super::*;
use crate::hagency::resend_offers::{self, SENT_AGAIN_FIELD};
use matrix_sdk::ruma::{event_id, room_id, serde::Raw};
use serde_json::{json, Value};
use wiremock::{
    matchers::{method, path_regex},
    Mock, MockServer, ResponseTemplate,
};

/// A message as the server returns it, with `extra` in its content.
fn message(id: &str, sender: &str, extra: Value) -> Value {
    let mut content = json!({"msgtype":"m.text","body":"hello"});
    content.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap_or_default());
    json!({"type":"m.room.message","event_id":id,"sender":sender,"origin_server_ts":10,"content":content})
}

fn raw(event: Value) -> Raw<AnySyncTimelineEvent> {
    Raw::from_json_string(event.to_string()).unwrap()
}

#[test]
fn a_copy_names_its_first_message_wherever_a_timeline_shows_it() {
    let room = room_id!("!copies-in-a-timeline:x");
    let copy = raw(message("$copy", "@me:x", json!({SENT_AGAIN_FIELD: {"event_id": "$first"}})));
    let theirs = raw(message("$their_copy", "@them:x", json!({SENT_AGAIN_FIELD: {"event_id": "$theirs"}})));
    let plain = raw(message("$plain", "@me:x", json!({})));
    // A message whose text mentions the field is read, and names nothing.
    let mentions = raw(message("$mentions", "@me:x", json!({"body": SENT_AGAIN_FIELD})));
    // The items a timeline holds when it is first watched: the copy came
    // from another device of the sender, or before a restart.
    assert!(note_sent_again_by(room, [(true, Some(&plain)), (false, Some(&theirs)), (true, Some(&mentions)), (true, Some(&copy))]));
    assert!(sent_again(room, event_id!("$first")));
    // Someone else's message names nothing for this sender.
    assert!(!sent_again(room, event_id!("$theirs")));
    // A later change that shows the same copy again changes nothing, so the
    // room is not drawn again for it.
    assert!(!note_sent_again_by(room, [(true, Some(&copy))]));
    // A copy that arrives later is read the same way.
    let later = raw(message("$later_copy", "@me:x", json!({SENT_AGAIN_FIELD: {"event_id": "$second"}})));
    assert!(note_sent_again_by(room, [(true, Some(&plain)), (true, Some(&later))]));
    assert!(sent_again(room, event_id!("$second")));
    // A malformed name is no name.
    for bad in [json!("$bad"), json!({"event_id": "bad"}), json!({"event_id": "$ bad"}), json!({"event_id": 7}), json!({})] {
        let malformed = raw(message("$malformed", "@me:x", json!({SENT_AGAIN_FIELD: bad})));
        assert!(!note_sent_again_by(room, [(true, Some(&malformed))]), "{bad}");
    }
    assert!(!sent_again(room, event_id!("$bad")));
    // A local echo has no JSON: a copy that has not gone out is read from
    // the send queue.
    assert!(!note_sent_again_by(room, [(true, None)]));
}

#[test]
fn a_failure_another_try_can_fix_offers_the_message_again_and_one_no_try_can_does_not() {
    let _turn = resend_offers::test_turn();
    let room = room_id!("!send-again-failures:x");
    let (later, never) = (event_id!("$fails_for_now"), event_id!("$fails_for_good"));
    let read = json!({"executions":[
        {"roomId":room.as_str(),"threadRoot":later.as_str(),"eventId":later.as_str(),"agentMxid":"@first:x","canSendAgain":true},
        {"roomId":room.as_str(),"threadRoot":never.as_str(),"eventId":never.as_str(),"agentMxid":"@first:x","canSendAgain":true},
    ]});
    assert!(resend_offers::merge_read(resend_offers::TEST_READER, room.as_str(), "", &read));
    // Pressed: neither is offered while its copy is being made.
    note_sent_again(room, later);
    note_sent_again(room, never);
    assert!(resend_offers::for_event(room.as_str(), later.as_str()).is_none());
    // Not loaded or not readable yet, the room's encryption not confirmed,
    // an Agent's device not ready, the send refused: offered again.
    assert_eq!(send_again_failed(room, later, &SendAgain::Later), "Could not send the message again. Try again in a moment.");
    assert!(!sent_again(room, later));
    assert!(resend_offers::for_event(room.as_str(), later.as_str()).is_some());
    // Not the sender's message, or removed: not offered again while this
    // client runs.
    assert_eq!(send_again_failed(room, never, &SendAgain::Never), "This message can no longer be sent again.");
    assert!(sent_again(room, never));
    assert!(resend_offers::for_event(room.as_str(), never.as_str()).is_none());
}

/// The SDK as the app runs it: the items a timeline starts with carry the
/// JSON of the user's own messages, and a copy that has not gone out is a
/// local echo without its content's extra fields, kept with them in the
/// room's send queue.
#[tokio::test]
async fn copies_are_read_from_a_timelines_first_items_and_from_the_send_queue() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path_regex(".*/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "next_batch": "first",
            "rooms": {"join": {"!first-items:example.org": {
                "state": {"events": [
                    {"type":"m.room.create","state_key":"","event_id":"$create","sender":"@me:example.org","origin_server_ts":1,"content":{"creator":"@me:example.org","room_version":"10"}},
                    {"type":"m.room.member","state_key":"@me:example.org","event_id":"$me","sender":"@me:example.org","origin_server_ts":2,"content":{"membership":"join"}},
                    {"type":"m.room.member","state_key":"@them:example.org","event_id":"$them","sender":"@them:example.org","origin_server_ts":3,"content":{"membership":"join"}}
                ]},
                "timeline": {"limited": false, "prev_batch": "older", "events": [
                    message("$first_message", "@me:example.org", json!({})),
                    message("$copy_message", "@me:example.org", json!({SENT_AGAIN_FIELD: {"event_id": "$first_message"}})),
                    message("$their_message", "@them:example.org", json!({SENT_AGAIN_FIELD: {"event_id": "$their_first"}})),
                ]}
            }}}
        })))
        .mount(&server).await;
    let client = Client::builder().homeserver_url(server.uri())
        .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_11]).build().await.unwrap();
    client.matrix_auth().restore_session(serde_json::from_value(json!({
        "access_token":"fixture", "user_id":"@me:example.org", "device_id":"COPIES"
    })).unwrap(), Default::default()).await.unwrap();
    client.sync_once(matrix_sdk::config::SyncSettings::default().timeout(Duration::ZERO)).await.unwrap();
    let room = client.get_room(room_id!("!first-items:example.org")).unwrap();
    let timeline = room.timeline_builder().build().await.unwrap();
    let (items, _updates) = timeline.subscribe().await;
    assert!(note_sent_again_by(room.room_id(), seen_events(&items, 0)));
    assert!(sent_again(room.room_id(), event_id!("$first_message")));
    assert!(!sent_again(room.room_id(), event_id!("$their_first")));

    // A copy that has not gone out: the send queue keeps it, across a restart.
    client.send_queue().set_enabled(false).await;
    timeline.send_with_extra_content(
        RoomMessageEventContent::text_plain("hello").into(),
        Some(resend_offers::marker("$queued_first")),
    ).await.unwrap();
    let echo = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Some(echo) = timeline.items().await.iter().find_map(|item| item.as_event().filter(|e| e.is_local_echo()).cloned()) {
                break echo;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("the copy shows as a local echo");
    assert!(echo.original_json().is_none());
    assert!(!note_sent_again_by(room.room_id(), [(echo.is_own(), echo.original_json())]));
    assert!(!sent_again(room.room_id(), event_id!("$queued_first")));
    let (queued, _) = room.send_queue().subscribe().await.unwrap();
    assert!(note_queued_copies(room.room_id(), queued_contents(&queued)));
    assert!(sent_again(room.room_id(), event_id!("$queued_first")));
    // Read again when the room's timeline is watched anew, it changes nothing.
    assert!(!note_queued_copies(room.room_id(), queued_contents(&queued)));
}
