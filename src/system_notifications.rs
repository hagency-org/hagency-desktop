//! Privacy-preserving, opt-in system notifications. OS acceptance is not proof of display.
use crate::matrix_context::SessionPin;
use makepad_widgets::Cx;
use matrix_sdk::{
    Client,
    ruma::{
        events::{room::encrypted::SyncRoomEncryptedEvent, room::message::SyncRoomMessageEvent},
        push::Action,
    },
};
use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
static STATUS: Mutex<&'static str> = Mutex::new(
    "Notifications are off until you enable them. Use the test button to check system permission.",
);
pub(crate) fn status() -> &'static str {
    *STATUS.lock().unwrap()
}
static ENABLED: AtomicBool = AtomicBool::new(false);
static FOREGROUND: AtomicBool = AtomicBool::new(true);
static NATIVE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
#[derive(Default)]
struct Recent {
    pin: Option<SessionPin>,
    ids: VecDeque<String>,
    last: Option<Instant>,
}
static RECENT: Mutex<Recent> = Mutex::new(Recent {
    pin: None,
    ids: VecDeque::new(),
    last: None,
});
#[derive(Clone, Debug)]
pub(crate) struct NotificationStatus {
    pub pin: SessionPin,
    pub text: &'static str,
}
pub(crate) fn set_enabled(value: bool) {
    let changed = ENABLED.swap(value, Ordering::SeqCst) != value;
    #[cfg(target_os = "macos")]
    if !value {
        native::clear_pending();
    }
    if changed {
        *STATUS.lock().unwrap() = if value {
            "Notifications enabled. Use the test button to check system permission."
        } else {
            "Notifications are off until you enable them. Use the test button to check system permission."
        };
    }
}
pub(crate) fn foreground(value: bool) {
    FOREGROUND.store(value, Ordering::SeqCst);
}
/// Window focus hints can remain true after a background launch or synthetic
/// accessibility focus. On macOS use the thread-safe, OS-owned app activity.
fn foreground_now() -> bool {
    #[cfg(target_os = "macos")]
    {
        objc2::rc::autoreleasepool(|_| {
            objc2_app_kit::NSRunningApplication::currentApplication().isActive()
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        FOREGROUND.load(Ordering::SeqCst)
    }
}
pub(crate) fn clear() {
    set_enabled(false);
    *STATUS.lock().unwrap() = "Notifications are off until you enable them. Use the test button to check system permission.";
    let mut s = RECENT.lock().unwrap();
    *s = Recent::default();
}
fn eligible(
    enabled: bool,
    foreground: bool,
    own: bool,
    notify: bool,
    ts: u64,
    start: u64,
    now: u64,
) -> bool {
    enabled && !foreground && !own && notify && ts >= start && ts <= now.saturating_add(300_000)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
fn event(pin: &SessionPin, id: &str, ts: u64, own: bool, notify: bool, start: u64) {
    if !pin.is_current()
        || !eligible(
            ENABLED.load(Ordering::SeqCst),
            foreground_now(),
            own,
            notify,
            ts,
            start,
            now(),
        )
    {
        return;
    }
    {
        let mut s = RECENT.lock().unwrap();
        if s.pin.as_ref() != Some(pin) {
            *s = Recent {
                pin: Some(pin.clone()),
                ..Default::default()
            };
        }
        if s.ids.iter().any(|v| v == id) {
            return;
        }
        if s.ids.len() == 64 {
            s.ids.pop_front();
        }
        s.ids.push_back(id.into());
        if s.last.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
            return;
        }
        s.last = Some(Instant::now());
    }
    submit(pin.clone(), false);
}
fn submit(pin: SessionPin, authorize: bool) {
    let Ok(permit) = NATIVE.try_acquire() else {
        return;
    };
    crate::account_session::spawn(async move {
        // The permit belongs to the actual blocking OS operation even after async cancellation.
        let _=tokio::task::spawn_blocking(move||{let _permit=permit;
   if !pin.is_current()||!ENABLED.load(Ordering::SeqCst){return;}
   let result=native::send(&pin,authorize);
   if !pin.is_current() || !ENABLED.load(Ordering::SeqCst) {
       #[cfg(target_os = "macos")]
       native::clear_pending();
       return;
   }
   if pin.is_current(){let text=match result{Ok(()) if authorize && cfg!(target_os="macos")=>"Test notification scheduled for five seconds. Switch to another app; display still depends on system settings and Focus mode.",Ok(())=>"Notification request accepted by the system. Display depends on system settings and Focus mode.",Err(e)=>e};*STATUS.lock().unwrap()=text;Cx::post_action(NotificationStatus{pin,text});}
  }).await;
    });
}
pub(crate) fn request_permission_and_test() {
    let Some(client) = crate::sliding_sync::get_client() else {
        return;
    };
    let Some(pin) = SessionPin::capture(&client) else {
        return;
    };
    submit(pin, true);
}
pub(crate) fn show_activity() {
    let Some(client) = crate::sliding_sync::get_client() else {
        return;
    };
    let Some(pin) = SessionPin::capture(&client) else {
        return;
    };
    event(
        &pin,
        &format!("agent-question-{}", now() / 2000),
        now(),
        false,
        true,
        0,
    );
}
pub(crate) fn attach(client: Client) -> tokio::task::JoinHandle<()> {
    crate::account_session::spawn(async move {
        let Some(pin) = SessionPin::capture(&client) else {
            return;
        };
        let start = now();
        let p = pin.clone();
        let own = client.user_id().map(ToOwned::to_owned);
        let h = client.add_event_handler(move |ev: SyncRoomMessageEvent, actions: Vec<Action>| {
            let pin = p.clone();
            let own = own.clone();
            async move {
                if let Some(ev) = ev.as_original() {
                    event(
                        &pin,
                        ev.event_id.as_str(),
                        ev.origin_server_ts.0.into(),
                        Some(&ev.sender) == own.as_ref(),
                        actions.iter().any(Action::should_notify),
                        start,
                    );
                }
            }
        });
        let p = pin;
        let own = client.user_id().map(ToOwned::to_owned);
        let encrypted =
            client.add_event_handler(move |ev: SyncRoomEncryptedEvent, actions: Vec<Action>| {
                let pin = p.clone();
                let own = own.clone();
                async move {
                    if let Some(ev) = ev.as_original() {
                        event(
                            &pin,
                            ev.event_id.as_str(),
                            ev.origin_server_ts.0.into(),
                            Some(&ev.sender) == own.as_ref(),
                            actions.iter().any(Action::should_notify),
                            start,
                        );
                    }
                }
            });
        let _guards = (
            client.event_handler_drop_guard(h),
            client.event_handler_drop_guard(encrypted),
        );
        std::future::pending::<()>().await;
    })
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use block2::RcBlock;
    use objc2_foundation::{NSArray, NSBundle, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent,
        UNNotificationRequest, UNNotificationSettings, UNNotificationTrigger,
        UNTimeIntervalNotificationTrigger, UNUserNotificationCenter,
    };
    const SETTINGS: &str = "Notifications are unavailable or denied. Enable Hagency notifications in system settings and test again.";
    static DID_SUBMIT: AtomicBool = AtomicBool::new(false);
    static CENTER_FAILED: AtomicBool = AtomicBool::new(false);
    const IDENTITY: &str = "System notification service rejected this app identity. Restart the installed app, then test again.";
    pub(super) fn is_app_executable(
        bundle: &std::path::Path,
        executable: &std::path::Path,
    ) -> bool {
        bundle.extension().is_some_and(|ext| ext == "app")
            && executable
                .parent()
                .is_some_and(|dir| dir == bundle.join("Contents/MacOS"))
    }
    pub(super) fn clear_pending() {
        // Ordinary startup/logout must not initialize UserNotifications for a
        // process which never submitted anything (notably embedded/test hosts).
        if !DID_SUBMIT.load(Ordering::SeqCst) {
            return;
        }
        let Ok(center) = center() else {
            return;
        };
        let id = NSString::from_str("hagency-new-activity");
        center.removePendingNotificationRequestsWithIdentifiers(&NSArray::from_slice(&[&*id]));
        DID_SUBMIT.store(false, Ordering::SeqCst);
    }
    pub(super) fn unbundled() -> bool {
        let bundle = NSBundle::mainBundle();
        let path = std::path::PathBuf::from(bundle.bundlePath().to_string());
        bundle.bundleIdentifier().is_none()
            || !path.join("Contents/Info.plist").is_file()
            || !std::env::current_exe().is_ok_and(|exe| is_app_executable(&path, &exe))
    }
    fn center() -> Result<objc2::rc::Retained<UNUserNotificationCenter>, &'static str> {
        if unbundled() {
            return Err(
                "System notifications require the installed Hagency app bundle. This unbundled build cannot request permission.",
            );
        }
        if CENTER_FAILED.load(Ordering::SeqCst) {
            return Err(IDENTITY);
        }
        objc2::exception::catch(UNUserNotificationCenter::currentNotificationCenter).map_err(|_| {
            // A rejected LaunchServices identity can leave Apple's dispatch_once
            // initializer unusable. Do not invoke it again in this process.
            CENTER_FAILED.store(true, Ordering::SeqCst);
            IDENTITY
        })
    }
    pub(super) fn send(pin: &SessionPin, authorize: bool) -> Result<(), &'static str> {
        if unbundled() {
            return Err(
                "System notifications require the installed Hagency app bundle. This unbundled build cannot request permission.",
            );
        }
        let center = center()?;
        if authorize {
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            let callback = RcBlock::new(
                move |granted: objc2::runtime::Bool, _error: *mut objc2_foundation::NSError| {
                    let _ = tx.try_send(granted.as_bool());
                },
            );
            center.requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert,
                &callback,
            );
            if rx.recv_timeout(Duration::from_secs(30)) != Ok(true) {
                return Err(SETTINGS);
            }
        }
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let callback = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
            let status = unsafe { settings.as_ref() }.authorizationStatus();
            let _ = tx.try_send(
                status == UNAuthorizationStatus::Authorized
                    || status == UNAuthorizationStatus::Provisional,
            );
        });
        center.getNotificationSettingsWithCompletionHandler(&callback);
        if rx.recv_timeout(Duration::from_secs(5)) != Ok(true) {
            return Err(SETTINGS);
        }
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str("Hagency"));
        content.setBody(&NSString::from_str(crate::i18n::tr(
            "New activity. Open Hagency to view it.",
        )));
        // The test gives the owner time to leave the foreground. Do not replace
        // an embedding host's global notification delegate merely to show a test.
        let delayed =
            UNTimeIntervalNotificationTrigger::triggerWithTimeInterval_repeats(5.0, false);
        let trigger: Option<&UNNotificationTrigger> = if authorize { Some(&delayed) } else { None };
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str("hagency-new-activity"),
            &content,
            trigger,
        );
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let callback = RcBlock::new(move |error: *mut objc2_foundation::NSError| {
            let _ = tx.try_send(error.is_null());
        });
        {
            let _gate = crate::persistence::SESSION_IO
                .try_lock()
                .map_err(|_| SETTINGS)?;
            if !pin.is_current() || !ENABLED.load(Ordering::SeqCst) {
                return Err(SETTINGS);
            }
            center.addNotificationRequest_withCompletionHandler(&request, Some(&callback));
            DID_SUBMIT.store(true, Ordering::SeqCst);
        }
        if rx.recv_timeout(Duration::from_secs(5)) == Ok(true) {
            Ok(())
        } else {
            Err(SETTINGS)
        }
    }
}
#[cfg(target_os = "linux")]
mod native {
    use super::*;
    pub(super) fn send(pin: &SessionPin, _authorize: bool) -> Result<(), &'static str> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "System notifications could not be initialized.")?;
        runtime.block_on(async{
   tokio::time::timeout(Duration::from_secs(5),async{
    let proxy=ashpd::desktop::notification::NotificationProxy::new().await.map_err(|_|"Desktop notification portal is unavailable. Check your desktop notification settings.")?;
    if !pin.is_current()||!ENABLED.load(Ordering::SeqCst){return Err("Account session changed.");}
    proxy.add_notification("hagency-new-activity",ashpd::desktop::notification::Notification::new("Hagency").body(Some(crate::i18n::tr("New activity. Open Hagency to view it.")))).await.map_err(|_|"Desktop notification portal rejected the request. Check app identity and notification settings.")
   }).await.unwrap_or(Err("System notification request timed out."))
  })
    }
}
#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use windows::{
        Data::Xml::Dom::XmlDocument,
        UI::Notifications::{ToastNotification, ToastNotificationManager},
        core::HSTRING,
    };
    pub(super) fn send(pin: &SessionPin, _authorize: bool) -> Result<(), &'static str> {
        unsafe {
            windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
            )
        }
        .map_err(|_| "System notifications could not be initialized.")?;
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe { windows::Win32::System::WinRT::RoUninitialize() }
            }
        }
        let _apartment = Apartment;
        let document =
            XmlDocument::new().map_err(|_| "System notifications could not be initialized.")?;
        // Static text only: no message, room, sender, URL or access token enters XML.
        document.LoadXml(&HSTRING::from("<toast><visual><binding template=\"ToastGeneric\"><text>Hagency</text><text>New activity. Open Hagency to view it.</text></binding></visual></toast>")).map_err(|_|"System notifications could not be initialized.")?;
        let toast = ToastNotification::CreateToastNotification(&document)
            .map_err(|_| "System notifications could not be initialized.")?;
        toast
            .SetTag(&HSTRING::from("activity"))
            .map_err(|_| "System notifications could not be initialized.")?;
        let notifier=ToastNotificationManager::CreateToastNotifier().map_err(|_|"System notifications require the installed app identity. Check Windows notification settings.")?;
        let _gate = crate::persistence::SESSION_IO
            .try_lock()
            .map_err(|_| "System notifications are busy.")?;
        if !pin.is_current() || !ENABLED.load(Ordering::SeqCst) {
            return Err("Account session changed.");
        }
        notifier.Show(&toast).map_err(|_|"System notification request failed. Check the installed app identity and Windows notification settings.")
    }
}
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
mod native {
    use super::*;
    pub(super) fn send(_: &SessionPin, _: bool) -> Result<(), &'static str> {
        Err("System notifications are unavailable on this platform.")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_opted_in_background_new_nonself_push_events_are_eligible() {
        assert!(eligible(true, false, false, true, 100, 100, 200));
        for (en, fg, own, push, ts) in [
            (false, false, false, true, 100),
            (true, true, false, true, 100),
            (true, false, true, true, 100),
            (true, false, false, false, 100),
            (true, false, false, true, 99),
            (true, false, false, true, 400_000),
        ] {
            assert!(!eligible(en, fg, own, push, ts, 100, 200));
        }
    }
    #[test]
    fn old_preferences_default_to_notifications_disabled() {
        let p: crate::settings::app_preferences::AppPreferences =
            serde_json::from_str("{}").unwrap();
        assert!(!p.system_notifications);
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn unbundled_test_process_never_requests_os_notification_permission() {
        assert!(
            super::native::unbundled(),
            "this Cargo test fixture must run outside an app bundle"
        );
        super::native::clear_pending();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_bundle_identifier_alone_cannot_turn_an_unbundled_executable_into_an_app() {
        use std::path::Path;
        assert!(!super::native::is_app_executable(
            Path::new("/tmp/deps"),
            Path::new("/tmp/deps/rinx-test")
        ));
        assert!(!super::native::is_app_executable(
            Path::new("/tmp/Fake.app"),
            Path::new("/tmp/rinx")
        ));
        assert!(super::native::is_app_executable(
            Path::new("/Applications/Hagency.app"),
            Path::new("/Applications/Hagency.app/Contents/MacOS/rinx")
        ));
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn a_stale_window_focus_hint_does_not_suppress_a_real_background_process() {
        // Cargo's unbundled test process is not an activated GUI application.
        // This reproduces the live failure without granting or posting notifications.
        assert!(!objc2_app_kit::NSRunningApplication::currentApplication().isActive());
        let previous = FOREGROUND.swap(true, Ordering::SeqCst);
        let observed = foreground_now();
        FOREGROUND.store(previous, Ordering::SeqCst);
        assert!(
            !observed,
            "a window hint must not override OS application activity"
        );
        assert!(eligible(true, observed, false, true, 100, 100, 200));
    }
}
