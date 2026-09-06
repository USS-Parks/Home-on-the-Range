//! Owner-only encrypted snapshots and restoration into exclusively new paths.
use crate::{keyed_connection, owner, schema, windows_security as security};
use rusqlite::{
    Connection,
    backup::{Backup, StepResult},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

const MAX_BYTES: u64 = 1_073_741_824;
const BUDGET: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Watermark {
    pub schema_version: u32,
    pub records: i64,
    pub revisions: i64,
    pub receipts: i64,
    pub audit_sequence: i64,
    pub clients: i64,
    pub grants: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub snapshot_id: String,
    pub sqlcipher: String,
    pub bytes: u64,
    pub ciphertext_sha256: String,
    pub watermark: Watermark,
}

pub(crate) struct Request {
    pub destination: PathBuf,
    pub key: Zeroizing<Vec<u8>>,
}

fn rejected() -> io::Error {
    io::Error::other("backup or restore rejected; any new incomplete files retained")
}

fn valid_manifest(manifest: &Manifest) -> bool {
    let hex =
        |value: &str, length| value.len() == length && value.bytes().all(|b| b.is_ascii_hexdigit());
    manifest.format == 1
        && manifest.sqlcipher == "4.18.0"
        && (5..=schema::VERSION).contains(&manifest.watermark.schema_version)
        && (1..=MAX_BYTES).contains(&manifest.bytes)
        && hex(&manifest.snapshot_id, 32)
        && hex(&manifest.ciphertext_sha256, 64)
        && [
            manifest.watermark.records,
            manifest.watermark.revisions,
            manifest.watermark.receipts,
            manifest.watermark.audit_sequence,
            manifest.watermark.clients,
            manifest.watermark.grants,
        ]
        .into_iter()
        .all(|value| value >= 0)
}

fn time_left(deadline: Instant) -> io::Result<()> {
    if Instant::now() >= deadline {
        Err(io::Error::other(
            "backup operation deadline; new incomplete files retained",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn encode(destination: &Path, key: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    let absolute = owner::safe_absolute(destination)?;
    let path = absolute.to_str().ok_or_else(rejected)?.as_bytes();
    if path.is_empty() || path.len() > 32768 || !(16..=1024).contains(&key.len()) {
        return Err(rejected());
    }
    let mut payload = Zeroizing::new(Vec::with_capacity(4 + path.len() + key.len()));
    payload.extend_from_slice(&(path.len() as u32).to_le_bytes());
    payload.extend_from_slice(path);
    payload.extend_from_slice(key);
    Ok(payload)
}

pub(crate) fn decode(payload: &[u8]) -> io::Result<Request> {
    if payload.len() < 4 {
        return Err(rejected());
    }
    let length = u32::from_le_bytes(payload[..4].try_into().map_err(|_| rejected())?) as usize;
    if length == 0
        || length > 32768
        || payload.len() < 4 + length
        || !(16..=1024).contains(&(payload.len() - 4 - length))
    {
        return Err(rejected());
    }
    let path = std::str::from_utf8(&payload[4..4 + length]).map_err(|_| rejected())?;
    Ok(Request {
        destination: owner::safe_absolute(Path::new(path))?,
        key: Zeroizing::new(payload[4 + length..].to_vec()),
    })
}

fn watermark(db: &Connection) -> io::Result<Watermark> {
    let scalar = |sql| {
        db.query_row(sql, [], |r| r.get::<_, i64>(0))
            .map_err(|_| rejected())
    };
    Ok(Watermark {
        schema_version: scalar("PRAGMA user_version")?
            .try_into()
            .map_err(|_| rejected())?,
        records: scalar("SELECT count(*) FROM records")?,
        revisions: scalar("SELECT count(*) FROM revisions")?,
        receipts: scalar("SELECT count(*) FROM write_receipts")?,
        audit_sequence: scalar("SELECT coalesce(max(sequence),0) FROM mutation_audit")?,
        clients: scalar("SELECT count(*) FROM clients")?,
        grants: scalar("SELECT count(*) FROM client_grants")?,
    })
}

fn integrity(db: &Connection, fts: bool, expected_schema: u32) -> io::Result<()> {
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|_| rejected())?;
    // Format 1 snapshots start with schema 5. Future schemas fail before creation.
    if version != expected_schema || !(5..=schema::VERSION).contains(&version) {
        return Err(rejected());
    }
    if db
        .query_row("SELECT format FROM hotr_vault", [], |r| r.get::<_, u32>(0))
        .map_err(|_| rejected())?
        != 1
    {
        return Err(rejected());
    }
    let checked: String = db
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|_| rejected())?;
    if checked != "ok" {
        return Err(rejected());
    }
    for sql in ["PRAGMA cipher_integrity_check", "PRAGMA foreign_key_check"] {
        let mut statement = db.prepare(sql).map_err(|_| rejected())?;
        if statement
            .query([])
            .map_err(|_| rejected())?
            .next()
            .map_err(|_| rejected())?
            .is_some()
        {
            return Err(rejected());
        }
    }
    if fts {
        db.execute(
            "INSERT INTO record_fts(record_fts) VALUES('integrity-check')",
            [],
        )
        .map_err(|_| rejected())?;
    }
    Ok(())
}

fn fresh(destination: &Path) -> io::Result<PathBuf> {
    let directory = owner::safe_absolute(destination)?;
    if directory.try_exists()? || !directory.parent().is_some_and(Path::is_dir) {
        return Err(io::Error::other(
            "destination must be new with an existing local parent; nothing replaced",
        ));
    }
    security::create_directory(&directory)?;
    security::verify_file_owner(&directory, true)?;
    drop(security::create_file(&directory.join("vault.db"))?);
    Ok(directory)
}

fn copy(source: &Connection, target: &mut Connection, deadline: Instant) -> io::Result<()> {
    let count: i64 = source
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .map_err(|_| io::Error::other("backup page count unavailable"))?;
    // SQLCipher returns cipher_page_size as TEXT, including for page_size.
    let size: i64 = source
        .query_row("PRAGMA cipher_page_size", [], |r| r.get::<_, String>(0))
        .map_err(|_| io::Error::other("backup page size unavailable"))?
        .parse()
        .map_err(|_| io::Error::other("backup page size invalid"))?;
    if count < 0 || size <= 0 || count.checked_mul(size).is_none_or(|n| n as u64 > MAX_BYTES) {
        return Err(io::Error::other("backup size rejected"));
    }
    let backup = Backup::new(source, target)
        .map_err(|_| io::Error::other("backup initialization rejected"))?;
    loop {
        time_left(deadline)?;
        match backup
            .step(128)
            .map_err(|_| io::Error::other("backup step rejected"))?
        {
            StepResult::Done => return Ok(()),
            StepResult::More => {}
            StepResult::Busy | StepResult::Locked => std::thread::sleep(Duration::from_millis(5)),
            _ => return Err(rejected()),
        }
    }
}

fn checks_with_deadline(
    db: &Connection,
    deadline: Instant,
    fts: bool,
    expected_schema: u32,
) -> io::Result<()> {
    db.progress_handler(1000, Some(move || Instant::now() >= deadline))
        .map_err(|_| rejected())?;
    let result = integrity(db, fts, expected_schema);
    db.progress_handler(0, None::<fn() -> bool>)
        .map_err(|_| rejected())?;
    result?;
    time_left(deadline)
}

fn file_hash(file: &mut fs::File, deadline: Instant) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut block = [0u8; 65536];
    loop {
        time_left(deadline)?;
        let count = file.read(&mut block)?;
        if count == 0 {
            break;
        }
        hash.update(&block[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn closed_file(path: &Path) -> io::Result<fs::File> {
    security::verify_file_owner(path, false)?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut side = path.as_os_str().to_owned();
        side.push(suffix);
        if Path::new(&side).try_exists()? {
            return Err(rejected());
        }
    }
    // Deny writes and deletion while hashing/opening the immutable snapshot.
    fs::OpenOptions::new().read(true).share_mode(1).open(path)
}

fn json_file(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| rejected())?;
    let mut file = security::create_file(path)?;
    file.write_all(&bytes)?;
    file.sync_all()
}

pub(crate) fn create(source: &Connection, request: Request) -> io::Result<Manifest> {
    let deadline = Instant::now() + BUDGET;
    let expected = watermark(source)?;
    let directory = fresh(&request.destination)?;
    let path = directory.join("vault.db");
    let mut target = keyed_connection(&path, &request.key, None).map_err(|_| rejected())?;
    target
        .execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")
        .map_err(|_| rejected())?;
    copy(source, &mut target, deadline)?;
    checks_with_deadline(&target, deadline, true, schema::VERSION)
        .map_err(|_| io::Error::other("encrypted snapshot integrity rejected"))?;
    if watermark(&target)? != expected {
        return Err(rejected());
    }
    target
        .execute_batch("PRAGMA journal_mode=DELETE;")
        .map_err(|_| rejected())?;
    drop(target);
    let mut file = closed_file(&path)?;
    let bytes = file.metadata()?.len();
    if bytes == 0 || bytes > MAX_BYTES {
        return Err(rejected());
    }
    let manifest = Manifest {
        format: 1,
        snapshot_id: crate::credentials::random_hex(16)?.to_string(),
        sqlcipher: "4.18.0".into(),
        bytes,
        ciphertext_sha256: file_hash(&mut file, deadline)?,
        watermark: expected,
    };
    json_file(&directory.join("backup.json"), &manifest)?;
    Ok(manifest)
}

pub fn restore(backup: &Path, destination: &Path, key: &[u8]) -> io::Result<serde_json::Value> {
    let deadline = Instant::now() + BUDGET;
    if !(16..=1024).contains(&key.len()) {
        return Err(rejected());
    }
    let backup = owner::safe_absolute(backup)?.canonicalize()?;
    security::verify_file_owner(&backup, true)?;
    let metadata = owner::safe_absolute(&backup.join("backup.json"))?;
    security::verify_file_owner(&metadata, false)?;
    let mut raw = Vec::new();
    let manifest_guard = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&metadata)?;
    (&manifest_guard).take(16385).read_to_end(&mut raw)?;
    if raw.len() > 16384 {
        return Err(rejected());
    }
    let manifest: Manifest = serde_json::from_slice(&raw).map_err(|_| rejected())?;
    if !valid_manifest(&manifest) {
        return Err(rejected());
    }
    let path = owner::safe_absolute(&backup.join("vault.db"))?;
    let mut source_guard = closed_file(&path)?;
    if source_guard.metadata()?.len() != manifest.bytes
        || file_hash(&mut source_guard, deadline)? != manifest.ciphertext_sha256
    {
        return Err(rejected());
    }
    let source = keyed_connection(&path, key, Some(true)).map_err(|_| rejected())?;
    checks_with_deadline(&source, deadline, false, manifest.watermark.schema_version)?;
    if watermark(&source)? != manifest.watermark {
        return Err(rejected());
    }
    // No destination creation until the closed input is authenticated and checked.
    let directory = fresh(destination)?;
    let mut target =
        keyed_connection(&directory.join("vault.db"), key, None).map_err(|_| rejected())?;
    target
        .execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")
        .map_err(|_| rejected())?;
    copy(&source, &mut target, deadline)?;
    if watermark(&target)? != manifest.watermark {
        return Err(rejected());
    }
    let invalidated = target
        .execute("UPDATE clients SET revoked=1 WHERE revoked=0", [])
        .map_err(|_| rejected())?;
    let active: i64 = target
        .query_row("SELECT count(*) FROM clients WHERE revoked=0", [], |r| {
            r.get(0)
        })
        .map_err(|_| rejected())?;
    if active != 0 {
        return Err(rejected());
    }
    // Migrate only the verified new copy. The guarded backup stays read-only;
    // credential revocation precedes migration and the final vault marker.
    target
        .progress_handler(1000, Some(move || Instant::now() >= deadline))
        .map_err(|_| rejected())?;
    let migrated = schema::migrate(&mut target).map_err(|_| rejected());
    target
        .progress_handler(0, None::<fn() -> bool>)
        .map_err(|_| rejected())?;
    migrated?;
    let mut expected = manifest.watermark.clone();
    expected.schema_version = schema::VERSION;
    if watermark(&target)? != expected {
        return Err(rejected());
    }
    checks_with_deadline(&target, deadline, true, schema::VERSION)?;
    target
        .execute_batch("PRAGMA journal_mode=DELETE;")
        .map_err(|_| rejected())?;
    drop(target);
    let _target_guard = closed_file(&directory.join("vault.db"))?;
    let result = serde_json::json!({"snapshot_id":manifest.snapshot_id,"watermark":manifest.watermark,"restored_schema_version":schema::VERSION,"clients_invalidated":invalidated,"active_clients":0,"reenrollment_required":true,"restored_key":"same passphrase as backup"});
    json_file(&directory.join("restore.json"), &result)?;
    // The ordinary vault marker is the final commit point. Failed staging has none.
    let mut marker = security::create_file(&directory.join(".hotr-vault"))?;
    marker.write_all(owner::MARKER)?;
    marker.sync_all()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_owner_backup_frames_fail_before_io() {
        for bytes in [
            vec![],
            vec![0; 4],
            vec![255; 4],
            vec![1, 0, 0, 0, 255],
            vec![1, 0, 0, 0, b'a'],
        ] {
            assert!(decode(&bytes).is_err());
        }
    }

    #[test]
    fn encrypted_native_backup_spans_multiple_steps() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("work/hotr-tests");
        owner::safe_absolute(&root).unwrap();
        fs::create_dir_all(&root).unwrap();
        let run = root.join(format!(
            "HOTR-07-backup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&run).unwrap();
        fs::write(
            run.join("SYNTHETIC-ONLY"),
            "HOTR-07; encrypted backup fixture\n",
        )
        .unwrap();
        owner::create(&run.join("source"), b"HOTR-07-synthetic-key-866bc4ad").unwrap();
        let source = schema::open(
            &run.join("source/vault.db"),
            b"HOTR-07-synthetic-key-866bc4ad",
        )
        .unwrap();
        source.execute_batch("CREATE TABLE backup_pages(value BLOB); WITH RECURSIVE span(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM span WHERE x<128) INSERT INTO backup_pages SELECT zeroblob(8192) FROM span;").unwrap();
        assert!(
            source
                .query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))
                .unwrap()
                > 128
        );
        create(
            &source,
            Request {
                destination: run.join("full"),
                key: Zeroizing::new(b"HOTR11BackupKey-8bcd49c1-different".to_vec()),
            },
        )
        .unwrap();
    }
}
