//! The RoomsSideBar is the widget that contains the RoomsList and other items.
//!
//! It differs in what content it includes based on the adaptive view:
//! * On a narrow mobile view, it acts as the root_view of StackNavigation
//!   * It includes a title label, a search bar, and the RoomsList.
//! * On a wide desktop view, it acts as a permanent tab that is on the left side of the dock.
//!   * It includes a title label, a search bar and the RoomsList. Spaces are listed
//!     in the navigation rail rather than behind an "All Chats | Spaces" switch.
//!
//! Neither layout repeats a creation or tool shortcut above the list: New workspace
//! lives in the rail's "New or Join" menu, and File Transfer has its own rail
//! button on desktop and a Discover row on mobile.

use makepad_widgets::*;

use crate::home::rooms_list::RoomsListWidgetExt;
use crate::home::rooms_list_header::RoomsListHeaderAction;
use crate::home::joined_spaces::JoinedSpacesWidgetExt;
use crate::home::navigation_tab_bar::{NavigationBarAction, SelectedTab};
use crate::shared::navigation_bar_button::NavigationBarButtonWidgetExt;
use crate::shared::room_filter_input_bar::{MainFilterAction, RoomFilterInputBarWidgetExt};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    let ChatSwitchButton = NavigationBarButton {
        width: Fill height: 32
        draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_SURFACE border_radius: 5}
        label := Label {
            draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: (11.5 * mod.widgets.RINX_TEXT_SCALE)}}
        }
    }
    let ChatViewSwitch = SolidView {
        width: Fill height: 46 flow: Right
        padding: Inset{left: 12 right: 12 bottom: 10 top: 2}
        draw_bg.color: MOBILE_BG
        RoundedView {
            width: Fill height: Fill flow: Right padding: 2
            draw_bg +: {color: mod.widgets.RINX_FIELD border_radius: 6}
            all_chats := ChatSwitchButton {label.text: #(crate::i18n::tr("All Chats")) label.i18n_text: "All Chats"}
            joined_spaces_tab := ChatSwitchButton {label.text: #(crate::i18n::tr("Spaces")) label.i18n_text: "Spaces"}
        }
    }

    mod.widgets.RoomsSideBar = #(RoomsSideBar::register_widget(vm)) {
        Desktop := SolidView {
            padding: Inset{top: 20, left: 10, right: 10}
            flow: Down, spacing: 5
            width: Fill, height: Fill

            draw_bg.color: (RBX_BG_CANVAS)

            CachedWidget {
                rooms_list_header := RoomsListHeader {}
            }
            CachedWidget {
                room_filter_input_bar := RoomFilterInputBar {}
            }
            // No "All Chats | Spaces" switch here: on desktop, spaces live in the
            // navigation rail on the left, and selecting one filters this list.
            conversations := View {
                width: Fill height: Fill
                CachedWidget {rooms_list := RoomsList {}}
            }
        },

        Mobile := SolidView {
            width: Fill height: Fill flow: Down
            draw_bg.color: mod.widgets.RINX_SURFACE
            title_bar := MobileTitle {
                title.text: #(crate::i18n::tr("Chats")) title.i18n_text: "Chats"
                controls.right.visible: true
            }
            search_area := SolidView {
                width: Fill height: 48
                padding: Inset{left: 10 right: 10 bottom: 10}
                draw_bg.color: MOBILE_BG
                room_filter_input_bar := RoomFilterInputBar {
                    height: 36
                    draw_bg +: {color: mod.widgets.RINX_SURFACE border_size: 0 border_radius: 5}
                    input +: {
                        empty_text: #(crate::i18n::tr("Search")) i18n_empty_text: "Search"
                        draw_text.text_style.font_size: (11.5 * mod.widgets.RINX_TEXT_SCALE)
                        draw_bg +: {color: mod.widgets.RINX_SURFACE color_hover: mod.widgets.RINX_SURFACE color_focus: mod.widgets.RINX_SURFACE color_empty: mod.widgets.RINX_SURFACE}
                    }
                }
            }
            ChatViewSwitch {}
            conversations := View {
                width: Fill height: Fill
                CachedWidget {rooms_list := RoomsList {}}
            }
            spaces_tree := View {
                width: Fill height: Fill visible: false
                CachedWidget {joined_spaces := JoinedSpaces {}}
            }
        }
    }
}

/// Shared across adaptive sidebar replacements, but reset when signing out.
#[derive(Default)]
struct ChatsViewState { spaces: bool }

/// A simple wrapper around `AdaptiveView` that contains several global singleton widgets.
///
/// * In the mobile view, it serves as the root view of the StackNavigation,
///   showing the title label, the search bar, and the RoomsList.
/// * In the desktop view, it is a permanent tab in the dock,
///   showing the title label, the search bar and the RoomsList.
#[derive(Script, Widget)]
pub struct RoomsSideBar {
    #[deref] view: AdaptiveView,

    #[rust] mobile_search_collapsed: bool,
    #[rust] pending_search_focus: bool,
}

impl ScriptHook for RoomsSideBar {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            // Here we set the global singleton for the RoomsList widget,
            // which is used to access the list of rooms from anywhere in the app.
            cx.set_global(self.view.rooms_list(cx, ids!(rooms_list)));

            // Follow the enclosing HomeScreen, including its view-mode override.
            // The desktop dock pane is narrow even in a wide window; selecting
            // by this pane's width would incorrectly put mobile UI inside it.
            self.view.set_variant_selector(|cx, _| {
                if crate::home::home_screen::effective_is_desktop(cx) {
                    live_id!(Desktop)
                } else {
                    live_id!(Mobile)
                }
            });
        });
    }
}

impl RoomsSideBar {
    fn active_mobile_list(&self, cx: &mut Cx) -> PortalListRef {
        if cx.global::<ChatsViewState>().spaces {
            self.view.portal_list(cx, ids!(joined_spaces.list))
        } else {
            self.view.portal_list(cx, ids!(rooms_list.list))
        }
    }

    fn update_mobile_search(&mut self, cx: &mut Cx) {
        if crate::home::home_screen::effective_is_desktop(cx) {
            self.pending_search_focus = false;
            return;
        }
        let list = self.active_mobile_list(cx);
        let collapsed = list.first_id() != 0 || list.scroll_position() < -0.5;
        self.view.view(cx, ids!(search_area)).set_visible(cx, !collapsed);
        self.view.button(cx, ids!(title_bar.controls.search)).set_visible(cx, collapsed);
        if collapsed != self.mobile_search_collapsed {
            self.mobile_search_collapsed = collapsed;
            if collapsed {
                let input = self.view.text_input(cx, ids!(room_filter_input_bar.input));
                if cx.has_key_focus(input.area()) {
                    cx.set_key_focus(Area::Empty);
                    cx.hide_text_ime();
                }
            }
            // Redraw the enclosing layout only when the header changes. The
            // list keeps its per-scroll actions disabled on the hot path.
            self.view.redraw(cx);
        }
    }

    fn open_mobile_search(&mut self, cx: &mut Cx) {
        if crate::home::home_screen::effective_is_desktop(cx) { return; }
        self.pending_search_focus = true;
        // This API cancels a running fling before returning to the beginning.
        self.active_mobile_list(cx).smooth_scroll_to(cx, 0, 1000.0, Some(1), 0.0);
        self.view.redraw(cx);
    }
}

impl Widget for RoomsSideBar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // If the main room filter input bar changed keywords, re-emit that action
        // as a MainFilterAction so that other widgets can handle it.
        if let Event::Actions(actions) = event {
            if self.view.navigation_bar_button(cx, ids!(all_chats)).clicked(actions) {
                cx.global::<ChatsViewState>().spaces = false;
                cx.action(NavigationBarAction::GoToHome);
                self.view.redraw(cx);
            }
            if self.view.navigation_bar_button(cx, ids!(joined_spaces_tab)).clicked(actions) {
                cx.global::<ChatsViewState>().spaces = true;
                cx.action(NavigationBarAction::GoToHome);
                self.view.redraw(cx);
            }
            if self.view.button(cx, ids!(title_bar.controls.right)).clicked(actions) {
                cx.action(crate::home::navigation_tab_bar::NavigationBarAction::GoToTab(
                    crate::home::navigation_tab_bar::SelectedTab::Contacts,
                ));
            }
            if self.view.button(cx, ids!(title_bar.controls.search)).clicked(actions) {
                self.open_mobile_search(cx);
            }
            if let Some(keywords) = self.view.room_filter_input_bar(cx, ids!(room_filter_input_bar)).changed(actions) {
                cx.action(MainFilterAction::Changed(keywords));
            }

            for action in actions {
                if let Some(crate::logout::logout_confirm_modal::LogoutAction::ClearAppState {..}) = action.downcast_ref() {
                    cx.global::<ChatsViewState>().spaces = false;
                    self.pending_search_focus = false;
                }
                // A legacy desktop Space shortcut still opens its filtered list.
                if let Some(NavigationBarAction::TabSelected(SelectedTab::Space {..})) = action.downcast_ref() {
                    cx.global::<ChatsViewState>().spaces = false;
                }
                // Both layouts own their search field in this sidebar.
                if let Some(RoomsListHeaderAction::OpenRoomFilterModal) = action.downcast_ref() {
                    if crate::home::home_screen::effective_is_desktop(cx) {
                        self.view.text_input(cx, ids!(room_filter_input_bar.input)).take_key_focus(cx);
                    } else {
                        self.open_mobile_search(cx);
                    }
                }
            }
        }
        self.view.handle_event(cx, event, scope);
        self.update_mobile_search(cx);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // The desktop layout has no spaces tree (the rail lists spaces instead).
        let spaces = cx.global::<ChatsViewState>().spaces
            && !crate::home::home_screen::effective_is_desktop(cx);
        self.view.view(cx, ids!(conversations)).set_visible(cx, !spaces);
        self.view.view(cx, ids!(spaces_tree)).set_visible(cx, spaces);
        self.view.navigation_bar_button(cx, ids!(all_chats)).set_selected(cx, !spaces);
        self.view.navigation_bar_button(cx, ids!(joined_spaces_tab)).set_selected(cx, spaces);
        self.view.joined_spaces(cx, ids!(joined_spaces)).set_active(cx, spaces);
        self.update_mobile_search(cx);
        let search_drawn = !self.mobile_search_collapsed;
        let step = self.view.draw_walk(cx, scope, walk);
        if step.is_done() {
            // Drawing may normalize/clamp the list after filtering or loading.
            self.update_mobile_search(cx);
            if self.pending_search_focus && search_drawn && !self.mobile_search_collapsed {
                self.pending_search_focus = false;
                self.view.text_input(cx, ids!(room_filter_input_bar.input)).take_key_focus(cx);
            }
        }
        step
    }
}

/// The rooms list column, built and drawn offscreen in both layouts.
#[cfg(test)]
mod tests {
    use super::*;
    use makepad_widgets::makepad_draw::cx_draw::CxDraw;

    fn texts(widget: &WidgetRef, found: &mut Vec<String>) {
        found.push(widget.text());
        widget.children(&mut |_, child| texts(&child, found));
    }

    #[test]
    fn rooms_list_has_no_new_workspace_button_or_file_transfer_row() {
        for layout in [id!(Desktop), id!(Mobile)] {
            let mut cx = Cx::new(Box::new(|_, _| {}));
            let mut sidebar = cx.with_vm(|vm| {
                makepad_widgets::script_mod(vm);
                crate::app::register_widgets(vm);
                vm.bx.captured_errors = Some(Vec::new());
                let value = script_eval!(vm, {mod.widgets.RoomsSideBar {}});
                let sidebar = RoomsSideBar::script_from_value(vm, value);
                let errors = vm.take_errors();
                assert!(errors.is_empty(), "Rooms sidebar registration: {errors:?}");
                sidebar
            });
            sidebar.view.set_variant_selector(move |_, _| layout);
            let size = dvec2(320.0, 760.0);
            cx.redraw_all();
            let pass = DrawPass::new(&mut cx);
            pass.set_size(&mut cx, size);
            let mut list = DrawList2d::new(&mut cx);
            let event = DrawEvent::default();
            {
                let mut draw = CxDraw::new(&mut cx, &event);
                let mut cx2d = Cx2d::new(&mut draw);
                cx2d.begin_pass(&pass, None);
                list.begin_always(&mut cx2d);
                cx2d.begin_root_turtle(size, Layout::flow_overlay());
                // The rooms list reads the selected room from the app state.
                let mut state = crate::app::AppState::default();
                sidebar.draw_walk_all(&mut cx2d, &mut Scope::with_data(&mut state), Walk {
                    width: Size::Fixed(size.x),
                    height: Size::Fixed(size.y),
                    ..Walk::default()
                });
                cx2d.end_pass_sized_turtle();
                list.end(&mut cx2d);
                cx2d.end_pass(&pass);
            }
            assert_eq!(sidebar.view.active_variant(), Some(layout));
            for removed in [ids!(new_workspace), ids!(file_transfer_entry)] {
                assert!(sidebar.view.widget(&mut cx, removed).is_empty(), "{layout:?}: {removed:?} is gone");
            }
            let mut found = Vec::new();
            sidebar.children(&mut |_, child| texts(&child, &mut found));
            assert!(found.iter().any(|text| !text.is_empty()), "{layout:?}: the walk reaches the column's labels");
            for label in ["New workspace", "File Transfer"] {
                assert!(!found.iter().any(|text| text == crate::i18n::tr(label)), "{layout:?}: no {label} entry above the list: {found:?}");
            }
            // The conversation list now starts right under the search field.
            let conversations = sidebar.view.view(&mut cx, ids!(conversations)).area().rect(&cx);
            let search = sidebar.view.widget(&mut cx, ids!(room_filter_input_bar)).area().rect(&cx);
            assert!(conversations.size.y > 400.0, "{layout:?}: the list keeps the column: {conversations:?}");
            assert!(conversations.pos.y > search.pos.y && conversations.pos.y - (search.pos.y + search.size.y) < 60.0,
                "{layout:?}: nothing sits between search and list: {search:?} {conversations:?}");
        }
    }
}
