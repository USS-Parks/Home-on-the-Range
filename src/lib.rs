//! Storage foundation. Service authorization is added by later approved prompts.

pub mod api;
pub mod capabilities;
pub mod credentials;
pub mod owner;
pub mod retrieval;
pub mod schema;
pub mod windows_security;
pub mod writer;

use rusqlite::{Connection, OpenFlags, ffi};
use serde::Serialize;
use std::{fmt, path::Path, time::Duration};

unsafe extern "C" {
    // The SQLCipher extension is absent from SQLite's baseline generated bindings.
    fn sqlite3_key(db: *mut ffi::sqlite3, key: *const std::ffi::c_void, length: i32) -> i32;
    fn OpenSSL_version(kind: i32) -> *const std::ffi::c_char;
}

#[derive(Debug)]
pub enum StoreError {
    InvalidKey,
    CipherUnavailable,
    OpenFailed,
    DatabaseRejected,
    ConfigurationFailed,
    UnsupportedSchema,
    NativeRejected(i32),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidKey => "passphrase length must be 16 to 1024 bytes",
            Self::CipherUnavailable => "required SQLCipher build unavailable",
            Self::OpenFailed => "database could not be opened",
            Self::DatabaseRejected => "database or passphrase rejected",
            Self::ConfigurationFailed => "encrypted storage configuration failed",
            Self::UnsupportedSchema => "vault schema is newer than this executable",
            Self::NativeRejected(_) => "encrypted database operation rejected",
        })
    }
}

impl std::error::Error for StoreError {}

#[derive(Debug, Serialize)]
pub struct NativeVersions {
    pub sqlcipher: String,
    pub sqlite: String,
    pub crypto_provider: String,
    pub crypto_version: String,
}

/// Reports linked versions without opening any vault or requiring a passphrase.
pub fn linked_native_versions() -> Result<NativeVersions, StoreError> {
    let connection = Connection::open_in_memory().map_err(|_| StoreError::CipherUnavailable)?;
    let sqlcipher: String = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .map_err(|_| StoreError::CipherUnavailable)?;
    if sqlcipher.split_whitespace().next() != Some("4.18.0") {
        return Err(StoreError::CipherUnavailable);
    }
    // SAFETY: OpenSSL's version API returns an immutable, process-lifetime C string.
    // Selector 0 requests the complete version. No database or user input is involved.
    let crypto_version = unsafe {
        let pointer = OpenSSL_version(0);
        if pointer.is_null() {
            return Err(StoreError::CipherUnavailable);
        }
        std::ffi::CStr::from_ptr(pointer)
            .to_str()
            .map_err(|_| StoreError::CipherUnavailable)?
            .to_owned()
    };
    Ok(NativeVersions {
        sqlcipher,
        sqlite: rusqlite::version().to_owned(),
        crypto_provider: "openssl (statically linked; no vault opened)".to_owned(),
        crypto_version,
    })
}

/// Checks the linked native implementation, including the cipher extension.
pub fn native_versions(connection: &Connection) -> Result<NativeVersions, StoreError> {
    let query = |sql| {
        connection
            .query_row(sql, [], |row| row.get::<_, String>(0))
            .map_err(|error| {
                StoreError::NativeRejected(
                    error.sqlite_error().map_or(0, |code| code.extended_code),
                )
            })
    };
    let sqlcipher = query("PRAGMA cipher_version")?;
    if sqlcipher.split_whitespace().next() != Some("4.18.0") {
        return Err(StoreError::CipherUnavailable);
    }
    Ok(NativeVersions {
        sqlcipher,
        sqlite: query("SELECT sqlite_version()")?,
        crypto_provider: query("PRAGMA cipher_provider")?,
        crypto_version: query("PRAGMA cipher_provider_version")?,
    })
}

/// Opens an already-existing file. Creation is exclusively a later owner operation.
/// Keys travel through the native API, never through SQL strings or diagnostics.
pub fn open_encrypted(path: &Path, passphrase: &[u8]) -> Result<Connection, StoreError> {
    let connection = keyed_connection(path, passphrase, None)?;
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA secure_delete = ON;",
        )
        .map_err(|_| StoreError::ConfigurationFailed)?;
    Ok(connection)
}

/// A schema probe never enables WAL or requests a writable database handle.
/// `Some(true)` is allowed only while the caller holds a write-denying handle
/// and has verified that neither a WAL nor a rollback journal exists.
pub(crate) fn keyed_connection(
    path: &Path,
    passphrase: &[u8],
    read_only: Option<bool>,
) -> Result<Connection, StoreError> {
    if !(16..=1024).contains(&passphrase.len()) {
        return Err(StoreError::InvalidKey);
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|_| StoreError::OpenFailed)?;
    if !metadata.file_type().is_file() {
        return Err(StoreError::OpenFailed);
    }
    let connection = if let Some(immutable) = read_only {
        let name = path.canonicalize().map_err(|_| StoreError::OpenFailed)?;
        let name = name
            .to_str()
            .ok_or(StoreError::OpenFailed)?
            .trim_start_matches(r"\\?\")
            .replace('\\', "/");
        let encoded: String = name
            .bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"/:._-".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect();
        let mode = if immutable {
            "immutable=1"
        } else {
            "mode=ro&readonly_shm=1"
        };
        Connection::open_with_flags(
            format!("file:///{encoded}?{mode}"),
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
    } else {
        Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
    }
    .map_err(|_| StoreError::OpenFailed)?;
    // SAFETY: connection is live and exclusively owned here. SQLite copies the
    // supplied bytes before this call returns; length is bounded above.
    let result = unsafe {
        sqlite3_key(
            connection.handle(),
            passphrase.as_ptr().cast(),
            passphrase.len() as i32,
        )
    };
    if result != ffi::SQLITE_OK {
        return Err(StoreError::DatabaseRejected);
    }
    // Configure native logging before a schema-dependent query can reject a key.
    connection
        .execute_batch("PRAGMA cipher_memory_security = ON; PRAGMA cipher_log_level = NONE;")
        .map_err(|_| StoreError::ConfigurationFailed)?;
    native_versions(&connection)?;
    connection
        .set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)
        .map_err(|_| StoreError::ConfigurationFailed)?;
    connection
        .query_row("SELECT count(*) FROM sqlite_schema", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|_| StoreError::DatabaseRejected)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|_| StoreError::ConfigurationFailed)?;
    connection
        .execute_batch(
            "PRAGMA trusted_schema = OFF; PRAGMA foreign_keys = ON; PRAGMA temp_store = MEMORY;",
        )
        .map_err(|_| StoreError::ConfigurationFailed)?;
    Ok(connection)
}
