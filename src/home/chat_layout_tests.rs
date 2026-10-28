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
