//! The owner's choice, for each pool, to run its Claude Agents in a
//! Docker-compatible container instead of the macOS sandbox (see
//! `hagency_runtime::owned::container`). The container runtime is this
//! computer's and serves every pool that uses it. Desktop writes the choice
//! into the owner daemon's home; the daemon reads it at each pooled Claude
//! launch, so a change applies from the pool's next run. No file, or a pool
//! the file does not list, means the sandbox.
use hagency_runtime::owned::container::{self, ContainerIsolation};
pub use hagency_runtime::owned::container::ProbeError;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const FILE: &str = "pool-containers.json";
/// Held while the choice is read and rewritten, so that two windows never
/// drop each other's pools.
const LOCK: &str = "pool-containers.lock";
/// The client's own settings while it runs pool Agents: empty, private, and
/// outside every run's folders.
const CLIENT_CONFIG: &str = "container-client";
/// Far more pools than one computer hosts.
const MAX_POOLS: usize = 256;
const MAX_POOL_ID: usize = 128;
const MAX_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Setting {
    /// The Docker-compatible client.
    pub cli: PathBuf,
    /// Its engine's socket, `unix://…`.
    pub endpoint: String,
    /// The ID of the image pool Agents run.
    pub image: String,
    /// The pools (their resource IDs) whose Agents run in containers; never
    /// empty.
    pub pools: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The file could not be read or written.
    State,
    /// What the file says cannot run, or a pool ID is not one.
    Invalid,
    /// Turning on stopped at this step.
    Probe(ProbeError),
}
impl From<ProbeError> for Error {
    fn from(error: ProbeError) -> Self {
        Self::Probe(error)
    }
}

/// The saved choice, `None` when every pool uses the sandbox.
pub fn load(daemon_home: &Path) -> Result<Option<Setting>, Error> {
    let file = match hagency_store::private::open(&daemon_home.join(FILE), false) {
        Ok(file) => file,
        Err(hagency_store::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(_) => return Err(Error::State),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|_| Error::State)?;
    if bytes.len() > MAX_BYTES {
        return Err(Error::Invalid);
    }
    let setting: Setting = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
    if setting.pools.is_empty()
        || setting.pools.len() > MAX_POOLS
        || !setting.pools.iter().all(|pool| pool_id(pool))
    {
        return Err(Error::Invalid);
    }
    Ok(Some(setting))
}

/// What a launch for `pool` runs in: `None` for the sandbox. A file that
/// cannot be read refuses every pool's launch, since it cannot say which
/// pools the owner moved into containers.
pub fn isolation(daemon_home: &Path, pool: &str) -> Result<Option<ContainerIsolation>, Error> {
    let Some(setting) = load(daemon_home)? else {
        return Ok(None);
    };
    if !setting.pools.contains(pool) {
        return Ok(None);
    }
    let config = client_config(daemon_home)?;
    ContainerIsolation::new(setting.cli, setting.endpoint, setting.image, config)
        .map(Some)
        .map_err(|_| Error::Invalid)
}

/// Runs `pool`'s Agents in containers from their next run. Finds the client
/// and its engine and builds the image when the engine does not have it
/// (minutes the first time); the runtime found then serves every pool listed.
/// Blocking: never on the UI thread.
pub fn turn_on(daemon_home: &Path, home: &Path, pool: &str) -> Result<Setting, Error> {
    if !pool_id(pool) {
        return Err(Error::Invalid);
    }
    let cli = container::find_client(home).ok_or(ProbeError::NoClient)?;
    let endpoint = container::current_endpoint(&cli, home)?;
    let config = client_config(daemon_home)?;
    let image = container::prepare_image(&cli, &endpoint, &config, home)?;
    add(daemon_home, Runtime { cli, endpoint, image }, pool)
}

/// `pool` back to the sandbox from its next run; the file goes with the last
/// pool. A file that cannot be read goes too: it refuses every pool's launch,
/// and turning a pool off is the owner's way out.
pub fn turn_off(daemon_home: &Path, pool: &str) -> Result<(), Error> {
    let _lock = lock(daemon_home)?;
    let mut setting = match load(daemon_home) {
        Ok(Some(setting)) => setting,
        Ok(None) => return Ok(()),
        Err(Error::Invalid) => return remove(daemon_home),
        Err(error) => return Err(error),
    };
    if !setting.pools.remove(pool) {
        return Ok(());
    }
    if setting.pools.is_empty() {
        return remove(daemon_home);
    }
    save(daemon_home, &setting)
}

/// The container runtime turning on found on this computer.
struct Runtime {
    cli: PathBuf,
    endpoint: String,
    image: String,
}

/// Lists `pool` under the runtime just found, which replaces the one saved
/// for every pool.
fn add(daemon_home: &Path, runtime: Runtime, pool: &str) -> Result<Setting, Error> {
    let config = client_config(daemon_home)?;
    // What a launch will accept.
    ContainerIsolation::new(runtime.cli.clone(), runtime.endpoint.clone(), runtime.image.clone(), config)
        .map_err(|_| Error::Invalid)?;
    let _lock = lock(daemon_home)?;
    // A file that cannot be read stays as it is: it may name pools this
    // change cannot see.
    let mut pools = load(daemon_home)?.map(|setting| setting.pools).unwrap_or_default();
    pools.insert(pool.to_owned());
    if pools.len() > MAX_POOLS {
        return Err(Error::Invalid);
    }
    let setting = Setting { cli: runtime.cli, endpoint: runtime.endpoint, image: runtime.image, pools };
    save(daemon_home, &setting)?;
    Ok(setting)
}

fn save(daemon_home: &Path, setting: &Setting) -> Result<(), Error> {
    let bytes = serde_json::to_vec(setting).map_err(|_| Error::State)?;
    hagency_store::private::replace(&daemon_home.join(FILE), &bytes).map_err(|_| Error::State)
}

fn remove(daemon_home: &Path) -> Result<(), Error> {
    match std::fs::remove_file(daemon_home.join(FILE)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(Error::State),
    }
}

fn lock(daemon_home: &Path) -> Result<File, Error> {
    let path = daemon_home.join(LOCK);
    let file = match hagency_store::private::open(&path, true) {
        Ok(file) => file,
        Err(hagency_store::Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            hagency_store::private::open(&path, false).map_err(|_| Error::State)?
        }
        Err(_) => return Err(Error::State),
    };
    file.lock().map_err(|_| Error::State)?;
    Ok(file)
}

/// A resource ID as the owner's store makes them.
fn pool_id(id: &str) -> bool {
    hagency_core::project::identifier(id, MAX_POOL_ID).is_ok()
}

fn client_config(daemon_home: &Path) -> Result<PathBuf, Error> {
    let path = daemon_home.join(CLIENT_CONFIG);
    hagency_store::private::directory(&path).map_err(|_| Error::State)?;
    path.canonicalize().map_err(|_| Error::State)
}

static SERVED: OnceLock<PathBuf> = OnceLock::new();
/// The owner daemon names its home before hosting starts. Elsewhere pool
/// Agents keep the sandbox. The daemon is built for Unix only.
#[cfg(unix)]
pub(crate) fn serve_from(daemon_home: &Path) {
    let _ = SERVED.set(daemon_home.to_path_buf());
}
/// What the daemon this process is runs `pool`'s Agents in.
pub(crate) fn hosted_isolation(origin: &str, owner: &str, pool: &str) -> Result<Option<ContainerIsolation>, Error> {
    match SERVED.get() {
        Some(base) => isolation(&crate::daemon::accounts::home(base, origin, owner).map_err(|_| Error::State)?, pool),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().canonicalize().unwrap();
        (dir, home)
    }

    fn runtime(home: &Path, image: char) -> Runtime {
        let cli = home.join("docker");
        std::fs::write(&cli, "").unwrap();
        Runtime {
            cli,
            endpoint: "unix:///var/run/docker.sock".into(),
            image: format!("sha256:{}", image.to_string().repeat(64)),
        }
    }

    #[test]
    fn each_pool_keeps_its_own_choice_under_this_computers_runtime() {
        let (_dir, home) = home();
        assert_eq!(load(&home), Ok(None));
        assert!(isolation(&home, "resource_a").unwrap().is_none());
        add(&home, runtime(&home, 'a'), "resource_a").unwrap();
        assert_eq!(load(&home).unwrap().unwrap().pools, BTreeSet::from(["resource_a".to_owned()]));
        let running = isolation(&home, "resource_a").unwrap().unwrap();
        assert_eq!(running.cli(), home.join("docker").as_path());
        assert!(isolation(&home, "resource_b").unwrap().is_none(), "other pools keep the sandbox");
        assert!(home.join(CLIENT_CONFIG).is_dir());
        // A second pool: the runtime found last serves both.
        let saved = add(&home, runtime(&home, 'b'), "resource_b").unwrap();
        assert_eq!(saved.pools.len(), 2);
        assert_eq!(isolation(&home, "resource_a").unwrap().unwrap().image(), saved.image);
        add(&home, runtime(&home, 'b'), "resource_b").unwrap();
        assert_eq!(load(&home).unwrap().unwrap().pools.len(), 2, "listed once");
        // Off for one pool keeps the other's.
        turn_off(&home, "resource_a").unwrap();
        turn_off(&home, "resource_a").unwrap();
        turn_off(&home, "resource_never").unwrap();
        assert!(isolation(&home, "resource_a").unwrap().is_none());
        assert!(isolation(&home, "resource_b").unwrap().is_some());
        // The last pool takes the file with it.
        turn_off(&home, "resource_b").unwrap();
        assert!(!home.join(FILE).exists());
        assert_eq!(load(&home), Ok(None));
        turn_off(&home, "resource_b").unwrap();
    }

    #[test]
    fn a_file_that_cannot_run_is_refused_not_ignored() {
        let (_dir, home) = home();
        let image = format!("sha256:{}", "a".repeat(64));
        let too_many: Vec<_> = (0..=MAX_POOLS).map(|n| format!("resource_{n}")).collect();
        for text in [
            r#"{"cli":"/usr/local/bin/docker"}"#.to_owned(),
            format!(r#"{{"cli":"/nowhere/docker","endpoint":"unix:///s","image":"{image}"}}"#),
            format!(r#"{{"cli":"/nowhere/docker","endpoint":"unix:///s","image":"{image}","pools":[]}}"#),
            format!(r#"{{"cli":"/nowhere/docker","endpoint":"unix:///s","image":"{image}","pools":["../a"]}}"#),
            format!(r#"{{"cli":"/nowhere/docker","endpoint":"unix:///s","image":"{image}","pools":["a"],"extra":1}}"#),
            serde_json::json!({"cli":"/nowhere/docker","endpoint":"unix:///s","image":image,"pools":too_many}).to_string(),
            "not json".to_owned(),
        ] {
            hagency_store::private::replace(&home.join(FILE), text.as_bytes()).unwrap();
            assert_eq!(load(&home).err(), Some(Error::Invalid), "{text}");
            // It cannot say which pools chose containers: every launch stops.
            assert_eq!(isolation(&home, "a").err(), Some(Error::Invalid), "{text}");
            assert_eq!(isolation(&home, "resource_other").err(), Some(Error::Invalid), "{text}");
            // Turning another pool on leaves it for the owner to see.
            assert_eq!(add(&home, runtime(&home, 'a'), "resource_new").err(), Some(Error::Invalid));
            assert_eq!(std::fs::read_to_string(home.join(FILE)).unwrap(), text);
        }
        // Turning a pool off is the way out.
        turn_off(&home, "a").unwrap();
        assert_eq!(load(&home), Ok(None));
        // A listed pool whose client is gone cannot start; the others can.
        let gone = format!(r#"{{"cli":"/nowhere/docker","endpoint":"unix:///s","image":"{image}","pools":["resource_a"]}}"#);
        hagency_store::private::replace(&home.join(FILE), gone.as_bytes()).unwrap();
        assert_eq!(isolation(&home, "resource_a").err(), Some(Error::Invalid));
        assert!(isolation(&home, "resource_b").unwrap().is_none());
        // Names that are not resource IDs are never listed.
        assert_eq!(turn_on(&home, &home, "../a").err(), Some(Error::Invalid));
        assert_eq!(turn_on(&home, &home, "").err(), Some(Error::Invalid));
    }
}
