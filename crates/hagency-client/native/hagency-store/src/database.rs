//! Shared private-file and SQLite startup policy for explicitly named stores.
use crate::{Error, private};
use rusqlite::{Connection, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    path::Path,
    sync::{Mutex, OnceLock, PoisonError},
    time::Duration,
};

pub(crate) struct Database {
    pub connection: Connection,
    pub ownership: File,
}

/// One numbered migration: the schema version it brings a store to, its file
/// name and its SQL. Once a migration lands on main, its number, name and SQL
/// never change (ADR 0016, Known bounds).
#[derive(Clone, Copy)]
pub(crate) struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub(crate) struct Schema {
    pub name: &'static str,
    pub lock: &'static str,
    pub application_id: i32,
    pub version: i32,
    pub sql: &'static str,
    pub verify: &'static [&'static str],
    pub migrations: &'static [Migration],
}

/// The runner's own record of each numbered migration a store ran. No
/// migration creates it, so it takes no number: the runner creates it before
/// it applies anything, in every store whatever its version.
pub(crate) const MIGRATION_RECORDS: &str = "hagency_store_migrations";
const RECORDS: &str = "CREATE TABLE IF NOT EXISTS hagency_store_migrations(version INTEGER PRIMARY KEY NOT NULL CHECK(version>=2),name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 255),sql_sha256 TEXT NOT NULL CHECK(length(sql_sha256)=64))";

/// The SHA-256 of a migration's SQL, in lowercase hex. Each text is hashed
/// once per process, so opening an up-to-date store reads its records and
/// compares them, nothing more. The repository keeps migration files at LF
/// line ends on every system (`.gitattributes`), so every build of one file
/// has the same digest.
fn sql_sha256(sql: &'static str) -> String {
    static DIGESTS: OnceLock<Mutex<HashMap<(usize, usize), String>>> = OnceLock::new();
    // A 'static text's address and length name its bytes.
    let key = (sql.as_ptr() as usize, sql.len());
    let digests = DIGESTS.get_or_init(Default::default);
    if let Some(digest) = digests.lock().unwrap_or_else(PoisonError::into_inner).get(&key) {
        return digest.clone();
    }
    let digest = format!("{:x}", Sha256::digest(sql.as_bytes()));
    digests
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, digest.clone());
    digest
}

fn migration(migrations: &[Migration], version: i32) -> Result<&Migration, Error> {
    let mut matching = migrations.iter().filter(|m| m.version == version);
    match (matching.next(), matching.next()) {
        (Some(migration), None) => Ok(migration),
        _ => Err(Error::Schema),
    }
}

fn identity(name: &str, sha256: &str) -> String {
    format!("{name} (sha256 {})", sha256.get(..12).unwrap_or(sha256))
}

/// The records of the migrations a store ran, by version; `None` for a store
/// from before the records.
fn recorded(db: &Connection) -> Result<Option<BTreeMap<i32, (String, String)>>, Error> {
    let table: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [MIGRATION_RECORDS],
        |r| r.get(0),
    )?;
    if !table {
        return Ok(None);
    }
    let mut rows = db.prepare("SELECT version,name,sql_sha256 FROM hagency_store_migrations")?;
    let rows = rows
        .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?
        .collect::<Result<_, _>>()?;
    Ok(Some(rows))
}

/// Every record must be this build's migration of its number, and a store
/// with records has one for each version it is at. A store records its
/// migrations in the transaction that runs them, so a version without a
/// record was reached by a build that keeps none, past a build that does:
/// it is refused as well.
fn check(
    recorded: &BTreeMap<i32, (String, String)>,
    migrations: &[Migration],
    version: i32,
) -> Result<(), Error> {
    let expected = |number| {
        migration(migrations, number)
            .map(|m| identity(m.name, &sql_sha256(m.sql)))
            .unwrap_or_else(|_| "none".into())
    };
    for (&number, (name, sha256)) in recorded {
        let same = migration(migrations, number)
            .is_ok_and(|m| m.name == name && sql_sha256(m.sql) == *sha256);
        if !same {
            return Err(Error::MigrationMadeByAnotherBuild {
                version: number,
                stored: identity(name, sha256),
                expected: expected(number),
            });
        }
    }
    if !recorded.is_empty()
        && let Some(number) = (2..=version).find(|number| !recorded.contains_key(number))
    {
        return Err(Error::MigrationMadeByAnotherBuild {
            version: number,
            stored: "a migration it has no record of".into(),
            expected: expected(number),
        });
    }
    Ok(())
}

fn record(tx: &Connection, migration: &Migration) -> Result<(), Error> {
    // A record already there was checked against this build's list above;
    // only a test that rewinds `user_version` leaves one past the version.
    tx.execute(
        "INSERT OR IGNORE INTO hagency_store_migrations(version,name,sql_sha256) VALUES(?1,?2,?3)",
        params![migration.version, migration.name, sql_sha256(migration.sql)],
    )?;
    Ok(())
}
pub(crate) fn open(directory: &Path, definition: Schema) -> Result<Database, Error> {
    let Schema {
        name: database_name,
        lock: lock_name,
        application_id,
        version: expected_version,
        sql: schema,
        verify: verify_sql,
        migrations,
    } = definition;
    private::directory(directory)?;
    let lock_path = directory.join(lock_name);
    let lock = match private::open(&lock_path, true) {
        Ok(lock) => lock,
        Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            private::open(&lock_path, false)?
        }
        Err(e) => return Err(e),
    };
    lock.try_lock().map_err(|_| Error::Locked)?;
    let path = directory.join(database_name);
    let new = match private::open(&path, true) {
        Ok(_) => true,
        Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            private::open(&path, false)?;
            false
        }
        Err(e) => return Err(e),
    };
    for suffix in ["-wal", "-shm", "-journal"] {
        let suffix = format!("{database_name}{suffix}");
        if directory.join(&suffix).symlink_metadata().is_ok() {
            private::open_journal(&directory.join(&suffix))?;
        }
    }
    let mut db = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    // Even a rejected foreign/archive file must not be checkpointed or lose
    // its WAL companions when this connection drops during validation.
    db.set_db_config(
        rusqlite::config::DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE,
        true,
    )?;
    db.busy_timeout(Duration::from_millis(100))?;
    let version = if new {
        0
    } else {
        let id: i32 = db
            .pragma_query_value(None, "application_id", |r| r.get(0))
            .map_err(|_| Error::Schema)?;
        let version: i32 = db
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|_| Error::Schema)?;
        if id != application_id || version < 1 || version > expected_version {
            return Err(Error::Schema);
        }
        let check: String = db
            .query_row("PRAGMA quick_check(1)", [], |r| r.get(0))
            .map_err(|_| Error::Schema)?;
        if check != "ok" {
            return Err(Error::Schema);
        }
        version
    };
    // A newer store was refused above, as before the records. Every record
    // is checked before anything is applied.
    let recorded = if new { None } else { recorded(&db)? };
    if let Some(recorded) = &recorded {
        check(recorded, migrations, version)?;
    }
    // A store from before the records takes this build's for the versions it
    // has, once. That is trusted because released builds come from main,
    // where a migration's number, name and SQL never change once it lands; a
    // store a draft build migrated (never released) cannot be told apart and
    // is to be discarded. The verification queries below still run.
    let backfill = version >= 2 && recorded.as_ref().is_none_or(BTreeMap::is_empty);
    if version < expected_version || recorded.is_none() || backfill {
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(RECORDS)?;
        if backfill {
            for number in 2..=version {
                record(&tx, migration(migrations, number)?)?;
            }
        }
        let mut current = version;
        if new {
            tx.execute_batch(schema)?;
            tx.pragma_update(None, "application_id", application_id)?;
            current = 1;
        }
        while current < expected_version {
            let next = migration(migrations, current + 1)?;
            tx.execute_batch(next.sql)?;
            record(&tx, next)?;
            current = next.version;
        }
        for query in verify_sql {
            tx.prepare(query).map_err(|_| Error::Schema)?;
        }
        tx.pragma_update(None, "user_version", current)?;
        tx.commit()?;
    } else {
        for query in verify_sql {
            db.prepare(query).map_err(|_| Error::Schema)?;
        }
    }
    // The close path is a bounded resource release, not a checkpoint (ADR-120).
    // With checkpoints-on-close disabled SQLite skips the EXCLUSIVE lock and
    // the PASSIVE checkpoint at close and leaves both the -wal and the -shm in
    // place instead of unlinking them; committed frames stay in the WAL and
    // are replayed by the next open, which already admits leftover -wal, -shm
    // and -journal files above. Every commit is synced under synchronous=FULL,
    // so no committed data depends on the close-time checkpoint.
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.pragma_update(None, "synchronous", "FULL")?;
    db.pragma_update(None, "foreign_keys", "ON")?;
    Ok(Database {
        connection: db,
        ownership: lock,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;
    fn definition(version: i32, migrations: &'static [Migration]) -> Schema {
        Schema {
            name: "fixture.sqlite3",
            lock: "fixture.lock",
            application_id: 123456,
            version,
            sql: "CREATE TABLE probe(id INTEGER PRIMARY KEY); INSERT INTO probe VALUES(1);",
            verify: &["SELECT id FROM probe LIMIT 0"],
            migrations,
        }
    }
    #[test]
    fn native_schema_migrations_are_atomic() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        drop(open(&state, definition(1, &[])).unwrap());
        let failing = &[Migration {
            version: 2,
            name: "002-upgraded.sql",
            sql: "ALTER TABLE probe ADD COLUMN upgraded INTEGER; INSERT INTO missing_table VALUES(1);",
        }];
        assert!(open(&state, definition(2, failing)).is_err());
        let db = open(&state, definition(1, &[])).unwrap();
        assert_eq!(
            db.connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            1
        );
        assert!(db.connection.prepare("SELECT upgraded FROM probe").is_err());
        assert_eq!(
            db.connection
                .query_row("SELECT COUNT(*) FROM probe", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            1
        );
        drop(db);
        let success = &[Migration {
            version: 2,
            name: "002-upgraded.sql",
            sql: "ALTER TABLE probe ADD COLUMN upgraded INTEGER; UPDATE probe SET upgraded=7;",
        }];
        let mut later_verification_failure = definition(2, success);
        later_verification_failure.verify = &[
            "SELECT id, upgraded FROM probe LIMIT 0",
            "SELECT missing_column FROM probe LIMIT 0",
        ];
        assert!(matches!(
            open(&state, later_verification_failure),
            Err(Error::Schema)
        ));
        // Even after migration SQL and the first verification succeed, a later
        // invalid statement must roll back the schema and its version together.
        let db = open(&state, definition(1, &[])).unwrap();
        assert_eq!(
            db.connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            1
        );
        assert!(db.connection.prepare("SELECT upgraded FROM probe").is_err());
        assert_eq!(
            db.connection
                .query_row("SELECT id FROM probe", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            1
        );
        drop(db);
        for _ in 0..2 {
            let mut complete_verification = definition(2, success);
            complete_verification.verify = &[
                "SELECT id FROM probe LIMIT 0",
                "SELECT upgraded FROM probe LIMIT 0",
            ];
            let db = open(&state, complete_verification).unwrap();
            assert_eq!(
                db.connection
                    .query_row("SELECT upgraded FROM probe", [], |r| r.get::<_, i32>(0))
                    .unwrap(),
                7
            );
            assert_eq!(
                db.connection
                    .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                    .unwrap(),
                2
            );
        }
        // The already-current path must check every statement too, without
        // relying on a new migration to discover a malformed schema.
        let mut invalid_reopen = definition(2, success);
        invalid_reopen.verify = &[
            "SELECT id FROM probe LIMIT 0",
            "SELECT missing_column FROM probe LIMIT 0",
        ];
        assert!(matches!(open(&state, invalid_reopen), Err(Error::Schema)));
        assert!(matches!(
            open(&state, definition(1, &[])),
            Err(Error::Schema)
        ));
        // Exercise the actual domain1 -> domain2 migration with an existing pool.
        let old = root.path().join("old-domain");
        let database = open(
            &old,
            Schema {
                name: "domain.sqlite3",
                lock: "domain.lock",
                application_id: 0x48414732,
                version: 1,
                sql: include_str!("domain.sql"),
                verify: &["SELECT id FROM resources LIMIT 0"],
                migrations: &[],
            },
        )
        .unwrap();
        let config = r#"{"presetId":"pool","seatId":"seat","framework":"codex","model":"gpt-5.6-sol","reasoning":"medium","roles":["architect"],"ceiling":{"tokens":100},"published":true}"#;
        database
            .connection
            .execute(
                "INSERT INTO resources(id,preset_id,config) VALUES(?1,'pool',?2)",
                rusqlite::params![hagency_core::project::public_resource_id("pool"), config],
            )
            .unwrap();
        drop(database);
        let mut domain = crate::DomainRepository::open(&old).unwrap();
        let catalog = domain.catalog("", 100).unwrap();
        assert_eq!(catalog.len(), 1);
        assert!(catalog[0].roles.contains(&"coding".into()));
        assert!(!catalog[0].roles.contains(&"architect".into()));
        domain.set_role_publication("coding", false).unwrap();
        drop(domain);
        assert!(
            !crate::DomainRepository::open(&old)
                .unwrap()
                .catalog("", 100)
                .unwrap()[0]
                .roles
                .contains(&"coding".into())
        );
        // The domain store recorded each migration it ran, by its own list.
        let raw = Connection::open(old.join("domain.sqlite3")).unwrap();
        assert_eq!(
            raw.query_row(
                "SELECT count(*),min(version),max(version) FROM hagency_store_migrations",
                [],
                |r| Ok((r.get::<_, i32>(0)?, r.get::<_, i32>(1)?, r.get::<_, i32>(2)?)),
            )
            .unwrap(),
            (
                crate::domain::DOMAIN_SCHEMA_VERSION - 1,
                2,
                crate::domain::DOMAIN_SCHEMA_VERSION
            )
        );
    }

    const FIRST: Migration = Migration {
        version: 2,
        name: "002-upgraded.sql",
        sql: "ALTER TABLE probe ADD COLUMN upgraded INTEGER;",
    };
    const SECOND: Migration = Migration {
        version: 3,
        name: "003-second.sql",
        sql: "CREATE TABLE second(id INTEGER PRIMARY KEY);",
    };
    /// Another build's migration 2: the same file name with other SQL, as an
    /// earlier draft of it had.
    const DRAFT: Migration = Migration {
        version: 2,
        name: "002-upgraded.sql",
        sql: "CREATE TABLE draft(id INTEGER PRIMARY KEY);",
    };
    /// Another build's migration 2 by another name, with the same SQL.
    const RENAMED: Migration = Migration {
        version: 2,
        name: "002-renamed.sql",
        sql: "ALTER TABLE probe ADD COLUMN upgraded INTEGER;",
    };

    fn records(db: &Connection) -> Vec<(i32, String, String)> {
        db.prepare("SELECT version,name,sql_sha256 FROM hagency_store_migrations ORDER BY version")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }
    fn expected(migrations: &[Migration]) -> Vec<(i32, String, String)> {
        migrations
            .iter()
            .map(|m| {
                let sha256 = Sha256::digest(m.sql.as_bytes()).iter().fold(
                    String::new(),
                    |mut hex, byte| {
                        write!(hex, "{byte:02x}").unwrap();
                        hex
                    },
                );
                (m.version, m.name.to_owned(), sha256)
            })
            .collect()
    }
    fn user_version(db: &Connection) -> i32 {
        db.pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn a_fresh_store_records_every_migration_and_opens_again_without_a_write() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        let db = open(&state, definition(3, &[FIRST, SECOND])).unwrap();
        assert_eq!(records(&db.connection), expected(&[FIRST, SECOND]));
        drop(db);
        for _ in 0..2 {
            let db = open(&state, definition(3, &[FIRST, SECOND])).unwrap();
            // An up-to-date store with its records is only read at open.
            assert_eq!(db.connection.total_changes(), 0);
            assert_eq!(records(&db.connection), expected(&[FIRST, SECOND]));
            assert_eq!(user_version(&db.connection), 3);
        }
        // A newer store is still refused as before, not by its records.
        assert!(matches!(
            open(&state, definition(2, &[FIRST])),
            Err(Error::Schema)
        ));
    }

    #[test]
    fn a_store_that_ran_another_builds_migration_of_a_number_is_refused_by_name() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        drop(open(&state, definition(2, &[DRAFT])).unwrap());
        // At the same version and with a later migration to run: neither
        // runs over the draft's tables.
        for (version, migrations) in [(2, &[FIRST][..]), (3, &[FIRST, SECOND][..])] {
            let refused = match open(&state, definition(version, migrations)) {
                Err(error) => error,
                Ok(_) => panic!("another build's migration 2 was accepted"),
            };
            let Error::MigrationMadeByAnotherBuild {
                version: 2,
                stored,
                expected: this_build,
            } = &refused
            else {
                panic!("unexpected refusal {refused:?}");
            };
            assert_ne!(stored, this_build);
            assert!(stored.starts_with("002-upgraded.sql (sha256 "));
            assert!(
                refused
                    .to_string()
                    .starts_with("store migration 2 was made by another build")
            );
        }
        let raw = Connection::open(state.join("fixture.sqlite3")).unwrap();
        assert_eq!(user_version(&raw), 2);
        assert!(raw.prepare("SELECT id FROM second").is_err());
        assert!(raw.prepare("SELECT upgraded FROM probe").is_err());
        assert_eq!(records(&raw), expected(&[DRAFT]));
        drop(raw);
        // The same SQL by another name is another migration too.
        let renamed = root.path().join("renamed");
        drop(open(&renamed, definition(2, &[RENAMED])).unwrap());
        assert!(matches!(
            open(&renamed, definition(2, &[FIRST])),
            Err(Error::MigrationMadeByAnotherBuild { version: 2, .. })
        ));
    }

    #[test]
    fn a_store_from_before_the_records_takes_this_builds_once() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        drop(open(&state, definition(2, &[FIRST])).unwrap());
        // A store from before the records has neither the table nor rows.
        let raw = Connection::open(state.join("fixture.sqlite3")).unwrap();
        raw.execute_batch("DROP TABLE hagency_store_migrations").unwrap();
        drop(raw);
        let db = open(&state, definition(2, &[FIRST])).unwrap();
        assert_ne!(db.connection.total_changes(), 0);
        assert_eq!(records(&db.connection), expected(&[FIRST]));
        drop(db);
        let db = open(&state, definition(2, &[FIRST])).unwrap();
        assert_eq!(db.connection.total_changes(), 0);
        drop(db);
        // One from before the records and behind this build: its versions
        // take this build's records, and the next migration is recorded as
        // it runs.
        let raw = Connection::open(state.join("fixture.sqlite3")).unwrap();
        raw.execute_batch("DROP TABLE hagency_store_migrations").unwrap();
        drop(raw);
        let db = open(&state, definition(3, &[FIRST, SECOND])).unwrap();
        assert_eq!(records(&db.connection), expected(&[FIRST, SECOND]));
        assert_eq!(user_version(&db.connection), 3);
    }

    #[test]
    fn a_version_reached_without_a_record_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        drop(open(&state, definition(2, &[FIRST])).unwrap());
        // A build that keeps no records ran a migration 3 past a build that
        // does.
        let raw = Connection::open(state.join("fixture.sqlite3")).unwrap();
        raw.execute_batch("CREATE TABLE unknown(id INTEGER); PRAGMA user_version=3;")
            .unwrap();
        drop(raw);
        assert!(matches!(
            open(&state, definition(3, &[FIRST, SECOND])),
            Err(Error::MigrationMadeByAnotherBuild { version: 3, .. })
        ));
    }
}
