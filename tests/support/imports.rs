use super::*;
use hotr::imports::{self, Request};

fn import_cli(run: &Path, names: &[&str], commit: Option<&str>) -> (bool, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hotr"));
    command
        .arg("import")
        .arg(run.join("vault"))
        .arg("--root")
        .arg(run.join("selected"))
        .args(["--namespace", "alpha"]);
    for name in names {
        command.args(["--file", name]);
    }
    if let Some(digest) = commit {
        command.args(["--commit", digest]);
    }
    let output = command.output().unwrap();
    assert!(output.stdout.len() <= api::MAX_RESPONSE);
    assert!(output.stderr.len() < 2048);
    // Failure diagnostics must not include the selected contents.
    assert!(!String::from_utf8_lossy(&output.stderr).contains("importcanary13"));
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).unwrap_or(Value::Null),
    )
}

fn selected(run: &Path, names: &[&str]) -> imports::Batch {
    imports::prepare(
        &run.join("selected"),
        &names.iter().map(PathBuf::from).collect::<Vec<_>>(),
        "alpha",
    )
    .unwrap()
}

async fn send(run: &Path, batch: imports::Batch, commit: Option<String>) -> owner::Reply {
    owner::admin(
        &run.join("vault"),
        &AdminRequest::Import(Request { batch, commit }),
    )
    .await
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn actual_owner_import_preview_atomic_replay_and_preservation() {
    let run = run_dir();
    fs::create_dir(run.join("selected")).unwrap();
    let markdown = "# Selected roadmap\nimportcanary13 lilac stage 東京\n[opaque](https://unopened.invalid/)\n";
    write_new(&run.join("selected/a.md"), markdown.as_bytes());
    write_new(&run.join("selected/b.json"), br#"{"records":[{"kind":"fact","body":"importcanary13 amber fact","tags":["synthetic"]},{"kind":"decision","body":"importcanary13 decision"}]}"#);
    write_new(&run.join("selected/c.txt"), b"importcanary13 plain note");
    let names = ["a.md", "b.json", "c.txt"];
    let original: Vec<_> = names
        .iter()
        .map(|n| Sha256::digest(fs::read(run.join("selected").join(n)).unwrap()).to_vec())
        .collect();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "imports-before");
    assert!(
        !import_cli(&run, &names, None).0,
        "locked owner accepted import"
    );
    unlock(&run).await;
    let (_, token) = issue_cli(&run, "import-reader", "reader", "alpha");
    let (_, writer) = issue_cli(&run, "import-writer", "contributor", "alpha");
    let client = local_client();
    let count = |run: &Path| selected(run, &names);
    let batch = count(&run);
    assert_eq!(batch.files.len(), 3);
    let (success, preview) = import_cli(&run, &names, None);
    assert!(success);
    let preview = &preview["data"];
    assert_eq!(preview["outcome"], "preview");
    assert_eq!(preview["entries"].as_array().unwrap().len(), 4);
    assert_eq!(
        post(
            &client,
            server.port,
            &token,
            "/v1/records/count",
            &json!({"namespace":"alpha"})
        )
        .await
        .1["count"],
        0
    );
    let digest = preview["preview_digest"].as_str().unwrap().to_owned();
    // File argument order does not change the preview.
    assert_eq!(
        import_cli(&run, &["c.txt", "b.json", "a.md"], None).1["data"],
        *preview
    );
    let (success, committed) = import_cli(&run, &names, Some(&digest));
    assert!(success);
    let committed = committed["data"].clone();
    assert_eq!(committed["inserted"], 4);
    assert_eq!(committed["duplicates"], 0);
    assert_eq!(committed["preview_digest"], digest);
    for entry in preview["entries"].as_array().unwrap() {
        let record = &entry["record"];
        let reply = post(
            &client,
            server.port,
            &token,
            "/v1/records/get",
            &json!({"namespace":"alpha","id":record["id"]}),
        )
        .await;
        assert_eq!(reply.0, 200);
        assert_eq!(reply.1["revision"], 1);
        for field in [
            "namespace",
            "id",
            "kind",
            "body",
            "state",
            "sources",
            "tags",
        ] {
            assert_eq!(reply.1[field], record[field]);
        }
        assert_eq!(reply.1["state"], "proposed");
        assert!(
            reply.1["sources"][0]["reference"]
                .as_str()
                .unwrap()
                .contains(entry["source_sha256"].as_str().unwrap())
        );
    }
    let results = post(
        &client,
        server.port,
        &token,
        "/v1/search",
        &json!({"page":page("alpha",10,0),"query":"importcanary13"}),
    )
    .await;
    assert_eq!(results.1["total"], 4);
    // No owner import route is exposed to an application, even a contributor.
    assert_eq!(
        post(
            &client,
            server.port,
            &writer,
            "/v1/import",
            &json!({"batch":batch})
        )
        .await
        .0,
        404
    );
    let fresh = import_cli(&run, &names, None).1["data"].clone();
    assert!(
        fresh["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["action"] == "duplicate")
    );
    let duplicate = import_cli(&run, &names, fresh["preview_digest"].as_str());
    assert!(duplicate.0);
    assert_eq!(duplicate.1["data"]["inserted"], 0);
    assert_eq!(duplicate.1["data"]["duplicates"], 4);
    // A later owner acceptance is preserved by an old receipt and fresh imports.
    let id = preview["entries"][0]["record"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Accept(Accept {
                namespace: "alpha".into(),
                id: id.clone(),
                expected_revision: 1,
                idempotency_key: "accept-import".into()
            })
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    assert_eq!(import_cli(&run, &names, Some(&digest)).1["data"], committed);
    assert_eq!(
        post(
            &client,
            server.port,
            &token,
            "/v1/records/get",
            &json!({"namespace":"alpha","id":id})
        )
        .await
        .1["state"],
        "accepted"
    );
    let accepted_preview = import_cli(&run, &names, None).1["data"].clone();
    // Revision changes after a preview force a new preview; no silent overwrite.
    let other = accepted_preview["entries"][1]["record"]["id"]
        .as_str()
        .unwrap();
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Accept(Accept {
                namespace: "alpha".into(),
                id: other.into(),
                expected_revision: 1,
                idempotency_key: "accept-second-import".into()
            })
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    assert!(!import_cli(&run, &names, accepted_preview["preview_digest"].as_str()).0);
    // Both malformed raw JSON and a forged hash are rejected by the service.
    let mut malformed = batch.clone();
    malformed.files[1].text =
        "{\"records\":[{\"kind\":\"fact\",\"body\":\"x\",\"state\":\"accepted\"}]}".into();
    malformed.files[1].sha256 = format!("{:x}", Sha256::digest(malformed.files[1].text.as_bytes()));
    assert!(
        send(&run, malformed, Some(digest.clone()))
            .await
            .error
            .is_some()
    );
    let mut forged = batch.clone();
    forged.files[0].sha256 = "0".repeat(64);
    assert!(send(&run, forged, None).await.error.is_some());
    // Alter only a newly created synthetic input, then verify stale-file refusal.
    write_new(&run.join("selected/change.txt"), b"importcanary13 before");
    let changed_preview = import_cli(&run, &["change.txt"], None).1["data"].clone();
    fs::OpenOptions::new()
        .append(true)
        .open(run.join("selected/change.txt"))
        .unwrap()
        .write_all(b" after")
        .unwrap();
    assert!(
        !import_cli(
            &run,
            &["change.txt"],
            changed_preview["preview_digest"].as_str()
        )
        .0
    );
    assert_eq!(
        post(
            &client,
            server.port,
            &token,
            "/v1/records/count",
            &json!({"namespace":"alpha"})
        )
        .await
        .1["count"],
        4
    );
    server.stop(&run).await;
    let mut restart = Server::start(&run, "imports-after");
    unlock(&run).await;
    assert_eq!(
        import_cli(&run, &names, Some(&digest)).1["data"],
        committed,
        "durable retry changed after restart"
    );
    restart.stop(&run).await;
    let db = hotr::schema::open(&run.join("vault/vault.db"), KEY).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM records", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM mutation_audit", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM import_receipts", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    db.execute(
        "INSERT INTO record_fts(record_fts) VALUES('integrity-check')",
        [],
    )
    .unwrap();
    drop(db);
    for (name, hash) in names.iter().zip(original) {
        assert_eq!(
            Sha256::digest(fs::read(run.join("selected").join(name)).unwrap()).as_slice(),
            hash.as_slice()
        );
    }
    // Only the deliberately selected inputs contain plaintext. The vault and
    // normal operational logs must not contain bodies, keys or application tokens.
    for directory in [run.join("vault")] {
        for entry in fs::read_dir(directory).unwrap() {
            let bytes = fs::read(entry.unwrap().path()).unwrap();
            for value in ["importcanary13", token.as_str(), writer.as_str()] {
                assert!(!bytes.windows(value.len()).any(|b| b == value.as_bytes()));
            }
        }
    }
    scan(&run, &[&token, &writer]);
    write_new(&run.join("imports.json"),&serde_json::to_vec_pretty(&json!({"result":"PASS","records":4,"proposed_on_import":true,"preview_matches_records":true,"duplicate_inserts":0,"stale_file_and_revision_refused":true,"malformed_and_forged_input_refused":true,"restart_receipt_identical":true,"sources_preserved":true,"fts_integrity":"ok","binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap()))})).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn actual_import_transaction_rollback_collision_and_vault_binding() {
    let run = run_dir();
    fs::create_dir(run.join("selected")).unwrap();
    write_new(&run.join("selected/atomic.json"),br#"{"records":[{"kind":"fact","body":"first survives only if all commit"},{"kind":"fact","body":"forced rollback"}]}"#);
    owner::create(&run.join("vault"), KEY).unwrap();
    let db = hotr::schema::open(&run.join("vault/vault.db"), KEY).unwrap();
    db.execute_batch("CREATE TRIGGER import_test_fault BEFORE INSERT ON revisions WHEN NEW.body='forced rollback' BEGIN SELECT RAISE(ABORT,'synthetic persistence fault'); END;").unwrap();
    drop(db);
    let mut server = Server::start(&run, "atomic");
    unlock(&run).await;
    let preview = import_cli(&run, &["atomic.json"], None).1["data"].clone();
    assert_eq!(preview["entries"].as_array().unwrap().len(), 2);
    assert!(!import_cli(&run, &["atomic.json"], preview["preview_digest"].as_str()).0);
    server.stop(&run).await;
    let db = hotr::schema::open(&run.join("vault/vault.db"), KEY).unwrap();
    for table in [
        "records",
        "revisions",
        "mutation_audit",
        "import_receipts",
        "record_fts",
    ] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            0,
            "partial import survived in {table}"
        );
    }
    drop(db);
    let other = run_dir();
    fs::create_dir(other.join("selected")).unwrap();
    owner::create(&other.join("vault"), KEY).unwrap();
    let mut server = Server::start(&other, "other-vault");
    unlock(&other).await;
    let batch = selected(&run, &["atomic.json"]);
    assert!(
        send(
            &other,
            batch.clone(),
            preview["preview_digest"].as_str().map(str::to_owned)
        )
        .await
        .error
        .is_some()
    );
    // An application cannot preoccupy an import ID with different contents and
    // have the owner import treat it as a successful duplicate.
    let (_, token) = issue_cli(&other, "collision-writer", "contributor", "alpha");
    let record: RecordInput =
        serde_json::from_value(preview["entries"][0]["record"].clone()).unwrap();
    let mut collision = record.clone();
    collision.body = "different original".into();
    assert_eq!(
        post(
            &local_client(),
            server.port,
            &token,
            "/v1/records",
            &WriteRequest {
                record: collision,
                expected_revision: None,
                idempotency_key: "collision".into()
            }
        )
        .await
        .0,
        200
    );
    assert!(send(&other, batch, None).await.error.is_some());
    server.stop(&other).await;
    scan(&run, &[]);
    scan(&other, &[&token]);
    write_new(&run.join("import-rollback.json"),b"{\"result\":\"PASS\",\"partial_writes\":0,\"receipt_rollback\":true,\"fts_rollback\":true,\"cross_vault_digest_refused\":true,\"collision_refused\":true}");
}

#[test]
fn import_path_and_format_limits_with_actual_junction() {
    use std::os::windows::fs::MetadataExt;
    let run = run_dir();
    let root = run.join("selected");
    fs::create_dir(&root).unwrap();
    write_new(&root.join("safe.md"), b"safe selected content");
    let check = |root: &Path, name: &str| imports::prepare(root, &[PathBuf::from(name)], "alpha");
    assert!(check(&root, "safe.md").is_ok());
    for name in [
        "../safe.md",
        "sub/../safe.md",
        "./safe.md",
        "safe.md:stream",
        "NUL.txt",
        r"\\server\share\safe.md",
        r"C:relative.md",
        r"\\.\pipe\anything",
        "safe.md.",
        "safe.md ",
        "missing.md",
        "safe.md/",
    ] {
        assert!(check(&root, name).is_err(), "unsafe relative path accepted");
    }
    for path in [
        r"\\server\share",
        r"\\?\UNC\server\share",
        r"\\.\C:\",
        r"C:relative",
        r"C:\x\..\y",
    ] {
        assert!(check(Path::new(path), "safe.md").is_err());
    }
    assert!(imports::prepare(&root, &[], "alpha").is_err());
    assert!(imports::prepare(&root, &vec![PathBuf::from("safe.md"); 17], "alpha").is_err());
    assert!(
        imports::prepare(
            &root,
            &[PathBuf::from("safe.md"), PathBuf::from("safe.md")],
            "alpha"
        )
        .is_err()
    );
    for (name, bytes) in [
        ("invalid.txt", vec![0xff]),
        ("binary.exe", b"text".to_vec()),
        ("large.md", vec![b'x'; 65537]),
        ("huge.json", vec![b'x'; imports::MAX_BYTES + 1]),
        ("empty.txt", vec![]),
        ("nul.md", b"a\0b".to_vec()),
        (
            "bad.json",
            br#"{"records":[{"kind":"fact","body":"x","state":"accepted"}]}"#.to_vec(),
        ),
        (
            "many.json",
            serde_json::to_vec(&json!({"records":vec![json!({"kind":"note","body":"x"});65]}))
                .unwrap(),
        ),
    ] {
        write_new(&root.join(name), &bytes);
        assert!(check(&root, name).is_err());
    }
    // An open writer prevents capture: importing never races through a mutable
    // file handle by relaxing Windows share permissions.
    let writer = fs::OpenOptions::new()
        .append(true)
        .open(root.join("safe.md"))
        .unwrap();
    assert!(check(&root, "safe.md").is_err());
    drop(writer);
    let outside = run.join("outside-selection");
    fs::create_dir(&outside).unwrap();
    write_new(&outside.join("secret.md"), b"outside selection sentinel");
    let junction = root.join("junction");
    assert!(!junction.exists());
    let status = Command::new(std::env::var_os("ComSpec").unwrap())
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(
        status.success(),
        "actual Windows junction fixture could not be created"
    );
    let result = check(&root, "junction/secret.md");
    let root_result = check(&junction, "secret.md");
    // Remove only this test's newly created junction, using nonrecursive
    // RemoveDirectory through Rust. Its target and source files stay intact.
    assert!(junction.starts_with(&run));
    assert_ne!(
        fs::symlink_metadata(&junction).unwrap().file_attributes() & 0x400,
        0
    );
    fs::remove_dir(&junction).unwrap();
    assert!(result.is_err() && root_result.is_err());
    assert_eq!(
        fs::read(outside.join("secret.md")).unwrap(),
        b"outside selection sentinel"
    );
    assert_eq!(
        fs::read(root.join("safe.md")).unwrap(),
        b"safe selected content"
    );
    write_new(&run.join("import-paths.json"),b"{\"result\":\"PASS\",\"actual_junction_escape_refused\":true,\"junction_root_refused\":true,\"outside_target_preserved\":true,\"traversal_unc_device_ads_refused\":true,\"bounded_formats_utf8_and_size\":true,\"concurrent_writer_refused\":true}");
}
