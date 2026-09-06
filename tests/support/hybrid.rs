//! HOTR-16 external-contract coverage using real service and MCP bridge processes.
use super::mcp_protocol::{Mcp, data, denied};
use super::*;
use hotr::lifecycle::{Action, Request};

fn lifecycle(run: &Path, request: &Request) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .arg("lifecycle")
        .arg(run.join("vault"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "owner lifecycle operation failed; inspect retained diagnostics"
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn request(idempotency_key: &str, action: Action) -> Request {
    Request {
        idempotency_key: idempotency_key.into(),
        action,
    }
}

fn search(namespace: &str, query: &str) -> Value {
    json!({"page":page(namespace, 20, 0),"query":query})
}

fn assert_contract(value: &Value, mode: &str) {
    assert_eq!(value["context_mode"], "current");
    assert_eq!(value["retrieval_mode"], mode);
    assert!(value["freshness"]["visible"].is_u64());
    assert!(value["freshness"]["indexed"].is_u64());
    for key in ["lexical", "semantic", "fused"] {
        assert!(value["candidates"][key].is_u64());
    }
    assert!(value["total"].is_u64());
    assert!(value["omitted_for_budget"].is_u64());
    assert!(value["byte_budget"].is_u64());
    assert!(value["token_budget"].is_u64());
    assert_eq!(
        value["estimated_tokens"].as_u64().unwrap() as usize,
        serde_json::to_vec(value).unwrap().len(),
        "one token is charged per serialized response UTF-8 byte, including envelope"
    );
    for record in value["records"].as_array().unwrap() {
        for field in [
            "namespace",
            "id",
            "revision",
            "state",
            "sources",
            "tags",
            "body",
            "truncated",
        ] {
            assert!(record.get(field).is_some(), "flat record missing {field}");
        }
        assert!(
            record["sources"]
                .as_array()
                .is_some_and(|sources| !sources.is_empty())
        );
        for source in record["sources"].as_array().unwrap() {
            assert!(source["reference"].is_string());
            assert!(source["label"].is_string());
        }
        assert!(record["truncated"].is_boolean());
        assert!(record["body"].as_str().unwrap().len() <= 2048);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn actual_hotr16_hybrid_and_context_current_contract() {
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "hotr16-hybrid");
    unlock(&run).await;
    let (writer_profile, writer_token) = issue_cli(&run, "hotr16-writer", "contributor", "alpha");
    let (_, reader_token) = issue_cli(&run, "hotr16-reader", "reader", "alpha");
    let (_, other_token) = issue_cli(&run, "hotr16-other", "reader", "beta");
    let client = local_client();

    let mut corrected = write_request("alpha", "corrected", "hotr16-corrected-v1", None);
    corrected.record.body = "HOTR07canary 東京 stale hybridneedle sourced record".into();
    let mut old = write_request("alpha", "old", "hotr16-old", None);
    old.record.body = "HOTR07canary old hybridneedle superseded record".into();
    let mut replacement = write_request("alpha", "replacement", "hotr16-replacement", None);
    replacement.record.body = "HOTR07canary replacement hybridneedle current record".into();
    let mut large = write_request("alpha", "large", "hotr16-large", None);
    large.record.body = format!("HOTR07canary 東京 hybridneedle {}", "界".repeat(1400));
    for record in [&corrected, &old, &replacement, &large] {
        assert_eq!(
            post(&client, server.port, &writer_token, "/v1/records", record)
                .await
                .0,
            200
        );
    }

    let mut revision = corrected.record.clone();
    revision.body = "HOTR07canary 東京 corrected hybridneedle current record".into();
    let corrected_receipt = lifecycle(
        &run,
        &request(
            "hotr16-correct",
            Action::Correct {
                record: revision,
                expected_revision: 1,
            },
        ),
    );
    assert_eq!(corrected_receipt["data"]["receipt"]["revision"], 2);
    let superseded = lifecycle(
        &run,
        &request(
            "hotr16-supersede",
            Action::Supersede {
                namespace: "alpha".into(),
                old_id: "old".into(),
                old_revision: 1,
                replacement_id: "replacement".into(),
                replacement_revision: 1,
            },
        ),
    );
    assert_eq!(superseded["error"], Value::Null);

    // Disabled indexing retains an available lexical current-context path;
    // no synthetic embedding endpoint is used for this claim.
    let query = search("alpha", "hybridneedle");
    let hybrid = post(
        &client,
        server.port,
        &reader_token,
        "/v1/search/hybrid",
        &query,
    )
    .await;
    assert_eq!(hybrid.0, 200);
    assert_contract(&hybrid.1, "lexical_only");
    assert_eq!(hybrid.1["degraded_reason"], "disabled");
    let records = hybrid.1["records"].as_array().unwrap();
    assert!(
        records
            .iter()
            .any(|r| r["id"] == "corrected" && r["revision"] == 2)
    );
    assert!(records.iter().any(|r| r["id"] == "replacement"));
    assert!(records.iter().all(|r| r["id"] != "old"));
    let large = records.iter().find(|r| r["id"] == "large").unwrap();
    assert_eq!(large["truncated"], true);
    assert_eq!(
        large["sources"][0]["reference"],
        "https://unopened.invalid/synthetic-source"
    );

    let context = post(&client, server.port, &reader_token, "/v1/context", &query).await;
    assert_eq!(context.0, 200);
    assert_contract(&context.1, "lexical_only");
    assert_eq!(context.1, hybrid.1, "HTTP routes must return one contract");

    // The service charges the complete serialized envelope, including its
    // metadata. These are valid page bounds rather than a synthetic model
    // fixture, and the large Unicode record forces budget selection.
    for (byte_budget, token_budget) in [(1024, 512), (4096, 4096)] {
        let mut bounded = query.clone();
        bounded["page"]["byte_budget"] = json!(byte_budget);
        bounded["page"]["token_budget"] = json!(token_budget);
        for endpoint in ["/v1/search/hybrid", "/v1/context"] {
            let response = post(&client, server.port, &reader_token, endpoint, &bounded).await;
            assert_eq!(response.0, 200);
            assert_contract(&response.1, "lexical_only");
            assert!(serde_json::to_vec(&response.1).unwrap().len() <= byte_budget);
            assert!(serde_json::to_vec(&response.1).unwrap().len() <= token_budget);
            let records = response.1["records"].as_array().unwrap();
            let total = response.1["total"].as_u64().unwrap();
            let omitted = response.1["omitted_for_budget"].as_u64().unwrap();
            assert_eq!(total, records.len() as u64 + omitted);
            assert!(
                response.1["next_offset"].is_null(),
                "all three candidates were considered, including omitted records"
            );
        }
    }

    let mut paged = query.clone();
    paged["page"]["limit"] = json!(1);
    paged["page"]["byte_budget"] = json!(1024);
    paged["page"]["token_budget"] = json!(512);
    for offset in 0..3 {
        paged["page"]["offset"] = json!(offset);
        let result = post(&client, server.port, &reader_token, "/v1/context", &paged).await;
        assert_eq!(result.0, 200);
        assert!(result.1["records"].as_array().unwrap().is_empty());
        assert_eq!(result.1["omitted_for_budget"], 1);
        assert_eq!(
            result.1["next_offset"],
            if offset < 2 {
                json!(offset + 1)
            } else {
                Value::Null
            }
        );
    }

    // Authorization is checked before candidate formation and response output.
    for endpoint in ["/v1/search/hybrid", "/v1/context"] {
        let denied_http = post(&client, server.port, &other_token, endpoint, &query).await;
        assert_eq!(denied_http.0, 403);
        assert_eq!(denied_http.1, json!({"error":{"code":"forbidden"}}));
    }

    let mut writer_bridge = Mcp::start(&run, "hotr16-writer.credential", "hotr16-writer");
    let mut reader_bridge = Mcp::start(&run, "hotr16-reader.credential", "hotr16-reader");
    writer_bridge.initialize("2025-11-25").await;
    reader_bridge.initialize("2025-03-26").await;
    for bridge in [&mut writer_bridge, &mut reader_bridge] {
        let tool_hybrid = data(&bridge.tool("hotr_hybrid_search", query.clone()).await).clone();
        let tool_context = data(&bridge.tool("hotr_context_pack", query.clone()).await).clone();
        assert_contract(&tool_hybrid, "lexical_only");
        assert_eq!(tool_hybrid, hybrid.1);
        assert_eq!(tool_context, context.1);
    }

    // Revoke one live bridge credential and prove the other process keeps its own grant.
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Revoke {
                client_id: writer_profile.client_id
            }
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    denied(
        writer_bridge
            .tool("hotr_hybrid_search", query.clone())
            .await,
        401,
    );
    assert_eq!(
        data(&reader_bridge.tool("hotr_context_pack", query).await)["context_mode"],
        "current"
    );
    writer_bridge.finish(true).await;
    reader_bridge.finish(true).await;
    server.stop(&run).await;
    scan(&run, &[&writer_token, &reader_token, &other_token]);
    write_new(
        &run.join("HOTR-16-hybrid.json"),
        &serde_json::to_vec_pretty(&json!({
            "prompt":"HOTR-16",
            "result":"PASS",
            "binary_sha256":format!("{:x}", Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),
            "real_service":true,
            "real_mcp_processes":2,
            "http_and_mcp_contract":true,
            "disabled_lexical_mode":true,
            "current_correction_revision":2,
            "superseded_excluded":true,
            "unicode_and_source_preserved":true,
            "response_budgeted":true,
            "live_revocation":true
        }))
        .unwrap(),
    );
}
