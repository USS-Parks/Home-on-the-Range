use super::*;
use crate::{
    owner,
    schema::{self, Kind, SourceReference, State},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
};

const KEY: &[u8] = b"HOTR-06-synthetic-key-493bf46e";
const BODY: &str = "HOTR06canary Tokyo 東京 roadmap revision";

fn run_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let base = root.join("work/hotr-tests").canonicalize().unwrap();
    assert!(base.starts_with(&root));
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let run = base.join(format!("HOTR-06-{}-{stamp}", std::process::id()));
    fs::create_dir(&run).unwrap();
    write_new(
        &run.join("SYNTHETIC-ONLY"),
        b"HOTR-06; synthetic transaction fixtures\n",
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

fn connection(run: &Path) -> Connection {
    schema::open(&run.join("vault/vault.db"), KEY).unwrap()
}

fn fixture(run: &Path) {
    owner::create(&run.join("vault"), KEY).unwrap();
}

fn request(id: &str, key: &str, expected: Option<u32>) -> WriteRequest {
    WriteRequest {
        record: RecordInput {
            namespace: "project/test".into(),
            id: id.into(),
            kind: Kind::Roadmap,
            body: BODY.into(),
            state: State::Proposed,
            sources: vec![SourceReference {
                reference: "file:///opaque-unopened-source".into(),
                label: "synthetic".into(),
            }],
            tags: vec!["東京".into()],
        },
        expected_revision: expected,
        idempotency_key: key.into(),
    }
}

fn committed(result: Result) -> Receipt {
    match result.unwrap() {
        WriteOutcome::Committed(receipt) => receipt,
        outcome => panic!("expected committed receipt, received {outcome:?}"),
    }
}

fn count(db: &Connection, table: &str) -> i64 {
    // Names are fixed test literals; no product SQL surface uses this helper.
    assert!(["records", "revisions", "mutation_audit", "write_receipts"].contains(&table));
    db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn atomic_revisions_replay_and_principal_scoping() {
    let run = run_dir();
    fixture(&run);
    let writer = Writer::start(connection(&run)).unwrap();
    let a = writer.handle();
    let b = writer.handle();
    let original = request("shared", "create", None);
    let first = committed(a.submit("client-a", original.clone()).unwrap().wait().await);
    assert_eq!(first.revision, 1);
    let one = a
        .submit("client-a", request("shared", "race-a", Some(1)))
        .unwrap();
    let two = b
        .submit("client-b", request("shared", "race-b", Some(1)))
        .unwrap();
    let (one, two) = tokio::join!(one.wait(), two.wait());
    assert_eq!(
        [&one, &two]
            .iter()
            .filter(|r| matches!(r, Ok(WriteOutcome::Committed(_))))
            .count(),
        1
    );
    assert_eq!(
        [&one, &two]
            .iter()
            .filter(|r| matches!(r, Err(WriteError::RevisionConflict)))
            .count(),
        1
    );
    let replay = committed(a.submit("client-a", original.clone()).unwrap().wait().await);
    assert_eq!(
        replay, first,
        "receipt must remain stable after a later revision"
    );
    let mut changed = original.clone();
    changed.record.body.push('!');
    assert_eq!(
        a.submit("client-a", changed).unwrap().wait().await,
        Err(WriteError::IdempotencyConflict)
    );
    assert_eq!(
        a.submit("client-a", request("shared", "stale", Some(1)))
            .unwrap()
            .wait()
            .await,
        Err(WriteError::RevisionConflict)
    );
    // Same text key belongs to a different principal and is not a duplicate.
    let other = committed(
        b.submit("client-b", request("independent", "create", None))
            .unwrap()
            .wait()
            .await,
    );
    assert_eq!(other.revision, 1);
    writer.shutdown().await.unwrap();
    assert!(matches!(
        a.submit("client-a", request("closed", "closed", None)),
        Err(WriteError::Stopped)
    ));
    let mut db = connection(&run);
    assert_eq!(count(&db, "records"), 2);
    for table in ["revisions", "mutation_audit", "write_receipts"] {
        assert_eq!(count(&db, table), 3);
    }
    let latest = schema::revision(&db, "project/test", "shared", None)
        .unwrap()
        .unwrap();
    assert_eq!(latest.revision, 2);
    assert_eq!(latest.record.body, BODY);
    assert_eq!(latest.record.sources, original.record.sources);
    assert_eq!(latest.record.tags, original.record.tags);
    for sql in [
        "UPDATE write_receipts SET revision=1",
        "DELETE FROM write_receipts",
        "UPDATE mutation_audit SET principal='forged'",
        "DELETE FROM mutation_audit",
    ] {
        assert!(db.execute(sql, []).is_err());
    }
    // A persistence failure after record insertion must roll back all four tables.
    db.execute_batch("CREATE TRIGGER reject_audit BEFORE INSERT ON mutation_audit BEGIN SELECT RAISE(ABORT,'synthetic audit rejection'); END;").unwrap();
    let writer = Writer::start(db).unwrap();
    assert_eq!(
        writer
            .handle()
            .submit("client-a", request("audit-fail", "audit-fail", None))
            .unwrap()
            .wait()
            .await,
        Err(WriteError::PersistenceRejected)
    );
    writer.shutdown().await.unwrap();
    db = connection(&run);
    assert!(
        schema::revision(&db, "project/test", "audit-fail", None)
            .unwrap()
            .is_none()
    );
    assert_eq!(count(&db, "write_receipts"), 3);
    assert_eq!(count(&db, "mutation_audit"), 3);
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    write_new(&run.join("atomic.json"),b"{\"result\":\"PASS\",\"conflict_winners\":1,\"durable_revisions\":3,\"audit_rollback\":true,\"principal_scoped_replay\":true}");
}

type PausedHook = (
    Arc<dyn Fn() + Send + Sync>,
    mpsc::Receiver<()>,
    mpsc::SyncSender<()>,
);
fn pause_hook() -> PausedHook {
    let (ready_send, ready) = mpsc::sync_channel(1);
    let (release, receive) = mpsc::sync_channel(1);
    let receive = Mutex::new(receive);
    let used = AtomicBool::new(false);
    let hook = Arc::new(move || {
        if !used.swap(true, Ordering::SeqCst) {
            ready_send.send(()).unwrap();
            receive
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
    });
    (hook, ready, release)
}

#[tokio::test(flavor = "current_thread")]
async fn simultaneous_native_threads_have_one_revision_winner() {
    let run = run_dir();
    fixture(&run);
    let writer = Writer::start(connection(&run)).unwrap();
    committed(
        writer
            .handle()
            .submit("seed", request("shared", "seed", None))
            .unwrap()
            .wait()
            .await,
    );
    writer.shutdown().await.unwrap();
    let (hook, ready, release) = pause_hook();
    let writer = Writer::start_inner(
        connection(&run),
        Hooks {
            before_commit: Some(hook),
            after_commit: None,
        },
    )
    .unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(4));
    let mut threads = Vec::new();
    for index in 0..4 {
        let handle = writer.handle();
        let barrier = barrier.clone();
        threads.push(thread::spawn(move || {
            barrier.wait();
            handle
                .submit(
                    &format!("client-{index}"),
                    request("shared", &format!("race-{index}"), Some(1)),
                )
                .unwrap()
        }));
    }
    let tickets: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    release.send(()).unwrap();
    let mut winners = 0;
    let mut conflicts = 0;
    for ticket in tickets {
        match ticket.wait().await {
            Ok(WriteOutcome::Committed(receipt)) => {
                assert_eq!(receipt.revision, 2);
                winners += 1;
            }
            Err(WriteError::RevisionConflict) => conflicts += 1,
            other => panic!("unexpected race outcome {other:?}"),
        }
    }
    assert_eq!((winners, conflicts), (1, 3));
    writer.shutdown().await.unwrap();
    let db = connection(&run);
    for table in ["revisions", "mutation_audit", "write_receipts"] {
        assert_eq!(count(&db, table), 2);
    }
    let mismatches:i64=db.query_row("SELECT count(*) FROM write_receipts r JOIN mutation_audit a ON a.sequence=r.audit_sequence WHERE r.principal!=a.principal OR r.namespace!=a.namespace OR r.record_id!=a.record_id OR r.revision!=a.revision OR r.committed_at_ms!=a.committed_at_ms",[],|row|row.get(0)).unwrap();
    assert_eq!(mismatches, 0);
    write_new(&run.join("concurrency.json"),b"{\"result\":\"PASS\",\"native_submitter_threads\":4,\"winners\":1,\"conflicts\":3,\"receipt_audit_mismatches\":0}");
}

#[tokio::test(flavor = "current_thread")]
async fn queue_limit_cancellation_and_unknown_commit_outcome() {
    let run = run_dir();
    fixture(&run);
    let (hook, ready, release) = pause_hook();
    let writer = Writer::start_inner(
        connection(&run),
        Hooks {
            before_commit: Some(hook),
            after_commit: None,
        },
    )
    .unwrap();
    let handle = writer.handle();
    let first = handle
        .submit("client", request("first", "first", None))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    let mut queued = Vec::new();
    for i in 0..QUEUE_CAPACITY {
        queued.push(
            handle
                .submit("client", request(&format!("q{i}"), &format!("q{i}"), None))
                .unwrap(),
        );
    }
    assert!(matches!(
        handle.submit("client", request("overflow", "overflow", None)),
        Err(WriteError::Overloaded)
    ));
    assert_eq!(first.cancel().unwrap(), WriteOutcome::Canceled);
    for item in queued {
        assert_eq!(item.cancel().unwrap(), WriteOutcome::Canceled);
    }
    release.send(()).unwrap();
    writer.shutdown().await.unwrap();
    let db = connection(&run);
    for table in ["records", "revisions", "mutation_audit", "write_receipts"] {
        assert_eq!(count(&db, table), 0);
    }
    drop(db);

    let (hook, ready, release) = pause_hook();
    let writer = Writer::start_inner(
        connection(&run),
        Hooks {
            before_commit: None,
            after_commit: Some(hook),
        },
    )
    .unwrap();
    let handle = writer.handle();
    let original = request("ambiguous", "ambiguous", None);
    let pending = handle.submit("client", original.clone()).unwrap();
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(pending.cancel().unwrap(), WriteOutcome::UnknownToClient);
    release.send(()).unwrap();
    let replay = committed(handle.submit("client", original).unwrap().wait().await);
    assert_eq!(replay.revision, 1);
    writer.shutdown().await.unwrap();
    let db = connection(&run);
    for table in ["records", "revisions", "mutation_audit", "write_receipts"] {
        assert_eq!(count(&db, table), 1);
    }
    write_new(&run.join("bounds.json"),b"{\"result\":\"PASS\",\"queue_capacity\":256,\"canceled_rolled_back\":true,\"committed_timeout_resolved_by_retry\":true}");
}

#[tokio::test(flavor = "current_thread")]
async fn expired_wait_and_dropped_future_do_not_commit() {
    let run = run_dir();
    fixture(&run);
    let (hook, ready, release) = pause_hook();
    let writer = Writer::start_inner(
        connection(&run),
        Hooks {
            before_commit: Some(hook),
            after_commit: None,
        },
    )
    .unwrap();
    let handle = writer.handle();
    let mut pending = handle
        .submit("client", request("expired", "expired", None))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    // Exercise the same actual timer/cancellation path with a short test wait.
    pending.deadline = Instant::now() + Duration::from_millis(50);
    assert_eq!(pending.wait().await.unwrap(), WriteOutcome::Canceled);
    drop(
        handle
            .submit("client", request("dropped", "dropped", None))
            .unwrap(),
    );
    release.send(()).unwrap();
    let after = committed(
        handle
            .submit("client", request("after", "after", None))
            .unwrap()
            .wait()
            .await,
    );
    assert_eq!(after.revision, 1);
    writer.shutdown().await.unwrap();
    let db = connection(&run);
    assert_eq!(count(&db, "records"), 1);
    assert!(
        schema::revision(&db, "project/test", "expired", None)
            .unwrap()
            .is_none()
    );
    assert!(
        schema::revision(&db, "project/test", "dropped", None)
            .unwrap()
            .is_none()
    );
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn validated_child_run() -> PathBuf {
    let path =
        PathBuf::from(std::env::var_os("HOTR_WRITE_FIXTURE").expect("synthetic path required"))
            .canonicalize()
            .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("work/hotr-tests")
        .canonicalize()
        .unwrap();
    assert!(path.starts_with(&root) && path != root);
    assert!(
        fs::read_to_string(path.join("SYNTHETIC-ONLY"))
            .unwrap()
            .starts_with("HOTR-06;")
    );
    for ancestor in path.ancestors().take_while(|p| p.starts_with(&root)) {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            fs::symlink_metadata(ancestor).unwrap().file_attributes() & 0x400,
            0
        );
    }
    path
}

#[test]
#[ignore = "owned actual transaction crash helper"]
fn crash_fixture_child() {
    let run = validated_child_run();
    let phase = std::env::var("HOTR_WRITE_PHASE").unwrap();
    assert!(["before", "after", "ack"].contains(&phase.as_str()));
    let checkpoint = run.join(format!("{phase}-checkpoint"));
    let hook = Arc::new(move || {
        write_new(&checkpoint, b"synthetic crash boundary\n");
        thread::sleep(Duration::from_secs(30));
    });
    let hooks = match phase.as_str() {
        "before" => Hooks {
            before_commit: Some(hook),
            after_commit: None,
        },
        "after" => Hooks {
            before_commit: None,
            after_commit: Some(hook),
        },
        _ => Hooks::default(),
    };
    let writer = Writer::start_inner(connection(&run), hooks).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let receipt = runtime.block_on(async {
        committed(
            writer
                .handle()
                .submit("crash-client", request(&phase, &phase, None))
                .unwrap()
                .wait()
                .await,
        )
    });
    if phase == "ack" {
        println!("HOTR_ACK={}", serde_json::to_string(&receipt).unwrap());
        io::stdout().flush().unwrap();
        thread::sleep(Duration::from_secs(30));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn actual_crash_replay_and_durable_client_acknowledgment() {
    use std::io::{BufRead, BufReader};
    let run = run_dir();
    fixture(&run);
    for phase in ["before", "after", "ack"] {
        let stderr = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(run.join(format!("{phase}.stderr.txt")))
            .unwrap();
        let mut child = OwnedChild(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "writer::tests::crash_fixture_child",
                    "--ignored",
                    "--exact",
                    "--nocapture",
                ])
                .env("HOTR_WRITE_FIXTURE", &run)
                .env("HOTR_WRITE_PHASE", phase)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(stderr)
                .spawn()
                .unwrap(),
        );
        let stdout = child.0.stdout.take().unwrap();
        let (send, receive) = mpsc::sync_channel(1);
        let stdout_log = run.join(format!("{phase}.stdout.txt"));
        thread::spawn(move || {
            let mut captured = Vec::new();
            for line in BufReader::new(stdout)
                .lines()
                .map_while(std::result::Result::ok)
            {
                if captured.len() + line.len() + 1 > 16384 {
                    break;
                }
                captured.extend_from_slice(line.as_bytes());
                captured.push(b'\n');
                // With RUST_TEST_THREADS=1 libtest prints its test-name prefix
                // on the same line as nocapture output from the fixture.
                let payload = line
                    .strip_prefix("test writer::tests::crash_fixture_child ... ")
                    .unwrap_or(&line);
                if let Some(ack) = payload.strip_prefix("HOTR_ACK=") {
                    let _ = send.send(ack.to_owned());
                    break;
                }
            }
            write_new(&stdout_log, &captured);
        });
        let acknowledgment = if phase == "ack" {
            let line = receive
                .recv_timeout(Duration::from_secs(10))
                .expect("actual acknowledgment missing");
            let receipt: Receipt = serde_json::from_str(&line).unwrap();
            // The parent/client persists the received acknowledgment before kill.
            write_new(&run.join("client-acknowledgment.json"), line.as_bytes());
            Some(receipt)
        } else {
            let start = Instant::now();
            while !run.join(format!("{phase}-checkpoint")).exists() {
                assert!(
                    start.elapsed() < Duration::from_secs(10),
                    "crash boundary timeout"
                );
                assert!(
                    child.0.try_wait().unwrap().is_none(),
                    "helper exited; inspect retained stderr"
                );
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            None
        };
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        let db = connection(&run);
        let existed = schema::revision(&db, "project/test", phase, None)
            .unwrap()
            .is_some();
        assert_eq!(existed, phase != "before");
        let writer = Writer::start(db).unwrap();
        let handle = writer.handle();
        let replay = committed(
            handle
                .submit("crash-client", request(phase, phase, None))
                .unwrap()
                .wait()
                .await,
        );
        assert_eq!(replay.revision, 1);
        if let Some(ack) = acknowledgment {
            assert_eq!(replay, ack);
        }
        assert_eq!(
            committed(
                handle
                    .submit("crash-client", request(phase, phase, None))
                    .unwrap()
                    .wait()
                    .await
            ),
            replay
        );
        writer.shutdown().await.unwrap();
    }
    let db = connection(&run);
    for table in ["records", "revisions", "mutation_audit", "write_receipts"] {
        assert_eq!(count(&db, table), 3);
    }
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    write_new(&run.join("crashes.json"),b"{\"result\":\"PASS\",\"owned_crash_phases\":[\"before_commit\",\"after_commit_before_reply\",\"after_client_durable_ack\"],\"cycles\":3,\"missing_acknowledgments\":0,\"duplicate_revisions\":0}");
}
