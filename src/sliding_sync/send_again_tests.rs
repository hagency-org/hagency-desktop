//! A message sent again names the one it was sent for; every place a device
//! of its sender can find the new message stops the first one's offer. The
//! record of what was sent again is one for the process, so each test keeps
//! to a room of its own.
use super::*;
use crate::hagency::resend_offers::{self, SENT_AGAIN_FIELD};
use matrix_sdk::ruma::{event_id, room_id, serde::Raw};
use serde_json::{json, Value};
use wiremock::{
    matchers::{method, path_regex, query_param},
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
    test_read_copies(room.as_str(), None);
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

/// The room's status, offering each of `events` to its sender.
fn offered(room: &RoomId, events: &[&EventId]) -> Value {
    json!({"executions": events.iter().map(|event| json!({"roomId":room.as_str(),"threadRoot":event.as_str(),
        "eventId":event.as_str(),"agentMxid":"@first:x","canSendAgain":true})).collect::<Vec<_>>()})
}

#[test]
fn a_new_message_read_here_outlives_a_press_that_failed_for_now() {
    let _turn = resend_offers::test_turn();
    let room = room_id!("!read-outlives-press:x");
    let (pressed_first, read_first) = (event_id!("$pressed_then_read"), event_id!("$read_then_pressed"));
    test_read_copies(room.as_str(), None);
    assert!(resend_offers::merge_read(resend_offers::TEST_READER, room.as_str(), "", &offered(room, &[pressed_first, read_first])));
    for first in [pressed_first, read_first] {
        assert!(resend_offers::for_event(room.as_str(), first.as_str()).is_some());
    }
    let copy = |first: &EventId| raw(message("$from_elsewhere", "@me:x", json!({SENT_AGAIN_FIELD: {"event_id": first.as_str()}})));
    // Pressed here; then the new message another device sent for it arrives.
    // It was not offered while pressed, so nothing on screen changes.
    note_sent_again(room, pressed_first);
    assert!(!note_sent_again_by(room, [(true, Some(&copy(pressed_first)))]));
    // The other way round: read first, pressed afterwards from a card drawn
    // before.
    assert!(note_sent_again_by(room, [(true, Some(&copy(read_first)))]));
    note_sent_again(room, read_first);
    // This device's own new message cannot go out now. The press is taken
    // back, the new message read here is not: neither is offered again.
    for first in [pressed_first, read_first] {
        send_again_failed(room, first, &SendAgain::Later);
        assert!(sent_again(room, first));
        assert!(resend_offers::for_event(room.as_str(), first.as_str()).is_none());
    }
}

#[test]
fn the_record_drops_only_entries_of_rooms_no_timeline_reads() {
    let (read, unread) = (room_id!("!bound-read:x"), room_id!("!bound-unread:x"));
    let event = |n: usize| OwnedEventId::try_from(format!("$bound{n}")).unwrap();
    // A record of its own: the process's record is shared by every test.
    let mut record = SentAgainRecord::new();
    record.readings.push(CopyReading { id: u64::MAX, room: read.to_owned(), thread: None, ready: true });
    for n in 0..SENT_AGAIN_KEPT / 2 {
        assert!(record.note(read, &event(n), false));
    }
    for n in 0..SENT_AGAIN_KEPT {
        assert!(record.note(unread, &event(n), n % 2 == 0));
    }
    // At the bound, the oldest entries of the room no timeline reads went;
    // every entry of the room a timeline reads stayed.
    assert_eq!(record.entries.len(), SENT_AGAIN_KEPT);
    assert!((0..SENT_AGAIN_KEPT / 2).all(|n| record.has(read, &event(n))));
    assert!(!record.has(unread, &event(0)) && !record.has(unread, &event(SENT_AGAIN_KEPT / 2 - 1)));
    assert!(record.has(unread, &event(SENT_AGAIN_KEPT / 2)) && record.has(unread, &event(SENT_AGAIN_KEPT - 1)));
    // Past the bound, a room a timeline reads keeps all it reads: the record
    // grows by it alone.
    for n in SENT_AGAIN_KEPT / 2..SENT_AGAIN_KEPT * 2 {
        assert!(record.note(read, &event(n), false));
    }
    assert!((0..SENT_AGAIN_KEPT * 2).all(|n| record.has(read, &event(n))));
    assert!(record.entries.iter().all(|entry| *entry.room == *read));
    // Once no timeline reads it, its oldest entries go first.
    record.readings.clear();
    assert!(record.note(unread, &event(0), false));
    assert_eq!(record.entries.len(), SENT_AGAIN_KEPT);
    assert!(!record.has(read, &event(0)) && record.has(read, &event(SENT_AGAIN_KEPT * 2 - 1)));
    assert!(record.has(unread, &event(0)));
}

/// A room as the server returns it on a first sync, with `events` in its
/// timeline.
fn first_sync(room: &str, events: Vec<Value>) -> Value {
    json!({
        "next_batch": "first",
        "rooms": {"join": {room: {
            "state": {"events": [
                {"type":"m.room.create","state_key":"","event_id":"$create","sender":"@me:example.org","origin_server_ts":1,"content":{"creator":"@me:example.org","room_version":"10"}},
                {"type":"m.room.member","state_key":"@me:example.org","event_id":"$me","sender":"@me:example.org","origin_server_ts":2,"content":{"membership":"join"}},
                {"type":"m.room.member","state_key":"@them:example.org","event_id":"$them","sender":"@them:example.org","origin_server_ts":3,"content":{"membership":"join"}}
            ]},
            "timeline": {"limited": false, "prev_batch": "older", "events": events}
        }}}
    })
}

async fn signed_in(server: &MockServer, store: Option<&std::path::Path>, device: &str) -> Client {
    let mut builder = Client::builder().homeserver_url(server.uri())
        .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_11]);
    if let Some(store) = store {
        builder = builder.sqlite_store(store, None);
    }
    let client = builder.build().await.unwrap();
    // Nothing leaves the send queue: a new message waits there as it does
    // on a device that had not sent it yet.
    client.send_queue().set_enabled(false).await;
    client.matrix_auth().restore_session(serde_json::from_value(json!({
        "access_token":"fixture", "user_id":"@me:example.org", "device_id":device
    })).unwrap(), Default::default()).await.unwrap();
    client
}

/// What can and cannot be sent again, as the timeline shows a message.
#[tokio::test]
async fn a_message_not_loaded_or_not_readable_can_be_sent_later_and_one_not_the_users_or_removed_never() {
    let server = MockServer::start().await;
    let room_id = room_id!("!send-again-classes:example.org");
    Mock::given(method("GET")).and(path_regex(".*/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(first_sync(room_id.as_str(), vec![
            message("$mine", "@me:example.org", json!({})),
            message("$theirs", "@them:example.org", json!({})),
            json!({"type":"m.room.message","event_id":"$removed","sender":"@me:example.org","origin_server_ts":11,"content":{},
                "unsigned":{"redacted_because":{"type":"m.room.redaction","event_id":"$removal","sender":"@me:example.org",
                    "origin_server_ts":12,"redacts":"$removed","content":{"redacts":"$removed"}}}}),
            json!({"type":"m.room.encrypted","event_id":"$locked","sender":"@me:example.org","origin_server_ts":13,"content":{
                "algorithm":"m.megolm.v1.aes-sha2","ciphertext":"AwgAEoAB","device_id":"ELSEWHERE",
                "sender_key":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","session_id":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}),
        ])))
        .mount(&server).await;
    let client = signed_in(&server, None, "CLASSES").await;
    client.sync_once(matrix_sdk::config::SyncSettings::default().timeout(Duration::ZERO)).await.unwrap();
    let timeline = client.get_room(room_id).unwrap().timeline_builder().build().await.unwrap();
    // The user's own message, as shown.
    assert!(matches!(message_to_send_again(&timeline, event_id!("$mine")).await, Ok(MessageType::Text(_))));
    // Not loaded, or its key may still arrive: another try may work.
    assert_eq!(message_to_send_again(&timeline, event_id!("$absent")).await.err(), Some(SendAgain::Later));
    let locked = timeline.item_by_event_id(event_id!("$locked")).await.expect("the encrypted message is shown");
    assert!(locked.content().is_unable_to_decrypt());
    assert_eq!(message_to_send_again(&timeline, event_id!("$locked")).await.err(), Some(SendAgain::Later));
    // Someone else's, or removed: no try can.
    assert_eq!(message_to_send_again(&timeline, event_id!("$theirs")).await.err(), Some(SendAgain::Never));
    assert!(timeline.item_by_event_id(event_id!("$removed")).await.is_some());
    assert_eq!(message_to_send_again(&timeline, event_id!("$removed")).await.err(), Some(SendAgain::Never));
}

/// A restart as the app makes one: a new client opens the store the last one
/// left, with a new message still in the send queue and one sent from
/// another device in the room's timeline. The room's subscriber, the task
/// the app runs for an open timeline, reads both before the room offers
/// anything, and reads a new message that arrives while it runs.
#[tokio::test]
async fn after_a_restart_the_subscriber_reads_new_messages_before_the_room_offers_any() {
    let _turn = resend_offers::test_turn();
    let store = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    let room_id = room_id!("!restart:example.org");
    let marker = |first: &str| json!({SENT_AGAIN_FIELD: {"event_id": first}});
    Mock::given(method("GET")).and(path_regex(".*/sync")).and(query_param("since", "first"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "next_batch": "second",
            "rooms": {"join": {room_id.as_str(): {"timeline": {"limited": false, "events": [
                message("$arrived_copy", "@me:example.org", marker("$arrives_later")),
            ]}}}}
        })))
        .with_priority(1)
        .mount(&server).await;
    Mock::given(method("GET")).and(path_regex(".*/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(first_sync(room_id.as_str(), vec![
            message("$in_timeline", "@me:example.org", json!({})),
            message("$in_queue", "@me:example.org", json!({})),
            message("$arrives_later", "@me:example.org", json!({})),
            message("$no_copy", "@me:example.org", json!({})),
            message("$timeline_copy", "@me:example.org", marker("$in_timeline")),
        ])))
        .mount(&server).await;
    // Before the restart: a new message that has not gone out.
    {
        let client = signed_in(&server, Some(store.path()), "RESTART").await;
        client.sync_once(matrix_sdk::config::SyncSettings::default().timeout(Duration::ZERO)).await.unwrap();
        let room = client.get_room(room_id).unwrap();
        let timeline = room.timeline_builder().build().await.unwrap();
        timeline.send_with_extra_content(
            RoomMessageEventContent::text_plain("hello").into(),
            Some(resend_offers::marker("$in_queue")),
        ).await.unwrap();
        let (queued, _) = room.send_queue().subscribe().await.unwrap();
        assert_eq!(queued.len(), 1);
    }
    // The server offers all four.
    let firsts = ["$in_timeline", "$in_queue", "$arrives_later", "$no_copy"];
    let rows = firsts.iter().map(|first| json!({"roomId":room_id.as_str(),"threadRoot":first,"eventId":first,
        "agentMxid":"@first:x","canSendAgain":true})).collect::<Vec<_>>();
    resend_offers::merge_read(resend_offers::TEST_READER, room_id.as_str(), "", &json!({"executions":rows}));
    // After it: nothing is offered before the room's timeline has read what
    // it starts with.
    let client = signed_in(&server, Some(store.path()), "RESTART").await;
    let room = client.get_room(room_id).expect("the room is in the store");
    let timeline = Arc::new(room.timeline_builder().build().await.unwrap());
    for first in firsts {
        assert!(resend_offers::for_event(room_id.as_str(), first).is_none(), "{first}");
    }
    let (update_sender, _update_receiver) = crate::timeline_updates::channel();
    let (_requests, request_receiver) = watch::channel(TimelineRequest { backwards_paginate: Vec::new(), is_timeline_open: true });
    let subscriber = tokio::spawn(timeline_subscriber_handler(timeline.clone(), update_sender, request_receiver, None));
    tokio::time::timeout(Duration::from_secs(20), async {
        while !copies_read(room_id, None) { tokio::time::sleep(Duration::from_millis(10)).await; }
    }).await.expect("the subscriber reads the timeline's first items and the send queue");
    // The new message in the timeline and the one in the send queue were
    // read first: their messages are not offered. The one without a new
    // message is.
    assert!(sent_again(room_id, event_id!("$in_timeline")));
    assert!(sent_again(room_id, event_id!("$in_queue")));
    for first in ["$in_timeline", "$in_queue"] {
        assert!(resend_offers::for_event(room_id.as_str(), first).is_none(), "{first}");
    }
    assert!(resend_offers::for_event(room_id.as_str(), "$no_copy").is_some());
    assert!(resend_offers::for_event(room_id.as_str(), "$arrives_later").is_some());
    // A new message that arrives while the timeline runs withdraws its offer.
    client.sync_once(matrix_sdk::config::SyncSettings::default().timeout(Duration::ZERO)).await.unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        while !sent_again(room_id, event_id!("$arrives_later")) { tokio::time::sleep(Duration::from_millis(10)).await; }
    }).await.expect("the subscriber reads a new message that arrives");
    assert!(resend_offers::for_event(room_id.as_str(), "$arrives_later").is_none());
    // Once the timeline is gone, the room offers nothing until one reads it again.
    subscriber.abort();
    let _ = subscriber.await;
    assert!(!copies_read(room_id, None));
    assert!(resend_offers::for_event(room_id.as_str(), "$no_copy").is_none());
}
