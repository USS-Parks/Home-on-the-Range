//! HOTR-18 owner-viewer transport coverage.  The ignored browser gate lives in
//! `viewer_browser.cjs`; this module keeps the service-side security contract
//! executable without an installed browser.
use super::*;
use hotr::lifecycle::{Action, Request};

const VIEWER_MARKER: &[u8] = b"HOTR-18 synthetic viewer fixture\n";

fn viewer_origin(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

fn viewer_page(namespace: &str, offset: u32) -> Value {
    json!({"namespace":namespace,"limit":10,"offset":offset,"byte_budget":262144,"token_budget":262144})
}

fn owner_lifecycle(run: &Path, idempotency_key: &str, action: Action) -> Value {
    let request = Request {
        idempotency_key: idempotency_key.into(),
        action,
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .args(["lifecycle", run.join("vault").to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "owner lifecycle setup failed; inspect the retained run"
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

async fn viewer_session_code(run: &Path, seconds: u32) -> String {
    let reply = owner::admin(&run.join("vault"), &AdminRequest::ViewerSession { seconds })
        .await
        .unwrap();
    assert!(
        reply.error.is_none(),
        "viewer session request was rejected: {:?}",
        reply.error
    );
    let data = reply.data.unwrap();
    let code = data["code"].as_str().unwrap().to_owned();
    assert_eq!(code.len(), 64);
    assert!(code.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(data["code_expires_in_seconds"], 90);
    assert_eq!(data["session_seconds"], seconds);
    code
}

async fn viewer_exchange(client: &reqwest::Client, port: u16, code: &str) -> (u16, Value) {
    let response = client
        .post(format!("{}/viewer/api/session", viewer_origin(port)))
        .header("origin", viewer_origin(port))
        .header("sec-fetch-site", "same-origin")
        .header("x-hotr-viewer", "1")
        .header("content-type", "application/json")
        .json(&json!({"code":code}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    let status = response.status().as_u16();
    let body = response.bytes().await.unwrap();
    assert!(body.len() <= api::MAX_RESPONSE);
    (status, serde_json::from_slice(&body).unwrap())
}

async fn viewer_read(
    client: &reqwest::Client,
    port: u16,
    token: &str,
    body: &Value,
) -> (u16, Value) {
    let response = client
        .post(format!("{}/viewer/api/read", viewer_origin(port)))
        .header("origin", viewer_origin(port))
        .header("sec-fetch-site", "same-origin")
        .header("x-hotr-viewer", "1")
        .header("content-type", "application/json")
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    assert!(bytes.len() <= api::MAX_RESPONSE);
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn raw_viewer_status(port: u16, request: Zeroizing<String>) -> u16 {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    raw_response(&mut stream).await.0
}

async fn seed(run: &Path, server: &Server) -> (Zeroizing<String>, CredentialProfile) {
    let client = local_client();
    let (_writer, token) = issue_cli(run, "viewer-writer", "contributor", "alpha");
    let (reader, _) = issue_cli(run, "viewer-reader", "reader", "alpha");
    let (revoked, _) = issue_cli(run, "viewer-revoked", "reader", "alpha");
    let mut active = write_request("alpha", "active", "viewer-active", None);
    active.record.body = "HOTR07canary viewer long Unicode r1-historical <img src=x onerror=window.__hotr_stored=1><script>window.__hotr_stored=1</script>".into();
    while active.record.body.len() + "界".len() <= hotr::schema::MAX_BODY_BYTES {
        active.record.body.push('界');
    }
    active
        .record
        .body
        .push_str(&"x".repeat(hotr::schema::MAX_BODY_BYTES - active.record.body.len()));
    assert_eq!(
        active.record.body.len(),
        65_536,
        "exact UTF-8 long-text fixture"
    );
    active.record.sources[0].reference = "javascript:window.__hotr_stored=1".into();
    active.record.sources[0].label = "stored script-shaped source".into();
    let mut hidden = write_request("alpha", "hidden-retained", "viewer-hidden", None);
    hidden.record.body = "HOTR07canary retained hidden viewer record".into();
    for record in [&active, &hidden] {
        assert_eq!(
            post(&client, server.port, &token, "/v1/records", record)
                .await
                .0,
            200
        );
    }
    let mut corrected = active.record.clone();
    corrected.body = corrected.body.replacen("r1-historical", "r2-current", 1);
    corrected
        .body
        .push_str(&"x".repeat(hotr::schema::MAX_BODY_BYTES - corrected.body.len()));
    assert_eq!(corrected.body.len(), 65_536);
    let receipt = owner_lifecycle(
        run,
        "viewer-correct",
        Action::Correct {
            record: corrected,
            expected_revision: 1,
        },
    );
    assert!(
        receipt["data"].is_object(),
        "owner lifecycle reply must retain its DTO"
    );
    assert!(
        owner_lifecycle(
            run,
            "viewer-hide",
            Action::Visibility {
                namespace: "alpha".into(),
                id: "hidden-retained".into(),
                expected_revision: 1,
                tombstoned: true,
                valid_from_ms: None,
                expires_at_ms: None,
            },
        )["error"]
            .is_null()
    );
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Revoke {
                client_id: revoked.client_id
            }
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    let backup = owner::backup(
        &run.join("vault"),
        &run.join("viewer-backup"),
        b"HOTR18BackupKey-different-4e341",
    )
    .await
    .unwrap();
    assert!(backup.error.is_none());
    (token, reader)
}

#[tokio::test(flavor = "current_thread")]
async fn actual_hotr18_owner_viewer_http_security_and_read_contract() {
    let run = run_dir();
    write_new(&run.join(".fixture"), VIEWER_MARKER);
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "viewer-http");
    unlock(&run).await;
    let (writer, reader) = seed(&run, &server).await;
    let client = local_client();

    let code = viewer_session_code(&run, 30).await;
    let (status, session) = viewer_exchange(&client, server.port, &code).await;
    assert_eq!(status, 200);
    let viewer_token = Zeroizing::new(session["token"].as_str().unwrap().to_owned());
    assert!(viewer_token.len() >= 32);
    assert_eq!(session["expires_in_seconds"], 30);
    assert_eq!(
        viewer_exchange(&client, server.port, &code).await.0,
        401,
        "codes are one-time"
    );

    let checks = [
        ("ping", json!({"operation":"ping"})),
        ("index", json!({"operation":"index"})),
        ("backup", json!({"operation":"backup"})),
        ("namespaces", json!({"operation":"namespaces","offset":0})),
        ("clients", json!({"operation":"clients","offset":0})),
        (
            "records",
            json!({"operation":"records","namespace":"alpha","offset":0}),
        ),
        (
            "list",
            json!({"operation":"list","page":viewer_page("alpha", 0)}),
        ),
        (
            "search",
            json!({"operation":"search","query":{"page":viewer_page("alpha", 0),"query":"viewer"}}),
        ),
        (
            "inspect",
            json!({"operation":"inspect","query":{"namespace":"alpha","id":"active","expected_revision":1}}),
        ),
        (
            "history",
            json!({"operation":"history","query":{"page":viewer_page("alpha", 0),"id":"active"}}),
        ),
    ];
    for (name, request) in checks {
        let (status, value) = viewer_read(&client, server.port, &viewer_token, &request).await;
        assert_eq!(status, 200, "viewer read {name} failed: {value}");
        assert!(value.is_object());
    }
    let (_, listed) = viewer_read(
        &client,
        server.port,
        &viewer_token,
        &json!({"operation":"list","page":viewer_page("alpha", 0)}),
    )
    .await;
    assert!(
        listed["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["id"] == "active" && record["revision"] == 2)
    );
    assert!(
        listed["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| record["id"] != "hidden-retained")
    );
    let (_, history) = viewer_read(
        &client,
        server.port,
        &viewer_token,
        &json!({"operation":"history","query":{"page":viewer_page("alpha", 0),"id":"active"}}),
    )
    .await;
    assert_eq!(history["total"], 2);
    let (_, clients) = viewer_read(
        &client,
        server.port,
        &viewer_token,
        &json!({"operation":"clients","offset":0}),
    )
    .await;
    assert!(clients.to_string().contains(&reader.client_id));
    let (_, backup) = viewer_read(
        &client,
        server.port,
        &viewer_token,
        &json!({"operation":"backup"}),
    )
    .await;
    assert_eq!(backup["status"], "succeeded");
    assert!(backup.get("last_success").is_some());

    let static_response = client
        .get(format!("{}/viewer/", viewer_origin(server.port)))
        .send()
        .await
        .unwrap();
    assert_eq!(static_response.status().as_u16(), 200);
    for (header, expected) in [
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
    ] {
        assert_eq!(static_response.headers()[header], expected);
    }
    assert!(
        static_response
            .headers()
            .get("content-security-policy")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("default-src 'none'")
    );
    assert!(!static_response.headers().contains_key("set-cookie"));

    let read_url = format!("{}/viewer/api/read", viewer_origin(server.port));
    for response in [
        client
            .post(&read_url)
            .header("content-type", "application/json")
            .json(&json!({"operation":"ping"}))
            .send()
            .await
            .unwrap(),
        client
            .post(&read_url)
            .header("origin", viewer_origin(server.port))
            .header("sec-fetch-site", "same-origin")
            .header("x-hotr-viewer", "wrong")
            .header("content-type", "application/json")
            .bearer_auth(&*viewer_token)
            .json(&json!({"operation":"ping"}))
            .send()
            .await
            .unwrap(),
        client
            .post(&read_url)
            .header("origin", "http://127.0.0.1:1")
            .header("sec-fetch-site", "same-origin")
            .header("x-hotr-viewer", "1")
            .header("content-type", "application/json")
            .bearer_auth(&*viewer_token)
            .json(&json!({"operation":"ping"}))
            .send()
            .await
            .unwrap(),
        client
            .post(&read_url)
            .header("origin", viewer_origin(server.port))
            .header("sec-fetch-site", "cross-site")
            .header("x-hotr-viewer", "1")
            .header("content-type", "application/json")
            .bearer_auth(&*viewer_token)
            .json(&json!({"operation":"ping"}))
            .send()
            .await
            .unwrap(),
        client
            .post(&read_url)
            .header("origin", viewer_origin(server.port))
            .header("sec-fetch-site", "same-origin")
            .header("x-hotr-viewer", "1")
            .bearer_auth(&*viewer_token)
            .body("{}")
            .send()
            .await
            .unwrap(),
    ] {
        assert!(matches!(response.status().as_u16(), 400 | 401 | 403 | 415));
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    let duplicate = Zeroizing::new(format!(
        "POST /viewer/api/read HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: {}\r\nSec-Fetch-Site: same-origin\r\nX-HOTR-Viewer: 1\r\nX-HOTR-Viewer: 1\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{{\"operation\":\"ping\"}}",
        server.port,
        viewer_origin(server.port),
        viewer_token.as_str()
    ));
    assert_eq!(raw_viewer_status(server.port, duplicate).await, 403);
    for request in [
        Zeroizing::new(format!(
            "POST /viewer/api/read HTTP/1.1\r\nHost: evil.invalid\r\nOrigin: {}\r\nSec-Fetch-Site: same-origin\r\nX-HOTR-Viewer: 1\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{{\"operation\":\"ping\"}}",
            viewer_origin(server.port),
            viewer_token.as_str()
        )),
        Zeroizing::new(format!(
            "POST /viewer/api/read HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nSec-Fetch-Site: same-origin\r\nX-HOTR-Viewer: 1\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{{\"operation\":\"ping\"}}",
            server.port,
            viewer_token.as_str()
        )),
    ] {
        assert_eq!(raw_viewer_status(server.port, request).await, 403);
    }
    assert_eq!(
        client
            .get(format!("{}/v1/status", viewer_origin(server.port)))
            .bearer_auth(&*viewer_token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        client
            .post(format!("{}/v1/records", viewer_origin(server.port)))
            .bearer_auth(&*viewer_token)
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        viewer_read(&client, server.port, &writer, &json!({"operation":"ping"}))
            .await
            .0,
        401
    );
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::ViewerSession { seconds: 4 }
        )
        .await
        .unwrap()
        .error
        .is_some()
    );
    server.stop(&run).await;
    scan(&run, &[&writer, &code, &viewer_token]);
    write_new(&run.join("HOTR-18-viewer-http.json"), &serde_json::to_vec_pretty(&json!({
        "prompt":"HOTR-18","result":"PASS","real_loopback_service":true,"owner_session_one_time":true,
        "viewer_read_operations":10,"viewer_token_rejected_by_v1":true,"csrf_host_and_duplicate_header_denials":true,
        "backup_status":"succeeded","static_security_headers":true,"binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap()))
    })).unwrap());
}

fn redact_browser_output(bytes: &[u8]) -> Vec<u8> {
    let mut output = String::from_utf8_lossy(bytes).into_owned().into_bytes();
    for start in 0..output.len().saturating_sub(63) {
        if output[start..start + 64]
            .iter()
            .all(|byte| byte.is_ascii_hexdigit())
        {
            output[start..start + 64].fill(b'x');
        }
    }
    output
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "actual installed Chrome in a new HOTR-18 synthetic profile; bounded by xtask"]
async fn hotr18_actual_owner_viewer_browser() {
    let run = run_dir();
    write_new(&run.join(".fixture"), VIEWER_MARKER);
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "viewer-browser");
    unlock(&run).await;
    let (writer, _) = seed(&run, &server).await;
    let config = json!({
        "run":run,
        "vault":run.join("vault"),
        "binary":env!("CARGO_BIN_EXE_hotr"),
        "port":server.port,
    });
    let mut child = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/viewer_browser.cjs"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    write_new(
        &run.join("HOTR-18-browser.stdout.json"),
        &redact_browser_output(&output.stdout),
    );
    write_new(
        &run.join("HOTR-18-browser.stderr.txt"),
        &redact_browser_output(&output.stderr),
    );
    assert!(
        output.status.success(),
        "installed Chrome browser gate failed; inspect sanitized retained output"
    );
    let proof: Value = serde_json::from_slice(&output.stdout)
        .expect("browser gate must emit one JSON proof object");
    assert_eq!(proof["prompt"], "HOTR-18");
    assert_eq!(proof["result"], "PASS");
    assert_eq!(proof["headless"], true);
    assert!(
        proof["browser"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    let assertions = proof["assertions"]
        .as_array()
        .expect("browser assertions missing");
    for required in [
        "keyboard Enter exchanges owner code",
        "long Unicode data rendered",
        "stored markup did not execute",
        "expected revision conflict is rendered",
        "history retains both revisions and plaintext source",
        "foreign origin browser CSRF blocked",
        "logout clears DOM and controlled pending response cannot repaint",
        "expired captured viewer token is denied by service",
        "locked service state clears viewer DOM",
        "viewer page made no non-loopback requests",
        "all exchanged viewer credentials absent from retained fixture and Chrome profile",
    ] {
        assert!(
            assertions
                .iter()
                .any(|entry| entry.as_str() == Some(required)),
            "missing browser assertion: {required}"
        );
    }
    assert!(run.join("HOTR-18-search.png").is_file() && run.join("HOTR-18-final.png").is_file());
    assert!(run.join("HOTR-18-browser.json").is_file());
    let deadline = Instant::now() + Duration::from_secs(7);
    loop {
        if let Some(status) = server.child.try_wait().unwrap() {
            assert!(
                status.success(),
                "server did not exit cleanly after owned final lock"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "server remained active after owned final lock"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    scan(&run, &[&writer]);
    write_new(&run.join("HOTR-18-browser-launcher.json"), &serde_json::to_vec_pretty(&json!({
        "prompt":"HOTR-18","result":"PASS","installed_chrome":proof["browser"],"headless":true,
        "assertions":assertions.len(),"screenshots":["HOTR-18-search.png","HOTR-18-final.png"],
        "binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap()))
    })).unwrap());
}
