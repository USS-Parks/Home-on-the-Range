use super::*;
use crate::{
    lifecycle::{self, Action, Request},
    owner,
    schema::{self, Kind, RecordInput, State},
};
use rusqlite::Connection;
use serde_json::Value;
use std::{
    fs,
    io::Write,
    net::{Ipv4Addr, TcpListener},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const KEY: &[u8] = b"HOTR-07-synthetic-key-866bc4ad";
const BODY: &str = "HOTR07canary";

fn run_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let base = owner::safe_absolute(&root.join("work/hotr-tests")).unwrap();
    fs::create_dir_all(&base).unwrap();
    let base = base.canonicalize().unwrap();
    assert!(base.starts_with(&root));
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let run = base.join(format!("HOTR-07-{}-{stamp}", std::process::id()));
    fs::create_dir(&run).unwrap();
    write_new(
        &run.join("SYNTHETIC-ONLY"),
        b"HOTR-07; synthetic embedding fixtures\n",
    );
    run
}

fn write_new(path: &Path, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn database(run: &Path) -> Connection {
    schema::open(&run.join("vault/vault.db"), KEY).unwrap()
}

fn fixture(run: &Path) -> Connection {
    owner::create(&run.join("vault"), KEY).unwrap();
    database(run)
}

fn stop() -> AtomicBool {
    AtomicBool::new(false)
}

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(3)
}

fn record(id: &str, body: &str) -> RecordInput {
    RecordInput {
        namespace: "project/test".into(),
        id: id.into(),
        kind: Kind::Note,
        body: body.into(),
        state: State::Accepted,
        sources: vec![],
        tags: vec![],
    }
}

fn append(db: &mut Connection, id: &str, revision: u32, body: &str) {
    let tx = db.transaction().unwrap();
    crate::writer::append_revision(
        &tx,
        "owner",
        &record(id, body),
        (revision > 1).then_some(revision - 1),
        revision,
        now().unwrap(),
    )
    .unwrap();
    tx.commit().unwrap();
}

fn configure_port(db: &mut Connection, port: u16, generation: u32) {
    configure(
        db,
        Configure {
            port: Some(port),
            expected_generation: generation,
        },
        deadline(),
        &stop(),
    )
    .unwrap();
}

fn job(db: &mut Connection) -> Job {
    serde_json::from_value(next(db, deadline(), &stop()).unwrap()).unwrap()
}

fn unit(port: u16) -> crate::embedding_transport::Embedding {
    crate::embedding_transport::Embedding {
        vector: vec![
            1.0 / (crate::embedding_transport::DIMENSIONS as f32).sqrt();
            crate::embedding_transport::DIMENSIONS
        ],
        peer: format!("127.0.0.1:{port}"),
    }
}

fn completion(job: Job) -> Completion {
    Completion {
        result: Ok(unit(job.port)),
        job,
    }
}

fn closed_port() -> u16 {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[test]
fn encrypted_defaults_cas_and_lease_claim_are_durable() {
    let run = run_dir();
    let db = fixture(&run);
    assert_eq!(status(&db).unwrap()["enabled"], false);
    assert_eq!(status(&db).unwrap()["indexed"], 0);
    drop(db);
    assert_eq!(
        schema::inspect_version(&run.join("vault/vault.db"), KEY).unwrap(),
        schema::VERSION
    );
    let mut db = database(&run);
    assert_eq!(
        configure(
            &mut db,
            Configure {
                port: Some(0),
                expected_generation: 0
            },
            deadline(),
            &stop()
        ),
        Err(WriteError::InvalidRequest)
    );
    configure_port(&mut db, 23451, 0);
    assert_eq!(
        configure(
            &mut db,
            Configure {
                port: Some(23452),
                expected_generation: 0
            },
            deadline(),
            &stop()
        ),
        Err(WriteError::RevisionConflict)
    );
    append(&mut db, "lease", 1, BODY);
    let claimed = job(&mut db);
    assert_eq!(claimed.attempt, 1);
    assert!(
        next(&mut db, deadline(), &stop()).unwrap().is_null(),
        "a durable lease must prevent a second claim"
    );
    assert_eq!(
        db.query_row("SELECT attempts FROM embedding_index", [], |row| row
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn malformed_and_duplicate_completions_never_store_or_duplicate() {
    let run = run_dir();
    let mut db = fixture(&run);
    configure_port(&mut db, 23453, 0);
    append(&mut db, "vector", 1, BODY);
    let claimed = job(&mut db);
    for embedding in [
        crate::embedding_transport::Embedding {
            vector: vec![1.0],
            peer: format!("127.0.0.1:{}", claimed.port),
        },
        crate::embedding_transport::Embedding {
            vector: vec![f32::NAN; crate::embedding_transport::DIMENSIONS],
            peer: format!("127.0.0.1:{}", claimed.port),
        },
        crate::embedding_transport::Embedding {
            vector: vec![1.0; crate::embedding_transport::DIMENSIONS],
            peer: format!("127.0.0.1:{}", claimed.port),
        },
        crate::embedding_transport::Embedding {
            vector: unit(claimed.port).vector,
            peer: "127.0.0.1:1".into(),
        },
    ] {
        let duplicate = Job {
            namespace: claimed.namespace.clone(),
            id: claimed.id.clone(),
            revision: claimed.revision,
            generation: claimed.generation,
            attempt: claimed.attempt,
            port: claimed.port,
            body: claimed.body.clone(),
        };
        assert_eq!(
            complete(
                &mut db,
                Completion {
                    job: duplicate,
                    result: Ok(embedding)
                },
                deadline(),
                &stop()
            ),
            Err(WriteError::InvalidRequest)
        );
        assert!(
            db.query_row("SELECT vector IS NULL FROM embedding_index", [], |row| row
                .get::<_, bool>(
                0
            ))
            .unwrap()
        );
    }
    let duplicate = Job {
        namespace: claimed.namespace.clone(),
        id: claimed.id.clone(),
        revision: claimed.revision,
        generation: claimed.generation,
        attempt: claimed.attempt,
        port: claimed.port,
        body: claimed.body.clone(),
    };
    assert_eq!(
        complete(&mut db, completion(claimed), deadline(), &stop()).unwrap()["stored"],
        true
    );
    assert_eq!(
        complete(&mut db, completion(duplicate), deadline(), &stop()).unwrap()["stored"],
        false
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM current_embeddings", [], |row| row
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[tokio::test(flavor = "current_thread")]
async fn retries_stop_durably_but_new_revision_and_generation_are_eligible() {
    let run = run_dir();
    let mut db = fixture(&run);
    let port = closed_port();
    configure_port(&mut db, port, 0);
    append(&mut db, "retry", 1, BODY);
    for attempt in 1..=MAX_ATTEMPTS {
        let claimed = job(&mut db);
        assert_eq!(claimed.attempt, attempt);
        let result = crate::embedding_transport::embed(claimed.port, &claimed.body, false).await;
        assert!(result.is_err());
        assert_eq!(
            complete(
                &mut db,
                Completion {
                    job: claimed,
                    result
                },
                deadline(),
                &stop()
            )
            .unwrap()["stored"],
            false
        );
        db.execute("UPDATE embedding_index SET due_ms=0", [])
            .unwrap();
    }
    assert!(next(&mut db, deadline(), &stop()).unwrap().is_null());
    drop(db);
    let mut db = database(&run);
    assert_eq!(
        db.query_row("SELECT attempts FROM embedding_index", [], |row| row
            .get::<_, u32>(0))
            .unwrap(),
        MAX_ATTEMPTS
    );
    assert!(next(&mut db, deadline(), &stop()).unwrap().is_null());
    append(&mut db, "retry", 2, "HOTR07canary corrected");
    assert_eq!(job(&mut db).attempt, 1);
    configure_port(&mut db, port, 1);
    assert_eq!(job(&mut db).attempt, 1);
}

#[test]
fn stale_corrections_generations_and_visibility_cannot_resurrect_vectors() {
    let run = run_dir();
    let mut db = fixture(&run);
    configure_port(&mut db, 23454, 0);
    append(&mut db, "correct", 1, BODY);
    let stale = job(&mut db);
    let correction = Request {
        idempotency_key: "correct-race".into(),
        action: Action::Correct {
            record: record("correct", "HOTR07canary corrected"),
            expected_revision: 1,
        },
    };
    lifecycle::execute(&mut db, correction, deadline(), &stop()).unwrap();
    assert_eq!(
        complete(&mut db, completion(stale), deadline(), &stop()).unwrap()["reason"],
        "stale"
    );
    let current = job(&mut db);
    assert_eq!(current.revision, 2);
    assert_eq!(
        complete(&mut db, completion(current), deadline(), &stop()).unwrap()["stored"],
        true
    );
    assert_eq!(status(&db).unwrap()["indexed"], 1);
    configure_port(&mut db, 23455, 1);
    assert_eq!(
        status(&db).unwrap()["indexed"],
        0,
        "a generation change invalidates old vectors"
    );
    let stale_generation = job(&mut db);
    configure_port(&mut db, 23456, 2);
    assert_eq!(
        complete(&mut db, completion(stale_generation), deadline(), &stop()).unwrap()["reason"],
        "stale"
    );
    let regenerated = job(&mut db);
    assert_eq!(regenerated.generation, 3);
    assert_eq!(
        complete(&mut db, completion(regenerated), deadline(), &stop()).unwrap()["stored"],
        true
    );

    append(&mut db, "inflight-tombstone", 1, BODY);
    let tombstoned_inflight = job(&mut db);
    lifecycle::execute(
        &mut db,
        Request {
            idempotency_key: "inflight-tombstone".into(),
            action: Action::Visibility {
                namespace: "project/test".into(),
                id: "inflight-tombstone".into(),
                expected_revision: 1,
                tombstoned: true,
                valid_from_ms: None,
                expires_at_ms: None,
            },
        },
        deadline(),
        &stop(),
    )
    .unwrap();
    assert_eq!(
        complete(
            &mut db,
            completion(tombstoned_inflight),
            deadline(),
            &stop()
        )
        .unwrap()["reason"],
        "stale"
    );

    for id in ["tombstone", "expiry", "future", "old", "replacement"] {
        append(&mut db, id, 1, BODY);
        let visible_job = job(&mut db);
        assert_eq!(
            complete(&mut db, completion(visible_job), deadline(), &stop()).unwrap()["stored"],
            true
        );
    }
    let visibility = |id: &str,
                      tombstoned: bool,
                      valid_from_ms: Option<i64>,
                      expires_at_ms: Option<i64>| Request {
        idempotency_key: format!("visibility-{id}"),
        action: Action::Visibility {
            namespace: "project/test".into(),
            id: id.into(),
            expected_revision: 1,
            tombstoned,
            valid_from_ms,
            expires_at_ms,
        },
    };
    lifecycle::execute(
        &mut db,
        visibility("tombstone", true, None, None),
        deadline(),
        &stop(),
    )
    .unwrap();
    lifecycle::execute(
        &mut db,
        visibility("expiry", false, None, Some(0)),
        deadline(),
        &stop(),
    )
    .unwrap();
    lifecycle::execute(
        &mut db,
        visibility("future", false, Some(now().unwrap() + 60_000), None),
        deadline(),
        &stop(),
    )
    .unwrap();
    lifecycle::execute(
        &mut db,
        Request {
            idempotency_key: "supersede".into(),
            action: Action::Supersede {
                namespace: "project/test".into(),
                old_id: "old".into(),
                old_revision: 1,
                replacement_id: "replacement".into(),
                replacement_revision: 1,
            },
        },
        deadline(),
        &stop(),
    )
    .unwrap();
    for id in ["tombstone", "expiry", "future", "old"] {
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM current_embeddings WHERE record_id=?1",
                [id],
                |row| row.get::<_, u32>(0)
            )
            .unwrap(),
            0,
            "{id} retained an old vector"
        );
    }
    let view_count: u32 = db
        .query_row("SELECT count(*) FROM current_embeddings", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(status(&db).unwrap()["indexed"], Value::from(view_count));
    assert!(
        status(&db).unwrap()["pending"].as_u64().unwrap()
            <= status(&db).unwrap()["visible"].as_u64().unwrap()
    );
    db.execute(
        "UPDATE embedding_config SET model_digest='0000000000000000000000000000000000000000000000000000000000000000'",
        [],
    )
    .unwrap();
    assert_eq!(
        status(&db).unwrap()["indexed"],
        0,
        "wrong model digests fail closed"
    );
    assert!(next(&mut db, deadline(), &stop()).unwrap().is_null());
}
