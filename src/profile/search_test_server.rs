// Local authenticated Matrix fixture for account-search tests.
use matrix_sdk::Client;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) struct DirectoryServer {
    url: String,
    pub requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for DirectoryServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl DirectoryServer {
    pub async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let seen = seen.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut chunk = [0; 4096];
                    let head_end = loop {
                        let n = stream.read(&mut chunk).await.unwrap();
                        if n == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&chunk[..n]);
                        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            break at + 4;
                        }
                    };
                    let head = String::from_utf8_lossy(&bytes[..head_end]).to_string();
                    let length = head
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|n| n.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    while bytes.len() < head_end + length {
                        let n = stream.read(&mut chunk).await.unwrap();
                        if n == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                    let route = head.lines().next().unwrap().to_owned();
                    seen.lock().unwrap().push(route.clone());
                    assert!(
                        head.to_lowercase()
                            .contains("authorization: bearer directory-fixture")
                    );
                    let (status, body) = if route.contains("user_directory/search") {
                        let payload: Value =
                            serde_json::from_slice(&bytes[head_end..head_end + length])
                                .unwrap();
                        if (payload["search_term"] == "Chris" || payload["search_term"] == "ch") || payload["search_term"] == "" {
                            (
                                200,
                                json!({"limited":true,"results":[
                                    {"user_id":"@chris:example.test","display_name":"Chris"},
                                    {"user_id":"@chris:example.test","display_name":"Chris"},
                                    {"user_id":"@chris:remote.test","display_name":"Chris Remote"},
                                    {"user_id":"@_hagency_agent:example.test","display_name":"Agent"}
                                ]}),
                            )
                        } else {
                            (200, json!({"limited":false,"results":[]}))
                        }
                    } else if route.contains("private") {
                        (200, json!({"displayname":"Private account"}))
                    } else if route.contains("chris") {
                        (200, json!({"displayname":"Chris"}))
                    } else if route.contains("offline") {
                        (
                            403,
                            json!({"errcode":"M_FORBIDDEN","error":"Not available"}),
                        )
                    } else {
                        (
                            404,
                            json!({"errcode":"M_NOT_FOUND","error":"No such account"}),
                        )
                    };
                    let body = body.to_string();
                    let response = format!(
                        "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    stream.write_all(response.as_bytes()).await.unwrap();
                });
            }
        });
        Self {
            url,
            requests,
            task,
        }
    }
    pub async fn client(&self) -> Client {
        let client = Client::builder()
            .homeserver_url(&self.url)
            .server_versions([ruma::api::MatrixVersion::V1_15])
            .build()
            .await
            .unwrap();
        client.matrix_auth().restore_session(serde_json::from_value(json!({
            "user_id":"@owner:example.test", "device_id":"DIRECTORY_TEST", "access_token":"directory-fixture"
        })).unwrap(), Default::default()).await.unwrap();
        client
    }
}

