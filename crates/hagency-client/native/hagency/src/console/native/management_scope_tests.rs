use super::*;

fn context(instance: u64) -> service::HostContext {
    service::HostContext::new(
        "@owner:test".into(),
        1,
        "a".repeat(64),
        instance,
        "device".into(),
        1,
    )
    .unwrap()
}
fn session(store: &service::ServerManagementJournal, instance: u64) -> service::Session {
    service::Session::new_server(
        store.clone(),
        context(instance),
        [service::Permission::ReadResources],
        Instant::now() + Duration::from_secs(60),
    )
    .unwrap()
}
async fn fixture() -> (tempfile::TempDir, service::ServerManagementJournal) {
    let root = tempfile::tempdir().unwrap();
    let store = service::ServerManagementJournal::open(
        &root.path().join("server"), &"a".repeat(64), "@owner:test",
    ).unwrap();
    (root, store)
}
#[tokio::test]
async fn management_weak_registry_bounds_actual_live_scopes_and_prunes_dropped_cache_entries() {
    let (_root, store) = fixture().await;
    let registry = Registry::default();
    for _ in 0..100 {
        let old = session(&store, 1);
        registry.remember(old.clone()).unwrap();
        assert_eq!(registry.live_scope_count(), 1);
        drop(old);
        assert_eq!(registry.live_scope_count(), 0);
    }
    let mut live = Vec::new();
    for index in 1..=16 {
        let scope = session(&store, index);
        registry.remember(scope.clone()).unwrap();
        live.push(scope);
    }
    assert_eq!(registry.live_scope_count(), 16);
    let denied = session(&store, 17);
    assert_eq!(
        registry.remember(denied.clone()).unwrap_err().code,
        "management_capacity"
    );
    assert!(!denied.is_live());
    drop(live.pop());
    let admitted = session(&store, 18);
    registry.remember(admitted.clone()).unwrap();
    assert_eq!(registry.live_scope_count(), 16);
    registry.retire();
    assert!(live.iter().all(|scope| !scope.is_live()));
    assert!(!admitted.is_live());
    store.shutdown().await.unwrap();
}
#[tokio::test]
async fn management_package_retirement_fences_old_inflight_scope_after_cache_replacement() {
    let (_root, store) = fixture().await;
    let old = session(&store, 1);
    let latest = session(&store, 1);
    let mut cache = PackageSessions {
        deadline:None,
        current: Some((context(1), latest.clone())),
        issued: vec![old.authority().downgrade(), latest.authority().downgrade()],
    };
    assert!(old.is_live() && latest.is_live());
    cache.retire();
    assert!(!old.is_live() && !latest.is_live());
    assert!(cache.current.is_none());
    store.shutdown().await.unwrap();
}

/// What `authorized_device_for_cookie` issues: a session of this store,
/// remembered by the registry that admits sixteen live ones.
fn issuing<'a>(
    store: &'a service::ServerManagementJournal,
    registry: &'a Registry,
) -> impl Fn(Instant) -> Result<service::Session, NativeError> + 'a {
    move |expires| {
        let session = service::Session::new_server(
            store.clone(),
            context(1),
            [service::Permission::ReadResources],
            expires,
        )
        .unwrap();
        registry.remember(session.clone())?;
        Ok(session)
    }
}
#[tokio::test]
async fn calls_under_a_proof_longer_than_the_session_term_share_one_session() {
    let (_root, store) = fixture().await;
    let registry = Registry::default();
    let issue = issuing(&store, &registry);
    let mut sessions = PackageSessions::default();
    // Operations in flight keep the session they were given.
    let mut held = Vec::new();
    for _ in 0..17 {
        let now = Instant::now();
        // The proof of a two-minute authorization window, renewed a moment ago.
        let proof = now + Duration::from_secs(115);
        held.push(
            sessions
                .session_until(&context(1), proof, now, &issue)
                .expect("the seventeenth call is served too"),
        );
    }
    assert_eq!(registry.live_scope_count(), 1);
    assert!(held.iter().all(|session| session.valid_until() == held[0].valid_until()));
}
#[tokio::test]
async fn a_session_is_replaced_when_little_of_its_term_is_left_or_the_proof_ends_sooner() {
    let (_root, store) = fixture().await;
    let registry = Registry::default();
    let issue = issuing(&store, &registry);
    let mut sessions = PackageSessions::default();
    let now = Instant::now();
    let after = |seconds| now + Duration::from_secs(seconds);
    // Under a proof with twenty seconds left the session ends with the proof,
    // and every call under that proof shares it.
    let short = sessions.session_until(&context(1), after(20), now, &issue).unwrap();
    let again = sessions.session_until(&context(1), after(20), now, &issue).unwrap();
    assert_eq!(short.valid_until(), after(20));
    assert_eq!(again.valid_until(), after(20));
    assert_eq!(registry.live_scope_count(), 1);
    // A two-minute proof arrives. Twenty seconds are less than half a term,
    // so the next operation gets a session with a full term.
    let full = sessions.session_until(&context(1), after(115), now, &issue).unwrap();
    assert_eq!(full.valid_until(), now + SESSION_TERM);
    assert_eq!(registry.live_scope_count(), 2);
    // The proof then ends before that session would, as it does in a token's
    // last minute: the session may not outlive it.
    let cut = sessions.session_until(&context(1), after(45), now, &issue).unwrap();
    assert_eq!(cut.valid_until(), after(45));
}
#[test]
fn a_session_cut_to_its_term_serves_for_half_of_it() {
    let now = Instant::now();
    let after = |seconds| now + Duration::from_secs(seconds);
    // A two-minute proof: the term is the nearer end.
    let (proof, expires) = (after(115), now + SESSION_TERM);
    assert!(session_serves(after(50), expires, proof, now));
    assert!(session_serves(after(30), expires, proof, now));
    assert!(!session_serves(after(29), expires, proof, now));
    // The default window: the proof is the nearer end, and only the session
    // issued under this proof serves.
    let proof = after(25);
    assert!(session_serves(proof, proof, proof, now));
    assert!(!session_serves(after(20), proof, proof, now));
}
