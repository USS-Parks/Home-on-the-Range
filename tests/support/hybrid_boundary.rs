//! HOTR-16 query-vector authorization, cache, and cancellation boundaries.
use super::*;
use hotr::{
    embedding::Configure,
    embedding_transport::{DIMENSIONS, MODEL, MODEL_DIGEST},
    lifecycle::{Action, Request},
};
use std::{
    net::Ipv4Addr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

fn scenario(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    fs::create_dir(&path).unwrap();
    write_new(
        &path.join("SYNTHETIC-ONLY"),
        b"HOTR-16 synthetic query-boundary fixture\n",
    );
    path
}

async fn configure(run: &Path, port: Option<u16>, expected_generation: u32) -> Value {
    let reply = owner::admin(
        &run.join("vault"),
        &AdminRequest::EmbeddingConfigure(Configure {
            port,
            expected_generation,
        }),
    )
    .await
    .unwrap();
    assert!(
        reply.error.is_none(),
        "owner embedding configuration failed"
    );
    reply.data.unwrap()
}

fn query(namespace: &str, text: &str) -> Value {
    json!({"page":page(namespace, 5, 0),"query":text})
}

fn tags() -> Value {
    json!({
        "models": [{"name": MODEL, "model": MODEL, "digest": MODEL_DIGEST}]
    })
}

fn show() -> Value {
    json!({
        "license": "Apache License 2.0",
        "details": {"format": "gguf", "family": "nomic-bert"},
        "model_info": {
            "general.architecture": "nomic-bert",
            "nomic-bert.context_length": 8192,
            "nomic-bert.embedding_length": DIMENSIONS
        },
        "capabilities": ["embedding"]
    })
}

fn embedding() -> Value {
    let mut vector = vec![0.0_f32; DIMENSIONS];
    vector[0] = 1.0;
    json!({"model":MODEL,"embeddings":[vector]})
}

async fn accept_request(listener: &TcpListener, expected_path: &str) -> TcpStream {
    let (mut stream, peer) = timeout(Duration::from_secs(10), listener.accept())
        .await
        .unwrap()
        .unwrap();
    assert!(peer.ip().is_loopback());
    let mut request = Vec::new();
    timeout(Duration::from_secs(5), async {
        let mut buffer = [0_u8; 4096];
        loop {
            let count = stream.read(&mut buffer).await.unwrap();
            assert!(count > 0);
            request.extend_from_slice(&buffer[..count]);
            assert!(request.len() <= api::MAX_REQUEST);
            let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
            let length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|length| length.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= end + 4 + length {
                let path = headers
                    .lines()
                    .next()
                    .and_then(|line| line.split_ascii_whitespace().nth(1));
                assert_eq!(path, Some(expected_path));
                if expected_path == "/api/embed" {
                    assert!(
                        request
                            .windows(b"search_query: ".len())
                            .any(|bytes| bytes == b"search_query: ")
                    );
                }
                break;
            }
        }
    })
    .await
    .unwrap();
    stream
}

async fn send_json(stream: &mut TcpStream, value: &Value) {
    let body = serde_json::to_vec(value).unwrap();
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).await.unwrap();
    stream.write_all(&body).await.unwrap();
}

fn success_adapter(listener: TcpListener, queries: usize) -> (Arc<AtomicUsize>, JoinHandle<()>) {
    let hits = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&hits);
    let task = tokio::spawn(async move {
        for _ in 0..queries {
            for (path, reply) in [
                ("/api/tags", tags()),
                ("/api/show", show()),
                ("/api/embed", embedding()),
                ("/api/tags", tags()),
            ] {
                let mut stream = accept_request(&listener, path).await;
                observed.fetch_add(1, Ordering::SeqCst);
                send_json(&mut stream, &reply).await;
            }
        }
    });
    (hits, task)
}

async fn blocked_embedding(listener: &TcpListener) -> TcpStream {
    for (path, reply) in [("/api/tags", tags()), ("/api/show", show())] {
        let mut stream = accept_request(listener, path).await;
        send_json(&mut stream, &reply).await;
    }
    accept_request(listener, "/api/embed").await
}

async fn complete_embedding(listener: &TcpListener, mut inference: TcpStream) {
    send_json(&mut inference, &embedding()).await;
    drop(inference);
    let mut final_tags = accept_request(listener, "/api/tags").await;
    send_json(&mut final_tags, &tags()).await;
}

fn assert_socket_closed(result: std::io::Result<usize>) {
    assert!(
        result.is_ok()
            || result.as_ref().is_err_and(|error| matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn actual_hotr16_query_authorization_cache_and_cancellation_boundaries() {
    let root = run_dir();
    let client = local_client();
    let mut secrets = Vec::new();

    // A namespace denial completes before the configured adapter sees a socket.
    let run = scenario(&root, "unauthorized-before-adapter");
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let adapter_port = listener.local_addr().unwrap().port();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "query-unauthorized");
    unlock(&run).await;
    let (_, denied_token) = issue_cli(&run, "query-alpha-only", "reader", "alpha");
    assert_eq!(
        configure(&run, Some(adapter_port), 0).await["generation"],
        1
    );
    let denied = post(
        &client,
        server.port,
        &denied_token,
        "/v1/search/hybrid",
        &query("beta", "authorization boundary"),
    )
    .await;
    assert_eq!(denied.0, 403);
    assert!(
        timeout(Duration::from_millis(350), listener.accept())
            .await
            .is_err()
    );
    server.stop(&run).await;
    secrets.push(denied_token);

    // Query-vector reuse stays within one credential and one grant revision.
    let run = scenario(&root, "cache-partition");
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let adapter_port = listener.local_addr().unwrap().port();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "query-cache");
    unlock(&run).await;
    let (profile_a, token_a) = issue_cli(&run, "query-cache-a", "reader", "alpha");
    let (_, token_b) = issue_cli(&run, "query-cache-b", "reader", "alpha");
    assert_eq!(
        configure(&run, Some(adapter_port), 0).await["generation"],
        1
    );
    let (hits, adapter) = success_adapter(listener, 3);
    let cached_query = query("alpha", "cache partition boundary");
    let first = post(
        &client,
        server.port,
        &token_a,
        "/v1/search/hybrid",
        &cached_query,
    )
    .await;
    assert_eq!(first.0, 200);
    assert_eq!(first.1["retrieval_mode"], "hybrid");
    assert_eq!(hits.load(Ordering::SeqCst), 4);
    let repeat = post(&client, server.port, &token_a, "/v1/context", &cached_query).await;
    assert_eq!(repeat.0, 200);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(hits.load(Ordering::SeqCst), 4);
    assert_eq!(
        post(&client, server.port, &token_b, "/v1/context", &cached_query,)
            .await
            .0,
        200
    );
    assert_eq!(hits.load(Ordering::SeqCst), 8);
    let withdrawn = owner::admin(
        &run.join("vault"),
        &AdminRequest::Lifecycle(Request {
            idempotency_key: "query-withdraw-alpha".into(),
            action: Action::Grants {
                client_id: profile_a.client_id.clone(),
                expected_revision: 0,
                role: Role::Reader,
                namespaces: vec!["beta".into()],
            },
        }),
    )
    .await
    .unwrap();
    assert!(withdrawn.error.is_none());
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/search/hybrid",
            &cached_query,
        )
        .await
        .0,
        403
    );
    assert_eq!(hits.load(Ordering::SeqCst), 8);
    let restored = owner::admin(
        &run.join("vault"),
        &AdminRequest::Lifecycle(Request {
            idempotency_key: "query-restore-alpha".into(),
            action: Action::Grants {
                client_id: profile_a.client_id,
                expected_revision: 1,
                role: Role::Reader,
                namespaces: vec!["alpha".into()],
            },
        }),
    )
    .await
    .unwrap();
    assert!(restored.error.is_none());
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/search/hybrid",
            &cached_query,
        )
        .await
        .0,
        200
    );
    adapter.await.unwrap();
    assert_eq!(hits.load(Ordering::SeqCst), 12);
    server.stop(&run).await;
    secrets.extend([token_a, token_b]);

    // Revocation during inference is rechecked after the adapter completes.
    let run = scenario(&root, "revocation-final-gate");
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let adapter_port = listener.local_addr().unwrap().port();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "query-revoke");
    unlock(&run).await;
    let (profile, token) = issue_cli(&run, "query-revoke-client", "reader", "alpha");
    assert_eq!(
        configure(&run, Some(adapter_port), 0).await["generation"],
        1
    );
    let request_client = client.clone();
    let request_token = token.clone();
    let service_port = server.port;
    let request = tokio::spawn(async move {
        post(
            &request_client,
            service_port,
            &request_token,
            "/v1/search/hybrid",
            &query("alpha", "revocation final gate"),
        )
        .await
    });
    let inference = blocked_embedding(&listener).await;
    let revoked = owner::admin(
        &run.join("vault"),
        &AdminRequest::Revoke {
            client_id: profile.client_id,
        },
    )
    .await
    .unwrap();
    assert!(revoked.error.is_none());
    complete_embedding(&listener, inference).await;
    let revoked_result = timeout(Duration::from_secs(4), request)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(revoked_result.0, 401);
    assert_eq!(revoked_result.1, json!({"error":{"code":"unauthorized"}}));
    server.stop(&run).await;
    secrets.push(token);

    // Owner disable cancels the in-flight socket and returns an explicit
    // configuration-change degradation through the original request.
    let run = scenario(&root, "disable-cancellation");
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let adapter_port = listener.local_addr().unwrap().port();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "query-disable");
    unlock(&run).await;
    let (_, token) = issue_cli(&run, "query-disable-client", "reader", "alpha");
    assert_eq!(
        configure(&run, Some(adapter_port), 0).await["generation"],
        1
    );
    let request_client = client.clone();
    let request_token = token.clone();
    let service_port = server.port;
    let request = tokio::spawn(async move {
        post(
            &request_client,
            service_port,
            &request_token,
            "/v1/context",
            &query("alpha", "disable cancellation boundary"),
        )
        .await
    });
    let mut inference = blocked_embedding(&listener).await;
    let disable_started = Instant::now();
    assert_eq!(configure(&run, None, 1).await["generation"], 2);
    let mut remainder = Vec::new();
    let closed = timeout(
        Duration::from_secs(3),
        inference.read_to_end(&mut remainder),
    )
    .await
    .unwrap();
    assert_socket_closed(closed);
    let disabled = timeout(Duration::from_secs(3), request)
        .await
        .unwrap()
        .unwrap();
    assert!(disable_started.elapsed() < Duration::from_secs(3));
    assert_eq!(disabled.0, 200);
    assert_eq!(disabled.1["retrieval_mode"], "lexical_only");
    assert_eq!(disabled.1["degraded_reason"], "embedding_changed");
    assert!(
        timeout(Duration::from_millis(350), listener.accept())
            .await
            .is_err()
    );
    server.stop(&run).await;
    secrets.push(token);

    // A concurrent query degrades immediately while the single inference slot
    // is occupied; the original query then obeys the 1.5-second runtime budget.
    let run = scenario(&root, "busy-timeout");
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let adapter_port = listener.local_addr().unwrap().port();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "query-busy-timeout");
    unlock(&run).await;
    let (_, token) = issue_cli(&run, "query-busy-client", "reader", "alpha");
    assert_eq!(
        configure(&run, Some(adapter_port), 0).await["generation"],
        1
    );
    let request_client = client.clone();
    let request_token = token.clone();
    let service_port = server.port;
    let timeout_started = Instant::now();
    let first = tokio::spawn(async move {
        post(
            &request_client,
            service_port,
            &request_token,
            "/v1/search/hybrid",
            &query("alpha", "bounded timeout boundary"),
        )
        .await
    });
    let mut inference = blocked_embedding(&listener).await;
    let busy_started = Instant::now();
    let busy = post(
        &client,
        server.port,
        &token,
        "/v1/context",
        &query("alpha", "bounded busy boundary"),
    )
    .await;
    assert!(busy_started.elapsed() < Duration::from_secs(1));
    assert_eq!(busy.0, 200);
    assert_eq!(busy.1["retrieval_mode"], "lexical_only");
    assert_eq!(busy.1["degraded_reason"], "embedding_busy");
    let timed_out = timeout(Duration::from_secs(3), first)
        .await
        .unwrap()
        .unwrap();
    assert!(timeout_started.elapsed() < Duration::from_secs(3));
    assert_eq!(timed_out.0, 200);
    assert_eq!(timed_out.1["retrieval_mode"], "lexical_only");
    assert_eq!(timed_out.1["degraded_reason"], "embedding_timeout");
    let mut remainder = Vec::new();
    let closed = timeout(
        Duration::from_secs(2),
        inference.read_to_end(&mut remainder),
    )
    .await
    .unwrap();
    assert_socket_closed(closed);
    assert!(
        timeout(Duration::from_millis(350), listener.accept())
            .await
            .is_err()
    );
    server.stop(&run).await;
    secrets.push(token);

    let secret_refs = secrets
        .iter()
        .map(|secret| secret.as_str())
        .collect::<Vec<_>>();
    let evidence = json!({
        "prompt":"HOTR-16",
        "result":"PASS",
        "binary_sha256":format!("{:x}", Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),
        "real_service_processes":5,
        "synthetic_loopback_adapter":true,
        "actual_model_claimed":false,
        "pinned_model_digest":MODEL_DIGEST,
        "dimensions":DIMENSIONS,
        "unauthorized_adapter_requests":0,
        "cache_first_query_requests":4,
        "cache_same_client_repeat_requests":0,
        "cache_second_client_requests":4,
        "cache_scope_withdrawal_requests":0,
        "cache_restored_new_grant_revision_requests":4,
        "revocation_inflight_final_unauthorized":true,
        "owner_disable_closed_inflight_socket":true,
        "owner_disable_degraded_reason":"embedding_changed",
        "busy_degraded_reason":"embedding_busy",
        "timeout_degraded_reason":"embedding_timeout",
        "query_runtime_budget_ms":1500,
        "raw_queries_or_tokens_recorded":false
    });
    write_new(
        &root.join("HOTR-16-query-boundary.json"),
        &serde_json::to_vec_pretty(&evidence).unwrap(),
    );
    scan(&root, &secret_refs);
}
