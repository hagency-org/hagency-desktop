//! New daemon/OwnerRuntime lifecycle, using the NativeOwner OAuth fixture.
//! This is NOT encrypted Agent execution evidence: no Agent turn is launched.
use super::*;
use hagency::daemon::{self, bridge::ControllerClient, enrollment::Enrollment, host};
use hagency::native_owner::{
    Command as NativeCommand, MatrixAccessToken, MatrixTokenSource, NativeError, NativeOwner,
};
use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

const OWNER: &str = "@owner:example.test";
const TOKEN: &str = "daemon-fixture-access-token";
const CLIENT: &str = "fixture-public-client";
const DEVICE: &str = "OWNER_DAEMON_FIXTURE";

struct Source;
impl MatrixTokenSource for Source {
    fn access_token(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>> + Send + '_>,
    > {
        Box::pin(async {
            Ok(MatrixAccessToken {
                access_token: TOKEN.into(),
                client_id: CLIENT.into(),
            })
        })
    }
}

#[derive(Clone)]
struct MatrixApi {
    owner: &'static str,
    revocation_endpoint: String,
    fixture: OAuthFixture,
    whoami: Arc<AtomicU64>,
}
#[handler]
impl MatrixApi {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        let path = req.uri().path();
        let base = format!("{}/", self.fixture.origin);
        if path.ends_with("/account_data/m.secret_storage.default_key") {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Json(json!({"errcode":"M_NOT_FOUND","error":"fixture has no secret-storage default key"})));
            ctrl.skip_rest();
            return;
        }
        let value = if path.ends_with("/auth_metadata") {
            Some(
                json!({"issuer":format!("{base}auth/"),"authorization_endpoint":format!("{base}auth/oauth2/authorize"),"token_endpoint":format!("{base}auth/oauth2/token"),"revocation_endpoint":self.revocation_endpoint,"response_types_supported":["code"],"response_modes_supported":["query","fragment"],"grant_types_supported":["authorization_code","refresh_token"],"code_challenge_methods_supported":["S256"]}),
            )
        } else if path.ends_with("/account/whoami") {
            assert_eq!(
                req.headers().get("authorization").unwrap(),
                format!("Bearer {TOKEN}").as_str()
            );
            self.whoami.fetch_add(1, Ordering::AcqRel);
            Some(json!({"user_id":self.owner,"device_id":DEVICE}))
        } else if path.ends_with("/versions") {
            Some(json!({"versions":["v1.15"]}))
        } else if path == "/hagency/api/v1/devices/current" {
            assert_eq!(
                req.headers().get("authorization").unwrap(),
                format!("Bearer {}", "e".repeat(64)).as_str()
            );
            Some(
                json!({"userId":"stable-user-id","mxid":self.owner,"subject":self.owner,"clientId":CLIENT,"issuer":format!("{base}auth/"),"deviceId":"dev_fixture","generation":1,"validUntilMs":until()}),
            )
        } else if matches!(
            path,
            "/hagency/api/v1/management/catalog"
                | "/hagency/api/v1/management/pool-agent-requests/query"
        ) {
            Some(json!({"items":[],"nextAfter":null}))
        } else if path == "/hagency/api/v1/management/shared-execution/approvals/query" {
            Some(json!({"approvals":[]}))
        } else {
            None
        };
        if let Some(value) = value {
            res.render(Json(value));
            ctrl.skip_rest();
        } else {
            ctrl.call_next(req, depot, res).await;
        }
    }
}

async fn wait_running(home: &Path, owner: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut last = None;
    loop {
        if let host::Presence::Running(hello) = host::probe(home).await {
            if hello.hosting == hagency_metering::dashboard::HostingState::Running {
                return;
            }
            let mut client = daemon::DaemonClient::connect(home).await.unwrap();
            last = Some(
                client
                    .call(&daemon::Request::new(
                        daemon::new_request_id(),
                        Some(owner.into()),
                        daemon::Command::Snapshot {
                            period: None,
                            usage_filter: None,
                        },
                    ))
                    .await
                    .unwrap(),
            );
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "enrolled OwnerRuntime failed to run: {last:?}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
fn launch(
    home: &Path,
) -> (
    hagency_matrix::CancellationToken,
    tokio::task::JoinHandle<Result<(), host::HostError>>,
) {
    let stop = hagency_matrix::CancellationToken::new();
    let token = stop.clone();
    let path = home.to_path_buf();
    (
        stop,
        tokio::spawn(async move { host::run(&path, token).await }),
    )
}

/// Forward the SDK's HTTPS revocation request to the same OAuth fixture.
async fn tls_revocation_proxy(upstream: std::net::SocketAddr) -> (String, impl std::future::Future<Output = ()> + Send) {
    use tokio_rustls::{
        TlsAcceptor,
        rustls::{self, pki_types::{CertificateDer, PrivatePkcs8KeyDer}},
    };
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions().unwrap()
        .with_no_client_auth().with_single_cert(
            vec![CertificateDer::from(include_bytes!("../../../hagency-matrix/tests/fixtures/server.der").to_vec())],
            PrivatePkcs8KeyDer::from(include_bytes!("../../../hagency-matrix/tests/fixtures/public-test-key.der").to_vec()).into(),
        ).unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("https://{}/auth/oauth2/revoke", listener.local_addr().unwrap());
    let forward = async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                Ok((socket, _)) = listener.accept() => {
                    let acceptor = acceptor.clone();
                    connections.spawn(async move {
                        if let Ok(mut tls) = acceptor.accept(socket).await {
                            let mut http = tokio::net::TcpStream::connect(upstream).await.unwrap();
                            let _ = tokio::io::copy_bidirectional(&mut tls, &mut http).await;
                        }
                    });
                }
                Some(_) = connections.join_next() => (),
            }
        }
    };
    (url, forward)
}

async fn homeserver(owner: &'static str, tls_revocation: bool) -> (OAuthFixture, Arc<AtomicU64>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let fake = OAuthFixture {
        origin: format!("http://{address}"),
        state: Arc::new(Mutex::new(OAuthState {
            owner: owner.into(),
            active: true,
            challenge: None,
            enrollments: 0,
            lifetime: 900,
            refresh: false,
            refreshes: 0,
            revocations: 0,
            renewals: 0,
            token: TOKEN.into(),
            issuer_override: None,
            revoke_unavailable: false,
            binding_state: "joining".into(),
            agent_state: "creating".into(),
            management_calls: 0,
            commands: Default::default(),
            execution_device: Some("dev_fixture".into()),
            agent_generation: 1,
            direct_room: None,
            direct_mapping: None,
            direct_index: json!({}),
            direct_creates: 0,
            direct_unknown: false,
            direct_deny: false,
            direct_foreign: false,
            direct_journal: None,
        })),
    };
    let (revocation_endpoint, proxy) = if tls_revocation {
        let (url, task) = tls_revocation_proxy(address).await;
        (url, Some(task))
    } else {
        (format!("{}/auth/oauth2/revoke", fake.origin), None)
    };
    let whoami = Arc::new(AtomicU64::new(0));
    let service = Service::new(
        Router::new()
            .hoop(fake.clone())
            .hoop(MatrixApi {
                owner,
                revocation_endpoint,
                fixture: fake.clone(),
                whoami: whoami.clone(),
            })
            .push(Router::with_path("{**path}").goal(oauth)),
    );
    let acceptor = salvo::conn::TcpListener::new(address).bind().await;
    let server = tokio::spawn(async move {
        let serve = Server::new(acceptor).serve(service);
        if let Some(forward) = proxy {
            tokio::select! { _ = serve => (), _ = forward => () }
        } else {
            serve.await;
        }
    });
    (fake, whoami, server)
}

async fn enroll_account(home: &Path, root: &Path, origin: &str, owner: &str) -> Enrollment {
    let original = NativeOwner::open_with_matrix(root, origin, owner, Arc::new(Source))
        .await.unwrap();
    assert!(original.execute(NativeCommand::Agents).await.unwrap()["agents"].is_array());
    original.suspend().await.unwrap();
    drop(original);

    let matrix = home.join(daemon::MATRIX);
    hagency_store::private::directory(&matrix).unwrap();
    // The saved SDK session is fixture input, never borrowed from a UI SDK.
    let session = json!({"version":1,"homeserver":origin,"owner":owner,"oauthClientId":CLIENT,
        "user":{"user_id":owner,"device_id":DEVICE,"access_token":TOKEN}});
    hagency_store::private::write_new(&matrix.join("session.json"), &serde_json::to_vec(&session).unwrap()).unwrap();
    Enrollment::enroll(home, origin, owner, None, root).unwrap()
}

fn temporary_daemon_home() -> tempfile::TempDir {
    let home = tempfile::Builder::new().prefix("hag-daemon-").tempdir_in("/tmp").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(home.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    home
}

#[tokio::test]
async fn enrolled_owner_runtime_survives_two_controller_detaches_and_restart() {
    let (fake, whoami, server) = homeserver(OWNER, false).await;
    let owner_home = hagency_platform::legacy_writer::test_support::initialized_tempdir().unwrap();
    let root = owner_home.path().join("owner");
    let home = temporary_daemon_home();
    let origin = format!("{}/", fake.origin);
    let enrollment = enroll_account(home.path(), &root, &origin, OWNER).await;
    let matrix = home.path().join(daemon::MATRIX);
    assert_eq!(fake.state.lock().unwrap().enrollments, 1);
    let (stop, task) = launch(home.path());
    wait_running(home.path(), OWNER).await;
    let a = ControllerClient::new(home.path().into(), OWNER.into(), origin.clone(), 1);
    let b = ControllerClient::new(home.path().into(), OWNER.into(), origin.clone(), 2);
    for controller in [&a, &b] {
        assert!(controller.execute(NativeCommand::Agents).await.unwrap()["agents"].is_array());
    }
    assert!(
        NativeOwner::open_with_matrix(&root, &origin, OWNER, Arc::new(Source))
            .await
            .is_err(),
        "daemon must retain exclusive original root custody"
    );
    a.retire();
    b.retire();
    drop(a);
    drop(b);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !task.is_finished(),
        "detaching both controllers must not stop OwnerRuntime"
    );
    let detached_reads = whoami.load(Ordering::Acquire);
    tokio::time::timeout(Duration::from_secs(20), async {
        while whoami.load(Ordering::Acquire) == detached_reads {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("background OwnerRuntime must authenticate without either controller attached");
    wait_running(home.path(), OWNER).await;
    assert!(
        whoami.load(Ordering::Acquire) > 0,
        "the daemon's independent SDK must actually authenticate"
    );
    let key = std::fs::read(matrix.join("store-key")).unwrap();
    let session = std::fs::read(matrix.join("session.json")).unwrap();
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(15), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(
        host::probe(home.path()).await,
        host::Presence::NotRunning
    ));
    assert_eq!(
        Enrollment::load(home.path()).unwrap().unwrap().owner_root,
        enrollment.owner_root
    );

    let reads = whoami.load(Ordering::Acquire);
    let (stop, task) = launch(home.path());
    wait_running(home.path(), OWNER).await;
    let restored = ControllerClient::new(home.path().into(), OWNER.into(), origin.clone(), 3);
    assert!(restored.execute(NativeCommand::Agents).await.unwrap()["agents"].is_array());
    assert!(
        whoami.load(Ordering::Acquire) > reads,
        "restart must use restored independent SDK custody"
    );
    assert_eq!(
        fake.state.lock().unwrap().enrollments,
        1,
        "restore must not mint a duplicate execution-device enrollment"
    );
    assert_eq!(std::fs::read(matrix.join("store-key")).unwrap(), key);
    assert_eq!(std::fs::read(matrix.join("session.json")).unwrap(), session);
    restored.retire();
    drop(restored);
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(15), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    server.abort();
}

#[tokio::test]
async fn two_authenticated_accounts_share_a_daemon_and_sign_out_independently() {
    // Linux's TLS verifier accepts a CA bundle scoped to a child process.
    // Other platforms exercise the pending-revocation case with the HTTP
    // fixture. No system trust settings are changed.
    let tls = cfg!(target_os = "linux");
    if tls && std::env::var_os("HAGENCY_TEST_REVOCATION_TLS").is_none() {
        let ca = Path::new(env!("CARGO_MANIFEST_DIR")).join("../hagency-matrix/tests/fixtures/ca.pem");
        let thread = std::thread::current();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", thread.name().unwrap(), "--nocapture"])
            .env("SSL_CERT_FILE", ca).env("HAGENCY_TEST_REVOCATION_TLS", "1").output().unwrap();
        assert!(output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "isolated SDK logout test: {} {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        return;
    }
    const PEER: &str = "@owner:peer.test";
    let (first, _, first_server) = homeserver(OWNER, tls).await;
    let (second, _, second_server) = homeserver(PEER, false).await;
    let origins = [format!("{}/", first.origin), format!("{}/", second.origin)];
    let roots = hagency_platform::legacy_writer::test_support::initialized_tempdir().unwrap();
    let base = temporary_daemon_home();
    // Exercise a legacy account and a newly scoped account through one listener.
    enroll_account(base.path(), &roots.path().join("one"), &origins[0], OWNER).await;
    let peer = daemon::accounts::home(base.path(), &origins[1], PEER).unwrap();
    hagency_store::private::directory(peer.parent().unwrap()).unwrap();
    hagency_store::private::directory(&peer).unwrap();
    enroll_account(&peer, &roots.path().join("two"), &origins[1], PEER).await;
    let (stop, daemon) = launch(base.path());
    wait_running(base.path(), OWNER).await;
    wait_running(&peer, PEER).await;
    let peer_control = ControllerClient::new(peer.clone(), PEER.into(), origins[1].clone(), 1);
    let session = peer.join(daemon::MATRIX).join("session.json");
    let original_session = std::fs::read(&session).unwrap();
    let peer_revocations = second.state.lock().unwrap().revocations;
    let mut client = daemon::DaemonClient::connect(base.path()).await.unwrap();
    let pause = daemon::Request::new("pause-first", Some(OWNER.into()), daemon::Command::PauseHosting);
    client.call(&pause).await.unwrap().outcome().unwrap();
    assert!(peer_control.execute(NativeCommand::Agents).await.unwrap()["agents"].is_array());

    host::sign_out(base.path()).await.unwrap();
    if tls {
        let complete = tokio::time::timeout(Duration::from_secs(30), async {
            while Enrollment::load(base.path()).unwrap().is_some() {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }).await;
        if complete.is_err() {
            let request = daemon::Request::new("logout-state", Some(OWNER.into()), daemon::Command::Snapshot { period: None, usage_filter: None });
            let state = client.call(&request).await.unwrap().snapshot().unwrap().hosting;
            panic!("SDK sign-out did not finish: {state:?}; fixture revocations: {}", first.state.lock().unwrap().revocations);
        }
        assert!(!base.path().join(daemon::MATRIX).join("session.json").exists());
    } else {
        // RFC 7009 requires HTTPS. Native credentials are revoked and the
        // SDK retains a retry journal while the HTTP-only endpoint is refused.
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                if let host::Presence::Running(hello) = host::probe(base.path()).await
                    && hello.hosting == hagency_metering::dashboard::HostingState::Failed { break; }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }).await.expect("first account reports the unavailable revocation endpoint");
        assert!(daemon::revocation::load(base.path()).unwrap().is_some());
    }
    assert_eq!(std::fs::read(&session).unwrap(), original_session);
    assert!(peer_control.execute(NativeCommand::Agents).await.unwrap()["agents"].is_array());
    assert_eq!(second.state.lock().unwrap().revocations, peer_revocations);
    wait_running(&peer, PEER).await;
    peer_control.retire();
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(15), daemon).await.unwrap().unwrap().unwrap();
    first_server.abort();
    second_server.abort();
}
