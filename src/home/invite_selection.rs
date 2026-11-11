//! The people and Agents chosen in the invite dialog, and what happened to
//! each one's invitation.
//!
//! This holds no widgets, so selection, de-duplication and per-invitee
//! results can be tested without a window.

use ruma::{OwnedUserId, UserId};

/// Where one invitee stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Chosen, not sent yet.
    Selected,
    /// The invitation request is in flight.
    Sending,
    /// A person was invited and has yet to accept.
    Invited,
    /// An Agent joined the room.
    Joined,
    /// The Agent request is saved but the Agent has not joined yet. Sending
    /// again only checks its status.
    AgentPending,
    /// Already in the room; nothing was sent.
    AlreadyMember,
    /// Already invited and yet to accept; nothing was sent.
    AlreadyInvited,
    /// The invitation failed, with a reason to show.
    Failed(String),
}

impl Outcome {
    /// Included in the next confirmation: never sent, failed, or an Agent
    /// whose status can be checked again.
    pub fn is_sendable(&self) -> bool {
        matches!(self, Outcome::Selected | Outcome::Failed(_) | Outcome::AgentPending)
    }

    /// Finished successfully. These are never sent again.
    pub fn is_done(&self) -> bool {
        matches!(self, Outcome::Invited | Outcome::Joined | Outcome::AlreadyMember | Outcome::AlreadyInvited)
    }

    /// The person can take this invitee out of the selection.
    pub fn is_removable(&self) -> bool {
        matches!(self, Outcome::Selected | Outcome::Failed(_))
    }
}

/// One chosen person or Agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invitee {
    pub id: OwnedUserId,
    pub name: Option<String>,
    pub outcome: Outcome,
}

impl Invitee {
    /// The display name, or the ID's localpart when there is none.
    pub fn label(&self) -> &str {
        self.name.as_deref().filter(|name| !name.trim().is_empty()).unwrap_or(self.id.localpart())
    }
}

/// Everyone chosen in the dialog, in the order they were chosen. Each Matrix
/// ID appears at most once.
#[derive(Clone, Debug, Default)]
pub struct InviteSelection {
    entries: Vec<Invitee>,
}

impl InviteSelection {
    pub fn entries(&self) -> &[Invitee] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: &UserId) -> Option<&Invitee> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn contains(&self, id: &UserId) -> bool {
        self.get(id).is_some()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Adds `id` unless it is already chosen. A known name fills in a
    /// missing one, so a typed ID still shows a name once one is known.
    /// Returns whether a new invitee was added.
    pub fn add(&mut self, id: OwnedUserId, name: Option<String>) -> bool {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            if entry.name.as_deref().is_none_or(|n| n.trim().is_empty()) && name.is_some() {
                entry.name = name;
            }
            return false;
        }
        self.entries.push(Invitee { id, name, outcome: Outcome::Selected });
        true
    }

    /// Removes `id` if it has not been sent successfully and is not being
    /// sent. Returns whether it was removed.
    pub fn remove(&mut self, id: &UserId) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else { return false };
        if !self.entries[index].outcome.is_removable() {
            return false;
        }
        self.entries.remove(index);
        true
    }

    /// Selects or deselects `id`. Invitees that are being sent, or were sent
    /// successfully, stay as they are. Returns whether `id` is selected after.
    pub fn toggle(&mut self, id: OwnedUserId, name: Option<String>) -> bool {
        if self.contains(&id) {
            !self.remove(&id)
        } else {
            self.add(id, name)
        }
    }

    /// How many invitees the next confirmation would send.
    pub fn sendable_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.outcome.is_sendable()).count()
    }

    /// Whether every invitee the next confirmation would send is an Agent
    /// that is still joining, so the action only checks status.
    pub fn only_pending_agents(&self) -> bool {
        let mut sendable = self.entries.iter().filter(|entry| entry.outcome.is_sendable()).peekable();
        sendable.peek().is_some() && sendable.all(|entry| entry.outcome == Outcome::AgentPending)
    }

    /// Marks every sendable invitee as being sent and returns them. Invitees
    /// that already succeeded are not returned, so they are never resent.
    pub fn begin_send(&mut self) -> Vec<OwnedUserId> {
        self.entries
            .iter_mut()
            .filter(|entry| entry.outcome.is_sendable())
            .map(|entry| {
                entry.outcome = Outcome::Sending;
                entry.id.clone()
            })
            .collect()
    }

    /// Records the result for an invitee that is being sent. Results for
    /// anyone else (removed, never sent, or already finished) are ignored.
    /// Returns whether the result was recorded.
    pub fn record(&mut self, id: &UserId, outcome: Outcome) -> bool {
        match self.entries.iter_mut().find(|entry| entry.id == id) {
            Some(entry) if entry.outcome == Outcome::Sending => {
                entry.outcome = outcome;
                true
            }
            _ => false,
        }
    }

    /// Whether `id` is chosen and its invitation is in flight.
    pub fn is_sending_to(&self, id: &UserId) -> bool {
        self.get(id).is_some_and(|entry| entry.outcome == Outcome::Sending)
    }

    pub fn is_sending(&self) -> bool {
        self.entries.iter().any(|entry| entry.outcome == Outcome::Sending)
    }

    /// Everyone chosen was invited, or was already a member.
    pub fn all_done(&self) -> bool {
        !self.entries.is_empty() && self.entries.iter().all(|entry| entry.outcome.is_done())
    }

    /// Invitees that have been sent at least once, whatever the result.
    pub fn attempted(&self) -> impl Iterator<Item = &Invitee> {
        self.entries.iter().filter(|entry| entry.outcome != Outcome::Selected)
    }

    /// The finished results so far: (succeeded, failed, Agents still joining).
    pub fn tally(&self) -> (usize, usize, usize) {
        self.entries.iter().fold((0, 0, 0), |(done, failed, pending), entry| match entry.outcome {
            Outcome::Invited | Outcome::Joined | Outcome::AlreadyMember | Outcome::AlreadyInvited => (done + 1, failed, pending),
            Outcome::Failed(_) => (done, failed + 1, pending),
            Outcome::AgentPending => (done, failed, pending + 1),
            Outcome::Selected | Outcome::Sending => (done, failed, pending),
        })
    }
}

/// Matrix servers report inviting a current member as an error. The dialog
/// shows that as "already a member" rather than as a failure.
pub fn is_already_member_error(error: &str) -> bool {
    let error = error.to_lowercase();
    ["already in the room", "already joined", "is already a member"].iter().any(|text| error.contains(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> OwnedUserId {
        text.try_into().unwrap()
    }

    #[test]
    fn toggling_selects_and_deselects_without_duplicates() {
        let mut selection = InviteSelection::default();
        assert!(selection.toggle(id("@alex:test"), Some("Alex".into())));
        assert!(selection.toggle(id("@_hagency_agt_coder:test"), None));
        assert!(!selection.add(id("@alex:test"), None), "The same ID is chosen only once");
        assert_eq!(selection.entries().len(), 2);
        assert_eq!(selection.sendable_count(), 2);
        assert!(!selection.toggle(id("@alex:test"), Some("Alex".into())));
        assert!(!selection.contains(&id("@alex:test")));
        assert_eq!(selection.sendable_count(), 1);
    }

    #[test]
    fn a_typed_id_joins_the_same_selection_and_learns_its_name() {
        let mut selection = InviteSelection::default();
        assert!(selection.add(id("@alex:test"), None));
        assert_eq!(selection.entries()[0].label(), "alex");
        assert!(!selection.add(id("@alex:test"), Some("Alex".into())));
        assert_eq!(selection.entries()[0].label(), "Alex");
        assert!(!selection.add(id("@alex:test"), Some("Someone else".into())), "A known name is kept");
        assert_eq!(selection.entries()[0].label(), "Alex");
    }

    #[test]
    fn partial_failure_keeps_failures_selected_and_never_resends_successes() {
        let mut selection = InviteSelection::default();
        for user in ["@a:test", "@b:test", "@_hagency_agt_c:test", "@d:test"] {
            selection.add(id(user), None);
        }
        let sent = selection.begin_send();
        assert_eq!(sent.len(), 4);
        assert!(selection.is_sending());
        assert_eq!(selection.sendable_count(), 0, "Nothing is sent twice while in flight");
        assert!(!selection.remove(&id("@a:test")), "An invitee being sent stays");
        assert!(selection.record(&id("@a:test"), Outcome::Invited));
        assert!(selection.record(&id("@b:test"), Outcome::Failed("Forbidden".into())));
        assert!(selection.record(&id("@_hagency_agt_c:test"), Outcome::Joined));
        assert!(selection.record(&id("@d:test"), Outcome::AlreadyMember));
        assert!(!selection.is_sending());
        assert_eq!(selection.tally(), (3, 1, 0));
        assert!(!selection.all_done());

        // A late or repeated result never changes a finished invitee.
        assert!(!selection.record(&id("@a:test"), Outcome::Failed("late".into())));
        assert_eq!(selection.get(&id("@a:test")).unwrap().outcome, Outcome::Invited);

        // Successes cannot be deselected; the failure can.
        assert!(selection.toggle(id("@a:test"), None), "A success stays selected");
        assert_eq!(selection.sendable_count(), 1);
        let retried = selection.begin_send();
        assert_eq!(retried, vec![id("@b:test")], "Only the failure is sent again");
        assert!(selection.record(&id("@b:test"), Outcome::Invited));
        assert!(selection.all_done());
        assert!(selection.begin_send().is_empty());
    }

    #[test]
    fn failed_invitees_can_be_removed_before_retrying() {
        let mut selection = InviteSelection::default();
        selection.add(id("@a:test"), None);
        selection.add(id("@b:test"), None);
        selection.begin_send();
        selection.record(&id("@a:test"), Outcome::Invited);
        selection.record(&id("@b:test"), Outcome::Failed("Forbidden".into()));
        assert!(!selection.remove(&id("@a:test")));
        assert!(selection.remove(&id("@b:test")));
        assert!(selection.all_done());
    }

    #[test]
    fn results_only_apply_to_invitees_being_sent() {
        let mut selection = InviteSelection::default();
        selection.add(id("@a:test"), None);
        assert!(!selection.record(&id("@a:test"), Outcome::Invited), "Not sent yet");
        assert!(!selection.record(&id("@stranger:test"), Outcome::Invited));
        selection.begin_send();
        assert!(selection.is_sending_to(&id("@a:test")));
        assert!(!selection.is_sending_to(&id("@stranger:test")));
        assert!(selection.record(&id("@a:test"), Outcome::Failed("timeout".into())));
        assert!(!selection.is_sending_to(&id("@a:test")));
    }

    #[test]
    fn a_pending_agent_is_checked_again_but_is_not_a_success() {
        let mut selection = InviteSelection::default();
        selection.add(id("@_hagency_agt_c:test"), None);
        selection.add(id("@a:test"), None);
        selection.begin_send();
        selection.record(&id("@_hagency_agt_c:test"), Outcome::AgentPending);
        selection.record(&id("@a:test"), Outcome::Invited);
        assert_eq!(selection.tally(), (1, 0, 1));
        assert!(selection.only_pending_agents());
        assert!(!selection.all_done());
        assert!(!selection.remove(&id("@_hagency_agt_c:test")), "A saved Agent request stays visible");
        assert_eq!(selection.begin_send(), vec![id("@_hagency_agt_c:test")]);
        selection.record(&id("@_hagency_agt_c:test"), Outcome::Joined);
        assert!(selection.all_done());
        assert!(!selection.only_pending_agents());
    }

    #[test]
    fn already_member_errors_are_recognised() {
        assert!(is_already_member_error("[403 / M_FORBIDDEN] @a:test is already in the room."));
        assert!(is_already_member_error("User is already joined to room"));
        assert!(!is_already_member_error("[403 / M_FORBIDDEN] You don't have permission to invite users"));
    }
}
