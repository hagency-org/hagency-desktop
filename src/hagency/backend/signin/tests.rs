use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Fixture {
    origin: String,
    refreshes: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Fixture {
    async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}/", listener.local_addr().unwrap());
        let server_origin = origin.clone();
        let refreshes = Arc::new(AtomicUsize::new(0));
        let count = refreshes.clone();
        let server = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut bytes = Vec::new();
                let mut block = [0_u8; 2048];
                let (head, offset) = loop {
                    let n = stream.read(&mut block).await.unwrap();
                    assert_ne!(n, 0);
                    bytes.extend_from_slice(&block[..n]);
                    assert!(bytes.len() < 16384);
                    if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break (String::from_utf8(bytes[..i].to_vec()).unwrap(), i + 4);
                    }
                };
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (key, value) = line.split_once(':')?;
                        key.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                while bytes.len() < offset + length {
                    let n = stream.read(&mut block).await.unwrap();
                    assert_ne!(n, 0);
                    bytes.extend_from_slice(&block[..n]);
                    assert!(bytes.len() < 16384);
                }
                let path = head
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap();
                let mut status = 200;
                let value = if path.ends_with("/auth_metadata") {
                    json!({
                        "issuer":format!("{server_origin}auth/"),
                        "authorization_endpoint":format!("{server_origin}auth/authorize"),
                        "token_endpoint":format!("{server_origin}auth/token"),
                        "revocation_endpoint":format!("{server_origin}auth/revoke"),
                        "response_types_supported":["code"],
                        "grant_types_supported":["authorization_code","refresh_token"],
                        "code_challenge_methods_supported":["S256"],
                        "token_endpoint_auth_methods_supported":["none"]
                    })
                } else if path == "/auth/token" {
                    let body: std::collections::HashMap<String, String> =
                        url::form_urlencoded::parse(&bytes[offset..])
                            .into_owned()
                            .collect();
                    assert_eq!(body["grant_type"], "refresh_token");
                    let n = count.fetch_add(1, Ordering::AcqRel) + 1;
                    json!({"access_token":format!("synthetic-access-{n}"),
                        "refresh_token":format!("synthetic-refresh-{n}"),
                        "token_type":"Bearer","expires_in":3600})
                } else if path.ends_with("/versions") {
                    json!({"versions":["v1.15"],"unstable_features":{}})
                } else if path.starts_with("/_matrix/client/v3/user/@owner:test/account_data/") {
                    status = 404;
                    json!({"errcode":"M_NOT_FOUND","error":"No fixture account data"})
                } else {
                    panic!("unexpected sign-in fixture route: {path}");
                };
                let body = value.to_string();
                stream.write_all(format!(
                    "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()
                ).as_bytes()).await.unwrap();
            }
        });
        Self {
            origin,
            refreshes,
            server,
        }
    }

    async fn client(&self, owner: &str, oauth: bool) -> matrix_sdk::Client {
        let client = matrix_sdk::Client::builder()
            .homeserver_url(&self.origin)
            .handle_refresh_tokens()
            .build()
            .await
            .unwrap();
        let mut value = json!({"user_id":owner,"device_id":"SYNTHETIC",
            "access_token":"synthetic-access","refresh_token":"synthetic-refresh"});
        if oauth {
            value["oauth_client_id"] = json!("synthetic-client");
        }
        let saved: crate::persistence::PersistedUserSession =
            serde_json::from_value(value).unwrap();
        client.restore_session(saved.into_sdk()).await.unwrap();
        client
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

#[tokio::test]
async fn explicit_signin_survives_real_sdk_refresh_before_and_after_publication() {
    let f = Fixture::new().await;
    let client = f.client("@owner:test", true).await;
    let mut signins = Signins::default();
    signins.record(&client, 4, 1);
    client.oauth().refresh_access_token().await.unwrap();
    assert_eq!(client.access_token().as_deref(), Some("synthetic-access-1"));
    let slot = crate::sliding_sync::client_slot::ClientSlot::new();
    let candidate = client.clone();
    slot.replace(Some(client.clone()), || {
        signins.publish(Some(&candidate), 4, slot.revision())
    });
    client.oauth().refresh_access_token().await.unwrap();
    assert_eq!(f.refreshes.load(Ordering::Acquire), 2);
    assert_eq!(client.access_token().as_deref(), Some("synthetic-access-2"));
    assert!(signins.consume(&slot.get().unwrap(), 4, slot.revision()));
    assert!(
        !signins.consume(&client, 4, 1),
        "fresh consent is consumed only once"
    );
}

#[tokio::test]
async fn identical_restored_credentials_are_not_the_explicit_candidate() {
    let f = Fixture::new().await;
    let original = f.client("@owner:test", true).await;
    let restored = f.client("@owner:test", true).await;
    assert_eq!(original.access_token(), restored.access_token());
    assert!(std::ptr::eq(
        original.http_client(),
        original.clone().http_client()
    ));
    assert!(!std::ptr::eq(
        original.http_client(),
        restored.http_client()
    ));
    let mut signins = Signins::default();
    signins.record(&original, 4, 1);
    signins.publish(Some(&restored), 4, 1);
    assert!(!signins.consume(&restored, 4, 1));
    signins.publish(Some(&original), 4, 2);
    assert!(
        !signins.consume(&original, 4, 2),
        "failed publication cannot reuse the candidate"
    );
}

#[tokio::test]
async fn replacement_removal_or_epoch_change_discards_previous_consent() {
    let f = Fixture::new().await;
    let client = f.client("@owner:test", true).await;
    for (epoch, revision) in [(5, 1), (4, 2)] {
        let mut signins = Signins::default();
        signins.record(&client, 4, 1);
        signins.publish(Some(&client), epoch, revision);
        assert!(!signins.consume(&client, epoch, revision));
    }
    for replacement in [None, Some(&client)] {
        let mut signins = Signins::default();
        signins.record(&client, 4, 1);
        signins.publish(Some(&client), 4, 1);
        signins.publish(replacement, 4, 2);
        assert!(!signins.consume(&client, 4, 1));
        assert!(!signins.consume(&client, 4, 2));
    }
}

#[tokio::test]
async fn published_consent_requires_original_identity_and_epoch() {
    let f = Fixture::new().await;
    let original = f.client("@owner:test", true).await;
    let other = f.client("@other:test", true).await;
    for (client, epoch, revision) in [(&other, 4, 1), (&original, 5, 1), (&original, 4, 2)] {
        let mut signins = Signins::default();
        signins.record(&original, 4, 1);
        signins.publish(Some(&original), 4, 1);
        assert!(!signins.consume(client, epoch, revision));
        assert!(!signins.consume(&original, 4, 1));
    }
}

#[tokio::test]
async fn ordinary_matrix_session_cannot_create_oauth_consent() {
    let f = Fixture::new().await;
    let client = f.client("@owner:test", false).await;
    let mut signins = Signins::default();
    signins.record(&client, 4, 1);
    signins.publish(Some(&client), 4, 1);
    assert!(!signins.consume(&client, 4, 1));
}
