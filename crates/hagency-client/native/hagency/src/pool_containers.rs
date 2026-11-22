//! The owner's choice, for each pool, to run its Claude Agents in a
//! Docker-compatible container instead of the macOS sandbox (see
//! `hagency_runtime::owned::container`). The container runtime is this
//! computer's and serves every pool that uses it. Desktop writes the choice
//! into the owner daemon's home; the daemon reads it at each pooled Claude
//! launch, so a change applies from the pool's next run. No file, or a pool
//! the file does not list, means the sandbox.
//!
//! A pool created with containers chosen is listed before anything is
//! prepared, with no runtime when none was found yet: from then on its Agents
//! run in a container or do not start. The choice is kept with the command
//! that creates the pool from before the pool exists, so it holds however the
//! command's result arrives.
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
/// The commands that create a pool whose owner chose containers, each until
/// its pool's ID is known.
const REQUESTS: &str = "pool-container-requests.json";
/// A command is here for the moments its pool takes to appear; one that
/// never made a pool leaves with the oldest.
const MAX_REQUESTS: usize = 64;
const MAX_COMMAND_ID: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Setting {
    /// The Docker-compatible client; empty while no runtime was found for
    /// the pools listed, whose Agents then do not start.
    pub cli: PathBuf,
    /// Its engine's socket, `unix://…`.
    pub endpoint: String,
    /// The ID of the image pool Agents run.
    pub image: String,
    /// The pools (their resource IDs) whose Agents run in containers; never
    /// empty.
    pub pools: BTreeSet<String>,
}
impl Setting {
    /// Whether turning on has found a runtime for the pools listed. An
    /// earlier host reads the same file: it cannot start a listed pool whose
    /// client is not a file, and so keeps these pools out of the sandbox too.
    pub fn found(&self) -> bool {
        !self.cli.as_os_str().is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The file could not be read or written.
    State,
    /// What the file says cannot run, or a pool ID is not one.
    Invalid,
    /// The pool's owner chose containers and no runtime was found for it yet.
    NotReady,
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
    if !setting.found() {
        return Err(Error::NotReady);
    }
    let config = client_config(daemon_home)?;
    ContainerIsolation::new(setting.cli, setting.endpoint, setting.image, config)
        .map(Some)
        .map_err(|_| Error::Invalid)
}

/// Lists `pool` as its owner's choice without preparing anything: from now on
/// its Agents run in a container or do not start. A runtime saved for other
/// pools serves it too; with none saved it waits for [`turn_on`]. Under the
/// lock. A file that cannot be read stays as it is and is an error here: it
/// refuses every launch now, and the owner's way out of it (turning a pool
/// off) would leave this pool in the sandbox.
fn list(daemon_home: &Path, pool: &str) -> Result<(), Error> {
    let mut setting = load(daemon_home)?.unwrap_or_else(|| Setting {
        cli: PathBuf::new(),
        endpoint: String::new(),
        image: String::new(),
        pools: BTreeSet::new(),
    });
    if !setting.pools.insert(pool.to_owned()) {
        return Ok(());
    }
    if setting.pools.len() > MAX_POOLS {
        return Err(Error::Invalid);
    }
    save(daemon_home, &setting)
}

/// The commands whose pool is to run in containers, oldest first.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Requests {
    commands: Vec<String>,
}

/// Before `command` creates a pool whose owner chose containers: keeps the
/// choice with the command, so that [`created`] can list the pool whichever
/// way its ID arrives (the first reply, a retry, a later recovery). An error
/// means the choice cannot be kept, and the pool should not be created.
pub fn expect(daemon_home: &Path, command: &str) -> Result<(), Error> {
    if !command_id(command) {
        return Err(Error::Invalid);
    }
    let _lock = lock(daemon_home)?;
    // The pool will have to be listed in this file: it has to be readable
    // and have room for one more.
    if load(daemon_home)?.is_some_and(|setting| setting.pools.len() >= MAX_POOLS) {
        return Err(Error::Invalid);
    }
    let mut requests = requests(daemon_home);
    if requests.commands.iter().any(|known| known == command) {
        return Ok(());
    }
    requests.commands.push(command.to_owned());
    let extra = requests.commands.len().saturating_sub(MAX_REQUESTS);
    requests.commands.drain(..extra);
    save_requests(daemon_home, &requests)
}

/// `command` created `pool`. Lists the pool when the command asked for
/// containers, and says whether it did. On an error the command stays known,
/// for the next time its result arrives.
pub fn created(daemon_home: &Path, command: &str, pool: &str) -> Result<bool, Error> {
    if !command_id(command) || !pool_id(pool) {
        return Err(Error::Invalid);
    }
    let _lock = lock(daemon_home)?;
    let mut requests = requests(daemon_home);
    let Some(at) = requests.commands.iter().position(|known| known == command) else {
        return Ok(false);
    };
    list(daemon_home, pool)?;
    requests.commands.remove(at);
    // The pool is listed; a command that stays known only lists it again.
    let _ = save_requests(daemon_home, &requests);
    Ok(true)
}

/// Under the lock. A list that cannot be read starts again: what it held
/// were commands waiting for their pool, not pools.
fn requests(daemon_home: &Path) -> Requests {
    let Ok(file) = hagency_store::private::open(&daemon_home.join(REQUESTS), false) else {
        return Requests::default();
    };
    let mut bytes = Vec::new();
    if file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes).is_err() || bytes.len() > MAX_BYTES {
        return Requests::default();
    }
    let mut requests: Requests = serde_json::from_slice(&bytes).unwrap_or_default();
    requests.commands.retain(|command| command_id(command));
    requests
}

fn save_requests(daemon_home: &Path, requests: &Requests) -> Result<(), Error> {
    let path = daemon_home.join(REQUESTS);
    if requests.commands.is_empty() {
        return match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(Error::State),
        };
    }
    let bytes = serde_json::to_vec(requests).map_err(|_| Error::State)?;
    hagency_store::private::replace(&path, &bytes).map_err(|_| Error::State)
}

/// Whether this computer has a Docker-compatible client where turning on
/// looks for one. Says nothing about its engine running.
pub fn client_found(home: &Path) -> bool {
    container::find_client(home).is_some()
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

/// A command ID as the host issues them.
fn command_id(id: &str) -> bool {
    hagency_core::project::identifier(id, MAX_COMMAND_ID).is_ok()
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

    /// Lists `pool` the way the result of its creating command does.
    fn choose(home: &Path, pool: &str) -> Result<(), Error> {
        expect(home, "command-choose")?;
        created(home, "command-choose", pool).map(|_| ())
    }

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
        assert_eq!(choose(&home, "../a").err(), Some(Error::Invalid));
    }

    #[test]
    fn a_pool_chosen_before_a_runtime_is_found_starts_in_a_container_or_not_at_all() {
        let (_dir, home) = home();
        choose(&home, "resource_new").unwrap();
        choose(&home, "resource_new").unwrap();
        let saved = load(&home).unwrap().unwrap();
        assert!(!saved.found());
        assert_eq!(saved.pools, BTreeSet::from(["resource_new".to_owned()]));
        assert_eq!(isolation(&home, "resource_new").err(), Some(Error::NotReady));
        assert!(isolation(&home, "resource_other").unwrap().is_none(), "other pools keep the sandbox");
        // What a host from before this choice existed goes by: the pool is
        // listed and its client is no file, so it does not start it either.
        let config = client_config(&home).unwrap();
        assert!(!saved.cli.is_file());
        assert!(ContainerIsolation::new(saved.cli, saved.endpoint, saved.image, config).is_err());
        // The runtime found for any pool serves the waiting one too.
        let found = add(&home, runtime(&home, 'b'), "resource_other").unwrap();
        assert!(found.found() && found.pools.len() == 2);
        assert_eq!(isolation(&home, "resource_new").unwrap().unwrap().image(), found.image);
        // A pool chosen while a runtime is saved runs under that runtime.
        choose(&home, "resource_third").unwrap();
        assert_eq!(isolation(&home, "resource_third").unwrap().unwrap().image(), found.image);
        assert_eq!(load(&home).unwrap().unwrap().cli, found.cli, "choosing never changes the runtime");
        // Off is still the owner's way back to the sandbox.
        for pool in ["resource_other", "resource_third"] {
            turn_off(&home, pool).unwrap();
        }
        assert!(isolation(&home, "resource_new").unwrap().is_some());
        turn_off(&home, "resource_new").unwrap();
        assert_eq!(load(&home), Ok(None));
        choose(&home, "resource_new").unwrap();
        turn_off(&home, "resource_new").unwrap();
        assert!(!home.join(FILE).exists(), "a waiting pool turned off takes the file with it");
        // A file that cannot be read is not cleared for a new pool's sake.
        hagency_store::private::replace(&home.join(FILE), b"not json").unwrap();
        assert_eq!(choose(&home, "resource_new").err(), Some(Error::Invalid));
        assert_eq!(std::fs::read(home.join(FILE)).unwrap(), b"not json");
    }

    #[test]
    fn a_creating_command_keeps_the_choice_until_its_pool_is_known() {
        let (_dir, home) = home();
        // A command that never asked changes nothing.
        assert_eq!(created(&home, "command-1", "resource_a"), Ok(false));
        assert_eq!(load(&home), Ok(None));
        expect(&home, "command-1").unwrap();
        expect(&home, "command-1").unwrap();
        assert_eq!(load(&home), Ok(None), "nothing is listed before the pool exists");
        // Its result, whenever it comes, lists the pool once.
        assert_eq!(created(&home, "command-1", "resource_a"), Ok(true));
        assert_eq!(isolation(&home, "resource_a").err(), Some(Error::NotReady));
        assert!(!home.join(REQUESTS).exists());
        assert_eq!(created(&home, "command-1", "resource_a"), Ok(false));
        assert_eq!(isolation(&home, "resource_a").err(), Some(Error::NotReady), "and it stays listed");
        // A pool that cannot be listed leaves its command known for the next
        // time the result arrives.
        turn_off(&home, "resource_a").unwrap();
        expect(&home, "command-2").unwrap();
        hagency_store::private::replace(&home.join(FILE), b"not json").unwrap();
        assert_eq!(created(&home, "command-2", "resource_b").err(), Some(Error::Invalid));
        // While the choices cannot be read, no new choice is taken.
        assert_eq!(expect(&home, "command-3").err(), Some(Error::Invalid));
        turn_off(&home, "resource_b").unwrap();
        assert_eq!(created(&home, "command-3", "resource_c"), Ok(false));
        assert_eq!(created(&home, "command-2", "resource_b"), Ok(true));
        assert_eq!(load(&home).unwrap().unwrap().pools, BTreeSet::from(["resource_b".to_owned()]));
        // Only so many commands wait; the oldest leave first.
        for n in 0..MAX_REQUESTS + 6 {
            expect(&home, &format!("waiting-{n}")).unwrap();
        }
        assert_eq!(created(&home, "waiting-5", "resource_old"), Ok(false));
        assert_eq!(created(&home, &format!("waiting-{}", MAX_REQUESTS + 5), "resource_last"), Ok(true));
        // A list that cannot be read starts again.
        hagency_store::private::replace(&home.join(REQUESTS), b"not json").unwrap();
        assert_eq!(created(&home, "waiting-6", "resource_lost"), Ok(false));
        expect(&home, "command-4").unwrap();
        assert_eq!(created(&home, "command-4", "resource_d"), Ok(true));
        // Only IDs are kept or listed.
        assert_eq!(expect(&home, "../command").err(), Some(Error::Invalid));
        assert_eq!(expect(&home, "").err(), Some(Error::Invalid));
        assert_eq!(created(&home, "../command", "resource_e").err(), Some(Error::Invalid));
        expect(&home, "command-5").unwrap();
        assert_eq!(created(&home, "command-5", "../e").err(), Some(Error::Invalid));
        // A list with no room for one more takes no new choice: its pool
        // could not be listed.
        let pools = (0..MAX_POOLS).map(|n| format!("resource_{n}")).collect();
        save(&home, &Setting { cli: PathBuf::new(), endpoint: String::new(), image: String::new(), pools }).unwrap();
        assert_eq!(expect(&home, "command-6").err(), Some(Error::Invalid));
        assert!(client_found(Path::new("/")) == container::find_client(Path::new("/")).is_some());
    }
}
