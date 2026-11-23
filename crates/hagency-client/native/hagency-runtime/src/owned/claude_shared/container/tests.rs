use super::*;
use crate::owned::SharedClaudePolicy;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
    os::unix::fs::PermissionsExt,
    sync::mpsc,
};

const IMAGE: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const ENDPOINT: &str = "unix:///var/run/hagency-test.sock";
/// A client that records each call, answers `ps` and `rm` as told, and has
/// the image once it was built.
const FAKE_CLIENT: &str = r#"#!/bin/sh
here="$(dirname "$0")"
{ printf 'call'; for a in "$@"; do printf ' [%s]' "$a"; done; printf '\n'; env | sed 's/^/env /'; } >> "$here/calls"
case "$1" in
  ps) cat "$here/ps-output" 2>/dev/null ;;
  rm) case "$(cat "$here/rm-mode" 2>/dev/null)" in
        gone) echo "Error response from daemon: No such container: $3" >&2; exit 1 ;;
        fail) echo "Error response from daemon: engine refused" >&2; exit 1 ;;
      esac ;;
  image) if [ -f "$here/built" ]; then cat "$here/built"; else echo "Error response from daemon: No such image: $5" >&2; exit 1; fi ;;
  build) cat > "$here/recipe"; cp "$here/image-id" "$here/built" ;;
esac
exit 0
"#;

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    scope: PathBuf,
    cli: PathBuf,
    config: PathBuf,
}
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let scope = root.join("state/room");
    for name in ["home", "config", "tmp", "work"] {
        std::fs::create_dir_all(scope.join(name)).unwrap();
    }
    let bin = root.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let cli = bin.join("docker");
    std::fs::write(&cli, FAKE_CLIENT).unwrap();
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).unwrap();
    let config = root.join("client");
    std::fs::create_dir(&config).unwrap();
    Fixture {
        _dir: dir,
        root,
        scope,
        cli,
        config,
    }
}
impl Fixture {
    fn isolation(&self) -> ContainerIsolation {
        ContainerIsolation::new(self.cli.clone(), ENDPOINT.into(), IMAGE.into(), self.config.clone()).unwrap()
    }
    fn launch(&self, directory: &Path) -> Launch {
        let mut environment = BTreeMap::new();
        for (name, sub) in [
            ("HOME", "home"),
            ("CLAUDE_CONFIG_DIR", "config"),
            ("TMPDIR", "tmp"),
            ("CLAUDE_CODE_TMPDIR", "tmp"),
        ] {
            environment.insert(name.into(), self.scope.join(sub).into_os_string());
        }
        environment.insert("PATH".into(), CLIENT_PATH.into());
        environment.insert("ANTHROPIC_API_KEY".into(), "secret-key-value".into());
        environment.insert("ANTHROPIC_BASE_URL".into(), "https://models.example".into());
        Launch {
            executable: "/usr/local/bin/claude".into(),
            arguments: vec!["--print".into(), "--model=claude-opus-5".into()],
            directory: directory.to_path_buf(),
            environment,
            require_crash_containment: false,
        }
    }
    fn calls(&self) -> String {
        std::fs::read_to_string(self.cli.with_file_name("calls")).unwrap_or_default()
    }
    fn answer(&self, file: &str, text: &str) {
        std::fs::write(self.cli.with_file_name(file), text).unwrap();
    }
}
fn text(arguments: &[OsString]) -> Vec<String> {
    arguments.iter().map(|value| value.to_str().unwrap().to_owned()).collect()
}

#[test]
fn a_run_becomes_a_container_of_its_own_folders_without_the_credential_on_the_command_line() {
    let f = fixture();
    let isolation = f.isolation();
    let mut launch = f.launch(&f.scope.join("work"));
    let stop = confine(&mut launch, &f.scope, None, &isolation).unwrap();
    assert_eq!(launch.executable, f.cli);
    assert!(stop.name().starts_with("hagency-claude-") && stop.name().len() == 31);
    let (uid, gid) = owner(&f.scope).unwrap();
    let scope = f.scope.to_str().unwrap();
    let work = f.scope.join("work");
    let host = format!("io.hagency.host={}", isolation.host);
    let user = format!("{uid}:{gid}");
    let mount = format!("type=bind,\"source={scope}\",\"target={scope}\"");
    let expected = [
        "run", "--rm", "--interactive", "--init", "--pull=never", "--name", stop.name(),
        "--label", "io.hagency.role=shared-claude", "--label", host.as_str(), "--user", user.as_str(),
        "--cap-drop=ALL", "--security-opt=no-new-privileges", "--security-opt=seccomp=unconfined",
        "--security-opt=systempaths=unconfined", "--pids-limit=1024", "--read-only",
        "--tmpfs", "/tmp:rw,exec,nosuid,nodev,size=1073741824",
        "--mount", mount.as_str(), "--workdir", work.to_str().unwrap(),
        "--env", "ANTHROPIC_API_KEY", "--env", "ANTHROPIC_BASE_URL", "--env", "CLAUDE_CONFIG_DIR",
        "--env", "HOME", "--env", "TMPDIR=/tmp", "--env", "CLAUDE_CODE_TMPDIR=/tmp",
        "--entrypoint", "/usr/local/bin/claude", IMAGE, "--print", "--model=claude-opus-5",
    ];
    assert_eq!(text(&launch.arguments), expected);
    assert!(!text(&launch.arguments).iter().any(|value| value.contains("secret-key-value")));
    let env = |key: &str| launch.environment.get(OsStr::new(key)).map(|v| v.to_str().unwrap().to_owned());
    assert_eq!(env("ANTHROPIC_API_KEY").as_deref(), Some("secret-key-value"));
    assert_eq!(env("PATH").as_deref(), Some(CLIENT_PATH));
    assert_eq!(env("DOCKER_HOST").as_deref(), Some(ENDPOINT));
    assert_eq!(env("DOCKER_CONFIG").as_deref(), f.config.to_str());
    assert_eq!(env("DOCKER_CLI_HINTS").as_deref(), Some("false"));
    assert_eq!(env("HOME").as_deref(), f.scope.join("home").to_str());
}

#[test]
fn a_space_project_folder_is_the_second_mount_and_the_working_folder() {
    let f = fixture();
    let projects = f.root.join("Projects");
    let folder = projects.join("app");
    std::fs::create_dir_all(&folder).unwrap();
    let project = SharedProject::new(projects, folder.clone()).unwrap();
    let mut launch = f.launch(&folder);
    confine(&mut launch, &f.scope, Some(&project), &f.isolation()).unwrap();
    let arguments = text(&launch.arguments);
    let after = |flag: &str| -> Vec<String> {
        arguments.windows(2).filter(|pair| pair[0] == flag).map(|pair| pair[1].clone()).collect()
    };
    assert_eq!(after("--mount"), [bind(&f.scope).unwrap(), bind(&folder).unwrap()]);
    assert_eq!(after("--workdir"), [folder.to_str().unwrap()]);
}

#[test]
fn what_the_sandbox_refuses_and_what_would_reach_the_wrong_place_are_refused() {
    let f = fixture();
    let isolation = f.isolation();
    let refused = |mut launch: Launch, isolation: &ContainerIsolation| {
        matches!(confine(&mut launch, &f.scope, None, isolation), Err(StartError::Settings))
    };
    // The same folder rules as the macOS sandbox.
    assert!(refused(f.launch(&f.root), &isolation));
    let mut outside = f.launch(&f.scope.join("work"));
    outside.environment.insert("HOME".into(), f.root.clone().into_os_string());
    assert!(refused(outside, &isolation));
    // Inside the container, loopback is the container itself.
    for url in [
        "http://127.0.0.1:8080",
        "http://localhost:3000/v1",
        "http://[::1]:9",
        "https://user@127.0.0.2/",
        "http://0.0.0.0",
        "http://models.localhost.",
    ] {
        let mut launch = f.launch(&f.scope.join("work"));
        launch.environment.insert("ANTHROPIC_BASE_URL".into(), url.into());
        assert!(refused(launch, &isolation), "{url}");
    }
    let mut host = f.launch(&f.scope.join("work"));
    host.environment.insert("ANTHROPIC_BASE_URL".into(), "http://host.docker.internal:8080".into());
    assert!(confine(&mut host, &f.scope, None, &isolation).is_ok());
    // Nothing the host does not set may steer the client.
    let mut steered = f.launch(&f.scope.join("work"));
    steered.environment.insert("DOCKER_CONTEXT".into(), "other".into());
    assert!(refused(steered, &isolation));
    // The client's own settings must not be where a run writes.
    let inside = ContainerIsolation::new(f.cli.clone(), ENDPOINT.into(), IMAGE.into(), f.scope.join("home")).unwrap();
    assert!(refused(f.launch(&f.scope.join("work")), &inside));
    let around = ContainerIsolation::new(f.cli.clone(), ENDPOINT.into(), IMAGE.into(), f.root.join("state")).unwrap();
    assert!(refused(f.launch(&f.scope.join("work")), &around));
}

#[test]
fn a_runtime_is_a_local_client_a_local_socket_and_an_image_id() {
    let f = fixture();
    let new = |cli: &Path, endpoint: &str, image: &str, config: &Path| {
        ContainerIsolation::new(cli.into(), endpoint.into(), image.into(), config.into()).is_ok()
    };
    assert!(new(&f.cli, ENDPOINT, IMAGE, &f.config));
    assert!(!new(Path::new("docker"), ENDPOINT, IMAGE, &f.config));
    assert!(!new(&f.root.join("bin/podman"), ENDPOINT, IMAGE, &f.config));
    for endpoint in ["tcp://127.0.0.1:2375", "unix://relative.sock", "ssh://host", "unix:///a\nb"] {
        assert!(!new(&f.cli, endpoint, IMAGE, &f.config), "{endpoint}");
    }
    let (upper, long) = (IMAGE.to_uppercase(), format!("{IMAGE}0"));
    for image in ["hagency/claude-agent:abc", upper.as_str(), "sha256:0123", long.as_str()] {
        assert!(!new(&f.cli, ENDPOINT, image, &f.config), "{image}");
    }
    assert!(!new(&f.cli, ENDPOINT, IMAGE, &f.root.join("client/../client")));
    assert!(!new(&f.cli, ENDPOINT, IMAGE, &f.root.join("missing")));
}

#[test]
fn a_mount_value_is_one_csv_record() {
    assert_eq!(
        bind(Path::new("/a,b/\"c\"")).unwrap(),
        r#"type=bind,"source=/a,b/""c""","target=/a,b/""c""""#
    );
    assert!(bind(Path::new("/a\nb")).is_err());
}

#[test]
fn removal_ends_the_container_and_a_gone_container_counts_as_removed() {
    let f = fixture();
    let mut launch = f.launch(&f.scope.join("work"));
    let stop = confine(&mut launch, &f.scope, None, &f.isolation()).unwrap();
    assert_eq!(stop.remove(), Ok(()));
    f.answer("rm-mode", "gone");
    assert_eq!(stop.remove(), Ok(()));
    f.answer("rm-mode", "fail");
    assert_eq!(stop.remove(), Err(io::ErrorKind::Other));
    let calls = f.calls();
    assert_eq!(calls.matches(&format!("call [rm] [--force] [{}]", stop.name())).count(), 3);
    // The client gets the engine and its own settings, and nothing of the run.
    assert!(calls.contains(&format!("env DOCKER_HOST={ENDPOINT}")));
    assert!(calls.contains(&format!("env DOCKER_CONFIG={}", f.config.display())));
    assert!(!calls.contains("ANTHROPIC") && !calls.contains("env HOME="));
}

#[test]
fn a_failed_start_removes_what_may_have_started_and_says_when_it_could_not() {
    let f = fixture();
    let mut launch = f.launch(&f.scope.join("work"));
    let stop = confine(&mut launch, &f.scope, None, &f.isolation()).unwrap();
    assert_eq!(stop.after_failed_start(StartError::Settings), StartError::Settings);
    assert!(f.calls().is_empty(), "nothing started, nothing to remove");
    let uncertain = StartError::Uncertain {
        kind: io::ErrorKind::BrokenPipe,
        cleanup: Cleanup::Pending,
    };
    assert_eq!(stop.after_failed_start(uncertain), uncertain);
    f.answer("rm-mode", "fail");
    assert_eq!(
        stop.after_failed_start(uncertain),
        StartError::Uncertain {
            kind: io::ErrorKind::BrokenPipe,
            cleanup: Cleanup::Unknown { kind: io::ErrorKind::Other },
        }
    );
}

#[test]
fn the_first_launch_removes_what_an_earlier_host_left_and_only_once() {
    let f = fixture();
    let isolation = f.isolation();
    f.answer("ps-output", "abc123\ndef456\n");
    isolation.sweep_once().unwrap();
    let calls = f.calls();
    let host = format!("[label=io.hagency.host={}]", isolation.host);
    assert!(calls.contains(&format!(
        "call [ps] [--all] [--quiet] [--filter] [label=io.hagency.role=shared-claude] [--filter] {host}"
    )));
    assert!(calls.contains("call [rm] [--force] [abc123] [def456]"));
    f.isolation().sweep_once().unwrap();
    assert_eq!(f.calls(), calls, "the same host settings sweep once per process");
}

#[test]
fn loopback_names_and_addresses() {
    for url in ["http://127.0.0.1", "http://LOCALHOST:1", "http://[::1]/v1", "http://[::ffff:127.0.0.1]", "http://", "localhost:80"] {
        assert!(loopback(url), "{url}");
    }
    for url in ["https://api.anthropic.com", "http://host.docker.internal:9", "http://10.0.0.1", "http://localhost.example.com"] {
        assert!(!loopback(url), "{url}");
    }
}

#[test]
fn the_image_tag_follows_the_recipe_and_the_owner() {
    let tag = image_tag(505, 20);
    let (digest, owner) = tag.strip_prefix(IMAGE_TAG_PREFIX).unwrap().split_once('-').unwrap();
    assert!(digest.len() == 12 && digest.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(owner, "505-20");
    assert_ne!(image_tag(506, 20), tag, "another owner's image names another user");
    // Claude Code's command sandbox needs both inside the container.
    assert!(DOCKERFILE.contains("bubblewrap") && DOCKERFILE.contains("socat"));
    // git works in a folder the engine reports as another user's, a commit
    // has an author, and the user a container runs as has a name.
    assert!(DOCKERFILE.contains("--system --add safe.directory '*'"));
    assert!(DOCKERFILE.contains("--system user.name") && DOCKERFILE.contains("--system user.email"));
    assert!(DOCKERFILE.contains("ARG AGENT_UID") && DOCKERFILE.contains("ARG AGENT_GID"));
}

#[test]
fn the_image_is_built_once_for_the_owner_its_containers_run_as() {
    use std::os::unix::fs::MetadataExt;
    let f = fixture();
    f.answer("image-id", IMAGE);
    let folder = std::fs::metadata(&f.config).unwrap();
    let (uid, gid) = (folder.uid(), folder.gid());
    assert_eq!(prepare_image(&f.cli, ENDPOINT, &f.config, &f.root).unwrap(), IMAGE);
    let tag = image_tag(uid, gid);
    let calls = f.calls();
    assert!(
        calls.contains(&format!(
            "call [build] [--quiet] [--build-arg] [AGENT_UID={uid}] [--build-arg] [AGENT_GID={gid}] [--tag] [{tag}] [-]"
        )),
        "{calls}"
    );
    // The recipe arrives on the client's input; nothing of it is a file.
    assert_eq!(std::fs::read_to_string(f.cli.with_file_name("recipe")).unwrap(), DOCKERFILE);
    // The engine has the image now: it is looked up, not built again.
    assert_eq!(prepare_image(&f.cli, ENDPOINT, &f.config, &f.root).unwrap(), IMAGE);
    assert_eq!(f.calls().matches("call [build]").count(), 1);
    assert_eq!(f.calls().matches(&format!("[{tag}]")).count(), 4, "looked up, built, looked up, and looked up once more");
}

#[test]
fn a_client_command_is_bounded_and_gets_its_input() {
    let started = Instant::now();
    assert!(run(Command::new("/bin/sleep").arg("5"), None, Duration::from_millis(200)).is_none());
    assert!(started.elapsed() < Duration::from_secs(3));
    let echoed = run(&mut Command::new("/bin/cat"), Some(b"recipe".as_slice()), Duration::from_secs(5)).unwrap();
    assert!(echoed.success);
    assert_eq!(echoed.stdout, "recipe");
}

/// The Hosting switch's setup step on a real engine: the client and its
/// engine are found, the image is built when the engine lacks it, and the
/// next time it is only looked up. The build uses the client settings in
/// `HAGENCY_TEST_DOCKER_HOME`, as the switch uses the owner's.
#[test]
#[ignore = "explicit Docker-compatible engine; the first build takes minutes and the network"]
fn a_real_engine_prepares_the_image_once() {
    let home = PathBuf::from(std::env::var("HAGENCY_TEST_DOCKER_HOME").expect("explicit HAGENCY_TEST_DOCKER_HOME"));
    let cli = find_client(&home).expect("a Docker-compatible client");
    let endpoint = current_endpoint(&cli, &home).unwrap();
    let f = fixture();
    let started = Instant::now();
    let id = prepare_image(&cli, &endpoint, &f.config, &home).unwrap();
    eprintln!("{id} in {:?} via {} at {endpoint}", started.elapsed(), cli.display());
    let again = Instant::now();
    assert_eq!(prepare_image(&cli, &endpoint, &f.config, &home).unwrap(), id);
    assert!(again.elapsed() < SHORT_WAIT);
    ContainerIsolation::new(cli, endpoint, id, f.config.clone()).unwrap();
}

/// A real engine and the pool Agent image: one turn with a tool command
/// through `run --interactive`, then the container is gone. The command
/// sees neither the credential nor the CLI's process, and cannot write the
/// image. A killed client's container is removed, and so is one an earlier
/// host process left.
///
///   HAGENCY_TEST_DOCKER=/usr/local/bin/docker \
///   HAGENCY_TEST_DOCKER_HOST=unix:///Users/me/.docker/run/docker.sock \
///   HAGENCY_TEST_CLAUDE_IMAGE=sha256:… (what `a_real_engine_prepares_the_image_once` prints)
#[test]
#[ignore = "explicit Docker-compatible engine and the pool Agent image"]
fn a_real_engine_runs_a_turn_with_a_tool_and_removes_its_containers() {
    let env = |key: &str| std::env::var(key).unwrap_or_else(|_| panic!("explicit {key}"));
    let cli = PathBuf::from(env("HAGENCY_TEST_DOCKER"));
    let endpoint = env("HAGENCY_TEST_DOCKER_HOST");
    let image = env("HAGENCY_TEST_CLAUDE_IMAGE");
    let model_host = std::env::var("HAGENCY_TEST_DOCKER_HOST_NAME").unwrap_or_else(|_| "host.docker.internal".into());
    let f = fixture();
    let isolation = ContainerIsolation::new(cli.clone(), endpoint.clone(), image, f.config.clone()).unwrap();
    isolation.sweep_once().unwrap();
    let secret = "container-test-credential";
    // `[t]` keeps the probe's own command line from matching the CLI's.
    let probe = format!(
        "echo env-has-key=$(env | grep -c {secret}); \
         echo proc-with-key=$(grep -l {secret} /proc/[0-9]*/environ 2>/dev/null | wc -l); \
         echo cli-visible=$(grep -l 'input-forma[t]' /proc/[0-9]*/cmdline 2>/dev/null | wc -l); \
         echo image-write=$(touch /usr/local/probe 2>/dev/null && echo yes || echo no); \
         echo here=$(pwd)"
    );
    let model = f.root.join("model.py");
    std::fs::write(&model, include_str!("model_fixture.py")).unwrap();
    let mut server = std::process::Command::new("/usr/bin/python3")
        .arg("-I")
        .arg(&model)
        .arg(&probe)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut port = String::new();
    BufReader::new(server.stdout.take().unwrap()).read_line(&mut port).unwrap();
    let port: u16 = port.trim().parse().unwrap();

    let start = |f: &Fixture, isolation: &ContainerIsolation| {
        let mut launch = f.launch(&f.scope.join("work"));
        launch.arguments = SharedClaudePolicy::Standard { web: false }
            .arguments_resuming("claude-opus-5", "medium", None)
            .unwrap()
            .into_iter()
            .map(Into::into)
            .collect();
        for (key, value) in [
            ("ANTHROPIC_API_KEY", secret.to_owned()),
            ("ANTHROPIC_BASE_URL", format!("http://{model_host}:{port}")),
            ("CLAUDE_CODE_SUBPROCESS_ENV_SCRUB", "1".into()),
            ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1".into()),
        ] {
            launch.environment.insert(key.into(), value.into());
        }
        let stop = confine(&mut launch, &f.scope, None, isolation).unwrap();
        let child = std::process::Command::new(&launch.executable)
            .args(&launch.arguments)
            .env_clear()
            .envs(&launch.environment)
            .current_dir(&launch.directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        (stop, child)
    };
    let present = |name: &str| {
        let output = run(
            isolation.client().args(["ps", "--all", "--quiet", "--filter", &format!("name=^{name}$")]),
            None,
            SHORT_WAIT,
        )
        .unwrap();
        assert!(output.success, "{}", output.stderr);
        !output.stdout.trim().is_empty()
    };
    // An engine that was already removing it may take a moment.
    let gone = |name: &str| {
        let until = Instant::now() + Duration::from_secs(10);
        while present(name) {
            if Instant::now() >= until {
                return false;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        true
    };
    let appeared = |name: &str| {
        let until = Instant::now() + Duration::from_secs(30);
        while !present(name) {
            if Instant::now() >= until {
                return false;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        true
    };

    // One turn: the tool asks for permission over stdio and runs.
    let (stop, mut child) = start(&f, &isolation);
    let mut stdin = child.stdin.take().unwrap();
    let (lines, frames) = mpsc::channel::<String>();
    let stdout = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = lines.send(line);
        }
    });
    // Owns the client's input: dropping it is the end of the conversation.
    let mut send = move |value: serde_json::Value| {
        writeln!(stdin, "{value}").unwrap();
        stdin.flush().unwrap();
    };
    send(serde_json::json!({"type":"control_request","request_id":"init","request":{"subtype":"initialize"}}));
    send(serde_json::json!({"type":"user","message":{"role":"user","content":"probe"}}));
    let deadline = Instant::now() + Duration::from_secs(90);
    let (mut asked, mut answer) = (false, None);
    while answer.is_none() {
        let line = frames
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or_else(|_| panic!("no answer; asked={asked}"));
        let Ok(frame) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if frame["type"] == "control_request" && frame["request"]["subtype"] == "can_use_tool" {
            asked = true;
            send(serde_json::json!({"type":"control_response","response":{"subtype":"success",
                "request_id":frame["request_id"],"response":{"behavior":"allow",
                "updatedInput":frame["request"]["input"]}}}));
        }
        if frame["type"] == "result" {
            answer = frame["result"].as_str().map(str::to_owned);
        }
    }
    let answer = answer.unwrap();
    eprintln!("tool command in the container: {answer}");
    assert!(asked, "the tool asked over stdio");
    let here = format!("here={}", f.scope.join("work").display());
    for line in ["env-has-key=0", "proc-with-key=0", "cli-visible=0", "image-write=no", here.as_str()] {
        assert!(answer.contains(line), "{line} in {answer}");
    }
    drop(send);
    let status = child.wait().unwrap();
    assert!(status.success(), "{status:?}");
    assert_eq!(stop.remove(), Ok(()));
    assert!(gone(stop.name()));

    // A killed client: the container is removed all the same.
    let (stop, mut child) = start(&f, &isolation);
    assert!(appeared(stop.name()));
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(stop.remove(), Ok(()));
    assert!(gone(stop.name()));

    // What an earlier host process left with other settings: its own sweep.
    let earlier_config = f.root.join("earlier-client");
    std::fs::create_dir(&earlier_config).unwrap();
    let earlier = ContainerIsolation::new(cli, endpoint, isolation.image.clone(), earlier_config).unwrap();
    let (stop, mut child) = start(&f, &earlier);
    assert!(appeared(stop.name()));
    earlier.sweep_once().unwrap();
    assert!(gone(stop.name()));
    let _ = child.kill();
    let _ = child.wait();
    let _ = server.kill();
    let _ = server.wait();
}
