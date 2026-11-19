//! An installation has one socket. Each hosted (origin, Matrix ID) has its
//! own enrollment, controller receipts, SDK device and runtime under it.
use super::enrollment::Enrollment;
use hagency_metering::daemon_endpoint::ACCOUNTS;
#[cfg(unix)]
use hagency_metering::daemon_endpoint::valid_key;
use std::path::{Path, PathBuf};

pub fn home(base: &Path, origin: &str, owner: &str) -> Result<PathBuf, &'static str> {
    let key = crate::owner_roots::key(origin, owner);
    let scoped = base.join(ACCOUNTS).join(key);
    // Keep an existing single-owner installation at its original location,
    // including its SDK credentials, control intent and unfinished receipts.
    if Enrollment::load(base)
        .map_err(|e| e.code())?
        .is_some_and(|legacy| legacy.owner == owner && super::same_origin(&legacy.origin, origin))
    {
        return Ok(base.to_owned());
    }
    Ok(scoped)
}

/// Reject an enrollment addressed to a different account before touching its runtime.
#[cfg(unix)]
pub(super) fn accepts(home: &Path, origin: &str, owner: &str) -> bool {
    let (base, scope) = hagency_metering::daemon_endpoint::endpoint(home);
    let key = crate::owner_roots::key(origin, owner);
    match scope {
        Some(scope) => scope == key,
        None => !base.join(ACCOUNTS).join(key).exists(),
    }
}

/// Create account state before contacting the running service. An offline
/// enrollment helper takes the same per-account lock as that service.
#[cfg(unix)]
pub(super) fn provision(base: &Path, home: &Path) -> Result<(), &'static str> {
    if let Some(parent) = base.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "daemon_home_unavailable")?;
    }
    for dir in [base, &base.join(ACCOUNTS), home] {
        hagency_store::private::directory(dir).map_err(|_| "daemon_home_unavailable")?;
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn directories(base: &Path) -> std::io::Result<Vec<(String, PathBuf)>> {
    let directory = base.join(ACCOUNTS);
    match std::fs::symlink_metadata(&directory) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        entry => {
            entry?;
        }
    }
    hagency_store::private::directory(&directory).map_err(std::io::Error::other)?;
    let entries = std::fs::read_dir(directory)?;
    let mut homes = Vec::new();
    for entry in entries {
        let entry = entry?;
        let key = entry.file_name();
        if let Some(key) = key.to_str().filter(|key| valid_key(key))
            && entry.file_type()?.is_dir()
        {
            homes.push((key.to_owned(), entry.path()));
        }
    }
    Ok(homes)
}
