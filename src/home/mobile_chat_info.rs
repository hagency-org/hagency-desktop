//! Mobile room information backed by the current Matrix room and push rules.

use makepad_widgets::*;
use std::collections::{HashMap, HashSet};
use matrix_sdk::{
    notification_settings::RoomNotificationMode,
    ruma::{events::{room::power_levels::RoomPowerLevels, StateEventType}, OwnedRoomId, OwnedUserId, UserId},
    RoomMemberships,
};

use crate::{
    home::{invite_modal::InviteModalAction, rooms_list::RoomNotificationModeUpdated},
    logout::logout_confirm_modal::LogoutAction,
    moderation::{MemberAbilitiesInvalidated, MemberAction, MemberModeration},
    moderation_modal::{ModerationModalAction, ModerationRequest},
    profile::user_profile::{ShowUserProfileAction, UserProfile, UserProfileAndRoomId},
    shared::{
        avatar::{AvatarState, AvatarWidgetRefExt},
        navigation_bar_button::{NavigationBarButton, NavigationBarButtonWidgetRefExt, NavigationBarButtonWidgetExt},
        popup_list::{enqueue_popup_notification, PopupKind},
    },
    sliding_sync::{
        current_user_id, get_client, spawn_async_task, submit_async_request, MatrixRequest,
    },
    utils::{self, RoomNameId},
};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    let InfoMemberCell = NavigationBarButton {
        width: 64 height: 84 flow: Down spacing: 2
        padding: Inset{top: 4 bottom: 4 left: 2 right: 2}
        align: Align{x: 0.5 y: 0.0}
        draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_HOVER border_radius: 6}
        face := View {width: 48 height: 46 flow: Overlay
            photo := View {width: 40 height: 40 margin: Inset{left: 4 top: 6}
                avatar := mod.widgets.MobileAvatar {width: 40 height: 40}
            }
            tile := RoundedView {visible: false width: 40 height: 40 margin: Inset{left: 4 top: 6}
                flow: Overlay align: Align{x: 0.5 y: 0.5}
                draw_bg +: {color: mod.widgets.RINX_SURFACE border_radius: 5.0 border_size: 1.0 border_color: mod.widgets.RINX_DISABLED}
                SolidView {width: 16 height: 2 draw_bg.color: mod.widgets.RINX_MUTED}
                upright := SolidView {width: 2 height: 16 draw_bg.color: mod.widgets.RINX_MUTED}
            }
            badge := RoundedView {visible: false width: 16 height: 16 align: Align{x: 0.5 y: 0.5}
                draw_bg +: {color: mod.widgets.RINX_DANGER_FG border_radius: 8.0}
                SolidView {width: 8 height: 2 draw_bg.color: mod.widgets.RINX_ON_ACCENT}
            }
        }
        name := Label {width: Fill height: Fit padding: 0 align: Align{x: 0.5} max_lines: 1 text_overflow: Ellipsis
            draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: (10 * mod.widgets.RINX_TEXT_SCALE)}}}
        note := Label {width: Fill height: Fit padding: 0 align: Align{x: 0.5} max_lines: 1 text_overflow: Ellipsis
            draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (8 * mod.widgets.RINX_TEXT_SCALE)}}}
    }

    mod.widgets.MobileChatInfo = #(MobileChatInfo::register_widget(vm)) {
        ..mod.widgets.SolidView
        width: Fill height: Fill flow: Down
        draw_bg.color: mod.widgets.MOBILE_BG
        summary := View {width: Fill height: Fit flow: Down
            summary_details := View {width: Fill height: Fit flow: Down spacing: 8 padding: Inset{left: 16 right: 16 top: 16 bottom: 4}
                summary_name := Label {width: Fill height: Fit max_lines: 2 text_overflow: Ellipsis
                    draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_bold{font_size: (15 * mod.widgets.RINX_TEXT_SCALE)}}}
                View {width: Fill height: 32 flow: Right spacing: 8 align: Align{y: 0.5}
                    summary_count := Label {width: Fill height: Fit draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular{font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}}
                    history_search := NavigationBarButton {width: 32 height: 32 padding: 7 tooltip_text: #(crate::i18n::tr("Search Chat History"))
                        Icon {icon_walk: Walk{width: 16 height: 16} draw_icon +: {svg: ICON_SEARCH color: mod.widgets.RINX_MUTED}}}
                    history_media := NavigationBarButton {width: 32 height: 32 padding: 7 tooltip_text: #(crate::i18n::tr("Shared Attachments"))
                        Icon {icon_walk: Walk{width: 16 height: 16} draw_icon +: {svg: ICON_ADD_ATTACHMENT color: mod.widgets.RINX_MUTED}}}
                }
            }
        }
        status := Label {
            width: Fill height: Fit margin: 16
            flow: Flow.Right{wrap: true}
            draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}
        }
        retry := RobrixNeutralIconButton {visible: false text: #(crate::i18n::tr("Retry")) i18n_text: "Retry" height: 44 margin: 12}
        rename_panel := View {visible: false width: Fill height: Fit flow: Down padding: Inset{left: 16 right: 16 top: 12 bottom: 12} spacing: 10
            rename_back := RinxBackButton {}
            Label {width: Fill height: Fit text: #(crate::i18n::tr("Room name")) i18n_text: "Room name"
                draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_bold{font_size: (12 * mod.widgets.RINX_TEXT_SCALE)}}}
            rename_input := RinxInput {width: Fill height: (44 * mod.widgets.RINX_TEXT_SCALE) padding: Inset{left: 12 right: 12 top: 10 bottom: 10}
                draw_bg +: {border_size: 0 border_radius: 5} draw_text.text_style.font_size: (12 * mod.widgets.RINX_TEXT_SCALE)
                empty_text: #(crate::i18n::tr("Room name")) i18n_empty_text: "Room name"}
            rename_status := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
            rename_save := RobrixPositiveIconButton {text: #(crate::i18n::tr("Save")) i18n_text: "Save" width: Fill height: 40 padding: 10 icon_walk: Walk{width: 0 height: 0}}
        }
        services_panel := View {visible: false width: Fill height: Fit flow: Down padding: Inset{left: 16 right: 16 top: 12 bottom: 12} spacing: 10
            services_back := RinxBackButton {}
            Label {width: Fill height: Fit text: #(crate::i18n::tr("Room Agent services")) i18n_text: "Room Agent services"
                draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_bold{font_size: (12 * mod.widgets.RINX_TEXT_SCALE)}}}
            Label {width: Fill height: Fit flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("Only your Agents connected to this Room are shown. Service settings apply to the selected Agent across its connected Rooms."))
                i18n_text: "Only your Agents connected to this Room are shown. Service settings apply to the selected Agent across its connected Rooms."
                draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular{font_size: (10 * mod.widgets.RINX_TEXT_SCALE)}}}
            services_status := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
            room_services := RobrixSettingsDropDown {width: Fill height: 40 margin: 0 labels: ["Choose an Agent"]
                popup_menu_position: PopupMenuPosition.BelowInput
                popup_menu: RobrixSettingsPopupMenu {width: 256 menu_item: RobrixSettingsPopupMenuItem {height: 36}}}
            View {width: Fill height: 36 flow: Right spacing: 8
                configure_service := RobrixNeutralIconButton {text: #(crate::i18n::tr("Open Service")) i18n_text: "Open Service" height: 36 padding: Inset{left: 10 right: 10 top: 0 bottom: 0} width: Fill icon_walk: Walk{width: 0 height: 0}}
                refresh_services := RobrixNeutralIconButton {text: #(crate::i18n::tr("Refresh")) height: 36 padding: Inset{left: 10 right: 10 top: 0 bottom: 0} icon_walk: Walk{width: 0 height: 0}}
            }
        }
        information_list := View {width: Fill height: Fill
            list := PortalList {
                width: Fill height: Fill flow: Down
                MemberRow := mod.widgets.MobileSection {
                    flow: Right padding: Inset{left: 12 right: 12}
                    c0 := InfoMemberCell {} c1 := InfoMemberCell {} c2 := InfoMemberCell {}
                    c3 := InfoMemberCell {} c4 := InfoMemberCell {} c5 := InfoMemberCell {}
                }
                AddChoice := mod.widgets.MobileSection {
                    flow: Right spacing: 6 padding: Inset{left: 12 right: 12 top: 4 bottom: 10}
                    invite_people := RobrixNeutralIconButton {text: #(crate::i18n::tr("Invite people")) i18n_text: "Invite people" width: Fill height: 32 padding: 6 spacing: 5
                        icon_walk: Walk{width: 14 height: 14} draw_icon.svg: ICON_ADD_USER}
                    invite_my_agent := RobrixNeutralIconButton {text: #(crate::i18n::tr("Add agent")) i18n_text: "Add agent" width: Fill height: 32 padding: 6 spacing: 5
                        icon_walk: Walk{width: 14 height: 14} draw_icon.svg: ICON_ROBOT}
                }
                Gap := View {width: Fill height: 8}
                Topic := mod.widgets.MobileSection {
                    padding: Inset{left: 20 right: 20 top: 12 bottom: 12} spacing: 6 margin: 0
                    Label {width: Fill text: #(crate::i18n::tr("Topic")) i18n_text: "Topic"
                        draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular{font_size: (10 * mod.widgets.RINX_TEXT_SCALE)}}}
                    topic := Label {width: Fill height: Fit flow: Flow.Right{wrap: true}
                        draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular{font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}}
                }
                Back := mod.widgets.MobileSection {
                    padding: Inset{left: 16 right: 16 top: 8 bottom: 8}
                    row := RinxBackButton {}
                }
                Action := mod.widgets.MobileSection {
                    row := NavigationBarButton {
                        width: Fill height: 56 flow: Right spacing: 10
                        padding: Inset{left: 20 right: 20} align: Align{y: 0.5}
                        draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_HOVER border_radius: 0}
                        title := Label {
                            width: Fill max_lines: 1 text_overflow: Ellipsis
                            draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: (12.5 * mod.widgets.RINX_TEXT_SCALE)}}
                        }
                        value := Label {
                            max_lines: 1
                            draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (10 * mod.widgets.RINX_TEXT_SCALE)}}
                        }
                        chevron := View {width: Fit height: Fit
                            Icon {
                                icon_walk: Walk{width: 8 height: 13}
                                draw_icon +: {color: mod.widgets.RINX_DISABLED svg: ICON_CHEVRON_RIGHT}
                            }
                        }
                    }
                    mod.widgets.MobileDivider {margin: Inset{left: 20}}
                }
                Filler := View {height: 100 width: Fill}
            }
        }
        agent_picker_modal := Modal {
            content := SmallModal {
                title := ModalTitle {text: #(crate::i18n::tr("Add agent")) i18n_text: "Add agent"}
                agent_room := ModalBody {margin: Inset{bottom: 12}}
                my_agents := RobrixSettingsDropDown {width: Fill height: 40 margin: 0 labels: ["Choose my Agent"]
                    draw_bg.border_radius: 4
                    popup_menu_position: PopupMenuPosition.BelowInput
                    draw_text.text_style: theme.font_regular{font_size: (12 * mod.widgets.RINX_TEXT_SCALE)}
                    popup_menu: RobrixSettingsPopupMenu {width: 310 menu_item: RobrixSettingsPopupMenuItem {height: 36}}}
                ModalBody {margin: Inset{top: 12}
                    text: #(crate::i18n::tr("Only available Agents are listed. If a pool Agent is missing, check its resource authorization in Agent → My Agents."))
                    i18n_text: "Only available Agents are listed. If a pool Agent is missing, check its resource authorization in Agent → My Agents."
                    draw_text.color: mod.widgets.RINX_MUTED
                }
                buttons_view := ModalButtonsRow {
                    close_agent_picker := RobrixNeutralIconButton {
                        width: Fit{min: FitBound.Abs(120.0)} height: 44 align: Align{x: 0.5 y: 0.5} padding: 12
                        text: #(crate::i18n::tr("Cancel")) i18n_text: "Cancel"
                        draw_icon.svg: ICON_FORBIDDEN icon_walk: Walk{width: 16 height: 16 margin: Inset{left: -2 right: -1}}
                    }
                    invite_selected_agent := RobrixPositiveIconButton {
                        width: Fit{min: FitBound.Abs(120.0)} height: 44 align: Align{x: 0.5 y: 0.5} padding: 12
                        text: #(crate::i18n::tr("Add agent")) i18n_text: "Add agent"
                        draw_icon.svg: ICON_ROBOT icon_walk: Walk{width: 16 height: 16 margin: Inset{left: -2 right: -1}}
                    }
                }
                agent_status := ModalBody {}
                resume_agent_requests := RobrixNeutralIconButton {visible: false text: #(crate::i18n::tr("Resume original request in Agents")) i18n_text: "Resume original request in Agents" height: 40 margin: Inset{top: 12} icon_walk: Walk{width: 0 height: 0}}
            }
        }
    }
}

/// The most member cells one row of the grid can hold: the `MemberRow` template has this many.
const MAX_COLUMNS: usize = 6;
/// The fewest cells in a row, however narrow the panel is.
const MIN_COLUMNS: usize = 4;
/// The narrowest and the widest a member cell may be.
const CELL_WIDTHS: (f64, f64) = (62.0, 84.0);
/// The grid's padding on each side, as in the `MemberRow` template.
const GRID_SIDE_PADDING: f64 = 12.0;

/// The cells of a `MemberRow`, left to right.
macro_rules! cell_ids {
    () => { [ids!(c0), ids!(c1), ids!(c2), ids!(c3), ids!(c4), ids!(c5)] };
}

/// How many member cells fit side by side in a panel this wide, and how wide each one is.
fn grid_columns(panel_width: f64) -> (usize, f64) {
    let available = (panel_width - 2.0 * GRID_SIDE_PADDING).max(MIN_COLUMNS as f64 * CELL_WIDTHS.0);
    let columns = ((available / CELL_WIDTHS.0) as usize).clamp(MIN_COLUMNS, MAX_COLUMNS);
    (columns, (available / columns as f64).min(CELL_WIDTHS.1).floor())
}

/// One cell of the member grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InfoCell { Member(usize), Add, Remove }

/// What a tap on a drawn grid cell acts on. A member is kept by profile, not by position,
/// so that a list that changed after the draw can't turn the tap to somebody else.
#[derive(Clone, Debug)]
enum CellTarget { Member(UserProfile), Add, Remove }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InfoRow {
    /// A row of the member grid, by the position of its first cell.
    Members(usize),
    AddChoice, Gap, Name, Topic, Notifications, Mode(usize), Back, AgentServices, Filler,
}

/// The one page of room information, top to bottom: the member grid, which ends in the tile
/// that adds and, for a viewer who may remove members, the tile that removes; then the
/// room's name, its topic when it has one, and its settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InfoPage {
    members: usize,
    /// Whether the viewer may remove at least one of the members.
    can_remove: bool,
    columns: usize,
    /// Whether the choice between inviting a person and adding an Agent is open.
    adding: bool,
    /// Whether the room has a topic to show.
    topic: bool,
    /// Whether the notification modes are listed in place of the settings.
    choosing: bool,
}
impl InfoPage {
    fn cells(&self) -> usize {
        self.members + 1 + usize::from(self.can_remove)
    }
    fn cell(&self, index: usize) -> Option<InfoCell> {
        if index < self.members {Some(InfoCell::Member(index))}
        else if index == self.members {Some(InfoCell::Add)}
        else if index == self.members + 1 && self.can_remove {Some(InfoCell::Remove)}
        else {None}
    }
    fn grid_rows(&self) -> usize {
        self.cells().div_ceil(self.columns.max(1))
    }
    fn rows(&self) -> usize {
        self.grid_rows() + usize::from(self.adding) + 2 + usize::from(self.topic) + if self.choosing {5} else {2}
    }
    fn row(&self, index: usize) -> InfoRow {
        let grid = self.grid_rows();
        if index < grid {return InfoRow::Members(index * self.columns.max(1));}
        let mut index = index - grid;
        if self.adding {
            if index == 0 {return InfoRow::AddChoice;}
            index -= 1;
        }
        match index {
            0 => return InfoRow::Gap,
            1 => return InfoRow::Name,
            _ => index -= 2,
        }
        if self.topic {
            if index == 0 {return InfoRow::Topic;}
            index -= 1;
        }
        match (index, self.choosing) {
            (0..=3, true) => InfoRow::Mode(index),
            (4, true) => InfoRow::Back,
            (0, false) => InfoRow::Notifications,
            (1, false) => InfoRow::AgentServices,
            _ => InfoRow::Filler,
        }
    }
}

/// The members `viewer` may remove from the room: those in it whose power level is below the
/// viewer's, when the viewer's own level allows removing at all. The Hagency service is
/// never offered, because the room's Agents are invited through it.
fn removable_members(
    levels: &RoomPowerLevels, viewer: &UserId, members: &[UserProfile], banned: &HashSet<OwnedUserId>,
) -> HashSet<OwnedUserId> {
    members.iter().map(|member| &member.user_id)
        .filter(|user| !banned.contains(*user) && !is_hagency_service(user, viewer) && levels.user_can_kick_user(viewer, user))
        .cloned().collect()
}

/// The room a row has for the room's name beside its title in the narrowest panel,
/// in half widths: a Latin letter takes about one, a Chinese character two.
const NAME_SHOWN: usize = 12;
/// `name`, cut to what a row has room for. The whole name is in the field that changes it.
fn shown_name(name: &str) -> String {
    let wide = |c: char| if c.is_ascii() {1} else {2};
    if name.chars().map(wide).sum::<usize>() <= NAME_SHOWN {return name.to_owned();}
    let mut room = NAME_SHOWN - 1;
    name.chars().take_while(|c| match room.checked_sub(wide(*c)) {
        Some(left) => {room = left; true}
        None => false,
    }).chain(['…']).collect()
}
#[derive(Clone, Debug)]
pub(crate) struct ChatMembershipChanged {
    pub room_id: OwnedRoomId,
    pub epoch: u64,
}

#[derive(Clone, Debug)]
struct ChatInfoData {
    room: RoomNameId,
    can_invite: bool,
    topic: String,
    members: Vec<UserProfile>,
    banned_members: HashSet<OwnedUserId>,
    /// The members the viewer may remove from the room.
    removable: HashSet<OwnedUserId>,
    /// Whether the viewer may change the room's name.
    can_rename: bool,
    mode: Option<RoomNotificationMode>,
}


#[derive(Debug)]
struct MemberOwnershipLoaded {
    owner: OwnedUserId, origin: String, epoch: u64, room_id: OwnedRoomId, request: u64,
    result: Result<(HashMap<OwnedUserId, OwnedUserId>, bool), String>,
}
fn add_member_owners(owners: &mut HashMap<OwnedUserId, OwnedUserId>, conflicts: &mut HashSet<OwnedUserId>, rows: &serde_json::Value, fallback_owner: Option<&OwnedUserId>) {
    for row in rows.as_array().into_iter().flatten() {
        let Some(agent) = row["puppetMxid"].as_str().and_then(|id| OwnedUserId::try_from(id).ok()) else {continue;};
        let owner = row["ownerMxid"].as_str().and_then(|id| OwnedUserId::try_from(id).ok()).or_else(|| fallback_owner.cloned());
        let Some(owner) = owner else {continue;};
        if agent == owner || agent.localpart() == "_hagency_service" || conflicts.contains(&agent) {continue;}
        if owners.get(&agent).is_some_and(|old| old != &owner) {
            owners.remove(&agent);conflicts.insert(agent);
        } else {owners.insert(agent,owner);}
    }
}
fn group_members(members: &mut Vec<UserProfile>, owners: &HashMap<OwnedUserId, OwnedUserId>) -> HashSet<OwnedUserId> {
    // The Hagency service takes no part in the room's talk: it comes after everyone else.
    members.sort_by_key(|p| (p.user_id.localpart() == "_hagency_service", p.displayable_name().to_lowercase(), p.user_id.clone()));
    let present: HashSet<_> = members.iter().map(|p|p.user_id.clone()).collect();
    let grouped: HashSet<_> = members.iter().filter(|p|p.user_id.localpart() != "_hagency_service" && owners.get(&p.user_id).is_some_and(|owner| present.contains(owner) && !owners.contains_key(owner) && owner.localpart() != "_hagency_service" && *owner != p.user_id)).map(|p|p.user_id.clone()).collect();
    let mut ordered = Vec::with_capacity(members.len());
    for member in members.iter().filter(|p|!grouped.contains(&p.user_id)) {
        ordered.push(member.clone());
        ordered.extend(members.iter().filter(|p|grouped.contains(&p.user_id) && owners.get(&p.user_id) == Some(&member.user_id)).cloned());
    }
    *members = ordered;
    grouped
}
pub(crate) async fn fetch_member_owners(room: &str, owner: &OwnedUserId) -> Result<(HashMap<OwnedUserId, OwnedUserId>, bool), String> {
    use crate::hagency::{backend::Service,model::Command};
    let service = Service::for_current()?;
    let roster = service.execute(Command::Agents).await?;
    if roster["ownerMxid"].as_str() != Some(owner.as_str()) {return Err("matrix_account_changed".into());}
    let rows=roster["agents"].as_array().ok_or("room_discovery_unavailable")?;
    if rows.len()>128 {return Err("room_discovery_incomplete".into());}
    let mut owners = HashMap::new();let mut conflicts = HashSet::new();
    add_member_owners(&mut owners, &mut conflicts, &roster["agents"], Some(owner));
    let incomplete = match service.agents_for_registered_room(room).await {
        Ok(context) => {
            let project = context["projectId"].as_str().filter(|id|!id.is_empty()).ok_or("room_discovery_unavailable")?;
            match service.execute(Command::RoomRoster {project_id:project.into(),room_id:room.into()}).await {
                Ok(roster) if roster["agents"].as_array().is_some_and(|rows|rows.len()<=128) => {add_member_owners(&mut owners,&mut conflicts,&roster["agents"],None);false},
                Ok(_) => true,
                Err(_) => true,
            }
        },
        // Private/non-project Rooms can still group this owner's known Agents.
        Err(error) if error == "room_project_registration_required" => false,
        Err(_) => true,
    };
    Ok((owners,incomplete || !conflicts.is_empty()))
}

#[derive(Debug)]
struct RoomAgentsLoaded {
    owner: OwnedUserId,
    origin: String,
    epoch: u64,
    room_id: OwnedRoomId,
    request: u64,
    binding: bool,
    result: Result<serde_json::Value, String>,
}
#[derive(Debug)]
struct RoomAgentsSlow {
    owner: OwnedUserId, origin: String, epoch: u64, room_id: OwnedRoomId,
    request: u64, binding: bool,
}
impl RoomAgentsSlow {
    fn call(&self) -> RoomAgentsLoaded {
        RoomAgentsLoaded {owner:self.owner.clone(),origin:self.origin.clone(),epoch:self.epoch,
            room_id:self.room_id.clone(),request:self.request,binding:self.binding,result:Err(String::new())}
    }
}

pub(super) fn agent_binding_error(code: &str) -> &'static str {
    match code {
        "room_project_registration_required" => "To chat with Agents, use an Agent space. Open + → Create a space or room, choose Agent space, then create a room inside it.",
        "room_join_required" | "room_membership_required" => "Join this room and its Agent space before adding an Agent.",
        "service_invitation_denied" | "agent_service_invite_permission_required" => "The Hagency service needs permission to invite Agents. Ask a room administrator to allow it to invite members, then retry.",
        "agent_service_banned" => "The Hagency service is banned from this room. Ask a room administrator to unban it, then retry.",
        "matrix_permission_denied" => "You do not have permission to invite members to this room. Ask a room administrator to enable invitations.",
        "agent_creation_denied" => "This space or room does not allow you to add Agents. Ask its owner to update the Agent permissions.",
        "agent_owner_binding_required" => "This Agent is not available to add. Check that it belongs to you and its resource authorization is still valid in Agent → My Agents.",
        "agent_retired" => "This Agent has been retired and cannot join another room.",
        "agent_command_recovery_required" => "An original binding request needs confirmation. Resume it in Agents.",
        "sign_in_required" | "matrix_account_changed" => "Your account session changed. Reopen this room after signing in.",
        "invalid_server_response" => "The server returned an invalid binding result. Retry to check the original request.",
        "agent_binding_suspended" => "The Agent could not join this room. Check its membership and room permissions, then check the request again.",
        _ => "Could not confirm the binding. Retry to check the original request; no duplicate Agent will be created.",
    }
}
/// The answer to a request to change the room's name: the new name, or why it wasn't set.
#[derive(Debug)]
struct RoomRenamed {
    owner: OwnedUserId,
    origin: String,
    epoch: u64,
    room_id: OwnedRoomId,
    request: u64,
    result: Result<String, String>,
}

#[derive(Debug)]
struct ChatInfoLoaded {
    owner: OwnedUserId,
    origin: String,
    epoch: u64,
    room_id: OwnedRoomId,
    request: u64,
    result: Result<ChatInfoData, String>,
}

#[derive(Debug)]
struct RoomServicesLoaded {
    owner: OwnedUserId,
    origin: String,
    epoch: u64,
    room_id: OwnedRoomId,
    request: u64,
    result: Result<serde_json::Value, String>,
}

fn room_service_status(service: &serde_json::Value) -> String {
    if service["ownerServicePaused"] == true {return crate::i18n::tr("Paused in this Room. Requests receive a pause notice.").into();}
    if service["bindingState"] == "suspended" {return crate::i18n::tr("Room access unavailable. Check membership and permissions.").into();}
    if service["bindingState"] == "pending" {return crate::i18n::tr("Joining this Room…").into();}
    if service["thisDevice"] != true {return crate::i18n::tr("Not assigned to this device.").into();}
    if service["statusAvailable"] != true {return crate::i18n::tr("Could not verify this Room's service. Refresh to retry.").into();}
    crate::hagency::ui::runtime_summary(&service["runtime"], service["agentId"].as_str().unwrap_or(""), service["bindingId"].as_str())
}

#[derive(Script, Widget)]
pub struct MobileChatInfo {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    /// Set by a room's screen that shows the room's history itself, over its timeline:
    /// a request for the history then goes to that screen, not to the app, whose view of
    /// it covers the whole window.
    #[live]
    history_in_room: bool,
    #[rust]
    owner: Option<OwnedUserId>,
    #[rust]
    origin: String,
    #[rust]
    epoch: u64,
    #[rust]
    room: Option<RoomNameId>,
    #[rust]
    request: u64,
    #[rust]
    load_deadline: Option<std::time::Instant>,
    #[rust] load_timer: Timer,
    #[rust]
    data: Option<ChatInfoData>,
    #[rust] member_owners: HashMap<OwnedUserId, OwnedUserId>,
    #[rust]
    status: String,
    #[rust]
    failed: bool,
    #[rust]
    choosing_notifications: bool,
    /// Whether the choice between inviting a person and adding an Agent is open under the grid.
    #[rust] adding: bool,
    /// Whether the grid marks the members who can be removed, for one of them to be picked.
    #[rust] removing: bool,
    /// Whether the page is replaced by the field that changes the room's name.
    #[rust] renaming: bool,
    #[rust] rename_busy: bool,
    #[rust] rename_request: u64,
    #[rust] rendered_rows: HashMap<WidgetUid, InfoRow>,
    /// What each cell of a drawn grid row stood for, by the row's widget and in cell order.
    #[rust] rendered_cells: HashMap<WidgetUid, Vec<Option<CellTarget>>>,
    #[rust] agent_request: u64,
    #[rust] agent_busy: bool,
    #[rust] agent_added: bool,
    #[rust] agent_pending: bool,
    #[rust] agent_slow: bool,
    #[rust] agent_project: String,
    #[rust] agents: Vec<serde_json::Value>,
    #[rust] agent_command: Option<(String, String)>,
    #[rust] agent_picker_open: bool,
    #[rust] services_open: bool,
    #[rust] services_busy: bool,
    #[rust] services_request: u64,
    #[rust] services: Vec<serde_json::Value>,
    #[cfg(feature = "onboarding-test-support")]
    #[rust] review_mode: bool,
}

fn is_hagency_service(user: &matrix_sdk::ruma::UserId, owner: &matrix_sdk::ruma::UserId) -> bool {
    user.localpart() == "_hagency_service" && user.server_name() == owner.server_name()
}

fn modes() -> [(Option<RoomNotificationMode>, &'static str); 4] {
    [
        (None, crate::i18n::tr("Use Account Defaults")),
        (
            Some(RoomNotificationMode::AllMessages),
            crate::i18n::tr("All Messages"),
        ),
        (
            Some(RoomNotificationMode::MentionsAndKeywordsOnly),
            crate::i18n::tr("Mentions and Keywords"),
        ),
        (
            Some(RoomNotificationMode::Mute),
            crate::i18n::tr("Mute Notifications"),
        ),
    ]
}

const INFO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

impl MobileChatInfo {
    fn expire_pending_load(&mut self, cx: &mut Cx, now: std::time::Instant) {
        if self.load_deadline.is_some_and(|deadline| now >= deadline) {
            self.fail_load(cx, crate::i18n::tr("The chat information request timed out."));
        }
    }

    fn finish_load(&mut self, cx: &mut Cx) {
        self.load_deadline = None;
        cx.stop_timer(std::mem::take(&mut self.load_timer));
    }

    fn fail_load(&mut self, cx: &mut Cx, error: &str) {
        self.finish_load(cx);
        self.data = None;
        self.close_agent_picker(cx);
        self.failed = true;
        self.status = crate::i18n::format(
            "Could not load chat information: {error}", &[("error", error.to_owned())],
        );
        self.view.redraw(cx);
    }

    fn services_response_matches(&self, loaded: &RoomServicesLoaded) -> bool {
        self.owner.as_ref() == Some(&loaded.owner) && self.origin == loaded.origin
            && self.epoch == loaded.epoch && self.services_request == loaded.request
            && self.room.as_ref().is_some_and(|room|room.room_id() == &loaded.room_id)
            && loaded.result.as_ref().map_or(true, |v|v["roomId"] == loaded.room_id.as_str())
    }
    fn load_services(&mut self, cx: &mut Cx) {
        if self.services_busy || !self.pin_is_current() {return;}
        let (Some(owner), Some(room)) = (self.owner.clone(), self.room.clone()) else {return;};
        let Ok(service) = crate::hagency::backend::Service::for_current() else {
            self.services.clear();
            self.view.drop_down(cx, ids!(room_services)).set_labels(cx, vec![crate::i18n::tr("Choose an Agent").into()]);
            self.view.drop_down(cx, ids!(room_services)).set_selected_item(cx, 0);
            self.view.label(cx, ids!(services_status)).set_text(cx, crate::i18n::tr("Could not load Agent services. Refresh to retry."));
            self.view.redraw(cx);
            return;
        };
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let request = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.services_request = request;
        self.services_busy = true;
        self.services_open = true;
        self.services.clear();
        self.view.drop_down(cx, ids!(room_services)).set_labels(cx, vec![crate::i18n::tr("Choose an Agent").into()]);
        self.view.drop_down(cx, ids!(room_services)).set_selected_item(cx, 0);
        self.view.label(cx, ids!(services_status)).set_text(cx, crate::i18n::tr("Checking Agent services in this Room…"));
        let origin = self.origin.clone(); let epoch = self.epoch; let room_id = room.room_id().clone();
        spawn_async_task(async move {
            let result = service.room_agent_services(room_id.as_str()).await;
            Cx::post_action(RoomServicesLoaded {owner, origin, epoch, room_id, request, result});
        });
        self.view.redraw(cx);
    }
    /// Acts on a tap on the room's name: opens the field that changes it, or says who can.
    fn tap_name(&mut self, cx: &mut Cx) {
        let Some(data) = self.data.as_ref() else {return;};
        if !data.can_rename {
            enqueue_popup_notification(
                crate::i18n::tr("Only the room's administrators can change its name.").to_owned(), PopupKind::Info, Some(4.0),
            );
            return;
        }
        let name = data.room.display().into_owned();
        self.renaming = true;
        self.rename_busy = false;
        self.services_open = false;
        self.adding = false;
        self.removing = false;
        let input = self.view.text_input(cx, ids!(rename_input));
        input.set_text(cx, &name);
        input.set_key_focus(cx);
        let mut status = self.view.label(cx, ids!(rename_status));
        status.set_text(cx, "");
        script_apply_eval!(cx, status, {draw_text +: {color: mod.widgets.RINX_MUTED}});
    }
    /// Sends the name in the field to the homeserver, unless there is nothing to change.
    fn save_name(&mut self, cx: &mut Cx) {
        if self.rename_busy {return;}
        let Some(data) = self.data.as_ref().filter(|data| data.can_rename) else {return;};
        let name = self.view.text_input(cx, ids!(rename_input)).text().trim().to_owned();
        if name.is_empty() {
            self.view.label(cx, ids!(rename_status)).set_text(cx, crate::i18n::tr("Enter a room name."));
            self.view.redraw(cx);
            return;
        }
        if name == data.room.display() {
            self.renaming = false;
            self.view.redraw(cx);
            return;
        }
        let room_id = data.room.room_id().clone();
        if !self.pin_is_current() {return;}
        let (Some(owner), Some(client)) = (self.owner.clone(), get_client()) else {return;};
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let request = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.rename_request = request;
        self.rename_busy = true;
        self.view.label(cx, ids!(rename_status)).set_text(cx, crate::i18n::tr("Saving…"));
        let origin = self.origin.clone();
        let epoch = self.epoch;
        spawn_async_task(async move {
            let result = tokio::time::timeout(INFO_TIMEOUT, async {
                let room = client.get_room(&room_id).ok_or_else(|| crate::i18n::tr("This chat is no longer available.").to_owned())?;
                room.set_name(name.clone()).await.map_err(|e| e.to_string())?;
                Ok(name)
            }).await.unwrap_or_else(|_| Err(crate::i18n::tr("The chat information request timed out.").into()));
            Cx::post_action(RoomRenamed {owner, origin, epoch, room_id, request, result});
        });
        self.view.redraw(cx);
    }
    fn rename_response_matches(&self, renamed: &RoomRenamed) -> bool {
        self.owner.as_ref() == Some(&renamed.owner) && self.origin == renamed.origin
            && self.epoch == renamed.epoch && self.rename_request == renamed.request
            && self.room.as_ref().is_some_and(|room| room.room_id() == &renamed.room_id)
    }
    /// Takes the answer to this panel's own request: the page shows the new name at once,
    /// or the field stays open with the reason the name wasn't set.
    fn finish_rename(&mut self, cx: &mut Cx, renamed: &RoomRenamed) {
        self.rename_busy = false;
        match &renamed.result {
            Ok(name) => {
                let room = RoomNameId::new(matrix_sdk::RoomDisplayName::Named(name.clone()), renamed.room_id.clone());
                if let Some(data) = self.data.as_mut() {data.room = room.clone();}
                self.room = Some(room);
                self.renaming = false;
            }
            Err(error) => {
                let mut status = self.view.label(cx, ids!(rename_status));
                status.set_text(cx, &crate::i18n::format("Could not change the room's name: {error}", &[("error", error.clone())]));
                script_apply_eval!(cx, status, {draw_text +: {color: mod.widgets.RINX_DANGER_FG}});
            }
        }
        self.view.redraw(cx);
    }
    fn open_agent_picker(&mut self, cx: &mut Cx) {
        if self.agent_busy {
            self.agent_picker_open = true;
            self.view.modal(cx, ids!(agent_picker_modal)).open(cx);
            self.view.redraw(cx);
        } else {
            self.request_agents(cx, false);
        }
    }
    /// Acts on a tap on a cell of the member grid.
    fn tap_cell(&mut self, cx: &mut Cx, target: CellTarget) {
        let Some(data) = self.data.as_ref() else {return;};
        match target {
            // While members are being removed, a tap picks one of those who can be.
            // Nothing is sent before the dialog it opens is confirmed.
            CellTarget::Member(profile) if self.removing => {
                if !data.removable.contains(&profile.user_id) {return;}
                cx.action(ModerationModalAction::Open(ModerationRequest::Member(MemberModeration {
                    room_id: data.room.room_id().clone(),
                    room_name: data.room.to_string(),
                    user_name: profile.displayable_name().to_owned(),
                    user_id: profile.user_id,
                    action: MemberAction::Kick,
                })));
            }
            CellTarget::Member(profile) => cx.widget_action(self.widget_uid(), ShowUserProfileAction::ShowUserProfile(
                UserProfileAndRoomId {user_profile: profile, room_id: data.room.room_id().clone()},
            )),
            CellTarget::Add => {
                // Inviting a person takes the right to invite; an Agent is added through
                // the Hagency service, so without that right only the Agent picker is left.
                let choose = data.can_invite && !self.adding;
                let agent_only = !data.can_invite;
                self.removing = false;
                self.adding = choose;
                if agent_only {self.open_agent_picker(cx);}
            }
            CellTarget::Remove => {
                self.removing = !self.removing && !data.removable.is_empty();
                self.adding = false;
            }
        }
    }
    fn update_room(&mut self, cx: &mut Cx, room: RoomNameId, opening: bool) {
        if !self.pin_is_current()
            || self.room.as_ref().is_none_or(|current| current.room_id() != room.room_id())
        {
            self.load(cx, room);
        } else if opening {
            // The hidden pane can miss membership events. Keep its invitation
            // state, but fetch the roster before showing member actions again.
            self.data = None;
            self.adding = false;
            self.removing = false;
            self.renaming = false;
            self.fetch_info(cx, room);
        }
    }
    fn load(&mut self, cx: &mut Cx, room: RoomNameId) {
        self.clear();
        self.fetch_info(cx, room);
    }
    // Refresh metadata without clearing an in-flight Agent invitation or its original key.
    fn fetch_info(&mut self, cx: &mut Cx, room: RoomNameId) {
        self.room = Some(room.clone());
        let epoch = crate::account_session::epoch();
        let Some(client) = get_client() else { return };
        let Some(owner) = client.user_id().map(ToOwned::to_owned) else {
            return;
        };
        if epoch != crate::account_session::epoch() || crate::sliding_sync::account_changing() {
            return;
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.request = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.owner = Some(owner.clone());
        let request = self.request;
        self.origin = client.homeserver().to_string();
        self.epoch = epoch;
        let origin = self.origin.clone();
        let epoch = self.epoch;
        let room_id = room.room_id().clone();
        self.member_owners.clear();
        if let Some(data)=self.data.as_mut() {group_members(&mut data.members,&self.member_owners);}
        let ownership_owner=owner.clone();let ownership_origin=origin.clone();let ownership_room=room_id.clone();
        spawn_async_task(async move {
            let result=tokio::time::timeout(std::time::Duration::from_secs(30),fetch_member_owners(ownership_room.as_str(),&ownership_owner)).await.unwrap_or_else(|_|Err("room_discovery_unavailable".into()));
            Cx::post_action(MemberOwnershipLoaded {owner:ownership_owner,origin:ownership_origin,epoch,room_id:ownership_room,request,result});
        });
        cx.stop_timer(self.load_timer);
        self.load_timer = cx.start_timeout(INFO_TIMEOUT.as_secs_f64());
        self.load_deadline = Some(std::time::Instant::now() + INFO_TIMEOUT);
        self.failed = false;
        self.status = crate::i18n::tr("Loading chat information…").into();
        self.view
            .portal_list(cx, ids!(list))
            .set_first_id_and_scroll(0, 0.0);
        self.view.redraw(cx);
        spawn_async_task(async move {
            let result = tokio::time::timeout(INFO_TIMEOUT, async {
                let room = client
                    .get_room(room.room_id())
                    .ok_or(crate::i18n::tr("This chat is no longer available."))?;
                // The stored list can keep a person who has left: ask the homeserver
                // each time, and show the stored list when it can't be asked.
                crate::room_members::reload(&room).await;
                let members = room
                    .members_no_sync(RoomMemberships::JOIN | RoomMemberships::BAN)
                    .await
                    .map_err(|e| e.to_string())?;
                let banned_members: HashSet<OwnedUserId> = members.iter().filter(|m| *m.membership() == ruma::events::room::member::MembershipState::Ban)
                    .map(|m| m.user_id().to_owned()).collect();
                let mut members: Vec<_> = members
                    .into_iter()
                    .map(|member| UserProfile {
                        user_id: member.user_id().to_owned(),
                        username: member.display_name().map(ToOwned::to_owned),
                        avatar_state: AvatarState::Known(
                            member.avatar_url().map(ToOwned::to_owned),
                        ),
                    })
                    .collect();
                members.sort_by_key(|p| (p.displayable_name().to_lowercase(), p.user_id.clone()));
                // Without the room's power levels nobody is offered for removal
                // and the name can't be changed.
                let levels = room.power_levels().await.ok();
                let removable = levels.as_ref()
                    .map(|levels| removable_members(levels, &owner, &members, &banned_members))
                    .unwrap_or_default();
                let can_rename = levels.as_ref().is_some_and(|levels| levels.user_can_send_state(&owner, StateEventType::RoomName));
                Ok(ChatInfoData {
                    room: RoomNameId::from_room(&room).await,
                    can_invite: crate::sliding_sync::UserPowerLevels::from_room(&room, &owner).await
                        .is_some_and(crate::sliding_sync::UserPowerLevels::can_invite),
                    topic: room.topic().unwrap_or_default(),
                    members,
                    banned_members,
                    removable,
                    can_rename,
                    mode: room.user_defined_notification_mode().await,
                })
            }).await.unwrap_or_else(|_| Err(crate::i18n::tr("The chat information request timed out.").into()));
            Cx::post_action(ChatInfoLoaded {
                owner,
                origin,
                epoch,
                room_id,
                request,
                result,
            });
        });
    }

    fn pin_is_current(&self) -> bool {
        #[cfg(feature = "onboarding-test-support")]
        if self.review_mode {return get_client().is_none();}
        self.owner == current_user_id()
            && self.epoch == crate::account_session::epoch()
            && !crate::sliding_sync::account_changing()
            && get_client().is_some_and(|client| client.homeserver().as_str() == self.origin)
    }
    fn accepts(&self, loaded: &ChatInfoLoaded) -> bool {
        self.pin_is_current() && self.matches_response(loaded)
    }
    fn matches_response(&self, loaded: &ChatInfoLoaded) -> bool {
        self.owner.as_ref() == Some(&loaded.owner)
            && self.origin == loaded.origin
            && self.epoch == loaded.epoch
            && self.request == loaded.request
            && self
                .room
                .as_ref()
                .is_some_and(|room| room.room_id() == &loaded.room_id)
            && match &loaded.result {
                Ok(data) => data.room.room_id() == &loaded.room_id,
                Err(_) => true,
            }
    }
    fn ownership_response_matches(&self, loaded: &MemberOwnershipLoaded) -> bool {
        self.owner.as_ref()==Some(&loaded.owner) && self.origin==loaded.origin && self.epoch==loaded.epoch && self.request==loaded.request
            && self.room.as_ref().is_some_and(|room|room.room_id()==&loaded.room_id)
    }
    fn agent_call_matches(&self, loaded: &RoomAgentsLoaded) -> bool {
        self.owner.as_ref() == Some(&loaded.owner) && self.origin == loaded.origin && self.epoch == loaded.epoch && self.agent_request == loaded.request
            && self.room.as_ref().is_some_and(|room| room.room_id() == &loaded.room_id)
    }
    fn agent_response_matches(&self, loaded: &RoomAgentsLoaded) -> bool {
        self.agent_call_matches(loaded)
            && loaded.result.as_ref().map_or(true, |value| if loaded.binding {value["creation"]["binding"]["roomId"] == loaded.room_id.as_str()} else {value["roomId"] == loaded.room_id.as_str()})
    }
    fn close_agent_picker(&mut self, cx: &mut Cx) {
        let modal = self.view.modal(cx, ids!(agent_picker_modal));
        if self.agent_picker_open || modal.is_open() {
            self.agent_picker_open = false;
            // Modal dismissal does not close a dropdown's separate popup.
            // Release its sweep lock before hiding the containing modal.
            if let Some(mut dropdown) = self.view.drop_down(cx, ids!(my_agents)).borrow_mut() {
                dropdown.set_closed(cx);
            }
            modal.close(cx);
            cx.redraw_all();
        }
    }
    fn show_agent_slow(&mut self, cx: &mut Cx, binding: bool) {
        self.agent_slow=true;
        self.view.label(cx,ids!(agent_status)).set_text(cx,crate::i18n::tr(if binding {
            "Still waiting for the server. You can close this panel; the original request will continue."
        } else {"Checking is taking longer than expected. You can close this panel and return later."}));
        self.view.redraw(cx);
    }
    fn finish_agent_request(&mut self, cx: &mut Cx, loaded: &RoomAgentsLoaded) {
        // A response for this exact call always releases its waiting state.
        // A malformed/cross-room payload is an error, not an ignored completion.
        self.agent_busy=false;
        self.agent_slow=false;
        self.agent_added=false;
        self.agent_pending=false;
        self.view.button(cx,ids!(resume_agent_requests)).set_visible(cx,false);
        let result = if self.agent_response_matches(loaded) {loaded.result.clone()}
            else {Err("invalid_server_response".into())};
        let mut error = None;
        let message = match &result {
            Ok(value) if loaded.binding => match value["commandState"].as_str() {
                Some("active") if value["creation"]["binding"]["state"] == "active" => {
                    self.agent_added=true;
                    "Agent added. In a group, @ the Agent to request a reply."
                }
                Some("pending") => {
                    self.agent_pending=true;
                    "The request is saved. The Agent is still joining. You can close this panel or check its status."
                }
                Some("suspended") => {error=Some("agent_binding_suspended");agent_binding_error("agent_binding_suspended")}
                _ => {error=Some("invalid_server_response");agent_binding_error("invalid_server_response")}
            },
            Ok(value) => {
                self.agent_project=value["projectId"].as_str().unwrap_or("").into();
                self.agents=value["agents"].as_array().map(|rows|rows.iter().filter(|a|a["state"] == "active").cloned().collect()).unwrap_or_default();
                let labels=std::iter::once(crate::i18n::tr("Choose my Agent").to_owned()).chain(self.agents.iter().map(|a|a["displayName"].as_str().filter(|name|!name.is_empty()).unwrap_or("Agent").to_owned())).collect();
                self.view.drop_down(cx,ids!(my_agents)).set_labels(cx,labels);
                self.view.drop_down(cx,ids!(my_agents)).set_selected_item(cx,0);
                if self.agents.is_empty() {"No active Agents owned by you are available."}
                else {"Choose an Agent you own to bind to this Room."}
            },
            Err(code) => {error=Some(code.as_str());agent_binding_error(code)},
        };
        let mut label=self.view.label(cx,ids!(agent_status));
        label.set_text(cx,crate::i18n::tr(message));
        if error.is_some() {script_apply_eval!(cx,label,{draw_text +: {color: mod.widgets.RINX_DANGER_FG}});}
        else if self.agent_added {script_apply_eval!(cx,label,{draw_text +: {color: mod.widgets.RINX_SUCCESS_FG}});}
        else {script_apply_eval!(cx,label,{draw_text +: {color: mod.widgets.RINX_MUTED}});}
        self.view.button(cx,ids!(resume_agent_requests)).set_visible(cx,error==Some("agent_command_recovery_required"));
        cx.redraw_all();
    }
    fn request_agents(&mut self, cx: &mut Cx, binding: bool) {
        if self.agent_busy || !self.pin_is_current() {return;}
        let (Some(owner),Some(room)) = (self.owner.clone(),self.room.clone()) else {return;};
        let service = crate::hagency::backend::Service::for_current().ok();
        #[cfg(feature = "onboarding-test-support")]
        let offline_review = self.review_mode && get_client().is_none();
        #[cfg(not(feature = "onboarding-test-support"))]
        let offline_review = false;
        if service.is_none() && !offline_review {return;}
        let command = if binding {
            let Some(index) = self.view.drop_down(cx,ids!(my_agents)).selected_item().checked_sub(1) else {return;};
            let Some(agent) = self.agents.get(index).and_then(|a| a["id"].as_str()) else {return;};
            if self.agent_project.is_empty() {return;}
            let fingerprint = serde_json::to_string(&(agent,&self.agent_project,room.room_id())).unwrap_or_default();
            if self.agent_command.as_ref().is_none_or(|(old,_)|old != &fingerprint) {self.agent_command=Some((fingerprint,uuid::Uuid::new_v4().to_string()));}
            Some(crate::hagency::model::Command::BindAgent {agent_id:agent.into(),input:serde_json::json!({"projectId":self.agent_project,"roomId":room.room_id(),"idempotencyKey":self.agent_command.as_ref().unwrap().1})})
        } else {None};
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.agent_request=NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
        let request=self.agent_request;
        let origin=self.origin.clone();
        let epoch=self.epoch;
        let room_id=room.room_id().clone();
        self.agent_busy=true;
        self.agent_slow=false;
        self.agent_added=false;
        self.agent_pending=false;
        self.agent_picker_open=true;
        self.view.label(cx, ids!(agent_room)).set_text(cx, &room.to_string());
        self.view.modal(cx, ids!(agent_picker_modal)).open(cx);
        self.view.label(cx,ids!(agent_status)).set_text(cx,crate::i18n::tr(if binding {"Adding Agent to this Room…"} else {"Checking this Room and your Agents…"}));
        let mut status = self.view.label(cx,ids!(agent_status));
        script_apply_eval!(cx,status,{draw_text +: {color: mod.widgets.RINX_MUTED}});
        self.view.button(cx,ids!(resume_agent_requests)).set_visible(cx,false);
        if !binding {self.view.button(cx, ids!(resume_agent_requests)).set_visible(cx, false);self.agents.clear(); self.agent_project.clear(); self.view.drop_down(cx,ids!(my_agents)).set_labels(cx,vec![crate::i18n::tr("Choose my Agent").into()]);}
        if let Some(service) = service {spawn_async_task(async move {
            let future = async {
                if let Some(command)=command {service.bind_agent_to_room(command).await}
                else {service.agents_for_registered_room(room_id.as_str()).await}
            };
            tokio::pin!(future);
            let result = tokio::select! {
                result = &mut future => result,
                _ = tokio::time::sleep(std::time::Duration::from_secs(15)) => {
                    Cx::post_action(RoomAgentsSlow {owner:owner.clone(),origin:origin.clone(),epoch,room_id:room_id.clone(),request,binding});
                    future.await
                }
            };
            Cx::post_action(RoomAgentsLoaded {owner,origin,epoch,room_id:room_id.clone(),request,binding,result});
        });}
        cx.redraw_all();
    }
    fn clear(&mut self) {
        self.owner = None;
        self.origin.clear();
        self.epoch = 0;
        self.room = None;
        self.request = 0;
        self.load_deadline = None;
        self.data = None;
        self.member_owners.clear();
        self.status.clear();
        self.failed = false;
        self.choosing_notifications = false;
        self.adding = false;
        self.removing = false;
        self.renaming = false;
        self.rename_busy = false;
        self.rename_request = 0;
        self.rendered_rows.clear(); self.rendered_cells.clear();
        self.agent_request = 0;
        self.agent_busy = false;
        self.agent_added = false;
        self.agent_pending = false;
        self.agent_slow = false;
        self.agent_project.clear();
        self.agents.clear();
        self.agent_command = None;
        self.agent_picker_open = false;
        self.services_open = false;
        self.services_busy = false;
        self.services_request = 0;
        self.services.clear();
    }
}

impl ScriptHook for MobileChatInfo {}

impl Widget for MobileChatInfo {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.owner.is_some() && !self.pin_is_current() {
            self.clear();
        }
        if self.load_timer.is_event(event).is_some() && self.load_deadline.is_some() {
            self.fail_load(cx, crate::i18n::tr("The chat information request timed out."));
        }
        if matches!(event, Event::Signal) {
            crate::avatar_cache::process_avatar_updates(cx);
            self.view.redraw(cx);
        }
        if !self.agent_picker_open {
            self.close_agent_picker(cx);
        }
        // Let the overlay consume input before the underlying members panel.
        if self.agent_picker_open && !matches!(event, Event::Actions(_) | Event::Signal) {
            self.view.modal(cx, ids!(agent_picker_modal)).handle_event(cx, event, scope);
        } else {
            self.view.handle_event(cx, event, scope);
        }
        if let Event::Actions(actions) = event {
            for action in actions {
                if let Some(verified) = action.downcast_ref::<crate::verification::VerificationStateAction>()
                    && verified.state == matrix_sdk::encryption::VerificationState::Verified
                    && verified.pin.is_current() && self.pin_is_current()
                    && (self.failed || self.load_deadline.is_some())
                    && let Some(room) = self.room.clone()
                { self.fetch_info(cx, room); }
                if let Some(loaded) = action.downcast_ref::<RoomServicesLoaded>()
                    && self.pin_is_current() && self.services_response_matches(loaded) {
                    self.services_busy = false;
                    match &loaded.result {
                        Ok(value) => {
                            self.services = value["services"].as_array().cloned().unwrap_or_default();
                            let labels = std::iter::once(crate::i18n::tr("Choose an Agent").to_owned()).chain(self.services.iter().map(|s|s["displayName"].as_str().filter(|n|!n.is_empty()).unwrap_or("Agent").to_owned())).collect();
                            self.view.drop_down(cx, ids!(room_services)).set_labels(cx, labels);
                            let selected = usize::from(!self.services.is_empty());
                            self.view.drop_down(cx, ids!(room_services)).set_selected_item(cx, selected);
                            let status = self.services.first().map(room_service_status).unwrap_or_else(||crate::i18n::tr("None of your Agents is bound to this Room. Add one with + among the members.").into());
                            self.view.label(cx, ids!(services_status)).set_text(cx, &status);
                        }
                        Err(_) => {self.view.label(cx, ids!(services_status)).set_text(cx, crate::i18n::tr("Could not load Agent services. Refresh to retry."));}
                    }
                    self.view.redraw(cx);
                }
                if let Some(update) = action.downcast_ref::<crate::sliding_sync::UserPowerLevelsUpdated>()
                    && self.pin_is_current() && update.epoch == self.epoch
                    && let Some(data) = self.data.as_mut()
                    && update.is_current_for(data.room.room_id())
                {
                    data.can_invite = update.power_levels.can_invite();
                    self.view.redraw(cx);
                }
                if let Some(update) = action.downcast_ref::<ChatMembershipChanged>()
                    && update.epoch == self.epoch && self.pin_is_current()
                    && self.room.as_ref().is_some_and(|room| room.room_id() == &update.room_id)
                    && let Some(room) = self.room.clone()
                { self.fetch_info(cx, room); }
                // The viewer's rights in the room changed, or a member was just removed:
                // either changes who is listed and who can be removed.
                if let Some(changed) = action.downcast_ref::<MemberAbilitiesInvalidated>()
                    && changed.epoch == self.epoch && self.pin_is_current()
                    && self.room.as_ref().is_some_and(|room| room.room_id() == &changed.room_id)
                    && let Some(room) = self.room.clone()
                { self.fetch_info(cx, room); }
                if matches!(
                    action.downcast_ref::<LogoutAction>(),
                    Some(LogoutAction::ClearAppState { .. })
                ) {
                    self.clear();
                }
                if let Some(slow) = action.downcast_ref::<RoomAgentsSlow>() {
                    let call = slow.call();
                    if self.pin_is_current() && self.agent_call_matches(&call) && self.agent_busy {
                        self.show_agent_slow(cx, slow.binding);
                    }
                }
                if let Some(loaded)=action.downcast_ref::<RoomAgentsLoaded>() {
                    if self.pin_is_current() && self.agent_call_matches(loaded) {
                        self.finish_agent_request(cx, loaded);
                    }
                }
                if let Some(loaded) = action.downcast_ref::<MemberOwnershipLoaded>() {
                    if self.pin_is_current() && self.ownership_response_matches(loaded) {
                        self.member_owners = loaded.result.as_ref().map(|(owners, _)| owners.clone()).unwrap_or_default();
                        if let Some(data)=self.data.as_mut() {group_members(&mut data.members,&self.member_owners);}
                        self.view.redraw(cx);
                    }
                }
                if let Some(loaded) = action.downcast_ref::<ChatInfoLoaded>() {
                    if self.accepts(loaded) {
                        self.finish_load(cx);
                        match &loaded.result {
                            Ok(data) => {
                                let mut data=data.clone();
                                group_members(&mut data.members,&self.member_owners);
                                // With nobody left to remove there is nothing to pick.
                                self.removing &= !data.removable.is_empty();
                                self.adding &= data.can_invite;
                                self.renaming &= data.can_rename;
                                self.data = Some(data);
                                self.failed = false;
                                self.status.clear();
                            }
                            Err(error) => self.fail_load(cx, error),
                        }
                        self.view.redraw(cx);
                    }
                }
                if let Some(renamed) = action.downcast_ref::<RoomRenamed>()
                    && self.pin_is_current() && self.rename_response_matches(renamed)
                { self.finish_rename(cx, renamed); }
                if let Some(update) = action.downcast_ref::<RoomNotificationModeUpdated>() {
                    if let Some(data) = self.data.as_mut() {
                        if data.room.room_id() == &update.room_id {
                            data.mode = update.mode;
                            self.view.redraw(cx);
                        }
                    }
                }
            }
            if self.view.button(cx, ids!(services_back)).clicked(actions) {self.services_open=false;self.view.redraw(cx);return;}
            if self.view.button(cx, ids!(rename_back)).clicked(actions) {self.renaming=false;self.view.redraw(cx);return;}
            if self.renaming && (self.view.button(cx, ids!(rename_save)).clicked(actions)
                || self.view.text_input(cx, ids!(rename_input)).returned(actions).is_some()) {self.save_name(cx);return;}
            if self.services_open && self.view.button(cx, ids!(refresh_services)).clicked(actions) {self.load_services(cx);return;}
            if self.view.drop_down(cx, ids!(room_services)).changed(actions).is_some()
                && let Some(index) = self.view.drop_down(cx, ids!(room_services)).selected_item().checked_sub(1)
                && let Some(service) = self.services.get(index) {
                self.view.label(cx, ids!(services_status)).set_text(cx, &room_service_status(service));
            }
            if self.services_open && self.view.button(cx, ids!(configure_service)).clicked(actions) && self.pin_is_current() && !self.services_busy
                && let Some(index) = self.view.drop_down(cx, ids!(room_services)).selected_item().checked_sub(1)
                && let Some(service) = self.services.get(index)
                && let (Some(agent), Some(binding)) = (service["agentId"].as_str(),service["bindingId"].as_str()) {
                cx.action(crate::hagency::ui::HagencyAction::AgentRun {agent_id:agent.into(),binding_id:binding.into()});
                return;
            }
            if self.view.navigation_bar_button(cx, ids!(history_search)).clicked(actions) || self.view.navigation_bar_button(cx, ids!(history_media)).clicked(actions) {
                if self.pin_is_current() && let Some(data) = self.data.as_ref() {
                    let request = super::room_history::RoomHistoryAction::Open {room: data.room.clone(), filter: if self.view.navigation_bar_button(cx, ids!(history_search)).clicked(actions) {super::room_history::HistoryFilter::Messages} else {super::room_history::HistoryFilter::Media}};
                    if self.history_in_room {cx.widget_action(self.widget_uid(), request);} else {cx.action(request);}
                }
                return;
            }
            if self.view.button(cx, ids!(close_agent_picker)).clicked(actions)
                || self.view.modal(cx, ids!(agent_picker_modal)).dismissed(actions) {
                self.close_agent_picker(cx);
                return;
            }
            if self.view.drop_down(cx,ids!(my_agents)).changed(actions).is_some() && !self.agent_busy {
                self.agent_added=false;self.agent_pending=false;
                self.view.label(cx,ids!(agent_status)).set_text(cx,crate::i18n::tr("Choose an Agent you own to bind to this Room."));
                cx.redraw_all();
            }
            if self.view.button(cx,ids!(invite_selected_agent)).clicked(actions) {self.request_agents(cx,true);return;}
            if self.view.button(cx,ids!(resume_agent_requests)).clicked(actions) {self.close_agent_picker(cx);cx.action(crate::hagency::ui::HagencyAction::OpenAgents);return;}
            if self.view.button(cx, ids!(retry)).clicked(actions) {
                if let Some(room) = self.room.clone() {
                    self.load(cx, room);
                }
                return;
            }
            let list = self.view.portal_list(cx, ids!(list));
            for (_index, widget) in list.items_with_actions(actions) {
                if list.was_scrolling() || !self.pin_is_current() {continue;}
                let Some(row) = self.rendered_rows.get(&widget.widget_uid()).copied() else {continue;};
                match row {
                    InfoRow::Members(_) => {
                        let Some(target) = self.rendered_cells.get(&widget.widget_uid()).and_then(|cells| {
                            cell_ids!().into_iter().zip(cells).find_map(|(id, target)| {
                                target.as_ref().filter(|_| widget.navigation_bar_button(cx, id).clicked(actions)).cloned()
                            })
                        }) else {continue;};
                        self.tap_cell(cx, target);
                    }
                    InfoRow::AddChoice => {
                        let Some(data) = self.data.as_ref() else {continue;};
                        if data.can_invite && widget.button(cx, ids!(invite_people)).clicked(actions) {
                            cx.action(InviteModalAction::Open(data.room.clone()));
                            self.adding = false;
                        } else if widget.button(cx, ids!(invite_my_agent)).clicked(actions) {
                            self.adding = false;
                            self.open_agent_picker(cx);
                        } else {
                            continue;
                        }
                    }
                    _ => {
                        if !(widget.navigation_bar_button(cx, ids!(row)).clicked(actions) || widget.button(cx, ids!(row)).clicked(actions)) {continue;}
                        let Some(data) = self.data.as_ref() else {continue;};
                        match row {
                            InfoRow::Name => self.tap_name(cx),
                            InfoRow::Notifications => self.choosing_notifications = true,
                            InfoRow::Mode(mode) => {
                                if let Some((mode, _))=modes().get(mode) {submit_async_request(MatrixRequest::SetRoomNotificationMode {room_id:data.room.room_id().clone(),mode:*mode});}
                            }
                            InfoRow::Back => self.choosing_notifications = false,
                            InfoRow::AgentServices => {self.services_open = !self.services_open;if self.services_open {self.load_services(cx);}},
                            _ => continue,
                        }
                    }
                }
                self.view.redraw(cx);
                // PortalList returns an item once per grouped child action.
                // A single pointer event can include hover/focus and click;
                // handle its click only once, especially for Matrix writes.
                break;
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if self.owner.is_some() && !self.pin_is_current() {
            self.clear();
        }
        // A covered tab can miss the one-shot result. When it next draws it
        // must still leave the loading state and offer a new request.
        self.expire_pending_load(cx, std::time::Instant::now());
        self.view.view(cx, ids!(rename_panel)).set_visible(cx, self.renaming);
        self.view.button(cx, ids!(rename_save)).set_enabled(cx, !self.rename_busy);
        self.view.view(cx, ids!(services_panel)).set_visible(cx, self.services_open && !self.renaming);
        self.view.view(cx, ids!(information_list)).set_visible(cx, !self.services_open && !self.renaming);
        let service_selected = self.view.drop_down(cx, ids!(room_services)).selected_item().checked_sub(1).is_some_and(|i| i < self.services.len());
        self.view.button(cx, ids!(configure_service)).set_enabled(cx, service_selected && !self.services_busy && self.pin_is_current());
        self.view.button(cx, ids!(refresh_services)).set_enabled(cx, !self.services_busy);
        if !self.agent_picker_open {
            self.close_agent_picker(cx);
        }
        let summary_room = self.data.as_ref().map(|data|&data.room).or(self.room.as_ref());
        self.view.label(cx, ids!(summary_name)).set_text(cx, &summary_room.map_or_else(|| crate::i18n::tr("Chat information").to_owned(), |room|room.display().into_owned()));
        let count = self.data.as_ref().map_or_else(|| crate::i18n::tr("Loading members…").to_owned(), |data| crate::i18n::format("{0} members", &[("0",data.members.len().saturating_sub(data.banned_members.len()).to_string())]));
        let summary_count = self.view.label(cx, ids!(summary_count));
        summary_count.set_text(cx, &count);
        summary_count.set_visible(cx, self.data.is_some() || (self.request != 0 && !self.failed));
        let selected=self.view.drop_down(cx,ids!(my_agents)).selected_item().checked_sub(1).is_some_and(|i|i<self.agents.len());
        self.view.button(cx,ids!(invite_selected_agent)).set_enabled(cx,selected && !self.agent_project.is_empty() && !self.agent_busy && !self.agent_added);
        self.view.drop_down(cx,ids!(my_agents)).set_disabled(cx,self.agent_busy);
        self.view.button(cx,ids!(invite_selected_agent)).set_text(cx,crate::i18n::tr(if self.agent_added {"Added"} else if self.agent_pending {"Check status"} else {"Add agent"}));
        self.view.button(cx,ids!(close_agent_picker)).set_text(cx,crate::i18n::tr(if self.agent_added {"Done"} else {"Close"}));
        self.view.label(cx, ids!(status)).set_text(cx, &self.status);
        self.view
            .label(cx, ids!(status))
            .set_visible(cx, !self.status.is_empty());
        self.view
            .button(cx, ids!(retry))
            .set_visible(cx, self.failed);
        let (columns, cell_width) = grid_columns(cx.peek_walk_turtle(walk).size.x);
        let page = self.data.as_ref().map(|data| InfoPage {
            members: data.members.len(), can_remove: !data.removable.is_empty(), columns,
            adding: self.adding, topic: !data.topic.is_empty(), choosing: self.choosing_notifications,
        });
        self.rendered_rows.clear(); self.rendered_cells.clear();
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.borrow_mut::<PortalList>() {
                let count = page.map_or(0, |page| page.rows());
                list.set_item_range(cx, 0, count);
                while let Some(index) = list.next_visible_item(cx) {
                    let (Some(page), Some(data)) = (page.filter(|_| index < count), self.data.as_mut()) else {
                        list.item(cx, index, id!(Filler)).draw_all(cx, scope);
                        continue;
                    };
                    let row = page.row(index);
                    let widget = match row {
                        InfoRow::Members(first) => {
                            let widget = list.item(cx, index, id!(MemberRow));
                            let mut targets = Vec::with_capacity(MAX_COLUMNS);
                            for (slot, id) in cell_ids!().into_iter().enumerate() {
                                let cell = widget.widget(cx, id);
                                let kind = (slot < page.columns).then(|| page.cell(first + slot)).flatten();
                                if let Some(mut button) = cell.borrow_mut::<NavigationBarButton>() {
                                    button.view.set_visible(cx, kind.is_some());
                                    button.view.walk.width = Size::Fixed(cell_width);
                                }
                                let member = match kind {
                                    Some(InfoCell::Member(member)) => data.members.get_mut(member),
                                    _ => None,
                                };
                                cell.view(cx, ids!(photo)).set_visible(cx, member.is_some());
                                cell.view(cx, ids!(tile)).set_visible(cx, matches!(kind, Some(InfoCell::Add | InfoCell::Remove)));
                                cell.view(cx, ids!(upright)).set_visible(cx, kind == Some(InfoCell::Add));
                                let (name, note, marked, target) = match (kind, member) {
                                    (Some(InfoCell::Member(_)), Some(profile)) => {
                                        let service = self.owner.as_ref().is_some_and(|owner| is_hagency_service(&profile.user_id, owner));
                                        let name = if service {crate::i18n::tr("Hagency service")} else {profile.displayable_name()}.to_owned();
                                        let note = if data.banned_members.contains(&profile.user_id) {crate::i18n::tr("Banned")}
                                            else {crate::agent_access::framework_label(&profile.user_id).unwrap_or("")};
                                        let avatar = cell.avatar(cx, ids!(avatar));
                                        let loaded = profile.avatar_state.update_from_cache(cx).is_some_and(|image| {
                                            avatar.show_image(cx, None, |cx, img| utils::load_avatar_image(&img, cx, image)).is_ok()
                                        });
                                        if !loaded {
                                            avatar.show_user_text(cx, &profile.user_id, profile.displayable_name());
                                        }
                                        let marked = self.removing && data.removable.contains(&profile.user_id);
                                        (name, note, marked, Some(CellTarget::Member(profile.clone())))
                                    }
                                    (Some(InfoCell::Add), _) => (crate::i18n::tr("Add").to_owned(), "", false, Some(CellTarget::Add)),
                                    (Some(InfoCell::Remove), _) => (
                                        crate::i18n::tr(if self.removing {"Done"} else {"Kick"}).to_owned(), "", false, Some(CellTarget::Remove),
                                    ),
                                    _ => (String::new(), "", false, None),
                                };
                                cell.view(cx, ids!(badge)).set_visible(cx, marked);
                                cell.label(cx, ids!(name)).set_text(cx, &name);
                                cell.label(cx, ids!(note)).set_text(cx, note);
                                targets.push(target);
                            }
                            self.rendered_cells.insert(widget.widget_uid(), targets);
                            widget
                        }
                        InfoRow::AddChoice => {
                            let widget = list.item(cx, index, id!(AddChoice));
                            widget.button(cx, ids!(invite_people)).set_visible(cx, data.can_invite);
                            widget.button(cx, ids!(invite_my_agent)).set_enabled(cx, self.owner.is_some());
                            widget
                        }
                        InfoRow::Gap => list.item(cx, index, id!(Gap)),
                        InfoRow::Topic => {
                            let widget = list.item(cx, index, id!(Topic));
                            widget.label(cx, ids!(topic)).set_text(cx, &data.topic);
                            widget
                        }
                        InfoRow::Back => list.item(cx, index, id!(Back)),
                        InfoRow::Filler => list.item(cx, index, id!(Filler)),
                        _ => {
                            let widget = list.item(cx, index, id!(Action));
                            let name = shown_name(&data.room.display());
                            // The arrow promises that a tap leads somewhere.
                            widget.view(cx, ids!(chevron)).set_visible(cx, row != InfoRow::Name || data.can_rename);
                            let (title, value) = match row {
                                InfoRow::Name => (crate::i18n::tr("Room name"), name.as_str()),
                                InfoRow::Mode(mode) => modes().get(mode).map(|(mode, title)|(*title, if *mode == data.mode {"✓"} else {""})).unwrap_or(("", "")),
                                InfoRow::Notifications => (crate::i18n::tr("Notifications"), match data.mode {
                                    Some(RoomNotificationMode::Mute) => crate::i18n::tr("Muted"),
                                    Some(RoomNotificationMode::AllMessages) => crate::i18n::tr("All"),
                                    Some(RoomNotificationMode::MentionsAndKeywordsOnly) => crate::i18n::tr("Mentions"),
                                    None => crate::i18n::tr("Default"),
                                }),
                                InfoRow::AgentServices => (crate::i18n::tr("Room Agent services"), ""),
                                _ => ("", ""),
                            };
                            widget.label(cx, ids!(title)).set_text(cx, title);
                            widget.label(cx, ids!(value)).set_text(cx, value);
                            widget
                        }
                    };
                    self.rendered_rows.insert(widget.widget_uid(), row);
                    widget.draw_all(cx, scope);
                }
            }
        }
        DrawStep::done()
    }
}

impl MobileChatInfoRef {
    /// Isolated native UX fixtures: no signed-in client or network commands.
    #[cfg(feature = "onboarding-test-support")]
    pub fn review_agent_picker(&self, cx: &mut Cx, scenario: &str) {
        assert!(get_client().is_none(), "Agent picker review requires an offline profile");
        let Some(mut panel) = self.borrow_mut() else {return;};
        panel.review_mode=true;
        panel.owner=Some("@review:example.invalid".try_into().unwrap());
        panel.origin="https://example.invalid/".into();
        let room: OwnedRoomId="!review:example.invalid".try_into().unwrap();
        panel.room=Some(RoomNameId::new(matrix_sdk::RoomDisplayName::Named("Team room".into()),room.clone()));
        if scenario=="members" {
            panel.data=Some(ChatInfoData {can_invite:true,room:panel.room.clone().unwrap(),topic:String::new(),members:Vec::new(),banned_members:HashSet::new(),removable:HashSet::new(),can_rename:false,mode:None});
            panel.view.redraw(cx);
            return;
        }
        panel.agent_picker_open=true;
        panel.view.modal(cx,ids!(agent_picker_modal)).open(cx);
        panel.view.label(cx,ids!(agent_room)).set_text(cx,"Team room");
        if scenario=="slow" {panel.show_agent_slow(cx,true);return;}
        let result=match scenario {
            "ready" => Ok(serde_json::json!({"roomId":room,"projectId":"review-project","agents":[{"id":"review-agent","displayName":"小白菜","state":"active"}]})),
            "success" => Ok(serde_json::json!({"commandState":"active","creation":{"binding":{"roomId":room,"state":"active"}}})),
            "pending" => Ok(serde_json::json!({"commandState":"pending","creation":{"binding":{"roomId":room,"state":"pending"}}})),
            "malformed" => Ok(serde_json::json!({"commandState":"active"})),
            error => Err(error.to_owned()),
        };
        let loaded=RoomAgentsLoaded {owner:panel.owner.clone().unwrap(),origin:panel.origin.clone(),epoch:panel.epoch,
            room_id:room,request:panel.agent_request,binding:scenario!="ready",result};
        panel.finish_agent_request(cx,&loaded);
        if scenario=="ready" {panel.view.drop_down(cx,ids!(my_agents)).set_selected_item(cx,1);}
        panel.view.redraw(cx);
    }
    #[cfg(test)]
    pub(crate) fn displayed_room_id(&self) -> Option<OwnedRoomId> {
        self.borrow()
            .and_then(|inner| inner.room.as_ref().map(|room| room.room_id().clone()))
    }
    pub fn update_room(&self, cx: &mut Cx, room: RoomNameId, opening: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.update_room(cx, room, opening);
        }
    }

    pub fn close_picker(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.close_agent_picker(cx);
        }
    }
    pub fn show(&self, cx: &mut Cx, room: RoomNameId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.load(cx, room);
        }
    }

    pub fn clear(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear();
        }
    }
}

#[cfg(test)]
mod information_target_tests {
    use super::*;
    fn member(id: &str, name: &str) -> UserProfile {
        UserProfile {user_id:OwnedUserId::try_from(id).unwrap(),username:Some(name.into()),avatar_state:AvatarState::Known(None)}
    }
    #[test]
    fn agents_follow_only_their_present_human_owner_without_losing_members() {
        let mut members=vec![member("@alice:s","Alice"),member("@bob:s","Bob"),member("@a:s","Z agent"),member("@b:s","A agent"),member("@orphan:s","Orphan"),member("@external:s","External"),member("@_hagency_service:s","Hagency service")];
        let mut owners=HashMap::new();let mut conflicts=HashSet::new();
        add_member_owners(&mut owners,&mut conflicts,&serde_json::json!([
            {"puppetMxid":"@a:s","ownerMxid":"@alice:s"},
            {"puppetMxid":"@b:s","ownerMxid":"@bob:s"},
            {"puppetMxid":"@orphan:s","ownerMxid":"@missing:s"},
            {"puppetMxid":"@_hagency_service:s","ownerMxid":"@alice:s"}
        ]),None);
        let indented=group_members(&mut members,&owners);
        let ids:Vec<_>=members.iter().map(|p|p.user_id.as_str()).collect();
        assert_eq!(ids,vec!["@alice:s","@a:s","@bob:s","@b:s","@external:s","@orphan:s","@_hagency_service:s"],"the service comes last");
        assert_eq!(indented.len(),2);
        assert!(indented.contains(&OwnedUserId::try_from("@a:s").unwrap()));
        assert!(!indented.contains(&OwnedUserId::try_from("@orphan:s").unwrap()));
        assert_eq!(group_members(&mut members,&owners),indented,"refresh preserves grouping");
    }
    fn new_panel(cx: &mut Cx) -> MobileChatInfo {
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);vm.bx.captured_errors=Some(Vec::new());crate::app::register_widgets(vm);
            let value=script_eval!(vm,{mod.widgets.MobileChatInfo {}});
            let info=MobileChatInfo::script_from_value(vm,value);assert!(vm.take_errors().is_empty());info
        })
    }
    fn user(id: &str) -> OwnedUserId {OwnedUserId::try_from(id).unwrap()}
    #[test]
    fn members_fill_a_grid_that_ends_in_the_add_tile_and_for_a_remover_the_remove_tile() {
        let mut members=vec![member("@alice:s","Alice"),member("@a:s","Z agent"),member("@bob:s","Bob"),member("@b:s","A agent"),member("@carol:s","Carol")];
        group_members(&mut members,&HashMap::new());
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut info=new_panel(&mut cx);
        info.data=Some(ChatInfoData {can_invite:true,room:RoomNameId::empty(OwnedRoomId::try_from("!r:s").unwrap()),topic:String::new(),members,banned_members:HashSet::from([user("@carol:s")]),removable:HashSet::new(),can_rename:false,mode:None});
        draw_information(&mut info,&mut cx);
        let list=info.view.portal_list(&mut cx,ids!(list));
        // What a cell shows: (shown at all, photo, tile, the tile's upright bar, badge, name, note).
        let cell=|cx:&mut Cx,row:usize,slot:usize| {
            let cell=list.item(cx,row,id!(MemberRow)).widget(cx,cell_ids!()[slot]);
            let shown=cell.borrow::<NavigationBarButton>().unwrap().view.visible();
            (shown,cell.view(cx,ids!(photo)).visible(),cell.view(cx,ids!(tile)).visible(),cell.view(cx,ids!(upright)).visible(),
                cell.view(cx,ids!(badge)).visible(),cell.label(cx,ids!(name)).text(),cell.label(cx,ids!(note)).text())
        };
        let rect=|cx:&mut Cx,row:usize,slot:usize| list.item(cx,row,id!(MemberRow)).widget(cx,cell_ids!()[slot]).area().rect(cx);
        // A 280 wide panel holds four cells to a row: five members and the tile that adds.
        for (slot,name) in ["A agent","Alice","Bob","Carol"].into_iter().enumerate() {
            let note=if name=="Carol" {"Banned"} else {""};
            assert_eq!(cell(&mut cx,0,slot),(true,true,false,false,false,name.to_owned(),note.to_owned()));
            let here=rect(&mut cx,0,slot);
            assert_eq!((here.pos.x,here.size.x,here.size.y),(12.0+64.0*slot as f64,64.0,84.0));
        }
        for slot in [4,5] {assert!(!cell(&mut cx,0,slot).0,"a row holds no more than its columns");}
        let avatar=list.item(&mut cx,0,id!(MemberRow)).widget(&mut cx,ids!(c0)).avatar(&mut cx,ids!(avatar)).area().rect(&cx);
        assert_eq!(avatar.size,dvec2(40.0,40.0));
        assert_eq!(cell(&mut cx,1,0),(true,true,false,false,false,"Z agent".to_owned(),String::new()));
        assert_eq!(cell(&mut cx,1,1),(true,false,true,true,false,"Add".to_owned(),String::new()));
        assert!(!cell(&mut cx,1,2).0,"nobody here can be removed, so no tile offers it");
        assert_eq!(rect(&mut cx,1,0).pos.y-rect(&mut cx,0,0).pos.y,84.0);
        assert_eq!(info.rendered_rows.values().filter(|row|matches!(row,InfoRow::Members(_))).count(),2);

        // The viewer may remove Bob and the Agent: the tile that removes follows the tile that adds.
        info.data.as_mut().unwrap().removable=HashSet::from([user("@bob:s"),user("@a:s")]);
        draw_information(&mut info,&mut cx);
        assert_eq!(cell(&mut cx,1,2),(true,false,true,false,false,"Kick".to_owned(),String::new()));
        assert!(!cell(&mut cx,0,2).4,"nobody is marked before the tile is tapped");
        // A tap on a member opens their profile; nothing is asked of the homeserver.
        let bob=member("@bob:s","Bob");
        let opened=cx.capture_actions(|cx|info.tap_cell(cx,CellTarget::Member(bob.clone())));
        assert!(opened.iter().all(|action|action.downcast_ref::<ModerationModalAction>().is_none()));

        info.tap_cell(&mut cx,CellTarget::Remove);
        assert!(info.removing && !info.adding);
        draw_information(&mut info,&mut cx);
        let marked:Vec<_>=[(0,0),(0,1),(0,2),(0,3),(1,0)].into_iter().map(|(row,slot)|cell(&mut cx,row,slot).4).collect();
        assert_eq!(marked,[false,false,true,false,true],"only the members who can be removed are marked");
        assert_eq!(cell(&mut cx,1,2).5,"Done");
        // Now a tap on a marked member asks for confirmation, naming that member and this room.
        let asked=cx.capture_actions(|cx|info.tap_cell(cx,CellTarget::Member(bob)));
        let asked:Vec<_>=asked.iter().filter_map(|action|action.downcast_ref::<ModerationModalAction>()).collect();
        assert!(matches!(asked.as_slice(),[ModerationModalAction::Open(ModerationRequest::Member(MemberModeration {room_id,user_id,action:MemberAction::Kick,..}))]
            if room_id.as_str()=="!r:s" && user_id.as_str()=="@bob:s"),"{asked:?}");
        // A tap on anyone else does nothing while members are being removed.
        let ignored=cx.capture_actions(|cx|info.tap_cell(cx,CellTarget::Member(member("@alice:s","Alice"))));
        assert!(ignored.is_empty());
        info.tap_cell(&mut cx,CellTarget::Remove);
        assert!(!info.removing);
        cx.with_vm(|vm|assert!(vm.take_errors().is_empty()));
    }
    #[test]
    fn only_members_below_the_viewer_can_be_removed_and_never_the_service() {
        use matrix_sdk::ruma::{events::room::power_levels::RoomPowerLevelsEventContent, int, user_id};
        use ruma::room_version_rules::AuthorizationRules;
        let mut content=RoomPowerLevelsEventContent::new(&AuthorizationRules::V1);
        content.users.insert(user_id!("@owner:s").to_owned(),int!(100));
        content.users.insert(user_id!("@peer:s").to_owned(),int!(100));
        let levels=RoomPowerLevels::new(content.into(),&AuthorizationRules::V1,[]);
        let members=vec![member("@owner:s","Owner"),member("@peer:s","Peer"),member("@bob:s","Bob"),member("@agent:s","Agent"),
            member("@gone:s","Gone"),member("@_hagency_service:s","Hagency service")];
        let banned=HashSet::from([user("@gone:s")]);
        let sorted=|users:HashSet<OwnedUserId>| {let mut users:Vec<_>=users.into_iter().map(String::from).collect();users.sort();users};
        // The room's owner: people and Agents alike, but not an equal, not themselves,
        // not someone already out of the room, and not the service that invites Agents.
        assert_eq!(sorted(removable_members(&levels,user_id!("@owner:s"),&members,&banned)),["@agent:s","@bob:s"]);
        // Anyone else in the room: nobody.
        assert!(removable_members(&levels,user_id!("@bob:s"),&members,&banned).is_empty());
        assert!(removable_members(&levels,user_id!("@agent:s"),&members,&banned).is_empty());
    }
    #[test]
    fn conflicting_ownership_and_cycles_never_create_false_grouping() {
        let mut owners=HashMap::new();let mut conflicts=HashSet::new();
        add_member_owners(&mut owners,&mut conflicts,&serde_json::json!([
            {"puppetMxid":"@agent:s","ownerMxid":"@alice:s"},
            {"puppetMxid":"@agent:s","ownerMxid":"@bob:s"},
            {"puppetMxid":"@agent:s","ownerMxid":"@alice:s"},
            {"puppetMxid":"@a:s","ownerMxid":"@b:s"},
            {"puppetMxid":"@b:s","ownerMxid":"@a:s"},
            {"puppetMxid":"invalid","ownerMxid":"@alice:s"}
        ]),None);
        assert!(!owners.contains_key(&OwnedUserId::try_from("@agent:s").unwrap()));
        let mut members=vec![member("@a:s","Same"),member("@b:s","Same"),member("@agent:s","Same")];
        assert!(group_members(&mut members,&owners).is_empty());assert_eq!(members.len(),3);
    }
    #[test]
    fn owner_grouping_rejects_stale_room_account_and_request_results() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut info=cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);crate::app::register_widgets(vm);
            let value=script_eval!(vm,{mod.widgets.MobileChatInfo {}});
            MobileChatInfo::script_from_value(vm,value)
        });
        info.owner=Some(OwnedUserId::try_from("@alice:s").unwrap());info.origin="https://s".into();info.epoch=4;info.request=8;
        info.room=Some(RoomNameId::empty(OwnedRoomId::try_from("!r:s").unwrap()));
        let mut result=MemberOwnershipLoaded{owner:info.owner.clone().unwrap(),origin:info.origin.clone(),epoch:4,request:8,room_id:OwnedRoomId::try_from("!r:s").unwrap(),result:Ok((HashMap::new(),false))};
        assert!(info.ownership_response_matches(&result));
        result.request=7;assert!(!info.ownership_response_matches(&result));result.request=8;
        result.epoch=3;assert!(!info.ownership_response_matches(&result));result.epoch=4;
        result.room_id=OwnedRoomId::try_from("!other:s").unwrap();assert!(!info.ownership_response_matches(&result));result.room_id=OwnedRoomId::try_from("!r:s").unwrap();
        result.owner=OwnedUserId::try_from("@bob:s").unwrap();assert!(!info.ownership_response_matches(&result));
    }
    #[test]
    fn explicit_room_pause_and_membership_loss_have_different_service_messages() {
        assert!(room_service_status(&serde_json::json!({"bindingState":"suspended","ownerServicePaused":true})).contains("pause notice"));
        assert!(room_service_status(&serde_json::json!({"bindingState":"suspended","ownerServicePaused":false})).contains("access unavailable"));
    }
    #[test]
    fn room_service_status_never_substitutes_a_running_private_chat() {
        let service = serde_json::json!({"agentId":"agent","bindingId":"project-room","thisDevice":true,"statusAvailable":true,"runtime":{"agentId":"agent","bindingId":"private-chat","phase":"online","bindings":[{"bindingId":"private-chat","phase":"online"}]}});
        assert!(room_service_status(&service).starts_with("This Room is not running"));
        let mut running = service.clone();
        running["runtime"]["bindings"][0]["bindingId"] = "project-room".into();
        assert!(room_service_status(&running).starts_with("Responding in this Room"));
        running["thisDevice"] = false.into();
        assert_eq!(room_service_status(&running), "Not assigned to this device.");
        running["thisDevice"] = true.into();running["statusAvailable"] = false.into();
        assert!(room_service_status(&running).starts_with("Could not verify"));
    }
    #[test]
    fn service_results_cannot_cross_room_or_account_or_request() {
        let mut cx = Cx::new(Box::new(|_,_|{}));
        let mut info = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);crate::app::register_widgets(vm);
            let value = script_eval!(vm, {mod.widgets.MobileChatInfo {}});
            MobileChatInfo::script_from_value(vm, value)
        });
        let owner = matrix_sdk::ruma::user_id!("@owner:server").to_owned();
        let room = matrix_sdk::ruma::room_id!("!target:server").to_owned();
        info.owner = Some(owner.clone());info.origin = "https://server/".into();info.epoch = 4;info.room = Some(RoomNameId::empty(room.clone()));info.services_request = 8;
        let mut result = RoomServicesLoaded {owner,origin:info.origin.clone(),epoch:4,room_id:room,request:8,result:Ok(serde_json::json!({"roomId":"!target:server"}))};
        assert!(info.services_response_matches(&result));
        result.result = Ok(serde_json::json!({"roomId":"!other:server"}));assert!(!info.services_response_matches(&result));
        result.result = Err("offline".into());result.request = 7;assert!(!info.services_response_matches(&result));
        result.request = 8;result.epoch = 3;assert!(!info.services_response_matches(&result));
        result.epoch = 4;result.owner = matrix_sdk::ruma::user_id!("@other:server").to_owned();assert!(!info.services_response_matches(&result));
        info.clear();assert!(!info.services_response_matches(&result));
    }
    #[test]
    fn every_cell_and_row_of_the_page_has_one_meaning() {
        assert_eq!(grid_columns(280.0),(4,64.0));
        assert_eq!(grid_columns(390.0),(5,73.0));
        assert_eq!(grid_columns(1000.0),(6,84.0));
        assert_eq!(grid_columns(0.0),(4,62.0),"a panel not laid out yet still gets a usable grid");
        for members in [0, 1, 3, 4, 5, 200] {
            for variant in 0..48 {
                let (can_remove, adding, choosing, topic, columns) = (variant & 1 == 1, variant & 2 == 2, variant & 4 == 4, variant & 8 == 8, 4 + variant / 16);
                let page = InfoPage {members, can_remove, columns, adding, topic, choosing};
                // Members in their order, then the tile that adds, then the tile that removes.
                for index in 0..members {assert_eq!(page.cell(index), Some(InfoCell::Member(index)));}
                assert_eq!(page.cell(members), Some(InfoCell::Add));
                assert_eq!(page.cell(members + 1), can_remove.then_some(InfoCell::Remove));
                assert_eq!(page.cell(members + 2), None);
                // The grid's rows hold every cell once and no row is empty.
                let grid = page.grid_rows();
                assert!(grid * columns >= page.cells() && (grid - 1) * columns < page.cells());
                for row in 0..grid {assert_eq!(page.row(row), InfoRow::Members(row * columns));}
                let mut next = grid;
                if adding {assert_eq!(page.row(next), InfoRow::AddChoice);next += 1;}
                assert_eq!(page.row(next), InfoRow::Gap);
                assert_eq!(page.row(next + 1), InfoRow::Name);
                next += 2;
                if topic {assert_eq!(page.row(next), InfoRow::Topic);next += 1;}
                if choosing {
                    for mode in 0..4 {assert_eq!(page.row(next + mode), InfoRow::Mode(mode));}
                    assert_eq!(page.row(next + 4), InfoRow::Back);
                    assert_eq!(page.rows(), next + 5);
                } else {
                    assert_eq!(page.row(next), InfoRow::Notifications);
                    assert_eq!(page.row(next + 1), InfoRow::AgentServices);
                    assert_eq!(page.rows(), next + 2);
                }
                assert_eq!(page.row(page.rows()), InfoRow::Filler);
            }
        }
    }
    #[test]
    fn service_label_identifies_only_exact_service_identity_on_owner_server() {
        let owner=matrix_sdk::ruma::user_id!("@alice:server");
        assert!(is_hagency_service(matrix_sdk::ruma::user_id!("@_hagency_service:server"),owner));
        assert!(!is_hagency_service(matrix_sdk::ruma::user_id!("@_hagency_service:other"),owner));
        assert!(!is_hagency_service(matrix_sdk::ruma::user_id!("@agent:server"),owner));
    }
    fn draw_information(info: &mut MobileChatInfo, cx: &mut Cx) {
        use makepad_widgets::makepad_draw::cx_draw::CxDraw;
        cx.redraw_all();let pass=DrawPass::new(cx);pass.set_size(cx,dvec2(280.0,775.0));
        let mut list=DrawList2d::new(cx);let overlay=Overlay{draw_list:DrawList::new(cx)};
        let event=DrawEvent::default();let mut draw=CxDraw::new(cx,&event);let mut cx2d=Cx2d::new(&mut draw);
        cx2d.begin_pass(&pass,None);list.begin_always(&mut cx2d);overlay.begin(&mut cx2d);
        cx2d.begin_root_turtle(dvec2(280.0,775.0),Layout::flow_overlay());
        info.draw_walk_all(&mut cx2d,&mut Scope::empty(),Walk{width:Size::Fixed(280.0),height:Size::Fixed(775.0),..Walk::default()});
        overlay.end(&mut cx2d);cx2d.end_pass_sized_turtle();list.end(&mut cx2d);cx2d.end_pass(&pass);
    }
    #[test]
    fn verified_session_reloads_room_information_and_ignores_previous_device() {
        use crate::matrix_context::SessionPin;
        use crate::settings::crypto_settings::test_support::{Server, install_link_client};
        use crate::verification::VerificationStateAction;
        use std::{cell::RefCell, rc::Rc};

        const TEST: &str = "home::mobile_chat_info::information_target_tests::verified_session_reloads_room_information_and_ignores_previous_device";
        if std::env::var_os("HAGENCY_INFO_VERIFY_TEST").is_none() {
            let profile = tempfile::tempdir().unwrap();
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TEST, "--test-threads=1"])
                .env("HAGENCY_INFO_VERIFY_TEST", "1")
                .env("HAGENCY_DESKTOP_DATA_DIR", profile.path())
                .env("HAGENCY_NATIVE_CREDENTIAL_IO", "deny")
                .output().unwrap();
            assert!(result.status.success(), "{}\n{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
            assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
            return;
        }

        crate::sliding_sync::backend_runtime().block_on(async {
            let server = Server::with_handler(false, std::sync::Arc::new(|_| None));
            let old = server.room().await.client();
            install_link_client(Some(old.clone()));
            let old_pin = SessionPin::capture(&old).unwrap();
            let room = server.room().await;
            let current = room.client();
            install_link_client(Some(current.clone()));

            let panel = Rc::new(RefCell::new(None::<MobileChatInfo>));
            let listener = Rc::clone(&panel);
            let mut cx = Cx::new(Box::new(move |cx, event| {
                if matches!(event, Event::Actions(_)) && let Some(info) = listener.borrow_mut().as_mut() {
                    info.handle_event(cx, event, &mut Scope::empty());
                }
            }));
            let mut info = cx.with_vm(|vm| {
                makepad_widgets::script_mod(vm);
                crate::app::register_widgets(vm);
                let value = script_eval!(vm, {mod.widgets.MobileChatInfo {}});
                MobileChatInfo::script_from_value(vm, value)
            });
            info.room = Some(RoomNameId::empty(room.room_id().to_owned()));
            info.owner = current.user_id().map(ToOwned::to_owned);
            info.epoch = crate::account_session::epoch();
            info.origin = current.homeserver().to_string();
            info.fail_load(&mut cx, "offline");
            let verified = |pin| Event::Actions(vec![Box::new(VerificationStateAction {
                pin, state: matrix_sdk::encryption::VerificationState::Verified,
            })]);
            info.handle_event(&mut cx, &verified(old_pin), &mut Scope::empty());
            assert!(info.failed && info.load_deadline.is_none());
            info.handle_event(&mut cx, &verified(SessionPin::capture(&current).unwrap()), &mut Scope::empty());
            assert!(!info.failed && info.load_deadline.is_some());
            *panel.borrow_mut() = Some(info);
            async fn wait_for_info(cx: &mut Cx, panel: &RefCell<Option<MobileChatInfo>>) {
                tokio::time::timeout(INFO_TIMEOUT, async {
                    loop {
                        cx.handle_action_receiver();
                        if panel.borrow().as_ref().unwrap().load_deadline.is_none() { break; }
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                    }
                }).await.expect("the current room must receive a terminal response");
            }
            wait_for_info(&mut cx, &panel).await;
            {
                let mut loaded = panel.borrow_mut();
                let info = loaded.as_mut().unwrap();
                assert!(!info.failed, "{}", info.status);
                assert_eq!(info.data.as_ref().unwrap().room.room_id(), room.room_id());
                let selected = info.room.clone().unwrap();
                info.data.as_mut().unwrap().members.push(member("@stale:example.org", "Stale"));
                let previous = info.request;
                let command = ("agent".to_owned(), "original-key".to_owned());
                info.agent_command = Some(command.clone());
                info.update_room(&mut cx, selected.clone(), false);
                assert_eq!(info.request, previous);
                assert!(info.data.as_ref().unwrap().members.iter().any(|m| m.user_id.as_str() == "@stale:example.org"));
                info.update_room(&mut cx, selected.clone(), true);
                assert!(info.data.is_none() && info.load_deadline.is_some());
                assert_ne!(info.request, previous);
                assert_eq!(info.agent_command, Some(command));
                let refresh = info.request;
                info.update_room(&mut cx, selected, false);
                assert_eq!(info.request, refresh);
            }
            wait_for_info(&mut cx, &panel).await;
            assert!(panel.borrow().as_ref().unwrap().data.as_ref().unwrap().members
                .iter().all(|m| m.user_id.as_str() != "@stale:example.org"));
            install_link_client(None);
        });
    }

    #[test]
    fn returning_to_a_pane_that_missed_its_reply_shows_retry_and_ends_loading() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut info = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            let value = script_eval!(vm, {mod.widgets.MobileChatInfo {}});
            MobileChatInfo::script_from_value(vm, value)
        });
        info.room = Some(RoomNameId::empty(ruma::owned_room_id!("!room:example.org")));
        info.request = 1;
        info.load_deadline = Some(std::time::Instant::now());
        draw_information(&mut info, &mut cx);
        assert!(info.failed && info.load_deadline.is_none());
        assert!(info.view.button(&cx, ids!(retry)).visible());
        assert!(!info.view.label(&cx, ids!(summary_count)).visible());
        assert!(info.status.contains(crate::i18n::tr("The chat information request timed out.")));
    }

    #[test]
    fn one_page_keeps_the_summary_fixed_and_the_original_invitation_intent() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut info=new_panel(&mut cx);
        let room=RoomNameId::new(matrix_sdk::RoomDisplayName::Named("Room A".into()),matrix_sdk::ruma::room_id!("!a:server").to_owned());
        info.room=Some(room.clone());info.data=Some(ChatInfoData{room,topic:"A topic".into(),members:vec![],banned_members:HashSet::new(),removable:HashSet::new(),can_rename:false,mode:None,can_invite:true});
        info.agent_command=Some(("exact-original-input".into(),"original-key".into()));
        draw_information(&mut info,&mut cx);
        assert_eq!(info.view.label(&mut cx,ids!(summary_name)).text(),"Room A");
        assert!(!info.view.modal(&mut cx,ids!(agent_picker_modal)).is_open());
        assert!(!info.view.navigation_bar_button(&mut cx,ids!(history_search)).is_empty());
        let summary=info.view.view(&mut cx,ids!(summary)).area().rect(&cx);
        let page=info.view.portal_list(&mut cx,ids!(list)).area().rect(&cx);
        assert!(page.pos.y>=summary.pos.y+summary.size.y-1.0,"the page follows the fixed summary");
        assert!(page.size.y>200.0,"the summary must leave space for the page");
        // Members, room details and settings are on the one page: no tab hides any of them.
        let rows:Vec<_>=(0..5).map(|index|InfoPage {members:0,can_remove:false,columns:4,adding:false,topic:true,choosing:false}.row(index)).collect();
        assert_eq!(rows,[InfoRow::Members(0),InfoRow::Gap,InfoRow::Name,InfoRow::Topic,InfoRow::Notifications]);
        let mut drawn:Vec<_>=info.rendered_rows.values().copied().collect();
        drawn.sort_by_key(|row|format!("{row:?}"));
        assert_eq!(drawn,[InfoRow::AgentServices,InfoRow::Gap,InfoRow::Members(0),InfoRow::Name,InfoRow::Notifications,InfoRow::Topic]);

        // The tile that adds opens the choice between a person and an Agent, right under the grid.
        info.tap_cell(&mut cx,CellTarget::Add);
        assert!(info.adding);
        draw_information(&mut info,&mut cx);
        let list=info.view.portal_list(&mut cx,ids!(list));
        let choice=list.item(&mut cx,1,id!(AddChoice));
        let invite=choice.button(&mut cx,ids!(invite_people)).area().rect(&cx);
        let agent=choice.button(&mut cx,ids!(invite_my_agent)).area().rect(&cx);
        assert!((invite.pos.y-agent.pos.y).abs()<1.0,"the two ways to add share a compact row");
        assert!(invite.size.y<=32.1 && agent.size.y<=32.1);
        assert!(invite.pos.x+invite.size.x<=agent.pos.x && agent.pos.x+agent.size.x<=280.0);
        let grid=list.item(&mut cx,0,id!(MemberRow)).area().rect(&cx);
        assert!((invite.pos.y-(grid.pos.y+grid.size.y)).abs()<=5.0,"the choice sits right under the grid");
        info.tap_cell(&mut cx,CellTarget::Add);
        assert!(!info.adding,"a second tap puts the choice away");
        // Without the right to invite people there is no choice to show: only an Agent can be added.
        info.data.as_mut().unwrap().can_invite=false;
        info.tap_cell(&mut cx,CellTarget::Add);
        assert!(!info.adding);
        info.data.as_mut().unwrap().can_invite=true;
        // With nobody the viewer may remove, the tile that removes does nothing.
        info.tap_cell(&mut cx,CellTarget::Remove);
        assert!(!info.removing);
        info.data.as_mut().unwrap().removable=HashSet::from([user("@bob:server")]);
        info.tap_cell(&mut cx,CellTarget::Remove);
        assert!(info.removing);
        info.tap_cell(&mut cx,CellTarget::Add);
        assert!(info.adding && !info.removing,"adding and removing are never open together");
        assert_eq!(info.agent_command,Some(("exact-original-input".into(),"original-key".into())),"moving about the page doesn't duplicate or discard the invitation key");

        draw_information(&mut info,&mut cx);
        let members_list = info.view.portal_list(&mut cx, ids!(list)).area().rect(&cx);
        info.agent_picker_open = true;
        info.view.modal(&mut cx, ids!(agent_picker_modal)).open(&mut cx);
        draw_information(&mut info, &mut cx);
        assert!(info.view.modal(&mut cx, ids!(agent_picker_modal)).is_open());
        assert_eq!(info.view.portal_list(&mut cx, ids!(list)).area().rect(&cx), members_list,
            "opening the overlay must not move or shrink the page");
        let dialog = info.view.view(&mut cx, ids!(agent_picker_modal.content)).area().rect(&cx);
        assert!(dialog.size.x > 200.0 && dialog.size.x <= 400.0);
        assert!((dialog.pos.y + dialog.size.y * 0.5 - 775.0 * 0.5).abs() <= 1.0,
            "dialog is centered in the window: {dialog:?}");
        assert!(dialog.pos.y >= 0.0 && dialog.pos.y + dialog.size.y <= 775.0);
        info.close_agent_picker(&mut cx);
        assert_eq!(info.view.view(&mut cx,ids!(summary)).area().rect(&cx),summary);
        cx.with_vm(|vm|assert!(vm.take_errors().is_empty()));

        info.services_open=true;draw_information(&mut info,&mut cx);
        assert!(info.view.view(&mut cx,ids!(services_panel)).visible());
        assert!(!info.view.view(&mut cx,ids!(information_list)).visible());
        assert_eq!(info.view.button(&mut cx,ids!(services_back)).text(),crate::i18n::tr("Back"));
        info.services_open=false;draw_information(&mut info,&mut cx);
        assert!(!info.view.view(&mut cx,ids!(services_panel)).visible());
        assert!(info.view.view(&mut cx,ids!(information_list)).visible());
        info.removing=true;
        info.clear();
        assert!(!info.adding && !info.removing && info.agent_command.is_none());
    }
    #[test]
    fn a_panel_inside_a_rooms_screen_asks_that_screen_for_the_history_and_not_the_app() {
        use super::super::room_history::{HistoryFilter, RoomHistoryAction};
        use crate::shared::navigation_bar_button::NavigationBarButtonAction;
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut info=new_panel(&mut cx);
        let room=RoomNameId::new(matrix_sdk::RoomDisplayName::Named("Room A".into()),matrix_sdk::ruma::room_id!("!a:server").to_owned());
        info.room=Some(room.clone());info.data=Some(ChatInfoData{room,topic:String::new(),members:vec![],banned_members:HashSet::new(),removable:HashSet::new(),can_rename:false,mode:None,can_invite:true});
        #[cfg(feature = "onboarding-test-support")]
        {info.review_mode=true;}
        if !info.pin_is_current() {return;}
        let uid=info.widget_uid();
        for (id,filter) in [(ids!(history_search),HistoryFilter::Messages),(ids!(history_media),HistoryFilter::Media)] {
            let button=info.view.navigation_bar_button(&mut cx,id).widget_uid();
            for in_room in [false,true] {
                info.history_in_room=in_room;
                let click=cx.capture_actions(|cx|cx.widget_action(button,NavigationBarButtonAction::Clicked));
                let out=cx.capture_actions(|cx|info.handle_event(cx,&Event::Actions(click),&mut Scope::empty()));
                let to_app=out.iter().any(|action|matches!(action.downcast_ref(),Some(RoomHistoryAction::Open{filter:actual,..}) if *actual==filter));
                let to_screen=out.iter().filter_map(|action|action.as_widget_action()).any(|action|action.widget_uid==uid
                    && matches!(action.action.downcast_ref(),Some(RoomHistoryAction::Open{filter:actual,..}) if *actual==filter));
                assert_eq!((to_app,to_screen),(!in_room,in_room),"{filter:?} in a room's screen: {in_room}");
            }
        }
    }
    #[test]
    fn the_room_is_shown_by_its_name_which_only_those_allowed_can_change() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut info=new_panel(&mut cx);
        let room_id=matrix_sdk::ruma::room_id!("!a:server").to_owned();
        let room=RoomNameId::new(matrix_sdk::RoomDisplayName::Named("Room A".into()),room_id.clone());
        info.room=Some(room.clone());info.data=Some(ChatInfoData{room,topic:String::new(),members:vec![],banned_members:HashSet::new(),removable:HashSet::new(),can_rename:false,mode:None,can_invite:true});
        draw_information(&mut info,&mut cx);
        let list=info.view.portal_list(&mut cx,ids!(list));
        // Under the grid's one row and the gap: the room by its name, not by its Matrix ID.
        let name=list.item(&mut cx,2,id!(Action));
        assert_eq!(name.label(&mut cx,ids!(title)).text(),"Room name");
        assert_eq!(name.label(&mut cx,ids!(value)).text(),"Room A");
        assert!(!name.view(&mut cx,ids!(chevron)).visible(),"no arrow for someone who can't change the name");
        assert!(!info.rendered_rows.values().any(|row|*row==InfoRow::Topic),"a room without a topic has no topic row");
        info.tap_name(&mut cx);
        assert!(!info.renaming,"a tap only says who can change the name");

        info.data.as_mut().unwrap().can_rename=true;
        draw_information(&mut info,&mut cx);
        assert!(list.item(&mut cx,2,id!(Action)).view(&mut cx,ids!(chevron)).visible());
        info.tap_name(&mut cx);
        assert!(info.renaming);
        assert_eq!(info.view.text_input(&mut cx,ids!(rename_input)).text(),"Room A","the field starts from the present name");
        draw_information(&mut info,&mut cx);
        assert!(info.view.view(&mut cx,ids!(rename_panel)).visible());
        assert!(!info.view.view(&mut cx,ids!(information_list)).visible());
        // A name of spaces is not sent.
        info.view.text_input(&mut cx,ids!(rename_input)).set_text(&mut cx,"   ");
        info.save_name(&mut cx);
        assert!(info.renaming && !info.rename_busy);
        assert_eq!(info.view.label(&mut cx,ids!(rename_status)).text(),"Enter a room name.");
        // The present name closes the field without a request.
        info.view.text_input(&mut cx,ids!(rename_input)).set_text(&mut cx," Room A ");
        info.save_name(&mut cx);
        assert!(!info.renaming && !info.rename_busy && info.rename_request==0);

        // Only the answer to this panel's own request ends a change.
        info.owner=Some(user("@owner:server"));info.origin="https://server/".into();info.epoch=4;
        info.tap_name(&mut cx);info.rename_request=8;info.rename_busy=true;
        let mut renamed=RoomRenamed{owner:user("@owner:server"),origin:"https://server/".into(),epoch:4,room_id:room_id.clone(),request:8,result:Err("not allowed".into())};
        assert!(info.rename_response_matches(&renamed));
        renamed.request=7;assert!(!info.rename_response_matches(&renamed));renamed.request=8;
        renamed.epoch=3;assert!(!info.rename_response_matches(&renamed));renamed.epoch=4;
        renamed.origin="https://other/".into();assert!(!info.rename_response_matches(&renamed));renamed.origin="https://server/".into();
        renamed.room_id=matrix_sdk::ruma::room_id!("!other:server").to_owned();assert!(!info.rename_response_matches(&renamed));renamed.room_id=room_id;
        renamed.owner=user("@other:server");assert!(!info.rename_response_matches(&renamed));renamed.owner=user("@owner:server");
        info.finish_rename(&mut cx,&renamed);
        assert!(info.renaming && !info.rename_busy,"a refused change keeps the field open");
        assert!(info.view.label(&mut cx,ids!(rename_status)).text().contains("not allowed"));
        renamed.result=Ok("Room B".into());
        info.finish_rename(&mut cx,&renamed);
        assert!(!info.renaming);
        assert_eq!(info.data.as_ref().unwrap().room.display(),"Room B");
        assert_eq!(info.room.as_ref().unwrap().display(),"Room B");
        // A panel of a signed-in account draws only while that sign-in is the current one.
        info.owner=None;
        draw_information(&mut info,&mut cx);
        assert_eq!(list.item(&mut cx,2,id!(Action)).label(&mut cx,ids!(value)).text(),"Room B","the page shows the new name at once");
        assert_eq!(shown_name("一二三四五六"),"一二三四五六");
        assert_eq!(shown_name("一二三四五六七"),"一二三四五…");
        assert_eq!(shown_name("Project room"),"Project room");
        assert_eq!(shown_name("Project rooms"),"Project roo…");
        assert_eq!(shown_name("A一二三四五六"),"A一二三四五…");
        cx.with_vm(|vm|assert!(vm.take_errors().is_empty()));
    }
    #[test]
    fn room_agent_picker_results_cannot_cross_room_account_or_original_call() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut info=cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);vm.bx.captured_errors=Some(Vec::new());crate::app::register_widgets(vm);
            let value=script_eval!(vm,{mod.widgets.MobileChatInfo {}});
            let info=MobileChatInfo::script_from_value(vm,value);assert!(vm.take_errors().is_empty());info
        });
        let owner=matrix_sdk::ruma::user_id!("@owner:server").to_owned();
        let room=matrix_sdk::ruma::room_id!("!target:server").to_owned();
        info.owner=Some(owner.clone());info.origin="https://server/".into();info.epoch=4;info.room=Some(RoomNameId::empty(room.clone()));info.agent_request=8;
        let mut result=RoomAgentsLoaded {owner,origin:"https://server/".into(),epoch:4,room_id:room,request:8,binding:false,result:Ok(serde_json::json!({"roomId":"!target:server","projectId":"project"}))};
        assert!(info.agent_response_matches(&result));
        result.result=Ok(serde_json::json!({"roomId":"!other:server"}));assert!(!info.agent_response_matches(&result));
        result.binding=true;result.result=Ok(serde_json::json!({"creation":{"binding":{"roomId":"!target:server"}}}));assert!(info.agent_response_matches(&result));
        result.result=Ok(serde_json::json!({"creation":{"binding":{"roomId":"!other:server"}}}));assert!(!info.agent_response_matches(&result));
        assert!(info.agent_call_matches(&result));
        info.agent_busy=true;
        info.finish_agent_request(&mut cx,&result);
        assert!(!info.agent_busy, "a malformed completion must release its exact call");
        assert!(!info.agent_added);
        assert!(info.view.label(&mut cx,ids!(agent_status)).text().contains("invalid binding result"));
        result.result=Ok(serde_json::json!({"commandState":"pending","creation":{"binding":{"roomId":"!target:server","state":"pending"}}}));
        info.finish_agent_request(&mut cx,&result);
        assert!(info.agent_pending && !info.agent_added && !info.agent_busy);
        result.result=Ok(serde_json::json!({"commandState":"active","creation":{"binding":{"roomId":"!target:server","state":"active"}}}));
        info.finish_agent_request(&mut cx,&result);
        assert!(info.agent_added && !info.agent_pending && !info.agent_busy);
        result.result=Err("agent_service_invite_permission_required".into());
        info.finish_agent_request(&mut cx,&result);
        assert!(!info.agent_added && !info.agent_busy);
        assert!(info.view.label(&mut cx,ids!(agent_status)).text().contains("permission to invite"));
        result.result=Err("offline".into());assert!(info.agent_response_matches(&result));
        result.epoch=3;assert!(!info.agent_response_matches(&result));result.epoch=4;
        result.origin="https://other/".into();assert!(!info.agent_response_matches(&result));result.origin="https://server/".into();
        result.request=7;assert!(!info.agent_response_matches(&result));result.request=8;
        result.owner=matrix_sdk::ruma::user_id!("@other:server").to_owned();assert!(!info.agent_response_matches(&result));
        info.clear();assert!(!info.agent_response_matches(&result));
    }
    #[test]
    fn late_members_cannot_cross_room_server_or_account_epoch() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut info = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            let value = script_eval!(vm, {mod.widgets.MobileChatInfo {}});
            MobileChatInfo::script_from_value(vm, value)
        });
        let owner = matrix_sdk::ruma::user_id!("@alice:server").to_owned();
        let room_a = matrix_sdk::ruma::room_id!("!a:server").to_owned();
        let room_b = matrix_sdk::ruma::room_id!("!b:server").to_owned();
        info.owner = Some(owner.clone());
        info.origin = "https://a.test/".into();
        info.epoch = 11;
        info.request = 10;
        info.room = Some(RoomNameId::empty(room_a.clone()));
        let mut result = ChatInfoLoaded {
            owner,
            origin: "https://a.test/".into(),
            epoch: 11,
            room_id: room_a.clone(),
            request: 10,
            result: Ok(ChatInfoData {
                can_invite: true,
                room: RoomNameId::empty(room_a),
                topic: String::new(),
                members: vec![], banned_members: HashSet::new(), removable: HashSet::new(), can_rename: false,
                mode: None,
            }),
        };
        assert!(info.matches_response(&result));
        info.room = Some(RoomNameId::empty(room_b.clone()));
        assert!(
            !info.matches_response(&result),
            "same request cannot fill another Room"
        );
        result.room_id = room_b.clone();
        assert!(
            !info.matches_response(&result),
            "payload must agree with requested Room"
        );
        result.result = Err("offline".into());
        assert!(info.matches_response(&result));
        info.epoch = 12;
        assert!(!info.matches_response(&result));
        info.epoch = 11;
        info.origin = "https://b.test/".into();
        assert!(!info.matches_response(&result));
        info.origin = "https://a.test/".into();
        info.clear();
        assert!(!info.matches_response(&result));
    }
}
