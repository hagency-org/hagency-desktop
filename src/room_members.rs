//! Bringing a room's stored member list in line with the homeserver's.
//!
//! In sliding sync the Matrix library takes a room's state only from the `required_state`
//! part of a response, never from the events of its timeline. A homeserver that leaves a
//! changed membership out of that part leaves the stored list with a person who is gone,
//! although the timeline shows them leaving. Palpo did so for anyone removed, banned or
//! invited by somebody else, since it sent member state only for the senders of events.

use std::time::Duration;

use makepad_widgets::error;
use matrix_sdk::Room;

/// How long a reload may take before the stored list is used as it is.
const RELOAD_TIMEOUT: Duration = Duration::from_secs(15);

/// Replaces the stored member list of `room` with the homeserver's, then tells the room's
/// open timelines so that their own copies of the list follow.
///
/// Returns false and leaves the stored list as it was when the homeserver can't be asked.
/// The room then still counts as not loaded, so the library asks again by itself before
/// it next encrypts a message for the room.
pub(crate) async fn reload(room: &Room) -> bool {
    // The library asks the homeserver only for a room it counts as not loaded.
    room.mark_members_missing();
    match tokio::time::timeout(RELOAD_TIMEOUT, room.sync_members()).await {
        Ok(Ok(())) => {
            crate::sliding_sync::room_members_reloaded(room.room_id());
            true
        }
        Ok(Err(e)) => {
            error!("Failed to reload the members of {}: {e:?}", room.room_id());
            false
        }
        Err(_) => {
            error!("Timed out reloading the members of {}.", room.room_id());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::{Client, RoomMemberships, ruma::OwnedRoomId};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path, path_regex},
    };

    const ALICE: &str = "@alice:example.org";
    const BOB: &str = "@bob:example.org";
    const ROOM: &str = "!members:example.org";

    fn member(user: &str, sender: &str, event: &str, membership: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "m.room.member", "room_id": ROOM, "state_key": user, "event_id": event,
            "sender": sender, "origin_server_ts": 3, "content": {"membership": membership}
        })
    }

    /// A client whose stored member list of `ROOM` has alice and bob as joined and counts
    /// as loaded: the state a device is left in when the homeserver never sent it the
    /// event by which alice removed bob.
    async fn with_bob_still_listed(server: &MockServer) -> Room {
        let joined = serde_json::json!({ ROOM: {
            "state": {"events": [
                {"type":"m.room.create","state_key":"","event_id":"$create","sender":ALICE,"origin_server_ts":1,"content":{"creator":ALICE,"room_version":"10"}},
                member(ALICE, ALICE, "$alice", "join"),
                member(BOB, BOB, "$bob", "join")
            ]},
            "timeline": {"events": [], "limited": false, "prev_batch": "older"}
        }});
        Mock::given(method("GET"))
            .and(path_regex(".*/sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"next_batch": "initial", "rooms": {"join": joined}}),
            ))
            .mount(server)
            .await;
        let client = Client::builder()
            .homeserver_url(server.uri())
            .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_11])
            .build()
            .await
            .unwrap();
        client
            .matrix_auth()
            .restore_session(
                serde_json::from_value(serde_json::json!({
                    "access_token": "fixture", "user_id": ALICE, "device_id": "MEMBERRELOAD"
                }))
                .unwrap(),
                Default::default(),
            )
            .await
            .unwrap();
        client
            .sync_once(matrix_sdk::config::SyncSettings::default().timeout(Duration::ZERO))
            .await
            .unwrap();
        let room = client.get_room(&OwnedRoomId::try_from(ROOM).unwrap()).unwrap();
        // The first load, answered while bob was still in the room.
        let first = Mock::given(method("GET"))
            .and(path(members_path()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"chunk": [
                member(ALICE, ALICE, "$alice", "join"),
                member(BOB, BOB, "$bob", "join")
            ]})))
            .expect(1)
            .mount_as_scoped(server)
            .await;
        room.sync_members().await.unwrap();
        drop(first);
        assert!(room.are_members_synced());
        assert_eq!(joined_members(&room).await, [ALICE, BOB]);
        room
    }

    fn members_path() -> String {
        format!("/_matrix/client/v3/rooms/{ROOM}/members")
    }

    async fn joined_members(room: &Room) -> Vec<String> {
        let mut joined: Vec<_> = room
            .members_no_sync(RoomMemberships::JOIN)
            .await
            .unwrap()
            .iter()
            .map(|member| member.user_id().to_string())
            .collect();
        joined.sort();
        joined
    }

    fn run(test: impl std::future::Future<Output = ()>) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(test);
    }

    #[test]
    fn a_reload_drops_a_person_the_stored_list_kept_after_their_removal() {
        run(async {
            let server = MockServer::start().await;
            let room = with_bob_still_listed(&server).await;
            Mock::given(method("GET"))
                .and(path(members_path()))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"chunk": [
                    member(ALICE, ALICE, "$alice", "join"),
                    member(BOB, ALICE, "$removed", "leave")
                ]})))
                .expect(2)
                .mount(&server)
                .await;

            // The fault: the library alone never asks again for a room it counts as loaded.
            room.sync_members().await.unwrap();
            assert_eq!(joined_members(&room).await, [ALICE, BOB]);

            assert!(reload(&room).await);
            assert_eq!(joined_members(&room).await, [ALICE]);
            assert!(room.are_members_synced());
            // Every reload asks the homeserver, also for a list that was just loaded.
            assert!(reload(&room).await);
            assert_eq!(joined_members(&room).await, [ALICE]);
        });
    }

    #[test]
    fn a_reload_the_homeserver_refuses_keeps_the_stored_list_and_is_asked_again() {
        run(async {
            let server = MockServer::start().await;
            let room = with_bob_still_listed(&server).await;
            let refused = Mock::given(method("GET"))
                .and(path(members_path()))
                .respond_with(ResponseTemplate::new(403).set_body_json(
                    serde_json::json!({"errcode": "M_FORBIDDEN", "error": "Not allowed."}),
                ))
                .expect(1)
                .mount_as_scoped(&server)
                .await;

            assert!(!reload(&room).await);
            assert_eq!(joined_members(&room).await, [ALICE, BOB]);
            drop(refused);

            // What the library does by itself before it next encrypts for the room.
            Mock::given(method("GET"))
                .and(path(members_path()))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"chunk": [
                    member(ALICE, ALICE, "$alice", "join"),
                    member(BOB, ALICE, "$removed", "leave")
                ]})))
                .expect(1)
                .mount(&server)
                .await;
            assert!(!room.are_members_synced());
            room.sync_members().await.unwrap();
            assert_eq!(joined_members(&room).await, [ALICE]);
        });
    }
}
