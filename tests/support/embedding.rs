use super::*;
use hotr::embedding::Configure;
use std::io::Read;
use std::net::{Ipv4Addr, TcpListener};
use std::os::windows::process::CommandExt;

pub(super) async fn status(run: &Path) -> Value {
    let reply = owner::admin(&run.join("vault"), &AdminRequest::EmbeddingStatus)
        .await
        .unwrap();
    assert!(reply.error.is_none());
    reply.data.unwrap()
}
pub(super) fn configure_cli(run: &Path, port: Option<u16>, generation: u32) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hotr"));
    command
        .arg("embedding-configure")
        .arg(run.join("vault"))
        .args(["--expected-generation", &generation.to_string()]);
    if let Some(port) = port {
        command.args(["--port", &port.to_string()]);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "owner embedding configuration failed"
    );
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["data"].clone()
}
pub(super) async fn wait_count(run: &Path, key: &str, count: u64, seconds: u64) -> Value {
    let start = Instant::now();
    loop {
        let value = status(run).await;
        if value[key] == count {
            return value;
        }
        assert!(
            start.elapsed() < Duration::from_secs(seconds),
            "embedding count deadline: {value}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
fn unused_port() -> u16 {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test(flavor = "current_thread")]
async fn actual_owner_model_down_keeps_writes_and_lexical_available() {
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "embedding-down");
    unlock(&run).await;
    assert_eq!(status(&run).await["enabled"], false);
    let (_, token) = issue_cli(&run, "embedding-down-writer", "contributor", "alpha");
    let client = local_client();
    let port = unused_port();
    assert_eq!(configure_cli(&run, Some(port), 0)["generation"], 1);
    let stale = owner::admin(
        &run.join("vault"),
        &AdminRequest::EmbeddingConfigure(Configure {
            port: Some(port),
            expected_generation: 0,
        }),
    )
    .await
    .unwrap();
    assert!(stale.error.is_some());
    for id in ["first", "second"] {
        assert_eq!(
            post(
                &client,
                server.port,
                &token,
                "/v1/records",
                &write_request("alpha", id, id, None)
            )
            .await
            .0,
            200
        );
    }
    let failed = wait_count(&run, "failed", 2, 35).await;
    assert_eq!(failed["indexed"], 0);
    assert!(failed["last_error"].is_string());
    let lexical = post(
        &client,
        server.port,
        &token,
        "/v1/search",
        &json!({"page":page("alpha",10,0),"query":"roadmap"}),
    )
    .await;
    assert_eq!(lexical.0, 200);
    assert_eq!(lexical.1["total"], 2);
    server.stop(&run).await;
    let db = hotr::schema::open(&run.join("vault/vault.db"), KEY).unwrap();
    assert_eq!(
        db.query_row("SELECT sum(attempts) FROM embedding_index", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        6
    );
    drop(db);
    let mut server = Server::start(&run, "embedding-down-restart");
    unlock(&run).await;
    assert_eq!(status(&run).await["failed"], 2);
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(status(&run).await["failed"], 2);
    configure_cli(&run, None, 1);
    assert_eq!(status(&run).await["enabled"], false);
    server.stop(&run).await;
    scan(&run, &[&token]);
    write_new(&run.join("HOTR-15-model-down.json"),&serde_json::to_vec_pretty(&json!({"result":"PASS","binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),"writes":2,"lexical_matches":2,"max_attempts_per_revision":3,"retry_exhaustion_persisted":true,"configuration_cas":true})).unwrap());
}

pub(super) struct Ollama {
    pub(super) child: Child,
    pub(super) port: u16,
}
impl Drop for Ollama {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
impl Ollama {
    pub(super) async fn start(run: &Path) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let models = owner_path(&root.join("work/hotr-models"));
        let manifest = models.join("manifests/registry.ollama.ai/library/nomic-embed-text/v1.5");
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(
                    fs::read(&manifest).expect("run .cargo/prepare-embedding.ps1 first")
                )
            ),
            hotr::embedding_transport::MODEL_DIGEST
        );
        let model_manifest: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
        for blob in std::iter::once(&model_manifest["config"])
            .chain(model_manifest["layers"].as_array().unwrap())
        {
            let digest = blob["digest"].as_str().unwrap();
            let path = owner_path(&models.join("blobs").join(digest.replace(':', "-")));
            assert_eq!(
                fs::metadata(&path).unwrap().len(),
                blob["size"].as_u64().unwrap()
            );
            let mut file = fs::File::open(path).unwrap();
            let mut hash = Sha256::new();
            let mut buffer = vec![0u8; 1024 * 1024];
            loop {
                let count = file.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
            assert_eq!(format!("sha256:{:x}", hash.finalize()), digest);
        }
        let executable = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
            .join("Programs/Ollama/ollama.exe");
        assert!(executable.is_file(), "installed Ollama required");
        // Ollama's existing identity stays in place; this fixture never creates it.
        assert!(
            PathBuf::from(std::env::var_os("USERPROFILE").unwrap())
                .join(".ollama/id_ed25519")
                .is_file()
        );
        let version = Command::new(&executable).arg("--version").output().unwrap();
        let version_text = format!(
            "{}{}",
            String::from_utf8_lossy(&version.stdout),
            String::from_utf8_lossy(&version.stderr)
        );
        assert!(
            version_text.contains("0.32.6"),
            "installed Ollama version changed; requalify fixture"
        );
        let port = unused_port();
        let temp = run.join("ollama-temp");
        fs::create_dir(&temp).unwrap();
        let output = |name: &str| {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(run.join(name))
                .unwrap()
        };
        let child = Command::new(&executable)
            .arg("serve")
            .creation_flags(0x08000000)
            .env("OLLAMA_HOST", format!("127.0.0.1:{port}"))
            .env("OLLAMA_MODELS", &models)
            .env("OLLAMA_NO_CLOUD", "true")
            .env("OLLAMA_DEBUG", "false")
            .env("OLLAMA_DEBUG_LOG_REQUESTS", "false")
            .env("OLLAMA_NOPRUNE", "true")
            .env("OLLAMA_NOHISTORY", "true")
            .env("OLLAMA_NUM_PARALLEL", "1")
            .env("OLLAMA_MAX_LOADED_MODELS", "1")
            .env("OLLAMA_MAX_QUEUE", "4")
            .env("OLLAMA_LOAD_TIMEOUT", "30s")
            .env("CUDA_VISIBLE_DEVICES", "-1")
            .env("TEMP", &temp)
            .env("TMP", &temp)
            .stdin(Stdio::null())
            .stdout(output("ollama.stdout.txt"))
            .stderr(output("ollama.stderr.txt"))
            .spawn()
            .unwrap();
        let mut service = Self { child, port };
        let start = Instant::now();
        loop {
            assert!(
                service.child.try_wait().unwrap().is_none(),
                "owned Ollama exited; inspect retained logs"
            );
            if local_client()
                .get(format!("http://127.0.0.1:{port}/api/tags"))
                .send()
                .await
                .is_ok()
            {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(20),
                "owned Ollama readiness timeout"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        service
    }
}
fn owner_path(path: &Path) -> PathBuf {
    use std::os::windows::fs::MetadataExt;
    for ancestor in path.ancestors() {
        if let Ok(metadata) = fs::symlink_metadata(ancestor) {
            assert_eq!(
                metadata.file_attributes() & 0x400,
                0,
                "model reparse point refused"
            );
        }
    }
    let path = path.canonicalize().unwrap();
    assert!(
        path.starts_with(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .canonicalize()
                .unwrap()
        )
    );
    path
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires installed Ollama and pinned project model; use HOTR-15 bounded gate"]
async fn actual_pinned_ollama_index_resume() {
    let run = run_dir();
    let ollama = Ollama::start(&run).await;
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "embedding-live");
    unlock(&run).await;
    let (_, token) = issue_cli(&run, "embedding-live-writer", "contributor", "alpha");
    let client = local_client();
    for id in ["shared", "independent"] {
        assert_eq!(
            post(
                &client,
                server.port,
                &token,
                "/v1/records",
                &write_request("alpha", id, id, None)
            )
            .await
            .0,
            200
        );
    }
    configure_cli(&run, Some(ollama.port), 0);
    let indexed = wait_count(&run, "indexed", 2, 120).await;
    assert_eq!(indexed["last_peer"], format!("127.0.0.1:{}", ollama.port));
    assert_eq!(indexed["failed"], 0);
    let backup = run.join("embedding-backup");
    assert!(
        owner::backup(&run.join("vault"), &backup, KEY)
            .await
            .unwrap()
            .error
            .is_none()
    );
    server.stop(&run).await;
    let db = hotr::schema::open(&run.join("vault/vault.db"), KEY).unwrap();
    let before = db
        .prepare(
            "SELECT record_id,revision,attempts,vector FROM embedding_index ORDER BY record_id",
        )
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, u32>(1)?,
                r.get::<_, u32>(2)?,
                r.get::<_, Vec<u8>>(3)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(before.len(), 2);
    for (_, _, attempts, vector) in &before {
        assert_eq!(*attempts, 1);
        assert_eq!(vector.len(), 768 * 4);
        assert!(
            vector
                .as_chunks::<4>()
                .0
                .iter()
                .all(|v| f32::from_le_bytes(*v).is_finite())
        );
    }
    drop(db);
    let mut server = Server::start(&run, "embedding-live-resume");
    unlock(&run).await;
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(status(&run).await["indexed"], 2);
    let mut revised = write_request("alpha", "shared", "revised", Some(1));
    revised.record.body = "HOTR07canary revised context for the encrypted vault".into();
    assert_eq!(
        post(&client, server.port, &token, "/v1/records", &revised)
            .await
            .0,
        200
    );
    wait_count(&run, "indexed", 2, 60).await;
    server.stop(&run).await;
    let db = hotr::schema::open(&run.join("vault/vault.db"), KEY).unwrap();
    let after: Vec<(String, u32, u32, Vec<u8>)> = db
        .prepare(
            "SELECT record_id,revision,attempts,vector FROM current_embeddings ORDER BY record_id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(after.len(), 2);
    assert_eq!(
        after[0], before[0],
        "unchanged record was reindexed after restart"
    );
    assert_eq!(after[1].1, 2);
    assert_ne!(after[1].3, before[1].3);
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    drop(db);
    let restored_root = run.join("restored-index");
    fs::create_dir(&restored_root).unwrap();
    hotr::backup::restore(&backup, &restored_root.join("vault"), KEY).unwrap();
    let mut restored = Server::start(&restored_root, "restored-disabled");
    unlock(&restored_root).await;
    assert_eq!(status(&restored_root).await["enabled"], false);
    assert_eq!(status(&restored_root).await["indexed"], 0);
    let (_, restored_token) = issue_cli(&restored_root, "restored-reader", "reader", "alpha");
    assert_eq!(
        post(
            &client,
            restored.port,
            &restored_token,
            "/v1/search",
            &json!({"page":page("alpha",10,0),"query":"roadmap"})
        )
        .await
        .1["total"],
        2
    );
    restored.stop(&restored_root).await;
    // Ask only this owned server to unload its test model before process retirement.
    let _ = client
        .post(format!("http://127.0.0.1:{}/api/embed", ollama.port))
        .json(&json!({"model":hotr::embedding_transport::MODEL,"input":[],"keep_alive":0}))
        .send()
        .await;
    let pid = ollama.child.id();
    drop(ollama);
    scan(&run, &[&token]);
    write_new(&run.join("HOTR-15-local-embedding.json"),&serde_json::to_vec_pretty(&json!({"result":"PASS","binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),"ollama_version":"0.32.6","owned_ollama_pid":pid,"observed_peer":indexed["last_peer"],"model":hotr::embedding_transport::MODEL,"model_digest":hotr::embedding_transport::MODEL_DIGEST,"dimensions":768,"indexed_records":2,"restart_did_not_reindex":true,"revision_replaced":true,"model_blobs_verified":true,"project_model_cache":true,"restored_indexing_disabled":true})).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn actual_owner_disable_cancels_inflight_embedding_connection() {
    let run = run_dir();
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "embedding-cancel");
    unlock(&run).await;
    let (_, token) = issue_cli(&run, "embedding-cancel-writer", "contributor", "alpha");
    assert_eq!(
        post(
            &local_client(),
            server.port,
            &token,
            "/v1/records",
            &write_request("alpha", "cancel", "cancel", None)
        )
        .await
        .0,
        200
    );
    configure_cli(&run, Some(port), 0);
    let mut inference = None;
    for endpoint in ["/api/tags", "/api/show", "/api/embed"] {
        let (mut stream, peer) = timeout(Duration::from_secs(10), listener.accept())
            .await
            .unwrap()
            .unwrap();
        assert!(peer.ip().is_loopback());
        let mut request = Vec::new();
        timeout(Duration::from_secs(5), async {
            let mut buffer = [0u8; 4096];
            loop {
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
                assert!(request.len() <= api::MAX_REQUEST);
                let Some(end) = request.windows(4).position(|b| b == b"\r\n\r\n") else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length = headers
                    .lines()
                    .find_map(|s| s.strip_prefix("content-length:"))
                    .map(|s| s.trim().parse::<usize>().unwrap())
                    .unwrap_or(0);
                assert!(headers.lines().next().unwrap().contains(endpoint));
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        })
        .await
        .unwrap();
        if endpoint == "/api/embed" {
            assert!(request.windows(BODY.len()).any(|s| s == BODY.as_bytes()));
            inference = Some(stream);
            break;
        }
        let body = if endpoint == "/api/tags" {
            json!({"models":[{"name":hotr::embedding_transport::MODEL,"model":hotr::embedding_transport::MODEL,"digest":hotr::embedding_transport::MODEL_DIGEST}]})
        } else {
            json!({"license":"Apache License 2.0","details":{"format":"gguf","family":"nomic-bert"},"model_info":{"general.architecture":"nomic-bert","nomic-bert.context_length":8192,"nomic-bert.embedding_length":768},"capabilities":["embedding"]})
        };
        let bytes = serde_json::to_vec(&body).unwrap();
        stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).await.unwrap();
        stream.write_all(&bytes).await.unwrap();
    }
    let mut inference = inference.unwrap();
    configure_cli(&run, None, 1);
    let mut remainder = Vec::new();
    let closed = timeout(
        Duration::from_secs(3),
        inference.read_to_end(&mut remainder),
    )
    .await
    .unwrap();
    assert!(
        closed.is_ok()
            || closed.as_ref().is_err_and(|e| matches!(
                e.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ))
    );
    assert_eq!(status(&run).await["enabled"], false);
    assert_eq!(status(&run).await["indexed"], 0);
    assert!(
        timeout(Duration::from_millis(400), listener.accept())
            .await
            .is_err()
    );
    server.stop(&run).await;
    scan(&run, &[&token]);
    write_new(&run.join("HOTR-15-cancel.json"),b"{\"result\":\"PASS\",\"actual_owner_disable\":true,\"inflight_connection_closed\":true,\"no_post_disable_request\":true}");
}
