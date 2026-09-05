use hotr::{
    StoreError, open_encrypted, owner,
    schema::{self, Kind, RecordInput, SourceReference, State},
    windows_security,
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const KEY: &[u8] = b"HOTR-05-synthetic-key-373a2b7d";
const BODY: &str = "HOTR05canary 東京 café e\u{301} 👩🏽‍💻 roadmap ../source";

fn run_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base = root.join("work/hotr-tests").canonicalize().unwrap();
    assert!(base.starts_with(root.canonicalize().unwrap()));
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let run = base.join(format!("HOTR-05-{}-{stamp}", std::process::id()));
    fs::create_dir(&run).unwrap();
    fs::write(
        run.join("SYNTHETIC-ONLY"),
        b"HOTR-05; synthetic schema fixtures\n",
    )
    .unwrap();
    run
}
fn snapshot(directory: &Path) -> BTreeMap<String, String> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (
                entry.file_name().to_string_lossy().to_string(),
                format!("{:x}", Sha256::digest(fs::read(entry.path()).unwrap())),
            )
        })
        .collect()
}
fn insert_fixture(connection: &mut Connection, namespace: &str, id: &str) {
    let tx = connection.transaction().unwrap();
    tx.execute("INSERT OR IGNORE INTO namespaces VALUES(?1)", [namespace])
        .unwrap();
    tx.execute(
        "INSERT INTO records VALUES(?1,?2,2)",
        params![namespace, id],
    )
    .unwrap();
    for (revision, state) in [(1, "proposed"), (2, "accepted")] {
        tx.execute(
            "INSERT INTO revisions VALUES(?1,?2,?3,'roadmap',?4,?5,1234)",
            params![namespace, id, revision, BODY, state],
        )
        .unwrap();
    }
    tx.commit().unwrap();
}
fn input() -> RecordInput {
    RecordInput {
        namespace: "personal/roadmaps".into(),
        id: "record_01".into(),
        kind: Kind::Roadmap,
        body: BODY.into(),
        state: State::Proposed,
        sources: vec![SourceReference {
            reference: "http://127.0.0.1:1/never-fetch; file:///never/read".into(),
            label: "opaque selected source".into(),
        }],
        tags: vec!["東京".into(), "work".into()],
    }
}

#[test]
fn migration_reopen_history_provenance_and_sql_constraints() {
    let run = run_dir();
    let vault = run.join("v1");
    windows_security::create_directory(&vault).unwrap();
    let path = vault.join("vault.db");
    drop(windows_security::create_file(&path).unwrap());
    let mut old = open_encrypted(&path, KEY).unwrap();
    old.execute_batch("CREATE TABLE hotr_vault(format INTEGER PRIMARY KEY CHECK(format=1)); INSERT INTO hotr_vault VALUES(1);").unwrap();
    old.execute_batch(include_str!("../src/schema_v1.sql"))
        .unwrap();
    insert_fixture(&mut old, "personal/roadmaps", "record_01");
    drop(old);
    assert_eq!(schema::inspect_version(&path, KEY).unwrap(), 1);
    let mut connection = schema::open(&path, KEY).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        schema::VERSION
    );
    let original = schema::revision(&connection, "personal/roadmaps", "record_01", Some(1))
        .unwrap()
        .unwrap();
    assert!(original.record.body == BODY);
    assert_eq!(original.record.state, State::Proposed);
    assert_eq!(
        schema::revision(&connection, "personal/roadmaps", "record_01", None)
            .unwrap()
            .unwrap()
            .revision,
        2
    );
    let source_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    source_listener.set_nonblocking(true).unwrap();
    let mut expected = input();
    expected.sources[0].reference = format!(
        "http://{}/do-not-fetch",
        source_listener.local_addr().unwrap()
    );
    expected.validate().unwrap();
    connection
        .execute(
            "INSERT INTO revision_sources VALUES('personal/roadmaps','record_01',2,0,?1,?2)",
            params![expected.sources[0].reference, expected.sources[0].label],
        )
        .unwrap();
    for (index, tag) in expected.tags.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO revision_tags VALUES('personal/roadmaps','record_01',2,?1,?2)",
                params![index as u32, tag],
            )
            .unwrap();
    }
    insert_fixture(&mut connection, "personal/roadmaps", "record_02");
    insert_fixture(&mut connection, "separate", "record_03");
    connection.execute("INSERT INTO relations VALUES('personal/roadmaps','record_01','record_02','depends_on')",[]).unwrap();
    for invalid in [
        "INSERT INTO relations VALUES('personal/roadmaps','record_01','missing','related')",
        "INSERT INTO relations VALUES('personal/roadmaps','record_01','record_03','related')",
        "INSERT INTO relations VALUES('personal/roadmaps','record_01','record_01','related')",
        "INSERT INTO relations VALUES('personal/roadmaps','record_01','record_02','execute')",
        "UPDATE revisions SET body='silently replace'",
        "DELETE FROM revisions",
        "UPDATE records SET current_revision=9",
        "INSERT INTO namespaces VALUES('../escape')",
        "INSERT INTO revision_tags VALUES('personal/roadmaps','record_01',2,32,'overflow')",
        "INSERT INTO revision_tags VALUES('personal/roadmaps','record_01',2,3,'work')",
        "INSERT INTO revision_sources VALUES('personal/roadmaps','record_01',2,16,'overflow','')",
    ] {
        assert!(
            connection.execute_batch(invalid).is_err(),
            "SQL constraint accepted malformed data"
        );
    }
    for body in [
        String::new(),
        "x".repeat(schema::MAX_BODY_BYTES + 1),
        "nul\0body".into(),
    ] {
        assert!(connection.execute("INSERT INTO revisions VALUES('personal/roadmaps','record_01',3,'note',?1,'proposed',0)",[body]).is_err());
    }
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
                .get::<_, u32>(
                0
            ))
            .unwrap(),
        0
    );
    drop(connection);
    let reopened = schema::open(&path, KEY).unwrap();
    assert!(
        schema::revision(&reopened, "personal/roadmaps", "record_01", Some(1))
            .unwrap()
            .unwrap()
            == original
    );
    let current = schema::revision(&reopened, "personal/roadmaps", "record_01", None)
        .unwrap()
        .unwrap();
    assert!(
        current.record.body == BODY
            && current.record.sources == expected.sources
            && current.record.tags == expected.tags
    );
    assert!(
        schema::revision(&reopened, "separate", "record_01", None)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        source_listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    fs::write(run.join("migration.json"),b"{\"result\":\"PASS\",\"migration\":\"1-to-2\",\"history_preserved\":true,\"opaque_sources_returned\":true,\"constraints_enforced\":true}").unwrap();
}

#[test]
fn typed_contract_preserves_unicode_and_enforces_byte_limits() {
    let record = input();
    record.validate().unwrap();
    assert!(
        serde_json::from_slice::<RecordInput>(&serde_json::to_vec(&record).unwrap()).unwrap()
            == record
    );
    for bytes in [schema::MAX_BODY_BYTES, schema::MAX_BODY_BYTES + 1] {
        let mut record = input();
        record.body = "x".repeat(bytes);
        assert_eq!(record.validate().is_ok(), bytes == schema::MAX_BODY_BYTES);
    }
    let mut multi = input();
    multi.body = "東京".repeat(11_000);
    assert!(multi.validate().is_err());
    let mut duplicate = input();
    duplicate.tags.push("work".into());
    assert!(duplicate.validate().is_err());
    let mut invalid = serde_json::to_value(input()).unwrap();
    invalid["principal"] = "owner".into();
    assert!(serde_json::from_value::<RecordInput>(invalid).is_err());
    for name in [
        "../escape",
        "a//b",
        "/absolute",
        "trailing/",
        "a/./b",
        "a/../b",
        "",
    ] {
        assert!(!schema::valid_identifier(name, true));
    }
}

#[test]
fn newer_closed_schema_is_byte_for_byte_untouched() {
    let run = run_dir();
    let vault = run.join("future");
    owner::create(&vault, KEY).unwrap();
    let path = vault.join("vault.db");
    let connection = open_encrypted(&path, KEY).unwrap();
    connection
        .execute_batch("PRAGMA user_version=999;")
        .unwrap();
    drop(connection);
    let before = snapshot(&vault);
    assert!(matches!(
        schema::open(&path, KEY),
        Err(StoreError::UnsupportedSchema)
    ));
    assert!(
        snapshot(&vault) == before,
        "newer database or sidecars changed"
    );
    fs::write(
        run.join("future-closed.json"),
        b"{\"result\":\"PASS\",\"files_byte_identical\":true}",
    )
    .unwrap();
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
#[ignore = "owned subprocess helper for the live WAL preservation gate"]
fn wal_fixture_child() {
    let run =
        PathBuf::from(std::env::var_os("HOTR_SCHEMA_FIXTURE").expect("synthetic path required"))
            .canonicalize()
            .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("work/hotr-tests")
        .canonicalize()
        .unwrap();
    assert!(
        run.starts_with(root)
            && fs::read_to_string(run.join("SYNTHETIC-ONLY"))
                .unwrap()
                .starts_with("HOTR-05;")
    );
    let connection = open_encrypted(&run.join("future/vault.db"), KEY).unwrap();
    connection
        .execute_batch("PRAGMA wal_autocheckpoint=0; PRAGMA user_version=999;")
        .unwrap();
    fs::write(run.join("wal-ready"), b"committed future version in WAL").unwrap();
    thread::sleep(Duration::from_secs(30));
    drop(connection);
}

#[test]
fn newer_crash_wal_schema_is_byte_for_byte_untouched() {
    let run = run_dir();
    let vault = run.join("future");
    owner::create(&vault, KEY).unwrap();
    let stderr = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(run.join("child.stderr.txt"))
        .unwrap();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["wal_fixture_child", "--ignored", "--exact"])
            .env("HOTR_SCHEMA_FIXTURE", &run)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()
            .unwrap(),
    );
    let start = Instant::now();
    while !run.join("wal-ready").exists() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "WAL helper readiness timeout"
        );
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "WAL helper failed; inspect retained stderr"
        );
        thread::sleep(Duration::from_millis(20));
    }
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    assert!(fs::metadata(vault.join("vault.db-wal")).unwrap().len() > 32);
    let before = snapshot(&vault);
    let result = schema::open(&vault.join("vault.db"), KEY);
    eprintln!("future WAL probe outcome: {:?}", result.as_ref().err());
    assert!(
        matches!(result, Err(StoreError::UnsupportedSchema)),
        "must read the committed future version from WAL"
    );
    assert!(
        snapshot(&vault) == before,
        "newer crash WAL fixture changed"
    );
    fs::write(run.join("future-wal.json"),b"{\"result\":\"PASS\",\"committed_wal_version_checked\":true,\"files_byte_identical\":true}").unwrap();
}
