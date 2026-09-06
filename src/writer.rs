//! One bounded database worker. Principals are supplied by trusted service code;
//! application authentication is added at the HOTR-07 boundary, not in JSON.
use crate::schema::{RecordInput, valid_identifier};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::oneshot;

pub const QUEUE_CAPACITY: usize = 256;
pub const REQUEST_DEADLINE: Duration = Duration::from_secs(10);
const PENDING: u8 = 0;
const COMMITTING: u8 = 1;
const RESOLVED: u8 = 2;
const CANCELED: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteRequest {
    pub record: RecordInput,
    /// None creates a new ID; Some(n) revises only current revision n.
    pub expected_revision: Option<u32>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub namespace: String,
    pub id: String,
    pub revision: u32,
    pub committed_at_ms: i64,
    pub audit_sequence: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", content = "receipt", rename_all = "snake_case")]
pub enum WriteOutcome {
    Committed(Receipt),
    Canceled,
    UnknownToClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteError {
    InvalidRequest,
    RevisionConflict,
    IdempotencyConflict,
    Overloaded,
    Stopped,
    PersistenceRejected,
    Unauthorized,
    Forbidden,
    NotFound,
    OutcomeUnknown,
}
impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidRequest => "write request rejected",
            Self::RevisionConflict => "expected revision does not match",
            Self::IdempotencyConflict => "idempotency key was used for a different request",
            Self::Overloaded => "writer queue is full",
            Self::Stopped => "writer is stopped",
            Self::PersistenceRejected => "write persistence rejected",
            Self::Unauthorized => "application credential rejected",
            Self::Forbidden => "operation is outside application grants",
            Self::NotFound => "record not found",
            Self::OutcomeUnknown => "operation outcome unknown; reconcile before retry",
        })
    }
}
impl std::error::Error for WriteError {}
impl From<rusqlite::Error> for WriteError {
    fn from(_: rusqlite::Error) -> Self {
        Self::PersistenceRejected
    }
}
type Result = std::result::Result<WriteOutcome, WriteError>;

struct Job {
    principal: String,
    credential_hash: Option<[u8; 32]>,
    request: WriteRequest,
    phase: Arc<AtomicU8>,
    deadline: Instant,
    reply: oneshot::Sender<Result>,
}

enum Message {
    Write(Job),
    Command {
        command: crate::capabilities::Command,
        reply: oneshot::Sender<crate::capabilities::CommandResult>,
        deadline: Instant,
    },
}

#[derive(Clone)]
pub struct WriterHandle {
    sender: mpsc::SyncSender<Message>,
    stopped: Arc<AtomicBool>,
}

pub struct PendingWrite {
    reply: oneshot::Receiver<Result>,
    phase: Arc<AtomicU8>,
    deadline: Instant,
}

impl PendingWrite {
    fn cancel_before_commit(&self) -> bool {
        self.phase
            .compare_exchange(PENDING, CANCELED, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
            || self.phase.load(Ordering::SeqCst) == CANCELED
    }

    /// Cancel this attempt. This does not undo an earlier committed retry.
    pub fn cancel(mut self) -> Result {
        if let Ok(result) = self.reply.try_recv() {
            return result;
        }
        Ok(if self.cancel_before_commit() {
            WriteOutcome::Canceled
        } else {
            WriteOutcome::UnknownToClient
        })
    }

    pub async fn wait(mut self) -> Result {
        match tokio::time::timeout_at(self.deadline.into(), &mut self.reply).await {
            Ok(Ok(result)) => result,
            _ => self.cancel(),
        }
    }
}

impl Drop for PendingWrite {
    fn drop(&mut self) {
        self.cancel_before_commit();
    }
}

impl WriterHandle {
    pub(crate) fn submit(
        &self,
        principal: &str,
        request: WriteRequest,
    ) -> std::result::Result<PendingWrite, WriteError> {
        if !valid_identifier(principal, false) {
            return Err(WriteError::InvalidRequest);
        }
        self.submit_inner(principal.to_owned(), None, request)
    }

    pub fn submit_authenticated(
        &self,
        credential_hash: [u8; 32],
        request: WriteRequest,
    ) -> std::result::Result<PendingWrite, WriteError> {
        self.submit_inner(String::new(), Some(credential_hash), request)
    }

    fn submit_inner(
        &self,
        principal: String,
        credential_hash: Option<[u8; 32]>,
        request: WriteRequest,
    ) -> std::result::Result<PendingWrite, WriteError> {
        if !valid_identifier(&request.idempotency_key, false)
            || request.expected_revision == Some(0)
            || request.record.validate().is_err()
        {
            return Err(WriteError::InvalidRequest);
        }
        if self.stopped.load(Ordering::SeqCst) {
            return Err(WriteError::Stopped);
        }
        let deadline = Instant::now() + REQUEST_DEADLINE;
        let phase = Arc::new(AtomicU8::new(PENDING));
        let (send, reply) = oneshot::channel();
        self.sender
            .try_send(Message::Write(Job {
                principal,
                credential_hash,
                request,
                phase: phase.clone(),
                deadline,
                reply: send,
            }))
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => WriteError::Overloaded,
                mpsc::TrySendError::Disconnected(_) => WriteError::Stopped,
            })?;
        Ok(PendingWrite {
            reply,
            phase,
            deadline,
        })
    }

    pub(crate) async fn command(
        &self,
        command: crate::capabilities::Command,
    ) -> crate::capabilities::CommandResult {
        if self.stopped.load(Ordering::SeqCst) {
            return Err(WriteError::Stopped);
        }
        let (send, receive) = oneshot::channel();
        self.sender
            .try_send(Message::Command {
                command,
                reply: send,
                deadline: Instant::now() + REQUEST_DEADLINE,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => WriteError::Overloaded,
                mpsc::TrySendError::Disconnected(_) => WriteError::Stopped,
            })?;
        tokio::time::timeout(REQUEST_DEADLINE, receive)
            .await
            .map_err(|_| WriteError::OutcomeUnknown)?
            .map_err(|_| WriteError::OutcomeUnknown)?
    }
}

pub struct Writer {
    handle: WriterHandle,
    thread: Option<JoinHandle<()>>,
}

// Instrumentation is absent from the production executable. Tests use actual
// encrypted transactions and owned child processes at these boundaries.
#[cfg(test)]
#[derive(Clone, Default)]
struct Hooks {
    before_commit: Option<Arc<dyn Fn() + Send + Sync>>,
    after_commit: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Writer {
    pub fn start(connection: Connection) -> io::Result<Self> {
        Self::start_inner(
            connection,
            #[cfg(test)]
            Hooks::default(),
        )
    }

    fn start_inner(mut connection: Connection, #[cfg(test)] hooks: Hooks) -> io::Result<Self> {
        let (sender, receive) = mpsc::sync_channel::<Message>(QUEUE_CAPACITY);
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = stopped.clone();
        let thread = thread::Builder::new()
            .name("hotr-database".into())
            .spawn(move || {
                while !flag.load(Ordering::SeqCst) {
                    let message = match receive.recv_timeout(Duration::from_millis(100)) {
                        Ok(message) => message,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    let job = match message {
                        Message::Write(job) => job,
                        Message::Command {
                            command,
                            reply,
                            deadline,
                        } => {
                            if reply.is_closed()
                                || Instant::now() >= deadline
                                || flag.load(Ordering::SeqCst)
                            {
                                let _ = reply.send(Err(WriteError::Stopped));
                            } else {
                                let result = (|| {
                                    let stop = flag.clone();
                                    connection.progress_handler(
                                        1000,
                                        Some(move || {
                                            Instant::now() >= deadline
                                                || stop.load(Ordering::SeqCst)
                                        }),
                                    )?;
                                    let result = crate::capabilities::execute(
                                        &mut connection,
                                        command,
                                        deadline,
                                        &flag,
                                    );
                                    connection.progress_handler(0, None::<fn() -> bool>)?;
                                    result
                                })();
                                let _ = reply.send(result);
                            }
                            if !connection.is_autocommit() {
                                break;
                            }
                            continue;
                        }
                    };
                    if flag.load(Ordering::SeqCst) || Instant::now() >= job.deadline {
                        let _ = job.phase.compare_exchange(
                            PENDING,
                            CANCELED,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        );
                    }
                    let result = apply(
                        &mut connection,
                        &job,
                        &flag,
                        #[cfg(test)]
                        &hooks,
                    );
                    let _ = job.phase.compare_exchange(
                        PENDING,
                        RESOLVED,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    );
                    let _ = job.reply.send(result);
                    if !connection.is_autocommit() {
                        break;
                    }
                }
                flag.store(true, Ordering::SeqCst);
                for message in receive.try_iter() {
                    match message {
                        Message::Write(job) => {
                            job.phase.store(CANCELED, Ordering::SeqCst);
                            let _ = job.reply.send(Ok(WriteOutcome::Canceled));
                        }
                        Message::Command { reply, .. } => {
                            let _ = reply.send(Err(WriteError::Stopped));
                        }
                    }
                }
                drop(connection);
            })?;
        Ok(Self {
            handle: WriterHandle { sender, stopped },
            thread: Some(thread),
        })
    }

    pub fn handle(&self) -> WriterHandle {
        self.handle.clone()
    }
    pub fn is_stopped(&self) -> bool {
        self.handle.stopped.load(Ordering::SeqCst)
            || self.thread.as_ref().is_some_and(JoinHandle::is_finished)
    }

    /// Owner lock waits for the database connection's thread to end before reply.
    pub async fn shutdown(mut self) -> io::Result<()> {
        self.handle.stopped.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            tokio::task::spawn_blocking(move || thread.join())
                .await
                .map_err(|_| io::Error::other("writer shutdown failed"))?
                .map_err(|_| io::Error::other("writer failed"))?;
        }
        Ok(())
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.handle.stopped.store(true, Ordering::SeqCst);
    }
}

fn apply(
    connection: &mut Connection,
    job: &Job,
    stopped: &AtomicBool,
    #[cfg(test)] hooks: &Hooks,
) -> Result {
    if job.phase.load(Ordering::SeqCst) == CANCELED {
        return Ok(WriteOutcome::Canceled);
    }
    let request = &job.request;
    let principal = if let Some(hash) = job.credential_hash {
        crate::capabilities::authorize_write(connection, &hash, &request.record)?
    } else {
        job.principal.clone()
    };
    let hash = Sha256::digest(serde_json::to_vec(request).map_err(|_| WriteError::InvalidRequest)?);
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let previous = tx.query_row(
        "SELECT request_hash,namespace,record_id,revision,committed_at_ms,audit_sequence FROM write_receipts WHERE principal=?1 AND idempotency_key=?2",
        params![principal, request.idempotency_key], |row| Ok((row.get::<_,Vec<u8>>(0)?, Receipt {
            namespace:row.get(1)?, id:row.get(2)?, revision:row.get(3)?, committed_at_ms:row.get(4)?, audit_sequence:row.get(5)?,
        }))
    ).optional()?;
    if let Some((previous_hash, receipt)) = previous {
        return if previous_hash.as_slice() == hash.as_slice() {
            Ok(WriteOutcome::Committed(receipt))
        } else {
            Err(WriteError::IdempotencyConflict)
        };
    }
    let record = &request.record;
    if job.credential_hash.is_some() {
        crate::capabilities::ensure_mutable(&tx, record)?;
    }
    let current: Option<u32> = tx
        .query_row(
            "SELECT current_revision FROM records WHERE namespace=?1 AND id=?2",
            params![record.namespace, record.id],
            |row| row.get(0),
        )
        .optional()?;
    if current != request.expected_revision {
        return Err(WriteError::RevisionConflict);
    }
    let revision = current
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(WriteError::RevisionConflict)?;
    let now: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| WriteError::PersistenceRejected)?
        .as_millis()
        .try_into()
        .map_err(|_| WriteError::PersistenceRejected)?;
    let audit_sequence = append_revision(&tx, &principal, record, current, revision, now)?;
    tx.execute(
        "INSERT INTO write_receipts VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            principal,
            request.idempotency_key,
            hash.as_slice(),
            record.namespace,
            record.id,
            revision,
            now,
            audit_sequence
        ],
    )?;
    #[cfg(test)]
    if let Some(hook) = &hooks.before_commit {
        hook();
    }
    if stopped.load(Ordering::SeqCst) || Instant::now() >= job.deadline {
        let _ = job
            .phase
            .compare_exchange(PENDING, CANCELED, Ordering::SeqCst, Ordering::SeqCst);
    }
    if job
        .phase
        .compare_exchange(PENDING, COMMITTING, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        tx.rollback()?;
        return Ok(WriteOutcome::Canceled);
    }
    // Once COMMIT is attempted a failed/absent reply cannot establish rollback.
    // Replaying this same request after recovery consults its durable receipt.
    if tx.commit().is_err() {
        return Ok(WriteOutcome::UnknownToClient);
    }
    #[cfg(test)]
    if let Some(hook) = &hooks.after_commit {
        hook();
    }
    job.phase.store(RESOLVED, Ordering::SeqCst);
    Ok(WriteOutcome::Committed(Receipt {
        namespace: record.namespace.clone(),
        id: record.id.clone(),
        revision,
        committed_at_ms: now,
        audit_sequence,
    }))
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod tests;

/// Storage seam shared by individual writes and atomic owner imports.
pub(crate) fn append_revision(
    tx: &rusqlite::Transaction<'_>,
    principal: &str,
    record: &RecordInput,
    current: Option<u32>,
    revision: u32,
    now: i64,
) -> std::result::Result<i64, WriteError> {
    if current.is_none() {
        tx.execute(
            "INSERT OR IGNORE INTO namespaces VALUES(?1)",
            [&record.namespace],
        )?;
        tx.execute(
            "INSERT INTO records VALUES(?1,?2,1)",
            params![record.namespace, record.id],
        )?;
    }
    let kind = serde_json::to_value(record.kind).map_err(|_| WriteError::InvalidRequest)?;
    let state = serde_json::to_value(record.state).map_err(|_| WriteError::InvalidRequest)?;
    tx.execute(
        "INSERT INTO revisions VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            record.namespace,
            record.id,
            revision,
            kind.as_str().ok_or(WriteError::InvalidRequest)?,
            record.body,
            state.as_str().ok_or(WriteError::InvalidRequest)?,
            now
        ],
    )?;
    for (ordinal, source) in record.sources.iter().enumerate() {
        tx.execute(
            "INSERT INTO revision_sources VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                record.namespace,
                record.id,
                revision,
                ordinal as u32,
                source.reference,
                source.label
            ],
        )?;
    }
    for (ordinal, tag) in record.tags.iter().enumerate() {
        tx.execute(
            "INSERT INTO revision_tags VALUES(?1,?2,?3,?4,?5)",
            params![record.namespace, record.id, revision, ordinal as u32, tag],
        )?;
    }
    if current.is_some() {
        tx.execute(
            "UPDATE records SET current_revision=?3 WHERE namespace=?1 AND id=?2",
            params![record.namespace, record.id, revision],
        )?;
    }
    crate::retrieval::reindex(tx, record)?;
    tx.execute("INSERT INTO mutation_audit(principal,namespace,record_id,revision,operation,committed_at_ms) VALUES(?1,?2,?3,?4,?5,?6)",params![principal,record.namespace,record.id,revision,if current.is_some(){"revise"}else{"create"},now])?;
    let audit_sequence = tx.last_insert_rowid();
    Ok(audit_sequence)
}
