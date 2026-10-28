//! Notify a retained script about language changes without rerunning its startup.
use super::*;

impl MiniAppsPanel {
    pub(super) fn refresh_script_language(&mut self, cx: &mut Cx, language: crate::i18n::Language) {
        if std::mem::replace(&mut self.language, language) == language {
            return;
        }
        if self.lease.is_none() {
            return;
        }
        if self.package.as_ref().is_some_and(|package| package.script) {
            let code = match language {
                crate::i18n::Language::English => "en",
                crate::i18n::Language::Chinese => "zh-CN",
            };
            // Optional hook, invoked in the existing heap with its instruction
            // policy. Do not reinstall the source, replay startup requests or
            // discard drafts and callbacks owned by this instance.
            let called = self.view.splash(cx, ids!(card)).call_script_fn_with_strings(
                cx,
                id!(on_app_locale_changed),
                &[code],
            );
            if called {
                self.reply_redraw = cx.new_next_frame();
                self.view.redraw(cx);
            }
        } else {
            self.restyle_card = true;
        }
    }
}
