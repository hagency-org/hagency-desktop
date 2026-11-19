//! Bind the TSP badge to the Matrix content used by the timeline.
use super::{tsp_sign_indicator::TspSignState, tsp_state_ref};
use matrix_sdk_ui::timeline::{EventTimelineItem, MsgLikeContent, MsgLikeKind};
use ruma::{
    events::room::message::MessageType,
    html::{HtmlSanitizerMode, RemoveReplyFallback},
    serde::Base64,
};
use serde_json::Value;
use std::sync::Arc;
use tsp_sdk::{VerifiedVid, cesr::SignatureType};

const SIGNATURE: &str = "org.robius.tsp_signature";

pub(crate) fn timeline_signature(
    item: &EventTimelineItem,
    content: &MsgLikeContent,
) -> Option<TspSignState> {
    let MsgLikeKind::Message(message) = &content.kind else {
        return None;
    };
    // A local or bundled edit may have no raw edit event yet. Never inherit
    // the original event's signature for a newer displayed revision.
    let raw = if message.is_edited() {
        item.latest_edit_json()
    } else {
        item.original_json()
    }?;
    let event = raw.get_field::<Value>("content").ok()??;
    classify(
        &event,
        message.msgtype(),
        message.is_edited(),
        content.in_reply_to.is_some(),
        || {
            tsp_state_ref()
                .lock()
                .unwrap()
                .get_verified_vid_for(item.sender())
        },
    )
}

fn classify(
    event: &Value,
    displayed: &MessageType,
    edited: bool,
    reply: bool,
    sender: impl FnOnce() -> Option<Arc<dyn VerifiedVid>>,
) -> Option<TspSignState> {
    let effective = if edited {
        if event.pointer("/m.relates_to/rel_type")?.as_str()? != "m.replace" {
            return None;
        }
        event.get("m.new_content")?
    } else {
        event
    };
    let outer = event.get(SIGNATURE);
    let inner = effective.get(SIGNATURE);
    let signature = inner.or(outer)?;
    if outer.zip(inner).is_some_and(|(a, b)| a != b) {
        return Some(TspSignState::WrongSignature);
    }
    let Ok(envelope) = serde_json::from_value::<Base64>(signature.clone()) else {
        return Some(TspSignState::WrongSignature);
    };
    let Some(sender) = sender() else {
        return Some(TspSignState::Unknown);
    };
    let mut bytes = envelope.into_inner();
    let valid = tsp_sdk::crypto::verify(&*sender, &mut bytes).is_ok_and(|(payload, kind)| {
        kind.signature_type != SignatureType::NoSignature
            && matches_displayed_content(payload, effective, displayed, reply && !edited)
    });
    Some(if valid {
        TspSignState::Verified
    } else {
        TspSignState::WrongSignature
    })
}

fn matches_displayed_content(
    payload: &[u8],
    event: &Value,
    displayed: &MessageType,
    strip_reply: bool,
) -> bool {
    let Ok(mut signed) = serde_json::from_slice::<serde_json::Map<String, Value>>(payload) else {
        return false;
    };
    let Some(mut received) = event.as_object().cloned() else {
        return false;
    };
    received.remove(SIGNATURE);
    let room_mention = |map: &serde_json::Map<String, Value>| {
        map.get("m.mentions")
            .and_then(|v| v.get("room"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    if room_mention(&signed) != room_mention(&received) {
        return false;
    }
    // The composer signs before the SDK adds reply/thread context, user
    // mentions and agent routing. These fields do not supply the message body.
    for key in [
        "m.relates_to",
        "m.mentions",
        "org.octos.target_user_id",
        "org.octos.explicit_room",
        "org.octos.broadcast_targets",
    ] {
        signed.remove(key);
        received.remove(key);
    }
    if signed != received {
        return false;
    }

    let Ok(mut message) = serde_json::from_value::<MessageType>(Value::Object(signed)) else {
        return false;
    };
    // The SDK sanitizes formatted messages before giving them to the renderer.
    // Use that same transformation, including its reply fallback rule.
    message.sanitize(
        HtmlSanitizerMode::Compat,
        if strip_reply {
            RemoveReplyFallback::Yes
        } else {
            RemoveReplyFallback::No
        },
    );
    matches!((serde_json::to_value(message), serde_json::to_value(displayed)),
        (Ok(signed), Ok(shown)) if signed == shown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tsp_sdk::OwnedVid;

    fn identity() -> Arc<OwnedVid> {
        Arc::new(OwnedVid::new_did_peer(
            "https://example.invalid".parse().unwrap(),
        ))
    }

    fn signed(vid: &OwnedVid, content: &Value) -> Value {
        let bytes =
            tsp_sdk::crypto::sign(vid, None, &serde_json::to_vec(content).unwrap()).unwrap();
        Value::String(Base64::<ruma::serde::base64::Standard>::new(bytes).encode())
    }

    fn shown(content: &Value) -> MessageType {
        serde_json::from_value(content.clone()).unwrap()
    }

    #[test]
    fn genuine_signature_requires_the_same_raw_and_displayed_message() {
        let vid = identity();
        let content = json!({"msgtype":"m.text","body":"one","format":"org.matrix.custom.html","formatted_body":"<strong>one</strong>"});
        let mut event = content.clone();
        event[SIGNATURE] = signed(&vid, &content);
        let verify = |event: &Value, displayed: &MessageType| {
            classify(event, displayed, false, false, || Some(vid.clone()))
        };
        assert!(matches!(
            verify(&event, &shown(&content)),
            Some(TspSignState::Verified)
        ));
        for (field, value) in [
            ("body", "two"),
            ("formatted_body", "<i>one</i>"),
            ("msgtype", "m.notice"),
            ("org.example.card", "extra"),
        ] {
            let mut changed = event.clone();
            changed[field] = json!(value);
            assert!(matches!(
                verify(&changed, &shown(&changed)),
                Some(TspSignState::WrongSignature)
            ));
        }
        let mut local = content.clone();
        local["body"] = json!("local edit");
        assert!(matches!(
            verify(&event, &shown(&local)),
            Some(TspSignState::WrongSignature)
        ));
        assert!(matches!(
            classify(&event, &shown(&content), false, false, || Some(identity())),
            Some(TspSignState::WrongSignature)
        ));
        assert!(matches!(
            classify(&event, &shown(&content), false, false, || None),
            Some(TspSignState::Unknown)
        ));
        assert!(verify(&content, &shown(&content)).is_none());
        event[SIGNATURE] = json!("!");
        assert!(matches!(
            verify(&event, &shown(&content)),
            Some(TspSignState::WrongSignature)
        ));
    }

    #[test]
    fn edit_uses_replacement_content_and_its_own_signature() {
        let vid = identity();
        let content = json!({"msgtype":"m.text","body":"revised"});
        let mut edit = json!({"msgtype":"m.text","body":"* revised","m.relates_to":{"rel_type":"m.replace","event_id":"$original"},"m.new_content":content});
        let verify =
            |event: &Value| classify(event, &shown(&content), true, false, || Some(vid.clone()));
        assert!(verify(&edit).is_none());
        edit["m.new_content"][SIGNATURE] = signed(&vid, &content);
        assert!(matches!(verify(&edit), Some(TspSignState::Verified)));
        edit[SIGNATURE] = signed(&vid, &json!({"msgtype":"m.text","body":"original"}));
        assert!(matches!(verify(&edit), Some(TspSignState::WrongSignature)));
        edit["m.new_content"]
            .as_object_mut()
            .unwrap()
            .remove(SIGNATURE);
        assert!(matches!(verify(&edit), Some(TspSignState::WrongSignature)));
        assert!(verify(&content).is_none());
    }

    #[test]
    fn signed_reply_matches_the_sdk_sanitized_body() {
        let vid = identity();
        let content = json!({"msgtype":"m.text","body":"> quote\n\nresponse","format":"org.matrix.custom.html","formatted_body":"<mx-reply><blockquote>quote</blockquote></mx-reply><b>response</b>"});
        let mut event = content.clone();
        event[SIGNATURE] = signed(&vid, &content);
        event["m.relates_to"] = json!({"m.in_reply_to":{"event_id":"$parent"}});
        event["m.mentions"] = json!({"user_ids":["@parent:example.org"]});
        event["org.octos.target_user_id"] = json!("@agent:example.org");
        let mut displayed = shown(&content);
        displayed.sanitize(HtmlSanitizerMode::Compat, RemoveReplyFallback::Yes);
        assert!(matches!(
            classify(&event, &displayed, false, true, || Some(vid.clone())),
            Some(TspSignState::Verified)
        ));
        event["m.mentions"]["room"] = json!(true);
        assert!(matches!(
            classify(&event, &displayed, false, true, || Some(vid.clone())),
            Some(TspSignState::WrongSignature)
        ));
    }
}
