//! Saved layouts from the management-first shell must keep their conversations.
use super::*;

fn room(name: &str) -> SelectedRoom {
    SelectedRoom::JoinedRoom {room_name_id: RoomNameId::new(
        matrix_sdk::RoomDisplayName::Named(name.into()),
        format!("!{name}:example.org").try_into().unwrap(),
    )}
}
fn tabs(ids: Vec<LiveId>) -> DockItem {
    DockItem::Tabs {tabs: ids, selected: 0, closable: true, hide_tab_bar: false}
}
fn tab(kind: LiveId) -> DockItem {
    DockItem::Tab {name: "Saved tab".into(), template: id!(PermanentTab), kind}
}
fn verify(saved: &mut SavedDockState, rooms: &[SelectedRoom]) {
    assert!(dock_state_repair::validate_and_repair_dock_state(saved).unwrap());
    assert!(matches!(saved.dock_items[&id!(home_tab)], DockItem::Tab {kind, ..} if kind == id!(welcome_screen)));
    assert!(matches!(&saved.dock_items[&id!(rooms_sidebar_tabs)], DockItem::Tabs {tabs, hide_tab_bar: true, ..} if tabs == &[id!(rooms_sidebar_tab)]));
    assert_eq!(saved.room_order, rooms);
    assert_eq!(saved.selected_room.as_ref(), rooms.last());
    assert_eq!(saved.open_rooms.len(), rooms.len());
    for room in rooms {
        assert_eq!(saved.open_rooms.get(&room.tab_id()), Some(room));
        assert!(saved.dock_items.contains_key(&room.tab_id()));
    }
    assert!(!dock_state_repair::validate_and_repair_dock_state(saved).unwrap(), "migration is idempotent");
}

#[test]
fn management_root_migrates_to_chat_sidebar_without_losing_current_room() {
    let room = room("current");
    let mut saved = SavedDockState::default();
    saved.dock_items.extend([
        (id!(root), tabs(vec![id!(home_tab), room.tab_id()])),
        (id!(home_tab), tab(id!(hagency_panel))),
        (room.tab_id(), tab(room.dock_kind())),
    ]);
    saved.open_rooms.insert(room.tab_id(), room.clone());
    saved.room_order.push(room.clone()); saved.selected_room = Some(room.clone());
    verify(&mut saved, &[room]);
    assert!(matches!(saved.dock_items[&id!(root)], DockItem::Splitter {a, b, ..} if a == id!(rooms_sidebar_tabs) && b == id!(main_tabs)));
}

#[test]
fn management_split_chat_panes_survive_sidebar_restoration() {
    let a = room("first"); let b = room("second");
    let mut saved = SavedDockState::default();
    saved.dock_items.extend([
        (id!(root), DockItem::Splitter {axis: SplitterAxis::Horizontal, align: SplitterAlign::Weighted(0.5), a: id!(main_tabs), b: id!(other_tabs)}),
        (id!(main_tabs), tabs(vec![id!(home_tab), a.tab_id()])),
        (id!(other_tabs), tabs(vec![b.tab_id()])),
        (id!(home_tab), tab(id!(hagency_panel))),
        (a.tab_id(), tab(a.dock_kind())), (b.tab_id(), tab(b.dock_kind())),
    ]);
    saved.open_rooms.extend([(a.tab_id(), a.clone()), (b.tab_id(), b.clone())]);
    saved.room_order = vec![a.clone(), b.clone()]; saved.selected_room = Some(b.clone());
    verify(&mut saved, &[a, b.clone()]);
    assert!(matches!(&saved.dock_items[&id!(other_tabs)], DockItem::Tabs {tabs, ..} if tabs.contains(&b.tab_id())));
    assert_eq!(saved.dock_items.values().filter(|item| matches!(item, DockItem::Splitter {..})).count(), 2);
}

#[test]
fn existing_rinx_sidebar_is_not_duplicated_and_lost_room_ref_is_repaired() {
    let room = room("restored");
    let mut saved = SavedDockState::default();
    saved.dock_items.extend([
        (id!(root), DockItem::Splitter {axis: SplitterAxis::Horizontal, align: SplitterAlign::FromA(300.0), a: id!(rooms_sidebar_tabs), b: id!(main_tabs)}),
        (id!(rooms_sidebar_tabs), DockItem::Tabs {tabs: vec![id!(rooms_sidebar_tab)], selected: 0, closable: false, hide_tab_bar: true}),
        (id!(rooms_sidebar_tab), tab(id!(rooms_sidebar))),
        (id!(main_tabs), tabs(vec![id!(home_tab)])),
        (id!(home_tab), tab(id!(welcome_screen))),
    ]);
    saved.room_order.push(room.clone()); saved.selected_room = Some(room.clone());
    verify(&mut saved, &[room]);
    assert_eq!(saved.dock_items.values().filter(|item| matches!(item, DockItem::Splitter {..})).count(), 1);
}

fn desktop() -> (Cx, MainDesktopUI) {
    let mut cx = Cx::new(Box::new(|_, _| {}));
    let desktop = cx.with_vm(|vm| {
        makepad_widgets::script_mod(vm);
        crate::app::register_widgets(vm);
        let value = script_eval!(vm, { mod.widgets.MainDesktopUI {} });
        MainDesktopUI::script_from_value(vm, value)
    });
    (cx, desktop)
}

#[test]
fn moments_uses_the_chat_pane_without_changing_saved_conversations_or_windows() {
    let (mut cx, mut desktop) = desktop();
    let room = room("current");
    desktop.focus_or_create_tab(&mut cx, room.clone());
    let before = desktop.save_dock_state(&mut cx);
    let windows = cx.windows.len();
    desktop.open_moments(&mut cx, &MomentsAction::Open {author: None});
    let dock = desktop.view.dock(&cx, ids!(dock));
    assert!(desktop.moments_open);
    assert!(!dock.item(id!(moments_tab)).is_empty());
    assert_eq!(dock.find_tab_bar_of_tab(id!(moments_tab)).map(|p| p.0),
        dock.find_tab_bar_of_tab(room.tab_id()).map(|p| p.0));
    assert_eq!(cx.windows.len(), windows, "Moments must not create a window");
    let during = desktop.save_dock_state(&mut cx);
    assert!(!during.dock_items.contains_key(&id!(moments_tab)));
    assert_eq!(during.selected_room, before.selected_room);
    assert_eq!(during.room_order, before.room_order);
    assert_eq!(during.open_rooms, before.open_rooms);
    desktop.close_moments(&mut cx);
    assert!(!desktop.moments_open);
    assert!(dock.item(id!(moments_tab)).is_empty());
    assert_eq!(desktop.most_recently_selected_room.as_ref(), Some(&room));
}

#[test]
fn choosing_the_same_chat_or_another_chat_leaves_moments() {
    let (mut cx, mut desktop) = desktop();
    let first = room("first");
    let second = room("second");
    desktop.focus_or_create_tab(&mut cx, first.clone());
    for next in [first, second] {
        desktop.open_moments(&mut cx, &MomentsAction::Open {author: None});
        desktop.focus_or_create_tab(&mut cx, next.clone());
        assert!(!desktop.moments_open);
        assert_eq!(desktop.most_recently_selected_room.as_ref(), Some(&next));
        let dock = desktop.view.dock(&cx, ids!(dock));
        assert!(dock.item(id!(moments_tab)).is_empty());
        assert!(!dock.item(next.tab_id()).is_empty());
    }
}
