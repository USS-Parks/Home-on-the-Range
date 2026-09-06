//! Actual bridge processes reuse the capability suite's isolated vault lifecycle.
use super::*;
use std::{io::Read, process::ChildStdin};

struct Mcp {
    child: Child,
    input: Option<ChildStdin>,
    replies: mpsc::Receiver<Result<Value, &'static str>>,
    next: u64,
}
impl Drop for Mcp {
    fn drop(&mut self) {
        self.input.take();
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
impl Mcp {
    fn start(run: &Path, credential: &str, label: &str) -> Self {
        let stderr = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(run.join(format!("{label}.mcp.stderr.txt")))
            .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hotr"))
            .args(["mcp", "--credential"])
            .arg(run.join(credential))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (send, replies) = mpsc::sync_channel(32);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = Vec::new();
                let n = Read::by_ref(&mut reader)
                    .take(1_048_577)
                    .read_until(b'\n', &mut line);
                if matches!(n, Ok(0)) {
                    break;
                }
                let value = if matches!(n, Ok(n) if n<=1_048_576) && line.last() == Some(&b'\n') {
                    serde_json::from_slice::<Value>(&line)
                        .map_err(|_| "stdout is not JSON")
                        .and_then(|v| {
                            if v["jsonrpc"] == "2.0"
                                && (v.get("result").is_some()
                                    || v.get("error").is_some()
                                    || v.get("method").is_some())
                            {
                                Ok(v)
                            } else {
                                Err("stdout is not a protocol frame")
                            }
                        })
                } else {
                    Err("stdout frame exceeds bound or is incomplete")
                };
                let stop = value.is_err();
                if send.send(value).is_err() || stop {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            replies,
            next: 0,
        }
    }
    fn send(&mut self, value: Value) {
        let mut line = serde_json::to_vec(&value).unwrap();
        line.push(b'\n');
        self.input.as_mut().unwrap().write_all(&line).unwrap();
        self.input.as_mut().unwrap().flush().unwrap();
    }
    async fn response(&mut self) -> Value {
        let start = Instant::now();
        loop {
            match self.replies.try_recv() {
                Ok(value) => return value.expect("stdout protocol-only contract"),
                Err(mpsc::TryRecvError::Disconnected) => {
                    panic!("MCP stdout closed; inspect retained stderr")
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
            assert!(
                start.elapsed() < Duration::from_secs(13),
                "MCP response deadline"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
    async fn request(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let reply = self.response().await;
        assert_eq!(reply["id"], id);
        reply
    }
    async fn initialize(&mut self, version: &str) -> Value {
        let reply=self.request("initialize",json!({"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"hotr-native-protocol-fixture","version":"1"}})).await;
        assert!(reply.get("error").is_none());
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        reply
    }
    async fn tool(&mut self, name: &str, args: Value) -> Value {
        self.request("tools/call", json!({"name":name,"arguments":args}))
            .await
    }
    async fn finish(&mut self, success: bool) {
        self.input.take();
        let start = Instant::now();
        loop {
            for frame in self.replies.try_iter() {
                frame.expect("every stdout line must be protocol");
            }
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.success(), success);
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(7),
                "bridge failed to exit"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        for frame in self.replies.try_iter() {
            frame.expect("trailing stdout must be protocol");
        }
    }
}

fn data(reply: &Value) -> &Value {
    assert!(reply.get("error").is_none(), "protocol rejected valid call");
    assert_eq!(reply["result"]["isError"], false);
    &reply["result"]["structuredContent"]
}
fn denied(reply: Value, status: u16) {
    assert_eq!(reply["result"]["isError"], true);
    assert_eq!(reply["result"]["structuredContent"]["http_status"], status);
}

#[tokio::test(flavor = "current_thread")]
async fn actual_mcp_separate_credentials_calls_reconnect_and_denial() {
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "mcp-owner");
    unlock(&run).await;
    let (a, token_a) = issue_cli(&run, "mcp-a", "contributor", "alpha");
    let (b, token_b) = issue_cli(&run, "mcp-b", "reader", "alpha");
    let mut ca = Mcp::start(&run, "mcp-a.credential", "writer");
    let mut cb = Mcp::start(&run, "mcp-b.credential", "reader");
    let init = ca.initialize("2025-11-25").await;
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(
        cb.initialize("2025-03-26").await["result"]["protocolVersion"],
        "2025-03-26"
    );
    assert_eq!(
        data(&ca.tool("hotr_health", json!({})).await)["client_id"],
        a.client_id
    );
    assert_eq!(
        data(&cb.tool("hotr_health", json!({})).await)["client_id"],
        b.client_id
    );
    let listed = ca.request("tools/list", json!({})).await;
    let tools = listed["result"]["tools"].as_array().unwrap();
    let names = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        std::collections::BTreeSet::from([
            "hotr_health",
            "hotr_search",
            "hotr_get",
            "hotr_create",
            "hotr_revise"
        ])
    );
    assert!(tools.iter().all(|t| t["inputSchema"]["type"] == "object"));
    write_new(
        &run.join("HOTR-10-tools.json"),
        &serde_json::to_vec_pretty(&listed["result"]).unwrap(),
    );
    let original =
        serde_json::to_value(write_request("alpha", "mcp-shared", "mcp-create", None)).unwrap();
    let created = ca.tool("hotr_create", original.clone()).await;
    assert_eq!(data(&created)["receipt"]["revision"], 1);
    assert_eq!(
        ca.tool("hotr_create", original.clone()).await["result"],
        created["result"]
    );
    denied(cb.tool("hotr_create", original.clone()).await, 403);
    let query = json!({"namespace":"alpha","id":"mcp-shared"});
    let recalled = cb.tool("hotr_get", query.clone()).await;
    assert_eq!(data(&recalled)["body"], BODY);
    assert_eq!(
        data(&recalled)["sources"][0]["reference"],
        "https://unopened.invalid/synthetic-source"
    );
    let searched = cb
        .tool(
            "hotr_search",
            json!({"page":page("alpha",10,0),"query":"shared"}),
        )
        .await;
    assert_eq!(data(&searched)["total"], 1);
    let update =
        serde_json::to_value(write_request("alpha", "mcp-shared", "mcp-revise", Some(1))).unwrap();
    assert_eq!(
        data(&ca.tool("hotr_revise", update.clone()).await)["receipt"]["revision"],
        2
    );
    denied(
        ca.tool(
            "hotr_revise",
            serde_json::to_value(write_request("alpha", "mcp-shared", "mcp-stale", Some(1)))
                .unwrap(),
        )
        .await,
        409,
    );
    for name in ["owner_unlock", "hotr_accept", "shell", "sql", "hotr_revoke"] {
        assert!(ca.tool(name, json!({})).await.get("error").is_some());
    }
    for (name, args) in [
        ("hotr_get", json!({"namespace":"beta","id":"mcp-shared"})),
        (
            "hotr_search",
            json!({"page":page("beta",10,0),"query":"shared"}),
        ),
    ] {
        denied(cb.tool(name, args).await, 403);
    }
    assert!(
        ca.tool("hotr_revise", original.clone())
            .await
            .get("error")
            .is_some()
    );
    assert!(ca.tool("hotr_create", update).await.get("error").is_some());
    assert!(
        ca.tool(
            "hotr_get",
            json!({"namespace":"alpha","id":"mcp-shared","principal":"owner"})
        )
        .await
        .get("error")
        .is_some()
    );
    let acceptance = owner::admin(
        &run.join("vault"),
        &AdminRequest::Accept(Accept {
            namespace: "alpha".into(),
            id: "mcp-shared".into(),
            expected_revision: 2,
            idempotency_key: "mcp-owner-accept".into(),
        }),
    )
    .await
    .unwrap();
    assert!(acceptance.error.is_none());
    denied(
        ca.tool(
            "hotr_revise",
            serde_json::to_value(write_request(
                "alpha",
                "mcp-shared",
                "mcp-accepted-edit",
                Some(3),
            ))
            .unwrap(),
        )
        .await,
        403,
    );
    assert_eq!(
        data(&cb.tool("hotr_get", query.clone()).await)["state"],
        "accepted"
    );
    ca.finish(true).await;
    let mut reconnect = Mcp::start(&run, "mcp-a.credential", "reconnect");
    reconnect.initialize("2024-11-05").await;
    assert_eq!(
        data(&reconnect.tool("hotr_get", query.clone()).await)["revision"],
        3
    );
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Revoke {
                client_id: a.client_id
            }
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    denied(reconnect.tool("hotr_get", query.clone()).await, 401);
    denied(reconnect.tool("hotr_health", json!({})).await, 401);
    assert_eq!(data(&cb.tool("hotr_get", query).await)["revision"], 3);
    reconnect.finish(true).await;
    cb.finish(true).await;
    let mut modern = Mcp::start(&run, "mcp-b.credential", "current-protocol");
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}});
    assert!(
        modern
            .request("server/discover", json!({"_meta":meta}))
            .await
            .get("error")
            .is_none()
    );
    let modern_tools = modern.request("tools/list", json!({"_meta":meta})).await;
    assert_eq!(modern_tools["result"]["tools"].as_array().unwrap().len(), 5);
    assert!(
        modern
            .tool("hotr_health", json!({}))
            .await
            .get("error")
            .is_some()
    );
    let modern_call = modern
        .request(
            "tools/call",
            json!({"_meta":meta,"name":"hotr_health","arguments":{}}),
        )
        .await;
    assert_eq!(data(&modern_call)["client_id"], b.client_id);
    assert_eq!(modern_call["result"]["resultType"], "complete");
    denied(modern.request("tools/call",json!({"_meta":meta,"name":"hotr_get","arguments":{"namespace":"beta","id":"mcp-shared"}})).await,403);
    modern.finish(true).await;
    server.stop(&run).await;
    scan(&run, &[&token_a, &token_b]);
    write_new(&run.join("HOTR-10-protocol.json"),&serde_json::to_vec_pretty(&json!({"result":"PASS","binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),"real_bridge_processes":4,"independent_credentials":2,"legacy_protocols":["2025-11-25","2025-03-26","2024-11-05"],"current_protocol":"2026-07-28","inline_metadata_enforced":true,"protocol_only_stdout":true,"current_sourced_revision":3,"revoked_client_denied_while_reader_allowed":true,"owner_tools_absent":true,"named_application_acceptance":false})).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn actual_mcp_cancellation_limits_and_recovery() {
    let run = run_dir();
    // A same-owner delayed HTTP peer is used only to make cancellation observable.
    // The other test uses the real SQLCipher service for all authorization claims.
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "cancel-enrollment");
    unlock(&run).await;
    let (mut profile, token) = issue_cli(&run, "cancel-source", "reader", "alpha");
    server.stop(&run).await;
    profile.port = listener.local_addr().unwrap().port();
    credentials::save(&run.join("delayed.credential"), &profile).unwrap();
    let mut bridge = Mcp::start(&run, "delayed.credential", "cancel");
    bridge.initialize("2025-11-25").await;
    bridge.next = 2;
    bridge.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"hotr_health","arguments":{}}}));
    let (mut socket, _) = timeout(Duration::from_secs(3), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut header = Zeroizing::new(Vec::new());
    timeout(Duration::from_secs(3), async {
        while !header.ends_with(b"\r\n\r\n") {
            let mut b = [0; 1];
            socket.read_exact(&mut b).await.unwrap();
            header.extend_from_slice(&b);
            assert!(header.len() < 8192);
        }
    })
    .await
    .unwrap();
    bridge.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2,"reason":"synthetic cancellation"}}));
    let mut byte = [0; 1];
    assert_eq!(
        timeout(Duration::from_secs(2), socket.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0,
        "cancellation did not close forwarded connection"
    );
    assert!(
        bridge
            .request("ping", json!({}))
            .await
            .get("result")
            .is_some()
    );
    bridge.finish(true).await;
    for (label, bytes) in [
        ("malformed", b"{not-json}\n".to_vec()),
        ("oversized", vec![b'x'; hotr::mcp::MAX_FRAME + 1]),
    ] {
        let mut invalid = Mcp::start(&run, "delayed.credential", label);
        let _ = invalid.input.as_mut().unwrap().write_all(&bytes);
        invalid.finish(false).await;
    }
    let mut excess = Mcp::start(&run, "delayed.credential", "excess");
    excess.initialize("2025-11-25").await;
    for id in 2..=18 {
        excess.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"hotr_health","arguments":{}}}));
    }
    // Keep stdin open: capacity rejection itself must end the bridge.
    let start = Instant::now();
    while excess.child.try_wait().unwrap().is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "in-flight limit did not close the bridge"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    excess.finish(false).await;
    let mut duplicate = Mcp::start(&run, "delayed.credential", "duplicate-id");
    duplicate.initialize("2025-11-25").await;
    for _ in 0..2 {
        duplicate.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"hotr_health","arguments":{}}}));
    }
    duplicate.finish(false).await;
    let mut idle = Mcp::start(&run, "delayed.credential", "initialization-timeout");
    let start = Instant::now();
    while idle.child.try_wait().unwrap().is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(18),
            "initialization did not time out with stdin still open"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(start.elapsed() >= Duration::from_secs(14));
    idle.finish(false).await;
    scan(&run, &[&token]);
    write_new(&run.join("HOTR-10-cancellation.json"),b"{\"result\":\"PASS\",\"real_bridge_cancel_closes_delayed_http\":true,\"post_cancel_ping\":true,\"malformed_and_oversized_frames_rejected\":true,\"in_flight_limit\":16,\"duplicate_in_flight_id_rejected\":true,\"initialization_timeout_with_stdin_open\":true,\"delayed_peer_is_not_named_app_evidence\":true}");
}
