//! JIT PORT: small fork-only `Cx` helpers rebuilt on upstream makepad.

use makepad_widgets::*;

/// Background Agent attention uses the same opt-in, generic system notification path.
pub fn show_notification(_cx: &mut Cx, _title: &str, _body: &str) {
    crate::system_notifications::show_activity();
}

/// Redraw `pass` and every pass it is drawn into, up to its window.
pub fn redraw_pass_and_parent_passes(cx: &mut Cx, pass: DrawPassId) {
    let mut walk = pass;
    loop {
        if let Some(list) = cx.passes[walk].main_draw_list_id {
            cx.redraw_list_and_children(list);
        }
        match cx.passes[walk].parent.clone() {
            CxDrawPassParent::DrawPass(next) => walk = next,
            _ => break,
        }
    }
}
