use hotr::{
    api,
    capabilities::{Accept, NewClient, Role},
    credentials::{self, CredentialProfile},
    owner::{self, AdminRequest},
    schema::{Kind, RecordInput, SourceReference, State},
    writer::WriteRequest,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};
use zeroize::Zeroizing;

const KEY: &[u8] = b"HOTR-07-synthetic-key-866bc4ad";
const BODY: &str = "HOTR07canary shared sourced roadmap 東京";

#[path = "support/mcp.rs"]
mod mcp_protocol;

fn local_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(2))
        .http1_only()
        .build()
        .unwrap()
}

fn page(namespace: &str, limit: u32, offset: u32) -> Value {
    json!({"namespace":namespace,"limit":limit,"offset":offset,"byte_budget":65536,"token_budget":65536})
}

#[tokio::test(flavor = "current_thread")]
async fn actual_scoped_fts_current_history_visibility_and_budgets() {
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "fts-before");
    unlock(&run).await;
    let (_, token) = issue_cli(&run, "fts-a", "contributor", "alpha");
    let (_, hidden) = issue_cli(&run, "fts-b", "contributor", "beta");
    let client = local_client();
    let path = r"C:\selected\roadmap.md";
    for id in [
        "current",
        "deleted",
        "expired",
        "superseded",
        "new",
        "desired-name",
        "a-path-mentions",
        "z-path",
    ] {
        let mut request = write_request("alpha", id, id, None);
        request.record.body = format!("{BODY} commonword desired-name {path}");
        if id == "current" {
            request.record.body.push_str(" obsoleteword");
        }
        if id == "z-path" {
            request.record.sources[0].reference = path.into();
        }
        assert_eq!(
            post(&client, server.port, &token, "/v1/records", &request)
                .await
                .0,
            200
        );
    }
    let hidden_record = write_request("beta", "hidden", "hidden", None);
    assert_eq!(
        post(&client, server.port, &hidden, "/v1/records", &hidden_record)
            .await
            .0,
        200
    );
    let mut update = write_request("alpha", "current", "updated", Some(1));
    update.record.body = format!("{BODY} commonword currentword");
    assert_eq!(
        post(&client, server.port, &token, "/v1/records", &update)
            .await
            .0,
        200
    );
    let stale = post(
        &client,
        server.port,
        &token,
        "/v1/search",
        &json!({"page":page("alpha",10,0),"query":"obsoleteword"}),
    )
    .await;
    assert_eq!(stale.0, 200);
    assert_eq!(stale.1["total"], 0);
    server.stop(&run).await;
    // Only the marked synthetic vault is opened while its owner is stopped.
    // HOTR-09 supplies retrieval filters; owner retirement operations follow in 14.
    let db = hotr::schema::open(&run.join("vault/vault.db"), KEY).unwrap();
    db.execute(
        "UPDATE record_visibility SET tombstoned=1 WHERE namespace='alpha' AND record_id='deleted'",
        [],
    )
    .unwrap();
    db.execute("UPDATE record_visibility SET expires_at_ms=1 WHERE namespace='alpha' AND record_id='expired'",[]).unwrap();
    db.execute(
        "INSERT INTO relations VALUES('alpha','new','superseded','supersedes')",
        [],
    )
    .unwrap();
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    db.execute(
        "INSERT INTO record_fts(record_fts) VALUES('integrity-check')",
        [],
    )
    .unwrap();
    drop(db);
    let mut server = Server::start(&run, "fts-after");
    unlock(&run).await;
    let (profile, reader) = issue_cli(&run, "fts-reader", "reader", "alpha");
    let current = post(
        &client,
        server.port,
        &reader,
        "/v1/search",
        &json!({"page":page("alpha",50,0),"query":"commonword"}),
    )
    .await;
    assert_eq!(current.0, 200);
    assert_eq!(current.1["total"], 5);
    let rows = current.1["records"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    assert!(rows.iter().all(|r| r["namespace"] == "alpha"
        && !["deleted", "expired", "superseded", "hidden"].contains(&r["id"].as_str().unwrap())));
    assert_eq!(
        rows.iter().find(|r| r["id"] == "current").unwrap()["revision"],
        2
    );
    assert!(
        rows.iter()
            .all(|r| !r["sources"].as_array().unwrap().is_empty())
    );
    for id in ["deleted", "expired", "superseded"] {
        assert_eq!(
            post(
                &client,
                server.port,
                &reader,
                "/v1/records/get",
                &json!({"namespace":"alpha","id":id})
            )
            .await
            .0,
            404
        );
        assert_eq!(
            post(
                &client,
                server.port,
                &reader,
                "/v1/records/get",
                &json!({"namespace":"alpha","id":id,"revision":1})
            )
            .await
            .0,
            200
        );
    }
    assert_eq!(
        post(
            &client,
            server.port,
            &reader,
            "/v1/records/count",
            &json!({"namespace":"alpha"})
        )
        .await
        .1["count"],
        5
    );
    for (query, expected) in [
        ("desired-name", "desired-name"),
        (path, "z-path"),
        ("currentword", "current"),
    ] {
        let result = api::scoped_request(
            &profile,
            "POST",
            "/v1/search",
            Some(&json!({"page":page("alpha",10,0),"query":query})),
        )
        .await
        .unwrap();
        assert_eq!(result.0, 200);
        assert_eq!(result.1["records"][0]["id"], expected);
    }
    let first = post(
        &client,
        server.port,
        &reader,
        "/v1/records/list",
        &page("alpha", 2, 0),
    )
    .await;
    assert_eq!(first.0, 200);
    assert_eq!(first.1["records"].as_array().unwrap().len(), 2);
    assert_eq!(first.1["next_offset"], 2);
    let second = post(
        &client,
        server.port,
        &reader,
        "/v1/records/list",
        &page("alpha", 2, 2),
    )
    .await;
    assert!(first.1["records"].as_array().unwrap().iter().all(|a| {
        second.1["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|b| a["id"] != b["id"])
    }));
    let history = post(
        &client,
        server.port,
        &reader,
        "/v1/records/history",
        &json!({"page":page("alpha",1,0),"id":"current"}),
    )
    .await;
    assert_eq!(history.0, 200);
    assert_eq!(history.1["total"], 2);
    assert_eq!(history.1["records"][0]["revision"], 1);
    assert_eq!(history.1["next_offset"], 1);
    for endpoint in [
        "/v1/search",
        "/v1/records/list",
        "/v1/records/count",
        "/v1/records/history",
    ] {
        let request = match endpoint {
            "/v1/search" => json!({"page":page("beta",10,0),"query":"shared"}),
            "/v1/records/history" => json!({"page":page("beta",10,0),"id":"hidden"}),
            "/v1/records/count" => json!({"namespace":"beta"}),
            _ => page("beta", 10, 0),
        };
        let denied = post(&client, server.port, &reader, endpoint, &request).await;
        assert_eq!(denied.0, 403);
        assert_eq!(denied.1, json!({"error":{"code":"forbidden"}}));
    }
    for query in [
        "\" OR *",
        "body:commonword",
        "commonword NOT hidden",
        "東京",
        "\"",
        "' ; DROP TABLE records; --",
    ] {
        let result = post(
            &client,
            server.port,
            &reader,
            "/v1/search",
            &json!({"page":page("alpha",10,0),"query":query}),
        )
        .await;
        assert_eq!(result.0, 200);
        assert!(
            result.1["records"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["namespace"] == "alpha")
        );
    }
    for query in [
        "".to_owned(),
        "x".repeat(513),
        "x ".repeat(33),
        "x\0y".into(),
    ] {
        assert_eq!(
            post(
                &client,
                server.port,
                &reader,
                "/v1/search",
                &json!({"page":page("alpha",10,0),"query":query})
            )
            .await
            .0,
            400
        );
    }
    for (field, value) in [
        ("limit", 51),
        ("offset", 100001),
        ("byte_budget", 262145),
        ("token_budget", 511),
    ] {
        let mut invalid = page("alpha", 10, 0);
        invalid[field] = json!(value);
        assert_eq!(
            post(&client, server.port, &reader, "/v1/records/list", &invalid)
                .await
                .0,
            400
        );
    }
    let mut tight = page("alpha", 50, 0);
    tight["byte_budget"] = json!(1024);
    tight["token_budget"] = json!(1024);
    let bounded = post(&client, server.port, &reader, "/v1/records/list", &tight).await;
    assert_eq!(bounded.0, 200);
    assert!(serde_json::to_vec(&bounded.1).unwrap().len() <= 1024);
    assert!(bounded.1["estimated_tokens"].as_u64().unwrap() <= 1024);
    assert!(bounded.1["omitted_for_budget"].as_u64().unwrap() > 0);
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Revoke {
                client_id: profile.client_id
            }
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    assert_eq!(
        post(
            &client,
            server.port,
            &reader,
            "/v1/search",
            &json!({"page":page("alpha",10,0),"query":"shared"})
        )
        .await
        .0,
        401
    );
    server.stop(&run).await;
    scan(&run, &[&token, &hidden, &reader]);
    write_new(&run.join("HOTR-09-retrieval.json"),&serde_json::to_vec_pretty(&json!({"prompt":"HOTR-09","result":"PASS","current_revision":true,"scoped_fts":true,"exact_id_source_boosts":true,"history_list_count":true,"tombstone_expiry_supersession_filters":true,"budgets":true,"no_plaintext_canary":true})).unwrap());
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
fn run_dir() -> PathBuf {
    use std::os::windows::fs::MetadataExt;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let base = root.join("work/hotr-tests");
    for ancestor in base.ancestors().take_while(|path| path.starts_with(&root)) {
        if let Ok(metadata) = fs::symlink_metadata(ancestor) {
            assert_eq!(metadata.file_attributes() & 0x400, 0);
        }
    }
    fs::create_dir_all(&base).unwrap();
    let base = base.canonicalize().unwrap();
    assert!(base.starts_with(root));
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let run = base.join(format!("HOTR-07-{}-{stamp}", std::process::id()));
    fs::create_dir(&run).unwrap();
    write_new(
        &run.join("SYNTHETIC-ONLY"),
        b"HOTR-07; synthetic capability and HTTP fixtures\n",
    );
    run
}
struct Server {
    child: Child,
    port: u16,
}
impl Drop for Server {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
impl Server {
    fn start(run: &Path, label: &str) -> Self {
        let stderr = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(run.join(format!("{label}.stderr.txt")))
            .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hotr"))
            .arg("serve")
            .arg(run.join("vault"))
            .args(["--port", "0"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, receive) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut line = String::new();
            let _ = BufReader::new(stdout).read_line(&mut line);
            let _ = send.send(line);
        });
        let mut server = Self { child, port: 0 };
        let line = receive
            .recv_timeout(Duration::from_secs(12))
            .expect("server readiness timed out");
        let value: Value =
            serde_json::from_str(&line).expect("server readiness invalid; inspect retained stderr");
        server.port = value["port"].as_u64().unwrap() as u16;
        server
    }
    async fn stop(&mut self, run: &Path) {
        assert!(
            owner::request(&run.join("vault"), owner::LOCK, &[])
                .await
                .unwrap()
                .closing
        );
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(7));
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
async fn unlock(run: &Path) {
    assert!(
        owner::request(&run.join("vault"), owner::UNLOCK, KEY)
            .await
            .unwrap()
            .error
            .is_none()
    );
}

fn issue_cli(
    run: &Path,
    label: &str,
    role: &str,
    namespace: &str,
) -> (CredentialProfile, Zeroizing<String>) {
    let path = run.join(format!("{label}.credential"));
    let output = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .arg("issue")
        .arg(run.join("vault"))
        .arg("--credential")
        .arg(&path)
        .args(["--label", label, "--role", role, "--namespace", namespace])
        .output()
        .unwrap();
    write_new(&run.join(format!("{label}.cli.stdout.txt")), &output.stdout);
    write_new(&run.join(format!("{label}.cli.stderr.txt")), &output.stderr);
    assert!(
        output.status.success(),
        "owner CLI issue failed; inspect retained diagnostics"
    );
    let profile = credentials::load(&path).unwrap();
    let token = credentials::unprotect(&profile).unwrap();
    assert!(credentials::token_hash(&token).is_some());
    assert!(
        !output
            .stdout
            .windows(token.len())
            .any(|w| w == token.as_bytes())
    );
    let before = Sha256::digest(fs::read(&path).unwrap());
    assert!(credentials::save(&path, &profile).is_err());
    let stream_path = run.join(format!("{label}.credential:alternate"));
    assert!(credentials::save(&stream_path, &profile).is_err());
    assert_eq!(before, Sha256::digest(fs::read(path).unwrap()));
    (profile, token)
}
fn write_request(namespace: &str, id: &str, key: &str, revision: Option<u32>) -> WriteRequest {
    WriteRequest {
        record: RecordInput {
            namespace: namespace.into(),
            id: id.into(),
            kind: Kind::Roadmap,
            body: BODY.into(),
            state: State::Proposed,
            sources: vec![SourceReference {
                reference: "https://unopened.invalid/synthetic-source".into(),
                label: "opaque source".into(),
            }],
            tags: vec!["東京".into()],
        },
        expected_revision: revision,
        idempotency_key: key.into(),
    }
}

fn load_record(index: usize, revision: u32) -> WriteRequest {
    let namespace = format!("load/ns{:02}", index / 1000);
    let id = format!("item-{index:05}");
    let mut request = write_request(
        &namespace,
        &id,
        &format!("load-{index}-{revision}"),
        if revision == 1 {
            None
        } else {
            Some(revision - 1)
        },
    );
    let size = 1024 + (index % 4) * 768;
    let prefix = format!(
        "{BODY} loadrecord{index:05} topic{:02} revision{revision} ",
        index % 25
    );
    request.record.body = prefix + &" meadow".repeat(size / 7 + 1);
    request.record.body.truncate(size);
    request.record.sources[0].reference = format!("synthetic:corpus/{index}");
    request.record.tags = vec![format!("topic{:02}", index % 25)];
    request
}

async fn load_identity(run: &Path, label: &str) -> CredentialProfile {
    let reply = owner::admin(
        &run.join("vault"),
        &AdminRequest::Issue(NewClient {
            label: label.into(),
            role: Role::Contributor,
            namespaces: (0..10).map(|i| format!("load/ns{i:02}")).collect(),
        }),
    )
    .await
    .unwrap();
    assert!(reply.error.is_none());
    let profile: CredentialProfile = serde_json::from_value(reply.data.unwrap()).unwrap();
    credentials::save(&run.join(format!("{label}.credential")), &profile).unwrap();
    profile
}

#[derive(Default)]
struct LoadSamples {
    writes: Vec<u64>,
    search: Vec<u64>,
    reads: Vec<u64>,
    errors: u64,
    violations: u64,
}

fn percentiles(samples: &mut [u64]) -> Value {
    samples.sort_unstable();
    let at = |percent: usize| {
        if samples.is_empty() {
            0.0
        } else {
            samples[(samples.len() * percent)
                .div_ceil(100)
                .saturating_sub(1)
                .min(samples.len() - 1)] as f64
                / 1000.0
        }
    };
    json!({"count":samples.len(),"p50_ms":at(50),"p95_ms":at(95),"p99_ms":at(99),"max_ms":samples.last().copied().unwrap_or(0) as f64/1000.0})
}

/// The full 900-second workload is mandatory in HOTR-09's registered gate.
#[tokio::test(flavor = "current_thread")]
#[ignore = "15-minute 10k-record prototype load; invoke through cargo xtask verify --prompt HOTR-09"]
async fn prototype_10k_load_15_minutes() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use tokio::{
        task::JoinSet,
        time::{Instant as TokioInstant, sleep_until},
    };
    let run = run_dir();
    write_new(&run.join("HOTR-09-load-start.json"),&serde_json::to_vec_pretty(&json!({"prompt":"HOTR-09","seed":47821,"records":10000,"namespaces":10,"clients":8,"rate_per_second":20,"duration_seconds":900,"body_bytes":[1024,1792,2560,3328]})).unwrap());
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "prototype-load");
    unlock(&run).await;
    let seeder = load_identity(&run, "seeder").await;
    let seed_start = Instant::now();
    let mut seeds = JoinSet::new();
    for worker in 0..4 {
        let profile = seeder.clone();
        seeds.spawn(async move {
            for index in (worker..10000).step_by(4) {
                let request = serde_json::to_value(load_record(index, 1)).unwrap();
                let response = api::scoped_request(&profile, "POST", "/v1/records", Some(&request))
                    .await
                    .unwrap();
                assert_eq!(response.0, 200);
                assert_eq!(response.1["receipt"]["revision"], 1);
            }
        });
    }
    timeout(Duration::from_secs(600), async {
        while let Some(done) = seeds.join_next().await {
            done.unwrap();
        }
    })
    .await
    .unwrap();
    let seed_ms = seed_start.elapsed().as_millis();
    let mut identities = Vec::new();
    for worker in 0..8 {
        identities.push(load_identity(&run, &format!("load-client-{worker}")).await);
    }
    for namespace in 0..10 {
        let response = api::scoped_request(
            &seeder,
            "POST",
            "/v1/records/count",
            Some(&json!({"namespace":format!("load/ns{namespace:02}")})),
        )
        .await
        .unwrap();
        assert_eq!(response.0, 200);
        assert_eq!(response.1["count"], 1000);
    }
    let cold = Instant::now();
    let first = api::scoped_request(
        &seeder,
        "POST",
        "/v1/search",
        Some(&json!({"page":page("load/ns00",10,0),"query":"topic00"})),
    )
    .await
    .unwrap();
    let first_query_ms = cold.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(first.0, 200);
    assert_eq!(first.1["total"], 40);
    let completed = Arc::new(AtomicUsize::new(0));
    let stopped = Arc::new(AtomicBool::new(false));
    let start = TokioInstant::now();
    let mut progress = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(run.join("HOTR-09-load-progress.jsonl"))
        .unwrap();
    let count = completed.clone();
    let reporter_stop = stopped.clone();
    let reporter = tokio::spawn(async move {
        for seconds in 0..=900 {
            sleep_until(start + Duration::from_secs(seconds)).await;
            let stopping = reporter_stop.load(Ordering::SeqCst);
            if seconds % 60 == 0 || stopping {
                writeln!(progress,"{}",json!({"elapsed_seconds":seconds,"completed":count.load(Ordering::SeqCst),"planned_requests":18000,"phase":"mixed-load","aborting":stopping})).unwrap();
                progress.flush().unwrap();
            }
            if stopping {
                break;
            }
        }
        progress.sync_all().unwrap();
    });
    let mut workers = JoinSet::new();
    for (worker, profile) in identities.iter().cloned().enumerate() {
        let completed = completed.clone();
        let stopped = stopped.clone();
        workers.spawn(async move {
            let mut result=LoadSamples::default();
            let mut revisions=vec![1u32;1250];
            for sequence in 0..2250usize {
                let scheduled=start+Duration::from_millis((sequence*400+worker*50) as u64);
                sleep_until(scheduled).await;
                if stopped.load(Ordering::SeqCst) {break;}
                // Each five-operation group writes, then searches/reads the
                // updated group member. Reads therefore exercise revised data.
                let local=(sequence%1250)/5*5;
                let index=worker*1250+local;
                let namespace=format!("load/ns{:02}",index/1000);
                let id=format!("item-{index:05}");
                let kind=sequence%5;
                let (endpoint,body)=match kind {
                    0=>("/v1/records",serde_json::to_value(load_record(index,revisions[local]+1)).unwrap()),
                    1|2=>("/v1/search",json!({"page":page(&namespace,10,0),"query":format!("topic{:02}",index%25)})),
                    3=>("/v1/records/get",json!({"namespace":namespace,"id":id})),
                    _=>("/v1/records/count",json!({"namespace":namespace})),
                };
                let response=api::scoped_request(&profile,"POST",endpoint,Some(&body)).await;
                let elapsed=TokioInstant::now().duration_since(scheduled).as_micros() as u64;
                match kind {0=>result.writes.push(elapsed),1|2=>result.search.push(elapsed),_=>result.reads.push(elapsed)}
                completed.fetch_add(1,Ordering::SeqCst);
                match response {
                    Ok((200,value))=>{
                        let valid=match kind {
                            0=>{let valid=value["receipt"]["namespace"]==namespace && value["receipt"]["id"]==id && value["receipt"]["revision"]==revisions[local]+1;if valid {revisions[local]+=1;}valid}
                            1|2=>value["total"]==40 && value["records"].as_array().is_some_and(|rows|!rows.is_empty() && rows.iter().all(|r|r["namespace"]==namespace && r["revision"].as_u64().is_some_and(|revision|r["body"].as_str().is_some_and(|b|b.contains(&format!("revision{revision} ")))) && !r["sources"].as_array().unwrap().is_empty())),
                            3=>value["namespace"]==namespace && value["id"]==id && value["revision"]==revisions[local],
                            _=>value["count"]==1000,
                        };
                        if !valid {result.violations+=1;stopped.store(true,Ordering::SeqCst);break;}
                    }
                    _=>{result.errors+=1;stopped.store(true,Ordering::SeqCst);break;}
                }
            }
            (worker,result,revisions)
        });
    }
    let mut all = LoadSamples::default();
    let mut expected = vec![1u32; 10000];
    while let Some(done) = workers.join_next().await {
        let (worker, mut result, revisions) = done.unwrap();
        all.writes.append(&mut result.writes);
        all.search.append(&mut result.search);
        all.reads.append(&mut result.reads);
        all.errors += result.errors;
        all.violations += result.violations;
        expected[worker * 1250..(worker + 1) * 1250].copy_from_slice(&revisions);
    }
    reporter.await.unwrap();
    let duration_seconds = start.elapsed().as_secs_f64();
    server.stop(&run).await;
    let db = hotr::schema::open(&run.join("vault/vault.db"), KEY).unwrap();
    let mut ack_mismatches = 0usize;
    for (index, revision) in expected.iter().enumerate() {
        let record = hotr::schema::revision(
            &db,
            &format!("load/ns{:02}", index / 1000),
            &format!("item-{index:05}"),
            None,
        )
        .unwrap()
        .unwrap();
        if record.revision != *revision {
            ack_mismatches += 1;
        }
    }
    let receipts: i64 = db
        .query_row("SELECT count(*) FROM write_receipts", [], |r| r.get(0))
        .unwrap();
    let record_count: i64 = db
        .query_row("SELECT count(*) FROM records", [], |r| r.get(0))
        .unwrap();
    let integrity = db
        .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
        .unwrap()
        == "ok";
    let fts_integrity = db
        .execute(
            "INSERT INTO record_fts(record_fts) VALUES('integrity-check')",
            [],
        )
        .is_ok();
    drop(db);
    let write_stats = percentiles(&mut all.writes);
    let search_stats = percentiles(&mut all.search);
    let read_stats = percentiles(&mut all.reads);
    let pass = completed.load(Ordering::SeqCst) == 18000
        && all.errors == 0
        && all.violations == 0
        && ack_mismatches == 0
        && receipts == 13600
        && record_count == 10000
        && integrity
        && fts_integrity
        && write_stats["p95_ms"].as_f64().unwrap() <= 500.0
        && search_stats["p95_ms"].as_f64().unwrap() <= 500.0
        && duration_seconds >= 900.0;
    let report = json!({"prompt":"HOTR-09","result":if pass{"PASS"}else{"FAIL"},"seed":47821,"records":record_count,"namespaces":10,"clients":8,"duration_seconds":duration_seconds,"scheduled_requests":18000,"completed_requests":completed.load(Ordering::SeqCst),"unexpected_errors":all.errors,"correctness_violations":all.violations,"acknowledgment_mismatches":ack_mismatches,"durable_receipts":receipts,"seed_elapsed_ms":seed_ms,"first_query_after_seed_ms":first_query_ms,"first_query_is_os_cache_cold":false,"latency_policy":"scheduled arrival through complete response, including client scheduling delay; failures included","write":write_stats,"keyword":search_stats,"other_reads":read_stats,"integrity":integrity,"fts_integrity":fts_integrity,"binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap()))});
    write_new(
        &run.join("HOTR-09-load-result.json"),
        &serde_json::to_vec_pretty(&report).unwrap(),
    );
    let tokens: Vec<_> = identities
        .iter()
        .chain(std::iter::once(&seeder))
        .map(|p| credentials::unprotect(p).unwrap())
        .collect();
    scan(&run, &tokens.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    println!(
        "HOTR-09 load result: {}",
        if pass { "PASS" } else { "FAIL" }
    );
    assert!(
        pass,
        "prototype target failed; inspect the retained numerical report"
    );
}
async fn post(
    client: &reqwest::Client,
    port: u16,
    token: &str,
    path: &str,
    body: &impl serde::Serialize,
) -> (u16, Value) {
    let response = client
        .post(format!("http://127.0.0.1:{port}{path}"))
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    assert!(
        !response
            .headers()
            .contains_key("access-control-allow-origin")
    );
    let bytes = response.bytes().await.unwrap();
    assert!(bytes.len() <= api::MAX_RESPONSE);
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn get_status(client: &reqwest::Client, port: u16, token: &str) -> u16 {
    client
        .get(format!("http://127.0.0.1:{port}/v1/status"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16()
}
async fn raw_response(stream: &mut TcpStream) -> (u16, Vec<u8>) {
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        assert!(headers.len() < 8192);
        headers.push(
            timeout(Duration::from_secs(13), stream.read_u8())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    let text = std::str::from_utf8(&headers).unwrap();
    let code = text.split_whitespace().nth(1).unwrap().parse().unwrap();
    let length: usize = text
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse().unwrap())
        })
        .unwrap();
    assert!(length <= api::MAX_RESPONSE);
    let mut body = vec![0; length];
    timeout(Duration::from_secs(3), stream.read_exact(&mut body))
        .await
        .unwrap()
        .unwrap();
    (code, body)
}
async fn raw_status(stream: &mut TcpStream, port: u16, token: &str) -> u16 {
    let request = Zeroizing::new(format!(
        "GET /v1/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\r\n"
    ));
    stream.write_all(request.as_bytes()).await.unwrap();
    raw_response(stream).await.0
}
fn scan(run: &Path, tokens: &[&str]) {
    fn visit(path: &Path, tokens: &[&str]) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            assert!(!entry.file_type().unwrap().is_symlink());
            if entry.file_type().unwrap().is_dir() {
                visit(&entry.path(), tokens);
            } else {
                let bytes = fs::read(entry.path()).unwrap();
                for secret in tokens {
                    assert!(
                        !bytes
                            .windows(secret.len())
                            .any(|part| part == secret.as_bytes()),
                        "plaintext credential in managed fixture"
                    );
                }
                for secret in [KEY, BODY.as_bytes()] {
                    assert!(
                        !bytes.windows(secret.len()).any(|part| part == secret),
                        "plaintext vault canary in managed fixture"
                    );
                }
            }
        }
    }
    visit(run, tokens);
}

#[tokio::test(flavor = "current_thread")]
async fn actual_service_role_namespace_revision_and_revocation_matrix() {
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "matrix");
    let client = local_client();
    let fake = "1".repeat(64);
    assert_eq!(get_status(&client, server.port, &fake).await, 503);
    let locked = owner::admin(
        &run.join("vault"),
        &AdminRequest::Issue(NewClient {
            label: "locked".into(),
            role: Role::Reader,
            namespaces: vec!["alpha".into()],
        }),
    )
    .await
    .unwrap();
    assert!(locked.error.is_some());
    unlock(&run).await;
    let (a, token_a) = issue_cli(&run, "contributor-a", "contributor", "alpha");
    let (b, token_b) = issue_cli(&run, "reader-b", "reader", "alpha");
    let (_, token_c) = issue_cli(&run, "contributor-c", "contributor", "beta");
    assert!(token_a.as_str() != token_b.as_str() && token_a.as_str() != token_c.as_str());
    assert_eq!(a.port, server.port);
    assert_eq!(b.port, server.port);
    let cli_status = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .args(["request", "--credential"])
        .arg(run.join("reader-b.credential"))
        .output()
        .unwrap();
    assert!(cli_status.status.success());
    assert!(cli_status.stderr.is_empty());
    let cli_reply: Value = serde_json::from_slice(&cli_status.stdout).unwrap();
    assert_eq!(cli_reply["client_id"], b.client_id);
    assert!(
        !cli_status
            .stdout
            .windows(token_b.len())
            .any(|bytes| bytes == token_b.as_bytes())
    );
    assert_eq!(
        api::scoped_request(&a, "GET", "/v1/status", None)
            .await
            .unwrap()
            .0,
        200
    );
    for token in [&*token_a, &*token_b, &*token_c] {
        assert_eq!(get_status(&client, server.port, token).await, 200);
    }
    assert_eq!(get_status(&client, server.port, &fake).await, 401);
    let original = write_request("alpha", "shared", "create", None);
    assert_eq!(
        post(&client, server.port, &token_b, "/v1/records", &original)
            .await
            .0,
        403
    );
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/records",
            &write_request("beta", "forbidden", "forbidden", None)
        )
        .await
        .0,
        403
    );
    let first = api::scoped_request(
        &a,
        "POST",
        "/v1/records",
        Some(&serde_json::to_value(&original).unwrap()),
    )
    .await
    .unwrap();
    assert_eq!(first.0, 200);
    assert_eq!(first.1["receipt"]["revision"], 1);
    assert_eq!(
        post(
            &client,
            server.port,
            &token_c,
            "/v1/records",
            &write_request("beta", "hidden", "hidden", None)
        )
        .await
        .0,
        200
    );
    let query = json!({"namespace":"alpha","id":"shared"});
    assert_eq!(
        api::scoped_request(&b, "POST", "/v1/records/get", Some(&query))
            .await
            .unwrap()
            .0,
        200
    );
    for token in [&*token_a, &*token_b] {
        let response = post(&client, server.port, token, "/v1/records/get", &query).await;
        assert_eq!(response.0, 200);
        assert!(response.1["body"].as_str() == Some(BODY));
        assert_eq!(response.1["revision"], 1);
        assert!(
            response.1["sources"][0]["reference"].as_str()
                == Some("https://unopened.invalid/synthetic-source")
        );
    }
    for token in [&*token_a, &*token_b] {
        for revision in [None, Some(1)] {
            assert_eq!(
                post(
                    &client,
                    server.port,
                    token,
                    "/v1/records/get",
                    &json!({"namespace":"beta","id":"hidden","revision":revision})
                )
                .await
                .0,
                403
            );
        }
    }
    assert_eq!(
        post(&client, server.port, &token_c, "/v1/records/get", &query)
            .await
            .0,
        403
    );
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/records/get",
            &json!({"namespace":"alpha","id":"missing"})
        )
        .await
        .0,
        404
    );
    let mut forged = serde_json::to_value(&original).unwrap();
    forged["principal"] = json!("owner");
    assert_eq!(
        post(&client, server.port, &token_a, "/v1/records", &forged)
            .await
            .0,
        400
    );
    let mut accepted = write_request("alpha", "accepted-spoof", "accept-spoof", None);
    accepted.record.state = State::Accepted;
    assert_eq!(
        post(&client, server.port, &token_a, "/v1/records", &accepted)
            .await
            .0,
        403
    );
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/admin/issue",
            &json!({})
        )
        .await
        .0,
        404
    );
    let updated = post(
        &client,
        server.port,
        &token_a,
        "/v1/records",
        &write_request("alpha", "shared", "update", Some(1)),
    )
    .await;
    assert_eq!(updated.0, 200);
    assert_eq!(updated.1["receipt"]["revision"], 2);
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/records",
            &write_request("alpha", "shared", "stale", Some(1))
        )
        .await
        .0,
        409
    );
    let accepted = owner::admin(
        &run.join("vault"),
        &AdminRequest::Accept(Accept {
            namespace: "alpha".into(),
            id: "shared".into(),
            expected_revision: 2,
            idempotency_key: "owner-accept".into(),
        }),
    )
    .await
    .unwrap();
    assert!(accepted.error.is_none());
    assert_eq!(accepted.data.unwrap()["receipt"]["revision"], 3);
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/records",
            &write_request("alpha", "shared", "silent-overwrite", Some(3))
        )
        .await
        .0,
        403
    );
    let replay = post(&client, server.port, &token_a, "/v1/records", &original).await;
    assert_eq!(replay, first, "receipt changed after owner acceptance");
    let current = post(&client, server.port, &token_b, "/v1/records/get", &query).await;
    assert_eq!(current.1["revision"], 3);
    assert_eq!(current.1["state"], "accepted");
    let old = post(
        &client,
        server.port,
        &token_b,
        "/v1/records/get",
        &json!({"namespace":"alpha","id":"shared","revision":1}),
    )
    .await;
    assert_eq!(old.0, 200);
    assert_eq!(old.1["revision"], 1);
    let mut established = TcpStream::connect(("127.0.0.1", server.port))
        .await
        .unwrap();
    let local = established.local_addr().unwrap();
    assert_eq!(
        raw_status(&mut established, server.port, &token_a).await,
        200
    );
    let revoked = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .arg("revoke")
        .arg(run.join("vault"))
        .arg(&a.client_id)
        .output()
        .unwrap();
    assert!(revoked.status.success());
    assert_eq!(
        raw_status(&mut established, server.port, &token_a).await,
        401
    );
    assert_eq!(established.local_addr().unwrap(), local);
    drop(established);
    assert_eq!(
        post(&client, server.port, &token_a, "/v1/records", &original)
            .await
            .0,
        401
    );
    assert_eq!(get_status(&client, server.port, &token_b).await, 200);
    server.stop(&run).await;
    let mut restart = Server::start(&run, "restart");
    unlock(&run).await;
    assert_eq!(get_status(&client, restart.port, &token_a).await, 401);
    assert_eq!(get_status(&client, restart.port, &token_b).await, 200);
    let restored = post(&client, restart.port, &token_b, "/v1/records/get", &query).await;
    assert_eq!(restored.1["revision"], 3);
    restart.stop(&run).await;
    scan(&run, &[&token_a, &token_b, &token_c]);
    write_new(&run.join("matrix.json"),&serde_json::to_vec_pretty(&json!({"result":"PASS","binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),"independent_clients":3,"same_connection_revocation":true,"accepted_revision":3,"restart_persisted":true,"raw_tokens_in_storage_or_logs":false})).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn actual_http_limits_origins_deadlines_and_overload() {
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "http");
    unlock(&run).await;
    let (_, token) = issue_cli(&run, "http-client", "contributor", "alpha");
    let client = local_client();
    let url = format!("http://127.0.0.1:{}/v1/status", server.port);
    for host in ["evil.invalid", "localhost:1", "127.0.0.1.evil.invalid"] {
        let response = client
            .get(&url)
            .bearer_auth(&*token)
            .header("host", host)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 403);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    for origin in [
        "https://evil.invalid",
        "null",
        &format!("http://127.0.0.1:{}", server.port),
    ] {
        assert_eq!(
            client
                .get(&url)
                .bearer_auth(&*token)
                .header("origin", origin)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            403
        );
    }
    let endpoint = format!("http://127.0.0.1:{}/v1/records", server.port);
    for malformed in [
        "{",
        "[]",
        "{\"record\":{}}",
        &format!("{}0{}", "[".repeat(33), "]".repeat(33)),
    ] {
        assert_eq!(
            client
                .post(&endpoint)
                .bearer_auth(&*token)
                .header("content-type", "application/json")
                .body(malformed.to_owned())
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            400
        );
    }
    // Read the early refusal before uploading a rejected body. A full reqwest
    // upload races Windows' connection reset against the already-issued 413.
    let mut oversized_stream = TcpStream::connect(("127.0.0.1", server.port))
        .await
        .unwrap();
    let headers = Zeroizing::new(format!(
        "POST /v1/records HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        server.port,
        token.as_str(),
        api::MAX_REQUEST + 1
    ));
    oversized_stream
        .write_all(headers.as_bytes())
        .await
        .unwrap();
    assert_eq!(raw_response(&mut oversized_stream).await.0, 413);
    drop(oversized_stream);
    assert_eq!(
        client
            .post(&endpoint)
            .bearer_auth(&*token)
            .body("{}")
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        415
    );
    let mut oversized = write_request("alpha", "oversized", "oversized", None);
    oversized.record.body = "x".repeat(65537);
    assert_eq!(
        post(&client, server.port, &token, "/v1/records", &oversized)
            .await
            .0,
        400
    );
    let mut slow_headers = TcpStream::connect(("127.0.0.1", server.port))
        .await
        .unwrap();
    slow_headers
        .write_all(b"GET /v1/status HTTP/1.1\r\nHost: ")
        .await
        .unwrap();
    let mut result = Vec::new();
    timeout(
        Duration::from_secs(8),
        slow_headers.read_to_end(&mut result),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(result.is_empty() || result.starts_with(b"HTTP/1.1 408"));
    let partial = Zeroizing::new(format!(
        "POST /v1/records HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 1000\r\n\r\n{{",
        server.port,
        token.as_str()
    ));
    let mut slow_body = TcpStream::connect(("127.0.0.1", server.port))
        .await
        .unwrap();
    slow_body.write_all(partial.as_bytes()).await.unwrap();
    let start = Instant::now();
    assert_eq!(raw_response(&mut slow_body).await.0, 504);
    assert!(start.elapsed() < Duration::from_secs(13));
    drop(slow_body);
    let mut pending = Vec::new();
    for _ in 0..api::MAX_ACTIVE_REQUESTS {
        let mut stream = TcpStream::connect(("127.0.0.1", server.port))
            .await
            .unwrap();
        stream.write_all(partial.as_bytes()).await.unwrap();
        pending.push(stream);
    }
    let start = Instant::now();
    loop {
        if get_status(&client, server.port, &token).await == 429 {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(3));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    drop(pending);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(get_status(&client, server.port, &token).await, 200);
    let mut held = Vec::new();
    for _ in 0..api::MAX_CONNECTIONS {
        held.push(
            TcpStream::connect(("127.0.0.1", server.port))
                .await
                .unwrap(),
        );
    }
    let mut excess = TcpStream::connect(("127.0.0.1", server.port))
        .await
        .unwrap();
    let overflow = raw_response(&mut excess).await;
    assert_eq!(overflow.0, 503);
    drop(held);
    drop(excess);
    server.stop(&run).await;
    scan(&run, &[&token]);
    write_new(&run.join("http-limits.json"),b"{\"result\":\"PASS\",\"active_requests\":64,\"connections\":128,\"header_timeout_seconds\":5,\"request_timeout_seconds\":10,\"origin_host_denials\":true,\"json_depth\":32,\"overload_recovered\":true}");
}
