//! Direct, operator-authorized remote bindings for the native owner runtime.
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use hagency_agent_local::inbox::Prepared;
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use super::RuntimeError;

const UNAVAILABLE: RuntimeError = RuntimeError::Execution("remote_runtime_unavailable");
const PENDING: RuntimeError = RuntimeError::Execution("remote_runtime_pending_reconciliation");
const INCOMPATIBLE: RuntimeError = RuntimeError::Execution("remote_runtime_incompatible");

#[derive(Clone)]
pub(in crate::console) struct RemoteBinding {
    settings: Settings,
    endpoint: Url,
    client: Client,
    receipts: PathBuf,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    endpoint: String,
    binding_id: String,
    token: String,
    provider: String,
    model: String,
}

#[derive(Deserialize)]
struct Capabilities {
    protocol_version: u8,
    binding_id: String,
    agent_id: String,
    owner: String,
    provider: String,
    model: String,
    active: bool,
    available: bool,
    scopes: Vec<String>,
}

#[derive(Serialize)]
struct Submission<'a> {
    binding_id: &'a str,
    agent_id: &'a str,
    caller: &'a str,
    scope: DirectScope<'a>,
    provider: &'a str,
    model: &'a str,
    prompt: &'a str,
}

#[derive(Serialize)]
struct DirectScope<'a> {
    kind: &'static str,
    room_id: &'a str,
    thread_id: &'a str,
}

#[derive(Deserialize)]
struct Run {
    id: String,
    binding_id: String,
    status: String,
    answer: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Receipt {
    run_id: String,
    binding_id: String,
    agent_id: String,
    dispatch_id: String,
    endpoint: String,
}

pub(super) fn pending_receipt(profile: &Path, agent: &str) -> Option<Receipt> {
    if !safe_id(agent) {
        return None;
    }
    let directory = profile.with_file_name("remote-runtime-runs").join(agent);
    for entry in std::fs::read_dir(directory).ok()?.take(128).flatten() {
        let Ok(file) = hagency_store::private::open(&entry.path(), false) else {
            continue;
        };
        let mut bytes = Vec::new();
        if file.take(16_384).read_to_end(&mut bytes).is_err() {
            continue;
        }
        let Ok(receipt) = serde_json::from_slice::<Receipt>(&bytes) else {
            continue;
        };
        if receipt.agent_id == agent
            && safe_id(&receipt.run_id)
            && valid_binding_id(&receipt.binding_id)
            && parse_endpoint(&receipt.endpoint).is_ok()
        {
            return Some(receipt);
        }
    }
    None
}

pub(super) enum RemoteOutcome {
    Succeeded(String),
    Rejected,
    Failed,
}

async fn decode<T: DeserializeOwned>(mut response: Response, size: usize) -> Option<T> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len().saturating_add(chunk.len()) > size {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).ok()
}

pub(in crate::console) fn load(
    ledger_path: &Path,
    agent: &str,
) -> Result<Option<RemoteBinding>, RuntimeError> {
    if !safe_id(agent) {
        return Err(INCOMPATIBLE);
    }
    let path = ledger_path
        .with_file_name("runtime-bindings")
        .join(format!("{agent}.json"));
    match path.symlink_metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(INCOMPATIBLE),
        Ok(meta) if !meta.is_file() => return Err(INCOMPATIBLE),
        Ok(_) => {}
    }
    let file = hagency_store::private::open(&path, false).map_err(|_| INCOMPATIBLE)?;
    let mut bytes = Vec::new();
    file.take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| INCOMPATIBLE)?;
    if bytes.len() > 8192 {
        return Err(INCOMPATIBLE);
    }
    let settings: Settings = serde_json::from_slice(&bytes).map_err(|_| INCOMPATIBLE)?;
    let endpoint = parse_endpoint(&settings.endpoint)?;
    if !matches!(settings.provider.as_str(), "codex" | "claude")
        || settings.model.is_empty()
        || settings.model.len() > 128
        || settings.model.chars().any(char::is_control)
        || !valid_binding_id(&settings.binding_id)
        || settings.token.len() != 64
        || !settings.token.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(INCOMPATIBLE);
    }
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| UNAVAILABLE)?;
    Ok(Some(RemoteBinding {
        settings,
        endpoint,
        client,
        receipts: ledger_path
            .with_file_name("remote-runtime-runs")
            .join(agent),
    }))
}

fn parse_endpoint(value: &str) -> Result<Url, RuntimeError> {
    let url = Url::parse(value).map_err(|_| INCOMPATIBLE)?;
    if url.scheme() != "https"
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || ch == '\\' || ch.is_control())
    {
        return Err(INCOMPATIBLE);
    }
    Ok(url)
}

fn safe_id(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
}

fn valid_binding_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}

impl RemoteBinding {
    pub(super) fn endpoint(&self) -> &str {
        self.endpoint.as_str()
    }

    pub(in crate::console) fn provider(&self) -> &str {
        &self.settings.provider
    }

    pub(in crate::console) fn model(&self) -> &str {
        &self.settings.model
    }

    pub(super) fn consent(&self) -> String {
        let identity = serde_json::json!([
            self.endpoint.as_str(),
            self.settings.binding_id,
            self.settings.provider,
            self.settings.model,
            self.settings.token,
        ]);
        format!("{:x}", Sha256::digest(identity.to_string()))
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(
                method,
                self.endpoint.join(path).expect("fixed runtime route"),
            )
            .bearer_auth(&self.settings.token)
    }

    pub(super) async fn verify(&self, owner: &str, agent: &str) -> Result<(), RuntimeError> {
        let response = self
            .request(Method::GET, "v1/runtime")
            .send()
            .await
            .map_err(|_| UNAVAILABLE)?;
        if matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            return Err(RuntimeError::Execution("remote_runtime_entitlement_denied"));
        }
        if !response.status().is_success() {
            return Err(UNAVAILABLE);
        }
        let caps: Capabilities = decode(response, 8192).await.ok_or(INCOMPATIBLE)?;
        if caps.protocol_version != 1
            || caps.binding_id != self.settings.binding_id
            || caps.owner != owner
            || caps.agent_id != agent
            || caps.provider != self.settings.provider
            || caps.model != self.settings.model
            || !caps.scopes.iter().any(|scope| scope == "owner_direct")
        {
            return Err(INCOMPATIBLE);
        }
        if !caps.active {
            return Err(RuntimeError::Execution("remote_runtime_entitlement_denied"));
        }
        if !caps.available {
            return Err(UNAVAILABLE);
        }
        Ok(())
    }

    // The existing ledger already owns the dispatch and execution ID. Save the
    // immutable runtime route before the network request, without prompt or token.
    pub(super) fn record(&self, prepared: &Prepared) -> Result<(), RuntimeError> {
        if !safe_id(&prepared.execution_id) {
            return Err(INCOMPATIBLE);
        }
        let parent = self.receipts.parent().ok_or(RuntimeError::Profile)?;
        hagency_store::private::directory(parent).map_err(|_| RuntimeError::Profile)?;
        hagency_store::private::directory(&self.receipts).map_err(|_| RuntimeError::Profile)?;
        let path = self.receipt_path(&prepared.execution_id);
        let receipt = Receipt {
            run_id: prepared.execution_id.clone(),
            binding_id: self.settings.binding_id.clone(),
            agent_id: prepared.dispatch.agent_id.clone(),
            dispatch_id: prepared.dispatch.id.clone(),
            endpoint: self.endpoint.as_str().into(),
        };
        let data = serde_json::to_vec_pretty(&receipt).map_err(|_| RuntimeError::Profile)?;
        hagency_store::private::write_new(&path, &data).map_err(|_| RuntimeError::Profile)
    }

    fn receipt_path(&self, id: &str) -> PathBuf {
        self.receipts.join(format!("{id}.json"))
    }

    pub(super) fn resolved(&self, id: &str) {
        if let Err(error) = std::fs::remove_file(self.receipt_path(id)) {
            tracing::warn!(%error, run_id = id, "Remote run receipt could not be removed");
        }
    }

    pub(super) async fn run(&self, prepared: &Prepared) -> Result<RemoteOutcome, RuntimeError> {
        let dispatch = &prepared.dispatch;
        let id = &prepared.execution_id;
        let path = format!("v1/runs/{id}");
        let input = Submission {
            binding_id: &self.settings.binding_id,
            agent_id: &dispatch.agent_id,
            caller: &dispatch.requester_mxid,
            scope: DirectScope {
                kind: "owner_direct",
                room_id: &dispatch.room_id,
                thread_id: &dispatch.thread_root,
            },
            provider: &self.settings.provider,
            model: &self.settings.model,
            prompt: &dispatch.body,
        };
        let mut submit = true;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3660);
        while tokio::time::Instant::now() < deadline {
            let request = if submit {
                self.request(Method::PUT, &path).json(&input)
            } else {
                self.request(Method::GET, &path)
            };
            let sent_submission = submit;
            submit = false;
            if let Ok(response) = request.send().await {
                match response.status() {
                    StatusCode::NOT_FOUND if !sent_submission => submit = true,
                    StatusCode::FORBIDDEN
                    | StatusCode::UNAUTHORIZED
                    | StatusCode::TOO_MANY_REQUESTS
                        if sent_submission =>
                    {
                        return Ok(RemoteOutcome::Rejected);
                    }
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(PENDING),
                    StatusCode::CONFLICT | StatusCode::BAD_REQUEST if sent_submission => {
                        return Err(INCOMPATIBLE);
                    }
                    status if status.is_success() => {
                        let record: Run = decode(response, 400_000).await.ok_or(PENDING)?;
                        if record.id != *id || record.binding_id != self.settings.binding_id {
                            return Err(PENDING);
                        }
                        match record.status.as_str() {
                            "succeeded" => {
                                return record
                                    .answer
                                    .filter(|text| !text.is_empty() && text.len() <= 65_536)
                                    .map(RemoteOutcome::Succeeded)
                                    .ok_or(PENDING);
                            }
                            "rejected" => return Ok(RemoteOutcome::Rejected),
                            "canceled" | "failed" => return Ok(RemoteOutcome::Failed),
                            "queued" | "running" | "cancel_requested" => {}
                            _ => return Err(PENDING),
                        }
                    }
                    _ => {}
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Err(PENDING)
    }

    pub(super) async fn cancel(&self, id: &str) {
        let _ = self
            .request(Method::POST, &format!("v1/runs/{id}/cancel"))
            .send()
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_https_origins() {
        assert!(parse_endpoint("https://runtime.example.test:8443").is_ok());
        for url in [
            "http://localhost:8077",
            "https://user:pass@host",
            "https://host/base",
            "https://host?agent=x",
            "https://host/#secret",
        ] {
            assert!(parse_endpoint(url).is_err(), "{url}");
        }
    }
}
