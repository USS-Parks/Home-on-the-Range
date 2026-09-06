use super::*;
use hotr::windows_security as security;

#[tokio::test(flavor = "current_thread")]
#[ignore = "installed Lamprey connection-only preflight; no model calls"]
async fn installed_lamprey_preflight() {
    assert_eq!(std::env::var("HOTR_RUN_LAMPREY").as_deref(), Ok("1"));
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "lamprey-preflight");
    unlock(&run).await;
    let (_, token) = issue_cli(&run, "lamprey-client", "contributor", "demo");
    let request = profile_request(&run);
    let mut child = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/clients/lamprey_probe.cjs"))
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
    let value: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| json!({"invalid_output":String::from_utf8_lossy(&output.stdout)}));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(token.as_str()));
    write_new(
        &run.join("lamprey-preflight.json"),
        &serde_json::to_vec_pretty(&value).unwrap(),
    );
    write_new(&run.join("lamprey-driver.stderr.txt"), &output.stderr);
    server.stop(&run).await;
    assert!(
        output.status.success(),
        "installed Lamprey preflight failed; see {run:?}"
    );
    assert_eq!(value["result"], "PREFLIGHT_PASS");
    assert_eq!(value["model_prompts"], 0);
    println!("Installed Lamprey preflight: {}", run.display());
}

fn profile_request(run: &Path) -> Value {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("work/hotr-client-profiles")
        .join(run.file_name().unwrap());
    security::create_directory(&base).unwrap();
    let profile = base.join("lamprey");
    let workspace = base.join("workspace");
    security::create_directory(&profile).unwrap();
    security::create_directory(&workspace).unwrap();
    let mut marker = security::create_file(&profile.join("SYNTHETIC-ONLY")).unwrap();
    marker
        .write_all(b"HOTR-12-LAMPREY; isolated application proof\n")
        .unwrap();
    marker.sync_all().unwrap();
    drop(marker);
    json!({
        "mode":"preflight", "profile":profile, "workspace":workspace,
        "credential":run.join("lamprey-client.credential"),
        "hotr":env!("CARGO_BIN_EXE_hotr"),
        "executable":std::env::var("HOTR_LAMPREY_EXE").unwrap(),
        "lamprey_source":std::env::var("HOTR_LAMPREY_SOURCE").unwrap()
    })
}

struct Lamprey {
    child: Child,
    input: std::process::ChildStdin,
    output: mpsc::Receiver<String>,
}
impl Drop for Lamprey {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
impl Lamprey {
    fn start(run: &Path, request: &Value) -> Self {
        let stderr = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(run.join("lamprey-chat-driver.stderr.txt"))
            .unwrap();
        let mut child = Command::new("node")
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/clients/lamprey_chat.cjs"),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, output) = mpsc::sync_channel(4);
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if line.len() > 8 * 1024 * 1024 || send.send(line).is_err() {
                    break;
                }
            }
        });
        let mut app = Self {
            child,
            input,
            output,
        };
        app.send(request);
        app
    }
    fn send(&mut self, request: &Value) {
        writeln!(self.input, "{}", request).unwrap();
        self.input.flush().unwrap();
    }
    fn receive(&self, run: &Path, label: &str, tokens: &[&str]) -> Value {
        let line = self
            .output
            .recv_timeout(Duration::from_secs(200))
            .expect("Lamprey response timeout; owned descendants are confined to the runner job");
        for token in tokens {
            assert!(!line.contains(token));
        }
        let value: Value = serde_json::from_str(&line).unwrap();
        write_new(
            &run.join(format!("{label}.json")),
            &serde_json::to_vec_pretty(&value).unwrap(),
        );
        value
    }
}

fn nested_result(value: &Value, predicate: &impl Fn(&Value) -> bool, depth: u8) -> bool {
    if depth > 24 {
        return false;
    }
    if predicate(value) {
        return true;
    }
    match value {
        Value::Object(v) => v.values().any(|v| nested_result(v, predicate, depth + 1)),
        Value::Array(v) => v.iter().any(|v| nested_result(v, predicate, depth + 1)),
        Value::String(v) => serde_json::from_str::<Value>(v.strip_prefix("Error: ").unwrap_or(v))
            .is_ok_and(|v| nested_result(&v, predicate, depth + 1)),
        _ => false,
    }
}
fn tool_result(value: &Value, predicate: impl Fn(&Value) -> bool) -> bool {
    value["events"].as_array().is_some_and(|events| {
        events.iter().any(|event| {
            event["type"] == "tool_result" && nested_result(&event["result"], &predicate, 0)
        })
    })
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "actual Lamprey chat; approved compatibility budget; bounded xtask only"]
async fn actual_lamprey_smoke() {
    lamprey_sequence(false).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "actual Lamprey acceptance; approved compatibility budget; bounded xtask only"]
async fn actual_lamprey_acceptance() {
    lamprey_sequence(true).await;
}

async fn lamprey_sequence(full: bool) {
    assert_eq!(std::env::var("HOTR_RUN_LAMPREY").as_deref(), Ok("1"));
    assert_eq!(std::env::var("HOTR_BOUNDED_LAMPREY").as_deref(), Ok("1"));
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "lamprey-smoke");
    unlock(&run).await;
    let (credential, token) = issue_cli(&run, "lamprey-client", "contributor", "demo");
    let (_, reader) = issue_cli(&run, "independent-reader", "reader", "demo");
    let mut app = Lamprey::start(&run, &profile_request(&run));
    let ready = app.receive(&run, "lamprey-ready", &[&token, &reader]);
    assert_eq!(
        ready["type"], "ready",
        "Lamprey readiness failed; inspect {run:?}"
    );
    let mut initial = write_request("demo", "lamprey-colour", "lamprey-create", None);
    initial.record.body = "Synthetic rover signal: blue.".into();
    let mut corrected = initial.clone();
    corrected.record.body = "Synthetic rover signal: green.".into();
    corrected.expected_revision = Some(1);
    corrected.idempotency_key = "lamprey-correct".into();
    app.send(&json!({"operation":"turn", "id":"lamprey-save-recall-correct", "model":"claude-opus-5", "prompt":format!(
        "Synthetic HOTR acceptance. Use only the five HOTR tools and exactly this sequence. Call hotr__hotr_create with {initial_json}. Then call hotr__hotr_get for namespace demo and id lamprey-colour. Then call hotr__hotr_revise with {corrected_json}. Then get that same demo record again. Finally call hotr__hotr_get for namespace forbidden and id lamprey-colour to verify permission denial. Do not retry an error, use other tools, open sources, or change anything else. Briefly report the actual tool outcomes.", initial_json=serde_json::to_string(&initial).unwrap(), corrected_json=serde_json::to_string(&corrected).unwrap())}));
    let outcome = app.receive(&run, "lamprey-save-recall-correct", &[&token, &reader]);
    assert_eq!(outcome["type"], "response");
    assert_eq!(outcome["calls"], 5);
    assert!(tool_result(&outcome, |v| v["revision"] == 2
        && v["body"] == "Synthetic rover signal: green."));
    assert!(tool_result(&outcome, |v| v["http_status"] == 403));
    if full {
        assert!(
            owner::admin(
                &run.join("vault"),
                &AdminRequest::Accept(Accept {
                    namespace: "demo".into(),
                    id: "lamprey-colour".into(),
                    expected_revision: 2,
                    idempotency_key: "lamprey-owner-accept".into(),
                })
            )
            .await
            .unwrap()
            .error
            .is_none()
        );
        let port = server.port;
        server.stop(&run).await;
        server = Server::start_at(&run, "lamprey-restarted", port);
        unlock(&run).await;
        app.send(&json!({"operation":"reconnect", "id":"lamprey-reconnect"}));
        let reconnect = app.receive(&run, "lamprey-reconnect", &[&token, &reader]);
        assert_eq!(reconnect["outcome"]["success"], true);
        for (label, model) in [
            ("lamprey-after-restart", "claude-opus-5"),
            ("lamprey-model-switch", "claude-sonnet-5"),
        ] {
            app.send(&json!({"operation":"turn", "id":label, "model":model,
                "prompt":"Call hotr__hotr_get exactly once for namespace demo, id lamprey-colour. Report its current body, revision, state and source from the tool result. Do not answer from previous messages or use any other tool."}));
            let step = app.receive(&run, label, &[&token, &reader]);
            assert_eq!(step["calls"], 1);
            assert_eq!(step["model"], model);
            assert!(tool_result(&step, |v| v["revision"] == 3
                && v["state"] == "accepted"
                && v["body"] == "Synthetic rover signal: green."
                && v["sources"][0]["reference"]
                    == "https://unopened.invalid/synthetic-source"));
        }
        app.send(&json!({"operation":"turn", "id":"lamprey-cancel", "model":"claude-sonnet-5", "cancel_on_tool":true,
            "prompt":"Call hotr__hotr_get once for namespace demo, id lamprey-colour. The owner will cancel this synthetic request. Do not retry or call any other tool."}));
        let cancelled = app.receive(&run, "lamprey-cancel", &[&token, &reader]);
        assert_eq!(cancelled["cancelled"], true);
        assert_eq!(cancelled["timedOut"], false);
        app.send(&json!({"operation":"turn", "id":"lamprey-after-cancel", "model":"claude-sonnet-5",
            "prompt":"Call hotr__hotr_get exactly once for namespace demo, id lamprey-colour to verify recovery after cancellation. Report the tool result; do not use other tools."}));
        let recovered = app.receive(&run, "lamprey-after-cancel", &[&token, &reader]);
        assert_eq!(recovered["calls"], 1);
        assert!(tool_result(&recovered, |v| v["revision"] == 3
            && v["state"] == "accepted"));
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
        app.send(&json!({"operation":"turn", "id":"lamprey-revoked", "model":"claude-sonnet-5",
            "prompt":"Call hotr__hotr_get exactly once for namespace demo, id lamprey-colour. Report the actual permission denial; do not retry, answer from history, or use any other tool."}));
        let denied = app.receive(&run, "lamprey-revoked", &[&token, &reader]);
        assert_eq!(denied["calls"], 1);
        assert!(tool_result(&denied, |v| v["http_status"] == 401));
    }
    app.send(&json!({"operation":"close"}));
    let closed = app.receive(&run, "lamprey-closed", &[&token, &reader]);
    assert!(app.child.wait().unwrap().success());
    assert_eq!(closed["type"], "closed");
    let current = post(
        &local_client(),
        server.port,
        &reader,
        "/v1/records/get",
        &json!({"namespace":"demo", "id":"lamprey-colour"}),
    )
    .await;
    server.stop(&run).await;
    assert_eq!(
        outcome["type"], "response",
        "Lamprey dispatch failed; inspect {run:?}"
    );
    assert_eq!(outcome["calls"], 5);
    assert!(tool_result(&outcome, |v| v["namespace"] == "demo"
        && v["id"] == "lamprey-colour"
        && v["revision"] == 1
        && v["body"] == "Synthetic rover signal: blue."));
    assert!(tool_result(&outcome, |v| v["namespace"] == "demo"
        && v["revision"] == 2
        && v["body"] == "Synthetic rover signal: green."
        && v["sources"][0]["reference"]
            == "https://unopened.invalid/synthetic-source"));
    assert!(tool_result(&outcome, |v| v["http_status"] == 403));
    assert_eq!(current.0, 200);
    assert_eq!(current.1["revision"], if full { 3 } else { 2 });
    assert_eq!(current.1["body"], corrected.record.body);
    write_new(
        &run.join("HOTR-12-LAMPREY-application.json"),
        &serde_json::to_vec_pretty(&json!({
            "result":"PASS", "full_acceptance":full, "model_prompts":if full {6}else{1},
            "application":ready["preflight"]["version"], "preflight":ready["preflight"]["result"],
            "installed_executable_sha256":ready["preflight"]["executable_sha256"],
            "installed_main_sha256":ready["preflight"]["installed_main_sha256"],
            "save_recall_correct_forbidden_namespace":true,
            "owner_accept_restart_model_switch_cancel_recover_revoke":full,
            "independent_reader_current_revision":current.1["revision"],
            "active_profile_changed":false, "application_exit":closed["report"]["application"]
        }))
        .unwrap(),
    );
    println!("Actual installed Lamprey evidence: {}", run.display());
}
