use crate::{open_encrypted, windows_security as security};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    os::windows::fs::MetadataExt,
    path::{Component, Path, PathBuf, Prefix},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions},
    time::{sleep, timeout},
};
use zeroize::Zeroizing;

pub(crate) const MARKER: &[u8] = b"Home on the Range vault format 1\n";
const DEADLINE: Duration = Duration::from_secs(5);
const MAX_FRAME: usize = crate::api::MAX_REQUEST + 1;
pub const STATUS: u8 = 1;
pub const UNLOCK: u8 = 2;
pub const LOCK: u8 = 3;
pub const ADMIN: u8 = 4;
pub const BACKUP: u8 = 5;

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "arguments",
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub enum AdminRequest {
    ViewerSession { seconds: u32 },
    EmbeddingConfigure(crate::embedding::Configure),
    EmbeddingStatus,
    Lifecycle(crate::lifecycle::Request),
    Inspect(crate::lifecycle::Inspect),
    Import(crate::imports::Request),
    Issue(crate::capabilities::NewClient),
    Revoke { client_id: String },
    Clients,
    Accept(crate::capabilities::Accept),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reply {
    pub state: String,
    pub pid: u32,
    pub closing: bool,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<serde_json::Value>,
}

fn state(unlocked: bool, closing: bool, error: Option<&str>) -> Reply {
    Reply {
        state: if unlocked { "unlocked" } else { "locked" }.to_owned(),
        pid: std::process::id(),
        closing,
        error: error.map(str::to_owned),
        data: None,
    }
}

pub(crate) fn safe_absolute(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix)
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) => {}
            Component::RootDir => (),
            Component::Normal(name) => {
                let text = name
                    .to_str()
                    .ok_or_else(|| io::Error::other("path encoding rejected"))?;
                let stem = text.split('.').next().unwrap_or("").to_ascii_uppercase();
                let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                    || (stem.len() == 4
                        && (stem.starts_with("COM") || stem.starts_with("LPT"))
                        && matches!(stem.as_bytes()[3], b'1'..=b'9'));
                if text.contains([':', '\0']) || text.ends_with(['.', ' ']) || device {
                    return Err(io::Error::other("ambiguous Windows path rejected"));
                }
            }
            _ => {
                return Err(io::Error::other(
                    "vault path must be a local path without traversal",
                ));
            }
        }
    }
    for ancestor in absolute.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_attributes() & 0x400 != 0 => {
                return Err(io::Error::other("vault reparse point refused"));
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => (),
        }
    }
    Ok(absolute)
}

pub fn create(path: &Path, passphrase: &[u8]) -> io::Result<()> {
    if !(16..=1024).contains(&passphrase.len()) {
        return Err(io::Error::other("passphrase must contain 16 to 1024 bytes"));
    }
    let directory = safe_absolute(path)?;
    if directory.exists() {
        return Err(io::Error::other(
            "destination exists; no files were replaced",
        ));
    }
    if !directory.parent().is_some_and(Path::is_dir) {
        return Err(io::Error::other(
            "vault parent directory must already exist",
        ));
    }
    security::create_directory(&directory)?;
    security::verify_file_owner(&directory, true)?;
    let database = directory.join("vault.db");
    drop(security::create_file(&database)?);
    security::verify_file_owner(&database, false)?;
    let mut connection = open_encrypted(&database, passphrase).map_err(io::Error::other)?;
    connection.execute_batch("CREATE TABLE hotr_vault(format INTEGER PRIMARY KEY CHECK(format=1)); INSERT INTO hotr_vault VALUES(1);")
        .map_err(|_| io::Error::other("vault initialization failed; new files retained"))?;
    crate::schema::migrate(&mut connection).map_err(io::Error::other)?;
    drop(connection);
    security::create_file(&directory.join(".hotr-vault"))?.write_all(MARKER)?;
    Ok(())
}

pub fn validate(path: &Path) -> io::Result<PathBuf> {
    let directory = safe_absolute(path)?.canonicalize()?;
    security::verify_file_owner(&directory, true)?;
    for name in ["vault.db", ".hotr-vault"] {
        let file = directory.join(name);
        if !fs::symlink_metadata(&file)?.file_type().is_file() {
            return Err(io::Error::other("vault file type rejected"));
        }
        security::verify_file_owner(&file, false)?;
    }
    if fs::metadata(directory.join(".hotr-vault"))?.len() != MARKER.len() as u64
        || fs::read(directory.join(".hotr-vault"))? != MARKER
    {
        return Err(io::Error::other("unsupported vault marker"));
    }
    Ok(directory)
}

pub fn pipe_name(path: &Path) -> io::Result<String> {
    let directory = validate(path)?;
    let sid = security::current_sid()?;
    let name = format!("{sid}\0{}", directory.to_string_lossy().to_lowercase());
    Ok(format!(
        r"\\.\pipe\hotr-owner-v1-{:x}",
        Sha256::digest(name.as_bytes())
    ))
}

async fn read_frame(
    stream: &mut (impl AsyncRead + Unpin),
    limit: usize,
) -> io::Result<Zeroizing<Vec<u8>>> {
    let length = stream.read_u32_le().await? as usize;
    if length == 0 || length > limit {
        return Err(io::Error::other("owner message length rejected"));
    }
    let mut bytes = Zeroizing::new(vec![0; length]);
    stream.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn write_frame(stream: &mut (impl AsyncWrite + Unpin), bytes: &[u8]) -> io::Result<()> {
    stream.write_u32_le(bytes.len() as u32).await?;
    stream.write_all(bytes).await?;
    stream.flush().await
}

pub async fn connect(path: &Path) -> io::Result<NamedPipeClient> {
    let endpoint = pipe_name(path)?;
    let mut client = None;
    for _ in 0..50 {
        match ClientOptions::new().open(&endpoint) {
            Ok(pipe) => {
                client = Some(pipe);
                break;
            }
            Err(error) if matches!(error.raw_os_error(), Some(231 | 233)) => {
                sleep(Duration::from_millis(20)).await
            }
            Err(error) => return Err(error),
        }
    }
    let client = client.ok_or_else(|| io::Error::other("owner pipe busy"))?;
    if security::pipe_server_sid(&client)? != security::current_sid()? {
        return Err(io::Error::other("owner server identity rejected"));
    }
    Ok(client)
}

pub async fn request(path: &Path, operation: u8, passphrase: &[u8]) -> io::Result<Reply> {
    if passphrase.len() > 1024 {
        return Err(io::Error::other("passphrase length rejected"));
    }
    request_payload(path, operation, passphrase).await
}

pub async fn admin(path: &Path, request: &AdminRequest) -> io::Result<Reply> {
    let payload =
        serde_json::to_vec(request).map_err(|_| io::Error::other("owner arguments rejected"))?;
    request_payload(path, ADMIN, &payload).await
}

pub async fn backup(path: &Path, destination: &Path, key: &[u8]) -> io::Result<Reply> {
    let payload = crate::backup::encode(destination, key)?;
    request_payload(path, BACKUP, &payload).await
}

async fn request_payload(path: &Path, operation: u8, payload: &[u8]) -> io::Result<Reply> {
    if payload.len() > crate::api::MAX_REQUEST {
        return Err(io::Error::other("owner payload too large"));
    }
    timeout(DEADLINE, async {
        let mut client = connect(path).await?;
        let mut body = Zeroizing::new(Vec::with_capacity(payload.len() + 1));
        body.push(operation);
        body.extend_from_slice(payload);
        write_frame(&mut client, &body).await?;
        let response = read_frame(&mut client, crate::api::MAX_RESPONSE).await?;
        let reply = serde_json::from_slice(&response)
            .map_err(|_| io::Error::other("invalid owner response"))?;
        client.write_u8(1).await?;
        Ok(reply)
    })
    .await
    .map_err(|_| io::Error::other("owner request timed out"))?
}

async fn next_instance(
    options: &ServerOptions,
    endpoint: &str,
    attributes: &mut windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
) -> io::Result<NamedPipeServer> {
    // A dropped Tokio pipe can retain its native handle until a canceled I/O
    // completion is dispatched. Yield for that retirement without increasing
    // the two-instance limit or replaying any received operation.
    timeout(Duration::from_secs(1), async {
        loop {
            // SAFETY: the caller retains the descriptor and initialized attributes.
            match unsafe {
                options.create_with_security_attributes_raw(
                    endpoint,
                    (attributes as *mut windows_sys::Win32::Security::SECURITY_ATTRIBUTES).cast(),
                )
            } {
                Ok(pipe) => return Ok(pipe),
                Err(error) if error.raw_os_error() == Some(231) => {
                    sleep(Duration::from_millis(10)).await;
                }
                Err(error) => return Err(error),
            }
        }
    })
    .await
    .map_err(|_| io::Error::other("owner pipe instance retirement timed out"))?
}

pub async fn serve(path: &Path, port: u16) -> io::Result<()> {
    let directory = validate(path)?;
    let endpoint = pipe_name(&directory)?;
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))?;
    let port = listener.local_addr()?.port();
    let descriptor = security::Descriptor::owner_only(false)?;
    let mut attributes = descriptor.attributes();
    let mut options = ServerOptions::new();
    options
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .max_instances(2)
        .in_buffer_size(4096)
        .out_buffer_size(4096);
    // SAFETY: initialized descriptor/attributes live through CreateNamedPipe.
    let mut available = unsafe {
        options.create_with_security_attributes_raw(
            &endpoint,
            (&mut attributes as *mut windows_sys::Win32::Security::SECURITY_ATTRIBUTES).cast(),
        )
    }?;
    listener.set_nonblocking(true)?;
    let shared: crate::api::SharedWriter = std::sync::Arc::new(std::sync::RwLock::new(None));
    let hybrid = std::sync::Arc::new(crate::hybrid_runtime::Runtime::new());
    let viewer = std::sync::Arc::new(crate::viewer::Runtime::new());
    let mut http = tokio::spawn(crate::api::run(
        tokio::net::TcpListener::from_std(listener)?,
        shared.clone(),
        hybrid.clone(),
        viewer.clone(),
    ));
    println!(
        "{}",
        serde_json::json!({"state":"locked", "pid":std::process::id(), "port":port})
    );
    io::stdout().flush()?;
    let owner_sid = security::current_sid()?;
    let mut connection: Option<crate::writer::Writer> = None;
    let mut embedding: Option<crate::embedding::Worker> = None;
    loop {
        tokio::select! {
            result=available.connect()=>result?,
            _=&mut http=>return Err(io::Error::other("application listener stopped")),
        }
        if connection
            .as_ref()
            .is_some_and(crate::writer::Writer::is_stopped)
        {
            return Err(io::Error::other("database worker stopped"));
        }
        // Keep an instance available before replying, avoiding a disconnect/
        // reconnect gap. One request is processed and one may wait in the kernel.
        options.first_pipe_instance(false);
        let successor = next_instance(&options, &endpoint, &mut attributes).await?;
        let mut pipe = std::mem::replace(&mut available, successor);
        let mut closing = false;
        let reply = match timeout(DEADLINE, read_frame(&mut pipe, MAX_FRAME)).await {
            Ok(Ok(body)) => {
                if security::pipe_client_sid(&pipe).ok().as_deref() != Some(&owner_sid) {
                    state(false, false, Some("owner identity rejected"))
                } else {
                    match body[0] {
                        STATUS if body.len() == 1 => state(connection.is_some(), false, None),
                        UNLOCK if connection.is_none() && body.len() <= 1025 => {
                            let opened =
                                crate::schema::open(&directory.join("vault.db"), &body[1..])
                                    .and_then(|db| {
                                        let format = db
                                            .query_row("SELECT format FROM hotr_vault", [], |row| {
                                                row.get::<_, u32>(0)
                                            })
                                            .map_err(|_| crate::StoreError::DatabaseRejected)?;
                                        if format != 1 {
                                            return Err(crate::StoreError::DatabaseRejected);
                                        }
                                        Ok(db)
                                    });
                            match opened {
                                Ok(db) => {
                                    let worker = crate::writer::Writer::start(db)?;
                                    *shared
                                        .write()
                                        .map_err(|_| io::Error::other("owner state failed"))? =
                                        Some(worker.handle());
                                    embedding =
                                        Some(crate::embedding::Worker::start(worker.handle()));
                                    connection = Some(worker);
                                    state(true, false, None)
                                }
                                Err(_) => {
                                    sleep(Duration::from_millis(500)).await;
                                    state(false, false, Some("vault or passphrase rejected"))
                                }
                            }
                        }
                        LOCK if body.len() == 1 => {
                            viewer.clear().map_err(io::Error::other)?;
                            hybrid.pause().map_err(io::Error::other)?;
                            *shared
                                .write()
                                .map_err(|_| io::Error::other("owner state failed"))? = None;
                            if let Some(worker) = embedding.take() {
                                worker.stop().await;
                            }
                            if let Some(writer) = connection.take() {
                                writer.shutdown().await?;
                            }
                            closing = true;
                            state(false, true, None)
                        }
                        BACKUP if connection.is_some() => {
                            let handle = connection.as_ref().unwrap().handle();
                            let result = match crate::backup::decode(&body[1..]) {
                                Ok(request) => {
                                    handle
                                        .command(crate::capabilities::Command::Backup(request))
                                        .await
                                }
                                Err(_) => Err(crate::writer::WriteError::InvalidRequest),
                            };
                            viewer
                                .backup_result(result.as_ref().ok())
                                .map_err(io::Error::other)?;
                            match result {
                                Ok(data) => {
                                    let mut reply = state(true, false, None);
                                    reply.data = Some(data);
                                    reply
                                }
                                Err(_) => state(
                                    true,
                                    false,
                                    Some("backup rejected; any new incomplete files retained"),
                                ),
                            }
                        }
                        ADMIN if connection.is_some() => {
                            let command = crate::api::decode::<AdminRequest>(&body[1..]);
                            let handle = connection.as_ref().unwrap().handle();
                            let reconfigure =
                                matches!(&command, Ok(AdminRequest::EmbeddingConfigure(_)));
                            if reconfigure {
                                hybrid.pause().map_err(io::Error::other)?;
                            }
                            if reconfigure && let Some(worker) = embedding.take() {
                                worker.stop().await;
                            }
                            let result = match command {
                                Ok(command) => {
                                    admin_dispatch(&handle, command, port, &viewer).await
                                }
                                Err(error) => Err(error),
                            };
                            if reconfigure {
                                hybrid.resume().map_err(io::Error::other)?;
                                embedding = Some(crate::embedding::Worker::start(handle.clone()));
                            }
                            match result {
                                Ok(data) => {
                                    let mut reply = state(true, false, None);
                                    reply.data = Some(data);
                                    reply
                                }
                                Err(error) => state(true, false, Some(&error.to_string())),
                            }
                        }
                        _ => state(
                            connection.is_some(),
                            false,
                            Some("owner operation rejected"),
                        ),
                    }
                }
            }
            _ => state(connection.is_some(), false, Some("owner request rejected")),
        };
        let _ = timeout(DEADLINE, async {
            write_frame(
                &mut pipe,
                &serde_json::to_vec(&reply).map_err(io::Error::other)?,
            )
            .await?;
            // Ensure the client consumed the reply before disconnect discards buffers.
            if pipe.read_u8().await? != 1 {
                return Err(io::Error::other("owner acknowledgement rejected"));
            }
            Ok::<(), io::Error>(())
        })
        .await;
        let _ = pipe.disconnect();
        if closing {
            http.abort();
            return Ok(());
        } // main exits; the key-holding process ends.
    }
}

async fn admin_dispatch(
    handle: &crate::writer::WriterHandle,
    request: AdminRequest,
    port: u16,
    viewer: &crate::viewer::Runtime,
) -> crate::capabilities::CommandResult {
    use crate::capabilities::Command;
    match request {
        AdminRequest::ViewerSession { seconds } => viewer.approve(seconds, port),
        AdminRequest::EmbeddingConfigure(request) => {
            handle.command(Command::EmbeddingConfigure(request)).await
        }
        AdminRequest::EmbeddingStatus => handle.command(Command::EmbeddingStatus).await,
        AdminRequest::Lifecycle(request) => handle.command(Command::Lifecycle(request)).await,
        AdminRequest::Inspect(request) => handle.command(Command::Inspect(request)).await,
        AdminRequest::Import(request) => handle.command(Command::Import(request)).await,
        AdminRequest::Issue(request) => handle.command(Command::Issue { request, port }).await,
        AdminRequest::Revoke { client_id } => {
            handle.command(Command::Revoke { id: client_id }).await
        }
        AdminRequest::Clients => handle.command(Command::Clients).await,
        AdminRequest::Accept(request) => {
            let value = handle.command(Command::AcceptedInput { request }).await?;
            let request = serde_json::from_value(value)
                .map_err(|_| crate::writer::WriteError::InvalidRequest)?;
            let outcome = handle.submit("owner", request)?.wait().await?;
            serde_json::to_value(outcome)
                .map_err(|_| crate::writer::WriteError::PersistenceRejected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn native_instance_retirement_is_bounded() {
        let endpoint = format!(
            r"\\.\pipe\hotr-test-retirement-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let descriptor = security::Descriptor::owner_only(false).unwrap();
        let mut attributes = descriptor.attributes();
        let mut options = ServerOptions::new();
        options.max_instances(2).reject_remote_clients(true);
        let first = next_instance(&options, &endpoint, &mut attributes)
            .await
            .unwrap();
        let second = next_instance(&options, &endpoint, &mut attributes)
            .await
            .unwrap();
        // All native slots are held: this must time out rather than spin or grow.
        let start = std::time::Instant::now();
        assert!(
            next_instance(&options, &endpoint, &mut attributes)
                .await
                .is_err()
        );
        assert!((Duration::from_secs(1)..Duration::from_secs(3)).contains(&start.elapsed()));
        let (successor, ()) =
            tokio::join!(next_instance(&options, &endpoint, &mut attributes), async {
                sleep(Duration::from_millis(100)).await;
                drop(first);
            });
        assert!(successor.is_ok());
        drop(second);
    }
}
