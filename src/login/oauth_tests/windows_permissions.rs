use super::*;
use crate::persistence::{self, credential_state as credentials};
use matrix_sdk_crypto::{
    Account,
    store::{CryptoStore, types::PendingChanges},
};
use matrix_sdk_sqlite::SqliteCryptoStore;

#[tokio::test]
async fn windows_acl_recovers_matching_history() {
    const NAME: &str = "login::homeserver::oauth_tests::windows_permissions::windows_acl_recovers_matching_history";
    if !credentials::tests::isolated_case(NAME, "HAGENCY_ACL_HISTORY_CHILD") {
        login_with_changed_permissions(true, Interruption::None).await;
    }
}

#[tokio::test]
async fn windows_acl_preserves_unverified_history_and_activates_fresh_session() {
    const NAME: &str = "login::homeserver::oauth_tests::windows_permissions::windows_acl_preserves_unverified_history_and_activates_fresh_session";
    if !credentials::tests::isolated_case(NAME, "HAGENCY_ACL_FRESH_CHILD") {
        login_with_changed_permissions(false, Interruption::None).await;
    }
}

#[test]
fn windows_acl_refuses_a_hard_link_and_keeps_the_original_permissions() {
    let temporary = tempfile::tempdir().unwrap();
    let account = temporary.path().join("account");
    hagency_store::private::directory(&account).unwrap();
    let outside = temporary.path().join("outside");
    std::fs::write(&outside, b"sentinel").unwrap();
    assert!(
        std::process::Command::new("icacls.exe")
            .arg(&outside)
            .args(["/grant", "*S-1-1-0:(M)"])
            .status()
            .unwrap()
            .success()
    );
    let linked = account.join("retained_db.json");
    std::fs::hard_link(&outside, &linked).unwrap();
    let paths = hagency_platform::private_state::AccountRepairPath::at(&account).unwrap();
    assert!(
        paths
            .repair_file("retained_db.json", Some("archived"), |_| panic!(
                "linked file reached validation"
            ))
            .is_err()
    );
    assert_eq!(std::fs::read(&linked).unwrap(), b"sentinel");
    assert!(!account.join("archived").exists());
    std::fs::remove_file(linked).unwrap();
    assert!(hagency_store::private::open(&outside, false).is_err());
}

#[test]
fn windows_acl_interrupted_archive_copy_keeps_original_until_private_copy_is_durable() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    hagency_store::private::directory(&state).unwrap();
    let source = state.join("session");
    let original = b"unverified fixture bytes";
    hagency_store::private::write_new(&source, original).unwrap();
    icacls(&source, &["/grant", "*S-1-1-0:(M)"]);
    let pending = "session.unavailable-pending";
    let copying = state.join(format!("{pending}.copying"));
    hagency_store::private::write_new(&copying, &original[..5]).unwrap();
    let busy = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&copying)
        .unwrap();
    let held = hagency_platform::private_state::AccountRepairPath::at(&state).unwrap();
    assert!(
        held.repair_file("session", Some(pending), |_| Ok(false))
            .is_err()
    );
    assert!(hagency_store::private::open(&source, false).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert!(!state.join(pending).exists());
    drop(busy);
    assert!(
        held.repair_file("session", Some(pending), |_| Ok(false))
            .unwrap()
    );
    assert!(!source.exists());
    assert!(!copying.exists());
    assert_eq!(std::fs::read(state.join(pending)).unwrap(), original);
    hagency_store::private::open(&state.join(pending), false).unwrap();

    // A different replacement of the same length cannot consume the first archive.
    let replacement = vec![b'x'; original.len()];
    hagency_store::private::write_new(&source, &replacement).unwrap();
    icacls(&source, &["/grant", "*S-1-1-0:(M)"]);
    assert!(
        held.repair_file("session", Some(pending), |_| Ok(false))
            .unwrap()
    );
    assert_eq!(std::fs::read(state.join(pending)).unwrap(), replacement);
    let previous = std::fs::read_dir(&state)
        .unwrap()
        .map(Result::unwrap)
        .find(|item| {
            item.file_name()
                .to_string_lossy()
                .starts_with("session.unavailable-pending-")
        })
        .unwrap();
    assert_eq!(std::fs::read(previous.path()).unwrap(), original);
    hagency_store::private::open(&previous.path(), false).unwrap();
}

#[derive(Clone, Copy)]
enum Interruption {
    None,
    PendingArchive,
    BeforeOriginalRemoval,
    LockedDatabase,
}

#[tokio::test]
async fn windows_acl_resumes_interrupted_archive_and_restores_verified_subset() {
    const NAME: &str = "login::homeserver::oauth_tests::windows_permissions::windows_acl_resumes_interrupted_archive_and_restores_verified_subset";
    if !credentials::tests::isolated_case(NAME, "HAGENCY_ACL_STAGE_CHILD") {
        login_with_changed_permissions(true, Interruption::PendingArchive).await;
    }
}

#[tokio::test]
async fn windows_acl_reuses_durable_archive_after_interrupted_source_removal() {
    const NAME: &str = "login::homeserver::oauth_tests::windows_permissions::windows_acl_reuses_durable_archive_after_interrupted_source_removal";
    if !credentials::tests::isolated_case(NAME, "HAGENCY_ACL_REMOVAL_CHILD") {
        login_with_changed_permissions(true, Interruption::BeforeOriginalRemoval).await;
    }
}

#[tokio::test]
async fn windows_acl_retries_history_after_a_database_lock() {
    const NAME: &str = "login::homeserver::oauth_tests::windows_permissions::windows_acl_retries_history_after_a_database_lock";
    if !credentials::tests::isolated_case(NAME, "HAGENCY_ACL_LOCK_CHILD") {
        login_with_changed_permissions(true, Interruption::LockedDatabase).await;
    }
}

fn icacls(path: &std::path::Path, args: &[&str]) {
    assert!(
        std::process::Command::new("icacls.exe")
            .arg(path)
            .args(args)
            .status()
            .unwrap()
            .success()
    );
}

async fn login_with_changed_permissions(matching: bool, interruption: Interruption) {
    let fixture = Fixture::new();
    let root = crate::app_data_dir();
    let service = credentials::tests::install_empty_store(root);
    let candidate = Client::builder()
        .homeserver_url(&fixture.origin)
        .build()
        .await
        .unwrap();
    let fresh = persistence::ClientSessionPersisted {
        homeserver: format!("{}/", fixture.origin),
        db_path: "db_new".into(),
        passphrase: "new-database-secret".into(),
    };
    credentials::register_preauth(&candidate, fresh.clone())
        .await
        .unwrap();
    authorize(&candidate, fixture.device.clone(), "fixture-code")
        .await
        .unwrap();
    let user = candidate.user_id().unwrap().to_owned();
    let old = persistence::ClientSessionPersisted {
        homeserver: fresh.homeserver.clone(),
        db_path: "db_original".into(),
        passphrase: "fixture-database-secret".into(),
    };
    let old_db = root.join(&old.db_path);
    hagency_store::private::directory(&old_db).unwrap();
    let crypto = SqliteCryptoStore::open(&old_db.canonicalize().unwrap(), Some(&old.passphrase))
        .await
        .unwrap();
    let old_owner = if matching {
        user.as_ref()
    } else {
        matrix_sdk::ruma::user_id!("@other:localhost")
    };
    crypto
        .save_pending_changes(PendingChanges {
            account: Some(Account::with_device_id(
                old_owner,
                candidate.device_id().unwrap(),
            )),
        })
        .await
        .unwrap();
    crypto.close().await.unwrap();
    let session = persistence::session_file_path(&user);
    let state = session.parent().unwrap();
    let account = state.parent().unwrap();
    let list = state.join("retained_db.json");
    hagency_store::private::directory(account).unwrap();
    hagency_store::private::directory(state).unwrap();
    // One unverifiable reference must not hide a second, matching database.
    let other = persistence::ClientSessionPersisted {
        db_path: "db_absent".into(),
        ..old.clone()
    };
    let original = serde_json::to_vec(&[&old, &other]).unwrap();
    hagency_store::private::write_new(&list, &original).unwrap();
    // Reproduce inherited Modify on both the legacy state directory and document.
    for (path, args) in [
        (account, vec!["/grant", "*S-1-1-0:(OI)(CI)(M)"]),
        (state, vec!["/inheritance:e"]),
        (list.as_path(), vec!["/inheritance:e"]),
    ] {
        assert!(
            std::process::Command::new("icacls.exe")
                .arg(path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    assert!(hagency_store::private::directory(account).is_err());
    assert!(hagency_store::private::open(&list, false).is_err());
    match interruption {
        Interruption::None => (),
        Interruption::PendingArchive | Interruption::BeforeOriginalRemoval => {
            // On disk immediately after preserving the original, before publishing the subset.
            let pending = list.with_file_name("retained_db.json.unavailable-pending");
            hagency_store::private::write_new(&pending, &original).unwrap();
            if matches!(interruption, Interruption::PendingArchive) {
                std::fs::remove_file(&list).unwrap();
            }
        }
        Interruption::LockedDatabase => {
            use std::os::windows::fs::OpenOptionsExt;
            let db = old_db.join("matrix-sdk-crypto.sqlite3");
            let check_retry = || async {
                assert!(
                    credentials::recover_verified_account(
                        &candidate,
                        crate::account_session::epoch()
                    )
                    .await
                    .is_err()
                );
                assert_eq!(std::fs::read(&list).unwrap(), original);
                assert!(std::fs::read_dir(state).unwrap().all(|entry| {
                    !entry
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .contains(".unavailable-")
                }));
            };
            let exclusive = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&db)
                .unwrap();
            check_retry().await;
            drop(exclusive);
            let sqlite = rusqlite::Connection::open(&db).unwrap();
            sqlite
                .execute_batch("PRAGMA journal_mode=DELETE; BEGIN EXCLUSIVE")
                .unwrap();
            check_retry().await;
            sqlite.execute_batch("ROLLBACK").unwrap();
            drop(sqlite);
        }
    }
    let (active, _) = crate::sliding_sync::finish_oauth_login(
        candidate,
        fresh.clone(),
        crate::account_session::epoch(),
    )
    .await
    .unwrap();
    assert_eq!(active.whoami().await.unwrap().user_id, user);
    // A later tool can widen a v2 session again while its keys stay in the vault.
    let encrypted = std::fs::read(&session).unwrap();
    assert!(
        std::process::Command::new("icacls.exe")
            .arg(&session)
            .args(["/grant", "*S-1-1-0:(M)"])
            .status()
            .unwrap()
            .success()
    );
    for _ in 0..2 {
        assert!(
            !credentials::recover_verified_account(&active, crate::account_session::epoch())
                .await
                .unwrap()
        );
    }
    assert_eq!(std::fs::read(&session).unwrap(), encrypted);
    let saved = credentials::test_load(service, session.clone())
        .await
        .unwrap();
    assert_eq!(
        saved.client_session.db_path,
        if matching { old.db_path } else { fresh.db_path }
    );
    hagency_store::private::directory(account).unwrap();
    hagency_store::private::directory(state).unwrap();
    if matching {
        hagency_store::private::open(&list, false).unwrap();
    }
    {
        let mut archives = std::fs::read_dir(state)
            .unwrap()
            .map(Result::unwrap)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("retained_db.json.unavailable-")
            });
        let archive = archives.next().unwrap();
        assert!(
            archives.next().is_none(),
            "one original must produce one complete archive"
        );
        assert_eq!(std::fs::read(archive.path()).unwrap(), original);
        hagency_store::private::open(&archive.path(), false).unwrap();
    }
    assert!(old_db.join("matrix-sdk-crypto.sqlite3").is_file());
}

#[tokio::test]
async fn windows_acl_normal_signin_restores_original_device_and_history() {
    const NAME: &str = "login::homeserver::oauth_tests::windows_permissions::windows_acl_normal_signin_restores_original_device_and_history";
    if credentials::tests::isolated_case(NAME, "HAGENCY_ACL_COLD_LOGIN_CHILD") {
        return;
    }
    let fixture = Fixture::new();
    let service = credentials::tests::install_empty_store(crate::app_data_dir());
    let login = || {
        let device = fixture.device.clone();
        crate::sliding_sync::run_sso_login(fixture.origin.clone(), None, false, move |url| {
            complete_authorization(url, device, "fixture-code")
        })
    };
    let (first, original) = login().await.unwrap();
    let user = first.user_id().unwrap().to_owned();
    let original_device = first.device_id().unwrap().to_owned();
    let path = persistence::session_file_path(&user);
    let (active, _) = crate::sliding_sync::finish_oauth_login(
        first,
        original.clone(),
        crate::account_session::epoch(),
    )
    .await
    .unwrap();
    let room = matrix_sdk::ruma::room_id!("!history:localhost");
    let event_id = matrix_sdk::ruma::event_id!("$before_acl_change");
    let message = json!({"type":"m.room.message","event_id":event_id,"sender":user,
        "origin_server_ts":1,"content":{"msgtype":"m.text","body":"message from the original login"}});
    let event = matrix_sdk::deserialized_responses::TimelineEvent::from_plaintext(
        serde_json::from_value(message.clone()).unwrap(),
    );
    active
        .event_cache_store()
        .lock()
        .await
        .unwrap()
        .as_clean()
        .unwrap()
        .save_event(room, event)
        .await
        .unwrap();
    crate::accounts::retain_database(crate::app_data_dir(), &user, original.clone())
        .await
        .unwrap();
    let saved = credentials::test_load(service.clone(), path.clone())
        .await
        .unwrap();
    credentials::tests::leave_session_save_intent(&service, &path, saved);
    drop(active);
    crate::account_session::close().await;
    crate::account_session::open();

    let state = path.parent().unwrap();
    assert!(state.join(".session.vault-journal").is_file());
    icacls(state.parent().unwrap(), &["/grant", "*S-1-1-0:(OI)(CI)(M)"]);
    icacls(state, &["/inheritance:e"]);
    icacls(&path, &["/inheritance:e"]);
    icacls(&state.join("retained_db.json"), &["/inheritance:e"]);
    let last = persistence::most_recent_user_id().await;
    assert_eq!(last.as_ref(), Some(&user));
    assert!(
        crate::sliding_sync::restore_previous_login(last)
            .await
            .is_err()
    );
    assert!(crate::sliding_sync::reauthentication_origin().is_some());
    *fixture.device.lock().unwrap() = None;
    let (second, selected) = login().await.unwrap();
    assert_eq!(
        fixture.device.lock().unwrap().as_deref(),
        Some(original_device.as_str())
    );
    assert_eq!(second.device_id(), Some(original_device.as_ref()));
    assert_eq!(selected.db_path, original.db_path);
    assert!(hagency_store::private::open(&path, false).is_err());
    let (restored, _) =
        crate::sliding_sync::finish_oauth_login(second, selected, crate::account_session::epoch())
            .await
            .unwrap();
    assert_eq!(restored.whoami().await.unwrap().user_id, user);
    let history = restored
        .event_cache_store()
        .lock()
        .await
        .unwrap()
        .as_clean()
        .unwrap()
        .find_event(room, event_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(history.raw().json().get()).unwrap(),
        message
    );
    hagency_store::private::open(&state.join("retained_db.json"), false).unwrap();
    assert!(!state.join(".session.vault-journal").exists());
    let persisted = credentials::test_load(service, path.clone()).await.unwrap();
    assert_eq!(persisted.client_session.db_path, original.db_path);
    assert_eq!(persisted.user_session.meta().device_id, original_device);
    assert_eq!(
        fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(route, _)| route.contains("/auth/register "))
            .count(),
        1
    );
    hagency_store::private::open(&path, false).unwrap();
}
