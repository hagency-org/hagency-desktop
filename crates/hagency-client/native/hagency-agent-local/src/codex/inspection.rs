//! Keep the inspection cwd owned until its child is observed exiting.
use super::{Error, Process, Result, inspection_step};
use hagency_platform::CodexInspectionDirectory;
use std::ops::{Deref, DerefMut};

pub(super) struct Probe {
    state: Option<(Process, CodexInspectionDirectory)>,
    runtime: tokio::runtime::Handle,
}

impl Probe {
    pub fn new(process: Process, directory: CodexInspectionDirectory) -> Self {
        Self {
            state: Some((process, directory)),
            runtime: tokio::runtime::Handle::current(),
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some((process, _)) = self.state.as_mut() {
            process.stop().await?;
        }
        // Successful stop observes exit before removing the cwd.
        if let Some((process, directory)) = self.state.take() {
            drop(process);
            cleanup_directory(directory);
        }
        Ok(())
    }
}

impl Deref for Probe {
    type Target = Process;
    fn deref(&self) -> &Process {
        &self.state.as_ref().expect("live inspection probe").0
    }
}

impl DerefMut for Probe {
    fn deref_mut(&mut self) -> &mut Process {
        &mut self.state.as_mut().expect("live inspection probe").0
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        let Some((mut process, directory)) = self.state.take() else {
            return;
        };
        // Cancelling the caller must not drop the cwd while kill_on_drop is
        // still asynchronous. This detached reaper retains both resources.
        self.runtime.spawn(async move {
            if inspection_step("cleanup_stop", process.stop().await).is_err() {
                // If termination failed, retain the cwd until natural exit.
                let waited = process.child.wait().await.map(|_| ()).map_err(Error::from);
                let _ = inspection_step("cleanup_wait", waited);
            }
            drop(process);
            cleanup_directory(directory);
        });
    }
}

fn cleanup_directory(mut directory: CodexInspectionDirectory) {
    // Cleanup remains best-effort for metadata callers, but is observable if
    // a scanner, permissions change or unexpected file prevents removal.
    let result = directory.cleanup().map_err(Error::from);
    let _ = inspection_step("cleanup_directory", result);
}

#[cfg(all(test, windows))]
mod tests {
    use super::super::{Process, Profile, inspect_shared_configuration};
    use std::{
        io::{BufRead, Write},
        path::{Path, PathBuf},
        time::Duration,
    };
    use tokio::io::AsyncBufReadExt;

    const CHILD: &str = "codex::inspection::tests::fixture_child";
    const ROOT: &str = "HAGENCY_INSPECTION_TEST_ROOT";
    const MODE: &str = "HAGENCY_INSPECTION_TEST_MODE";
    const READY: &str = "HAGENCY_INSPECTION_FIXTURE_READY";

    #[test]
    #[ignore = "subprocess fixture for inspection lifetime tests"]
    fn fixture_child() {
        let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
        let mode = std::env::var(MODE).unwrap();
        println!("{READY}");
        std::io::stdout().flush().unwrap();
        for line in std::io::stdin().lock().lines() {
            let frame: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            let method = frame["method"].as_str().unwrap();
            if method == mode {
                std::fs::write(root.join("pending"), method).unwrap();
                // Keep the cwd open even when stdin closes: require child reaping.
                std::thread::sleep(Duration::from_secs(30));
                return;
            }
            let result = match method {
                "initialize" => serde_json::json!({"userAgent":"hagency-client/0.169.0"}),
                "config/read" => serde_json::json!({"config": {
                    "model_provider":if mode == "reject" { "unsupported" } else { "openai" },
                    "chatgpt_base_url":"https://chatgpt.com/backend-api",
                    "model_providers":{}, "mcp_servers":{}
                }}),
                "initialized" => continue,
                _ => panic!("unexpected inspection request"),
            };
            println!("{}", serde_json::json!({"id":frame["id"],"result":result}));
            std::io::stdout().flush().unwrap();
        }
    }

    async fn fixture(
        root: &Path,
        mode: &str,
    ) -> (hagency_platform::CodexInspectionDirectory, Process) {
        let home = root.join("home");
        let codex_home = root.join("codex");
        std::fs::create_dir(&home).unwrap();
        std::fs::create_dir(&codex_home).unwrap();
        std::fs::write(codex_home.join("preserved"), b"unchanged").unwrap();
        let directory = hagency_platform::codex_inspection_directory(&home).unwrap();
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                CHILD,
                "--nocapture",
                "--test-threads=1",
            ])
            .env(ROOT, root)
            .env(MODE, mode)
            .current_dir(directory.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let mut output = tokio::io::BufReader::new(child.stdout.take().unwrap());
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let mut line = String::new();
                assert_ne!(output.read_line(&mut line).await.unwrap(), 0);
                if line.contains(READY) {
                    break;
                }
            }
        })
        .await
        .unwrap();
        let profile = Profile {
            executable: std::env::current_exe().unwrap().canonicalize().unwrap(),
            home: home.canonicalize().unwrap(),
            codex_home: codex_home.canonicalize().unwrap(),
            cwd: directory.path().to_owned(),
            model: "probe".into(),
            effort: "medium".into(),
            shared_auth: true,
            web_search: false,
        };
        (
            directory,
            Process {
                session: profile.session(output.into_inner(), input),
                child,
            },
        )
    }

    async fn cancel_pending(mode: &str) {
        for _ in 0..10 {
            let root = tempfile::tempdir().unwrap();
            let (directory, process) = fixture(root.path(), mode).await;
            let path = directory.path().to_owned();
            let task = tokio::spawn(inspect_shared_configuration(directory, process));
            let pending = root.path().join("pending");
            let ready = tokio::time::timeout(Duration::from_secs(10), async {
                while !pending.exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await;
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
            ready.expect("inspection must reach the requested RPC before cancellation");
            let removed = tokio::time::timeout(Duration::from_secs(3), async {
                while path.exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await;
            if removed.is_err() {
                // Test-owned empty directory only; leave persistent account files intact.
                let _ = std::fs::remove_dir(&path);
                panic!("cancelled inspection left its cwd after child shutdown");
            }
            assert_eq!(
                std::fs::read(root.path().join("codex/preserved")).unwrap(),
                b"unchanged"
            );
        }
    }

    #[tokio::test]
    async fn cancelled_initialize_reaps_child_then_removes_cwd() {
        cancel_pending("initialize").await;
    }

    #[tokio::test]
    async fn cancelled_config_read_reaps_child_then_removes_cwd() {
        cancel_pending("config/read").await;
    }

    #[tokio::test]
    async fn completed_inspection_removes_cwd_before_returning() {
        let root = tempfile::tempdir().unwrap();
        let (directory, process) = fixture(root.path(), "complete").await;
        let path = directory.path().to_owned();
        assert!(
            inspect_shared_configuration(directory, process)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(!path.exists());
        assert_eq!(
            std::fs::read(root.path().join("codex/preserved")).unwrap(),
            b"unchanged"
        );
    }

    #[tokio::test]
    async fn rejected_configuration_removes_cwd_before_returning_error() {
        let root = tempfile::tempdir().unwrap();
        let (directory, process) = fixture(root.path(), "reject").await;
        let path = directory.path().to_owned();
        assert!(
            inspect_shared_configuration(directory, process)
                .await
                .is_err()
        );
        assert!(!path.exists());
        assert_eq!(
            std::fs::read(root.path().join("codex/preserved")).unwrap(),
            b"unchanged"
        );
    }
}
