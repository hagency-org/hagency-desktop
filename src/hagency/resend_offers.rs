//! What "Send again" offers for each message of a room that ended without a
//! turn for some of the Agents it was sent to, as the last read of the room's
//! status said. The notice at the end of the timeline speaks about the newest
//! message; the card on a message offers its own, so an earlier message keeps
//! its offer when a newer one arrives.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// The line of a message that ended without a turn for `{agent}`.
pub const UNPROCESSED_BY: &str = "{agent} did not process this message. It will not retry automatically.";

/// The sender's way out of a message that ended without a turn: the same
/// message, sent again to the Agents that did not process it. The server
/// decides for which Agents that is: it knows that nothing of the message
/// reached a model or a tool, that the reader sent it, and that the Agent is
/// in the room and not stopped.
#[derive(Clone, Debug, PartialEq)]
pub struct Resend {
    pub event: String,
    /// The thread the message was sent in, when it was.
    pub thread: Option<String>,
    pub agents: Vec<String>,
}

/// What some message offers changed: the cards on the messages are drawn
/// again.
#[derive(Clone, Copy, Debug)]
pub struct ResendOffersChanged;

/// The Agent a row is about, when the server names it.
pub fn row_agent(row: &Value) -> Option<&str> {
    row["agentMxid"].as_str().filter(|id| id.starts_with('@') && id.len() <= 255 && !id.chars().any(char::is_control))
}

/// What "Send again" would send for one message, from its rows: one row for
/// each Agent the message was sent to. A row from a server that does not say
/// whether a message can be sent again offers nothing.
pub fn of_message(rows: &[&Value]) -> Option<Resend> {
    let event = rows.first()?["eventId"].as_str()?;
    let agents = rows.iter()
        .filter(|r| r["eventId"] == event && r["canSendAgain"] == true)
        .filter_map(|r| row_agent(r))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if agents.is_empty() {
        return None;
    }
    let root = rows[0]["threadRoot"].as_str().filter(|root| *root != event);
    Some(Resend { event: event.to_owned(), thread: root.map(str::to_owned), agents })
}

/// Every message of a read of `room`, or of its thread `thread`, that can be
/// sent again, newest first as the server lists them.
pub fn of_read(value: &Value, room: &str, thread: &str) -> Vec<Resend> {
    let rows = value["executions"].as_array().into_iter().flatten()
        .filter(|r| r["roomId"] == room && (thread.is_empty() || r["threadRoot"] == thread))
        .collect::<Vec<_>>();
    let mut events: Vec<&str> = Vec::new();
    for row in &rows {
        if let Some(event) = row["eventId"].as_str()
            && !events.contains(&event)
        {
            events.push(event);
        }
    }
    events.into_iter()
        .filter_map(|event| of_message(&rows.iter().copied().filter(|r| r["eventId"] == event).collect::<Vec<_>>()))
        .collect()
}

#[derive(Default)]
struct State {
    /// The account the reads below belong to.
    identity: String,
    /// What the last read of each room, or thread of a room, offered.
    reads: HashMap<(String, String), Vec<Resend>>,
    /// Messages whose offer named something that cannot be addressed. They
    /// are not offered again by this client.
    refused: Vec<(String, String)>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(Default::default);

/// Keep what a read of `room`, or of its thread `thread`, offers. Returns
/// whether that changed what this client offers.
pub fn merge_read(identity: &str, room: &str, thread: &str, value: &Value) -> bool {
    let offers = of_read(value, room, thread);
    let mut state = STATE.lock().unwrap();
    if state.identity != identity {
        state.identity = identity.to_owned();
        state.reads.clear();
        state.refused.clear();
    }
    let key = (room.to_owned(), thread.to_owned());
    if state.reads.get(&key).map_or(offers.is_empty(), |kept| *kept == offers) {
        return false;
    }
    state.reads.insert(key, offers);
    true
}

/// What "Send again" offers on `event` of `room`, unless this client has
/// already sent it again.
pub fn for_event(room: &str, event: &str) -> Option<Resend> {
    if sent_again(room, event) {
        return None;
    }
    let state = STATE.lock().unwrap();
    if state.refused.iter().any(|(r, e)| r == room && e == event) {
        return None;
    }
    state.reads.iter()
        .filter(|((r, _), _)| r == room)
        .flat_map(|(_, offers)| offers)
        .find(|offer| offer.event == event)
        .cloned()
}

/// Whether this client has already sent `event` of `room` again, or found
/// that it cannot be.
pub fn sent_again(room: &str, event: &str) -> bool {
    use matrix_sdk::ruma::{EventId, RoomId};
    let sent = matches!(
        (<&RoomId>::try_from(room), <&EventId>::try_from(event)),
        (Ok(room), Ok(event)) if crate::sliding_sync::sent_again(room, event)
    );
    sent || STATE.lock().unwrap().refused.iter().any(|(r, e)| r == room && e == event)
}

/// Send `offer`, a message of `room`, again as a new message to the Agents it
/// names. It goes out as a new message of the same account, so it is taken in
/// and counted like any other; the first one stays as it ended. Returns false
/// when what the server named cannot be addressed: it never will be, and the
/// message is not offered again.
pub fn send(room: &str, offer: &Resend) -> bool {
    use matrix_sdk::ruma::{OwnedEventId, OwnedRoomId, OwnedUserId};
    // The notice and the card offer the newest message both: whichever is
    // pressed second finds it sent.
    if sent_again(room, &offer.event) {
        return true;
    }
    let parsed = (|| {
        let room = OwnedRoomId::try_from(room).ok()?;
        let event = OwnedEventId::try_from(offer.event.as_str()).ok()?;
        let thread = match offer.thread.as_deref() {
            Some(root) => Some(OwnedEventId::try_from(root).ok()?),
            None => None,
        };
        let agents = offer.agents.iter().map(|a| OwnedUserId::try_from(a.as_str()).ok()).collect::<Option<Vec<_>>>()?;
        Some((room, event, thread, agents))
    })();
    let Some((room_id, event_id, thread_root, agents)) = parsed else {
        let mut state = STATE.lock().unwrap();
        if state.refused.len() >= 256 {
            state.refused.remove(0);
        }
        state.refused.push((room.to_owned(), offer.event.clone()));
        return false;
    };
    // One click sends once, also after the room was left and opened again.
    crate::sliding_sync::note_sent_again(&room_id, &event_id);
    crate::sliding_sync::submit_async_request(crate::sliding_sync::MatrixRequest::SendAgainToAgents {
        room_id,
        event_id,
        thread_root,
        agents,
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(event: &str, agent: &str, again: bool) -> Value {
        json!({"roomId":"!room:x","threadRoot":event,"eventId":event,"agentMxid":agent,
            "state":"expired","canSendAgain":again})
    }

    #[test]
    fn every_message_that_can_be_sent_again_is_offered_with_its_own_agents() {
        let value = json!({"executions":[
            row("$new","@first:x",false),
            row("$old","@first:x",true),
            row("$old","@second:x",true),
            row("$older","@second:x",false),
            row("$oldest","@first:x",true),
            {"roomId":"!other:x","threadRoot":"$elsewhere","eventId":"$elsewhere","agentMxid":"@first:x","canSendAgain":true},
        ]});
        assert_eq!(of_read(&value, "!room:x", ""), vec![
            Resend { event: "$old".into(), thread: None, agents: vec!["@first:x".into(), "@second:x".into()] },
            Resend { event: "$oldest".into(), thread: None, agents: vec!["@first:x".into()] },
        ]);
        // A reply in a thread is offered with its thread, and only in reads of
        // the room or of that thread.
        let reply = json!({"executions":[
            {"roomId":"!room:x","threadRoot":"$root","eventId":"$reply","agentMxid":"@first:x","canSendAgain":true},
        ]});
        let offered = vec![Resend { event: "$reply".into(), thread: Some("$root".into()), agents: vec!["@first:x".into()] }];
        assert_eq!(of_read(&reply, "!room:x", ""), offered);
        assert_eq!(of_read(&reply, "!room:x", "$root"), offered);
        assert!(of_read(&reply, "!room:x", "$another").is_empty());
        // A server that does not describe each Agent offers nothing.
        let old = json!({"executions":[{"roomId":"!room:x","eventId":"$e","state":"cancelled"}]});
        assert!(of_read(&old, "!room:x", "").is_empty());
    }

    /// The store is one for the process, so every test reads as the same
    /// account and keeps to rooms of its own.
    const READER: &str = "@reader:x";

    fn offered(room: &str, event: &str, agent: &str) -> Value {
        json!({"executions":[{"roomId":room,"threadRoot":event,"eventId":event,"agentMxid":agent,"canSendAgain":true}]})
    }

    #[test]
    fn a_read_replaces_what_the_same_room_or_thread_offered() {
        assert!(merge_read(READER, "!kept:x", "", &offered("!kept:x", "$kept", "@first:x")));
        // The same read again changes nothing, and neither does a read of a
        // room that offers nothing.
        assert!(!merge_read(READER, "!kept:x", "", &offered("!kept:x", "$kept", "@first:x")));
        assert!(!merge_read(READER, "!empty:x", "", &json!({"executions":[]})));
        assert_eq!(for_event("!kept:x", "$kept").unwrap().agents, vec!["@first:x".to_owned()]);
        assert!(for_event("!kept:x", "$other").is_none());
        assert!(for_event("!other:x", "$kept").is_none());
        // A read of one of the room's threads keeps what the room's read offers.
        assert!(!merge_read(READER, "!kept:x", "$thread", &json!({"executions":[]})));
        assert!(for_event("!kept:x", "$kept").is_some());
        // The offer ends when the server stops making it.
        assert!(merge_read(READER, "!kept:x", "", &json!({"executions":[]})));
        assert!(for_event("!kept:x", "$kept").is_none());
    }

    #[test]
    fn an_offer_that_cannot_be_addressed_is_made_once() {
        // The server's row names an Agent that is no Matrix ID this client
        // can address.
        assert!(merge_read(READER, "!refused:x", "", &offered("!refused:x", "$refused", "@not an id")));
        let offer = for_event("!refused:x", "$refused").unwrap();
        assert!(!send("!refused:x", &offer));
        assert!(sent_again("!refused:x", "$refused"));
        assert!(for_event("!refused:x", "$refused").is_none());
        // A later read that offers it again does not bring it back.
        merge_read(READER, "!refused:x", "", &offered("!refused:x", "$refused", "@first:x"));
        assert!(for_event("!refused:x", "$refused").is_none());
    }

    #[test]
    fn a_message_sent_again_from_one_place_is_not_sent_from_the_other() {
        use matrix_sdk::ruma::{OwnedEventId, OwnedRoomId};
        assert!(merge_read(READER, "!twice:x", "", &offered("!twice:x", "$twice", "@first:x")));
        let offer = for_event("!twice:x", "$twice").unwrap();
        // The notice sent it: the card no longer offers it, and a press that
        // raced the notice sends nothing.
        crate::sliding_sync::note_sent_again(&OwnedRoomId::try_from("!twice:x").unwrap(), &OwnedEventId::try_from("$twice").unwrap());
        assert!(for_event("!twice:x", "$twice").is_none());
        assert!(send("!twice:x", &offer));
    }
}
