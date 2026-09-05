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
