//! Owner-only lifecycle mutations; each decision and receipt shares one transaction.
use crate::{
    capabilities::Role,
    schema::{self, RecordInput, State},
    writer::WriteError,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicBool, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Correct {
        record: RecordInput,
        expected_revision: u32,
    },
    Visibility {
        namespace: String,
        id: String,
        expected_revision: u32,
        tombstoned: bool,
        valid_from_ms: Option<i64>,
        expires_at_ms: Option<i64>,
    },
    Supersede {
        namespace: String,
        old_id: String,
        old_revision: u32,
        replacement_id: String,
        replacement_revision: u32,
    },
    Grants {
        client_id: String,
        expected_revision: u32,
        role: Role,
        namespaces: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub idempotency_key: String,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inspect {
    pub namespace: String,
    pub id: String,
    pub expected_revision: Option<u32>,
}

fn current(
    db: &Connection,
    namespace: &str,
    id: &str,
    expected: Option<u32>,
) -> Result<schema::Revision, WriteError> {
    if !schema::valid_identifier(namespace, true)
        || !schema::valid_identifier(id, false)
        || expected == Some(0)
    {
        return Err(WriteError::InvalidRequest);
    }
    let revision = schema::revision(db, namespace, id, None)?.ok_or(WriteError::NotFound)?;
    if expected.is_some_and(|e| e != revision.revision) {
        return Err(WriteError::RevisionConflict);
    }
    Ok(revision)
}

pub(crate) fn inspect(db: &Connection, request: Inspect) -> Result<Value, WriteError> {
    let revision = current(db, &request.namespace, &request.id, None)?;
    let policy=db.query_row("SELECT tombstoned,valid_from_ms,expires_at_ms FROM record_visibility WHERE namespace=?1 AND record_id=?2",params![request.namespace,request.id],|r|Ok(json!({"tombstoned":r.get::<_,bool>(0)?,"valid_from_ms":r.get::<_,Option<i64>>(1)?,"expires_at_ms":r.get::<_,Option<i64>>(2)?})))?;
    let relations=db.prepare("SELECT source_id,target_id,kind FROM relations WHERE namespace=?1 AND (source_id=?2 OR target_id=?2) ORDER BY kind,source_id,target_id LIMIT 50")?.query_map(params![request.namespace,request.id],|r|Ok(json!({"source_id":r.get::<_,String>(0)?,"target_id":r.get::<_,String>(1)?,"kind":r.get::<_,String>(2)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(
        json!({"conflict":request.expected_revision.is_some_and(|e|e!=revision.revision),"expected_revision":request.expected_revision,"current":revision,"policy":policy,"visible":crate::retrieval::visible(db,&request.namespace,&request.id)?,"relations":relations,"relations_limit":50}),
    )
}

pub(crate) fn execute(
    db: &mut Connection,
    request: Request,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<Value, WriteError> {
    if !schema::valid_identifier(&request.idempotency_key, false) {
        return Err(WriteError::InvalidRequest);
    }
    let request_hash =
        Sha256::digest(serde_json::to_vec(&request).map_err(|_| WriteError::InvalidRequest)?);
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let previous = tx
        .query_row(
            "SELECT request_hash,result_json FROM lifecycle_receipts WHERE idempotency_key=?1",
            [&request.idempotency_key],
            |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    if let Some((hash, result)) = previous {
        if hash.as_slice() != request_hash.as_slice() {
            return Err(WriteError::IdempotencyConflict);
        }
        return serde_json::from_str(&result).map_err(|_| WriteError::PersistenceRejected);
    }
    let now: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| WriteError::PersistenceRejected)?
        .as_millis()
        .try_into()
        .map_err(|_| WriteError::PersistenceRejected)?;
    let (operation, result) = match request.action {
        Action::Correct {
            mut record,
            expected_revision,
        } => {
            record.state = State::Accepted;
            record.validate().map_err(|_| WriteError::InvalidRequest)?;
            let previous = current(&tx, &record.namespace, &record.id, Some(expected_revision))?;
            let next = previous
                .revision
                .checked_add(1)
                .ok_or(WriteError::RevisionConflict)?;
            crate::writer::append_revision(
                &tx,
                "owner",
                &record,
                Some(previous.revision),
                next,
                now,
            )?;
            (
                "correct",
                json!({"namespace":record.namespace,"id":record.id,"revision":next}),
            )
        }
        Action::Visibility {
            namespace,
            id,
            expected_revision,
            tombstoned,
            valid_from_ms,
            expires_at_ms,
        } => {
            if valid_from_ms.is_some_and(|v| v < 0)
                || expires_at_ms.is_some_and(|v| v < 0)
                || matches!((valid_from_ms,expires_at_ms),(Some(start),Some(end)) if start>=end)
            {
                return Err(WriteError::InvalidRequest);
            }
            let previous = current(&tx, &namespace, &id, Some(expected_revision))?;
            let next = previous
                .revision
                .checked_add(1)
                .ok_or(WriteError::RevisionConflict)?;
            crate::writer::append_revision(
                &tx,
                "owner",
                &previous.record,
                Some(previous.revision),
                next,
                now,
            )?;
            tx.execute("UPDATE record_visibility SET tombstoned=?3,valid_from_ms=?4,expires_at_ms=?5 WHERE namespace=?1 AND record_id=?2",params![namespace,id,tombstoned,valid_from_ms,expires_at_ms])?;
            (
                "visibility",
                json!({"namespace":namespace,"id":id,"revision":next,"tombstoned":tombstoned,"valid_from_ms":valid_from_ms,"expires_at_ms":expires_at_ms}),
            )
        }
        Action::Supersede {
            namespace,
            old_id,
            old_revision,
            replacement_id,
            replacement_revision,
        } => {
            if old_id == replacement_id {
                return Err(WriteError::InvalidRequest);
            }
            let old = current(&tx, &namespace, &old_id, Some(old_revision))?;
            let mut replacement =
                current(&tx, &namespace, &replacement_id, Some(replacement_revision))?;
            // Both endpoints must currently be visible. This also prevents a
            // supersession cycle or a hidden replacement suppressing live data.
            if !crate::retrieval::visible(&tx, &namespace, &old_id)?
                || !crate::retrieval::visible(&tx, &namespace, &replacement_id)?
            {
                return Err(WriteError::RevisionConflict);
            }
            let next = replacement
                .revision
                .checked_add(1)
                .ok_or(WriteError::RevisionConflict)?;
            let old_next = old
                .revision
                .checked_add(1)
                .ok_or(WriteError::RevisionConflict)?;
            replacement.record.state = State::Accepted;
            crate::writer::append_revision(
                &tx,
                "owner",
                &replacement.record,
                Some(replacement.revision),
                next,
                now,
            )?;
            crate::writer::append_revision(
                &tx,
                "owner",
                &old.record,
                Some(old.revision),
                old_next,
                now,
            )?;
            tx.execute(
                "INSERT INTO relations VALUES(?1,?2,?3,'supersedes')",
                params![namespace, replacement_id, old_id],
            )?;
            (
                "supersede",
                json!({"namespace":namespace,"old_id":old_id,"old_revision":old_next,"replacement_id":replacement_id,"replacement_revision":next}),
            )
        }
        Action::Grants {
            client_id,
            expected_revision,
            role,
            namespaces,
        } => {
            if !schema::valid_identifier(&client_id, false)
                || namespaces.len() > 32
                || namespaces
                    .iter()
                    .any(|n| !schema::valid_identifier(n, true))
                || namespaces.iter().collect::<HashSet<_>>().len() != namespaces.len()
            {
                return Err(WriteError::InvalidRequest);
            }
            let (revision, revoked) = tx
                .query_row(
                    "SELECT grant_revision,revoked FROM clients WHERE id=?1",
                    [&client_id],
                    |r| Ok((r.get::<_, u32>(0)?, r.get::<_, bool>(1)?)),
                )
                .optional()?
                .ok_or(WriteError::NotFound)?;
            if revoked {
                return Err(WriteError::Unauthorized);
            }
            if revision != expected_revision {
                return Err(WriteError::RevisionConflict);
            }
            let next = revision
                .checked_add(1)
                .ok_or(WriteError::RevisionConflict)?;
            tx.execute("DELETE FROM client_grants WHERE client_id=?1", [&client_id])?;
            for namespace in &namespaces {
                tx.execute(
                    "INSERT INTO client_grants VALUES(?1,?2)",
                    params![client_id, namespace],
                )?;
            }
            tx.execute(
                "UPDATE clients SET role=?2,grant_revision=?3 WHERE id=?1",
                params![
                    client_id,
                    match role {
                        Role::Reader => "reader",
                        Role::Contributor => "contributor",
                    },
                    next
                ],
            )?;
            (
                "grants",
                json!({"client_id":client_id,"grant_revision":next,"role":role,"namespaces":namespaces}),
            )
        }
    };
    let result = json!({"outcome":"committed","operation":operation,"receipt":result});
    tx.execute(
        "INSERT INTO lifecycle_receipts VALUES(?1,?2,?3,?4,?5)",
        params![
            request.idempotency_key,
            request_hash.as_slice(),
            operation,
            serde_json::to_string(&result).map_err(|_| WriteError::PersistenceRejected)?,
            now
        ],
    )?;
    if stopped.load(Ordering::SeqCst) || Instant::now() >= deadline {
        return Err(WriteError::Stopped);
    }
    tx.commit().map_err(|_| WriteError::OutcomeUnknown)?;
    Ok(result)
}
