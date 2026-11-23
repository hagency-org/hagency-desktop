//! A restored tab can precede both its room-list entry and its timeline.
use super::*;
use matrix_sdk::{
    RoomState,
    config::RequestConfig,
    ruma::{
        api::{client::state::get_state_event_for_key, error::ErrorKind},
        events::StateEventType,
    },
};

const WAIT_SECONDS: f64 = 10.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Status {
    Waiting,
    TimedOut,
    Offline,
    Left,
    Banned,
    NotFound,
    Forbidden,
}

impl Status {
    fn message(self) -> &'static str {
        match self {
            Self::Waiting => "Waiting for this room to sync…",
            Self::TimedOut => "This room hasn’t loaded yet. You can retry or leave this tab open.",
            Self::Offline => "Offline. Waiting for the connection to return.",
            Self::Left => "You are no longer a member of this room.",
            Self::Banned => "You have been banned from this room.",
            Self::NotFound => "The server could not find this room.",
            Self::Forbidden => "You do not have access to this room.",
        }
    }

    fn membership(state: RoomState) -> Option<Self> {
        match state {
            RoomState::Left => Some(Self::Left),
            RoomState::Banned => Some(Self::Banned),
            _ => None,
        }
    }
}

#[derive(Default)]
pub(super) struct RoomRestore {
    pub(super) status: Option<Status>,
    timer: Timer,
    probe: Option<(u64, tokio::task::JoinHandle<()>)>,
}

impl RoomRestore {
    pub(super) fn clear(&mut self, cx: &mut Cx) {
        cx.stop_timer(self.timer);
        self.timer = Timer::empty();
        self.status = None;
        if let Some((_, probe)) = self.probe.take() {
            probe.abort();
        }
    }
}

impl Drop for RoomRestore {
    fn drop(&mut self) {
        if let Some((_, probe)) = self.probe.take() {
            probe.abort();
        }
    }
}

// The timer ID identifies the exact attempt, even after retry or tab reuse.
#[derive(Clone, Debug)]
struct ProbeResult {
    attempt: u64,
    epoch: u64,
    status: Status,
}

#[derive(Clone, Debug)]
pub(crate) struct RoomMembershipChanged(pub u64);

impl RoomScreen {
    pub(super) fn set_restore_status(&mut self, cx: &mut Cx, status: Status) {
        // While it restores a room this screen keeps events from its children, and a
        // history left open over the status page would cover it and answer nothing.
        self.close_room_history(cx);
        if status == Status::TimedOut {
            // The one-shot timer fired. A slower access check can still give
            // this attempt a definitive answer before the SDK request times out.
            self.restore.timer = Timer::empty();
        } else {
            self.restore.clear(cx);
        }
        self.restore.status = Some(status);
        self.view
            .restore_status_view(cx, ids!(restore_status_view))
            .show_room_status(
                cx,
                crate::i18n::tr(status.message()),
                status == Status::Waiting,
                status == Status::TimedOut,
            );
        self.redraw(cx);
    }

    pub(super) fn begin_room_restore(&mut self, cx: &mut Cx) {
        let Some(kind) = self.timeline_kind.as_ref() else {
            return;
        };
        let room_id = kind.room_id().clone();
        let client = crate::sliding_sync::get_client();
        let room = client.as_ref().and_then(|c| c.get_room(&room_id));
        if let Some(status) = room.as_ref().and_then(|r| Status::membership(r.state())) {
            self.set_restore_status(cx, status);
            return;
        }
        self.show_timeline(cx);
        if self.is_loaded {
            return;
        }
        if crate::sliding_sync::is_offline() {
            self.set_restore_status(cx, Status::Offline);
            return;
        }
        self.set_restore_status(cx, Status::Waiting);
        self.restore.timer = cx.start_timeout(WAIT_SECONDS);

        // A missing sidebar entry alone says nothing about access. Ask the
        // homeserver once about an event every room has. Sync owns recovery.
        if room.is_none()
            && let Some(client) = client
        {
            let attempt = self.restore.timer.0;
            let epoch = crate::account_session::epoch();
            let probe = crate::account_session::spawn(async move {
                let request = get_state_event_for_key::v3::Request::new(
                    room_id,
                    StateEventType::RoomCreate,
                    String::new(),
                );
                let result = client
                    .send(request)
                    .with_request_config(RequestConfig::default().retry_limit(0))
                    .await;
                let Err(error) = result else { return };
                let status = match error.client_api_error_kind() {
                    Some(ErrorKind::NotFound) => Status::NotFound,
                    Some(ErrorKind::Forbidden) => Status::Forbidden,
                    _ => return,
                };
                Cx::post_action(ProbeResult {
                    attempt,
                    epoch,
                    status,
                });
            });
            self.restore.probe = Some((attempt, probe));
        }
    }

    pub(super) fn handle_room_restore(
        &mut self,
        cx: &mut Cx,
        event: &Event,
        scope: &mut Scope,
    ) -> bool {
        if let Event::Actions(actions) = event
            && actions.iter().any(|a| matches!(a.downcast_ref(), Some(RoomMembershipChanged(epoch)) if *epoch == crate::account_session::epoch()))
            && let Some(kind) = &self.timeline_kind
            && let Some(room) = crate::sliding_sync::get_client().and_then(|c| c.get_room(kind.room_id()))
            && let Some(status) = Status::membership(room.state())
            && self.restore.status != Some(status)
        {
            invalidate_entire_room_timeline_states(cx, kind.room_id());
            self.hide_timeline();
            self.is_loaded = false;
            self.set_restore_status(cx, status);
        }
        if self.is_loaded || self.restore.status.is_none() {
            return false;
        }
        if self.restore.timer.is_event(event).is_some() && !self.restore.timer.is_empty() {
            self.set_restore_status(cx, Status::TimedOut);
        }
        if let Event::Actions(actions) = event {
            let view = self.view.restore_status_view(cx, ids!(restore_status_view));
            if view.button(cx, ids!(restore_close)).clicked(actions) {
                if crate::home::home_screen::effective_is_desktop(cx) {
                    let room_name_id = self.room_name_id.clone().unwrap();
                    let room = match self
                        .timeline_kind
                        .as_ref()
                        .and_then(|k| k.thread_root_event_id())
                    {
                        Some(root) => SelectedRoom::Thread {
                            room_name_id,
                            thread_root_event_id: root.clone(),
                        },
                        None => SelectedRoom::JoinedRoom { room_name_id },
                    };
                    cx.widget_action(
                        self.widget_uid(),
                        DockAction::TabCloseWasPressed(room.tab_id()),
                    );
                } else {
                    cx.widget_action(self.widget_uid(), StackNavigationAction::Pop);
                }
                return true;
            }
            if view.button(cx, ids!(restore_retry)).clicked(actions) {
                self.begin_room_restore(cx);
                return true;
            }
            for action in actions {
                if let Some(result) = action.downcast_ref::<ProbeResult>()
                    && result.epoch == crate::account_session::epoch()
                    && self
                        .restore
                        .probe
                        .as_ref()
                        .is_some_and(|(attempt, _)| result.attempt == *attempt)
                {
                    self.set_restore_status(cx, result.status);
                }
                if let Some(RoomsListHeaderAction::StateUpdate(state)) = action.downcast_ref() {
                    match (state, self.restore.status) {
                        (State::Offline, Some(Status::Waiting | Status::TimedOut)) => {
                            self.set_restore_status(cx, Status::Offline)
                        }
                        (State::Running, Some(Status::Offline)) => {
                            self.begin_room_restore(cx);
                            if self.is_loaded {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
                let Some(kind) = &self.timeline_kind else {
                    continue;
                };
                if let Some(
                    AppStateAction::RoomNameUpdated(name)
                    | AppStateAction::RoomLoadedSuccessfully {
                        room_name_id: name, ..
                    },
                ) = action.downcast_ref()
                    && name.room_id() == kind.room_id()
                {
                    self.room_name_id = Some(name.clone());
                }
                let room_ready = action
                    .downcast_ref::<TimelineEndpointsRecreated>()
                    .is_some_and(|a| &a.room_id == kind.room_id());
                let joined = matches!(action.downcast_ref(), Some(AppStateAction::RoomLoadedSuccessfully { room_name_id, is_invite: false }) if room_name_id.room_id() == kind.room_id());
                if (room_ready || joined) && !self.is_loaded {
                    // A queued notification from before a leave/ban must not
                    // revive the tab. A later confirmed rejoin can.
                    if matches!(
                        self.restore.status,
                        Some(Status::Left | Status::Banned | Status::Forbidden | Status::NotFound)
                    ) && !crate::sliding_sync::get_client()
                        .and_then(|c| c.get_room(kind.room_id()))
                        .is_some_and(|r| r.state() == RoomState::Joined)
                    {
                        continue;
                    }
                    self.show_timeline(cx);
                    if self.is_loaded {
                        return true;
                    }
                }
            }
        }
        // Only the restore controls are drawn. Forward their actions normally.
        self.view
            .restore_status_view(cx, ids!(restore_status_view))
            .handle_event(cx, event, scope);
        true
    }
}

#[cfg(test)]
impl RoomScreenRef {
    /// Drive the one-shot deadline without a wall-clock sleep.
    pub(crate) fn expire_room_restore(&self, cx: &mut Cx) {
        let timer = self.borrow().unwrap().restore.timer;
        assert!(!timer.is_empty());
        let event = Event::Timer(TimerEvent {
            timer_id: timer.0,
            time: None,
        });
        self.handle_event(cx, &event, &mut Scope::empty());
    }
}
