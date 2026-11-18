//! Recover only after OAuth supplies the account and device identities.
use super::*;
use hagency_platform::private_state::{AccountRepairPath, Error as PrivateError};
use std::{ffi::OsStr, fs::File, io::Read};

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
    fn recover_permissions(
        &self,
        user: &OwnedUserId,
        device: &str,
        homeserver: &str,
        epoch: u64,
    ) -> Result<bool> {
        let account = crate::accounts::account_dir(&self.root, user);
        let state = account.join("persistent_state");
        // Serialize account retirement with repairs and archival just as session publication does.
        let _gate = crate::persistence::SESSION_IO
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
        let mut archived = false;
        for name in ["session", "retained_db.json"] {
            let path = state.join(name);
            let lock = format!(".{name}.vault-lock");
            held.repair_file(&lock, None, |file| Ok(file.metadata()?.len() == 0))
                .map_err(|e| at(&state.join(lock), e))?;
            if !held.needs_document_repair(name).map_err(|e| at(&path, e))? {
                continue;
            }
            let _document = Document::open(&path, MAX_SESSION).map_err(|e| at(&path, e))?;
            // Archives cannot orphan a partially published document transaction.
            match std::fs::symlink_metadata(state.join(format!(".{name}.vault-journal"))) {
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => (),
                Err(err) => return Err(at(&path, err)),
                Ok(_) => return Err(at(&path, Error::Conflict)),
            }
            let archive = format!("{name}.unavailable-{}", uuid::Uuid::new_v4().simple());
            ensure!(crate::account_session::epoch() == epoch, Error::Retired);
            let mut verified_subset = None;
            let preserved = held
                .repair_file(name, Some(&archive), |file| {
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
            if preserved {
                if let Some(bytes) = verified_subset {
                    hagency_store::private::replace(&path, &bytes).map_err(|e| at(&path, e))?;
                }
                let directory =
                    cap_std::fs::Dir::open_ambient_dir(&state, cap_std::ambient_authority())?;
                hagency_platform::private_state::sync_directory(&directory)
                    .map_err(|e| at(&state, e))?;
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
        let verify = |path: &Path, server: &str, secret: &str| {
            let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                return false;
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
                return false;
            }
            let Ok(directory) = AccountRepairPath::at(&local) else {
                return false;
            };
            // An exposed passphrase cannot authenticate a database which was itself
            // writable by other principals. Only an independently private SDK store
            // can corroborate the repaired list. Never change permissions on that store.
            if hagency_store::private::directory(&local).is_err()
                || !private_database_files(&directory, &local)
            {
                return false;
            }
            database_inspect::inspect_existing_account_identity(&local, Some(secret))
                .is_ok_and(|id| id.user_id() == user && id.device_id().as_str() == device)
        };
        if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'[') {
            let Ok(list) = serde_json::from_slice::<Vec<ClientSessionPersisted>>(bytes) else {
                return Ok(false);
            };
            if list.len() > 128 {
                return Ok(false);
            }
            let mut seen = std::collections::HashSet::new();
            let valid: Vec<_> = list
                .iter()
                .filter(|db| {
                    verify(&db.db_path, &db.homeserver, &db.passphrase) && seen.insert(&db.db_path)
                })
                .collect();
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
                Ok(key)
                    if std::str::from_utf8(&key).is_ok_and(|s| {
                        verify(&entry.database.db_path, &entry.database.homeserver, s)
                    }) =>
                {
                    verified.push(entry);
                }
                Ok(_) | Err(Error::Missing) => (),
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

fn private_database_files(directory: &AccountRepairPath, path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    entries.into_iter().all(|entry| {
        entry.ok().is_some_and(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| matches!(directory.needs_file_repair(name), Ok(false)))
        })
    })
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
