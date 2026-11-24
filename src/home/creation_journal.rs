//! Persist before createRoom. Unknown POST outcomes are reconciled, never replayed.
use super::space_management::CreateRoomConfig;
use anyhow::{Result, anyhow, ensure};
use matrix_sdk::{
    Client,
    ruma::{OwnedRoomId, events::StateEventType},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{io::Read, path::PathBuf, time::Duration};
const MARKER: &str = "org.hagency.creation.v1";
const LIMIT: u64 = 64 * 1024;
const ARCHIVE_LIMIT: usize = 32;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Intent {
    pub operation: uuid::Uuid,
    pub origin: String,
    pub owner: String,
    pub config: CreateRoomConfig,
    pub agent_space: bool,
    pub project: Option<String>,
    /// The shared project folder asked for with an Agent space. It is not part of the
    /// room's marker: the server keeps it when the space is registered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_folder: Option<String>,
    pub room: Option<OwnedRoomId>,
    pub complete: bool,
}
impl Intent {
    pub fn marker(&self) -> Value {
        json!({"version":1,"operation":self.operation,"owner":self.owner,
            "intent":blake3::hash(&serde_json::to_vec(&(&self.config,self.agent_space,&self.project)).expect("serializable intent")).to_hex().to_string()})
    }
}
#[derive(Debug)]
pub(super) struct ReviewRecords {
    pub current: Option<Intent>,
    pub unresolved: Vec<Intent>,
}

#[derive(Clone)]
pub(super) struct Journal {
    root: PathBuf,
    origin: String,
    owner: String,
}
impl Journal {
    pub fn for_client(client: &Client) -> Result<Self> {
        let origin = client.homeserver().to_string();
        let owner = client
            .user_id()
            .ok_or_else(|| anyhow!("Sign in first."))?
            .to_string();
        let key = blake3::hash(&serde_json::to_vec(&(&origin, &owner))?).to_hex();
        Ok(Self {
            root: crate::app_data_dir()
                .join("room-creation-v1")
                .join(key.as_str()),
            origin,
            owner,
        })
    }
    fn read(&self) -> Result<Option<Intent>> {
        self.read_path(&self.root.join("current.json"))
    }
    fn read_path(&self, path: &std::path::Path) -> Result<Option<Intent>> {
        let mut bytes = Vec::new();
        let file = match hagency_store::private::open(&path, false) {
            Ok(file) => file,
            Err(hagency_store::Error::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        ensure!(
            file.metadata()?.len() <= LIMIT,
            "Saved creation is too large."
        );
        file.take(LIMIT + 1).read_to_end(&mut bytes)?;
        ensure!(bytes.len() as u64 <= LIMIT, "Saved creation is too large.");
        let intent: Intent = serde_json::from_slice(&bytes)?;
        ensure!(
            intent.origin == self.origin && intent.owner == self.owner,
            "Saved creation belongs to another account."
        );
        intent.config.request()?;
        Ok(Some(intent))
    }
    fn write(&self, intent: &Intent) -> Result<()> {
        let bytes = serde_json::to_vec(intent)?;
        ensure!(bytes.len() as u64 <= LIMIT, "Saved creation is too large.");
        hagency_store::private::replace(&self.root.join("current.json"), &bytes)?;
        Ok(())
    }
    async fn io<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Self) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let journal = self.clone();
        tokio::task::spawn_blocking(move || {
            hagency_store::private::directory(journal.root.parent().unwrap())?;
            hagency_store::private::directory(&journal.root)?;
            let lease_path = journal.root.join("lease");
            let lease = match hagency_store::private::open(&lease_path, true) {
                Ok(lease) => lease,
                Err(hagency_store::Error::Io(error))
                    if error.kind() == std::io::ErrorKind::AlreadyExists =>
                {
                    hagency_store::private::open(&lease_path, false)?
                }
                Err(error) => return Err(error.into()),
            };
            lease.try_lock().map_err(|_| {
                anyhow!("Another window is updating the saved creation. Try again shortly.")
            })?;
            let confirmed = journal.root.join("confirmed");
            #[cfg(unix)]
            let new_directory = !confirmed.try_exists()?;
            hagency_store::private::directory(&confirmed)?;
            #[cfg(unix)]
            if new_directory {
                std::fs::File::open(&journal.root)?.sync_all()?;
            }
            work(&journal)
        })
        .await
        .map_err(|_| anyhow!("Saved creation storage is unavailable."))?
    }
    fn unresolved_path(&self, operation: uuid::Uuid) -> PathBuf {
        self.root.join(format!("unresolved-{operation}.json"))
    }

    fn confirmed_path(&self, operation: uuid::Uuid) -> PathBuf {
        self.root
            .join("confirmed")
            .join(format!("{operation}.json"))
    }

    // Keep the returned room ID for late callbacks. Only unknown outcomes
    // consume one of the 32 slots. Write the receipt durably before unlinking
    // its unresolved copy so an interrupted migration can be retried.
    fn save_archive(&self, intent: &Intent) -> Result<()> {
        let unresolved = self.unresolved_path(intent.operation);
        let bytes = serde_json::to_vec(intent)?;
        ensure!(bytes.len() as u64 <= LIMIT, "Saved creation is too large.");
        if intent.room.is_none() {
            hagency_store::private::replace(&unresolved, &bytes)?;
        } else {
            hagency_store::private::replace(&self.confirmed_path(intent.operation), &bytes)?;
            match std::fs::remove_file(unresolved) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                result => result?,
            }
        }
        Ok(())
    }

    fn unresolved(&self) -> Result<Vec<Intent>> {
        let mut records = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry
                .file_name()
                .to_string_lossy()
                .starts_with("unresolved-")
            {
                continue;
            }
            let record = self
                .read_path(&entry.path())?
                .ok_or_else(|| anyhow!("Saved creation is missing."))?;
            ensure!(
                entry.path() == self.unresolved_path(record.operation),
                "Saved creation changed."
            );
            if let Some(confirmed) = self.read_path(&self.confirmed_path(record.operation))? {
                ensure!(
                    confirmed.operation == record.operation
                        && confirmed.room.is_some()
                        && record
                            .room
                            .as_ref()
                            .is_none_or(|id| Some(id) == confirmed.room.as_ref()),
                    "Saved creation changed."
                );
                std::fs::remove_file(entry.path())?;
            } else if record.room.is_some() {
                self.save_archive(&record)?;
            } else {
                records.push(record);
            }
        }
        records.sort_by_key(|record| record.operation);
        Ok(records)
    }

    pub async fn review_records(&self) -> Result<ReviewRecords> {
        self.io(|j| {
            Ok(ReviewRecords {
                current: j.read()?.filter(|i| !i.complete),
                unresolved: j.unresolved()?,
            })
        })
        .await
    }

    #[cfg(test)]
    pub async fn pending(&self) -> Result<Option<Intent>> {
        self.io(|j| Ok(j.read()?.filter(|i| !i.complete))).await
    }
    #[cfg(test)]
    pub async fn begin(
        &self,
        config: CreateRoomConfig,
        agent_space: bool,
        project: Option<String>,
    ) -> Result<Intent> {
        self.begin_with_folder(config, agent_space, project, None).await
    }
    pub async fn begin_with_folder(
        &self,
        config: CreateRoomConfig,
        agent_space: bool,
        project: Option<String>,
        project_folder: Option<String>,
    ) -> Result<Intent> {
        config.request()?;
        self.io(move |j| {
            ensure!(
                j.read()?.is_none_or(|i| i.complete),
                "A saved creation needs review. Recover it before starting another."
            );
            let intent = Intent {
                operation: uuid::Uuid::new_v4(),
                origin: j.origin.clone(),
                owner: j.owner.clone(),
                config,
                agent_space,
                project,
                project_folder,
                room: None,
                complete: false,
            };
            j.write(&intent)?;
            Ok(intent)
        })
        .await
    }
    fn update(
        &self,
        operation: uuid::Uuid,
        change: impl FnOnce(&mut Intent) -> Result<()>,
    ) -> Result<Intent> {
        let current = self.read()?;
        let active = current.filter(|i| i.operation == operation);
        let archive = match self.read_path(&self.confirmed_path(operation))? {
            Some(receipt) => Some(receipt),
            None => self.read_path(&self.unresolved_path(operation))?,
        };
        let mut intent = archive
            .clone()
            .or_else(|| active.clone())
            .ok_or_else(|| anyhow!("Saved creation is missing."))?;
        ensure!(intent.operation == operation, "Saved creation changed.");
        change(&mut intent)?;
        if archive.is_some() {
            self.save_archive(&intent)?;
        }
        if let Some(current) = active {
            // Explicitly stopping tracking closes the active slot even while
            // the archive still awaits confirmation of the room ID.
            let mut updated = intent.clone();
            updated.complete |= current.complete;
            self.write(&updated)?;
        }
        Ok(intent)
    }
    pub async fn identified(&self, operation: uuid::Uuid, room: OwnedRoomId) -> Result<Intent> {
        self.io(move |j| {
            j.update(operation, |intent| {
                ensure!(
                    intent.room.as_ref().is_none_or(|old| old == &room),
                    "Recovery cannot replace the original room."
                );
                intent.room = Some(room);
                Ok(())
            })
        })
        .await
    }
    pub async fn completed(&self, operation: uuid::Uuid) -> Result<()> {
        self.io(move |j| {
            j.update(operation, |intent| {
                ensure!(intent.room.is_some(), "Saved creation is not confirmed.");
                intent.complete = true;
                Ok(())
            })?;
            Ok(())
        })
        .await
    }
    pub async fn archive(&self, operation: uuid::Uuid) -> Result<()> {
        self.io(move |j| {
            let mut intent = j
                .read()?
                .ok_or_else(|| anyhow!("Saved creation is missing."))?;
            ensure!(
                intent.operation == operation && !intent.complete,
                "Saved creation changed."
            );
            let records = j.unresolved()?;
            if let Some(receipt) = j.read_path(&j.confirmed_path(operation))? {
                ensure!(
                    receipt.operation == operation
                        && receipt.room.is_some()
                        && intent
                            .room
                            .as_ref()
                            .is_none_or(|room| Some(room) == receipt.room.as_ref()),
                    "Saved creation changed."
                );
                intent.room = receipt.room;
            }
            ensure!(
                intent.room.is_some()
                    || records.len() < ARCHIVE_LIMIT
                    || records.iter().any(|saved| saved.operation == operation),
                "Too many unresolved creations. Review saved records before starting another."
            );
            j.save_archive(&intent)?;
            intent.complete = true;
            j.write(&intent)
        })
        .await
    }
}
fn matches_state(intent: &Intent, events: &[Value]) -> bool {
    let find = |kind: &str| {
        events
            .iter()
            .find(|e| e["type"] == kind && e["state_key"] == "")
    };
    find(MARKER).is_some_and(|e| e["sender"] == intent.owner && e["content"] == intent.marker())
        && find("m.room.create").is_some_and(|e| {
            e["sender"] == intent.owner
                && (e["content"]["type"] == "m.space") == intent.config.space
        })
        && find("m.room.name").is_some_and(|e| e["content"]["name"] == intent.config.name.trim())
        && find("m.room.join_rules").is_some_and(|e| {
            e["content"]["join_rule"]
                == if intent.config.public {
                    "public"
                } else {
                    "invite"
                }
        })
        && (find("m.room.encryption")
            .is_some_and(|e| e["content"]["algorithm"] == "m.megolm.v1.aes-sha2")
            == (!intent.config.space && intent.config.encrypted))
}
async fn validate_original(client: &Client, intent: &Intent, room: &OwnedRoomId) -> Result<()> {
    crate::matrix_context::ensure_current(client)?;
    let response = client
        .send(
            matrix_sdk::ruma::api::client::state::get_state_events::v3::Request::new(room.clone()),
        )
        .await?;
    crate::matrix_context::ensure_current(client)?;
    let events = response
        .room_state
        .iter()
        .map(|e| serde_json::from_str(e.json().get()))
        .collect::<std::result::Result<Vec<Value>, _>>()?;
    ensure!(
        matches_state(intent, &events)
            && events.iter().any(|e| e["type"] == "m.room.member"
                && e["state_key"] == intent.owner
                && e["content"]["membership"] == "join"),
        "The selected room does not match the original creation and current membership."
    );
    Ok(())
}
pub(super) async fn recover(client: &Client, intent: &Intent, selected: &str) -> Result<Intent> {
    crate::matrix_context::ensure_current(client)?;
    let journal = Journal::for_client(client)?;
    recover_in(client, intent, selected, &journal).await
}
async fn recover_in(
    client: &Client,
    intent: &Intent,
    selected: &str,
    journal: &Journal,
) -> Result<Intent> {
    crate::matrix_context::ensure_current(client)?;
    ensure!(
        intent.origin == journal.origin && intent.owner == journal.owner,
        "Saved creation belongs to another account."
    );
    let room = if let Some(room) = &intent.room {
        room.clone()
    } else if !selected.trim().is_empty() {
        selected.trim().try_into()?
    } else {
        let joined = client
            .send(matrix_sdk::ruma::api::client::membership::joined_rooms::v3::Request::new())
            .await?;
        ensure!(
            joined.joined_rooms.len() <= 128,
            "Too many rooms to check automatically. Enter the original room ID to verify it."
        );
        let mut found = None;
        let mut incomplete = false;
        for room in joined.joined_rooms {
            crate::matrix_context::ensure_current(client)?;
            let marker = client
                .send(
                    matrix_sdk::ruma::api::client::state::get_state_event_for_key::v3::Request::new(
                        room.clone(),
                        StateEventType::from(MARKER),
                        String::new(),
                    ),
                )
                .await;
            match marker {
                Ok(marker) => {
                    let value: Value = serde_json::from_str(marker.event_or_content.get())?;
                    if value == intent.marker() {
                        validate_original(client, intent, &room).await?;
                        ensure!(
                            found.is_none(),
                            "More than one room matches this creation. Enter the original room ID to choose explicitly."
                        );
                        found = Some(room);
                    }
                }
                Err(error)
                    if error
                        .as_client_api_error()
                        .is_some_and(|e| e.status_code.as_u16() == 404) => {}
                Err(_) => incomplete = true,
            }
        }
        ensure!(
            !incomplete,
            "Some rooms could not be checked. Retry recovery; no new room was created."
        );
        found.ok_or_else(||anyhow!("The original creation is not confirmed yet. Retry recovery or enter its room ID."))?
    };
    validate_original(client, intent, &room).await?;
    journal.identified(intent.operation, room).await
}
pub(super) async fn create(
    client: &Client,
    config: CreateRoomConfig,
    agent_space: bool,
    project: Option<String>,
    project_folder: Option<String>,
) -> Result<Intent> {
    crate::matrix_context::ensure_current(client)?;
    let journal = Journal::for_client(client)?;
    create_in(client, config, agent_space, project, project_folder, &journal).await
}
async fn create_in(
    client: &Client,
    config: CreateRoomConfig,
    agent_space: bool,
    project: Option<String>,
    project_folder: Option<String>,
    journal: &Journal,
) -> Result<Intent> {
    crate::matrix_context::ensure_current(client)?;
    let intent = journal.begin_with_folder(config, agent_space, project, project_folder).await?;
    let mut request = intent.config.request()?;
    request
        .initial_state
        .push(matrix_sdk::ruma::serde::Raw::from_json_string(
            serde_json::to_string(
                &json!({"type":MARKER,"state_key":"","content":intent.marker()}),
            )?,
        )?);
    crate::matrix_context::ensure_current(client)?;
    // A retry of this POST is never safe: SDK default retries are disabled here.
    let room = client
        .send(request)
        .with_request_config(
            matrix_sdk::config::RequestConfig::new()
                .retry_limit(0)
                .timeout(Duration::from_secs(30)),
        )
        .await;
    match room {
        Ok(room) => journal.identified(intent.operation, room.room_id).await,
        Err(_) => Err(anyhow!(
            "Creation outcome is unknown. Recover the saved request before starting another."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn journal(root: &std::path::Path) -> Journal {
        Journal {
            root: root.join("private/owner"),
            origin: "https://matrix.invalid/".into(),
            owner: "@alice:example.org".into(),
        }
    }
    fn config() -> CreateRoomConfig {
        CreateRoomConfig {
            name: "Original name".into(),
            topic: "Original topic".into(),
            encrypted: true,
            ..Default::default()
        }
    }
    #[tokio::test]
    async fn persisted_intent_precedes_effect_and_reopen_never_restarts_unknown_creation() {
        let root = tempfile::tempdir().unwrap();
        let j = journal(root.path());
        let original = j
            .begin(config(), false, Some("original-project".into()))
            .await
            .unwrap();
        let reopened = journal(root.path());
        let saved = reopened.pending().await.unwrap().unwrap();
        assert_eq!(saved.operation, original.operation);
        assert_eq!(saved.project.as_deref(), Some("original-project"));
        assert!(reopened.begin(config(), false, None).await.is_err());
        let room: OwnedRoomId = "!original:example.org".try_into().unwrap();
        reopened
            .identified(saved.operation, room.clone())
            .await
            .unwrap();
        assert_eq!(
            journal(root.path()).pending().await.unwrap().unwrap().room,
            Some(room)
        );
        assert!(
            reopened
                .identified(saved.operation, "!other:example.org".try_into().unwrap())
                .await
                .is_err()
        );
        reopened.completed(saved.operation).await.unwrap();
        assert!(reopened.pending().await.unwrap().is_none());
        assert_ne!(
            reopened
                .begin(config(), false, None)
                .await
                .unwrap()
                .operation,
            original.operation
        );
    }
    #[tokio::test]
    async fn explicit_archive_preserves_original_unknown_and_late_id_without_touching_new_request()
    {
        let root = tempfile::tempdir().unwrap();
        let j = journal(root.path());
        let old = j.begin(config(), true, None).await.unwrap();
        j.archive(old.operation).await.unwrap();
        let new = j.begin(config(), false, None).await.unwrap();
        let oldroom: OwnedRoomId = "!old:example.org".try_into().unwrap();
        j.identified(old.operation, oldroom.clone()).await.unwrap();
        j.completed(old.operation).await.unwrap();
        let current = j.pending().await.unwrap().unwrap();
        assert_eq!(current.operation, new.operation);
        assert!(current.room.is_none());
        let archive = j
            .read_path(&j.confirmed_path(old.operation))
            .unwrap()
            .unwrap();
        assert_eq!(archive.room, Some(oldroom));
        assert!(archive.complete);
    }
    #[tokio::test]
    async fn full_archive_releases_only_confirmed_rooms_and_keeps_the_current_request() {
        let root = tempfile::tempdir().unwrap();
        let j = journal(root.path());
        for _ in 0..ARCHIVE_LIMIT {
            let intent = j.begin(config(), false, None).await.unwrap();
            j.archive(intent.operation).await.unwrap();
        }
        let current = j.begin(config(), false, None).await.unwrap();
        assert!(j.archive(current.operation).await.is_err());
        let saved = j.review_records().await.unwrap();
        assert_eq!(saved.current.unwrap().operation, current.operation);
        assert_eq!(saved.unresolved.len(), ARCHIVE_LIMIT);
        let old = &saved.unresolved[0];
        let room: OwnedRoomId = "!old:example.org".try_into().unwrap();
        j.identified(old.operation, room.clone()).await.unwrap();
        j.archive(current.operation).await.unwrap();

        // The last archived intent is also still in current.json. A late ID
        // must not reopen it or turn another unknown outcome into a success.
        j.identified(
            current.operation,
            "!current:example.org".try_into().unwrap(),
        )
        .await
        .unwrap();
        let reopened = journal(root.path()).review_records().await.unwrap();
        assert!(reopened.current.is_none());
        assert_eq!(reopened.unresolved.len(), ARCHIVE_LIMIT - 1);
        assert!(
            reopened
                .unresolved
                .iter()
                .all(|i| i.operation != old.operation && i.room.is_none())
        );
        assert!(
            j.identified(old.operation, "!wrong:example.org".try_into().unwrap())
                .await
                .is_err()
        );
        assert_eq!(
            j.read_path(&j.confirmed_path(old.operation))
                .unwrap()
                .unwrap()
                .room,
            Some(room)
        );
    }

    #[tokio::test]
    async fn review_migrates_legacy_archives_and_retries_partial_writes_at_capacity() {
        let root = tempfile::tempdir().unwrap();
        let j = journal(root.path());
        let current = j.begin(config(), false, None).await.unwrap();
        let write_legacy = |intent: &Intent| {
            hagency_store::private::replace(
                &j.unresolved_path(intent.operation),
                &serde_json::to_vec(intent).unwrap(),
            )
            .unwrap();
        };
        for _ in 0..ARCHIVE_LIMIT - 1 {
            let mut unknown = current.clone();
            unknown.operation = uuid::Uuid::new_v4();
            // Old active tombstones could carry complete=true without a room.
            unknown.complete = true;
            write_legacy(&unknown);
        }
        // The last slot was persisted, but closing current.json was interrupted.
        write_legacy(&current);
        let mut known = current.clone();
        known.operation = uuid::Uuid::new_v4();
        known.room = Some("!verified:example.org".try_into().unwrap());
        write_legacy(&known);
        j.archive(current.operation).await.unwrap();
        assert_eq!(
            j.review_records().await.unwrap().unresolved.len(),
            ARCHIVE_LIMIT
        );
        assert!(!j.unresolved_path(known.operation).exists());
        assert_eq!(
            j.read_path(&j.confirmed_path(known.operation))
                .unwrap()
                .unwrap()
                .room,
            known.room
        );

        // A duplicate left after receipt creation must not replace a known ID.
        let mut stale = known.clone();
        stale.room = None;
        write_legacy(&stale);
        let next = j.begin(config(), false, None).await.unwrap();
        j.identified(next.operation, "!next:example.org".try_into().unwrap())
            .await
            .unwrap();
        j.archive(next.operation).await.unwrap();
        assert!(!j.unresolved_path(stale.operation).exists());
        assert_eq!(
            j.review_records().await.unwrap().unresolved.len(),
            ARCHIVE_LIMIT
        );
    }

    #[tokio::test]
    async fn private_storage_and_account_and_corruption_checks_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let j = journal(root.path());
        let old = j.begin(config(), false, None).await.unwrap();
        let file = hagency_store::private::open(&j.root.join("current.json"), false).unwrap();
        hagency_store::private::check_handle(&file).unwrap();
        let mut other = j.clone();
        other.owner = "@bob:example.org".into();
        assert!(other.pending().await.is_err());
        let mut other = j.clone();
        other.origin = "https://other.invalid/".into();
        assert!(other.pending().await.is_err());
        assert!(
            j.identified(uuid::Uuid::new_v4(), "!x:example.org".try_into().unwrap())
                .await
                .is_err()
        );
        std::fs::write(j.root.join("current.json"), b"broken").unwrap();
        assert!(j.pending().await.is_err());
        assert!(j.begin(config(), false, None).await.is_err());
        assert!(!old.marker().to_string().contains("Original topic"));
    }
    #[tokio::test]
    async fn concurrent_begin_and_lease_ownership_cannot_admit_two_creations() {
        let root = tempfile::tempdir().unwrap();
        let j = journal(root.path());
        let (a, b) = tokio::join!(
            j.begin(config(), false, None),
            j.begin(config(), false, None)
        );
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        let lease = hagency_store::private::open_journal(&j.root.join("lease")).unwrap();
        lease.try_lock().unwrap();
        assert!(j.pending().await.is_err());
        drop(lease);
        assert!(j.pending().await.unwrap().is_some());
    }
    #[test]
    fn real_http_lost_create_response_is_reconciled_without_another_post_for_all_wizard_kinds() {
        const NAME: &str = "home::creation_journal::tests::real_http_lost_create_response_is_reconciled_without_another_post_for_all_wizard_kinds";
        if std::env::var_os("HAGENCY_CREATION_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", NAME, "--test-threads=1"])
                .env("HAGENCY_CREATION_CHILD", "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
            return;
        }
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            use wiremock::{MockServer,Mock,ResponseTemplate,matchers::{method,path_regex}};
            for (space,agent,project) in [(true,false,None),(true,true,None),(false,false,Some("original-project".to_owned()))] {
                let server=MockServer::start().await;let root=tempfile::tempdir().unwrap();
                let client=Client::builder().homeserver_url(server.uri()).server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_11]).build().await.unwrap();
                client.matrix_auth().restore_session(serde_json::from_value(json!({"access_token":"fixture","user_id":"@alice:example.org","device_id":"CREATION"})).unwrap(),Default::default()).await.unwrap();
                crate::sliding_sync::test_set_current_client(Some(client.clone()));
                let mut j=Journal::for_client(&client).unwrap();j.root=root.path().join("private/journal");
                let marker=std::sync::Arc::new(std::sync::Mutex::new(Value::Null));let retained=marker.clone();let disk=j.root.clone();
                Mock::given(method("POST")).and(path_regex(".*/createRoom$")).respond_with(move|request:&wiremock::Request|{
                    let body:Value=serde_json::from_slice(&request.body).unwrap();let event=body["initial_state"].as_array().unwrap().iter().find(|e|e["type"]==MARKER).unwrap();*retained.lock().unwrap()=event["content"].clone();
                    let saved:Intent=serde_json::from_slice(&std::fs::read(disk.join("current.json")).unwrap()).unwrap();assert!(saved.room.is_none());assert_eq!(saved.marker(),event["content"]);
                    // The server applied the effect, but its HTTP response was lost.
                    ResponseTemplate::new(500).set_body_json(json!({"errcode":"M_UNKNOWN","error":"lost response"}))
                }).mount(&server).await;
                let config=CreateRoomConfig{space,encrypted:!space,parent:(!space).then(||"!parent:example.org".try_into().unwrap()),..config()};
                let mut unavailable=j.clone();unavailable.root=root.path().join("not-a-directory/owner");std::fs::write(root.path().join("not-a-directory"),b"keep").unwrap();
                assert!(create_in(&client,config.clone(),agent,project.clone(),None,&unavailable).await.is_err());
                assert!(!server.received_requests().await.unwrap().iter().any(|r|r.url.path().ends_with("/createRoom")),"no createRoom before durable intent");
                let lost=create_in(&client,config.clone(),agent,project.clone(),None,&j).await;
                assert!(lost.is_err());
                assert_eq!(server.received_requests().await.unwrap().iter().filter(|r|r.url.path().ends_with("/createRoom")).count(),1,"SDK must not replay createRoom: {lost:?}");
                let saved=j.pending().await.unwrap().unwrap();assert!(saved.room.is_none());assert_eq!(saved.project,project);
                assert!(create_in(&client,config.clone(),agent,None,None,&j).await.is_err());
                Mock::given(method("GET")).and(path_regex(".*/joined_rooms$")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"joined_rooms":["!original:example.org"]}))).mount(&server).await;
                let m=marker.lock().unwrap().clone();
                Mock::given(method("GET")).and(path_regex(".*/state/org.hagency.creation.v1/?$")).respond_with(ResponseTemplate::new(200).set_body_json(m.clone())).mount(&server).await;
                let mut state=vec![json!({"type":MARKER,"state_key":"","sender":"@alice:example.org","content":m}),json!({"type":"m.room.create","state_key":"","sender":"@alice:example.org","content":if space{json!({"type":"m.space"})}else{json!({})}}),json!({"type":"m.room.name","state_key":"","content":{"name":config.name}}),json!({"type":"m.room.join_rules","state_key":"","content":{"join_rule":"invite"}}),json!({"type":"m.room.member","state_key":"@alice:example.org","content":{"membership":"join"}})];
                if !space {state.push(json!({"type":"m.room.encryption","state_key":"","content":{"algorithm":"m.megolm.v1.aes-sha2"}}));}
                Mock::given(method("GET")).and(path_regex(".*/rooms/.*/state$")).respond_with(move|r:&wiremock::Request|if r.url.path().contains("foreign"){ResponseTemplate::new(200).set_body_json(json!([]))}else{ResponseTemplate::new(200).set_body_json(&state)}).mount(&server).await;
                assert!(recover_in(&client,&saved,"!foreign:example.org",&j).await.is_err());
                assert!(j.pending().await.unwrap().unwrap().room.is_none());
                if agent {
                    // The archived request can be verified while a newer creation is pending.
                    j.archive(saved.operation).await.unwrap();
                    let next = j.begin(config.clone(), false, None).await.unwrap();
                    let unknown = j.review_records().await.unwrap().unresolved.pop().unwrap();
                    assert!(recover_in(&client, &unknown, "!foreign:example.org", &j).await.is_err());
                    assert_eq!(j.review_records().await.unwrap().unresolved.len(), 1);
                    let verified = recover_in(&client, &unknown, "!original:example.org", &j).await.unwrap();
                    assert_eq!(verified.room.as_deref().unwrap().as_str(), "!original:example.org");
                    let view = j.review_records().await.unwrap();
                    assert!(view.unresolved.is_empty());
                    assert_eq!(view.current.unwrap().operation, next.operation);
                } else {
                    let recovered = recover_in(&client, &saved, "", &j).await.unwrap();
                    assert_eq!(recovered.operation, saved.operation);
                    assert_eq!(recovered.room.as_deref().unwrap().as_str(), "!original:example.org");
                    let reopened = j.pending().await.unwrap().unwrap();
                    assert_eq!(reopened.room, recovered.room);
                    assert_eq!(reopened.agent_space, agent);
                    assert_eq!(reopened.config.parent, config.parent);
                }
                assert_eq!(server.received_requests().await.unwrap().iter().filter(|r|r.url.path().ends_with("/createRoom")).count(),1);
                crate::sliding_sync::test_set_current_client(None);
            }
        });
    }
}
