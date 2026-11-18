//! The owner's choice, on this computer, to run pool Agents' Claude in a
//! Docker-compatible container instead of the macOS sandbox (see
//! `hagency_runtime::owned::container`). Desktop writes it into the owner
//! daemon's home; the daemon reads it at each pool Claude launch, so a change
//! applies from the next run. No file means the sandbox.
use hagency_runtime::owned::container::{self, ContainerIsolation};
pub use hagency_runtime::owned::container::ProbeError;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const FILE: &str = "pool-containers.json";
/// The client's own settings while it runs pool Agents: empty, private, and
/// outside every run's folders.
const CLIENT_CONFIG: &str = "container-client";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Setting {
    /// The Docker-compatible client.
    pub cli: PathBuf,
    /// Its engine's socket, `unix://…`.
    pub endpoint: String,
    /// The ID of the image pool Agents run.
    pub image: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The file could not be read or written.
    State,
    /// What the file says cannot run.
    Invalid,
    /// Turning on stopped at this step.
    Probe(ProbeError),
}
impl From<ProbeError> for Error {
    fn from(error: ProbeError) -> Self {
        Self::Probe(error)
    }
}

/// The saved choice, `None` when containers are off.
pub fn load(daemon_home: &Path) -> Result<Option<Setting>, Error> {
    let file = match hagency_store::private::open(&daemon_home.join(FILE), false) {
        Ok(file) => file,
        Err(hagency_store::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(_) => return Err(Error::State),
    };
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes).map_err(|_| Error::State)?;
    if bytes.len() > 4096 {
        return Err(Error::Invalid);
    }
    serde_json::from_slice(&bytes).map(Some).map_err(|_| Error::Invalid)
}

/// What a pool Claude launch runs in: `None` for the sandbox.
pub fn isolation(daemon_home: &Path) -> Result<Option<ContainerIsolation>, Error> {
    let Some(setting) = load(daemon_home)? else {
        return Ok(None);
    };
    let config = client_config(daemon_home)?;
    ContainerIsolation::new(setting.cli, setting.endpoint, setting.image, config)
        .map(Some)
        .map_err(|_| Error::Invalid)
}

/// Finds the client and its engine, builds the image when the engine does
/// not have it (minutes the first time), and saves the choice. Blocking:
/// never on the UI thread.
pub fn turn_on(daemon_home: &Path, home: &Path) -> Result<Setting, Error> {
    let cli = container::find_client(home).ok_or(ProbeError::NoClient)?;
    let endpoint = container::current_endpoint(&cli, home)?;
    let config = client_config(daemon_home)?;
    let image = container::prepare_image(&cli, &endpoint, &config, home)?;
    let setting = Setting { cli, endpoint, image };
    // What a launch will accept.
    ContainerIsolation::new(
        setting.cli.clone(),
        setting.endpoint.clone(),
        setting.image.clone(),
        config,
    )
    .map_err(|_| Error::Invalid)?;
    let bytes = serde_json::to_vec(&setting).map_err(|_| Error::State)?;
    hagency_store::private::replace(&daemon_home.join(FILE), &bytes).map_err(|_| Error::State)?;
    Ok(setting)
}

/// Back to the sandbox from the next run.
pub fn turn_off(daemon_home: &Path) -> Result<(), Error> {
    match std::fs::remove_file(daemon_home.join(FILE)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(Error::State),
    }
}

fn client_config(daemon_home: &Path) -> Result<PathBuf, Error> {
    let path = daemon_home.join(CLIENT_CONFIG);
    hagency_store::private::directory(&path).map_err(|_| Error::State)?;
    path.canonicalize().map_err(|_| Error::State)
}

static SERVED: OnceLock<PathBuf> = OnceLock::new();
/// The owner daemon names its home before hosting starts. Elsewhere pool
/// Agents keep the sandbox.
pub(crate) fn serve_from(daemon_home: &Path) {
    let _ = SERVED.set(daemon_home.to_path_buf());
}
/// The choice of the daemon this process is.
pub(crate) fn hosted_isolation() -> Result<Option<ContainerIsolation>, Error> {
    match SERVED.get() {
        Some(home) => isolation(home),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_file_is_the_sandbox_and_a_saved_choice_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().canonicalize().unwrap();
        assert_eq!(load(&home), Ok(None));
        assert!(isolation(&home).unwrap().is_none());
        let cli = home.join("docker");
        std::fs::write(&cli, "").unwrap();
        let setting = Setting {
            cli: cli.clone(),
            endpoint: "unix:///var/run/docker.sock".into(),
            image: format!("sha256:{}", "a".repeat(64)),
        };
        hagency_store::private::replace(&home.join(FILE), &serde_json::to_vec(&setting).unwrap()).unwrap();
        assert_eq!(load(&home), Ok(Some(setting.clone())));
        let isolation = isolation(&home).unwrap().unwrap();
        assert_eq!((isolation.cli(), isolation.image()), (cli.as_path(), setting.image.as_str()));
        assert!(home.join(CLIENT_CONFIG).is_dir());
        turn_off(&home).unwrap();
        turn_off(&home).unwrap();
        assert_eq!(load(&home), Ok(None));
    }

    #[test]
    fn a_file_that_cannot_run_is_refused_not_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().canonicalize().unwrap();
        for text in [
            r#"{"cli":"/usr/local/bin/docker"}"#,
            r#"{"cli":"/nowhere/docker","endpoint":"unix:///s","image":"sha256:00","extra":1}"#,
            "not json",
        ] {
            hagency_store::private::replace(&home.join(FILE), text.as_bytes()).unwrap();
            assert_eq!(load(&home).err(), Some(Error::Invalid), "{text}");
        }
        let valid_shape = format!(r#"{{"cli":"/nowhere/docker","endpoint":"unix:///s","image":"sha256:{}"}}"#, "a".repeat(64));
        hagency_store::private::replace(&home.join(FILE), valid_shape.as_bytes()).unwrap();
        assert_eq!(isolation(&home).err(), Some(Error::Invalid));
    }
}
