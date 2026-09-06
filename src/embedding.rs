//! One durable, bounded index worker; inference never holds the database queue.
use crate::{
    capabilities::Command,
    writer::{WriteError, WriterHandle},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const MAX_ATTEMPTS: u32 = 3;
const LEASE_MS: i64 = 65_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configure {
    /// None disables inference. Only numeric IPv4 loopback is used.
    pub port: Option<u16>,
    pub expected_generation: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Job {
    namespace: String,
    id: String,
    revision: u32,
    generation: u32,
    attempt: u32,
    port: u16,
    body: String,
}

pub(crate) struct Completion {
    job: Job,
    result: Result<crate::embedding_transport::Embedding, crate::embedding_transport::Error>,
}

fn now() -> Result<i64, WriteError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| WriteError::PersistenceRejected)?
        .as_millis()
        .try_into()
        .map_err(|_| WriteError::PersistenceRejected)
}

fn commit(
    tx: rusqlite::Transaction<'_>,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<(), WriteError> {
    if Instant::now() >= deadline || stopped.load(Ordering::SeqCst) {
        return Err(WriteError::Stopped);
    }
    tx.commit().map_err(|_| WriteError::OutcomeUnknown)
}

pub(crate) fn configure(
    db: &mut Connection,
    request: Configure,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<Value, WriteError> {
    if request.port == Some(0) {
        return Err(WriteError::InvalidRequest);
    }
    let generation = request
        .expected_generation
        .checked_add(1)
        .ok_or(WriteError::InvalidRequest)?;
    let tx = db.transaction()?;
    if tx.execute("UPDATE embedding_config SET port=?1,generation=?2,model_digest=?4 WHERE singleton=1 AND generation=?3", params![request.port,generation,request.expected_generation,crate::embedding_transport::MODEL_DIGEST])? != 1 {
        return Err(WriteError::RevisionConflict);
    }
    commit(tx, deadline, stopped)?;
    status(db)
}

pub(crate) fn status(db: &Connection) -> Result<Value, WriteError> {
    let (generation, port): (u32, Option<u16>) = db.query_row(
        "SELECT generation,port FROM embedding_config WHERE singleton=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let visible: i64 = db.query_row("SELECT count(*) FROM visible_records", [], |r| r.get(0))?;
    let indexed: i64 = db.query_row(
        "SELECT count(*) FROM current_embeddings WHERE model_digest=?1",
        [crate::embedding_transport::MODEL_DIGEST],
        |r| r.get(0),
    )?;
    let failed:i64=db.query_row("SELECT count(*) FROM embedding_index e JOIN visible_records r ON r.namespace=e.namespace AND r.id=e.record_id AND r.current_revision=e.revision WHERE e.generation=?1 AND e.vector IS NULL AND e.attempts=3 AND (e.last_error IS NOT NULL OR e.due_ms<=?2)",params![generation,now()?],|r|r.get(0))?;
    let last:Option<(Option<String>,Option<String>)>=db.query_row("SELECT last_error,peer FROM embedding_index WHERE generation=?1 ORDER BY completed_at_ms DESC LIMIT 1",[generation],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let (last_error, last_peer) = last.unwrap_or_default();
    Ok(
        json!({"enabled":port.is_some(),"port":port,"generation":generation,"model":crate::embedding_transport::MODEL,"model_digest":crate::embedding_transport::MODEL_DIGEST,"dimensions":crate::embedding_transport::DIMENSIONS,"visible":visible,"indexed":indexed,"pending":visible.saturating_sub(indexed).max(0),"failed":failed,"max_attempts":MAX_ATTEMPTS,"last_error":last_error,"last_peer":last_peer}),
    )
}

pub(crate) fn next(
    db: &mut Connection,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<Value, WriteError> {
    let time = now()?;
    let tx = db.transaction()?;
    let job=tx.query_row("SELECT r.namespace,r.id,r.current_revision,c.generation,c.port,v.body,CASE WHEN e.revision=r.current_revision AND e.generation=c.generation AND e.model_digest=c.model_digest THEN e.attempts+1 ELSE 1 END FROM visible_records r JOIN revisions v ON v.namespace=r.namespace AND v.record_id=r.id AND v.revision=r.current_revision JOIN embedding_config c ON c.singleton=1 AND c.port IS NOT NULL AND c.model_digest=?2 LEFT JOIN embedding_index e ON e.namespace=r.namespace AND e.record_id=r.id WHERE e.record_id IS NULL OR e.revision!=r.current_revision OR e.generation!=c.generation OR e.model_digest!=c.model_digest OR (e.vector IS NULL AND e.attempts<3 AND e.due_ms<=?1) ORDER BY r.namespace,r.id LIMIT 1",params![time,crate::embedding_transport::MODEL_DIGEST],|r|Ok(Job {namespace:r.get(0)?,id:r.get(1)?,revision:r.get(2)?,generation:r.get(3)?,port:r.get(4)?,body:r.get(5)?,attempt:r.get(6)?})).optional()?;
    let Some(job) = job else {
        return Ok(Value::Null);
    };
    tx.execute("INSERT INTO embedding_index(namespace,record_id,revision,generation,attempts,due_ms,model_digest) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(namespace,record_id) DO UPDATE SET revision=excluded.revision,generation=excluded.generation,model_digest=excluded.model_digest,attempts=excluded.attempts,due_ms=excluded.due_ms,vector=NULL,last_error=NULL,peer=NULL,completed_at_ms=NULL",params![job.namespace,job.id,job.revision,job.generation,job.attempt,time+LEASE_MS,crate::embedding_transport::MODEL_DIGEST])?;
    let result = serde_json::to_value(job).map_err(|_| WriteError::PersistenceRejected)?;
    commit(tx, deadline, stopped)?;
    Ok(result)
}

pub(crate) fn complete(
    db: &mut Connection,
    completion: Completion,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<Value, WriteError> {
    let job = completion.job;
    let tx = db.transaction()?;
    let current:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM visible_records r JOIN embedding_config c ON c.singleton=1 WHERE r.namespace=?1 AND r.id=?2 AND r.current_revision=?3 AND c.generation=?4 AND c.port=?5 AND c.model_digest=?6)",params![job.namespace,job.id,job.revision,job.generation,job.port,crate::embedding_transport::MODEL_DIGEST],|r|r.get(0))?;
    if !current {
        return Ok(json!({"stored":false,"reason":"stale"}));
    }
    let time = now()?;
    let (vector, error, peer) = match completion.result {
        Ok(value) => {
            let norm = value
                .vector
                .iter()
                .map(|&x| f64::from(x).powi(2))
                .sum::<f64>();
            if value.vector.len() != crate::embedding_transport::DIMENSIONS
                || value.vector.iter().any(|v| !v.is_finite())
                || !(0.999..=1.001).contains(&norm)
                || value.peer != format!("127.0.0.1:{}", job.port)
            {
                return Err(WriteError::InvalidRequest);
            }
            (
                Some(
                    value
                        .vector
                        .iter()
                        .flat_map(|f| f.to_le_bytes())
                        .collect::<Vec<_>>(),
                ),
                None,
                Some(value.peer),
            )
        }
        Err(error) => (None, Some(error.code()), None),
    };
    let changed=tx.execute("UPDATE embedding_index SET vector=?1,last_error=?2,peer=?3,completed_at_ms=?4,due_ms=?5 WHERE namespace=?6 AND record_id=?7 AND revision=?8 AND generation=?9 AND attempts=?10 AND model_digest=?11 AND vector IS NULL",params![vector,error,peer,time,time+if job.attempt==1 {1_000}else{5_000},job.namespace,job.id,job.revision,job.generation,job.attempt,crate::embedding_transport::MODEL_DIGEST])?;
    commit(tx, deadline, stopped)?;
    Ok(json!({"stored":changed==1 && vector.is_some()}))
}

pub(crate) struct Worker(tokio::task::JoinHandle<()>);
impl Worker {
    pub(crate) fn start(handle: WriterHandle) -> Self {
        Self(tokio::spawn(async move {
            loop {
                match handle.command(Command::EmbeddingNext).await {
                    Ok(value) if !value.is_null() => {
                        let Ok(job) = serde_json::from_value::<Job>(value) else {
                            return;
                        };
                        let result =
                            crate::embedding_transport::embed(job.port, &job.body, false).await;
                        if matches!(
                            handle
                                .command(Command::EmbeddingComplete(Completion { job, result }))
                                .await,
                            Err(WriteError::Stopped)
                        ) {
                            return;
                        }
                    }
                    Err(WriteError::Stopped) => return,
                    _ => tokio::time::sleep(Duration::from_millis(250)).await,
                }
            }
        }))
    }
    pub(crate) async fn stop(self) {
        self.0.abort();
        // Await through a borrow: Drop is retained as an error-path safety net.
        let mut this = self;
        let _ = (&mut this.0).await;
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
#[path = "embedding_tests.rs"]
mod tests;
