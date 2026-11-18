//! Recover only after OAuth supplies the account and device identities.
use super::*;
use hagency_platform::private_state::{AccountRepairPath, Error as PrivateError};
use std::{ffi::OsStr, fs::File, io::Read};

/// A protected v2 envelope can establish the previous device even when Windows
/// has widened its file ACL. No old grant is installed and no file is modified.
pub(crate) async fn authenticated_reauth_database(
    user: &OwnedUserId,
) -> Result<Option<ExistingDatabase>> {
    let service = global()?.clone();
    let user = user.clone();
    let original = service.clone();
    service
        .worker
        .call(move || Ok(original.read_reauth_database(&user)))
        .await?
}

impl Service {
    fn read_reauth_database(&self, user: &OwnedUserId) -> Result<Option<ExistingDatabase>> {
        let state = crate::accounts::account_dir(&self.root, user).join("persistent_state");
        let held = AccountRepairPath::at(&state)?;
        let mut file = held.open_owned_file("session")?;
        let bytes = bounded_bytes(&mut file)?.ok_or(Error::Invalid)?;
        let Ok(stored) = serde_json::from_slice::<Stored>(&bytes) else {
            return Ok(None);
        };
        ensure!(&stored.identity.owner == user, Error::Invalid);
        match self.open_v2_database(stored) {
            Ok(database) => Ok(Some(database)),
            Err(e) if e.downcast_ref::<Error>() == Some(&Error::Missing) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub(crate) async fn recover_verified_account(
    client: &matrix_sdk::Client,
    epoch: u64,
) -> Result<bool> {
    let user = client.user_id().ok_or(Error::Invalid)?.to_owned();
    let device = client.device_id().ok_or(Error::Invalid)?.to_string();
    let homeserver = client.homeserver().to_string();
    let service = global()?.clone();
    let original = service.clone();
    service
        .worker
        .call(move || Ok(original.recover_permissions(&user, &device, &homeserver, epoch)))
        .await?
}

impl Service {
    pub(super) fn recover_permissions(
        &self,
        user: &OwnedUserId,
        device: &str,
        homeserver: &str,
        epoch: u64,
    ) -> Result<bool> {
        let account = crate::accounts::account_dir(&self.root, user);
        let state = account.join("persistent_state");
        // Serialize account retirement with repairs and archival just as session publication does.
        let gate = crate::persistence::SESSION_IO
            .lock()
            .map_err(|_| Error::Storage)?;
        let mut held = AccountRepairPath::at(&self.root).map_err(|e| at(&self.root, e))?;
        ensure!(crate::account_session::epoch() == epoch, Error::Retired);
        if !held
            .enter(account.file_name().ok_or(Error::Invalid)?)
            .map_err(|e| at(&account, e))?
            || !held
                .enter(OsStr::new("persistent_state"))
                .map_err(|e| at(&state, e))?
        {
            return Ok(false);
        }
        drop(gate);
        let mut archived = false;
        for name in ["session", "retained_db.json"] {
            let gate = crate::persistence::SESSION_IO
                .lock()
                .map_err(|_| Error::Storage)?;
            ensure!(crate::account_session::epoch() == epoch, Error::Retired);
            let path = state.join(name);
            let lock = format!(".{name}.vault-lock");
            held.repair_file(&lock, None, |file| Ok(file.metadata()?.len() == 0))
                .map_err(|e| at(&state.join(lock), e))?;
            let pending_archive = format!("{name}.unavailable-pending");
            let staging = state.join(&pending_archive);
            let journal_path = state.join(format!(".{name}.vault-journal"));
            if !held.needs_document_repair(name).map_err(|e| at(&path, e))?
                && !staging.try_exists().map_err(|e| at(&staging, e))?
                && !journal_path
                    .try_exists()
                    .map_err(|e| at(&journal_path, e))?
            {
                continue;
            }
            let document = Document::open(&path, MAX_SESSION).map_err(|e| at(&path, e))?;
            // Another process can finish or create a transaction before we acquire its lock.
            let resume_archive = staging.try_exists().map_err(|e| at(&staging, e))?;
            let has_journal = journal_path
                .try_exists()
                .map_err(|e| at(&journal_path, e))?;
            if resume_archive {
                // The process may have stopped before publishing the verified subset.
                // Seal the staging handle before reading or publishing any verified subset.
                held.repair_file(&pending_archive, None, |_| Ok(true))
                    .map_err(|e| at(&staging, e))?;
                if !has_journal && name == "retained_db.json" && !path.try_exists()? {
                    let mut file = held.open_owned_file(&pending_archive)?;
                    if let Some(bytes) = bounded_bytes(&mut file)? {
                        let mut subset = None;
                        if self.verify_list(
                            &bytes,
                            user,
                            device,
                            homeserver,
                            &account,
                            &mut subset,
                        )? {
                            subset = Some(bytes);
                        }
                        if let Some(bytes) = subset {
                            hagency_store::private::replace(&path, &bytes)?;
                        }
                    }
                }
                // Leave the staging name in place until we have rechecked any live
                // source. An interrupted deletion can then reuse the durable copy.
                archived = true;
            }
            if has_journal {
                let journal = document
                    .pending_for_installation(&self.existing_installation()?)
                    .map_err(|e| at(&journal_path, e))?
                    .ok_or(Error::Conflict)?;
                held.repair_file(name, Some(&pending_archive), |file| {
                    let bytes = bounded_bytes(file)?.ok_or(PrivateError::Private)?;
                    if journal
                        .matches_original(file, &bytes)
                        .map_err(|_| PrivateError::Private)?
                        || journal.matches_candidate_bytes(&bytes)
                    {
                        Ok(true)
                    } else {
                        Err(PrivateError::Private)
                    }
                })
                .map_err(|e| at(&path, e))?;
                held.repair_file(journal.candidate_name(), None, |file| {
                    Ok(bounded_bytes(file)?.is_some_and(|b| journal.matches_candidate_bytes(&b)))
                })
                .map_err(|e| at(&journal_path, e))?;
                // With only an intent, the normal save will resume its original refs.
                // A legacy migration has its own resumable plaintext source.
                let replay_session = name == "session"
                    && (journal.candidate_digest().is_some()
                        || document.read()?.is_some_and(|source| {
                            serde_json::from_slice::<FullSessionPersisted>(source.bytes()).is_ok()
                        }));
                // The existing journal machinery takes both locks itself and rechecks
                // the installation, source identity, candidate digest, scopes and epoch.
                drop(journal);
                drop(document);
                drop(gate);
                if replay_session {
                    self.load_worker(&path, epoch, || crate::account_session::epoch() == epoch)
                        .map_err(|e| at(&path, e))?;
                } else if name == "retained_db.json" {
                    self.retain_worker(&path, Some(user.clone()), None, epoch)
                        .map_err(|e| at(&path, e))?;
                }
                continue;
            }
            let mut verified_subset = None;
            let preserved = held
                .repair_file(name, Some(&pending_archive), |file| {
                    let Some(bytes) = bounded_bytes(file)? else {
                        return Ok(false);
                    };
                    if name == "session" {
                        // Never promote a previously writable plaintext access/refresh token.
                        let Ok(stored) = serde_json::from_slice::<Stored>(&bytes) else {
                            return Ok(false);
                        };
                        if &stored.identity.owner != user
                            || stored.identity.homeserver.trim_end_matches('/')
                                != homeserver.trim_end_matches('/')
                        {
                            return Ok(false);
                        }
                        return match self.read_original(&stored) {
                            Ok(_) => Ok(true),
                            // The existing missing-key recovery checks the grant and preserves it.
                            Err(e) if e.downcast_ref::<Error>() == Some(&Error::Missing) => {
                                Ok(true)
                            }
                            Err(e)
                                if matches!(
                                    e.downcast_ref::<Error>(),
                                    Some(Error::Integrity | Error::Invalid)
                                ) =>
                            {
                                Ok(false)
                            }
                            Err(_) => Err(PrivateError::PlatformUnavailable),
                        };
                    }
                    self.verify_list(
                        &bytes,
                        user,
                        device,
                        homeserver,
                        &account,
                        &mut verified_subset,
                    )
                })
                .map_err(|e| at(&path, e))?;
            if let Some(bytes) = verified_subset {
                hagency_store::private::replace(&path, &bytes).map_err(|e| at(&path, e))?;
            }
            if preserved || resume_archive {
                finish_archive(&held, &state, name, &pending_archive)?;
                archived = true;
            }
        }
        Ok(archived)
    }

    fn verify_list(
        &self,
        bytes: &[u8],
        user: &OwnedUserId,
        device: &str,
        homeserver: &str,
        account: &Path,
        subset: &mut Option<Zeroizing<Vec<u8>>>,
    ) -> Result<bool, PrivateError> {
        let verify = |path: &Path, server: &str, secret: &str| -> Result<bool, PrivateError> {
            let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                return Ok(false);
            };
            let local = self.root.join(name);
            if !(path == Path::new(name) || path == local)
                || !name.starts_with("db_")
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
                || server.trim_end_matches('/') != homeserver.trim_end_matches('/')
                || secret.is_empty()
            {
                return Ok(false);
            }
            let directory = match AccountRepairPath::at(&local) {
                Ok(directory) => directory,
                Err(PrivateError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(false);
                }
                Err(PrivateError::Private) => return Ok(false),
                Err(e) => return Err(e),
            };
            // An exposed passphrase cannot authenticate a database which was itself
            // writable by other principals. Only an independently private SDK store
            // can corroborate the repaired list. Never change permissions on that store.
            match hagency_platform::private_state::directory(&local) {
                Err(PrivateError::Private) => return Ok(false),
                other => other?,
            }
            for entry in std::fs::read_dir(&local)? {
                let entry = entry?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    return Ok(false);
                };
                match directory.needs_file_repair(name) {
                    Ok(false) => (),
                    Ok(true) | Err(PrivateError::Private) => return Ok(false),
                    Err(err) => return Err(err),
                }
            }
            use database_inspect::InspectAccountError;
            match database_inspect::inspect_existing_account_identity(&local, Some(secret)) {
                Ok(id) => Ok(id.user_id() == user && id.device_id().as_str() == device),
                Err(InspectAccountError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                    Ok(false)
                }
                Err(InspectAccountError::Io(e)) => Err(e.into()),
                // Preserve the live list on operational SQLite errors, including BUSY,
                // LOCKED and IOERR. Do not convert a failed inspection into a mismatch.
                Err(InspectAccountError::Sqlite(e))
                    if matches!(
                        e.sqlite_error_code(),
                        Some(
                            rusqlite::ErrorCode::DatabaseCorrupt
                                | rusqlite::ErrorCode::NotADatabase
                        )
                    ) =>
                {
                    Ok(false)
                }
                Err(InspectAccountError::Sqlite(e)) => Err(std::io::Error::other(e).into()),
                Err(_) => Ok(false),
            }
        };
        if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'[') {
            let Ok(list) = serde_json::from_slice::<Vec<ClientSessionPersisted>>(bytes) else {
                return Ok(false);
            };
            if list.len() > 128 {
                return Ok(false);
            }
            let mut seen = std::collections::HashSet::new();
            let mut valid = Vec::new();
            for db in &list {
                if verify(&db.db_path, &db.homeserver, &db.passphrase)? && seen.insert(&db.db_path)
                {
                    valid.push(db);
                }
            }
            if valid.len() == list.len() {
                return Ok(true);
            }
            if !valid.is_empty() {
                *subset = Some(Zeroizing::new(
                    serde_json::to_vec(&valid).map_err(|_| PrivateError::Private)?,
                ));
            }
            return Ok(false);
        }
        // Versioned lists have native, scoped references. Check their binding before use.
        let installation = self
            .existing_installation()
            .map_err(|_| PrivateError::PlatformUnavailable)?;
        let hash = account
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(PrivateError::Private)?;
        let entries = match self.retained_entries(bytes, hash, &installation, true) {
            Ok(entries) => entries,
            Err(e)
                if matches!(
                    e.downcast_ref::<Error>(),
                    Some(Error::Integrity | Error::Invalid)
                ) =>
            {
                return Ok(false);
            }
            Err(e) if e.downcast_ref::<Error>().is_some() => {
                return Err(PrivateError::PlatformUnavailable);
            }
            Err(_) => return Ok(false),
        };
        let total = entries.len();
        let mut verified = Vec::new();
        for entry in entries {
            let scope = entry
                .identity
                .scope(&installation)
                .map_err(|_| PrivateError::Private)?;
            match self.vault.load(&entry.database.reference, &scope) {
                Ok(key) => {
                    if let Ok(secret) = std::str::from_utf8(&key)
                        && verify(&entry.database.db_path, &entry.database.homeserver, secret)?
                    {
                        verified.push(entry);
                    }
                }
                Err(Error::Missing) => (),
                Err(_) => return Err(PrivateError::PlatformUnavailable),
            }
        }
        if verified.len() == total {
            return Ok(true);
        }
        if !verified.is_empty() {
            let selected = retained::Retained {
                version: VERSION,
                account_hash: hash.to_owned(),
                databases: verified,
            };
            *subset = Some(Zeroizing::new(
                serde_json::to_vec(&selected).map_err(|_| PrivateError::Private)?,
            ));
        }
        Ok(false)
    }
}

fn finish_archive(held: &AccountRepairPath, state: &Path, name: &str, pending: &str) -> Result<()> {
    let directory = cap_std::fs::Dir::open_ambient_dir(state, cap_std::ambient_authority())?;
    // Make both the protected archive and any reconstructed active list durable
    // before removing the restart marker.
    hagency_platform::private_state::sync_directory(&directory)?;
    let archive = format!("{name}.unavailable-{}", uuid::Uuid::new_v4().simple());
    held.rename_private_file(pending, &archive)?;
    hagency_platform::private_state::sync_directory(&directory)?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(crate) struct PermissionRepairLocation(String);

fn at(path: &Path, error: impl Into<anyhow::Error>) -> anyhow::Error {
    let error = error.into();
    let message = crate::i18n::format(
        "Could not secure local sign-in storage at {path}. {reason}",
        &[
            ("path", path.display().to_string()),
            ("reason", error.root_cause().to_string()),
        ],
    );
    error.context(PermissionRepairLocation(message))
}

fn bounded_bytes(file: &mut File) -> Result<Option<Zeroizing<Vec<u8>>>, PrivateError> {
    if file.metadata()?.len() > MAX_SESSION as u64 {
        return Ok(None);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX_SESSION as u64 + 1).read_to_end(&mut bytes)?;
    Ok((bytes.len() <= MAX_SESSION).then_some(bytes))
}
