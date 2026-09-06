//! Query vectors only: no cached records or authorization decisions.
use crate::{
    capabilities::Command,
    embedding_transport,
    retrieval::Search,
    writer::{WriteError, WriterHandle},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

const CACHE_ENTRIES: usize = 256;
const CACHE_TTL: Duration = Duration::from_secs(300);
const QUERY_DEADLINE: Duration = Duration::from_millis(1500);

// Issued internally on the database queue after current authorization. Never
// accepted from an HTTP or MCP request and never included in a result.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub(crate) struct Ticket {
    pub client_id: String,
    pub grant_revision: u32,
    pub generation: u32,
    pub port: Option<u16>,
    pub digest: String,
}

#[derive(Clone, Eq, PartialEq, Hash)]
struct CacheKey {
    token_hash: [u8; 32],
    ticket: Ticket,
    namespace: String,
    query_hash: [u8; 32],
}
struct Entry {
    created: Instant,
    vector: Zeroizing<Vec<f32>>,
}
struct State {
    paused: bool,
    epoch: CancellationToken,
    entries: HashMap<CacheKey, Entry>,
}
pub(crate) struct Runtime {
    state: Mutex<State>,
    inference: Semaphore,
}

impl Runtime {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(State {
                paused: false,
                epoch: CancellationToken::new(),
                entries: HashMap::new(),
            }),
            inference: Semaphore::new(1),
        }
    }

    // Owner calls this before changing configuration or locking. Canceling
    // drops the adapter future and its owned HTTP connection driver.
    pub(crate) fn pause(&self) -> Result<(), WriteError> {
        let mut state = self.state.lock().map_err(|_| WriteError::Stopped)?;
        state.paused = true;
        state.epoch.cancel();
        state.entries.clear();
        Ok(())
    }

    pub(crate) fn resume(&self) -> Result<(), WriteError> {
        let mut state = self.state.lock().map_err(|_| WriteError::Stopped)?;
        state.epoch = CancellationToken::new();
        state.paused = false;
        Ok(())
    }

    pub(crate) async fn search(
        &self,
        writer: &WriterHandle,
        hash: [u8; 32],
        query: Search,
    ) -> Result<Value, WriteError> {
        // Capture the epoch before preflight so a concurrent owner change
        // cannot make a stale ticket usable with the new cancellation epoch.
        let epoch = {
            let state = self.state.lock().map_err(|_| WriteError::Stopped)?;
            if state.paused {
                None
            } else {
                Some(state.epoch.clone())
            }
        };
        let ticket: Ticket = serde_json::from_value(
            writer
                .command(Command::HybridPrepare {
                    hash,
                    query: query.clone(),
                })
                .await?,
        )
        .map_err(|_| WriteError::PersistenceRejected)?;
        let key = CacheKey {
            token_hash: hash,
            ticket: ticket.clone(),
            namespace: query.page.namespace.clone(),
            query_hash: Sha256::digest(query.query.as_bytes()).into(),
        };
        let mut status = if ticket.port.is_some() {
            "model_unavailable"
        } else {
            "disabled"
        };
        let mut vector = None;
        if let Some(port) = ticket
            .port
            .filter(|_| ticket.digest == embedding_transport::MODEL_DIGEST)
        {
            status = "embedding_changed";
            if let Some(epoch) = epoch.filter(|e| !e.is_cancelled()) {
                {
                    let mut state = self.state.lock().map_err(|_| WriteError::Stopped)?;
                    state
                        .entries
                        .retain(|_, entry| entry.created.elapsed() < CACHE_TTL);
                    if let Some(entry) = state.entries.get(&key) {
                        vector = Some(entry.vector.clone());
                        status = "ready";
                    }
                }
                if vector.is_none() {
                    status = "embedding_busy";
                    if let Ok(_permit) = self.inference.try_acquire() {
                        let outcome = tokio::select! {
                            biased;
                            _ = epoch.cancelled() => None,
                            result = tokio::time::timeout(QUERY_DEADLINE, embedding_transport::embed(port, &query.query, true)) => Some(result),
                        };
                        match outcome {
                            Some(Ok(Ok(embedding))) => {
                                let embedding = Zeroizing::new(embedding.vector);
                                let mut state =
                                    self.state.lock().map_err(|_| WriteError::Stopped)?;
                                if !epoch.is_cancelled() && !state.paused {
                                    if state.entries.len() >= CACHE_ENTRIES
                                        && let Some(oldest) = state
                                            .entries
                                            .iter()
                                            .min_by_key(|(_, entry)| entry.created)
                                            .map(|(key, _)| key.clone())
                                    {
                                        state.entries.remove(&oldest);
                                    }
                                    state.entries.insert(
                                        key,
                                        Entry {
                                            created: Instant::now(),
                                            vector: embedding.clone(),
                                        },
                                    );
                                    vector = Some(embedding);
                                    status = "ready";
                                } else {
                                    status = "embedding_changed";
                                }
                            }
                            Some(Ok(Err(_))) => status = "model_unavailable",
                            Some(Err(_)) => status = "embedding_timeout",
                            None => status = "embedding_changed",
                        }
                    }
                }
                if epoch.is_cancelled() {
                    vector = None;
                    status = "embedding_changed";
                }
            }
        }
        // This second command rechecks identity, grants, configuration and
        // current revisions; a cache hit never bypasses any of those checks.
        writer
            .command(Command::HybridSearch {
                hash,
                query,
                ticket,
                vector,
                status,
            })
            .await
    }
}
