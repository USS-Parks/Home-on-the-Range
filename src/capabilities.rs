//! Capability decisions execute on the same queue as writes and revocation.
use crate::{
    credentials,
    schema::{self, RecordInput, State},
    writer::WriteError,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Reader,
    Contributor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewClient {
    pub label: String,
    pub role: Role,
    pub namespaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lookup {
    pub namespace: String,
    pub id: String,
    #[serde(default)]
    pub revision: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accept {
    pub namespace: String,
    pub id: String,
    pub expected_revision: u32,
    pub idempotency_key: String,
}

pub(crate) enum Command {
    Lifecycle(crate::lifecycle::Request),
    Inspect(crate::lifecycle::Inspect),
    Import(crate::imports::Request),
    Backup(crate::backup::Request),
    #[cfg(test)]
    DeadlineProbe,
    Issue {
        request: NewClient,
        port: u16,
    },
    Revoke {
        id: String,
    },
    Clients,
    Get {
        hash: [u8; 32],
        query: Lookup,
    },
    Status {
        hash: [u8; 32],
    },
    Search {
        hash: [u8; 32],
        query: crate::retrieval::Search,
    },
    List {
        hash: [u8; 32],
        query: crate::retrieval::Page,
    },
    Count {
        hash: [u8; 32],
        query: crate::retrieval::Count,
    },
    History {
        hash: [u8; 32],
        query: crate::retrieval::History,
    },
    AcceptedInput {
        request: Accept,
    },
}
pub(crate) type CommandResult = Result<Value, WriteError>;

fn identity(db: &Connection, hash: &[u8; 32]) -> Result<(String, Role), WriteError> {
    let result = db
        .query_row(
            "SELECT id,role FROM clients WHERE token_hash=?1 AND revoked=0",
            [hash.as_slice()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((id, role)) = result else {
        return Err(WriteError::Unauthorized);
    };
    let role = match role.as_str() {
        "reader" => Role::Reader,
        "contributor" => Role::Contributor,
        _ => return Err(WriteError::Unauthorized),
    };
    Ok((id, role))
}

fn grant(db: &Connection, id: &str, namespace: &str) -> Result<(), WriteError> {
    if !schema::valid_identifier(namespace, true) {
        return Err(WriteError::InvalidRequest);
    }
    let allowed: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM client_grants WHERE client_id=?1 AND namespace=?2)",
        params![id, namespace],
        |row| row.get(0),
    )?;
    if allowed {
        Ok(())
    } else {
        Err(WriteError::Forbidden)
    }
}

pub(crate) fn authorize_write(
    db: &Connection,
    hash: &[u8; 32],
    record: &RecordInput,
) -> Result<String, WriteError> {
    let (id, role) = identity(db, hash)?;
    grant(db, &id, &record.namespace)?;
    if role != Role::Contributor || record.state == State::Accepted {
        return Err(WriteError::Forbidden);
    }
    Ok(id)
}

pub(crate) fn ensure_mutable(db: &Connection, record: &RecordInput) -> Result<(), WriteError> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM records WHERE namespace=?1 AND id=?2)",
        params![record.namespace, record.id],
        |r| r.get(0),
    )?;
    if exists && !crate::retrieval::visible(db, &record.namespace, &record.id)? {
        return Err(WriteError::Forbidden);
    }
    let accepted:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM revisions v JOIN records r ON r.namespace=v.namespace AND r.id=v.record_id AND r.current_revision=v.revision WHERE r.namespace=?1 AND r.id=?2 AND v.state='accepted')",params![record.namespace,record.id],|row|row.get(0))?;
    if accepted {
        Err(WriteError::Forbidden)
    } else {
        Ok(())
    }
}

pub(crate) fn execute(
    db: &mut Connection,
    command: Command,
    deadline: std::time::Instant,
    stopped: &std::sync::atomic::AtomicBool,
) -> CommandResult {
    match command {
        Command::Lifecycle(request) => crate::lifecycle::execute(db, request, deadline, stopped),
        Command::Inspect(request) => crate::lifecycle::inspect(db, request),
        Command::Import(request) => crate::imports::execute(db, request, deadline, stopped),
        Command::Backup(request) => serde_json::to_value(
            crate::backup::create(db, request).map_err(|_| WriteError::PersistenceRejected)?,
        )
        .map_err(|_| WriteError::PersistenceRejected),
        #[cfg(test)]
        Command::DeadlineProbe => {
            let value:i64=db.query_row("WITH RECURSIVE span(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM span WHERE x<1000000000) SELECT sum(x) FROM span",[],|r|r.get(0))?;
            Ok(json!(value))
        }
        Command::Search { hash, query } => {
            let (id, _) = identity(db, &hash)?;
            grant(db, &id, &query.page.namespace)?;
            crate::retrieval::search(db, query)
        }
        Command::List { hash, query } => {
            let (id, _) = identity(db, &hash)?;
            grant(db, &id, &query.namespace)?;
            crate::retrieval::list(db, query)
        }
        Command::Count { hash, query } => {
            let (id, _) = identity(db, &hash)?;
            grant(db, &id, &query.namespace)?;
            crate::retrieval::count(db, query)
        }
        Command::History { hash, query } => {
            let (id, _) = identity(db, &hash)?;
            grant(db, &id, &query.page.namespace)?;
            crate::retrieval::history(db, query)
        }
        Command::Issue { request, port } => issue(db, request, port),
        Command::Revoke { id } => {
            if !schema::valid_identifier(&id, false) {
                return Err(WriteError::InvalidRequest);
            }
            let tx = db.transaction()?;
            let changed = tx.execute(
                "UPDATE clients SET revoked=1 WHERE id=?1 AND revoked=0",
                [&id],
            )?;
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM clients WHERE id=?1)",
                [&id],
                |row| row.get(0),
            )?;
            tx.commit()?;
            if !exists {
                return Err(WriteError::NotFound);
            }
            Ok(json!({"client_id":id,"revoked":true,"changed":changed==1}))
        }
        Command::Clients => {
            let mut statement = db.prepare(
                "SELECT id,label,role,revoked,grant_revision FROM clients ORDER BY id LIMIT 50",
            )?;
            let rows=statement.query_map([],|row|Ok(json!({"client_id":row.get::<_,String>(0)?,"label":row.get::<_,String>(1)?,"role":row.get::<_,String>(2)?,"revoked":row.get::<_,bool>(3)?,"grant_revision":row.get::<_,u32>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(json!({"clients":rows,"limit":50}))
        }
        Command::Get { hash, query } => {
            let (id, _) = identity(db, &hash)?;
            grant(db, &id, &query.namespace)?;
            if !schema::valid_identifier(&query.id, false) || query.revision == Some(0) {
                return Err(WriteError::InvalidRequest);
            }
            if !crate::retrieval::visible(db, &query.namespace, &query.id)? {
                return Err(WriteError::NotFound);
            }
            let record = schema::revision(db, &query.namespace, &query.id, query.revision)?
                .ok_or(WriteError::NotFound)?;
            serde_json::to_value(record).map_err(|_| WriteError::PersistenceRejected)
        }
        Command::Status { hash } => {
            let (id, role) = identity(db, &hash)?;
            Ok(
                json!({"state":"unlocked","client_id":id,"role":role,"schema_version":schema::VERSION}),
            )
        }
        Command::AcceptedInput { request } => {
            if !schema::valid_identifier(&request.namespace, true)
                || !schema::valid_identifier(&request.id, false)
                || !schema::valid_identifier(&request.idempotency_key, false)
                || request.expected_revision == 0
            {
                return Err(WriteError::InvalidRequest);
            }
            let mut revision = schema::revision(
                db,
                &request.namespace,
                &request.id,
                Some(request.expected_revision),
            )?
            .ok_or(WriteError::RevisionConflict)?;
            revision.record.state = State::Accepted;
            serde_json::to_value(crate::writer::WriteRequest {
                record: revision.record,
                expected_revision: Some(request.expected_revision),
                idempotency_key: request.idempotency_key,
            })
            .map_err(|_| WriteError::PersistenceRejected)
        }
    }
}

fn issue(db: &mut Connection, request: NewClient, port: u16) -> CommandResult {
    if request.label.is_empty()
        || request.label.len() > 128
        || request.label.contains('\0')
        || request.namespaces.is_empty()
        || request.namespaces.len() > 32
        || request
            .namespaces
            .iter()
            .any(|name| !schema::valid_identifier(name, true))
        || request.namespaces.iter().collect::<HashSet<_>>().len() != request.namespaces.len()
    {
        return Err(WriteError::InvalidRequest);
    }
    let token = credentials::random_hex(32).map_err(|_| WriteError::PersistenceRejected)?;
    let id = credentials::random_hex(16)
        .map_err(|_| WriteError::PersistenceRejected)?
        .to_string();
    let profile = credentials::protect(&token, id.clone(), port)
        .map_err(|_| WriteError::PersistenceRejected)?;
    let hash = credentials::token_hash(&token).ok_or(WriteError::PersistenceRejected)?;
    let now: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| WriteError::PersistenceRejected)?
        .as_millis()
        .try_into()
        .map_err(|_| WriteError::PersistenceRejected)?;
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO clients(id,label,token_hash,role,created_at_ms) VALUES(?1,?2,?3,?4,?5)",
        params![
            id,
            request.label,
            hash.as_slice(),
            match request.role {
                Role::Reader => "reader",
                Role::Contributor => "contributor",
            },
            now
        ],
    )?;
    for namespace in request.namespaces {
        tx.execute(
            "INSERT INTO client_grants VALUES(?1,?2)",
            params![id, namespace],
        )?;
    }
    tx.commit()?;
    serde_json::to_value(profile).map_err(|_| WriteError::PersistenceRejected)
}
