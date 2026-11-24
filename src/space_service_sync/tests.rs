use super::*;
use matrix_sdk::config::SyncSettings;
use serde_json::json;
use std::time::Duration;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path_regex},
};

#[tokio::test]
async fn a_child_link_after_an_empty_first_page_reaches_the_space_worker() {
    let server = MockServer::start().await;
    let client = Client::builder()
        .homeserver_url(server.uri())
        .server_versions([ruma::api::MatrixVersion::V1_11])
        .build()
        .await
        .unwrap();
    let user = "@member:remote.test";
    client
        .matrix_auth()
        .restore_session(
            serde_json::from_value(json!({
                "access_token": "fixture", "user_id": user, "device_id": "SPACE"
            }))
            .unwrap(),
            Default::default(),
        )
        .await
        .unwrap();

    let space = ruma::owned_room_id!("!parent:home.test");
    let child = ruma::owned_room_id!("!child:home.test");
    let summary = |room: &ruma::RoomId| {
        json!({
            "room_id": room, "room_type": "m.space", "join_rule": "invite",
            "num_joined_members": 2, "world_readable": false, "guest_can_join": false,
            "children_state": []
        })
    };

    let first = Mock::given(method("GET"))
        .and(path_regex(".*/hierarchy$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"rooms":[summary(&space)]})))
        .expect(1)
        .mount_as_scoped(&server)
        .await;
    let list = SpaceRoomList::new(client.clone(), space.clone()).await;
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let (out, mut events) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(space_room_list_loop(
        client.clone(),
        space.clone(),
        ParentChain::new(),
        rx,
        list,
        out,
    ));
    tokio::time::timeout(Duration::from_secs(5), first.wait_until_satisfied())
        .await
        .unwrap();
    drop(first);

    let link = json!({"type":"m.space.child","state_key":child,"sender":"@owner:home.test",
        "origin_server_ts":3,"event_id":"$link","content":{"via":["home.test"]}});
    let mut parent = summary(&space);
    parent["children_state"] = json!([link.clone()]);
    Mock::given(method("GET"))
        .and(path_regex(".*/hierarchy$"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"rooms":[parent,summary(&child)]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET")).and(path_regex(".*/sync$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "next_batch":"after-link","rooms":{"join":{space.as_str():{
                "state":{"events":[
                    {"type":"m.room.create","state_key":"","sender":"@owner:home.test","event_id":"$create","origin_server_ts":1,"content":{"type":"m.space","creator":"@owner:home.test","room_version":"10"}},
                    {"type":"m.room.member","state_key":user,"sender":user,"event_id":"$join","origin_server_ts":2,"content":{"membership":"join"}}
                ]},"timeline":{"limited":false,"events":[link]}
            }}}
        }))).mount(&server).await;
    client
        .sync_once(SyncSettings::default().timeout(Duration::ZERO))
        .await
        .unwrap();

    let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .unwrap();
    assert!(
        matches!(event, Some(SpaceRequest::SubscribeToSpaceRoomList { space_id, .. }) if space_id == child)
    );
    tx.send(SpaceRoomListRequest::Shutdown).unwrap();
    task.await.unwrap();
}
