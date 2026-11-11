//! A modal dialog for inviting one or more people or Agents to a room.

use makepad_widgets::*;
use ruma::OwnedUserId;
use std::collections::HashMap;
use crate::shared::navigation_bar_button::NavigationBarButtonWidgetRefExt;

use crate::home::invite_selection::{InviteSelection, Invitee, Outcome, is_already_member_error};
use crate::home::room_screen::InviteResultAction;
use crate::sliding_sync::{MatrixRequest, get_client, spawn_async_task, submit_async_request};
use crate::utils::RoomNameId;


script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    mod.widgets.InviteModal = set_type_default() do #(InviteModal::register_widget(vm)) {
        ..mod.widgets.SmallModal

        title := ModalTitle {}

        user_id_input := RobrixTextInput {
            width: Fill
            height: 44
            draw_text +: {
                text_style: REGULAR_TEXT {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)},
                color: mod.widgets.RINX_INK
            }
            empty_text: #(crate::i18n::tr("Name or @user:example.org")) i18n_empty_text: "Name or @user:example.org",
            autocapitalize: None,
            autocorrect: Disabled,
        }

        selected_view := View {
            visible: false
            width: Fill height: Fit flow: Down spacing: 8 margin: Inset{top: 12}
            Label {width: Fill height: Fit
                text: #(crate::i18n::tr("Selected")) i18n_text: "Selected"
                draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}
            }
            chips := FlatList {
                width: Fill height: Fit
                flow: Flow.Right{wrap: true} spacing: 6
                drag_scrolling: false
                scroll_bars: ScrollBars {show_scroll_x: false show_scroll_y: false}
                Chip := NavigationBarButton {
                    width: Fit height: Fit{min: FitBound.Abs(32.0)} flow: Right spacing: 6
                    margin: Inset{bottom: 6}
                    padding: Inset{left: 12 right: 10 top: 6 bottom: 6}
                    align: Align{x: 0.0 y: 0.5}
                    tooltip_below: true
                    draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_SELECTED border_color: mod.widgets.RINX_BORDER border_size: 1 border_radius: 16}
                    chip_name := Label {width: Fit{max: FitBound.Abs(180.0)} height: Fit padding: 0 max_lines: 1 text_overflow: Ellipsis
                        draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}
                    }
                    chip_done := Icon {visible: false icon_walk: Walk{width: 12 height: 12} draw_icon +: {svg: ICON_CHECKMARK color: mod.widgets.RINX_SUCCESS_FG}}
                    chip_failed := Icon {visible: false icon_walk: Walk{width: 12 height: 12} draw_icon +: {svg: ICON_WARNING color: mod.widgets.RINX_DANGER_FG}}
                    chip_remove := Icon {icon_walk: Walk{width: 10 height: 10} draw_icon +: {svg: ICON_CLOSE color: mod.widgets.RINX_MUTED}}
                }
            }
        }

        people_section := View {
            width: Fill height: Fit flow: Down spacing: 8 margin: Inset{top: 12}
            Label {width: Fill height: Fit
                text: #(crate::i18n::tr("People you know")) i18n_text: "People you know"
                draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}
            }
            people_list := PortalList {
                width: Fill height: 176 flow: Down auto_tail: false
                Empty := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} padding: 10
                    text: #(crate::i18n::tr("Search by name or enter a Matrix ID.")) i18n_text: "Search by name or enter a Matrix ID."
                    draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}
                }
                Person := NavigationBarButton {
                    width: Fill height: Fit{min: FitBound.Abs(56.0)} flow: Right spacing: 10
                    padding: Inset{left: 10 right: 14 top: 8 bottom: 8}
                    align: Align{x: 0.0 y: 0.5}
                    draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_SELECTED border_radius: 8}
                    check := RoundedView {width: 20 height: 20 draw_bg +: {color: #0000 border_color: mod.widgets.RINX_BORDER border_size: 1 border_radius: 5}
                        mark := RoundedView {visible: false width: Fill height: Fill align: Align{x: 0.5 y: 0.5} draw_bg +: {color: mod.widgets.RINX_ACCENT border_radius: 5}
                            Icon {icon_walk: Walk{width: 12 height: 12} draw_icon +: {svg: ICON_CHECKMARK color: mod.widgets.RINX_ON_ACCENT}}
                        }
                    }
                    View {width: Fill height: Fit flow: Down spacing: 3
                        person_name := Label {width: Fill height: Fit padding: 0 max_lines: 1 text_overflow: Ellipsis
                            draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: (12 * mod.widgets.RINX_TEXT_SCALE)}}
                        }
                        person_id := Label {width: Fill height: Fit padding: 0 max_lines: 1 text_overflow: Ellipsis
                            draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (9 * mod.widgets.RINX_TEXT_SCALE)}}
                        }
                    }
                }
            }
        }

        results_view := View {
            visible: false
            width: Fill height: Fit flow: Down margin: Inset{top: 12}
            results := FlatList {
                width: Fill height: Fit flow: Down spacing: 8
                drag_scrolling: false
                scroll_bars: ScrollBars {show_scroll_x: false show_scroll_y: false}
                Result := View {
                    width: Fill height: Fit flow: Down spacing: 2
                    result_name := Label {width: Fill height: Fit padding: 0 max_lines: 1 text_overflow: Ellipsis
                        draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_bold {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}
                    }
                    result_info := Label {width: Fill height: Fit padding: 0 flow: Flow.Right{wrap: true}
                        draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (10.5 * mod.widgets.RINX_TEXT_SCALE)}}
                    }
                    result_ok := Label {visible: false width: Fill height: Fit padding: 0 flow: Flow.Right{wrap: true}
                        draw_text +: {color: mod.widgets.RINX_SUCCESS_FG text_style: theme.font_regular {font_size: (10.5 * mod.widgets.RINX_TEXT_SCALE)}}
                    }
                    result_error := Label {visible: false width: Fill height: Fit padding: 0 flow: Flow.Right{wrap: true}
                        draw_text +: {color: mod.widgets.RINX_DANGER_FG text_style: theme.font_regular {font_size: (10.5 * mod.widgets.RINX_TEXT_SCALE)}}
                    }
                }
            }
        }

        buttons_view := ModalButtonsRow {
            cancel_button := RobrixNeutralIconButton {
                height: 44
                width: Fit{min: FitBound.Abs(120.0)},
                align: Align{x: 0.5, y: 0.5}
                padding: 12,
                draw_icon.svg: (ICON_FORBIDDEN)
                icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                text: #(crate::i18n::tr("Cancel")) i18n_text: "Cancel"
            }

            confirm_button := RobrixPositiveIconButton {
                height: 44
                width: Fit{min: FitBound.Abs(120.0)}
                align: Align{x: 0.5, y: 0.5}
                padding: 12,
                draw_icon.svg: (ICON_ADD_USER)
                icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                text: #(crate::i18n::tr("Invite")) i18n_text: "Invite"
            }

            okay_button := RobrixIconButton {
                height: 44
                visible: false
                width: Fit{min: FitBound.Abs(120.0)}
                align: Align{x: 0.5, y: 0.5}
                padding: 12,
                draw_icon.svg: (ICON_CHECKMARK)
                icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                text: #(crate::i18n::tr("Okay")) i18n_text: "Okay"
            }
        }

        status_label_view := View {
            visible: false
            width: Fill,
            height: Fit,
            align: Align{x: 0.5, y: 0.0}

            status_label := Label {
                width: Fill,
                height: Fit,
                flow: Flow.Right{wrap: true},
                align: Align{x: 0.5, y: 0.0}
                margin: Inset{top: 10}
                draw_text +: {
                    text_style: REGULAR_TEXT {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)},
                    color: mod.widgets.RINX_INK
                }
                text: ""
            }
        }
    }
}

/// Actions emitted by other widgets to show or hide the `InviteModal`.
#[derive(Clone, Debug)]
pub enum InviteModalAction {
    /// Open the modal to invite people to the given room or space.
    Open(RoomNameId),
    /// Close the modal.
    Close,
}

/// People the invite dialog can offer, by Matrix ID and display name.
type Person = (OwnedUserId, Option<String>);

/// How a message reads: in progress or neutral, a success, or a problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    Info,
    Success,
    Danger,
}

/// What one invitee's result says, and how it reads.
fn outcome_text(invitee: &Invitee) -> (String, Tone) {
    let agent = crate::hagency::backend::is_agent_invitation(&invitee.id);
    match &invitee.outcome {
        Outcome::Selected => (String::new(), Tone::Info),
        Outcome::Sending if agent => (crate::i18n::tr("Adding Agent to this room…").to_owned(), Tone::Info),
        Outcome::Sending => (crate::i18n::tr("Sending invite...").to_owned(), Tone::Info),
        Outcome::Invited => (crate::i18n::tr("Invitation sent. Waiting for them to join.").to_owned(), Tone::Success),
        Outcome::Joined => (crate::i18n::tr("Agent joined this room. In group chats, @ the Agent to ask it to reply.").to_owned(), Tone::Success),
        Outcome::AgentPending => (crate::i18n::tr("The request is saved. The Agent is still joining. You can close this panel or check its status.").to_owned(), Tone::Info),
        Outcome::AlreadyMember => (crate::i18n::tr("Already a member of this room.").to_owned(), Tone::Success),
        Outcome::AlreadyInvited => (crate::i18n::tr("Already invited. Waiting for them to join.").to_owned(), Tone::Success),
        Outcome::Failed(reason) => (reason.clone(), Tone::Danger),
    }
}

/// The dialog's message once every invitation in flight has an answer. A
/// single invitee reads exactly as its own result; several are summarized,
/// with each one's result listed above.
fn summary_text(selection: &InviteSelection) -> (String, Tone) {
    let attempted: Vec<&Invitee> = selection.attempted().collect();
    if let [only] = attempted.as_slice() {
        return outcome_text(only);
    }
    let (done, failed, pending) = selection.tally();
    let total = done + failed + pending;
    if failed > 0 {
        (crate::i18n::format(
            "{failed} of {total} invitations failed. They stay selected so you can try again.",
            &[("failed", failed.to_string()), ("total", total.to_string())],
        ), Tone::Danger)
    } else if pending > 0 {
        (crate::i18n::tr("Some Agents are still joining. You can close this panel or check their status.").to_owned(), Tone::Info)
    } else {
        (crate::i18n::format("{0} invitations completed.", &[("0", total.to_string())]), Tone::Success)
    }
}

/// The confirm button's label for `count` invitees.
fn confirm_text(count: usize, only_pending_agents: bool) -> String {
    match count {
        0 => crate::i18n::tr("Invite").to_owned(),
        _ if only_pending_agents => crate::i18n::tr("Check status").to_owned(),
        1 => crate::i18n::tr("Invite 1 person").to_owned(),
        _ => crate::i18n::format("Invite {0} people", &[("0", count.to_string())]),
    }
}

/// Results of the dialog's background lookups, matched to the request that
/// asked for them so a late answer never overwrites a newer one.
#[derive(Debug)]
enum InvitePeopleAction {
    /// Everyone who shares a joined room with the user, and who is already
    /// joined to or invited into the target room.
    Known { request: u64, people: Vec<Person>, joined: Vec<OwnedUserId>, invited: Vec<OwnedUserId> },
    /// The homeserver directory's answer for a typed name.
    Searched { request: u64, query: String, people: Vec<Person> },
}

#[derive(Script, ScriptHook, Widget)]
pub struct InviteModal {
    #[deref] view: View,
    #[rust] room_name_id: Option<RoomNameId>,
    #[rust] offered: Vec<Person>,
    /// Pin clicks to the identity actually drawn, even if a lookup replaces the list.
    #[rust] rendered_people: HashMap<WidgetUid, OwnedUserId>,
    /// The drawn rows by list index, for moving between them with arrow keys.
    #[rust] rendered_rows: Vec<(usize, WidgetRef)>,
    /// A row to focus once it is drawn, after an arrow key scrolled to it.
    #[rust] pending_focus: Option<usize>,
    #[rust] rendered_chips: HashMap<WidgetUid, OwnedUserId>,
    /// Everyone chosen, and each one's result.
    #[rust] selection: InviteSelection,
    /// Already joined to the room; never invited again.
    #[rust] joined: Vec<OwnedUserId>,
    /// Already invited and not yet joined; never invited again.
    #[rust] invited: Vec<OwnedUserId>,
    /// Registered agents and people who share a room with the user.
    #[rust] known: Vec<Person>,
    #[rust] request: u64,
    /// The request of the member scan started when the dialog opened. A name
    /// search in the meantime must not discard its answer.
    #[rust] known_request: u64,
    #[rust] epoch: u64,
    #[rust] can_invite: bool,
    #[cfg(feature = "onboarding-test-support")]
    #[rust] review_mode: bool,
}

impl Widget for InviteModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if let Event::KeyDown(key) = event
            && matches!(key.key_code, KeyCode::ArrowDown | KeyCode::ArrowUp)
            && !key.modifiers.is_primary()
        {
            self.move_focus(cx, key.key_code == KeyCode::ArrowDown);
        }
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
        if matches!(event, Event::Actions(_)) {
            self.update_invite_button(cx);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.rendered_people.clear();
        self.rendered_rows.clear();
        self.rendered_chips.clear();
        let chips = self.view.flat_list(cx, ids!(chips)).widget_uid();
        let results = self.view.flat_list(cx, ids!(results)).widget_uid();
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.borrow_mut::<PortalList>() {
                list.set_item_range(cx, 0, self.offered.len().max(1));
                while let Some(index) = list.next_visible_item(cx) {
                    if self.offered.is_empty() {
                        list.item(cx, index, id!(Empty)).draw_all(cx, scope);
                        continue;
                    }
                    let Some((id, name)) = self.offered.get(index) else {continue;};
                    let row = list.item(cx, index, id!(Person));
                    row.label(cx, ids!(person_name)).set_text(cx, name.as_deref().filter(|n| !n.is_empty()).unwrap_or(id.localpart()));
                    row.label(cx, ids!(person_id)).set_text(cx, id.as_str());
                    let checked = self.selection.contains(id);
                    row.as_navigation_bar_button().set_selected(cx, checked);
                    row.view(cx, ids!(mark)).set_visible(cx, checked);
                    self.rendered_people.insert(row.widget_uid(), id.clone());
                    row.draw_all(cx, scope);
                    if self.pending_focus == Some(index) {
                        self.pending_focus = None;
                        row.set_key_focus(cx);
                    }
                    self.rendered_rows.push((index, row));
                }
                continue;
            }
            let uid = item.widget_uid();
            let list_ref = item.as_flat_list();
            let Some(mut list) = list_ref.borrow_mut() else { continue };
            if uid == chips {
                for invitee in self.selection.entries() {
                    let Some(chip) = list.item(cx, LiveId::from_str(invitee.id.as_str()), id!(Chip)) else { continue };
                    chip.label(cx, ids!(chip_name)).set_text(cx, invitee.label());
                    chip.widget(cx, ids!(chip_done)).set_visible(cx, invitee.outcome.is_done());
                    chip.widget(cx, ids!(chip_failed)).set_visible(cx, matches!(invitee.outcome, Outcome::Failed(_)));
                    chip.widget(cx, ids!(chip_remove)).set_visible(cx, invitee.outcome.is_removable());
                    let button = chip.as_navigation_bar_button();
                    button.set_selected(cx, true);
                    if let Some(mut button) = button.borrow_mut() {
                        let (result, _) = outcome_text(invitee);
                        button.set_tooltip_text(if result.is_empty() {
                            invitee.id.to_string()
                        } else {
                            format!("{}\n{result}", invitee.id)
                        });
                    }
                    self.rendered_chips.insert(chip.widget_uid(), invitee.id.clone());
                    chip.draw_all(cx, scope);
                }
            } else if uid == results {
                for invitee in self.selection.attempted() {
                    let Some(row) = list.item(cx, LiveId::from_str(invitee.id.as_str()), id!(Result)) else { continue };
                    row.label(cx, ids!(result_name)).set_text(cx, invitee.label());
                    let (text, tone) = outcome_text(invitee);
                    for (path, shown) in [(ids!(result_info), Tone::Info), (ids!(result_ok), Tone::Success), (ids!(result_error), Tone::Danger)] {
                        let label = row.label(cx, path);
                        label.set_visible(cx, tone == shown);
                        if tone == shown {
                            label.set_text(cx, &text);
                        }
                    }
                    row.draw_all(cx, scope);
                }
            }
            list.items.retain_visible();
        }
        DrawStep::done()
    }
}

impl WidgetMatchEvent for InviteModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        if self.epoch != crate::account_session::epoch() { return; }
        for action in actions {
            if let Some(update) = action.downcast_ref::<crate::sliding_sync::UserPowerLevelsUpdated>()
                && self.room_name_id.as_ref().is_some_and(|room| update.is_current_for(room.room_id()))
            {
                self.can_invite = update.power_levels.can_invite();
            }
            match action.downcast_ref() {
                Some(InvitePeopleAction::Known { request, people, joined, invited }) if *request == self.known_request => {
                    self.joined = joined.clone();
                    self.invited = invited.clone();
                    for person in people {
                        if !self.known.iter().any(|(id, _)| id == &person.0) {
                            self.known.push(person.clone());
                        }
                    }
                    self.known.sort_by_key(|(id, name)| (name.as_deref().unwrap_or(id.as_str()).to_lowercase(), id.clone()));
                    let query = self.view.text_input(cx, ids!(user_id_input)).text();
                    let matches = self.local_matches(query.trim());
                    self.offer(cx, &matches);
                }
                Some(InvitePeopleAction::Searched { request, query, people }) if *request == self.request
                    && self.view.text_input(cx, ids!(user_id_input)).text().trim() == query => {
                    let mut matches = self.local_matches(query);
                    for person in people {
                        if !matches.iter().any(|(id, _)| id == &person.0) {
                            matches.push(person.clone());
                        }
                    }
                    self.resolved(cx, query, matches);
                }
                _ => {}
            }
        }
        let input = self.view.text_input(cx, ids!(user_id_input));
        if let Some(query) = input.changed(actions) {
            // A directory reply for the old text is discarded. Its
            // progress message must not remain after the person edits.
            self.view.view(cx, ids!(status_label_view)).set_visible(cx, false);
            let matches = self.local_matches(query.trim());
            self.offer(cx, &matches);
        }
        let list = self.view.portal_list(cx, ids!(people_list));
        for (_, row) in list.items_with_actions(actions) {
            if !list.was_scrolling() && row.as_navigation_bar_button().clicked(actions)
                && let Some(user) = self.rendered_people.get(&row.widget_uid()).cloned() {
                let name = self.name_of(&user);
                self.selection.toggle(user, name);
                self.selection_changed(cx);
                break;
            }
        }
        let chips = self.view.flat_list(cx, ids!(chips));
        for (_, chip) in chips.items_with_actions(actions) {
            if chip.as_navigation_bar_button().clicked(actions)
                && let Some(user) = self.rendered_chips.get(&chip.widget_uid()).cloned() {
                if self.selection.remove(&user) {
                    self.selection_changed(cx);
                }
                break;
            }
        }
        let cancel_button = self.view.button(cx, ids!(cancel_button));

        // Handle canceling/closing the modal.
        let cancel_clicked = cancel_button.clicked(actions);
        if cancel_clicked ||
            actions.iter().any(|a| matches!(a.downcast_ref(), Some(ModalAction::Dismissed)))
        {
            // If the modal was dismissed by clicking outside of it, we MUST NOT emit
            // a `InviteModalAction::Close` action, as that would cause
            // an infinite action feedback loop.
            if cancel_clicked {
                cx.action(InviteModalAction::Close);
            }
            return;
        }

        // Handle the okay button (shown once everyone is invited).
        let okay_button = self.view.button(cx, ids!(okay_button));
        if okay_button.clicked(actions) {
            cx.action(InviteModalAction::Close);
            return;
        }

        // The return key adds what was typed; the button also sends.
        if self.view.button(cx, ids!(confirm_button)).clicked(actions) {
            self.submit(cx, false);
        } else if input.returned(actions).is_some() {
            self.submit(cx, true);
        }

        // Handle the results of previously-sent invites.
        for action in actions {
            if let Some(result) = action.downcast_ref::<InviteResultAction>() {
                self.finish_invite(cx, result);
            }
        }
    }
}

impl InviteModal {
    /// Adds a typed Matrix ID or looks up a typed name. Pressing Return with
    /// nothing typed, or the button, sends everyone selected; the button
    /// also sends a typed Matrix ID with them.
    fn submit(&mut self, cx: &mut Cx, from_return: bool) {
        if self.selection.is_sending() { return; }
        let input = self.view.text_input(cx, ids!(user_id_input));
        let text = input.text();
        let text = text.trim();
        if text.starts_with('@') {
            // A full Matrix ID is invited as typed.
            let Ok(user_id) = ruma::UserId::parse(text) else {
                self.status(cx, crate::i18n::tr("Invalid User ID. Expected format: @user:server.xyz"), Tone::Danger);
                input.set_key_focus(cx);
                return;
            };
            let name = self.name_of(&user_id);
            self.selection.add(user_id.clone(), name);
            self.clear_query(cx);
            if from_return {
                let label = self.selection.get(&user_id).map_or_else(|| user_id.to_string(), |i| i.label().to_owned());
                self.status(cx, &crate::i18n::format("Added {0}. Press Invite to send.", &[("0", label)]), Tone::Info);
                self.refresh(cx);
                return;
            }
        } else if !text.is_empty() {
            // Anything else is a name. It is looked up first, never skipped.
            self.search(cx, text.to_owned());
            return;
        }
        if self.selection.sendable_count() == 0 {
            self.status(cx, crate::i18n::tr("Enter a name or Matrix ID."), Tone::Danger);
            return;
        }
        self.send(cx);
    }

    /// Sends one invitation for each selected invitee that has not
    /// succeeded yet. Each keeps its own permission check and request, so
    /// Agents still go through the Hagency service and its bridge invites.
    fn send(&mut self, cx: &mut Cx) {
        let Some(room_id) = self.room_name_id.as_ref().map(|room| room.room_id().clone()) else { return };
        #[cfg(feature = "onboarding-test-support")]
        let offline_review = self.review_mode;
        #[cfg(not(feature = "onboarding-test-support"))]
        let offline_review = false;
        for user_id in self.selection.begin_send() {
            if self.joined.contains(&user_id) {
                self.selection.record(&user_id, Outcome::AlreadyMember);
            } else if self.invited.contains(&user_id) {
                self.selection.record(&user_id, Outcome::AlreadyInvited);
            } else if !self.may_invite_user(&user_id) {
                let reason = crate::i18n::tr("You do not have permission to invite people to this room.").to_owned();
                self.selection.record(&user_id, Outcome::Failed(reason));
            } else if !offline_review {
                submit_async_request(MatrixRequest::InviteUser { room_id: room_id.clone(), user_id });
            }
        }
        let sending: Vec<&Invitee> = self.selection.entries().iter().filter(|i| i.outcome == Outcome::Sending).collect();
        match sending.as_slice() {
            [] => {
                let (text, tone) = summary_text(&self.selection);
                self.status(cx, &text, tone);
            }
            [only] => {
                let (text, _) = outcome_text(only);
                self.status(cx, &text, Tone::Info);
            }
            several => {
                let text = crate::i18n::format("Sending {0} invitations…", &[("0", several.len().to_string())]);
                self.status(cx, &text, Tone::Info);
            }
        }
        self.refresh(cx);
    }

    fn finish_invite(&mut self, cx: &mut Cx, result: &InviteResultAction) {
        let (room, user, epoch) = match result {
            InviteResultAction::Agent {room_id,user_id,epoch,..} | InviteResultAction::Waiting {room_id,user_id,epoch} => (room_id,user_id,*epoch),
            InviteResultAction::Sent {room_id,user_id} | InviteResultAction::Failed {room_id,user_id,..} => (room_id,user_id,self.epoch),
        };
        if self.epoch != crate::account_session::epoch() || epoch != self.epoch
            || self.room_name_id.as_ref().is_none_or(|r| r.room_id() != room)
            || !self.selection.is_sending_to(user) { return; }
        let outcome = match result {
            InviteResultAction::Waiting {..} => {
                // In a batch, "Sending N invitations…" stays until each has an answer.
                let in_flight = self.selection.entries().iter().filter(|i| i.outcome == Outcome::Sending).count();
                if in_flight == 1 {
                    self.status(cx, crate::i18n::tr("Still waiting for the server. You can close this panel; the original request will continue."), Tone::Info);
                }
                return;
            }
            InviteResultAction::Agent {result: Ok(true), ..} => Outcome::Joined,
            InviteResultAction::Agent {result: Ok(false), ..} => Outcome::AgentPending,
            InviteResultAction::Agent {result: Err(code), ..} => Outcome::Failed(crate::i18n::tr(super::mobile_chat_info::agent_binding_error(code)).to_owned()),
            InviteResultAction::Sent {..} => Outcome::Invited,
            InviteResultAction::Failed {error, ..} => {
                let error = error.to_string();
                if is_already_member_error(&error) {
                    Outcome::AlreadyMember
                } else {
                    Outcome::Failed(crate::i18n::format("Failed to send invite: {error}", &[("error", error)]))
                }
            }
        };
        let user = user.clone();
        self.selection.record(&user, outcome);
        if !self.selection.is_sending() {
            let (text, tone) = summary_text(&self.selection);
            self.status(cx, &text, tone);
        }
        self.refresh(cx);
    }

    /// After a row or chip changes the selection, a summary of earlier
    /// results must describe the invitees that remain.
    fn selection_changed(&mut self, cx: &mut Cx) {
        if !self.selection.is_sending() && self.selection.attempted().next().is_some() {
            let (text, tone) = summary_text(&self.selection);
            self.status(cx, &text, tone);
        }
        self.refresh(cx);
    }

    /// Shows the selection, results and buttons for the current state.
    fn refresh(&mut self, cx: &mut Cx) {
        let done = self.selection.all_done();
        let sending = self.selection.is_sending();
        self.view.button(cx, ids!(confirm_button)).set_visible(cx, !done);
        self.view.button(cx, ids!(cancel_button)).set_visible(cx, !done);
        self.view.button(cx, ids!(okay_button)).set_visible(cx, done);
        self.view.text_input(cx, ids!(user_id_input)).set_is_read_only(cx, sending);
        self.view.view(cx, ids!(selected_view)).set_visible(cx, !self.selection.is_empty());
        self.view.view(cx, ids!(results_view)).set_visible(cx, self.selection.attempted().count() > 1);
        self.update_invite_button(cx);
        self.view.redraw(cx);
    }

    /// Moves key focus between the search field and the rows below it.
    fn move_focus(&mut self, cx: &mut Cx, down: bool) {
        let input = self.view.text_input(cx, ids!(user_id_input));
        let current = if input.key_focus(cx) {
            None
        } else if let Some((index, _)) = self.rendered_rows.iter().find(|(_, row)| row.key_focus(cx)) {
            Some(*index)
        } else {
            return; // Focus is outside this dialog's list.
        };
        if self.offered.is_empty() { return; }
        let last = self.offered.len() - 1;
        let next = match (current, down) {
            (None, true) => Some(0),
            (None, false) => return,
            (Some(0), false) => None,
            (Some(index), false) => Some(index - 1),
            (Some(index), true) => Some((index + 1).min(last)),
        };
        let Some(next) = next else {
            input.set_key_focus(cx);
            return;
        };
        let list = self.view.portal_list(cx, ids!(people_list));
        list.keep_index_visible(cx, next);
        match self.rendered_rows.iter().find(|(index, _)| *index == next) {
            Some((_, row)) => row.set_key_focus(cx),
            None => self.pending_focus = Some(next),
        }
        self.view.redraw(cx);
    }

    /// Keep results inside a bounded, scrollable part of the dialog. An overlay
    /// menu cannot fit a large directory or long federated IDs on compact screens.
    fn offer(&mut self, cx: &mut Cx, people: &[Person]) {
        if self.offered == people {return;}
        self.offered = people.to_vec();
        self.rendered_people.clear();
        self.rendered_rows.clear();
        self.pending_focus = None;
        self.view.portal_list(cx, ids!(people_list)).set_first_id_and_scroll(0, 0.0);
        self.view.redraw(cx);
    }

    /// Empties the search field and offers everyone known again.
    fn clear_query(&mut self, cx: &mut Cx) {
        self.view.text_input(cx, ids!(user_id_input)).set_text(cx, "");
        let everyone = self.local_matches("");
        self.offer(cx, &everyone);
    }

    /// A known display name for `id`, from the people offered so far.
    fn name_of(&self, id: &ruma::UserId) -> Option<String> {
        self.known.iter().chain(&self.offered)
            .filter(|(known, _)| known == id)
            .find_map(|(_, name)| name.clone().filter(|name| !name.trim().is_empty()))
    }

    /// Known people whose name or ID contains `query`, ignoring case.
    fn local_matches(&self, query: &str) -> Vec<Person> {
        let needle = query.to_lowercase();
        self.known
            .iter()
            .filter(|(id, name)| {
                id.as_str().to_lowercase().contains(&needle)
                    || name.as_deref().is_some_and(|n| n.to_lowercase().contains(&needle))
            })
            .cloned()
            .collect()
    }

    /// A typed name: ask the homeserver directory too, then resolve.
    fn search(&mut self, cx: &mut Cx, query: String) {
        self.request += 1;
        let request = self.request;
        self.status(cx, &crate::i18n::format("Searching for {0}…", &[("0", query.clone())]), Tone::Info);
        let Some(client) = get_client() else {
            let matches = self.local_matches(&query);
            self.resolved(cx, &query, matches);
            return;
        };
        spawn_async_task(async move {
            let people = match client.search_users(&query, 20).await {
                Ok(found) => found
                    .results
                    .into_iter()
                    .filter(|u| Some(u.user_id.as_ref()) != client.user_id())
                    .map(|u| (u.user_id, u.display_name))
                    .collect(),
                Err(_) => Vec::new(),
            };
            Cx::post_action(InvitePeopleAction::Searched { request, query, people });
            SignalToUI::set_ui_signal();
        });
    }

    /// One match joins the selection; several are offered in the list to
    /// choose from; none asks for the full ID.
    fn resolved(&mut self, cx: &mut Cx, query: &str, matches: Vec<Person>) {
        match matches.as_slice() {
            [(id, name)] => {
                let id = id.clone();
                self.selection.add(id.clone(), name.clone());
                self.clear_query(cx);
                let label = self.selection.get(&id).map_or_else(|| id.to_string(), |i| i.label().to_owned());
                self.status(cx, &crate::i18n::format("Added {0}. Press Invite to send.", &[("0", label)]), Tone::Info);
                self.refresh(cx);
            }
            [] => self.status(
                cx,
                &crate::i18n::format("No one found for \"{0}\". Enter their full ID, like @name:server.", &[("0", query.to_owned())]),
                Tone::Danger,
            ),
            _ => {
                self.offer(cx, &matches);
                self.status(cx, &crate::i18n::format("{0} people match. Select the ones to invite above.", &[("0", matches.len().to_string())]), Tone::Info);
            }
        }
    }

    fn status(&mut self, cx: &mut Cx, text: &str, tone: Tone) {
        let mut label = self.view.label(cx, ids!(status_label_view.status_label));
        let text = text.to_owned();
        match tone {
            Tone::Danger => script_apply_eval!(cx, label, { text: #(text), draw_text +: { color: mod.widgets.COLOR_FG_DANGER_RED } }),
            Tone::Success => script_apply_eval!(cx, label, { text: #(text), draw_text +: { color: mod.widgets.COLOR_FG_ACCEPT_GREEN } }),
            Tone::Info => script_apply_eval!(cx, label, { text: #(text), draw_text +: { color: mod.widgets.COLOR_ACTIVE_PRIMARY_DARKER } }),
        }
        self.view.view(cx, ids!(status_label_view)).set_visible(cx, true);
        self.view.redraw(cx);
    }

    /// Everyone who shares a joined room with the user, except the user and
    /// whoever is already in the target room.
    fn load_known(&mut self, room: ruma::OwnedRoomId) {
        let Some(client) = get_client() else { return };
        let request = self.request;
        spawn_async_task(async move {
            let me = client.user_id().map(ToOwned::to_owned);
            let mut members_joined: Vec<OwnedUserId> = Vec::new();
            let mut invited: Vec<OwnedUserId> = Vec::new();
            if let Some(target) = client.get_room(&room)
                && let Ok(members) = target
                    .members_no_sync(matrix_sdk::RoomMemberships::JOIN | matrix_sdk::RoomMemberships::INVITE)
                    .await
            {
                for member in members {
                    let id = member.user_id().to_owned();
                    if *member.membership() == ruma::events::room::member::MembershipState::Invite {
                        invited.push(id);
                    } else {
                        members_joined.push(id);
                    }
                }
            }
            let mut people: Vec<Person> = Vec::new();
            for joined in client.joined_rooms() {
                if crate::moments::is_moments(&joined) {
                    continue;
                }
                let Ok(members) = joined.members_no_sync(matrix_sdk::RoomMemberships::JOIN).await else {
                    continue;
                };
                for member in members {
                    let id = member.user_id().to_owned();
                    if Some(&id) == me.as_ref() || members_joined.contains(&id) || invited.contains(&id) || people.iter().any(|(p, _)| p == &id) {
                        continue;
                    }
                    people.push((id, member.display_name().map(str::to_owned)));
                    if people.len() >= 500 {
                        break;
                    }
                }
            }
            Cx::post_action(InvitePeopleAction::Known { request, people, joined: members_joined, invited });
            SignalToUI::set_ui_signal();
        });
    }

    pub fn show(&mut self, cx: &mut Cx, room_name_id: RoomNameId) {
        self.view.label(cx, ids!(title)).set_text(
            cx,
            &crate::i18n::format("Invite to {0}", &[("0", room_name_id.to_string())]),
        );
        let settings = crate::agent_access::current();
        self.request += 1;
        self.known_request = self.request;
        self.epoch = crate::account_session::epoch();
        self.can_invite = false;
        submit_async_request(MatrixRequest::GetRoomPowerLevels {room_id: room_name_id.room_id().clone()});
        self.known = settings
            .agent_registry
            .agents()
            .map(|(id, entry)| (id.clone(), entry.display_name.clone()))
            .collect();
        self.selection.clear();
        self.joined.clear();
        self.invited.clear();
        let known = self.known.clone();
        self.offer(cx, &known);
        self.load_known(room_name_id.room_id().clone());
        self.room_name_id = Some(room_name_id);

        // Reset the UI state
        let confirm_button = self.view.button(cx, ids!(confirm_button));
        let cancel_button = self.view.button(cx, ids!(cancel_button));
        let okay_button = self.view.button(cx, ids!(okay_button));
        let user_id_input = self.view.text_input(cx, ids!(user_id_input));
        confirm_button.set_enabled(cx, true);
        confirm_button.reset_hover(cx);
        cancel_button.set_enabled(cx, true);
        cancel_button.reset_hover(cx);
        okay_button.reset_hover(cx);
        user_id_input.set_text(cx, "");
        self.view.view(cx, ids!(status_label_view)).set_visible(cx, false);
        self.view.label(cx, ids!(status_label_view.status_label)).set_text(cx, "");
        self.refresh(cx);
        user_id_input.set_key_focus(cx);
    }

    fn may_invite_user(&self, user_id: &ruma::UserId) -> bool {
        self.epoch == crate::account_session::epoch()
            && (self.can_invite || crate::hagency::backend::is_agent_invitation(user_id))
    }

    /// A complete Matrix ID typed in the field that is not selected yet. The
    /// button counts it, since pressing Invite adds it to the selection.
    fn typed_invitee(&self, cx: &mut Cx) -> Option<OwnedUserId> {
        let text = self.view.text_input(cx, ids!(user_id_input)).text();
        let text = text.trim();
        text.starts_with('@')
            .then(|| ruma::UserId::parse(text).ok())
            .flatten()
            .filter(|id| !self.selection.contains(id))
    }

    fn update_invite_button(&mut self, cx: &mut Cx) {
        let typed = self.typed_invitee(cx);
        let count = self.selection.sendable_count() + usize::from(typed.is_some());
        let allowed = self.selection.entries().iter()
            .filter(|invitee| invitee.outcome.is_sendable())
            .map(|invitee| &invitee.id)
            .chain(typed.as_ref())
            .any(|id| self.may_invite_user(id));
        let text = confirm_text(count, typed.is_none() && self.selection.only_pending_agents());
        let confirm = self.view.button(cx, ids!(confirm_button));
        if confirm.text() != text {
            confirm.set_text(cx, &text);
        }
        confirm.set_enabled(cx,
            count > 0 && allowed && self.epoch == crate::account_session::epoch()
                && !self.selection.is_sending() && !self.selection.all_done());
    }
}

impl InviteModalRef {
    /// Offline native interaction tests use the production controls and result
    /// handler. This seam cannot submit commands or run with a signed-in client.
    #[cfg(feature = "onboarding-test-support")]
    pub fn review_result(&self, cx: &mut Cx, scenario: &str) {
        assert!(get_client().is_none(), "Invitation review requires an offline profile");
        let Some(mut inner)=self.borrow_mut() else {return;};
        inner.review_mode=true;
        // This offline fixture represents a room whose current user can invite people.
        inner.can_invite=true;
        inner.update_invite_button(cx);
        if scenario=="ready" {
            inner.known=vec![("@_hagency_agt_cabbage:example.invalid".try_into().unwrap(),Some("小白菜".into())),("@friend:example.invalid".try_into().unwrap(),Some("Alex".into()))];
            let known=inner.known.clone();inner.offer(cx,&known);
            return;
        }
        if scenario=="many-people" {
            inner.known=(0..80).map(|i| (format!("@person_{i:02}:a-very-long-federated-homeserver.example.invalid").try_into().unwrap(), Some(format!("Friend {i:02} · 同名测试")))).collect();
            let known=inner.known.clone();inner.offer(cx,&known);
            return;
        }
        let sending: Vec<OwnedUserId>=inner.selection.entries().iter().filter(|i| i.outcome==Outcome::Sending).map(|i| i.id.clone()).collect();
        assert!(!sending.is_empty(), "Submit through the real Invite button first");
        let room_id=inner.room_name_id.as_ref().unwrap().room_id().clone();
        let epoch=inner.epoch;
        let last=sending.len()-1;
        for (index,user_id) in sending.into_iter().enumerate() {
            let room_id=room_id.clone();
            let agent=crate::hagency::backend::is_agent_invitation(&user_id);
            let result=match scenario {
                // Several invitees at once: Agents join, people are invited, and the last person fails.
                "mixed" if agent => InviteResultAction::Agent {room_id,user_id,epoch,result:Ok(true)},
                "mixed" if index==last => InviteResultAction::Failed {room_id,user_id,error:matrix_sdk::Error::UnknownError("M_FORBIDDEN: review failure".into())},
                "mixed" => InviteResultAction::Sent {room_id,user_id},
                "slow" => InviteResultAction::Waiting {room_id,user_id,epoch},
                "human" => InviteResultAction::Sent {room_id,user_id},
                "stale" => InviteResultAction::Agent {room_id,user_id,epoch:epoch.wrapping_add(1),result:Ok(true)},
                "other-room" => InviteResultAction::Agent {room_id:"!other:example.invalid".try_into().unwrap(),user_id,epoch,result:Ok(true)},
                _ => InviteResultAction::Agent {room_id,user_id,epoch,result:match scenario {"success"=>Ok(true),"pending"=>Ok(false),code=>Err(code.into())}},
            };
            inner.finish_invite(cx,&result);
        }
    }
    pub fn show(&self, cx: &mut Cx, room_name_id: RoomNameId) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, room_name_id);
    }
}

#[cfg(test)]
mod permission_tests {
    use super::*;

    #[test]
    fn matrix_invite_power_does_not_replace_agent_authorization_or_session_scope() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut modal = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            let value = script_eval!(vm, {mod.widgets.InviteModal {}});
            InviteModal::script_from_value(vm, value)
        });
        modal.epoch = crate::account_session::epoch();
        let person = ruma::user_id!("@person:test");
        let agent = ruma::user_id!("@_hagency_agt_example:test");
        assert!(!modal.may_invite_user(person));
        assert!(modal.may_invite_user(agent), "Agent invitations still go through the Hagency service");
        modal.can_invite = true;
        assert!(modal.may_invite_user(person));
        modal.can_invite = false;
        assert!(!modal.may_invite_user(person));
        modal.epoch = modal.epoch.wrapping_add(1);
        assert!(!modal.may_invite_user(agent));
        assert!(!modal.may_invite_user(person));
        assert!(!crate::sliding_sync::UserPowerLevels::assumed_until_fetched().can_invite());
    }
}

#[cfg(test)]
mod result_tests {
    use super::*;
    use crate::home::invite_selection::InviteSelection;

    fn id(text: &str) -> OwnedUserId {
        text.try_into().unwrap()
    }

    #[test]
    fn the_button_states_the_count() {
        assert_eq!(confirm_text(0, false), "Invite");
        assert_eq!(confirm_text(1, false), "Invite 1 person");
        assert_eq!(confirm_text(3, false), "Invite 3 people");
        assert_eq!(confirm_text(1, true), "Check status");
        assert_eq!(crate::i18n::format_in(crate::i18n::Language::Chinese, "Invite {0} people", &[("0", "3".into())]), "邀请 3 人");
    }

    #[test]
    fn each_invitee_reports_its_own_result() {
        let mut selection = InviteSelection::default();
        for user in ["@alex:test", "@_hagency_agt_coder:test", "@sam:test"] {
            selection.add(id(user), None);
        }
        selection.begin_send();
        selection.record(&id("@alex:test"), Outcome::Invited);
        selection.record(&id("@_hagency_agt_coder:test"), Outcome::Joined);
        selection.record(&id("@sam:test"), Outcome::Failed("Failed to send invite: forbidden".into()));
        let lines: Vec<(String, Tone)> = selection.attempted().map(outcome_text).collect();
        assert_eq!(lines[0], ("Invitation sent. Waiting for them to join.".into(), Tone::Success));
        assert_eq!(lines[1].1, Tone::Success);
        assert!(lines[1].0.starts_with("Agent joined this room."), "The Agent hint applies per invitee");
        assert_eq!(lines[2], ("Failed to send invite: forbidden".into(), Tone::Danger));
        assert_eq!(summary_text(&selection),
            ("1 of 3 invitations failed. They stay selected so you can try again.".into(), Tone::Danger));
    }

    #[test]
    fn a_single_invitee_reads_as_before() {
        let mut selection = InviteSelection::default();
        selection.add(id("@_hagency_agt_coder:test"), None);
        selection.begin_send();
        assert_eq!(selection.attempted().map(outcome_text).next().unwrap(), ("Adding Agent to this room…".into(), Tone::Info));
        selection.record(&id("@_hagency_agt_coder:test"), Outcome::AgentPending);
        let (text, tone) = summary_text(&selection);
        assert!(text.starts_with("The request is saved."));
        assert_eq!(tone, Tone::Info);
    }

    #[test]
    fn several_finished_invitees_are_summarized() {
        let mut selection = InviteSelection::default();
        selection.add(id("@a:test"), None);
        selection.add(id("@b:test"), None);
        selection.add(id("@c:test"), None);
        selection.begin_send();
        selection.record(&id("@a:test"), Outcome::Invited);
        selection.record(&id("@b:test"), Outcome::AlreadyMember);
        selection.record(&id("@c:test"), Outcome::AlreadyInvited);
        assert!(selection.all_done(), "Nobody already joined or invited is sent again");
        assert_eq!(outcome_text(&selection.entries()[2]), ("Already invited. Waiting for them to join.".into(), Tone::Success));
        assert_eq!(summary_text(&selection), ("3 invitations completed.".into(), Tone::Success));
    }
}
