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
        login_with_changed_permissions(true).await;
    }
}

#[tokio::test]
async fn windows_acl_preserves_unverified_history_and_activates_fresh_session() {
    const NAME: &str = "login::homeserver::oauth_tests::windows_permissions::windows_acl_preserves_unverified_history_and_activates_fresh_session";
    if !credentials::tests::isolated_case(NAME, "HAGENCY_ACL_FRESH_CHILD") {
        login_with_changed_permissions(false).await;
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

async fn login_with_changed_permissions(matching: bool) {
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
        let archive = std::fs::read_dir(state)
            .unwrap()
            .map(Result::unwrap)
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("retained_db.json.unavailable-")
            })
            .unwrap();
        assert_eq!(std::fs::read(archive.path()).unwrap(), original);
        hagency_store::private::open(&archive.path(), false).unwrap();
    }
    assert!(old_db.join("matrix-sdk-crypto.sqlite3").is_file());
}
