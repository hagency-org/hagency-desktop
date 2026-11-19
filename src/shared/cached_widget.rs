//! Compatibility with the pinned Makepad CachedWidget during a DSL rebake.
//!
//! Upstream keeps its old template on Rebake/ScriptReapply, then mistakes the
//! newly visited single child for an additional child. Clear only that template
//! through its public hook; the cached child and its runtime state stay alive.
use makepad_widgets::*;
use std::collections::HashMap;

script_mod! {
    use mod.prelude.widgets_internal.*
    mod.widgets.CachedWidget = #(RebakeCachedWidget::register_widget(vm))
    mod.widgets.AccountCachedWidget = #(AccountCachedWidget::register_widget(vm))
}

#[derive(Script, Widget)]
pub struct RebakeCachedWidget {
    #[deref]
    cached: CachedWidget,
}

impl ScriptHook for RebakeCachedWidget {
    fn on_before_apply(
        &mut self,
        vm: &mut ScriptVm,
        apply: &Apply,
        scope: &mut Scope,
        value: ScriptValue,
    ) {
        if matches!(apply, Apply::Rebake | Apply::ScriptReapply) {
            self.cached
                .on_before_apply(vm, &Apply::Reload, scope, value);
        }
    }
}

impl Widget for RebakeCachedWidget {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.cached.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.cached.draw_walk(cx, scope, walk)
    }
}

#[derive(Default)]
struct AccountWidgetCache(HashMap<(usize, LiveId), WidgetRef>);

pub(crate) fn clear_session(vm: &mut ScriptVm) {
    let heap = vm.heap_key();
    vm.cx_mut().global::<AccountWidgetCache>().0.retain(|(owner, _), _| *owner != heap);
}

/// Share a widget between the desktop and mobile layouts of one signed-in workspace.
#[derive(Script, WidgetRef, WidgetRegister)]
pub struct AccountCachedWidget {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[rust]
    child: Option<(LiveId, WidgetRef)>,
}

impl ScriptHook for AccountCachedWidget {
    fn on_after_apply(
        &mut self,
        vm: &mut ScriptVm,
        apply: &Apply,
        scope: &mut Scope,
        value: ScriptValue,
    ) {
        let Some(object) = value.as_object() else {
            return;
        };
        let mut template = None;
        vm.vec_with(object, |_, fields| {
            template = fields
                .iter()
                .find_map(|kv| Some((kv.key.as_id()?, kv.value)));
        });
        let Some((id, value)) = template else { return };
        if let Some((_, child)) = &mut self.child {
            child.script_apply(vm, apply, scope, value);
        } else {
            let key = (vm.heap_key(), id);
            let cached = vm
                .cx_mut()
                .global::<AccountWidgetCache>()
                .0
                .get(&key)
                .cloned();
            let child = cached.unwrap_or_else(|| {
                let child = WidgetRef::script_from_value_scoped(vm, scope, value);
                vm.cx_mut()
                    .global::<AccountWidgetCache>()
                    .0
                    .insert(key, child.clone());
                child
            });
            self.child = Some((id, child));
        }
        if let Some((id, child)) = &self.child {
            vm.cx_mut()
                .widget_tree_insert_child_deep(self.uid, *id, child.clone());
        }
    }
}

impl WidgetNode for AccountCachedWidget {
    fn widget_uid(&self) -> WidgetUid {
        self.uid
    }
    fn walk(&mut self, cx: &mut Cx) -> Walk {
        self.child
            .as_ref()
            .map_or(self.walk, |(_, child)| child.walk(cx))
    }
    fn area(&self) -> Area {
        self.child
            .as_ref()
            .map_or(Area::Empty, |(_, child)| child.area())
    }
    fn children(&self, visit: &mut dyn FnMut(LiveId, WidgetRef)) {
        if let Some((id, child)) = &self.child {
            visit(*id, child.clone());
        }
    }
    fn redraw(&mut self, cx: &mut Cx) {
        if let Some((_, child)) = &self.child {
            child.redraw(cx);
        }
    }
}

impl Widget for AccountCachedWidget {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if let Some((_, child)) = &self.child {
            child.handle_event(cx, event, scope);
        }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.child.as_ref().map_or(DrawStep::done(), |(_, child)| {
            child.draw_walk(cx, scope, walk)
        })
    }
}
