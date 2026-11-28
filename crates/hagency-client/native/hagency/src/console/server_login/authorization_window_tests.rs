//! The window a renewal asks for, under both of its headers, over real TCP.
use super::*;
use crate::console::native::{MatrixAccessToken, MatrixTokenSource, NativeError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Source;
impl MatrixTokenSource for Source {
    fn access_token(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>> + Send + '_>,
    > {
        Box::pin(async {
            Ok(MatrixAccessToken {
                access_token: "sdk-access".into(),
                client_id: "client".into(),
            })
        })
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn header_values<'a>(request: &'a str, name: &str) -> Vec<&'a str> {
    request
        .split_once("\r\n\r\n")
        .unwrap()
        .0
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
        .collect()
}

#[tokio::test]
async fn both_renewals_ask_for_the_window_under_the_old_and_the_v2_header() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let now = Instant::now();
    let mut session = RemoteSession {
        matrix_source: Some(std::sync::Arc::new(Source)),
        token: "owner-session".into(),
        oauth_token: String::new(),
        user_id: "owner".into(),
        device: None,
        refresh_token: None,
        oauth_expires: now + Duration::from_secs(900),
        authorized_until: now,
        clock_allowance: Duration::ZERO,
        full_window: Duration::ZERO,
        sdk_refreshed: None,
        invalidated: false,
        binding: Binding {
            origin: origin.clone(),
            client_id: "client".into(),
            installation_id: "installation".into(),
            name: "fixture".into(),
            owner: Some("@owner:test".into()),
            subject: Some("subject".into()),
        },
        checked: now,
        expires: now + Duration::from_secs(900),
    };
    // The first proof is about to end, so the renewal has the SDK refresh its
    // token and renews again with it: the second call site.
    let replies = [
        json!({"validUntilMs": unix_ms() + 15_000}),
        json!({"validUntilMs": unix_ms() + 30_000}),
        json!({"mxid":"@owner:test","subject":"subject","clientId":"client",
            "issuer":format!("{origin}auth/"),"userId":"owner",
            "validUntilMs": unix_ms() + 30_000}),
    ];
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for body in replies {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut raw = Vec::new();
            loop {
                let mut chunk = [0u8; 4096];
                let size = socket.read(&mut chunk).await.unwrap();
                assert!(size > 0, "request ended before its declared body");
                raw.extend_from_slice(&chunk[..size]);
                let request = String::from_utf8_lossy(&raw);
                if let Some((headers, body)) = request.split_once("\r\n\r\n") {
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if body.len() >= length {
                        break;
                    }
                }
            }
            requests.push(String::from_utf8(raw).unwrap());
            let body = body.to_string();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
        requests
    });
    tokio::time::timeout(Duration::from_secs(10), renew(&mut session))
        .await
        .expect("the renewal must settle")
        .unwrap();
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[2].starts_with("GET /hagency/api/v1/identity "));
    let window = proof_clock::REQUESTED_WINDOW_MS.to_string();
    for request in &requests[..2] {
        assert!(request.starts_with("POST /hagency/api/v1/sessions/current/renew "));
        assert_eq!(
            header_values(request, "x-hagency-authorization-window-v2-ms"),
            [window.as_str()]
        );
        assert_eq!(
            header_values(request, "x-hagency-authorization-window-ms"),
            [window.as_str()]
        );
    }
    // The identity check asks for no window.
    assert!(header_values(&requests[2], "x-hagency-authorization-window-v2-ms").is_empty());
    assert!(header_values(&requests[2], "x-hagency-authorization-window-ms").is_empty());
}
