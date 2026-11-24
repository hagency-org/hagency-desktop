//! An `InviteScreen` widget shows information about a room invite.
//!
//! This is similar to how a `RoomScreen` shows the full timeline of a joined room,
//! but it only shows a simple summary of a room the current user has been invited to,
//! with buttons to accept or decline the invitation.

use std::ops::Deref;
use makepad_widgets::*;
use matrix_sdk::{Room, RoomMemberships, RoomState, ruma::{OwnedRoomId, UserId, events::room::member::MembershipState}};

use crate::{app::AppStateAction, avatar_cache::{self, AvatarCacheEntry}, block_user_modal::{BlockUserModalAction, BlockUserRequest}, home::rooms_list::RoomsListRef, join_leave_room_modal::{JoinLeaveModalKind, JoinLeaveRoomModalAction}, profile::user_profile::UserProfile, room::{BasicRoomDetails, FetchedRoomAvatar}, shared::{avatar::{AvatarState, AvatarWidgetRefExt}, restore_status_view::RestoreStatusViewWidgetExt}, sliding_sync::{get_client, is_user_blocked, spawn_async_task, submit_async_request, MatrixRequest}, utils::{self, RoomNameId}};

use super::rooms_list::{AcceptedInviteKind, InviteState, InviterInfo, RoomsListAction, get_invited_rooms, set_invite_state};


script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    let InviteMemberCell = View {
        width: 64 height: 84 flow: Down spacing: 2
        padding: Inset{top: 10 bottom: 4 left: 2 right: 2}
        align: Align{x: 0.5 y: 0.0}
        avatar := mod.widgets.MobileAvatar {width: 40 height: 40}
        name := Label {width: Fill height: Fit padding: 0 align: Align{x: 0.5} max_lines: 1 text_overflow: Ellipsis
            draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: (10 * mod.widgets.RINX_TEXT_SCALE)}}}
        note := Label {width: Fill height: Fit padding: 0 align: Align{x: 0.5} max_lines: 1 text_overflow: Ellipsis
            draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (8 * mod.widgets.RINX_TEXT_SCALE)}}}
    }

    mod.widgets.InviteScreen = set_type_default() do #(InviteScreen::register_widget(vm)) {
        ..mod.widgets.SolidView

        // make this a ScrollYView
        scroll_bars: mod.widgets.ScrollBars {
            show_scroll_x: false show_scroll_y: true
            scroll_bar_y.drag_scrolling: true
        }

        width: Fill,
        height: Fill,
        flow: Down,
        align: Align{x: 0.5, y: 0}
        padding: Inset{left: 20, right: 20, top: 50}
        spacing: 0,

        show_bg: true,
        draw_bg +: {
            color: mod.widgets.COLOR_PRIMARY_DARKER
        }

        restore_status_view := RestoreStatusView {}

        // This view is only shown if `inviter` is Some.
        inviter_view := View {
            width: Fill, height: Fit
            align: Align{x: 0.5, y: 0}
            spacing: 10,
            flow: Down,


            inviter_avatar := Avatar {
                width: 30,
                height: 30,
                text_view +: {
                    text +: {
                        draw_text +: {
                            text_style: TITLE_TEXT { font_size: (10.0 * mod.widgets.RINX_TEXT_SCALE) }
                        }
                    }
                }
            }


            inviter_name := Label {
                width: Fill, height: Fit,
                align: Align{x: 0.5, y: 0},
                margin: Inset{top: 2}
                padding: 0,
                flow: Flow.Right{wrap: true},
                text: ""
                draw_text +: {
                    text_style: TITLE_TEXT {
                        font_size: (15 * mod.widgets.RINX_TEXT_SCALE),
                    },
                    color: mod.widgets.RINX_INK
                }
            }

            inviter_user_id := Label {
                width: Fill, height: Fit,
                align: Align{x: 0.5, y: 0},
                margin: Inset{top: -3},
                flow: Flow.Right{wrap: true},
                text: ""
                draw_text +: {
                    text_style: TITLE_TEXT {
                        font_size: (10 * mod.widgets.RINX_TEXT_SCALE),
                    },
                    color: mod.widgets.RINX_MUTED
                }
            }

            LineH {
                width: 240,
                draw_bg.color: (COLOR_DIVIDER)
            }
        }

        invite_message := Label {
            margin: Inset{top: 15, bottom: 15},
            width: Fill, height: Fit,
            align: Align{x: 0.5, y: 0},
            flow: Flow.Right{wrap: true},
            text: "",
            draw_text +: {
                text_style: REGULAR_TEXT {
                    font_size: (15 * mod.widgets.RINX_TEXT_SCALE),
                },
                color: mod.widgets.RINX_INK
            }
        }

        room_view := View {
            width: Fill, height: Fit
            align: Align{x: 0.5, y: 0}
            spacing: 10,
            flow: Down,

            room_avatar := Avatar {
                width: 40,
                height: 40,

                text_view +: {
                    text +: {
                        draw_text +: {
                            text_style: TITLE_TEXT { font_size: (13.0 * mod.widgets.RINX_TEXT_SCALE) }
                        }
                    }
                }
            }

            room_name := Label {
                width: Fill, height: Fit,
                align: Align{x: 0.5, y: 0},
                text: ""
                // margin: Inset{top: 3}
                flow: Flow.Right{wrap: true},
                draw_text +: {
                    text_style: TITLE_TEXT {
                        font_size: (18 * mod.widgets.RINX_TEXT_SCALE),
                    },
                    color: mod.widgets.RINX_INK
                }
            }

            // Shown once it is known how many people are in the room.
            invite_members_view := View {
                visible: false
                width: Fill, height: Fit
                flow: Down
                align: Align{x: 0.5, y: 0}
                spacing: 4

                invite_members_title := Label {
                    width: Fill, height: Fit,
                    align: Align{x: 0.5, y: 0},
                    text: ""
                    draw_text +: {
                        text_style: REGULAR_TEXT {
                            font_size: (11 * mod.widgets.RINX_TEXT_SCALE),
                        },
                        color: mod.widgets.RINX_MUTED
                    }
                }

                // Its height follows the number of rows, up to `MAX_GRID_ROWS`.
                invite_members_grid := View {
                    visible: false
                    width: Fill, height: 84
                    invite_members_list := PortalList {
                        width: Fill, height: Fill
                        flow: Down
                        MemberRow := View {
                            width: Fill, height: 84
                            flow: Right
                            align: Align{x: 0.5, y: 0}
                            c0 := InviteMemberCell {} c1 := InviteMemberCell {} c2 := InviteMemberCell {}
                            c3 := InviteMemberCell {} c4 := InviteMemberCell {} c5 := InviteMemberCell {}
                        }
                        Filler := View {width: Fill, height: 1}
                    }
                }

                invite_members_partial := Label {
                    visible: false
                    width: Fill, height: Fit,
                    align: Align{x: 0.5, y: 0},
                    flow: Flow.Right{wrap: true},
                    text: #(crate::i18n::tr("Only some of the members are shown.")) i18n_text: "Only some of the members are shown."
                    draw_text +: {
                        text_style: REGULAR_TEXT {
                            font_size: (10 * mod.widgets.RINX_TEXT_SCALE),
                        },
                        color: mod.widgets.RINX_MUTED
                    }
                }
            }
        }

        buttons := View {
            width: Fill, height: Fit
            // We'd like to use RightWrap, but it doesn't work with x-centered alignment
            // flow: Flow.Right{wrap: true},
            flow: Right,
            align: Align{x: 0.5, y: 0.5}
            margin: Inset{top: 20}
            spacing: 40

            cancel_button := RobrixNegativeIconButton {
                align: Align{x: 0.5, y: 0.5}
                padding: 15,
                draw_icon.svg: (ICON_FORBIDDEN)
                icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                text: #(crate::i18n::tr("Reject Invite")) i18n_text: "Reject Invite"
            }

            accept_button := RobrixPositiveIconButton {
                align: Align{x: 0.5, y: 0.5}
                padding: 15,
                draw_icon.svg: (ICON_CHECKMARK)
                icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                text: #(crate::i18n::tr("Join Room")) i18n_text: "Join Room"
            }
        }

        // Put this on its own row with a bit of vertical space above it
        reject_and_block_button := RobrixNegativeIconButton {
            visible: false,
            align: Align{x: 0.5, y: 0.5}
            margin: Inset{top: 25}
            padding: 15,
            draw_icon.svg: (ICON_FORBIDDEN)
            icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
            text: #(crate::i18n::tr("Reject & Block Sender")) i18n_text: "Reject & Block Sender"
        }

        completion_label := Label {
            width: Fill, height: Fit,
            align: Align{x: 0.5, y: 0},
            margin: Inset{top: 10, bottom: 10},
            flow: Flow.Right{wrap: true},
            draw_text +: {
                color: (COLOR_FG_ACCEPT_GREEN),
                text_style: theme.font_bold {font_size: (12 * mod.widgets.RINX_TEXT_SCALE)}
            }
            text: ""
        }

        View {
            width: Fill, height: 30,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InviteDetails {
    pub room_info: BasicRoomDetails,
    pub inviter: Option<InviterInfo>,
}
impl Deref for InviteDetails {
    type Target = BasicRoomDetails;
    fn deref(&self) -> &Self::Target {
        &self.room_info
    }
}

/// Actions sent from the backend task as a result of a [`MatrixRequest::JoinRoom`].
///
/// Note that this *DOES NOT MEAN* that the room has actually been fully joined yet.
/// For that, you must wait for a [`AppStateAction::RoomLoadedSuccessfully`] action to occur.
#[derive(Debug)]
pub enum JoinRoomResultAction {
    /// The user has successfully joined the room.
    Joined {
        room_id: OwnedRoomId,
    },
    /// There was an error attempting to join the room.
    Failed {
        room_id: OwnedRoomId,
        error: matrix_sdk::Error,
    }
}

/// Actions sent from the backend task as a result of a [`MatrixRequest::LeaveRoom`].
///
/// Note that this *DOES NOT MEAN* that the room has actually been fully left yet.
#[derive(Debug)]
pub enum LeaveRoomResultAction {
    /// The user has successfully left the room.
    Left {
        room_id: OwnedRoomId,
    },
    /// There was an error attempting to leave the room.
    Failed {
        room_id: OwnedRoomId,
        error: matrix_sdk::Error,
    }
}


/// Actions that tell an `InviteScreen` to refresh part of its content.
#[derive(Debug)]
pub enum InviteScreenAction {
    /// We've fetched more info about who sent this invite.
    InviterInfoUpdated {
        room_id: OwnedRoomId,
        inviter_info: InviterInfo,
    },
    /// We've determined whether this invite is for a space.
    IsSpace {
        room_id: OwnedRoomId,
        is_space: bool,
    },
}


/// A person of a room the user is invited to, as the invitation names them.
#[derive(Clone, Debug)]
pub struct InviteMember {
    pub profile: UserProfile,
    /// Whether the person is invited too and has not answered.
    pub invited: bool,
    /// Whether this is the account of the Hagency service on the viewer's server.
    pub service: bool,
}

/// The people the invitation to `room_id` names, read for one request of the screen.
#[derive(Debug)]
pub struct InviteMembersLoaded {
    room_id: OwnedRoomId,
    request: u64,
    /// The account session the people were read in.
    epoch: u64,
    members: Vec<InviteMember>,
    /// How many people the homeserver says are in the room; 0 when it did not say.
    joined_count: u64,
}

/// The most cells one row of the grid can hold: the `MemberRow` template has this many.
const MAX_COLUMNS: usize = 6;
/// The fewest cells in a row, however narrow the screen is.
const MIN_COLUMNS: usize = 3;
/// The size of one cell, as in the `InviteMemberCell` template.
const CELL_SIZE: (f64, f64) = (64.0, 84.0);
/// The screen's padding on both sides together.
const SIDE_PADDING: f64 = 40.0;
/// The most rows the grid shows at once; it scrolls for more.
const MAX_GRID_ROWS: usize = 3;

/// The cells of a `MemberRow`, left to right.
macro_rules! member_cell_ids {
    () => { [ids!(c0), ids!(c1), ids!(c2), ids!(c3), ids!(c4), ids!(c5)] };
}

/// How many people fit side by side in a screen this wide.
fn member_columns(screen_width: f64) -> usize {
    (((screen_width - SIDE_PADDING) / CELL_SIZE.0) as usize).clamp(MIN_COLUMNS, MAX_COLUMNS)
}

/// The people to show for an invitation: those in the room, then those invited to it, each
/// by name. The invited user is not among them.
fn ordered_members(mut members: Vec<InviteMember>, viewer: &UserId) -> Vec<InviteMember> {
    members.retain(|member| member.profile.user_id != viewer);
    members.sort_by_cached_key(|member| (
        member.invited,
        member.profile.displayable_name().to_lowercase(),
        member.profile.user_id.clone(),
    ));
    members
}

/// What the stored invitation tells about the people of `room`.
///
/// The homeserver is not asked: it does not let an invited user read the room's member list.
/// What is known is what the invitation carried, which is everyone only when the homeserver
/// is set up to name the members in an invitation.
pub(crate) async fn stored_invite_members(room: &Room, viewer: &UserId) -> Vec<InviteMember> {
    let members = room
        .members_no_sync(RoomMemberships::JOIN | RoomMemberships::INVITE)
        .await
        .unwrap_or_default();
    ordered_members(
        members
            .into_iter()
            .map(|member| InviteMember {
                invited: *member.membership() == MembershipState::Invite,
                service: super::mobile_chat_info::is_hagency_service(member.user_id(), viewer),
                profile: UserProfile {
                    user_id: member.user_id().to_owned(),
                    username: member.display_name().map(ToOwned::to_owned),
                    avatar_state: AvatarState::Known(member.avatar_url().map(ToOwned::to_owned)),
                },
            })
            .collect(),
        viewer,
    )
}

/// What the screen shows of the room's people.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MembersSummary {
    /// How many people are in the room; nothing is shown for 0.
    joined: usize,
    /// Whether the people are shown one by one.
    grid: bool,
    /// Whether the room has more people than the invitation names.
    partial: bool,
}

fn members_summary(members: &[InviteMember], inviter: Option<&UserId>, joined_count: u64) -> MembersSummary {
    let listed = members.iter().filter(|member| !member.invited).count();
    let joined = listed.max(usize::try_from(joined_count).unwrap_or(usize::MAX));
    // The inviter is at the top of the screen already. An invitation that names nobody else
    // has no people to show.
    let grid = members.iter().any(|member| Some(&*member.profile.user_id) != inviter);
    MembersSummary { joined, grid, partial: grid && listed < joined }
}

/// A view that shows information about a room that the user has been invited to.
#[derive(Script, ScriptHook, Widget)]
pub struct InviteScreen {
    #[deref] view: View,

    #[rust] invite_state: InviteState,
    #[rust] info: Option<InviteDetails>,
    /// The name and ID of the invited room.
    #[rust] room_name_id: Option<RoomNameId>,
    #[rust] is_loaded: bool,
    #[rust] all_rooms_loaded: bool,
    #[rust] is_space: bool,
    /// The people the invitation names, once they are read.
    #[rust] members: Vec<InviteMember>,
    /// How many people the homeserver says are in the room; 0 when it did not say.
    #[rust] joined_count: u64,
    /// The latest request for the people; an answer to an earlier one is dropped.
    #[rust] members_request: u64,
}

impl Widget for InviteScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if let Event::Signal = event {
            // We use the avatar cache to populate the inviter's avatar.
            avatar_cache::process_avatar_updates(cx);

            // Otherwise, a Signal just means that the room might've been received from the homeserver.
            if let (false, Some(room_name_id), true) = (self.is_loaded, self.room_name_id.as_ref(), cx.has_global::<RoomsListRef>()) {
                let rooms_list_ref = cx.get_global::<RoomsListRef>();
                if !rooms_list_ref.is_room_loaded(room_name_id.room_id()) {
                    self.all_rooms_loaded = rooms_list_ref.all_rooms_loaded();
                    self.redraw(cx);
                    return;
                } else {
                    self.set_displayed_invite(cx, &room_name_id.clone());
                }
            }
        }

        self.view.handle_event(cx, event, scope);

        let orig_state = self.invite_state;

        // Handle any updates to this invite screen, e.g., it's been loaded or more inviter info has been fetched.
        if let Event::Actions(actions) = event {
            for action in actions {
                if let Some(AppStateAction::RoomNameUpdated(room_name_id)) = action.downcast_ref() {
                    self.update_displayed_name(cx, room_name_id);
                    continue;
                }
                if let Some(AppStateAction::RoomLoadedSuccessfully { room_name_id, .. }) = action.downcast_ref() {
                    if self.room_name_id.as_ref().is_some_and(|current| current.room_id() == room_name_id.room_id()) {
                        self.set_displayed_invite(cx, room_name_id);
                        break;
                    }
                    continue;
                }
                if let Some(InviteScreenAction::InviterInfoUpdated { room_id, inviter_info }) = action.downcast_ref() {
                    if self.room_name_id.as_ref().is_some_and(|r| r.room_id() == room_id) {
                        if let Some(info) = self.info.as_mut() {
                            info.inviter = Some(inviter_info.clone());
                        }
                        self.redraw(cx);
                    }
                    continue;
                }
                if let Some(loaded) = action.downcast_ref::<InviteMembersLoaded>() {
                    if loaded.request == self.members_request
                        && loaded.epoch == crate::account_session::epoch()
                        && self.room_name_id.as_ref().is_some_and(|r| r.room_id() == &loaded.room_id)
                    {
                        self.members = loaded.members.clone();
                        self.joined_count = loaded.joined_count;
                        self.view.portal_list(cx, ids!(invite_members_list)).set_first_id_and_scroll(0, 0.0);
                        self.redraw(cx);
                    }
                    continue;
                }
                if let Some(InviteScreenAction::IsSpace { room_id, is_space }) = action.downcast_ref() {
                    if self.room_name_id.as_ref().is_some_and(|r| r.room_id() == room_id) {
                        self.is_space = *is_space;
                        self.redraw(cx);
                    }
                    continue;
                }
            }

            let Some(info) = self.info.as_ref() else { return; };
            // Handle button clicks to accept or decline the invite
            if let Some(modifiers) = self.view.button(cx, ids!(cancel_button)).clicked_modifiers(actions) {
                if modifiers.shift {
                    submit_async_request(MatrixRequest::LeaveRoom {
                        room_id: info.room_id().clone(),
                    });
                    self.invite_state = InviteState::WaitingForLeaveResult;
                    set_invite_state(cx, info.room_id(), InviteState::WaitingForLeaveResult);
                } else {
                    cx.action(JoinLeaveRoomModalAction::Open {
                        kind: JoinLeaveModalKind::RejectInvite(info.clone()),
                        show_tip: true,
                    });
                }
            }
            // Blocking is a serious action, don't allow bypassing the confirmation modal
            if let Some(inviter) = info.inviter.as_ref()
                && self.view.button(cx, ids!(reject_and_block_button)).clicked(actions)
            {
                cx.action(BlockUserModalAction::Open(BlockUserRequest {
                    user_id: inviter.user_id.clone(),
                    display_name: inviter.display_name.clone(),
                    block: true,
                    reject_invite_to: Some(info.room_id().clone()),
                }));
            }
            if let Some(modifiers) = self.view.button(cx, ids!(accept_button)).clicked_modifiers(actions) {
                if modifiers.shift {
                    submit_async_request(MatrixRequest::JoinRoom {
                        room_id: info.room_id().clone(),
                    });
                    self.invite_state = InviteState::WaitingForJoinResult;
                    set_invite_state(cx, info.room_id(), InviteState::WaitingForJoinResult);
                } else {
                    cx.action(JoinLeaveRoomModalAction::Open {
                        kind: JoinLeaveModalKind::AcceptInvite(info.clone()),
                        show_tip: true,
                    });
                }
            }

            for action in actions {
                match action.downcast_ref() {
                    // The success/failure popups are shown by the backend task, which is
                    // more consistent than doing it here since the user may have left this screen already.
                    Some(JoinRoomResultAction::Joined { room_id }) if room_id == info.room_id() => {
                        self.invite_state = InviteState::WaitingForJoinedRoom;
                        continue;
                    }
                    Some(JoinRoomResultAction::Failed { room_id, .. }) if room_id == info.room_id() => {
                        self.invite_state = InviteState::WaitingOnUserInput;
                        continue;
                    }
                    _ => {}
                }

                match action.downcast_ref() {
                    Some(LeaveRoomResultAction::Left { room_id }) if room_id == info.room_id() => {
                        self.invite_state = InviteState::RoomLeft;
                        continue;
                    }
                    Some(LeaveRoomResultAction::Failed { room_id, .. }) if room_id == info.room_id() => {
                        self.invite_state = InviteState::WaitingOnUserInput;
                        continue;
                    }
                    _ => {}
                }

                if let Some(BlockUserModalAction::Confirmed(request)) = action.downcast_ref() {
                    if request.reject_invite_to.as_ref() == Some(info.room_id()) {
                        self.invite_state = InviteState::WaitingForLeaveResult;
                    }
                    continue;
                }

                if let Some(JoinLeaveRoomModalAction::Close { room_id, .. }) = action.downcast_ref() {
                    if room_id == info.room_id() {
                        // check the latest invite state for this room, as it may have changed
                        // even after the modal was closed.
                        let current = get_invited_rooms(cx)
                            .borrow()
                            .get(room_id)
                            .map(|invite| invite.invite_state);
                        if let Some(state) = current {
                            self.invite_state = state;
                        }
                    }
                    continue;
                }
            }
        }

        if self.invite_state != orig_state {
            self.redraw(cx);
        }
    }


    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if !self.is_loaded {
            let mut restore_status_view = self.view.restore_status_view(cx, ids!(restore_status_view));
            if let Some(room_name) = &self.room_name_id {
                restore_status_view.set_content(cx, self.all_rooms_loaded, room_name);
            }
            return restore_status_view.draw(cx, scope);
        }
        let Some(info) = self.info.as_ref() else {
            // If we don't have any info, just return.
            return self.draw_view(cx, scope, walk);
        };

        // First, populate the inviter info, if we have it.
        let inviter_view = self.view.view(cx, ids!(inviter_view));
        let (is_visible, invite_text) = if let Some(inviter) = info.inviter.as_ref() {
            let inviter_avatar = inviter_view.avatar(cx, ids!(inviter_avatar));
            let mut drew_avatar = false;
            if let Some(uri) = inviter.avatar_url.as_ref()
                && let AvatarCacheEntry::Loaded(data) = avatar_cache::get_or_fetch_avatar(cx, uri)
            {
                drew_avatar = inviter_avatar.show_image(
                    cx,
                    None, // don't make this avatar clickable.
                    |cx, img| utils::load_avatar_image(&img, cx, &(uri.clone(), data).into()),
                ).is_ok();
            }
            if !drew_avatar {
                inviter_avatar.show_user_text(
                    cx,
                    &inviter.user_id,
                    inviter.display_name.as_deref().unwrap_or_else(|| inviter.user_id.as_str()),
                );
            }
            let inviter_name = inviter_view.label(cx, ids!(inviter_name));
            let inviter_user_id = inviter_view.label(cx, ids!(inviter_user_id));
            if let Some(inviter_user_name) = inviter.display_name.as_deref() {
                // If we have an inviter display name, show that *and* the user ID.
                inviter_name.set_text(cx, inviter_user_name);
                inviter_user_id.set_visible(cx, true);
                inviter_user_id.set_text(cx, inviter.user_id.as_str());
            }
            else {
                // If we only have a user ID, show it in the user_name field,
                // and hide the user ID field.
                inviter_name.set_text(cx, inviter.user_id.as_str());
                inviter_user_id.set_visible(cx, false);
            }
            (true, "has invited you to join:")
        }
        else {
            (false, "You have been invited to join:")
        };
        inviter_view.set_visible(cx, is_visible);
        self.view.label(cx, ids!(invite_message)).set_text(cx, invite_text);

        // Second, populate the room info, if we have it.
        let room_view = self.view.view(cx, ids!(room_view));
        let room_avatar = room_view.avatar(cx, ids!(room_avatar));
        match &info.room_avatar() {
            FetchedRoomAvatar::Members(_) => room_avatar.show_room_avatar(cx, info.room_avatar()),
            FetchedRoomAvatar::Text(text) => {
                room_avatar.show_text(
                    cx,
                    None,
                    None, // don't make this avatar clickable.
                    text,
                );
            }
            FetchedRoomAvatar::Image(avatar_image) => {
                let _ = room_avatar.show_image(
                    cx,
                    None, // don't make this avatar clickable.
                    |cx, img| utils::load_avatar_image(&img, cx, avatar_image),
                );
            }
        }
        let invite_room_label = info.room_name_id().to_string();
        room_view.label(cx, ids!(room_name)).set_text(cx, &invite_room_label);

        // The people of the room, as far as the invitation names them.
        let summary = members_summary(
            &self.members,
            info.inviter.as_ref().map(|inviter| &*inviter.user_id),
            self.joined_count,
        );
        let invite_members_view = room_view.view(cx, ids!(invite_members_view));
        invite_members_view.set_visible(cx, summary.joined > 0);
        invite_members_view.label(cx, ids!(invite_members_title)).set_text(
            cx,
            &crate::i18n::format("{0} members", &[("0", summary.joined.to_string())]),
        );
        invite_members_view.label(cx, ids!(invite_members_partial)).set_visible(cx, summary.partial);
        let invite_members_grid = invite_members_view.view(cx, ids!(invite_members_grid));
        invite_members_grid.set_visible(cx, summary.grid);
        let rows = self.members.len().div_ceil(member_columns(cx.peek_walk_turtle(walk).size.x));
        if let Some(mut grid) = invite_members_grid.borrow_mut() {
            grid.walk.height = Size::Fixed(rows.clamp(1, MAX_GRID_ROWS) as f64 * CELL_SIZE.1);
        }

        // Third, set the buttons' text based on the invite state.
        let cancel_button = self.view.button(cx, ids!(cancel_button));
        let accept_button = self.view.button(cx, ids!(accept_button));
        let reject_and_block_button = self.view.button(cx, ids!(reject_and_block_button));
        reject_and_block_button.set_visible(
            cx,
            info.inviter.as_ref().is_some_and(|i| !is_user_blocked(&i.user_id)),
        );
        let join_text = match self.is_space { true => "Join Space", false => crate::i18n::tr("Join Room") };
        match self.invite_state {
            InviteState::WaitingOnUserInput => {
                cancel_button.set_enabled(cx, true);
                accept_button.set_enabled(cx, true);
                reject_and_block_button.set_enabled(cx, true);
                cancel_button.set_text(cx, crate::i18n::tr("Reject Invite"));
                accept_button.set_text(cx, join_text);
            }
            InviteState::WaitingForJoinResult => {
                cancel_button.set_enabled(cx, false);
                accept_button.set_enabled(cx, false);
                reject_and_block_button.set_enabled(cx, false);
                cancel_button.set_text(cx, crate::i18n::tr("Reject Invite"));
                accept_button.set_text(cx, crate::i18n::tr("Joining..."));
            }
            InviteState::WaitingForLeaveResult => {
                cancel_button.set_enabled(cx, false);
                accept_button.set_enabled(cx, false);
                reject_and_block_button.set_enabled(cx, false);
                cancel_button.set_text(cx, crate::i18n::tr("Rejecting..."));
                accept_button.set_text(cx, join_text);
            }
            InviteState::WaitingForJoinedRoom => {
                cancel_button.set_enabled(cx, false);
                accept_button.set_enabled(cx, false);
                reject_and_block_button.set_enabled(cx, false);
                cancel_button.set_text(cx, crate::i18n::tr("Reject Invite"));
                accept_button.set_text(cx, crate::i18n::tr("Joined!"));
            }
            InviteState::RoomLeft => {
                cancel_button.set_visible(cx, false);
                accept_button.set_visible(cx, false);
                reject_and_block_button.set_visible(cx, false);
                self.view.label(cx, ids!(completion_label)).set_text(
                    cx,
                    "Invite successfully rejected. You may close this invite.",
                );
            }
        }

        self.draw_view(cx, scope, walk)
    }
}

impl InviteScreen {
    /// Draws the screen and, in it, the people the invitation names.
    fn draw_view(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let columns = member_columns(cx.peek_walk_turtle(walk).size.x);
        let rows = self.members.len().div_ceil(columns);
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            let Some(mut list) = item.borrow_mut::<PortalList>() else { continue };
            list.set_item_range(cx, 0, rows);
            while let Some(index) = list.next_visible_item(cx) {
                if index >= rows {
                    list.item(cx, index, id!(Filler)).draw_all(cx, scope);
                    continue;
                }
                let row = list.item(cx, index, id!(MemberRow));
                for (slot, id) in member_cell_ids!().into_iter().enumerate() {
                    let cell = row.view(cx, id);
                    let member = (slot < columns)
                        .then(|| self.members.get_mut(index * columns + slot))
                        .flatten();
                    cell.set_visible(cx, member.is_some());
                    let Some(member) = member else { continue };
                    let avatar = cell.avatar(cx, ids!(avatar));
                    let loaded = member.profile.avatar_state.update_from_cache(cx).is_some_and(|image| {
                        avatar.show_image(cx, None, |cx, img| utils::load_avatar_image(&img, cx, image)).is_ok()
                    });
                    if !loaded {
                        avatar.show_user_text(cx, &member.profile.user_id, member.profile.displayable_name());
                    }
                    // As in the room's own member list.
                    let name = if member.service {
                        crate::i18n::tr("Hagency service")
                    } else {
                        member.profile.displayable_name()
                    };
                    let note = if member.invited {
                        crate::i18n::tr("Invited")
                    } else {
                        crate::agent_access::framework_label(&member.profile.user_id).unwrap_or("")
                    };
                    cell.label(cx, ids!(name)).set_text(cx, name);
                    cell.label(cx, ids!(note)).set_text(cx, note);
                }
                row.draw_all(cx, scope);
            }
        }
        DrawStep::done()
    }

    /// Reads the people the invitation to `room_id` names. The answer comes as an
    /// [`InviteMembersLoaded`] action.
    fn load_members(&mut self, room_id: &OwnedRoomId) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.members_request = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let Some(client) = get_client() else { return };
        let Some(viewer) = client.user_id().map(ToOwned::to_owned) else { return };
        let (request, epoch, room_id) = (self.members_request, crate::account_session::epoch(), room_id.clone());
        spawn_async_task(async move {
            let Some(room) = client.get_room(&room_id) else { return };
            let members = stored_invite_members(&room, &viewer).await;
            let joined_count = room.joined_members_count();
            Cx::post_action(InviteMembersLoaded { room_id, request, epoch, members, joined_count });
        });
    }

    /// Refresh metadata without resetting an in-flight Join or Reject operation.
    fn update_displayed_name(&mut self, cx: &mut Cx, room_name_id: &RoomNameId) {
        if !self.room_name_id.as_ref().is_some_and(|current| current.room_id() == room_name_id.room_id()) {
            return;
        }
        let invites = get_invited_rooms(cx);
        let invites = invites.borrow();
        let latest = invites.get(room_name_id.room_id());
        let room_name_id = latest.map_or(room_name_id, |invite| &invite.room_name_id);
        self.room_name_id = Some(room_name_id.clone());
        if let Some(info) = self.info.as_mut() {
            info.room_info = BasicRoomDetails::NameAndAvatar {
                room_name_id: room_name_id.clone(),
                room_avatar: latest.map_or_else(|| info.room_avatar().clone(), |invite| invite.room_avatar.clone()),
            };
        }
        self.redraw(cx);
    }

    /// Sets the ID of the invited room that will be displayed by this screen.
    pub fn set_displayed_invite(&mut self, cx: &mut Cx, room_name_id: &RoomNameId) {
        if !self.room_name_id.as_ref().is_some_and(|current| current.room_id() == room_name_id.room_id()) {
            // The people of another invitation.
            self.members.clear();
            self.joined_count = 0;
        }
        self.room_name_id = Some(room_name_id.clone());
        if let Some(invite) = get_invited_rooms(cx)
            .borrow()
            .get(room_name_id.room_id())
        {
            // Dock restoration and same-tab selections can carry an older name.
            self.room_name_id = Some(invite.room_name_id.clone());
            self.info = Some(InviteDetails {
                room_info: BasicRoomDetails::NameAndAvatar {
                    room_name_id: invite.room_name_id.clone(),
                    room_avatar: invite.room_avatar.clone(),
                },
                inviter: invite.inviter_info.clone(),
            });
            self.invite_state = invite.invite_state;
            self.is_space = invite.is_space;
            self.is_loaded = true;
            self.all_rooms_loaded = true;
            let room_id = invite.room_name_id.room_id().clone();
            self.load_members(&room_id);
            self.redraw(cx);
        }
        // If this invite has already been accepted (e.g., in another client, or while Rinx was offline),
        // we need to handle that and upgrade this screen to the corresponding joined room's RoomScreen.
        else if cx.has_global::<RoomsListRef>()
            && cx.get_global::<RoomsListRef>().get_room_state(room_name_id.room_id()) == Some(RoomState::Joined)
        {
            // Use the joined room's most current name, of course.
            let room_name_id = cx.get_global::<RoomsListRef>()
                .get_room_name(room_name_id.room_id())
                .unwrap_or_else(|| room_name_id.clone());

            self.is_loaded = true;
            cx.widget_action(
                self.widget_uid(),
                RoomsListAction::InviteAccepted { room_name_id, kind: AcceptedInviteKind::Room },
            );
            return;
        }

        let restore_status_view = self.view.restore_status_view(cx, ids!(restore_status_view));
        if !self.is_loaded {
            restore_status_view.set_content(
                cx,
                self.all_rooms_loaded,
                room_name_id,
            );
            restore_status_view.set_visible(cx, true);
        } else {
            restore_status_view.set_visible(cx, false);
        }
    }

    pub fn hide_displayed_invite(&mut self, cx: &mut Cx) {
        let cancel_button = self.view.button(cx, ids!(cancel_button));
        cancel_button.set_visible(cx, true);
        cancel_button.reset_hover(cx);
        let accept_button = self.view.button(cx, ids!(accept_button));
        accept_button.set_visible(cx, true);
        accept_button.reset_hover(cx);
        self.view.button(cx, ids!(reject_and_block_button)).reset_hover(cx);
        self.view.label(cx, ids!(completion_label)).set_text(cx, "");
        self.room_name_id = None;
        self.info = None;
        self.invite_state = InviteState::default();
        self.is_space = false;
        self.is_loaded = false;
        self.all_rooms_loaded = false;
        self.members.clear();
        self.joined_count = 0;
        self.members_request = 0;
        self.view.view(cx, ids!(invite_members_view)).set_visible(cx, false);
        self.view.restore_status_view(cx, ids!(restore_status_view)).set_visible(cx, false);
        self.redraw(cx);
    }
}

impl InviteScreenRef {
    /// See [`InviteScreen::set_displayed_invite()`].
    pub fn set_displayed_invite(&self, cx: &mut Cx, room_name_id: &RoomNameId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_displayed_invite(cx, room_name_id);
        }
    }

    pub fn hide_displayed_invite(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.hide_displayed_invite(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::rooms_list::InvitedRoomInfo;
    use matrix_sdk::RoomDisplayName;

    fn screen() -> (Cx, InviteScreen) {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        get_invited_rooms(&mut cx).borrow_mut().clear();
        let screen = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            let value = script_eval!(vm, {mod.widgets.InviteScreen {}});
            InviteScreen::script_from_value(vm, value)
        });
        (cx, screen)
    }

    fn name(id: &str, name: &str) -> RoomNameId {
        RoomNameId::new(RoomDisplayName::Named(name.into()), id.parse().unwrap())
    }

    fn remember(cx: &mut Cx, room: RoomNameId, is_space: bool) {
        get_invited_rooms(cx).borrow_mut().insert(room.room_id().clone(), InvitedRoomInfo {
            room_name_id: room,
            canonical_alias: None,
            alt_aliases: Vec::new(),
            room_avatar: FetchedRoomAvatar::Text("D".into()),
            inviter_info: None,
            invite_state: InviteState::WaitingOnUserInput,
            is_selected: false,
            is_direct: false,
            is_space,
        });
    }

    fn rename_event(screen: &mut InviteScreen, cx: &mut Cx, room: RoomNameId) {
        let actions = vec![Box::new(AppStateAction::RoomNameUpdated(room)) as Action];
        screen.handle_event(cx, &Event::Actions(actions), &mut Scope::empty());
    }

    #[test]
    fn invite_detail_tracks_late_room_name_without_resetting_pending_action() {
        let (mut cx, mut screen) = screen();
        let old = name("!project:example.test", "miniapp_manager");
        remember(&mut cx, old.clone(), false);
        screen.set_displayed_invite(&mut cx, &old);
        screen.invite_state = InviteState::WaitingForJoinResult;
        let latest = name("!project:example.test", "Project name from invite state");
        remember(&mut cx, latest.clone(), false);
        rename_event(&mut screen, &mut cx, latest.clone());
        assert_eq!(screen.room_name_id.as_ref().unwrap().display(), latest.display());
        let details = screen.info.as_ref().unwrap();
        assert_eq!(details.room_name_id().display(), latest.display());
        assert_eq!(details.room_id(), old.room_id(), "confirmation retains the original room");
        assert_eq!(screen.invite_state, InviteState::WaitingForJoinResult);
        assert!(screen.is_loaded);

        rename_event(&mut screen, &mut cx, name("!other:example.test", "Other room"));
        assert_eq!(screen.info.as_ref().unwrap().room_name_id().display(), latest.display());
        assert_eq!(screen.invite_state, InviteState::WaitingForJoinResult);
    }

    #[test]
    fn invite_detail_restoration_and_older_actions_use_latest_cached_name() {
        let (mut cx, mut screen) = screen();
        let old = name("!space:example.test", "miniapp_manager");
        let latest = name("!space:example.test", "Project space");
        remember(&mut cx, latest.clone(), true);
        screen.set_displayed_invite(&mut cx, &old);
        assert_eq!(screen.info.as_ref().unwrap().room_name_id().display(), latest.display());
        assert_eq!(screen.room_name_id.as_ref().unwrap().display(), latest.display());
        assert!(screen.is_space);
        screen.invite_state = InviteState::WaitingForLeaveResult;
        rename_event(&mut screen, &mut cx, old);
        assert_eq!(screen.info.as_ref().unwrap().room_name_id().display(), latest.display());
        assert_eq!(screen.invite_state, InviteState::WaitingForLeaveResult);
    }

    #[test]
    fn invite_detail_waiting_for_room_tracks_name_without_inventing_loaded_state() {
        let (mut cx, mut screen) = screen();
        let old = RoomNameId::empty("!pending:example.test".parse().unwrap());
        screen.set_displayed_invite(&mut cx, &old);
        let latest = name("!pending:example.test", "Arriving project");
        rename_event(&mut screen, &mut cx, latest.clone());
        assert_eq!(screen.room_name_id.as_ref().unwrap().display(), latest.display());
        assert!(!screen.is_loaded);
        assert!(screen.info.is_none());
    }

    fn person(id: &str, name: Option<&str>, invited: bool) -> InviteMember {
        InviteMember {
            profile: UserProfile {
                user_id: id.parse().unwrap(),
                username: name.map(Into::into),
                avatar_state: AvatarState::Known(None),
            },
            invited,
            service: false,
        }
    }

    fn ids(members: &[InviteMember]) -> Vec<&str> {
        members.iter().map(|member| member.profile.user_id.as_str()).collect()
    }

    #[test]
    fn invite_members_are_the_room_first_then_the_invited_and_never_the_viewer() {
        let viewer: &UserId = "@me:example.test".try_into().unwrap();
        let members = ordered_members(vec![
            person("@pending:example.test", Some("Alba"), true),
            person("@me:example.test", Some("Me"), true),
            person("@zed:example.test", Some("zed"), false),
            person("@nameless:example.test", None, false),
            person("@bea:example.test", Some("Bea"), false),
        ], viewer);
        // People without a name sort by their user ID, which starts with "@".
        assert_eq!(ids(&members), [
            "@nameless:example.test", "@bea:example.test", "@zed:example.test", "@pending:example.test",
        ]);
        assert!(members.last().unwrap().invited);
    }

    #[test]
    fn invite_people_section_follows_what_the_invitation_names() {
        let inviter: &UserId = "@inviter:example.test".try_into().unwrap();
        let alone = [person("@inviter:example.test", Some("Inviter"), false)];
        let named = [
            person("@inviter:example.test", Some("Inviter"), false),
            person("@bea:example.test", Some("Bea"), false),
            person("@pending:example.test", None, true),
        ];
        let summary = |members: &[InviteMember], joined_count| members_summary(members, Some(inviter), joined_count);
        // Nothing is known about the room's people.
        assert_eq!(summary(&[], 0), MembersSummary { joined: 0, grid: false, partial: false });
        // A homeserver that names nobody but the inviter: the number alone.
        assert_eq!(summary(&alone, 5), MembersSummary { joined: 5, grid: false, partial: false });
        assert_eq!(summary(&alone, 0), MembersSummary { joined: 1, grid: false, partial: false });
        // Everyone is named. A person who is only invited is shown and not counted.
        assert_eq!(summary(&named, 2), MembersSummary { joined: 2, grid: true, partial: false });
        assert_eq!(summary(&named, 0), MembersSummary { joined: 2, grid: true, partial: false });
        // The room has more people than the invitation names.
        assert_eq!(summary(&named, 7), MembersSummary { joined: 7, grid: true, partial: true });
        // Without a known inviter, everyone named is somebody to show.
        assert_eq!(members_summary(&alone, None, 1), MembersSummary { joined: 1, grid: true, partial: false });
    }

    #[test]
    fn invite_grid_has_three_to_six_people_in_a_row() {
        assert_eq!(member_columns(0.0), MIN_COLUMNS);
        assert_eq!(member_columns(SIDE_PADDING + 4.0 * CELL_SIZE.0 + 1.0), 4);
        assert_eq!(member_columns(2000.0), MAX_COLUMNS);
        assert_eq!(member_cell_ids!().len(), MAX_COLUMNS);
    }

    fn loaded(screen: &mut InviteScreen, cx: &mut Cx, room: &RoomNameId, request: u64, members: Vec<InviteMember>) {
        let action = InviteMembersLoaded {
            room_id: room.room_id().clone(),
            request,
            epoch: crate::account_session::epoch(),
            members,
            joined_count: 2,
        };
        screen.handle_event(cx, &Event::Actions(vec![Box::new(action) as Action]), &mut Scope::empty());
    }

    #[test]
    fn invite_people_of_an_earlier_request_or_another_invite_are_dropped() {
        let (mut cx, mut screen) = screen();
        let first = name("!first:example.test", "First");
        let second = name("!second:example.test", "Second");
        remember(&mut cx, first.clone(), false);
        remember(&mut cx, second.clone(), false);
        screen.set_displayed_invite(&mut cx, &first);
        let request = screen.members_request;
        assert_ne!(request, 0);

        loaded(&mut screen, &mut cx, &second, request, vec![person("@other:example.test", None, false)]);
        assert!(screen.members.is_empty(), "another invitation's people");
        loaded(&mut screen, &mut cx, &first, request - 1, vec![person("@old:example.test", None, false)]);
        assert!(screen.members.is_empty(), "an earlier request's people");
        loaded(&mut screen, &mut cx, &first, request, vec![person("@bea:example.test", Some("Bea"), false)]);
        assert_eq!(ids(&screen.members), ["@bea:example.test"]);
        assert_eq!(screen.joined_count, 2);

        // The same invitation shown again keeps its people until the new answer comes.
        screen.set_displayed_invite(&mut cx, &first);
        assert_eq!(ids(&screen.members), ["@bea:example.test"]);
        assert_ne!(screen.members_request, request);
        // Another invitation starts with none, and the first one's late answer is not its own.
        screen.set_displayed_invite(&mut cx, &second);
        assert!(screen.members.is_empty());
        assert_eq!(screen.joined_count, 0);
        let request = screen.members_request;
        loaded(&mut screen, &mut cx, &first, request, vec![person("@bea:example.test", None, false)]);
        assert!(screen.members.is_empty());

        screen.hide_displayed_invite(&mut cx);
        assert!(screen.members.is_empty());
        assert_eq!(screen.members_request, 0);
    }

    /// The Matrix library keeps every member an invitation names and gives them back for
    /// the invited room without a request, which the homeserver would refuse.
    #[test]
    fn invite_members_named_by_the_invitation_are_read_without_asking_the_homeserver() {
        use wiremock::{Mock, MockServer, ResponseTemplate, matchers::{method, path_regex}};
        const ROOM: &str = "!invited:example.org";
        const ME: &str = "@me:example.org";
        fn member(user: &str, membership: &str, name: Option<&str>) -> serde_json::Value {
            serde_json::json!({
                "type": "m.room.member", "state_key": user, "sender": "@alice:example.org",
                "content": {"membership": membership, "displayname": name}
            })
        }
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            let server = MockServer::start().await;
            let invited = serde_json::json!({ ROOM: {"invite_state": {"events": [
                {"type": "m.room.create", "state_key": "", "sender": "@alice:example.org", "content": {"room_version": "11"}},
                {"type": "m.room.name", "state_key": "", "sender": "@alice:example.org", "content": {"name": "Project"}},
                member("@alice:example.org", "join", Some("Alice")),
                member("@carol:example.org", "join", None),
                member("@bob:example.org", "join", Some("bob")),
                member("@_hagency_service:example.org", "join", None),
                member("@dana:example.org", "invite", Some("Dana")),
                member(ME, "invite", Some("Me")),
            ]}}});
            Mock::given(method("GET"))
                .and(path_regex(".*/sync"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"next_batch": "initial", "rooms": {"invite": invited}}),
                ))
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path_regex(".*/members"))
                .respond_with(ResponseTemplate::new(403).set_body_json(
                    serde_json::json!({"errcode": "M_FORBIDDEN", "error": "Not a member."}),
                ))
                .expect(0)
                .mount(&server)
                .await;
            let client = matrix_sdk::Client::builder()
                .homeserver_url(server.uri())
                .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_11])
                .build()
                .await
                .unwrap();
            client
                .matrix_auth()
                .restore_session(
                    serde_json::from_value(serde_json::json!({
                        "access_token": "fixture", "user_id": ME, "device_id": "INVITEMEMBERS"
                    }))
                    .unwrap(),
                    Default::default(),
                )
                .await
                .unwrap();
            client
                .sync_once(matrix_sdk::config::SyncSettings::default().timeout(std::time::Duration::ZERO))
                .await
                .unwrap();
            let room = client.get_room(&OwnedRoomId::try_from(ROOM).unwrap()).unwrap();
            assert_eq!(room.state(), RoomState::Invited);

            let members = stored_invite_members(&room, client.user_id().unwrap()).await;
            assert_eq!(ids(&members), [
                "@_hagency_service:example.org", "@carol:example.org", "@alice:example.org", "@bob:example.org",
                "@dana:example.org",
            ]);
            assert_eq!(members[2].profile.username.as_deref(), Some("Alice"));
            assert_eq!(members.iter().map(|member| member.invited).collect::<Vec<_>>(), [false, false, false, false, true]);
            assert_eq!(members.iter().map(|member| member.service).collect::<Vec<_>>(), [true, false, false, false, false]);
            server.verify().await;
        });
    }

    /// One draw of the screen at `width`, as a window would do it.
    fn draw(screen: &mut InviteScreen, cx: &mut Cx, width: f64) {
        use makepad_widgets::makepad_draw::cx_draw::CxDraw;
        let size = dvec2(width, 775.0);
        cx.redraw_all();
        let pass = DrawPass::new(cx);
        pass.set_size(cx, size);
        let mut list = DrawList2d::new(cx);
        let overlay = Overlay { draw_list: DrawList::new(cx) };
        let event = DrawEvent::default();
        let mut draw = CxDraw::new(cx, &event);
        let mut cx2d = Cx2d::new(&mut draw);
        cx2d.begin_pass(&pass, None);
        list.begin_always(&mut cx2d);
        overlay.begin(&mut cx2d);
        cx2d.begin_root_turtle(size, Layout::flow_overlay());
        screen.draw_walk_all(
            &mut cx2d,
            &mut Scope::empty(),
            Walk { width: Size::Fixed(width), height: Size::Fixed(775.0), ..Walk::default() },
        );
        overlay.end(&mut cx2d);
        cx2d.end_pass_sized_turtle();
        list.end(&mut cx2d);
        cx2d.end_pass(&pass);
    }

    /// What the cell at `slot` of the grid's row `row` shows: the name and the note under it.
    fn drawn_cell(screen: &InviteScreen, cx: &mut Cx, row: usize, slot: usize) -> Option<(String, String)> {
        let row = screen.view.portal_list(cx, ids!(invite_members_list)).item(cx, row, id!(MemberRow));
        let cell = row.view(cx, member_cell_ids!()[slot]);
        cell.visible().then(|| (cell.label(cx, ids!(name)).text(), cell.label(cx, ids!(note)).text()))
    }

    fn grid_height(screen: &InviteScreen, cx: &mut Cx) -> Size {
        screen.view.view(cx, ids!(invite_members_grid)).borrow().unwrap().walk.height
    }

    #[test]
    fn invite_screen_draws_the_people_the_invitation_names() {
        let (mut cx, mut screen) = screen();
        let room = name("!drawn:example.test", "Drawn");
        remember(&mut cx, room.clone(), false);
        screen.set_displayed_invite(&mut cx, &room);

        // Nothing is known about the room's people yet: no section for them.
        draw(&mut screen, &mut cx, 800.0);
        assert!(!screen.view.view(&mut cx, ids!(invite_members_view)).visible());

        // Seven people in the room and one who is invited, named a to h.
        let people = (0..8u8)
            .map(|n| {
                let letter = char::from(b'a' + n);
                person(&format!("@{letter}:example.test"), Some(&format!("Person {letter}")), n == 7)
            })
            .collect();
        let request = screen.members_request;
        loaded(&mut screen, &mut cx, &room, request, people);
        draw(&mut screen, &mut cx, 800.0);
        assert!(screen.view.view(&mut cx, ids!(invite_members_view)).visible());
        assert!(screen.view.view(&mut cx, ids!(invite_members_grid)).visible());
        assert!(!screen.view.label(&mut cx, ids!(invite_members_partial)).visible());
        assert_eq!(screen.view.label(&mut cx, ids!(invite_members_title)).text(), "7 members");
        // Six in a row on a wide screen: two rows.
        assert_eq!(grid_height(&screen, &mut cx), Size::Fixed(2.0 * CELL_SIZE.1));
        assert_eq!(drawn_cell(&screen, &mut cx, 0, 0), Some(("Person a".into(), String::new())));
        assert_eq!(drawn_cell(&screen, &mut cx, 0, 5), Some(("Person f".into(), String::new())));
        assert_eq!(drawn_cell(&screen, &mut cx, 1, 1), Some(("Person h".into(), "Invited".into())));
        assert_eq!(drawn_cell(&screen, &mut cx, 1, 2), None);

        // Three in a row on a narrow one: three rows, the most shown at once.
        draw(&mut screen, &mut cx, 240.0);
        assert_eq!(grid_height(&screen, &mut cx), Size::Fixed(3.0 * CELL_SIZE.1));
        assert_eq!(drawn_cell(&screen, &mut cx, 0, 2), Some(("Person c".into(), String::new())));
        assert_eq!(drawn_cell(&screen, &mut cx, 0, 3), None);
        assert_eq!(drawn_cell(&screen, &mut cx, 2, 1), Some(("Person h".into(), "Invited".into())));

        // The service's account goes by what it is, as in the room's own member list.
        let request = screen.members_request;
        let mut service = person("@_hagency_service:example.test", None, false);
        service.service = true;
        loaded(&mut screen, &mut cx, &room, request, vec![service]);
        draw(&mut screen, &mut cx, 800.0);
        assert_eq!(drawn_cell(&screen, &mut cx, 0, 0), Some(("Hagency service".into(), String::new())));

        // A homeserver that names nobody but the inviter: the number of people alone.
        let request = screen.members_request;
        screen.info.as_mut().unwrap().inviter = Some(InviterInfo {
            user_id: "@a:example.test".parse().unwrap(),
            display_name: Some("Person a".into()),
            avatar_url: None,
        });
        loaded(&mut screen, &mut cx, &room, request, vec![person("@a:example.test", Some("Person a"), false)]);
        draw(&mut screen, &mut cx, 800.0);
        assert!(screen.view.view(&mut cx, ids!(invite_members_view)).visible());
        assert!(!screen.view.view(&mut cx, ids!(invite_members_grid)).visible());
        assert_eq!(screen.view.label(&mut cx, ids!(invite_members_title)).text(), "2 members");
    }
}
