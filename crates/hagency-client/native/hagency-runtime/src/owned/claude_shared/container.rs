//! A shared Claude run in a container instead of the macOS sandbox.
//!
//! The owner turns this on for pool Agents on one computer. The container
//! gets the run's private scope and, in a space, the shared project folder,
//! each mounted at the path it has on the host, so the paths the CLI reports
//! and the host's file tools check stay the same. Nothing else of the host is
//! mounted. The CLI talks to the host over stdin and stdout as it does
//! natively: `run --interactive` carries the stream-json conversation, the
//! permission callback and the room's SDK MCP server unchanged.
//!
//! The container runs as the owner's user with no Linux capabilities, no new
//! privileges and a read-only image. Its temporary folder is the container's
//! own memory-backed /tmp: Claude Code's command sandbox keeps Unix sockets
//! there, which a folder shared from macOS cannot hold, and nothing in it
//! outlives the container. That sandbox (bubblewrap) keeps the credential
//! away from the commands a tool runs; it creates namespaces and mounts /proc
//! inside the container, so the runtime's syscall filter and /proc masking
//! are off. The credential reaches the CLI through the client's environment
//! by name only, never on a command line.
use super::super::{Cleanup, SharedProject, StartError};
use hagency_platform::Launch;
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, OsString},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// The image recipe. A build tags it with [`IMAGE_TAG_PREFIX`], the first 12
/// hex digits of its SHA-256 and the owner's user and group numbers, so a
/// changed recipe is a new image.
pub const DOCKERFILE: &str = include_str!("container/Dockerfile");
pub const IMAGE_TAG_PREFIX: &str = "hagency/claude-agent:";
/// Where the image keeps Claude Code.
const CLAUDE: &str = "/usr/local/bin/claude";
/// Every container of a shared Claude run carries this label,
const ROLE: &str = "io.hagency.role=shared-claude";
/// and this one, naming the host settings it was started with.
const HOST: &str = "io.hagency.host";
const TMP: &str = "/tmp";
/// Memory-backed, ends with the container. `exec` as in the host's own
/// temporary folder: tools run what they build there.
const TMP_MOUNT: &str = "/tmp:rw,exec,nosuid,nodev,size=1073741824";
/// The host's search path is the client's. The image keeps its own.
const CLIENT_ONLY: &str = "PATH";
/// Variables the container gets with its own value instead of the host's.
const OWN_VALUE: [&str; 2] = ["TMPDIR", "CLAUDE_CODE_TMPDIR"];
const CLIENT_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";
const LOCK: &str = "host.lock";
const SHORT_WAIT: Duration = Duration::from_secs(20);
const REMOVE_WAIT: Duration = Duration::from_secs(10);
const BUILD_WAIT: Duration = Duration::from_secs(20 * 60);

/// A Docker-compatible runtime and the image a host runs shared Claude in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerIsolation {
    cli: PathBuf,
    endpoint: String,
    image: String,
    client_config: PathBuf,
    host: String,
}
impl ContainerIsolation {
    /// `cli` is the runtime's client and `endpoint` its engine's socket
    /// (`unix://` only). `image` is an image ID, never a tag that a rebuild
    /// could move. `client_config` is an empty private folder for the
    /// client's own settings, outside every run's folders, so nothing a run
    /// writes changes how the next container starts.
    pub fn new(
        cli: PathBuf,
        endpoint: String,
        image: String,
        client_config: PathBuf,
    ) -> Result<Self, StartError> {
        check_client(&cli, &endpoint, &client_config)?;
        if !image_id(&image) {
            return Err(StartError::Settings);
        }
        let host = hex(&Sha256::digest(client_config.as_os_str().as_encoded_bytes()))[..32].to_owned();
        Ok(Self {
            cli,
            endpoint,
            image,
            client_config,
            host,
        })
    }
    pub fn cli(&self) -> &Path {
        &self.cli
    }
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
    pub fn image(&self) -> &str {
        &self.image
    }
    /// Before this host's first container in this process, removes the
    /// containers an earlier host process with the same settings left
    /// behind. A host process that is still running keeps the lock, and
    /// what runs is its own.
    pub(in crate::owned) fn sweep_once(&self) -> Result<(), StartError> {
        static HELD: Mutex<Vec<(String, Option<std::fs::File>)>> = Mutex::new(Vec::new());
        let mut held = HELD.lock().map_err(|_| StartError::Settings)?;
        if held.iter().any(|(host, _)| *host == self.host) {
            return Ok(());
        }
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.client_config.join(LOCK))
            .map_err(|_| StartError::Settings)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                held.push((self.host.clone(), None));
                return Ok(());
            }
            Err(_) => return Err(StartError::Settings),
        }
        let role = format!("label={ROLE}");
        let host = format!("label={HOST}={}", self.host);
        let listed = run(
            self.client()
                .args(["ps", "--all", "--quiet", "--filter", &role, "--filter", &host]),
            None,
            SHORT_WAIT,
        )
        .filter(|output| output.success)
        .ok_or(StartError::Settings)?;
        let left: Vec<&str> = listed
            .stdout
            .split_whitespace()
            .filter(|id| id.len() <= 128 && id.bytes().all(|b| b.is_ascii_hexdigit()))
            .collect();
        if !left.is_empty() {
            run(self.client().args(["rm", "--force"]).args(&left), None, SHORT_WAIT)
                .filter(|output| output.success)
                .ok_or(StartError::Settings)?;
            tracing::warn!(
                code = "shared_claude_containers_removed",
                count = left.len(),
                "Removed containers an earlier host process left behind"
            );
        }
        held.push((self.host.clone(), Some(lock)));
        Ok(())
    }
    fn client(&self) -> Command {
        client(&self.cli, &self.endpoint, &self.client_config)
    }
}

/// Why a Docker-compatible runtime cannot run pool Agents yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProbeError {
    #[error("no Docker-compatible command was found")]
    NoClient,
    #[error("the runtime's current engine is not a local socket")]
    NoEndpoint,
    #[error("the runtime's engine is not running")]
    EngineUnavailable,
    #[error("the image could not be built")]
    BuildFailed,
    #[error("the runtime answered in a way this host does not understand")]
    Unexpected,
}

/// Where a Docker-compatible client usually is: Docker Desktop, OrbStack,
/// Colima or Podman's Docker client, Rancher Desktop, Homebrew.
pub fn find_client(home: &Path) -> Option<PathBuf> {
    [
        PathBuf::from("/usr/local/bin/docker"),
        PathBuf::from("/opt/homebrew/bin/docker"),
        home.join(".docker/bin/docker"),
        home.join(".orbstack/bin/docker"),
        home.join(".rd/bin/docker"),
        PathBuf::from("/Applications/Docker.app/Contents/Resources/bin/docker"),
        PathBuf::from("/usr/bin/docker"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// The engine socket of the client's current context, read with the owner's
/// own client settings in `home`.
pub fn current_endpoint(cli: &Path, home: &Path) -> Result<String, ProbeError> {
    let mut command = owner_client(cli, home);
    command.args(["context", "inspect", "--format", "{{.Endpoints.docker.Host}}"]);
    let output = run(&mut command, None, SHORT_WAIT).ok_or(ProbeError::Unexpected)?;
    let endpoint = output.stdout.trim();
    if !output.success {
        return Err(ProbeError::Unexpected);
    }
    if !endpoint.starts_with("unix:///") || endpoint.len() > 1024 || endpoint.contains(['\n', '\r', '\0']) {
        return Err(ProbeError::NoEndpoint);
    }
    Ok(endpoint.to_owned())
}

/// The ID of this host's image on the engine, building it first when the
/// engine does not have it. A first build downloads the base image and
/// Claude Code and takes minutes.
pub fn prepare_image(
    cli: &Path,
    endpoint: &str,
    client_config: &Path,
    home: &Path,
) -> Result<String, ProbeError> {
    check_client(cli, endpoint, client_config).map_err(|_| ProbeError::Unexpected)?;
    // The client's settings folder is the owner's, as every run's folders are.
    let (uid, gid) = owner(client_config).map_err(|_| ProbeError::Unexpected)?;
    let tag = image_tag(uid, gid);
    let inspect = |tag: &str| {
        run(
            client(cli, endpoint, client_config).args(["image", "inspect", "--format", "{{.Id}}", tag]),
            None,
            SHORT_WAIT,
        )
        .ok_or(ProbeError::EngineUnavailable)
    };
    let found = inspect(&tag)?;
    if found.success {
        let id = found.stdout.trim();
        return image_id(id).then(|| id.to_owned()).ok_or(ProbeError::Unexpected);
    }
    let missing = found.stderr.to_ascii_lowercase();
    if !missing.contains("no such image") && !missing.contains("image not known") {
        return Err(ProbeError::EngineUnavailable);
    }
    // The owner's own client settings find the build plugin and the
    // credential helper for the registry the base image comes from.
    let mut build = owner_client(cli, home);
    build.env("DOCKER_HOST", endpoint).args([
        "build",
        "--quiet",
        "--build-arg",
        &format!("AGENT_UID={uid}"),
        "--build-arg",
        &format!("AGENT_GID={gid}"),
        "--tag",
        &tag,
        "-",
    ]);
    let built = run(&mut build, Some(DOCKERFILE.as_bytes()), BUILD_WAIT).ok_or(ProbeError::BuildFailed)?;
    if !built.success {
        tracing::warn!(code = "shared_claude_image_build_failed", "The pool Agent image did not build");
        return Err(ProbeError::BuildFailed);
    }
    let found = inspect(&tag)?;
    let id = found.stdout.trim();
    (found.success && image_id(id))
        .then(|| id.to_owned())
        .ok_or(ProbeError::Unexpected)
}

/// The tag of the image this host builds from [`DOCKERFILE`] for an owner
/// with these user and group numbers: the image names the user its
/// containers run as.
pub fn image_tag(uid: u32, gid: u32) -> String {
    format!(
        "{IMAGE_TAG_PREFIX}{}-{uid}-{gid}",
        &hex(&Sha256::digest(DOCKERFILE.as_bytes()))[..12]
    )
}

/// Rewrites a shared Claude launch to run in a fresh container. Refuses what
/// the macOS sandbox refuses, a model endpoint on this computer's loopback
/// (inside the container that is the container itself), and client settings
/// inside the run's folders.
pub(in crate::owned) fn confine(
    launch: &mut Launch,
    scope: &Path,
    project: Option<&SharedProject>,
    container: &ContainerIsolation,
) -> Result<ContainerStop, StartError> {
    super::isolation::check_paths(launch, scope, project, &super::isolation::CLAUDE_PATHS)?;
    let config = &container.client_config;
    let folders = [Some(scope), project.map(SharedProject::directory)];
    if folders
        .iter()
        .flatten()
        .any(|folder| config.starts_with(folder) || folder.starts_with(config))
    {
        return Err(StartError::Settings);
    }
    if launch
        .environment
        .get(OsStr::new("ANTHROPIC_BASE_URL"))
        .is_some_and(|url| url.to_str().is_none_or(loopback))
    {
        return Err(StartError::Settings);
    }
    let mut passed = Vec::new();
    for key in launch.environment.keys() {
        let text = key.to_str().ok_or(StartError::Settings)?;
        // The host never sets these for a run; they would steer the client.
        if text.starts_with("DOCKER_") || text.starts_with("BUILDKIT_") {
            return Err(StartError::Settings);
        }
        if text != CLIENT_ONLY && !OWN_VALUE.contains(&text) {
            passed.push(key.clone());
        }
    }
    let (uid, gid) = owner(scope)?;
    let name = format!("hagency-claude-{}", unique(scope));
    let mut arguments: Vec<OsString> = [
        "run",
        "--rm",
        "--interactive",
        "--init",
        "--pull=never",
        "--name",
        &name,
        "--label",
        ROLE,
        "--label",
        &format!("{HOST}={}", container.host),
        "--user",
        &format!("{uid}:{gid}"),
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
        "--security-opt=seccomp=unconfined",
        "--security-opt=systempaths=unconfined",
        "--pids-limit=1024",
        "--read-only",
        "--tmpfs",
        TMP_MOUNT,
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    for folder in folders.into_iter().flatten() {
        arguments.extend(["--mount".into(), bind(folder)?.into()]);
    }
    arguments.extend(["--workdir".into(), launch.directory.clone().into_os_string()]);
    for key in passed {
        // The client copies the value from its own environment.
        arguments.extend(["--env".into(), key]);
    }
    for key in OWN_VALUE {
        arguments.extend(["--env".into(), format!("{key}={TMP}").into()]);
    }
    arguments.extend(["--entrypoint".into(), CLAUDE.into(), container.image.clone().into()]);
    arguments.append(&mut launch.arguments);
    let mut environment = std::mem::take(&mut launch.environment);
    environment.insert(CLIENT_ONLY.into(), CLIENT_PATH.into());
    environment.extend(client_environment(&container.endpoint, &container.client_config));
    launch.executable = container.cli.clone();
    launch.arguments = arguments;
    launch.environment = environment;
    launch.validate().map_err(|_| StartError::Settings)?;
    Ok(ContainerStop {
        isolation: container.clone(),
        name,
    })
}

/// The container of one run, removed whatever state it is in.
pub(in crate::owned) struct ContainerStop {
    isolation: ContainerIsolation,
    name: String,
}
impl ContainerStop {
    /// Ok once the container is gone. The CLI normally ends with its input
    /// and the engine removes the container; a client that was killed can
    /// leave it running, which this ends.
    pub(in crate::owned) fn remove(&self) -> Result<(), io::ErrorKind> {
        let output = run(
            self.isolation.client().args(["rm", "--force", &self.name]),
            None,
            REMOVE_WAIT,
        )
        .ok_or(io::ErrorKind::TimedOut)?;
        let text = output.stderr.to_ascii_lowercase();
        if output.success || text.contains("no such container") || text.contains("already in progress") {
            Ok(())
        } else {
            Err(io::ErrorKind::Other)
        }
    }
    /// A start that failed after the client may have run.
    pub(in crate::owned) fn after_failed_start(&self, error: StartError) -> StartError {
        match error {
            StartError::Uncertain { kind, cleanup } => StartError::Uncertain {
                kind,
                cleanup: match self.remove() {
                    Ok(()) => cleanup,
                    Err(kind) => Cleanup::Unknown { kind },
                },
            },
            nothing_started => nothing_started,
        }
    }
    #[cfg(test)]
    pub(in crate::owned) fn name(&self) -> &str {
        &self.name
    }
}

fn check_client(cli: &Path, endpoint: &str, client_config: &Path) -> Result<(), StartError> {
    let socket = endpoint.strip_prefix("unix://").map(Path::new);
    if !cli.is_absolute()
        || !cli.is_file()
        || !socket.is_some_and(Path::is_absolute)
        || endpoint.len() > 1024
        || endpoint.contains(['\n', '\r', '\0'])
        || !client_config.is_absolute()
        || !client_config.is_dir()
        || client_config.canonicalize().ok().as_deref() != Some(client_config)
    {
        return Err(StartError::Settings);
    }
    Ok(())
}

fn image_id(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn client_environment(endpoint: &str, client_config: &Path) -> [(OsString, OsString); 3] {
    [
        ("DOCKER_HOST".into(), endpoint.into()),
        ("DOCKER_CONFIG".into(), client_config.as_os_str().into()),
        ("DOCKER_CLI_HINTS".into(), "false".into()),
    ]
}

/// A client for this host's own short commands: nothing of the host's
/// environment, the run's settings or the owner's client settings.
fn client(cli: &Path, endpoint: &str, client_config: &Path) -> Command {
    let mut command = Command::new(cli);
    command
        .env_clear()
        .env("PATH", CLIENT_PATH)
        .envs(client_environment(endpoint, client_config))
        .current_dir("/");
    command
}

/// A client with the owner's own settings, for setting up only. Its helpers
/// (build plugin, credential helper) sit next to the client.
fn owner_client(cli: &Path, home: &Path) -> Command {
    let mut path = OsString::new();
    for folder in [cli.parent(), cli.canonicalize().ok().as_deref().and_then(Path::parent)]
        .into_iter()
        .flatten()
    {
        path.push(folder);
        path.push(":");
    }
    path.push("/usr/local/bin:/opt/homebrew/bin:");
    path.push(CLIENT_PATH);
    let mut command = Command::new(cli);
    command
        .env_clear()
        .env("PATH", path)
        .env("HOME", home)
        .env("DOCKER_CLI_HINTS", "false")
        .current_dir("/");
    command
}

/// `--mount` parses its value as one CSV record.
fn bind(path: &Path) -> Result<String, StartError> {
    let text = path.to_str().ok_or(StartError::Settings)?;
    if text.contains(['\n', '\r', '\0']) {
        return Err(StartError::Settings);
    }
    let quoted = |field: String| format!("\"{}\"", field.replace('"', "\"\""));
    Ok(format!(
        "type=bind,{},{}",
        quoted(format!("source={text}")),
        quoted(format!("target={text}"))
    ))
}

/// The run's folders belong to the owner; the container runs as them, and
/// never as root.
fn owner(folder: &Path) -> Result<(u32, u32), StartError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(folder).map_err(|_| StartError::Settings)?;
        if metadata.uid() == 0 {
            return Err(StartError::Settings);
        }
        Ok((metadata.uid(), metadata.gid()))
    }
    #[cfg(not(unix))]
    {
        let _ = folder;
        Err(StartError::Unsupported)
    }
}

fn loopback(url: &str) -> bool {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let authority = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => authority.rsplit_once(':').map_or(authority, |(host, _)| host),
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host.is_empty()
        || host == "localhost"
        || host.ends_with(".localhost")
        || host == "::1"
        || host == "::"
        || host == "0.0.0.0"
        || host.starts_with("127.")
        || host.starts_with("::ffff:127.")
}

fn unique(scope: &Path) -> String {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    let mut digest = Sha256::new();
    digest.update(scope.as_os_str().as_encoded_bytes());
    digest.update(std::process::id().to_le_bytes());
    digest.update(nanos.to_le_bytes());
    digest.update(COUNT.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    hex(&digest.finalize())[..16].to_owned()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct Output {
    success: bool,
    stdout: String,
    stderr: String,
}
/// Runs a client command, stopped once `wait` has passed. Output is read as
/// it comes, so a full pipe cannot stall the command.
fn run(command: &mut Command, input: Option<&'static [u8]>, wait: Duration) -> Option<Output> {
    command
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().ok()?;
    if let (Some(bytes), Some(mut stdin)) = (input, child.stdin.take()) {
        std::thread::spawn(move || {
            let _ = stdin.write_all(bytes);
        });
    }
    let reader = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(pipe) = pipe {
                let _ = pipe.take(64 * 1024).read_to_end(&mut bytes);
            }
            String::from_utf8_lossy(&bytes).into_owned()
        })
    };
    let stdout = reader(child.stdout.take().map(|pipe| Box::new(pipe) as Box<dyn Read + Send>));
    let stderr = reader(child.stderr.take().map(|pipe| Box::new(pipe) as Box<dyn Read + Send>));
    let deadline = Instant::now() + wait;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    // A client plugin that outlives the client can hold a pipe open: take
    // what arrived by then.
    let collect = |handle: std::thread::JoinHandle<String>| {
        let until = Instant::now() + Duration::from_secs(2);
        while !handle.is_finished() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        if handle.is_finished() {
            handle.join().unwrap_or_default()
        } else {
            String::new()
        }
    };
    Some(Output {
        success: status.success(),
        stdout: collect(stdout),
        stderr: collect(stderr),
    })
}

#[cfg(all(test, unix))]
mod tests;
