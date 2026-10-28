//! Shared SVG handles for contained mini apps; colors come from Rinx themes.
use makepad_widgets::*;
use makepad_widgets::widget_async::ScriptAsyncResult;
use makepad_widgets::makepad_script::ScriptFnRef;
use std::collections::HashMap;

/// An exact integer editor with explicit steps. No artificial capacity ceiling:
/// the maximum only prevents losing integer precision at the script boundary.
#[derive(Script, Widget)]
pub struct MiniAppNumberInput {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[live] text: String,
    #[live] empty_text: String,
    #[live] is_read_only: bool,
    #[live(100000.0)] step: f64,
    #[live(0.0)] minimum: f64,
    #[live] on_change: Option<ScriptFnRef>,
}
impl ScriptHook for MiniAppNumberInput {
    fn on_after_apply(&mut self, vm: &mut ScriptVm, _: &Apply, _: &mut Scope, _: ScriptValue) {
        let cx=vm.cx_mut();
        self.view.text_input(cx,ids!(input)).set_text(cx,&self.text);
        self.view.text_input(cx,ids!(input)).set_is_read_only(cx,self.is_read_only);
        self.view.text_input(cx,ids!(input)).set_empty_text(cx,self.empty_text.clone());
        self.sync_steps(cx);
    }
}
impl MiniAppNumberInput {
    const MAX: u64=9_007_199_254_740_991;
    fn number(&self)->Option<u64>{
        if self.text.is_empty(){return Some(0)}
        if !self.text.bytes().all(|v|v.is_ascii_digit()){return None}
        self.text.parse::<u64>().ok().filter(|v|*v<=Self::MAX)
    }
    fn sync_steps(&self,cx:&mut Cx){
        let value=self.number();
        self.view.button(cx,ids!(decrease)).set_disabled(cx,self.is_read_only || value.is_none_or(|v|v<=self.minimum.max(0.0) as u64));
        self.view.button(cx,ids!(increase)).set_disabled(cx,self.is_read_only || value.is_none_or(|v|v==Self::MAX));
    }
    fn notify_change(&self,cx:&mut Cx){
        // Trusted confirmation screens capture their draft on Actions. Keep
        // them informed even when this instance has no Splash callback.
        cx.widget_action(self.widget_uid(),TextInputAction::Changed(self.text.clone()));
        let Some(handler)=self.on_change.clone() else{return};
        let Some(vm_id)=cx.script_ref_vm_id(&self.source) else{return};
        let value=cx.with_script_vm_id(vm_id,|vm| ScriptValue::from(vm.bx.heap.new_string_from_str(&self.text)));
        cx.widget_to_script_call(self.widget_uid(),NIL,self.source.clone(),handler,&[value]);
    }
    fn adjust(&mut self,cx:&mut Cx,up:bool){
        if self.is_read_only{return}
        let Some(value)=self.number() else{return};
        let step=if self.step.is_finite(){self.step.max(1.0).min(Self::MAX as f64) as u64}else{1};
        let minimum=self.minimum.max(0.0).min(Self::MAX as f64) as u64;
        let value=if up{value.saturating_add(step).min(Self::MAX)}else{value.saturating_sub(step).max(minimum)};
        self.text=value.to_string();
        self.view.text_input(cx,ids!(input)).set_text(cx,&self.text);
        self.sync_steps(cx);self.notify_change(cx);
    }
}
impl Widget for MiniAppNumberInput {
    fn handle_event(&mut self,cx:&mut Cx,event:&Event,scope:&mut Scope){
        let actions=cx.capture_actions(|cx|self.view.handle_event(cx,event,scope));
        if self.is_read_only{return}
        if let Some(text)=self.view.text_input(cx,ids!(input)).changed(&actions){
            self.text=text;self.sync_steps(cx);self.notify_change(cx);
        }
        let decrease=self.view.button(cx,ids!(decrease));let increase=self.view.button(cx,ids!(increase));
        let mut down=decrease.clicked(&actions);let mut up=increase.clicked(&actions);
        if let Event::KeyDown(key)=event {
            if !key.is_repeat && !key.modifiers.logo && !key.modifiers.control && !key.modifiers.alt
                && matches!(key.key_code,KeyCode::Space|KeyCode::ReturnKey|KeyCode::NumpadEnter){
                down|=cx.has_key_focus(decrease.area()) && !decrease.disabled(cx);
                up|=cx.has_key_focus(increase.area()) && !increase.disabled(cx);
            }
        }
        if down{self.adjust(cx,false)}else if up{self.adjust(cx,true)}
    }
    fn draw_walk(&mut self,cx:&mut Cx2d,scope:&mut Scope,walk:Walk)->DrawStep{self.view.draw_walk(cx,scope,walk)}
    fn text(&self)->String{self.text.clone()}
    fn set_text(&mut self,cx:&mut Cx,text:&str){self.text=text.into();self.view.text_input(cx,ids!(input)).set_text(cx,text);self.sync_steps(cx)}
    fn script_call(&mut self,vm:&mut ScriptVm,method:LiveId,args:ScriptValue)->ScriptAsyncResult{
        if method==id!(set_text){
            if let Some(args)=args.as_object(){
                let value=vm.bx.heap.vec_value(args,0,NoTrap);
                if let Some(text)=vm.bx.heap.cast_to_owned_string(value,"number editor") {self.set_text(vm.cx_mut(),&text)}
            }
            return ScriptAsyncResult::Return(NIL)
        }
        self.view.script_call(vm,method,args)
    }
}

/// The pinned Slider has no script change callback. Keep the adapter local and
/// queue callbacks in the owning Splash isolate, just like TextInput does.
#[derive(Script, Widget)]
pub struct MiniAppAmountSlider {
    #[source] source: ScriptObjectRef,
    #[deref] slider: Slider,
    #[live] value: f64,
    #[live(true)] enabled: bool,
    #[live] on_change: Option<ScriptFnRef>,
    #[rust] focus_area: Area,
}
impl ScriptHook for MiniAppAmountSlider {
    fn on_after_apply(&mut self, vm: &mut ScriptVm, _: &Apply, _: &mut Scope, _: ScriptValue) {
        self.value = if self.value.is_finite() {self.value.clamp(0.0, 1.0)} else {0.0};
        if self.slider.dragging.is_none() {
            self.slider.set_value(vm.cx_mut(), self.value);
        }
        self.slider.set_disabled(vm.cx_mut(), !self.enabled);
    }
}
impl MiniAppAmountSlider {
    fn changed(&mut self, cx: &mut Cx, value: f64) {
        if !value.is_finite() {return;}
        self.value = value.clamp(0.0, 1.0);
        self.slider.set_value(cx, self.value);
        self.slider.redraw(cx);
        if let Some(handler) = self.on_change.clone() {
            cx.widget_to_script_call(self.widget_uid(), NIL, self.source.clone(), handler,
                &[ScriptValue::from_f64(self.value)]);
        }
    }
}
impl Widget for MiniAppAmountSlider {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        for action in cx.capture_actions(|cx| self.slider.handle_event(cx, event, scope)) {
            if !self.enabled {continue;}
            match action.as_widget_action().cast() {
                SliderAction::StartSlide | SliderAction::EndSlide(_) => cx.set_key_focus(self.focus_area),
                SliderAction::Slide(v) | SliderAction::TextSlide(v) => self.changed(cx, v),
                _ => (),
            }
        }
        if let Event::KeyDown(key) = event {
            if self.enabled && cx.has_key_focus(self.focus_area) {
                let value = match key.key_code {
                    KeyCode::ArrowLeft | KeyCode::ArrowDown => Some(self.value - 0.01),
                    KeyCode::ArrowRight | KeyCode::ArrowUp => Some(self.value + 0.01),
                    KeyCode::Home => Some(0.0), KeyCode::End => Some(1.0), _ => None,
                };
                if let Some(value) = value {self.changed(cx, value);}
            }
        }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let step = self.slider.draw_walk(cx, scope, walk);
        if step.is_done() {
            let rect = self.slider.area().rect(cx);
            cx.add_aligned_rect_area(&mut self.focus_area, rect);
            if self.enabled {cx.add_nav_stop(self.focus_area, NavRole::TextInput, Inset::default());}
        }
        step
    }
    fn script_call(&mut self, vm: &mut ScriptVm, method: LiveId, args: ScriptValue) -> ScriptAsyncResult {
        if method == id!(set_value) {
            if let Some(args) = args.as_object() {
                let trap = vm.bx.threads.cur().trap.pass();
                if let Some(value) = vm.bx.heap.vec_value(args, 0, trap).as_f64() {
                    self.value = if value.is_finite() {value.clamp(0.0, 1.0)} else {0.0};
                    self.slider.set_value(vm.cx_mut(), self.value);
                }
            }
            return ScriptAsyncResult::Return(NIL);
        }
        self.slider.script_call(vm, method, args)
    }
}

/// The pinned Button supports Tab focus but not keyboard activation. Keep the
/// adapter local to mini apps until the framework supplies the same contract.
#[derive(Script, Widget)]
pub struct MiniAppButton {
    #[deref]
    button: Button,
}
impl ScriptHook for MiniAppButton {
    fn on_after_apply(&mut self, vm: &mut ScriptVm, _: &Apply, _: &mut Scope, _: ScriptValue) {
        self.button.set_disabled(vm.cx_mut(), !self.button.enabled());
    }
}
impl Widget for MiniAppButton {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.button.handle_event(cx, event, scope);
        if let Event::KeyDown(key) = event {
            if self.button.enabled() && !self.button.disabled(cx) && self.button.visible()
                && cx.has_key_focus(self.button.area()) && !key.is_repeat
                && !key.modifiers.control && !key.modifiers.logo && !key.modifiers.alt
                && matches!(key.key_code, KeyCode::ReturnKey | KeyCode::NumpadEnter | KeyCode::Space)
            {
                cx.with_vm(|vm| { self.button.script_call(vm, id!(on_click), NIL); });
            }
        }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.button.draw_walk(cx, scope, walk)
    }
    fn script_call(&mut self, vm: &mut ScriptVm, method: LiveId, args: ScriptValue) -> ScriptAsyncResult {
        // Update one button without rebuilding its parent and stealing input focus.
        if method == id!(set_enabled) {
            if let Some(args) = args.as_object() {
                let trap = vm.bx.threads.cur().trap.pass();
                if let Some(enabled) = vm.bx.heap.vec_value(args, 0, trap).as_bool() {
                    let patch = vm.bx.heap.new_object();
                    vm.bx.heap.set_value(patch, id!(enabled).into(), enabled.into(), NoTrap);
                    self.button.script_apply(vm, &Apply::Animate, &mut Scope::empty(), patch.into());
                    self.button.set_disabled(vm.cx_mut(), !enabled);
                    self.button.redraw(vm.cx_mut());
                }
            }
            return ScriptAsyncResult::Return(NIL);
        }
        self.button.script_call(vm, method, args)
    }
    fn text(&self) -> String { self.button.text() }
    fn set_text(&mut self, cx: &mut Cx, text: &str) { self.button.set_text(cx,text); }
    fn disabled(&self, cx: &Cx) -> bool { !self.button.enabled() || self.button.disabled(cx) }
    fn set_disabled(&mut self, cx: &mut Cx, disabled: bool) { self.button.set_disabled(cx,disabled); }
}

/// Use the macOS UI's installed CJK sans fallback inside mini apps. Keep this
/// in the script font family so lazy emoji loading cannot discard the font.
/// Only the framework's default WenKai fallback is replaced; customer fonts
/// and the Latin face, metrics and lazy-loading policy are retained.
pub(super) fn install_system_cjk_fallback(vm: &mut ScriptVm) {
    #[cfg(target_os = "macos")]
    {
        const FONT: &str = "/System/Library/Fonts/Hiragino Sans GB.ttc";
        if !std::path::Path::new(FONT).is_file() { return; }
        vm.bx.code.crate_manifests.borrow_mut().insert(
            "rinx_system_fonts".into(), "/System/Library/Fonts".into());
        let resource = makepad_widgets::makepad_platform::script::res::register_crate_resource_path(
            vm, "rinx_system_fonts/Hiragino Sans GB.ttc");
        let theme = vm.module(id!(theme));
        for role in [id!(font_regular), id!(font_label), id!(font_bold), id!(font_italic), id!(font_bold_italic)] {
            let Some(style) = vm.bx.heap.value(theme, role.into(), NoTrap).as_object() else { continue; };
            let Some(family) = vm.bx.heap.value(style, id!(font_family).into(), NoTrap).as_object() else { continue; };
            for i in 0..vm.bx.heap.vec_len(family) {
                let kv = vm.bx.heap.vec_key_value(family, i, NoTrap);
                let Some(member) = kv.value.as_object() else { continue; };
                let Some(handle) = vm.bx.heap.value(member, id!(res).into(), NoTrap).as_handle() else { continue; };
                let heap_key = vm.bx.heap.heap_key();
                let path = vm.cx().script_data.resources.path_for_handle(heap_key, handle);
                if path.as_deref().is_some_and(|p| p.ends_with("/LXGWWenKaiRegular.ttf") || p.ends_with("/LXGWWenKaiBold.ttf")) {
                    vm.bx.heap.set_value(member, id!(res).into(), resource, NoTrap);
                }
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = vm;
}

/// A persistent scroll container for multi-page mini apps. Navigating to a new
/// route can reset its offset without rebuilding the scroll view or scheduling
/// a second render before newly created widgets have entered the widget tree.
#[derive(Script, ScriptHook, Widget)]
pub struct MiniAppPageScroll {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    #[rust]
    scroll_positions: HashMap<String, Vec2d>,
    #[rust]
    last_scroll: Vec2d,
    #[rust]
    restore_position: Option<Vec2d>,
    #[rust]
    restore_frame: NextFrame,
}
impl Widget for MiniAppPageScroll {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.restore_frame.is_event(event).is_some() {
            self.view.redraw(cx);
        }
        self.view.handle_event(cx, event, scope);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let step = self.view.draw_walk(cx, scope, walk);
        if step.is_done() {
            // Area data is valid immediately after drawing. Script callbacks
            // may run after render() has invalidated those areas, so retain
            // the measured displacement here rather than reading stale areas.
            let viewport = self.view.view(cx, ids!(body)).area().rect(cx);
            let page = self.view.view(cx, ids!(body.page)).area().rect(cx);
            self.last_scroll = dvec2(0.0, (viewport.pos.y - page.pos.y).max(0.0));
            if let Some(pos) = self.restore_position.take() {
                // The new page's scroll bounds are now known; applying before
                // this draw can clamp a saved offset to a loading placeholder.
                self.view.view(cx, ids!(body)).set_scroll_pos(cx, pos);
                self.restore_frame = cx.new_next_frame();
            }
        }
        step
    }
    fn script_call(&mut self, vm: &mut ScriptVm, method: LiveId, args: ScriptValue) -> ScriptAsyncResult {
        if method == id!(remember_scroll) || method == id!(restore_scroll) {
            let key = args.as_object().and_then(|args| {
                let trap = vm.bx.threads.cur().trap.pass();
                let value = vm.bx.heap.vec_value(args, 0, trap);
                vm.bx.heap.cast_to_owned_string(value, "mini app scroll route")
            });
            if let Some(key) = key {
                vm.with_cx_mut(|cx| {
                    if method == id!(remember_scroll) {
                        self.scroll_positions.insert(key, self.last_scroll);
                    } else {
                        self.restore_position = self.scroll_positions.get(&key).copied();
                        self.view.redraw(cx);
                    }
                });
            }
            return ScriptAsyncResult::Return(NIL);
        }
        if method == id!(reset_scroll) {
            self.restore_position = None;
            self.last_scroll = dvec2(0.0, 0.0);
            vm.with_cx_mut(|cx| {
                self.view.view(cx, ids!(body)).set_scroll_pos(cx, dvec2(0.0, 0.0));
                self.view.redraw(cx);
            });
            return ScriptAsyncResult::Return(NIL);
        }
        ScriptAsyncResult::MethodNotFound
    }
}
script_mod! {
        use mod.prelude.widgets.*
        mod.widgets.RinxNumberInput = set_type_default() do #(MiniAppNumberInput::register_widget(vm)) {
            width: Fill height: Fit flow: Right spacing: 8 align: Align{y:0.5}
            decrease := mod.widgets.RinxButton{width:max(44,mod.widgets.RINX_CONTROL_HEIGHT) height:max(44,mod.widgets.RINX_CONTROL_HEIGHT) text:"−" grab_key_focus:true}
            input := mod.widgets.RinxInput{width:Fill height:max(44,mod.widgets.RINX_CONTROL_HEIGHT)}
            increase := mod.widgets.RinxButton{width:max(44,mod.widgets.RINX_CONTROL_HEIGHT) height:max(44,mod.widgets.RINX_CONTROL_HEIGHT) text:"+" grab_key_focus:true}
        }
        mod.widgets.RinxAmountSlider = set_type_default() do #(MiniAppAmountSlider::register_widget(vm)) {
            ..mod.widgets.Slider
            width: Fill height: 44 margin: 0 padding: 0
            min: 0 max: 1 step: 0 default: 0
            label_walk: Walk{width: 0 height: 0}
            text_input +: {width: 0 height: 0 is_read_only: true}
            draw_bg +: {
                label_size: 0
                track_color: uniform(mod.widgets.RINX_BORDER)
                fill_color: uniform(mod.widgets.RINX_ACCENT)
                knob_color: uniform(mod.widgets.RINX_FIELD)
                pixel: fn() {
                    let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                    let x = 12.0 + (self.rect_size.x - 24.0) * self.slide_pos
                    let y = self.rect_size.y * 0.5
                    sdf.box(12.0, y - 3.0, self.rect_size.x - 24.0, 6.0, 3.0)
                    sdf.fill(self.track_color)
                    sdf.box(12.0, y - 3.0, max(0.0, x - 12.0), 6.0, 3.0)
                    sdf.fill(self.fill_color.mix(self.track_color, self.disabled))
                    sdf.circle(x, y, 9.0)
                    sdf.fill_keep(self.knob_color)
                    sdf.stroke(self.fill_color.mix(self.track_color, self.disabled), 2.0 + self.focus)
                    return sdf.result
                }
            }
        }
        mod.widgets.RinxKeyButton = set_type_default() do #(MiniAppButton::register_widget(vm)) {
            ..mod.widgets.RinxButton
        }
        mod.widgets.RinxKeyPrimaryButton = mod.widgets.RinxKeyButton {
            draw_bg +: {color: mod.widgets.RINX_ACCENT color_focus: mod.widgets.RINX_ACCENT color_hover: mod.widgets.RINX_ACCENT_HOVER color_down: mod.widgets.RINX_ACCENT_DOWN
                border_color: mod.widgets.RINX_ACCENT border_color_hover: mod.widgets.RINX_ACCENT_HOVER border_color_down: mod.widgets.RINX_ACCENT_DOWN border_color_focus: mod.widgets.RINX_ON_ACCENT}
            draw_text +: {color: mod.widgets.RINX_ON_ACCENT color_hover: mod.widgets.RINX_ON_ACCENT color_down: mod.widgets.RINX_ON_ACCENT color_focus: mod.widgets.RINX_ON_ACCENT}
            draw_icon +: {color: mod.widgets.RINX_ON_ACCENT}
        }
mod.widgets.RinxToggle = mod.widgets.RinxKeyButton{height:max(44,mod.widgets.RINX_CONTROL_HEIGHT) padding:Inset{left:60 right:12 top:10 bottom:10}
    draw_bg +: {selected:instance(0.0) accent:uniform(mod.widgets.RINX_ACCENT) muted:uniform(mod.widgets.RINX_MUTED) knob:uniform(mod.widgets.RINX_ON_ACCENT)
        pixel:fn(){
            let sdf=Sdf2d.viewport(self.pos * self.rect_size)
            let y=self.rect_size.y * 0.5
            sdf.box(1.0,1.0,self.rect_size.x-2.0,self.rect_size.y-2.0,6.0)
            sdf.fill_keep(self.color.mix(self.color_hover,self.hover))
            sdf.stroke(self.accent, self.focus)
            sdf.box(12.0,y-10.0,34.0,20.0,10.0) sdf.fill(self.muted.mix(self.accent,self.selected).mix(self.muted,self.disabled))
            sdf.circle(22.0+14.0*self.selected,y,7.0) sdf.fill(self.knob)
            return sdf.result
        }
    }
}
mod.widgets.RinxProgress = mod.widgets.RinxAmountSlider{height:8 enabled:false
    draw_bg +: {pixel:fn(){
        let sdf=Sdf2d.viewport(self.pos * self.rect_size)
        sdf.box(0.0,0.0,self.rect_size.x,self.rect_size.y,4.0) sdf.fill(self.track_color)
        sdf.box(0.0,0.0,self.rect_size.x * self.slide_pos,self.rect_size.y,4.0) sdf.fill(self.fill_color)
        return sdf.result
    }}
}
        mod.widgets.RinxPageScroll = set_type_default() do #(MiniAppPageScroll::register_widget(vm)) {
            width: Fill height: Fill
            body := ScrollYView {width: Fill height: Fill flow: Down
                page := View {width: Fill height: Fit flow: Down spacing: 16}
            }
        }
        mod.rinx_icons = {
            hagency: crate_resource("self://resources/icons/hagency-network.svg")
            projects: crate_resource("self://resources/icons/hagency-project.svg")
            tasks: crate_resource("self://resources/icons/hagency-tasks.svg")
            activity: crate_resource("self://resources/icons/hagency-activity.svg")
            home: crate_resource("self://resources/icons/home.svg")
            resources: crate_resource("self://resources/icons/squares_filled.svg")
            connection: crate_resource("self://resources/icons/link.svg")
            inbox: crate_resource("self://resources/icons/shield.svg")
            refresh: crate_resource("self://resources/icons/article_refresh.svg")
            search: crate_resource("self://resources/icons/search.svg")
            add: crate_resource("self://resources/icons/add.svg")
            check: crate_resource("self://resources/icons/checkmark.svg")
            close: crate_resource("self://resources/icons/close.svg")
            back: crate_resource("self://resources/icons/arrow_back.svg")
            person: crate_resource("self://resources/icons/person.svg")
            device: crate_resource("self://resources/icons/device.svg")
            expand: crate_resource("self://resources/icons/chevron_down.svg")
            collapse: crate_resource("self://resources/icons/chevron_up.svg")
        }
}
