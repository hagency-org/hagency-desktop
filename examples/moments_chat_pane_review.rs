//! Offline review of production Moments routing. Never signs in or publishes.
//! F1: chat, F2: Discover, F3: close Moments, F4: tabs, F5: inspect, F6: reload.
pub use makepad_widgets;
use makepad_widgets::*;
use rinx::app::App as RinxApp;

app_main!(App);

fn review_size() -> Vec2d {
    dvec2(std::env::var("MOMENTS_REVIEW_WIDTH").ok().and_then(|s| s.parse().ok()).unwrap_or(1280.0), 900.0)
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    startup() do #(App::script_component(vm)) {
        app: #(RinxApp::script_component(vm)) {
            ui: Root {main_window := Window {
                window.inner_size: #(review_size())
                window.title: "Hagency · Moments navigation review"
                body +: {content := RinxContent {}}
            }}
        }
    }
}

#[derive(Script, ScriptHook)]
struct App { #[live] app: RinxApp }

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        makepad_widgets::theme_mod(vm);
        rinx::theme::init_standalone(vm);
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        article_makepad::apple_fonts::install(vm);
        makepad_widgets::widgets_mod(vm);
        desktop_style::apply_widgets(vm);
        rinx::app::register_widgets(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        let ui = self.app.content();
        if matches!(event, Event::Startup) {
            self.app.prepare_offline_review();
            ui.view(cx, ids!(home_screen_view)).set_visible(cx, true);
            ui.view(cx, ids!(login_screen_view)).set_visible(cx, false);
        }
        let command = match event {
            Event::Custom(command) => Some(command.as_str()),
            Event::KeyDown(key) => match key.key_code {
                KeyCode::F1 => Some("moments:chat"),
                KeyCode::F2 => Some("moments:discover"),
                KeyCode::F3 => Some("moments:close"),
                KeyCode::F4 => Some("moments:tabbed"),
                KeyCode::F5 => Some("moments:inspect"),
                KeyCode::F6 => Some("moments:reload"),
                _ => None,
            },
            _ => None,
        };
        if let Some(command) = command {
            use rinx::moments::ui::MomentsAction;
            match command {
                "moments:close" => cx.action(MomentsAction::Close),
                "moments:open" => cx.action(MomentsAction::Open {author: None}),
                "moments:discover" => cx.action(rinx::home::navigation_tab_bar::NavigationBarAction::GoToTab(
                    rinx::home::navigation_tab_bar::SelectedTab::Discover)),
                "moments:tabbed" => cx.action(rinx::settings::app_preferences::AppPreferencesAction::TabbedChatsChanged(true)),
                "moments:reload" => cx.action(rinx::home::main_desktop_ui::MainDesktopUiAction::LoadDockFromAppState),
                "moments:chat" => {
                    let room = rinx::app::SelectedRoom::JoinedRoom {
                        room_name_id: rinx::utils::RoomNameId::new(matrix_sdk::RoomDisplayName::Named("Review chat".into()),
                            "!moments-review:example.invalid".try_into().unwrap()),
                    };
                    cx.widget_action(ui.widget(cx, ids!(rooms_list)).widget_uid(),
                        rinx::home::rooms_list::RoomsListAction::Selected(room));
                }
                "moments:inspect" => {
                    let dock = ui.dock(cx, ids!(dock));
                    let state = dock.clone_state().unwrap();
                    let moments = dock.item(id!(moments_tab));
                    let result = serde_json::json!({
                        "windows": cx.windows.len(),
                        "moments_modal_open": ui.modal(cx, ids!(moments_modal)).is_open(),
                        "moments_in_dock": !moments.is_empty(),
                        "moments_bounds": format!("{:?}", moments.area().rect(cx)),
                        "dock_state": format!("{state:?}"),
                    });
                    std::fs::write(rinx::app_data_dir().join("moments-inspection.json"), result.to_string()).unwrap();
                }
                _ => {}
            }
        }
        self.app.handle_event(cx, event);
    }
}
