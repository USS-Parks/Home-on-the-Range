use super::*;
use hotr::windows_security as security;

fn profile(run: &Path) -> PathBuf {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("work/hotr-client-profiles")
        .join(run.file_name().unwrap());
    security::create_directory(&base).unwrap();
    let profile = base.join("hermes");
    security::create_directory(&profile).unwrap();
    let mut marker = security::create_file(&profile.join("SYNTHETIC-ONLY")).unwrap();
    marker
        .write_all(b"HOTR-12A; isolated Hermes proof\n")
        .unwrap();
    marker.sync_all().unwrap();
    profile
}

fn client(run: &Path, profile: &Path, label: &str, prompt: Option<String>, token: &str) -> Value {
    let request = json!({
        "mode":if prompt.is_some(){"turn"}else{"preflight"},
        "profile":profile,"credential":run.join("hermes-client.credential"),
        "hotr":env!("CARGO_BIN_EXE_hotr"),"label":label,"prompt":prompt
    });
    let mut child = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/clients/hermes_cli.cjs"))
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
    assert!(!String::from_utf8_lossy(&output.stdout).contains(token));
    let value: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| json!({"result":"FAIL","error":"driver output was not JSON"}));
    write_new(
        &run.join(format!("{label}.hermes.json")),
        &serde_json::to_vec_pretty(&value).unwrap(),
    );
    write_new(
        &run.join(format!("{label}.driver.stderr.txt")),
        &output.stderr,
    );
    assert!(
        output.status.success(),
        "Hermes driver failed; inspect {run:?}"
    );
    value
}

fn nested(value: &Value, predicate: &impl Fn(&Value) -> bool, depth: u8) -> bool {
    if depth > 24 {
        return false;
    }
    if predicate(value) {
        return true;
    }
    match value {
        Value::Object(fields) => fields.values().any(|v| nested(v, predicate, depth + 1)),
        Value::Array(values) => values.iter().any(|v| nested(v, predicate, depth + 1)),
        Value::String(text) => {
            serde_json::from_str::<Value>(text).is_ok_and(|v| nested(&v, predicate, depth + 1))
        }
        _ => false,
    }
}

fn actual_result(value: &Value, predicate: impl Fn(&Value) -> bool) -> bool {
    // Only native role=tool messages qualify; exclude model prose and arguments.
    value["tool_results"]
        .as_array()
        .is_some_and(|results| results.iter().any(|v| nested(&v["result"], &predicate, 0)))
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "installed Hermes native MCP discovery; bounded xtask; zero model prompts"]
async fn installed_hermes_preflight() {
    sequence(false).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "actual installed Hermes CLI; three approved synthetic prompts; bounded xtask"]
async fn actual_hermes_acceptance() {
    sequence(true).await;
}

async fn sequence(full: bool) {
    assert_eq!(std::env::var("HOTR_BOUNDED_HERMES").as_deref(), Ok("1"));
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "hermes-owner");
    unlock(&run).await;
    let (credential, token) = issue_cli(&run, "hermes-client", "contributor", "demo");
    let (_, reader) = issue_cli(&run, "independent-reader", "reader", "demo");
    let profile = profile(&run);
    let ready = client(&run, &profile, "preflight", None, &token);
    assert_eq!(ready["result"], "PREFLIGHT_PASS");
    if full {
        let mut initial = write_request("demo", "hermes-colour", "hermes-create", None);
        initial.record.body = "Synthetic rover signal: blue.".into();
        let mut corrected = initial.clone();
        corrected.record.body = "Synthetic rover signal: green.".into();
        corrected.expected_revision = Some(1);
        corrected.idempotency_key = "hermes-correct".into();
        let first = client(
            &run,
            &profile,
            "save-recall-correct",
            Some(format!(
                "Synthetic HOTR acceptance. Use only the supplied HOTR MCP tools. Call mcp__hotr__hotr_create with {initial}. Get namespace demo, id hermes-colour. Revise using mcp__hotr__hotr_revise with {corrected}. Get that demo record again. Finally get namespace forbidden, id hermes-colour to verify denial. Exactly five tool calls. No retries, other tools, source opening or extra changes. Briefly report actual outcomes.",
                initial = serde_json::to_string(&initial).unwrap(),
                corrected = serde_json::to_string(&corrected).unwrap()
            )),
            &token,
        );
        assert_eq!(first["result"], "PASS");
        assert_eq!(first["calls"].as_array().unwrap().len(), 5);
        for (revision, body) in [
            (1, "Synthetic rover signal: blue."),
            (2, "Synthetic rover signal: green."),
        ] {
            assert!(actual_result(&first, |v| v["revision"] == revision && v["body"] == body));
        }
        assert!(actual_result(&first, |v| v["http_status"] == 403));
        assert!(
            owner::admin(
                &run.join("vault"),
                &AdminRequest::Accept(Accept {
                    namespace: "demo".into(),
                    id: "hermes-colour".into(),
                    expected_revision: 2,
                    idempotency_key: "hermes-owner-accept".into()
                })
            )
            .await
            .unwrap()
            .error
            .is_none()
        );
        let port = server.port;
        server.stop(&run).await;
        server = Server::start_at(&run, "hermes-restarted", port);
        unlock(&run).await;
        let recalled = client(&run, &profile, "restart-recall-search", Some(
            "Synthetic HOTR acceptance. Make exactly two calls: get namespace demo, id hermes-colour; then search query green with page namespace demo, limit 5, offset 0, byte_budget 4096, token_budget 1024. Report the current record, revision, state and source from those tool results. Do not use other tools, open sources, retry, or answer from prior knowledge.".into()
        ), &token);
        assert_eq!(recalled["result"], "PASS");
        assert_eq!(recalled["calls"].as_array().unwrap().len(), 2);
        assert!(actual_result(&recalled, |v| v["revision"] == 3
            && v["state"] == "accepted"
            && v["body"] == "Synthetic rover signal: green."
            && v["sources"][0]["reference"]
                == "https://unopened.invalid/synthetic-source"));
        assert!(
            recalled["calls"]
                .as_array()
                .unwrap()
                .iter()
                .any(|call| call["function"]["name"] == "mcp__hotr__hotr_search")
        );
        assert!(
            owner::admin(
                &run.join("vault"),
                &AdminRequest::Revoke {
                    client_id: credential.client_id
                }
            )
            .await
            .unwrap()
            .error
            .is_none()
        );
        let revoked = client(&run, &profile, "revoked", Some(
            "Synthetic HOTR acceptance. Call mcp__hotr__hotr_get exactly once with namespace demo and id hermes-colour. Report the actual permission denial. Do not retry, use other tools, or answer from prior knowledge.".into()
        ), &token);
        assert_eq!(revoked["result"], "PASS");
        assert!(actual_result(&revoked, |v| v["http_status"] == 401));
        let independent = post(
            &local_client(),
            server.port,
            &reader,
            "/v1/records/get",
            &json!({"namespace":"demo","id":"hermes-colour"}),
        )
        .await;
        assert_eq!(independent.0, 200);
        assert_eq!(independent.1["revision"], 3);
    }
    server.stop(&run).await;
    write_new(
        &run.join("HOTR-12A-application.json"),
        &serde_json::to_vec_pretty(&json!({
            "result":"PASS","full_acceptance":full,"version":ready["version"],
            "installed_main_sha256":ready["installed_main_sha256"],"model":"claude-sonnet-5",
            "model_prompts":if full {3}else{0},"native_mcp_discovery":true,
            "save_recall_correct_denial":full,"owner_accept_restart_search_revoke":full,
            "evidence_source":"installed Hermes session database role=tool messages",
            "active_profile_changed":false
        }))
        .unwrap(),
    );
    println!("Installed Hermes evidence: {}", run.display());
}
