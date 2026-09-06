//! Owner-selected, bounded imports. Source text is data, never instructions.
use crate::{
    schema::{self, Kind, RecordInput, SourceReference, State},
    writer::WriteError,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{self, Read},
    os::windows::{
        ffi::OsStringExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Component, Path, PathBuf, Prefix},
    sync::atomic::{AtomicBool, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
    FILE_SHARE_READ, FILE_SHARE_WRITE, GetDriveTypeW, GetFinalPathNameByHandleW,
};

pub const MAX_FILES: usize = 16;
pub const MAX_RECORDS: usize = 64;
pub const MAX_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Text,
    Markdown,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedFile {
    pub source: String,
    pub sha256: String,
    pub format: Format,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub namespace: String,
    pub files: Vec<SelectedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub batch: Batch,
    /// Omitted for a read-only preview. Commit must name its exact digest.
    #[serde(default)]
    pub commit: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    records: Vec<Input>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    kind: Kind,
    body: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Serialize)]
struct Entry {
    record: RecordInput,
    source_sha256: String,
    action: &'static str,
    current_revision: Option<u32>,
}

fn rejected() -> io::Error {
    io::Error::other(
        "import rejected: select bounded UTF-8 .txt/.md/.json files on a local drive; no links or traversal",
    )
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn local_path(path: &Path) -> io::Result<PathBuf> {
    // Reject network/device/drive-relative spelling before any filesystem access.
    let text = path.to_str().ok_or_else(rejected)?;
    if text
        .split(['/', '\\'])
        .any(|part| part == "." || part == "..")
    {
        return Err(rejected());
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        if path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(rejected());
        }
        std::env::current_dir()?.join(path)
    };
    let drive = match absolute.components().next() {
        Some(Component::Prefix(p)) => match p.kind() {
            Prefix::Disk(d) | Prefix::VerbatimDisk(d) => d,
            _ => return Err(rejected()),
        },
        _ => return Err(rejected()),
    };
    let drive_root = [drive as u16, b':' as u16, b'\\' as u16, 0];
    // SAFETY: drive_root is a terminated, three-character DOS root.
    if !matches!(unsafe { GetDriveTypeW(drive_root.as_ptr()) }, 2 | 3) {
        return Err(rejected());
    }
    crate::owner::safe_absolute(&absolute).map_err(|_| rejected())
}

fn final_path(file: &File) -> io::Result<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    // SAFETY: a live File owns the handle; buffer is writable for its given length.
    let length = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            0,
        )
    };
    if length == 0 || length as usize >= buffer.len() {
        return Err(rejected());
    }
    buffer.truncate(length as usize);
    Ok(PathBuf::from(OsString::from_wide(&buffer)))
}

fn directory_guard(path: &Path) -> io::Result<File> {
    // Metadata access and shared writes permit ordinary app activity, but deny
    // rename/delete while the selected path chain is being captured.
    let handle = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|_| rejected())?;
    let metadata = handle.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return Err(rejected());
    }
    Ok(handle)
}

/// Reads only individually selected files. It never enumerates a directory,
/// writes a source, opens a vault, follows a reparse point, or contacts a provider.
pub fn prepare(root: &Path, files: &[PathBuf], namespace: &str) -> io::Result<Batch> {
    if files.is_empty() || files.len() > MAX_FILES || !schema::valid_identifier(namespace, true) {
        return Err(rejected());
    }
    let root = local_path(root)?;
    let mut guards = Vec::new();
    let mut ancestors: Vec<_> = root.ancestors().collect();
    ancestors.reverse();
    for path in ancestors {
        guards.push(directory_guard(path)?);
    }
    let root = final_path(guards.last().ok_or_else(rejected)?)?;
    local_path(&root)?;
    let mut selected = Vec::new();
    let mut total = 0;
    let mut seen = HashSet::new();
    for relative in files {
        let text = relative.to_str().ok_or_else(rejected)?;
        if text.is_empty()
            || text
                .split(['/', '\\'])
                .any(|p| p.is_empty() || p == "." || p == "..")
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(rejected());
        }
        let path = local_path(&root.join(relative))?;
        let mut parent = root.clone();
        let parts: Vec<_> = relative.components().collect();
        for part in &parts[..parts.len() - 1] {
            parent.push(part.as_os_str());
            let guard = directory_guard(&parent)?;
            if !final_path(&guard)?.starts_with(&root) {
                return Err(rejected());
            }
            guards.push(guard);
        }
        let format = match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("txt") => Format::Text,
            Some("md") => Format::Markdown,
            Some("json") => Format::Json,
            _ => return Err(rejected()),
        };
        let mut file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&path)
            .map_err(|_| rejected())?;
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.file_attributes() & 0x400 != 0
            || metadata.len() > (MAX_BYTES - total) as u64
        {
            return Err(rejected());
        }
        let actual = final_path(&file)?;
        if !actual.starts_with(&root) || actual == root {
            return Err(rejected());
        }
        local_path(&actual)?;
        let text_path = actual
            .to_str()
            .ok_or_else(rejected)?
            .strip_prefix(r"\\?\")
            .ok_or_else(rejected)?;
        let source = url::Url::from_file_path(text_path)
            .map_err(|_| rejected())?
            .to_string();
        if !seen.insert(source.clone()) {
            return Err(rejected());
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take((MAX_BYTES - total + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES - total || bytes.len() as u64 != metadata.len() {
            return Err(rejected());
        }
        total += bytes.len();
        selected.push(SelectedFile {
            source,
            sha256: digest(&bytes),
            format,
            text: String::from_utf8(bytes).map_err(|_| rejected())?,
        });
        guards.push(file);
    }
    selected.sort_by(|a, b| a.source.cmp(&b.source));
    let batch = Batch {
        namespace: namespace.to_owned(),
        files: selected,
    };
    records(&batch).map_err(|_| rejected())?;
    let request = Request {
        batch: batch.clone(),
        commit: Some("0".repeat(64)),
    };
    if serde_json::to_vec(&crate::owner::AdminRequest::Import(request))?.len()
        > crate::api::MAX_REQUEST
    {
        return Err(rejected());
    }
    Ok(batch)
}

fn records(batch: &Batch) -> Result<Vec<(RecordInput, String)>, WriteError> {
    let invalid = WriteError::InvalidRequest;
    if !schema::valid_identifier(&batch.namespace, true)
        || batch.files.is_empty()
        || batch.files.len() > MAX_FILES
    {
        return Err(invalid);
    }
    let mut total = 0usize;
    let mut previous: Option<&str> = None;
    let mut result = Vec::new();
    for file in &batch.files {
        total = total.checked_add(file.text.len()).ok_or(invalid)?;
        let url = url::Url::parse(&file.source).map_err(|_| invalid)?;
        if total > MAX_BYTES
            || !hex(&file.sha256)
            || digest(file.text.as_bytes()) != file.sha256
            || file.source.len() > 1800
            || url.scheme() != "file"
            || url.host_str().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || previous.is_some_and(|p| p >= file.source.as_str())
        {
            return Err(invalid);
        }
        previous = Some(&file.source);
        let inputs = match file.format {
            Format::Text | Format::Markdown => vec![Input {
                kind: Kind::Note,
                body: file.text.clone(),
                tags: vec![],
            }],
            Format::Json => {
                serde_json::from_str::<Document>(&file.text)
                    .map_err(|_| invalid)?
                    .records
            }
        };
        if inputs.is_empty() || result.len() + inputs.len() > MAX_RECORDS {
            return Err(invalid);
        }
        for (ordinal, input) in inputs.into_iter().enumerate() {
            let identity = serde_json::to_vec(&(
                "hotr-import-v1",
                &batch.namespace,
                &file.source,
                &file.sha256,
                ordinal,
            ))
            .map_err(|_| invalid)?;
            let record = RecordInput {
                namespace: batch.namespace.clone(),
                id: format!("imp-{}", digest(&identity)),
                kind: input.kind,
                body: input.body,
                state: State::Proposed,
                sources: vec![SourceReference {
                    reference: format!(
                        "{}#hotr-sha256={}&record={ordinal}",
                        file.source, file.sha256
                    ),
                    label: format!("Owner-selected import; record {ordinal}"),
                }],
                tags: input.tags,
            };
            record.validate().map_err(|_| invalid)?;
            result.push((record, file.sha256.clone()));
        }
    }
    Ok(result)
}

pub(crate) fn execute(
    db: &mut Connection,
    request: Request,
    deadline: Instant,
    stopped: &AtomicBool,
) -> crate::capabilities::CommandResult {
    let records = records(&request.batch)?;
    if request.commit.as_ref().is_some_and(|value| !hex(value)) {
        return Err(WriteError::InvalidRequest);
    }
    let batch_hash =
        digest(&serde_json::to_vec(&request.batch).map_err(|_| WriteError::InvalidRequest)?);
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(approval) = &request.commit {
        let receipt = tx
            .query_row(
                "SELECT batch_hash,result_json FROM import_receipts WHERE preview_digest=?1",
                [approval],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((hash, result)) = receipt {
            if hash != batch_hash {
                return Err(WriteError::IdempotencyConflict);
            }
            return serde_json::from_str(&result).map_err(|_| WriteError::PersistenceRejected);
        }
    }
    let mut entries = Vec::new();
    for (record, source_sha256) in records {
        if stopped.load(Ordering::SeqCst) || Instant::now() >= deadline {
            return Err(WriteError::Stopped);
        }
        let current = schema::revision(&tx, &record.namespace, &record.id, None)?;
        if current.is_some()
            && schema::revision(&tx, &record.namespace, &record.id, Some(1))?.map(|r| r.record)
                != Some(record.clone())
        {
            return Err(WriteError::IdempotencyConflict);
        }
        entries.push(Entry {
            record,
            source_sha256,
            action: if current.is_some() {
                "duplicate"
            } else {
                "insert"
            },
            current_revision: current.map(|r| r.revision),
        });
    }
    let vault_identity: Vec<u8> =
        tx.query_row("SELECT nonce FROM import_identity WHERE id=1", [], |r| {
            r.get(0)
        })?;
    let preview_digest = digest(
        &serde_json::to_vec(&("hotr-preview-v1", &vault_identity, &batch_hash, &entries))
            .map_err(|_| WriteError::InvalidRequest)?,
    );
    let Some(approval) = request.commit else {
        return Ok(
            serde_json::json!({"outcome":"preview", "preview_digest":preview_digest,"batch_hash":batch_hash,"entries":entries}),
        );
    };
    if approval != preview_digest {
        return Err(WriteError::RevisionConflict);
    }
    let now: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| WriteError::PersistenceRejected)?
        .as_millis()
        .try_into()
        .map_err(|_| WriteError::PersistenceRejected)?;
    let mut inserted = 0;
    let mut receipts = Vec::new();
    for entry in &entries {
        if stopped.load(Ordering::SeqCst) || Instant::now() >= deadline {
            return Err(WriteError::Stopped);
        }
        if entry.current_revision.is_none() {
            crate::writer::append_revision(&tx, "owner", &entry.record, None, 1, now)?;
            inserted += 1;
        }
        receipts.push(serde_json::json!({"namespace":entry.record.namespace,"id":entry.record.id,"revision":entry.current_revision.unwrap_or(1),"action":entry.action}));
    }
    let result = serde_json::json!({"outcome":"committed","preview_digest":preview_digest,"batch_hash":batch_hash,"inserted":inserted,"duplicates":entries.len()-inserted,"records":receipts});
    tx.execute(
        "INSERT INTO import_receipts VALUES(?1,?2,?3,?4)",
        params![
            approval,
            batch_hash,
            serde_json::to_string(&result).map_err(|_| WriteError::PersistenceRejected)?,
            now
        ],
    )?;
    if stopped.load(Ordering::SeqCst) || Instant::now() >= deadline {
        return Err(WriteError::Stopped);
    }
    // A failed or lost COMMIT acknowledgement is reconciled with this same digest.
    tx.commit().map_err(|_| WriteError::OutcomeUnknown)?;
    Ok(result)
}
