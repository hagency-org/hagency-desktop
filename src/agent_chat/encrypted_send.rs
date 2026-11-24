//! Do not encrypt the first Agent request to an empty recipient device list.
//! Joining a room and publishing native device keys are separate operations.
use matrix_sdk::{
    Room, RoomMemberships,
    ruma::{OwnedUserId, events::room::message::RoomMessageEventContent},
};
use hagency::agent_encryption::wait_for_sendable_device;
use std::{collections::BTreeSet, time::Duration};

fn recipients(members: Vec<OwnedUserId>, mentions: &BTreeSet<OwnedUserId>) -> Vec<OwnedUserId> {
    let people: Vec<_> = members
        .into_iter()
        .filter(|u| u.localpart() != "_hagency_service")
        .collect();
    let direct = people.len() == 2;
    people
        .into_iter()
        .filter(|u| u.localpart().starts_with("_hagency_agt_") && (direct || mentions.contains(u)))
        .collect()
}

fn unjoined_agent_dm(target: Option<&matrix_sdk::ruma::UserId>, joined: &[OwnedUserId]) -> bool {
    matches!((target, joined), (Some(target), [member])
        if target.as_str() != member.as_str() && crate::hagency::backend::is_agent_invitation(target))
}

/// Uses the SDK's signature-checked device store and normal send policy. This
/// never marks a device trusted, shares old room keys, or falls back to plaintext.
pub(crate) async fn ensure_ready(
    room: &Room,
    message: &RoomMessageEventContent,
) -> Result<(), &'static str> {
    const KEYS_UNAVAILABLE: &str = "This Agent's device keys are not available; it may not have started on its computer yet. Your message was not sent. Check that its service is running, then try again.";
    tokio::time::timeout(Duration::from_secs(30), async {
        if !crate::room_encryption::confirmed_state(room)
            .await
            .map_err(|_| KEYS_UNAVAILABLE)?
            .is_encrypted()
        {
            return Ok(());
        }
        let members = room.members(RoomMemberships::JOIN).await.map_err(|_| KEYS_UNAVAILABLE)?;
        let joined: Vec<_> = members.into_iter().map(|m| m.user_id().to_owned()).collect();
        let targets = room.direct_targets();
        if targets.len() == 1 {
            let target = targets.iter().next().and_then(|id| matrix_sdk::ruma::UserId::parse(id.as_str()).ok());
            if unjoined_agent_dm(target.as_deref(), &joined) {
                return Err(crate::agent_access::AGENT_CHAT_GUIDANCE);
            }
        }
        let mentions = message
            .mentions
            .as_ref()
            .map(|m| m.user_ids.clone())
            .unwrap_or_default();
        let users = recipients(joined, &mentions);
        let client = room.client();
        for user in users {
            wait_for_sendable_device(&client, &user).await.map_err(|_| KEYS_UNAVAILABLE)?;
        }
        Ok(())
    })
    .await
    .unwrap_or(Err(KEYS_UNAVAILABLE))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn user(s: &str) -> OwnedUserId {
        s.try_into().unwrap()
    }
    #[test]
    fn only_a_lone_sender_with_a_managed_agent_dm_target_needs_binding_guidance() {
        use matrix_sdk::ruma::user_id;
        let agent = user_id!("@_hagency_agt_test:one.test");
        let user = user_id!("@member:two.test").to_owned();
        assert!(unjoined_agent_dm(Some(agent), std::slice::from_ref(&user)));
        assert!(!unjoined_agent_dm(Some(user_id!("@human:one.test")), std::slice::from_ref(&user)));
        assert!(!unjoined_agent_dm(None, std::slice::from_ref(&user)));
        assert!(!unjoined_agent_dm(Some(agent), &[user, agent.to_owned()]));
    }

    #[test]
    fn encrypted_agent_first_send_waits_only_for_the_addressed_agent() {
        let owner = user("@owner:test");
        let agent = user("@_hagency_agt_one:test");
        let service = user("@_hagency_service:test");
        let other = user("@member:test");
        assert_eq!(
            recipients(
                vec![owner.clone(), service.clone(), agent.clone()],
                &BTreeSet::new()
            ),
            vec![agent.clone()]
        );
        assert!(
            recipients(
                vec![owner.clone(), service.clone(), agent.clone(), other.clone()],
                &BTreeSet::new()
            )
            .is_empty()
        );
        assert_eq!(
            recipients(
                vec![owner.clone(), service, agent.clone(), other.clone()],
                &BTreeSet::from([agent.clone()])
            ),
            vec![agent]
        );
        assert!(recipients(vec![owner, other], &BTreeSet::new()).is_empty());
    }
}

#[cfg(test)]
#[path = "encrypted_send_http_tests.rs"]
mod http_tests;
