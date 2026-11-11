//! Account-scoped device inventory and exact remote revocation. Verification is not revocation.
use crate::matrix_context::SessionPin;
use makepad_widgets::*;
use matrix_sdk::{
    Client,
    ruma::{
        OwnedDeviceId,
        api::client::{
            device::delete_devices,
            discovery::get_authorization_server_metadata::v1::{
                AccountManagementAction, AccountManagementActionData,
            },
        },
    },
};
use std::time::Duration;
#[derive(Clone, Debug)]
struct DeviceRow {
    id: OwnedDeviceId,
    name: String,
    ip: String,
    seen: String,
    trust: &'static str,
    fingerprint: String,
}
#[derive(Clone, Debug)]
enum Operation {
    Refresh,
    Revoke(OwnedDeviceId, SessionPin),
    Management,
}
impl Operation {
    fn authorized_for(&self, current: &SessionPin) -> bool {
        !matches!(self, Self::Revoke(_, confirmed) if confirmed != current)
    }
}
#[derive(Debug)]
struct ResultAction {
    pin: SessionPin,
    nonce: u64,
    result: Result<Outcome, &'static str>,
}
#[derive(Debug)]
enum Outcome {
    Listed(Vec<DeviceRow>, Option<OwnedDeviceId>),
    Revoked(Vec<DeviceRow>, Option<OwnedDeviceId>),
    Management(String),
}
fn safe_target(
    current: Option<&matrix_sdk::ruma::DeviceId>,
    target: &matrix_sdk::ruma::DeviceId,
) -> bool {
    current.is_some_and(|current| current != target)
}
fn text(value: Option<&str>) -> String {
    value
        .unwrap_or("—")
        .chars()
        .filter(|c| !c.is_control())
        .take(256)
        .collect()
}
async fn inventory(client: &Client) -> Result<Vec<DeviceRow>, &'static str> {
    let response = client
        .devices()
        .await
        .map_err(|_| "Device list could not be read. Refresh to retry.")?;
    if response.devices.len() > 4096 {
        return Err("Device list is too large. Use account management.");
    }
    let user = client.user_id().ok_or("Account session changed.")?;
    let mut rows = Vec::with_capacity(response.devices.len());
    for device in response.devices {
        // Local crypto-store lookup only; absent crypto information is not an unverified verdict.
        let crypto = client
            .encryption()
            .get_device(user, &device.device_id)
            .await;
        let fingerprint = crypto
            .as_ref()
            .ok()
            .and_then(|d| d.as_ref())
            .and_then(|d| d.ed25519_key())
            .map(|k| k.to_base64())
            .unwrap_or_else(|| "—".into());
        let trust = match crypto {
            Ok(Some(d)) if d.is_verified_with_cross_signing() => "Cross-signed device",
            Ok(Some(d)) if d.is_verified() => "Locally verified device",
            Ok(Some(_)) => "Not verified",
            _ => "Verification information unavailable",
        };
        rows.push(DeviceRow {
            id: device.device_id,
            name: text(device.display_name.as_deref()),
            ip: text(device.last_seen_ip.as_deref()),
            seen: device
                .last_seen_ts
                .map(|ts| u64::from(ts.0).to_string())
                .unwrap_or_else(|| "—".into()),
            trust,
            fingerprint,
        });
    }
    rows.sort_by(|a, b| {
        (Some(a.id.as_str()) != client.device_id().map(|d| d.as_str()))
            .cmp(&(Some(b.id.as_str()) != client.device_id().map(|d| d.as_str())))
            .then(a.id.cmp(&b.id))
    });
    Ok(rows)
}
async fn revoke(client: &Client, target: &OwnedDeviceId) -> Result<(), &'static str> {
    if !safe_target(client.device_id(), target) {
        return Err("This device cannot be revoked here. Use sign out for the current device.");
    }
    // This POST is never automatically replayed. An error can mean an unknown outcome.
    client.send(delete_devices::v3::Request::new(vec![target.clone()]))
        .with_request_config(matrix_sdk::config::RequestConfig::new().retry_limit(0).timeout(Duration::from_secs(20)))
        .await.map_err(|_| "Revocation is not confirmed. Refresh the list; if authorization is required, open account management.")?;
    Ok(())
}
async fn operate(client: Client, pin: &SessionPin, op: Operation) -> Result<Outcome, &'static str> {
    if !op.authorized_for(pin) || !pin.is_current() {
        return Err("Account session changed.");
    }
    let current = client.device_id().map(ToOwned::to_owned);
    match op {
        Operation::Refresh => Ok(Outcome::Listed(inventory(&client).await?, current)),
        Operation::Revoke(target, _) => {
            revoke(&client, &target).await?;
            if !pin.is_current() {
                return Err("Account session changed.");
            }
            let rows = inventory(&client).await?;
            if rows.iter().any(|r| r.id == target) {
                return Err(
                    "Revocation is not confirmed. Refresh the list; if authorization is required, open account management.",
                );
            }
            Ok(Outcome::Revoked(rows, current))
        }
        Operation::Management => {
            let md = client.oauth().cached_server_metadata().await.map_err(
                |_| "Account management is unavailable. Contact your homeserver administrator.",
            )?;
            let url = if md
                .is_account_management_action_supported(&AccountManagementAction::DevicesList)
            {
                md.account_management_url_with_action(AccountManagementActionData::DevicesList)
            } else {
                md.account_management_uri
            };
            let url = url
                .filter(|u| {
                    u.scheme() == "https" && u.username().is_empty() && u.password().is_none()
                })
                .ok_or(
                    "Account management is unavailable. Contact your homeserver administrator.",
                )?;
            Ok(Outcome::Management(url.to_string()))
        }
    }
}
script_mod! {
 use mod.prelude.widgets.*
 use mod.widgets.*
 mod.widgets.DeviceSessions = #(DeviceSessions::register_widget(vm)) {
  width: Fill height: Fit flow: Down spacing: 12 padding: 12
  Label {width: Fill height: Fit flow: Flow.Right{wrap:true} text:#(crate::i18n::tr("Devices and sessions")) i18n_text:"Devices and sessions" draw_text.color:mod.widgets.RINX_INK}
  Label {width: Fill height: Fit flow: Flow.Right{wrap:true} text:#(crate::i18n::tr("Verification protects encryption identity. Revoking a session stops future authenticated requests; it cannot erase data already downloaded.")) i18n_text:"Verification protects encryption identity. Revoking a session stops future authenticated requests; it cannot erase data already downloaded." draw_text.color:mod.widgets.RINX_MUTED}
  devices := DropDown {width: Fill labels:[]}
  details := Label {width: Fill height: Fit flow: Flow.Right{wrap:true} draw_text.color:mod.widgets.RINX_INK}
  notice := Label {width: Fill height: Fit flow: Flow.Right{wrap:true} draw_text.color:mod.widgets.RINX_MUTED}
  View {width: Fill height: Fit flow: Flow.Right{wrap:true} spacing: 8
   refresh := Button {text:#(crate::i18n::tr("Refresh status")) i18n_text:"Refresh status"}
   revoke := Button {text:#(crate::i18n::tr("Revoke selected device")) i18n_text:"Revoke selected device"}
   manage := Button {text:#(crate::i18n::tr("Open account management")) i18n_text:"Open account management"}
  }
  confirm_text := Label {visible:false width: Fill height: Fit flow: Flow.Right{wrap:true} draw_text.color:mod.widgets.RINX_INK}
  confirm := Button {visible:false text:#(crate::i18n::tr("Confirm device revocation")) i18n_text:"Confirm device revocation"}
  cancel := Button {visible:false text:#(crate::i18n::tr("Cancel")) i18n_text:"Cancel"}
 }
}
#[derive(Script, ScriptHook, Widget)]
pub struct DeviceSessions {
    #[deref]
    view: View,
    #[rust]
    pin: Option<SessionPin>,
    #[rust]
    rows: Vec<DeviceRow>,
    #[rust]
    current: Option<OwnedDeviceId>,
    #[rust]
    selected: usize,
    #[rust]
    confirmation: Option<OwnedDeviceId>,
    #[rust]
    nonce: u64,
    #[rust]
    task: Option<tokio::task::JoinHandle<()>>,
    #[rust]
    notice: String,
}
impl DeviceSessions {
    fn clear(&mut self, cx: &mut Cx) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.nonce = self.nonce.wrapping_add(1);
        self.pin = None;
        self.rows.clear();
        self.current = None;
        self.confirmation = None;
        self.selected = 0;
        self.notice.clear();
        self.render(cx);
    }
    fn start(&mut self, cx: &mut Cx, op: Operation) {
        let Some(client) = crate::sliding_sync::get_client() else {
            self.clear(cx);
            return;
        };
        let Some(pin) = SessionPin::capture(&client) else {
            self.clear(cx);
            return;
        };
        if !op.authorized_for(&pin) {
            self.clear(cx);
            self.notice = crate::i18n::tr("Account session changed.").into();
            self.render(cx);
            return;
        }
        if self.pin.as_ref() != Some(&pin) {
            self.clear(cx);
            self.pin = Some(pin.clone());
        }
        if self.task.is_some() {
            return;
        }
        self.confirmation = None;
        self.nonce = self.nonce.wrapping_add(1);
        let nonce = self.nonce;
        self.notice = crate::i18n::tr("Checking devices…").into();
        self.task = Some(crate::account_session::spawn(async move {
            let result = tokio::time::timeout(Duration::from_secs(30), operate(client, &pin, op))
                .await
                .unwrap_or(Err(
                    "Device request timed out. Refresh before taking another action.",
                ));
            if pin.is_current() {
                Cx::post_action(ResultAction { pin, nonce, result });
            }
        }));
        self.render(cx);
    }
    fn render(&mut self, cx: &mut Cx) {
        self.view.drop_down(cx, ids!(devices)).set_labels(
            cx,
            self.rows
                .iter()
                .map(|r| {
                    format!(
                        "{} · {}{}",
                        text(Some(r.id.as_str())),
                        r.name,
                        if Some(&r.id) == self.current.as_ref() {
                            format!(" · {}", crate::i18n::tr("Current device"))
                        } else {
                            String::new()
                        }
                    )
                })
                .collect(),
        );
        self.view
            .drop_down(cx, ids!(devices))
            .set_selected_item(cx, self.selected);
        let detail = self
            .rows
            .get(self.selected)
            .map(|r| {
                format!(
                    "{}\n{}: {}\n{}: {}\n{}: {}\n{}: {}",
                    text(Some(r.id.as_str())),
                    crate::i18n::tr("Last seen IP"),
                    r.ip,
                    crate::i18n::tr("Last seen (Unix milliseconds)"),
                    r.seen,
                    crate::i18n::tr("Device verification"),
                    crate::i18n::tr(r.trust),
                    crate::i18n::tr("Ed25519 fingerprint"),
                    r.fingerprint
                )
            })
            .unwrap_or_default();
        self.view.label(cx, ids!(details)).set_text(cx, &detail);
        self.view.label(cx, ids!(notice)).set_text(cx, &self.notice);
        let confirm = self.confirmation.is_some();
        for id in [ids!(confirm), ids!(cancel), ids!(confirm_text)] {
            self.view.widget(cx, id).set_visible(cx, confirm);
        }
        if let Some(id) = &self.confirmation {
            self.view.label(cx, ids!(confirm_text)).set_text(
                cx,
                &format!(
                    "{}: {}",
                    crate::i18n::tr("Revoke selected device"),
                    text(Some(id.as_str()))
                ),
            );
        }
        let busy = self.task.is_some();
        for id in [ids!(refresh), ids!(manage), ids!(confirm)] {
            self.view.button(cx, id).set_enabled(cx, !busy);
        }
        self.view.button(cx, ids!(revoke)).set_enabled(
            cx,
            !busy
                && self
                    .rows
                    .get(self.selected)
                    .is_some_and(|r| safe_target(self.current.as_deref(), &r.id)),
        );
        self.view.redraw(cx);
    }
}
impl Widget for DeviceSessions {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.pin.as_ref().is_some_and(|p| !p.is_current()) {
            self.clear(cx);
        }
        self.view.handle_event(cx, event, scope);
        if let Event::Actions(actions) = event {
            for action in actions {
                if let Some(done) = action.downcast_ref::<ResultAction>() {
                    if self.task.is_none()
                        || self.pin.as_ref() != Some(&done.pin)
                        || self.nonce != done.nonce
                        || !done.pin.is_current()
                    {
                        continue;
                    }
                    self.task.take();
                    self.notice.clear();
                    match &done.result {
                        Ok(Outcome::Listed(rows, current))
                        | Ok(Outcome::Revoked(rows, current)) => {
                            self.rows = rows.clone();
                            self.current = current.clone();
                            self.selected = 0;
                            self.notice = crate::i18n::tr(
                                if matches!(&done.result, Ok(Outcome::Revoked(..))) {
                                    "Server confirmed the device was removed."
                                } else {
                                    "Device list refreshed."
                                },
                            )
                            .into();
                        }
                        Ok(Outcome::Management(url)) => {
                            if robius_open::Uri::new(url).open().is_err() {
                                self.notice =
                                    crate::i18n::tr("Could not open account management.").into();
                            } else {
                                self.notice=crate::i18n::tr("Finish authorization in your browser, then refresh this list. No revocation is assumed.").into();
                            }
                        }
                        Err(e) => self.notice = crate::i18n::tr(e).into(),
                    }
                    self.render(cx);
                }
            }
            if let Some(index) = self.view.drop_down(cx, ids!(devices)).selected(actions) {
                self.selected = index;
                self.confirmation = None;
                self.render(cx);
            }
            if self.view.button(cx, ids!(refresh)).clicked(actions) {
                self.start(cx, Operation::Refresh);
            }
            if self.view.button(cx, ids!(manage)).clicked(actions) {
                self.start(cx, Operation::Management);
            }
            if self.view.button(cx, ids!(revoke)).clicked(actions) && self.task.is_none() {
                self.confirmation = self
                    .rows
                    .get(self.selected)
                    .filter(|r| safe_target(self.current.as_deref(), &r.id))
                    .map(|r| r.id.clone());
                self.render(cx);
            }
            if self.view.button(cx, ids!(cancel)).clicked(actions) {
                self.confirmation = None;
                self.render(cx);
            }
            if self.view.button(cx, ids!(confirm)).clicked(actions)
                && let Some(id) = self.confirmation.take()
                && let Some(pin) = self.pin.clone()
            {
                self.start(cx, Operation::Revoke(id, pin));
            }
        }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
impl Drop for DeviceSessions {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
impl DeviceSessionsRef {
    pub(crate) fn refresh(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.start(cx, Operation::Refresh);
        }
    }
    pub(crate) fn clear(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear(cx);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revoke_requires_a_different_known_current_device() {
        let own = matrix_sdk::ruma::device_id!("CURRENT");
        assert!(!safe_target(None, own));
        assert!(!safe_target(Some(own), own));
        assert!(safe_target(
            Some(own),
            matrix_sdk::ruma::device_id!("OTHER")
        ));
    }
    #[test]
    fn metadata_is_bounded_and_control_characters_are_removed() {
        assert_eq!(text(Some("a\nb\r\t")), "ab");
        assert_eq!(text(Some(&"x".repeat(1000))).len(), 256);
    }
    #[tokio::test]
    async fn real_sdk_revoke_posts_only_the_selected_device_and_preserves_authorization_errors() {
        use serde_json::json;
        use wiremock::{
            Mock, MockServer, ResponseTemplate,
            matchers::{body_json, header, method, path},
        };
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/delete_devices"))
            .and(header("authorization", "Bearer fixture"))
            .and(body_json(json!({"devices":["OTHER"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::builder()
            .homeserver_url(server.uri())
            .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_13])
            .build()
            .await
            .unwrap();
        client.matrix_auth().restore_session(serde_json::from_value(json!({"access_token":"fixture","user_id":"@alice:example.org","device_id":"CURRENT"})).unwrap(),Default::default()).await.unwrap();
        assert!(revoke(&client, &"CURRENT".into()).await.is_err());
        revoke(&client, &"OTHER".into()).await.unwrap();
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/delete_devices"))
            .and(body_json(json!({"devices":["CHALLENGE"]})))
            .respond_with(
                ResponseTemplate::new(401).set_body_json(
                    json!({"session":"private-uiaa","flows":[{"stages":["m.oauth"]}]}),
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = revoke(&client, &"CHALLENGE".into()).await.unwrap_err();
        assert!(error.contains("not confirmed"));
        assert!(!error.contains("private-uiaa"));
    }
    #[tokio::test]
    async fn revocation_fixture_rejects_the_target_token_but_keeps_current_token_valid() {
        use serde_json::json;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use wiremock::{
            Mock, MockServer, ResponseTemplate,
            matchers::{body_json, method, path},
        };
        let server = MockServer::start().await;
        let revoked = Arc::new(AtomicBool::new(false));
        let effect = revoked.clone();
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/delete_devices"))
            .and(body_json(json!({"devices":["OTHER"]})))
            .respond_with(move |_: &wiremock::Request| {
                effect.store(true, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({}))
            })
            .expect(1)
            .mount(&server)
            .await;
        let state = revoked.clone();
        Mock::given(method("GET")).and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(move |req: &wiremock::Request| {
                let target=req.headers.get("authorization").is_some_and(|v|v=="Bearer target-token");
                if target&&state.load(Ordering::SeqCst){ResponseTemplate::new(401).set_body_json(json!({"errcode":"M_UNKNOWN_TOKEN","error":"revoked"}))}
                else {ResponseTemplate::new(200).set_body_json(json!({"user_id":"@alice:example.org","device_id":if target{"OTHER"}else{"CURRENT"}}))}
            }).mount(&server).await;
        async fn client(url: String, token: &str, id: &str) -> Client {
            let c = Client::builder()
                .homeserver_url(url)
                .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_13])
                .build()
                .await
                .unwrap();
            c.matrix_auth()
                .restore_session(
                    serde_json::from_value(
                        json!({"access_token":token,"user_id":"@alice:example.org","device_id":id}),
                    )
                    .unwrap(),
                    Default::default(),
                )
                .await
                .unwrap();
            c
        }
        let current = client(server.uri(), "current-token", "CURRENT").await;
        let other = client(server.uri(), "target-token", "OTHER").await;
        let who = || matrix_sdk::ruma::api::client::account::whoami::v3::Request::new();
        other.send(who()).await.unwrap();
        revoke(&current, &"OTHER".into()).await.unwrap();
        let rejected = other.send(who()).await.unwrap_err();
        assert_eq!(
            rejected.as_client_api_error().unwrap().status_code.as_u16(),
            401
        );
        current.send(who()).await.unwrap();
    }
    #[tokio::test]
    async fn an_old_confirmation_cannot_cross_a_same_user_and_device_relogin() {
        use serde_json::json;
        async fn client() -> Client {
            let c = Client::builder()
                .homeserver_url("http://127.0.0.1:9")
                .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_13])
                .build()
                .await
                .unwrap();
            c.matrix_auth().restore_session(serde_json::from_value(json!({"access_token":"fixture","user_id":"@alice:example.org","device_id":"CURRENT"})).unwrap(),Default::default()).await.unwrap();
            c
        }
        let first = client().await;
        let replacement = client().await;
        let original = SessionPin::capture(&first).unwrap();
        let fresh = SessionPin::capture(&replacement).unwrap();
        assert_ne!(original, fresh);
        let confirmed = Operation::Revoke("OTHER".into(), original.clone());
        assert!(confirmed.authorized_for(&original));
        assert!(!confirmed.authorized_for(&fresh));
        assert!(Operation::Refresh.authorized_for(&fresh));
    }
}
