//! Versioned context. Transport authorization belongs to the service boundary.
use crate::{StoreError, keyed_connection, open_encrypted};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, os::windows::fs::OpenOptionsExt, path::Path};

pub const VERSION: u32 = 5;
pub const MAX_BODY_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Fact,
    Preference,
    Decision,
    Procedure,
    Roadmap,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Proposed,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceReference {
    pub reference: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordInput {
    pub namespace: String,
    pub id: String,
    pub kind: Kind,
    pub body: String,
    pub state: State,
    #[serde(default)]
    pub sources: Vec<SourceReference>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    #[serde(flatten)]
    pub record: RecordInput,
    pub revision: u32,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationError;
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("record fields exceed the permitted shape or bounds")
    }
}
impl std::error::Error for ValidationError {}

pub fn valid_identifier(value: &str, namespace: bool) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        })
        && (namespace || !value.contains('/'))
}

fn bounded_text(text: &str, limit: usize, empty: bool) -> bool {
    (empty || !text.is_empty()) && text.len() <= limit && !text.contains('\0')
}

impl RecordInput {
    pub fn validate(&self) -> Result<(), ValidationError> {
        let unique: HashSet<_> = self.tags.iter().collect();
        if !valid_identifier(&self.namespace, true)
            || !valid_identifier(&self.id, false)
            || !bounded_text(&self.body, MAX_BODY_BYTES, false)
            || self.sources.len() > 16
            || self.tags.len() > 32
            || unique.len() != self.tags.len()
            || self.sources.iter().any(|s| {
                !bounded_text(&s.reference, 2048, false) || !bounded_text(&s.label, 256, true)
            })
            || self.tags.iter().any(|t| !bounded_text(t, 64, false))
        {
            return Err(ValidationError);
        }
        Ok(())
    }
}

/// Hold a read handle that denies all writers during the probe. A WAL must be
/// read, never silently bypassed through immutable mode. A hot rollback journal
/// requires a later explicit recovery flow and is refused without mutation.
pub fn inspect_version(path: &Path, passphrase: &[u8]) -> Result<u32, StoreError> {
    let _guard = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .map_err(|_| StoreError::OpenFailed)?;
    let sidecar = |suffix: &str| {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        std::path::PathBuf::from(name)
    };
    if sidecar("-journal")
        .try_exists()
        .map_err(|_| StoreError::OpenFailed)?
    {
        return Err(StoreError::DatabaseRejected);
    }
    let has_wal = sidecar("-wal")
        .try_exists()
        .map_err(|_| StoreError::OpenFailed)?;
    // Windows SQLite supports a private, read-only WAL-index fallback via
    // readonly_shm=1. Require both existing sidecars and deny writes/deletion
    // throughout the probe; an absent index needs explicit recovery later.
    let mut sidecar_guards = Vec::new();
    if has_wal {
        for suffix in ["-wal", "-shm"] {
            sidecar_guards.push(
                fs::OpenOptions::new()
                    .read(true)
                    .share_mode(1)
                    .open(sidecar(suffix))
                    .map_err(|_| StoreError::OpenFailed)?,
            );
        }
    }
    let connection = keyed_connection(path, passphrase, Some(!has_wal))?;
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
        .map_err(|_| StoreError::DatabaseRejected)?;
    if version > VERSION {
        return Err(StoreError::UnsupportedSchema);
    }
    Ok(version)
}

pub fn open(path: &Path, passphrase: &[u8]) -> Result<Connection, StoreError> {
    inspect_version(path, passphrase)?;
    let mut connection = open_encrypted(path, passphrase)?;
    migrate(&mut connection)?;
    Ok(connection)
}

pub fn migrate(connection: &mut Connection) -> Result<(), StoreError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| StoreError::DatabaseRejected)?;
    let version = transaction
        .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
        .map_err(|_| StoreError::DatabaseRejected)?;
    if version > VERSION {
        return Err(StoreError::UnsupportedSchema);
    }
    let format = transaction
        .query_row("SELECT format FROM hotr_vault", [], |row| {
            row.get::<_, u32>(0)
        })
        .map_err(|_| StoreError::DatabaseRejected)?;
    if format != 1 {
        return Err(StoreError::DatabaseRejected);
    }
    if version < 1 {
        transaction
            .execute_batch(include_str!("schema_v1.sql"))
            .map_err(|_| StoreError::DatabaseRejected)?;
    }
    if version < 2 {
        transaction
            .execute_batch(include_str!("schema_v2.sql"))
            .map_err(|_| StoreError::DatabaseRejected)?;
    }
    if version < 3 {
        transaction
            .execute_batch(include_str!("schema_v3.sql"))
            .map_err(|_| StoreError::DatabaseRejected)?;
    }
    if version < 4 {
        transaction
            .execute_batch(include_str!("schema_v4.sql"))
            .map_err(|_| StoreError::DatabaseRejected)?;
    }
    if version < 5 {
        transaction
            .execute_batch(include_str!("schema_v5.sql"))
            .map_err(|_| StoreError::DatabaseRejected)?;
    }
    transaction
        .pragma_update(None, "user_version", VERSION)
        .map_err(|_| StoreError::DatabaseRejected)?;
    transaction
        .commit()
        .map_err(|_| StoreError::DatabaseRejected)
}

/// Internal exact lookup. The future API must authorize namespace and operation
/// before calling storage; source references remain opaque strings, never fetched.
pub fn revision(
    connection: &Connection,
    namespace: &str,
    id: &str,
    number: Option<u32>,
) -> rusqlite::Result<Option<Revision>> {
    let row = connection.query_row(
        "SELECT v.revision,v.kind,v.body,v.state,v.created_at_ms FROM revisions v JOIN records r ON r.namespace=v.namespace AND r.id=v.record_id WHERE v.namespace=?1 AND v.record_id=?2 AND v.revision=coalesce(?3,r.current_revision)",
        params![namespace,id,number], |row| Ok((row.get::<_,u32>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,i64>(4)?))
    ).optional()?;
    let Some((number, kind, body, state, created_at_ms)) = row else {
        return Ok(None);
    };
    let decode = |text: &str| serde_json::Value::String(text.to_owned());
    let kind = serde_json::from_value(decode(&kind)).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let state =
        serde_json::from_value(decode(&state)).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let sources = connection.prepare("SELECT reference,label FROM revision_sources WHERE namespace=?1 AND record_id=?2 AND revision=?3 ORDER BY ordinal")?
        .query_map(params![namespace,id,number], |row| Ok(SourceReference {reference:row.get(0)?,label:row.get(1)?}))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let tags = connection.prepare("SELECT tag FROM revision_tags WHERE namespace=?1 AND record_id=?2 AND revision=?3 ORDER BY ordinal")?
        .query_map(params![namespace,id,number], |row| row.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Some(Revision {
        record: RecordInput {
            namespace: namespace.to_owned(),
            id: id.to_owned(),
            kind,
            body,
            state,
            sources,
            tags,
        },
        revision: number,
        created_at_ms,
    }))
}
