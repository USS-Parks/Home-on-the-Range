use super::local_embedding::{Ollama, configure_cli, wait_count};
use super::mcp_protocol::{Mcp, data, denied};
use super::*;

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires installed Ollama and pinned project model; use HOTR-16 bounded gate"]
async fn hotr16_actual_pinned_ollama_hybrid() {
    let run = run_dir();
    let ollama = Ollama::start(&run).await;
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "hybrid-live");
    unlock(&run).await;
    let (_, writer) = issue_cli(&run, "hybrid-live-writer", "contributor", "alpha");
    let (_, beta) = issue_cli(&run, "hybrid-beta-writer", "contributor", "beta");
    let (_, a) = issue_cli(&run, "hybrid-reader-a", "reader", "alpha");
    let (_, b) = issue_cli(&run, "hybrid-reader-b", "reader", "alpha");
    let client = local_client();
    for (namespace, id, body, token) in [
        (
            "alpha",
            "bicycle",
            "HOTR07canary Bicycle maintenance: replace worn brake pads, clean the wheel rims and adjust the cable tension to stop squeaking.",
            writer.as_str(),
        ),
        (
            "alpha",
            "soup",
            "HOTR07canary Tomato soup recipe: simmer tomatoes, onions, garlic and vegetable stock for twenty minutes.",
            writer.as_str(),
        ),
        (
            "beta",
            "private-bicycle",
            "HOTR07canary repairing squeaky bicycle brakes",
            beta.as_str(),
        ),
    ] {
        let mut record = write_request(namespace, id, id, None);
        record.record.body = body.into();
        assert_eq!(
            post(&client, server.port, token, "/v1/records", &record)
                .await
                .0,
            200
        );
    }
    configure_cli(&run, Some(ollama.port), 0);
    let indexed = wait_count(&run, "indexed", 3, 120).await;
    assert_eq!(indexed["failed"], 0);
    let query = json!({"page":page("alpha",5,0),"query":"how to mend squeaky bicycle brakes"});
    let http = post(&client, server.port, &a, "/v1/search/hybrid", &query).await;
    assert_eq!(http.0, 200);
    assert_eq!(http.1["retrieval_mode"], "hybrid");
    assert_eq!(http.1["degraded_reason"], Value::Null);
    assert_eq!(http.1["freshness"], json!({"visible":2,"indexed":2}));
    assert_eq!(http.1["records"][0]["id"], "bicycle");
    assert!(
        !http.1["records"][0]["sources"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        http.1["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["namespace"] == "alpha")
    );
    let mut ca = Mcp::start(&run, "hybrid-reader-a.credential", "hybrid-a");
    let mut cb = Mcp::start(&run, "hybrid-reader-b.credential", "hybrid-b");
    ca.initialize("2024-11-05").await;
    cb.initialize("2024-11-05").await;
    let via_a = data(&ca.tool("hotr_hybrid_search", query.clone()).await).clone();
    let via_b = data(&cb.tool("hotr_context_pack", query.clone()).await).clone();
    assert_eq!(via_a["records"], http.1["records"]);
    assert_eq!(via_b["records"], http.1["records"]);
    assert_eq!(via_b["retrieval_mode"], "hybrid");
    let mut revision = write_request("alpha", "bicycle", "bike-revision", Some(1));
    revision.record.body = "HOTR07canary Updated bicycle maintenance: replace worn brake pads and adjust the cable. Check wheel alignment before riding.".into();
    assert_eq!(
        post(&client, server.port, &writer, "/v1/records", &revision)
            .await
            .0,
        200
    );
    wait_count(&run, "indexed", 3, 60).await;
    // Both clients already queried this text. Cached query vectors must still
    // read the current revision from the database, not replay old result bodies.
    for reply in [
        data(&ca.tool("hotr_context_pack", query.clone()).await).clone(),
        data(&cb.tool("hotr_hybrid_search", query.clone()).await).clone(),
    ] {
        assert_eq!(reply["retrieval_mode"], "hybrid");
        assert_eq!(reply["records"][0]["revision"], 2);
        assert_eq!(reply["records"][0]["body"], revision.record.body);
    }
    denied(
        ca.tool(
            "hotr_hybrid_search",
            json!({"page":page("beta",5,0),"query":"bicycle"}),
        )
        .await,
        403,
    );
    configure_cli(&run, None, 1);
    let disabled = data(
        &cb.tool(
            "hotr_context_pack",
            json!({"page":page("alpha",5,0),"query":"bicycle"}),
        )
        .await,
    )
    .clone();
    assert_eq!(disabled["retrieval_mode"], "lexical_only");
    assert_eq!(disabled["degraded_reason"], "disabled");
    assert_eq!(disabled["records"][0]["revision"], 2);
    ca.finish(true).await;
    cb.finish(true).await;
    server.stop(&run).await;
    let _ = client
        .post(format!("http://127.0.0.1:{}/api/embed", ollama.port))
        .json(&json!({"model":hotr::embedding_transport::MODEL,"input":[],"keep_alive":0}))
        .send()
        .await;
    let pid = ollama.child.id();
    drop(ollama);
    scan(&run, &[&writer, &beta, &a, &b]);
    write_new(&run.join("HOTR-16-local-hybrid.json"), &serde_json::to_vec_pretty(&json!({
        "result":"PASS", "binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),
        "ollama_version":"0.32.6", "owned_ollama_pid":pid, "observed_peer":indexed["last_peer"],
        "model_digest":hotr::embedding_transport::MODEL_DIGEST, "dimensions":768,
        "indexed_records":3,"authorized_namespace_records":2,"actual_mcp_processes":2,
        "http_hybrid":true,"mcp_hybrid":true,"mcp_context_pack":true,"source_preserved":true,
        "corrected_revision_both_clients":true,"cross_namespace_denied":true,
        "disabled_explicitly_lexical":true,"quality_gate":"HOTR-17 remains separate"
    })).unwrap());
}
