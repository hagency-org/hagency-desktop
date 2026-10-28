//! Draw the actual chat shell and virtualized message rows without a Matrix account.
use super::*;
use makepad_widgets::makepad_draw::cx_draw::CxDraw;

fn screen() -> (Cx, RoomScreen) {
    let mut cx = Cx::new(Box::new(|_, _| {}));
    let screen = cx.with_vm(|vm| {
        makepad_widgets::script_mod(vm);
        crate::app::register_widgets(vm);
        vm.bx.captured_errors = Some(Vec::new());
        let value = script_eval!(vm, {mod.widgets.RoomScreen {desktop_chat_header.visible: true}});
        let screen = RoomScreen::script_from_value(vm, value);
        assert!(vm.take_errors().is_empty());
        screen
    });
    (cx, screen)
}

fn status(cx: &mut Cx, screen: &RoomScreen, permission: bool) -> WidgetRef {
    let status = screen.agent_status.widget(cx, ids!(shared_permission_notice));
    assert!(!status.is_empty());
    status.set_visible(cx, true);
    status.label(cx, ids!(title)).set_text(cx, if permission {"小白 · 权限请求"} else {"小白正在处理…"});
    status.widget(cx, ids!(activity_spinner)).set_visible(cx, !permission);
    for id in [ids!(context), ids!(request_details), ids!(hint), ids!(actions)] {
        status.widget(cx, id).set_visible(cx, permission);
    }
    status.label(cx, ids!(details)).set_text(cx, "Read files in the approved workspace");
    status.label(cx, ids!(hint)).set_text(cx, "Choose how to handle this request.");
    status
}

struct Geometry { header: Rect, timeline: Rect, input: Rect, last_message: Rect, status: Option<Rect> }

fn draw(cx: &mut Cx, screen: &mut RoomScreen, width: f64, messages: usize, tail: bool) -> Geometry {
    cx.redraw_all();
    screen.chat_info_requested = width >= 840.0;
    screen.layout_desktop_chat_info(cx, width);
    let has_status = screen.has_agent_status(cx);
    let mut last_message = WidgetRef::empty();
    let mut status_drawn = false;
    let pass = DrawPass::new(cx);
    pass.set_size(cx, dvec2(width, 900.0));
    let mut draw_list = DrawList2d::new(cx);
    let overlay = Overlay {draw_list: DrawList::new(cx)};
    let event = DrawEvent {redraw_all: true, ..DrawEvent::default()};
    let mut draw = CxDraw::new(cx, &event);
    let mut cx2d = Cx2d::new(&mut draw);
    cx2d.begin_pass(&pass, None);
    draw_list.begin_always(&mut cx2d);
    overlay.begin(&mut cx2d);
    cx2d.begin_root_turtle(dvec2(width, 900.0), Layout::flow_overlay());
    let mut scope = Scope::empty();
    while let Some(child) = screen.view.draw_walk(&mut cx2d, &mut scope, Walk {
        width: Size::Fixed(width), height: Size::Fixed(900.0), ..Walk::default()
    }).step() {
        let list_ref = child.as_portal_list();
        let mut list = list_ref.borrow_mut().unwrap();
        list.set_tail_range(tail);
        if !tail {list.set_first_id_and_scroll(0, 0.0);}
        list.set_item_range(&mut cx2d, 0, messages + usize::from(has_status));
        while let Some(index) = list.next_visible_item(&mut cx2d) {
            if has_status && index == messages {
                screen.agent_status.draw_all(&mut cx2d, &mut scope);
                status_drawn = true;
            } else {
                let mut item = list.item(&mut cx2d, index, id!(SmallStateEvent));
                script_apply_eval!(cx2d, item, {height: 76});
                item.label(&mut cx2d, ids!(content)).set_text(&mut cx2d, "Earlier conversation message");
                item.draw_all(&mut cx2d, &mut scope);
                if index == messages - 1 {last_message = item;}
            }
        }
    }
    overlay.end(&mut cx2d);
    cx2d.end_pass_sized_turtle();
    draw_list.end(&mut cx2d);
    cx2d.end_pass(&pass);
    drop(cx2d); drop(draw);
    Geometry {
        header: screen.view.view(cx, ids!(desktop_chat_header)).area().rect(cx),
        timeline: screen.view.portal_list(cx, ids!(timeline.list)).area().rect(cx),
        input: screen.view.room_input_bar(cx, ids!(room_input_bar)).area().rect(cx),
        last_message: last_message.area().rect(cx),
        status: status_drawn.then(|| screen.agent_status.area().rect(cx)),
    }
}

#[test]
fn activity_and_permission_cards_follow_messages_inside_chat_not_above_the_timeline() {
    for width in [360.0, 840.0, 1280.0] {
        let (mut cx, mut screen) = screen();
        let original = draw(&mut cx, &mut screen, width, 2, true);
        for permission in [false, true] {
            let notice = status(&mut cx, &screen, permission);
            let visible = draw(&mut cx, &mut screen, width, 2, true);
            let card = visible.status.expect("inline status must be drawn at the conversation tail");
            assert_eq!(original.header, visible.header);
            assert_eq!(original.timeline, visible.timeline, "status must not consume header space");
            assert_eq!(original.input, visible.input, "status must not become a fixed composer banner");
            assert!(card.size.y > 0.0 && card.pos.y >= visible.last_message.pos.y + visible.last_message.size.y - 0.1, "{card:?}");
            assert!(card.pos.y >= visible.timeline.pos.y && card.pos.y + card.size.y <= visible.input.pos.y + 0.1, "{card:?}");
            assert!(card.pos.x >= visible.timeline.pos.x && card.pos.x + card.size.x <= visible.timeline.pos.x + visible.timeline.size.x + 0.1);
            assert_eq!(notice.widget(&mut cx, ids!(activity_spinner)).visible(), !permission);
            if permission {
                for id in [ids!(once), ids!(session), ids!(deny)] {
                    let button = notice.button(&mut cx, id).area().rect(&cx);
                    assert!(button.size.y > 0.0 && button.pos.y >= card.pos.y && button.pos.y + button.size.y <= card.pos.y + card.size.y);
                }
            }
        }
        assert!(screen.view.widget(&mut cx, ids!(shared_permission_notice)).is_empty());
        assert!(screen.view.widget(&mut cx, ids!(agent_runtime_notice)).is_empty());
    }
}

#[test]
fn status_scrolls_with_history_preserves_its_identity_and_removes_the_completed_row() {
    let (mut cx, mut screen) = screen();
    let notice = status(&mut cx, &screen, false);
    let identity = notice.widget_uid();
    assert!(draw(&mut cx, &mut screen, 840.0, 30, true).status.is_some());
    assert!(draw(&mut cx, &mut screen, 840.0, 30, false).status.is_none(), "reading older messages must not show a pinned status banner");
    assert!(draw(&mut cx, &mut screen, 840.0, 31, true).status.is_some());
    assert_eq!(screen.agent_status.widget(&mut cx, ids!(shared_permission_notice)).widget_uid(), identity);
    notice.set_visible(&mut cx, false);
    assert!(!screen.has_agent_status(&mut cx));
    assert!(draw(&mut cx, &mut screen, 840.0, 31, true).status.is_none());
}
