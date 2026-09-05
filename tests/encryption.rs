use hotr::{native_versions, open_encrypted};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

fn new_run() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parent = root.join("work/hotr-tests");
    fs::create_dir_all(&parent).unwrap();
    assert!(
        parent
            .canonicalize()
            .unwrap()
            .starts_with(root.canonicalize().unwrap())
    );
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let run = parent.join(format!("HOTR-02-{}-{nonce}", std::process::id()));
    fs::create_dir(&run).unwrap();
    fs::write(
        run.join("SYNTHETIC-ONLY"),
        b"HOTR-02; retained test evidence; no user data",
    )
    .unwrap();
    run
}

fn scan(path: &Path, patterns: &[Vec<u8>]) -> usize {
    let mut count = 0;
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        assert!(!entry.file_type().unwrap().is_symlink());
        if entry.file_type().unwrap().is_dir() {
            count += scan(&entry.path(), patterns);
        } else {
            let bytes = fs::read(entry.path()).unwrap();
            for pattern in patterns {
                assert!(
                    !bytes.windows(pattern.len()).any(|window| window == pattern),
                    "plaintext canary found in test storage"
                );
            }
            count += 1;
        }
    }
    count
}

#[test]
fn native_windows_encryption_gate() {
    let diagnostic = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .arg("native-info")
        .output()
        .unwrap();
    assert!(
        diagnostic.status.success(),
        "metadata-only command must work while locked"
    );
    let metadata: serde_json::Value = serde_json::from_slice(&diagnostic.stdout).unwrap();
    assert_eq!(metadata["sqlcipher"], "4.18.0 community");
    assert!(
        metadata["crypto_version"]
            .as_str()
            .unwrap()
            .contains("4.0.2")
    );
    let run = new_run();
    let vault = run.join("synthetic.db");
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&vault)
        .unwrap();
    let key = Zeroizing::new(format!(
        "synthetic-key-{:x}",
        Sha256::digest(run.file_name().unwrap().to_string_lossy().as_bytes())
    ));
    let canary = Zeroizing::new(format!("hotrcanary{:x}", Sha256::digest(key.as_bytes())));
    let patterns = vec![
        canary.as_bytes().to_vec(),
        canary.encode_utf16().flat_map(u16::to_le_bytes).collect(),
        key.as_bytes().to_vec(),
    ];
    let connection = open_encrypted(&vault, key.as_bytes()).unwrap();
    let versions = native_versions(&connection).unwrap();
    assert!(versions.crypto_version.contains("4.0.2"));
    connection
        .execute_batch(
            "PRAGMA wal_autocheckpoint=0; CREATE VIRTUAL TABLE context_fts USING fts5(body);",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO context_fts(body) VALUES(?1)",
            [canary.as_str()],
        )
        .unwrap();
    connection
        .execute_batch("PRAGMA wal_checkpoint(FULL)")
        .unwrap();
    connection
        .execute(
            "INSERT INTO context_fts(body) VALUES(?1)",
            [canary.as_str()],
        )
        .unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM context_fts WHERE context_fts MATCH ?1",
            [canary.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
    let journal: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal, "wal");
    let sync: i64 = connection
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .unwrap();
    assert_eq!(sync, 2);
    assert!(fs::metadata(run.join("synthetic.db-wal")).unwrap().len() > 32);
    let live_files_scanned = scan(&run, &patterns);
    assert!(
        live_files_scanned >= 4,
        "DB, WAL, SHM, and ownership marker required"
    );
    let temp_store: i64 = connection
        .query_row("PRAGMA temp_store", [], |row| row.get(0))
        .unwrap();
    assert_eq!(temp_store, 2);
    let forced_memory: i64 = connection
        .query_row(
            "SELECT sqlite_compileoption_used('TEMP_STORE=3')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(forced_memory, 1);
    connection
        .execute_batch("CREATE TEMP TABLE scratch AS SELECT body FROM context_fts ORDER BY body;")
        .unwrap();
    assert!(
        connection
            .prepare("SELECT load_extension('untrusted')")
            .is_err()
    );
    drop(connection);
    let reopened = open_encrypted(&vault, key.as_bytes()).unwrap();
    let count: i64 = reopened
        .query_row(
            "SELECT count(*) FROM context_fts WHERE context_fts MATCH ?1",
            [canary.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
    let integrity: String = reopened
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    assert!(
        reopened
            .prepare("PRAGMA cipher_integrity_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none()
    );
    drop(reopened);
    assert!(open_encrypted(&vault, b"wrong-synthetic-passphrase").is_err());
    assert!(open_encrypted(&vault, b"").is_err());
    let missing = run.join("must-not-create.db");
    assert!(open_encrypted(&missing, key.as_bytes()).is_err());
    assert!(!missing.exists());
    assert!(open_encrypted(Path::new(":memory:"), key.as_bytes()).is_err());
    assert!(open_encrypted(&run, key.as_bytes()).is_err());
    let unkeyed = Connection::open(&vault).unwrap();
    assert!(
        unkeyed
            .query_row("SELECT count(*) FROM sqlite_schema", [], |row| row
                .get::<_, i64>(0))
            .is_err()
    );
    drop(unkeyed);

    let corrupt = run.join("synthetic-corrupt.db");
    let mut damaged = fs::read(&vault).unwrap();
    damaged[64] ^= 0x80;
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&corrupt)
        .unwrap()
        .write_all(&damaged)
        .unwrap();
    assert!(open_encrypted(&corrupt, key.as_bytes()).is_err());

    let python = std::env::var_os("HOTR_PYTHON").unwrap_or_else(|| "python".into());
    let plain = Command::new(python)
        .args([
            "-I",
            "-B",
            "-c",
            r#"
import sqlite3, sys, pathlib
c = sqlite3.connect(pathlib.Path(sys.argv[1]).as_uri() + '?mode=ro', uri=True)
assert c.execute('PRAGMA cipher_version').fetchone() is None
try:
    c.execute('SELECT count(*) FROM sqlite_schema').fetchone()
except sqlite3.DatabaseError:
    print('ordinary SQLite rejected encrypted database')
else:
    raise SystemExit('FAIL: ordinary SQLite read the encrypted database')
"#,
        ])
        .arg(&vault)
        .output()
        .expect("ordinary Python SQLite is required for this gate");
    assert!(
        plain.status.success(),
        "ordinary SQLite rejection check failed: {}",
        String::from_utf8_lossy(&plain.stderr)
    );
    let closed_files_scanned = scan(&run, &patterns);
    let file_hash = format!("{:x}", Sha256::digest(fs::read(&vault).unwrap()));
    let report = serde_json::json!({
        "prompt": "HOTR-02", "result": "PASS", "versions": versions,
        "canary_seed_scheme": "run-basename-v2",
        "fts_reopened_count": count, "journal_mode": journal, "synchronous": sync,
        "live_files_scanned": live_files_scanned, "closed_files_scanned": closed_files_scanned,
        "wrong_key_rejected": true, "ordinary_sqlite_rejected": true,
        "keyless_rejected": true, "temp_store_compile_time_memory": true,
        "tampered_file_rejected": true, "memory_alias_rejected": true,
        "vault_sha256": file_hash,
        "limitations": "Synthetic file encryption proof; no owner IPC, application integration, crash campaign, or RAM/pagefile/hibernation proof"
    });
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(run.join("result.json"))
        .unwrap()
        .write_all(serde_json::to_string_pretty(&report).unwrap().as_bytes())
        .unwrap();
    // Retain all evidence; no temporary-directory destructor or cleanup command.
}
