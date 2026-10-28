//! Compatibility for the original Matrix bridge's backend-hosted `/msg/:id`
//! links. They carry an opaque archive ID and a per-message view capability,
//! not a Matrix event ID or permission to read the current account's store.
use super::*;

fn routed(cx: &mut Cx, address: &str) -> WebBrowserAction {
    let actions = cx.capture_actions(|cx| assert!(open_chat_link(cx, address)));
    assert_eq!(actions.len(), 1, "one browser action, no management action");
    actions[0]
        .downcast_ref::<WebBrowserAction>()
        .expect("legacy archive URLs use the existing HTTP reader")
        .clone()
}

#[test]
fn legacy_message_links_preserve_original_origin_id_and_view_capability() {
    // Synthetic capabilities only. The archive server remains responsible for
    // validating them; Desktop must not invent local identity from `msg_7`.
    for address in [
        "https://archive.example.test/msg/msg_7?view=fixture_capability-123",
        "https://archive.example.test/msg/opaque%3Aid%2F7?view=fixture%2Btoken%2Fvalue%3D",
        "http://127.0.0.1:8090/msg/msg_7?view=fixture_capability-123",
        "http://127.0.0.1:8084/msg/msg_7?view=fixture_capability-123",
        "https://archive.example.test/msg/msg_7?agent=alpha",
    ] {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let WebBrowserAction::Open(url) = routed(&mut cx, address) else {
            panic!("archive content must not be replaced with another read")
        };
        assert_eq!(url.as_str(), address);
    }
}

#[test]
fn legacy_message_tabs_keep_same_ids_on_different_archive_origins_separate() {
    let mut cx = Cx::new(Box::new(|_, _| {}));
    let browser = cx.with_vm(|vm| {
        makepad_widgets::script_mod(vm);
        crate::app::register_widgets(vm);
        let value = script_eval!(vm, { mod.widgets.WebBrowser {} });
        WidgetRef::script_from_value(vm, value).as_web_browser()
    });
    let urls = [
        "https://first.example.test/msg/msg_7?view=fixture_first",
        "https://second.example.test/msg/msg_7?view=fixture_second",
    ];
    for address in urls {
        let action = routed(&mut cx, address);
        browser.action(&mut cx, ModalRef::default(), &action);
    }
    let session = browser.session();
    assert_eq!(session.active, 1);
    assert_eq!(session.tabs.len(), 2);
    for (tab, original) in session.tabs.iter().zip(urls) {
        assert!(matches!(tab, ReaderTab::Web { url } if url == original));
    }
    let first_id = browser.borrow().unwrap().tabs[0].id;
    let second_id = browser.borrow().unwrap().tabs[1].id;
    assert_ne!(first_id, second_id);
    browser.borrow_mut().unwrap().close_tab(&mut cx, first_id);
    assert!(matches!(&browser.session().tabs[0], ReaderTab::Web { url } if url == urls[1]));
}

#[test]
fn legacy_message_ids_without_archive_origin_do_not_gain_local_read_authority() {
    for address in ["/msg/msg_7", "msg_7", "file:///msg/msg_7"] {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let actions = cx.capture_actions(|cx| assert!(!open_chat_link(cx, address)));
        assert!(actions.is_empty());
    }
}
