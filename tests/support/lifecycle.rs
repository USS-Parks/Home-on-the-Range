use super::mcp_protocol::{Mcp, data, denied};
use super::*;
use hotr::lifecycle::{Action, Inspect, Request};

fn apply_cli(run: &Path, request: &Request) -> (bool, Value) {
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
    let result = child.wait_with_output().unwrap();
    assert!(!String::from_utf8_lossy(&result.stderr).contains("lifecyclecanary"));
    (
        result.status.success(),
        serde_json::from_slice(&result.stdout).unwrap_or(Value::Null),
    )
}
fn request(id: &str, action: Action) -> Request {
    Request {
        idempotency_key: id.into(),
        action,
    }
}
fn visibility(
    id: &str,
    revision: u32,
    tombstoned: bool,
    start: Option<i64>,
    end: Option<i64>,
) -> Action {
    Action::Visibility {
        namespace: "alpha".into(),
        id: id.into(),
        expected_revision: revision,
        tombstoned,
        valid_from_ms: start,
        expires_at_ms: end,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn actual_two_bridges_observe_corrections_retention_and_grant_changes() {
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "lifecycle");
    unlock(&run).await;
    let (a, token_a) = issue_cli(&run, "life-a", "contributor", "alpha");
    let (_, token_b) = issue_cli(&run, "life-b", "reader", "alpha");
    let mut ca = Mcp::start(&run, "life-a.credential", "life-a");
    let mut cb = Mcp::start(&run, "life-b.credential", "life-b");
    ca.initialize("2025-11-25").await;
    cb.initialize("2025-11-25").await;
    let client = local_client();
    for id in [
        "current",
        "old",
        "replacement",
        "future",
        "expires",
        "deleted",
    ] {
        let mut write = write_request("alpha", id, id, None);
        write.record.body = format!("lifecyclecanary initial {id}");
        assert_eq!(
            post(&client, server.port, &token_a, "/v1/records", &write)
                .await
                .0,
            200
        );
    }
    let mut record = write_request("alpha", "current", "unused", None).record;
    record.body = "lifecyclecanary corrected blue".into();
    let correction = request(
        "correction",
        Action::Correct {
            record: record.clone(),
            expected_revision: 1,
        },
    );
    let (ok, receipt) = apply_cli(&run, &correction);
    assert!(ok);
    assert_eq!(receipt["data"]["receipt"]["revision"], 2);
    for bridge in [&mut ca, &mut cb] {
        let result = bridge
            .tool("hotr_get", json!({"namespace":"alpha","id":"current"}))
            .await;
        assert_eq!(data(&result)["body"], record.body);
        assert_eq!(data(&result)["state"], "accepted");
        assert_eq!(data(&result)["revision"], 2);
        let search = bridge
            .tool(
                "hotr_search",
                json!({"page":page("alpha",10,0),"query":"blue"}),
            )
            .await;
        assert_eq!(data(&search)["records"][0]["revision"], 2);
    }
    assert_eq!(apply_cli(&run, &correction).1, receipt);
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/records",
            &write_request("alpha", "current", "unauthorized-correction", Some(2))
        )
        .await
        .0,
        403
    );
    let conflict = request(
        "stale-correction",
        Action::Correct {
            record,
            expected_revision: 1,
        },
    );
    assert!(!apply_cli(&run, &conflict).0);
    let inspect = owner::admin(
        &run.join("vault"),
        &AdminRequest::Inspect(Inspect {
            namespace: "alpha".into(),
            id: "current".into(),
            expected_revision: Some(1),
        }),
    )
    .await
    .unwrap();
    assert!(inspect.error.is_none());
    assert_eq!(inspect.data.as_ref().unwrap()["conflict"], true);
    assert_eq!(inspect.data.unwrap()["current"]["revision"], 2);
    let supersede = request(
        "supersede",
        Action::Supersede {
            namespace: "alpha".into(),
            old_id: "old".into(),
            old_revision: 1,
            replacement_id: "replacement".into(),
            replacement_revision: 1,
        },
    );
    assert!(apply_cli(&run, &supersede).0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    assert!(
        apply_cli(
            &run,
            &request(
                "future",
                visibility("future", 1, false, Some(now + 60_000), None)
            )
        )
        .0
    );
    assert!(
        apply_cli(
            &run,
            &request(
                "expires",
                visibility("expires", 1, false, None, Some(now + 1000))
            )
        )
        .0
    );
    assert!(
        apply_cli(
            &run,
            &request("delete", visibility("deleted", 1, true, None, None))
        )
        .0
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    for token in [&token_a, &token_b] {
        for id in ["old", "future", "expires", "deleted"] {
            for revision in [None, Some(1), Some(2)] {
                assert_eq!(
                    post(
                        &client,
                        server.port,
                        token,
                        "/v1/records/get",
                        &json!({"namespace":"alpha","id":id,"revision":revision})
                    )
                    .await
                    .0,
                    404
                );
            }
            let history = post(
                &client,
                server.port,
                token,
                "/v1/records/history",
                &json!({"page":page("alpha",10,0),"id":id}),
            )
            .await;
            assert_eq!(history.0, 200);
            assert_eq!(history.1["total"], 2);
        }
        let list = post(
            &client,
            server.port,
            token,
            "/v1/records/list",
            &page("alpha", 10, 0),
        )
        .await;
        assert_eq!(list.1["total"], 2);
        assert_eq!(
            post(
                &client,
                server.port,
                token,
                "/v1/records/count",
                &json!({"namespace":"alpha"})
            )
            .await
            .1["count"],
            2
        );
        assert_eq!(
            post(
                &client,
                server.port,
                token,
                "/v1/search",
                &json!({"page":page("alpha",10,0),"query":"lifecyclecanary"})
            )
            .await
            .1["total"],
            2
        );
    }
    for bridge in [&mut ca, &mut cb] {
        for id in ["old", "future", "expires", "deleted"] {
            denied(
                bridge
                    .tool(
                        "hotr_get",
                        json!({"namespace":"alpha","id":id,"revision":1}),
                    )
                    .await,
                404,
            );
        }
    }
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/records",
            &write_request("alpha", "deleted", "resurrect", Some(2))
        )
        .await
        .0,
        403
    );
    let grants = request(
        "scope-withdrawal",
        Action::Grants {
            client_id: a.client_id.clone(),
            expected_revision: 0,
            role: Role::Reader,
            namespaces: vec!["beta".into()],
        },
    );
    assert!(apply_cli(&run, &grants).0);
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/records",
            &write_request("beta", "reader-denial", "reader-denial", None)
        )
        .await
        .0,
        403
    );
    denied(
        ca.tool("hotr_get", json!({"namespace":"alpha","id":"current"}))
            .await,
        403,
    );
    assert_eq!(
        data(
            &cb.tool("hotr_get", json!({"namespace":"alpha","id":"current"}))
                .await
        )["revision"],
        2
    );
    for (endpoint, body) in [
        (
            "/v1/records/get",
            json!({"namespace":"alpha","id":"current","revision":1}),
        ),
        (
            "/v1/search",
            json!({"page":page("alpha",10,0),"query":"blue"}),
        ),
        ("/v1/records/list", page("alpha", 10, 0)),
        ("/v1/records/count", json!({"namespace":"alpha"})),
        (
            "/v1/records/history",
            json!({"page":page("alpha",10,0),"id":"current"}),
        ),
    ] {
        assert_eq!(
            post(&client, server.port, &token_a, endpoint, &body)
                .await
                .0,
            403
        );
    }
    let mut changed_grants = grants.clone();
    changed_grants.idempotency_key = "stale-grants".into();
    assert!(!apply_cli(&run, &changed_grants).0);
    ca.finish(true).await;
    cb.finish(true).await;
    server.stop(&run).await;
    let mut server = Server::start(&run, "lifecycle-restart");
    unlock(&run).await;
    assert_eq!(apply_cli(&run, &correction).1["data"], receipt["data"]);
    assert_eq!(
        post(
            &client,
            server.port,
            &token_a,
            "/v1/records/history",
            &json!({"page":page("alpha",10,0),"id":"current"})
        )
        .await
        .0,
        403
    );
    assert_eq!(
        post(
            &client,
            server.port,
            &token_b,
            "/v1/records/get",
            &json!({"namespace":"alpha","id":"deleted"})
        )
        .await
        .0,
        404
    );
    assert_eq!(
        post(
            &client,
            server.port,
            &token_b,
            "/v1/records/get",
            &json!({"namespace":"alpha","id":"current"})
        )
        .await
        .1["body"],
        "lifecyclecanary corrected blue"
    );
    server.stop(&run).await;
    scan(&run, &[&token_a, &token_b]);
    write_new(&run.join("HOTR-14-lifecycle.json"),&serde_json::to_vec_pretty(&json!({"result":"PASS","actual_bridges":2,"current_correction_revision":2,"accepted_writer_denial":403,"suppression_ids":4,"history_requires_current_grant":true,"stale_revision_conflict":true,"same_bridge_grant_withdrawal":true,"restart_retains_policy":true,"binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap()))})).unwrap());
}
