use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const BACKUP_KEY: &[u8] = b"HOTR11BackupKey-8bcd49c1-different";

#[tokio::test(flavor = "current_thread")]
async fn actual_legacy_backup_migration_preserves_source_and_revokes_clients() {
    fn private_file(path: &Path, bytes: &[u8]) {
        let mut file = hotr::windows_security::create_file(path).unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }
    let run = run_dir();
    let client = local_client();
    let legacy_token = "b".repeat(64);
    let token_hash: [u8; 32] = Sha256::digest(legacy_token.as_bytes()).into();
    for version in [5u32, 6] {
        let snapshot = run.join(format!("snapshot-v{version}"));
        hotr::windows_security::create_directory(&snapshot).unwrap();
        drop(hotr::windows_security::create_file(&snapshot.join("vault.db")).unwrap());
        let db = hotr::open_encrypted(&snapshot.join("vault.db"), KEY).unwrap();
        db.execute_batch("CREATE TABLE hotr_vault(format INTEGER PRIMARY KEY CHECK(format=1)); INSERT INTO hotr_vault VALUES(1);").unwrap();
        for migration in [
            include_str!("../../src/schema_v1.sql"),
            include_str!("../../src/schema_v2.sql"),
            include_str!("../../src/schema_v3.sql"),
            include_str!("../../src/schema_v4.sql"),
            include_str!("../../src/schema_v5.sql"),
            include_str!("../../src/schema_v6.sql"),
        ]
        .into_iter()
        .take(version as usize)
        {
            db.execute_batch(migration).unwrap();
        }
        db.pragma_update(None, "user_version", version).unwrap();
        db.execute("INSERT INTO clients(id,label,token_hash,role,revoked,created_at_ms) VALUES('legacy','legacy',?1,'contributor',0,0)", [token_hash.as_slice()]).unwrap();
        db.execute("INSERT INTO client_grants VALUES('legacy','alpha')", [])
            .unwrap();
        let writer = hotr::writer::Writer::start(db).unwrap();
        writer
            .handle()
            .submit_authenticated(
                token_hash,
                write_request("alpha", "legacy-record", "legacy-write", None),
            )
            .unwrap()
            .wait()
            .await
            .unwrap();
        writer.shutdown().await.unwrap();
        let db = hotr::open_encrypted(&snapshot.join("vault.db"), KEY).unwrap();
        db.execute_batch("PRAGMA journal_mode=DELETE;").unwrap();
        drop(db);
        let original = fs::read(snapshot.join("vault.db")).unwrap();
        let manifest = hotr::backup::Manifest {
            format: 1,
            snapshot_id: "11".repeat(16),
            sqlcipher: "4.18.0".into(),
            bytes: original.len() as u64,
            ciphertext_sha256: format!("{:x}", Sha256::digest(&original)),
            watermark: hotr::backup::Watermark {
                schema_version: version,
                records: 1,
                revisions: 1,
                receipts: 1,
                audit_sequence: 1,
                clients: 1,
                grants: 1,
            },
        };
        let original_manifest = serde_json::to_vec_pretty(&manifest).unwrap();
        private_file(&snapshot.join("backup.json"), &original_manifest);
        for rejected_version in [
            4,
            hotr::schema::VERSION + 1,
            if version == 5 { 6 } else { 5 },
        ] {
            let bad = run.join(format!("bad-v{version}-as-{rejected_version}"));
            hotr::windows_security::create_directory(&bad).unwrap();
            let mut changed = manifest.clone();
            changed.watermark.schema_version = rejected_version;
            private_file(
                &bad.join("backup.json"),
                &serde_json::to_vec(&changed).unwrap(),
            );
            private_file(&bad.join("vault.db"), &original);
            let rejected_destination =
                run.join(format!("rejected-v{version}-as-{rejected_version}"));
            assert!(hotr::backup::restore(&bad, &rejected_destination, KEY).is_err());
            assert!(!rejected_destination.exists());
        }
        let restored = run.join(format!("restored-v{version}"));
        fs::create_dir(&restored).unwrap();
        let result = hotr::backup::restore(&snapshot, &restored.join("vault"), KEY).unwrap();
        assert_eq!(result["watermark"]["schema_version"], version);
        assert_eq!(result["restored_schema_version"], hotr::schema::VERSION);
        assert_eq!(result["clients_invalidated"], 1);
        assert_eq!(fs::read(snapshot.join("vault.db")).unwrap(), original);
        assert_eq!(
            fs::read(snapshot.join("backup.json")).unwrap(),
            original_manifest
        );
        let mut server = Server::start(&restored, "legacy-restored");
        unlock(&restored).await;
        assert_eq!(
            post(
                &client,
                server.port,
                &legacy_token,
                "/v1/records/get",
                &json!({"namespace":"alpha","id":"legacy-record"})
            )
            .await
            .0,
            401
        );
        let (_, fresh_token) = issue_cli(&restored, "fresh-reader", "reader", "alpha");
        let recalled = post(
            &client,
            server.port,
            &fresh_token,
            "/v1/records/get",
            &json!({"namespace":"alpha","id":"legacy-record"}),
        )
        .await;
        assert_eq!(recalled.0, 200);
        assert_eq!(recalled.1["body"], BODY);
        assert_eq!(recalled.1["revision"], 1);
        server.stop(&restored).await;
        let db = hotr::schema::open(&restored.join("vault/vault.db"), KEY).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM write_receipts", [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM mutation_audit", [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row(
                "SELECT grant_revision FROM clients WHERE id='legacy'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
            0
        );
        drop(db);
        scan(&snapshot, &[]);
        scan(&restored, &[&fresh_token]);
    }
    write_new(&run.join("HOTR-14-legacy-restore.json"), &serde_json::to_vec_pretty(&json!({"result":"PASS","source_schema_versions":[5,6],"restored_schema_version":hotr::schema::VERSION,"original_backups_unchanged":true,"legacy_credentials_denied":401,"new_clients_recall_original_revision":true,"future_unsupported_and_mismatched_versions_refused_before_creation":true,"binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap()))})).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn actual_encrypted_backup_during_writes_and_fresh_restore() {
    let run = run_dir();
    owner::create(&run.join("vault"), KEY).unwrap();
    let mut server = Server::start(&run, "backup-owner");
    assert!(
        owner::backup(&run.join("vault"), &run.join("locked-snapshot"), BACKUP_KEY)
            .await
            .unwrap()
            .error
            .is_some()
    );
    assert!(!run.join("locked-snapshot").exists());
    unlock(&run).await;
    let (a, ta) = issue_cli(&run, "backup-writer", "contributor", "alpha");
    let (b, tb) = issue_cli(&run, "backup-reader", "reader", "alpha");
    let (c, tc) = issue_cli(&run, "backup-revoked", "reader", "alpha");
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Revoke {
                client_id: c.client_id.clone()
            }
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    let original =
        serde_json::to_value(write_request("alpha", "accepted", "backup-original", None)).unwrap();
    assert_eq!(
        api::scoped_request(&a, "POST", "/v1/records", Some(&original))
            .await
            .unwrap()
            .0,
        200
    );
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Accept(Accept {
                namespace: "alpha".into(),
                id: "accepted".into(),
                expected_revision: 1,
                idempotency_key: "backup-accept".into()
            })
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    let done = Arc::new(AtomicUsize::new(0));
    let mut writers = Vec::new();
    for worker in 0..4 {
        let profile = a.clone();
        let done = done.clone();
        writers.push(tokio::spawn(async move {
            let mut receipts = Vec::new();
            for index in 0..50 {
                let id = format!("backup-{worker}-{index}");
                let request = serde_json::to_value(write_request("alpha", &id, &id, None)).unwrap();
                let result = api::scoped_request(&profile, "POST", "/v1/records", Some(&request))
                    .await
                    .unwrap();
                assert_eq!(result.0, 200);
                receipts.push((id, result.1["receipt"]["audit_sequence"].as_i64().unwrap()));
                done.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            receipts
        }));
    }
    let start = Instant::now();
    while done.load(Ordering::SeqCst) < 10 {
        assert!(start.elapsed() < Duration::from_secs(5));
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let snapshot = run.join("snapshot");
    let reply = owner::backup(&run.join("vault"), &snapshot, BACKUP_KEY)
        .await
        .unwrap();
    assert!(reply.error.is_none(), "online backup rejected");
    let manifest: hotr::backup::Manifest = serde_json::from_value(reply.data.unwrap()).unwrap();
    assert!(manifest.watermark.records > 1 && manifest.watermark.records < 201);
    let mut receipts = Vec::new();
    for writer in writers {
        receipts.extend(writer.await.unwrap());
    }
    assert_eq!(done.load(Ordering::SeqCst), 200);
    assert!(
        owner::admin(
            &run.join("vault"),
            &AdminRequest::Revoke {
                client_id: a.client_id.clone()
            }
        )
        .await
        .unwrap()
        .error
        .is_none()
    );
    let before = Sha256::digest(fs::read(run.join("vault/vault.db")).unwrap());
    let before_wal = Sha256::digest(fs::read(run.join("vault/vault.db-wal")).unwrap());
    let snapshot_before = Sha256::digest(fs::read(snapshot.join("vault.db")).unwrap());
    assert!(
        owner::backup(&run.join("vault"), &snapshot, BACKUP_KEY)
            .await
            .unwrap()
            .error
            .is_some()
    );
    assert_eq!(
        snapshot_before,
        Sha256::digest(fs::read(snapshot.join("vault.db")).unwrap())
    );
    assert!(hotr::backup::restore(&snapshot, &run.join("wrong-key"), KEY).is_err());
    assert!(!run.join("wrong-key").exists());
    assert!(hotr::backup::restore(&snapshot, &run.join("vault"), BACKUP_KEY).is_err());
    // Mutate only new marked synthetic backup copies; preserve the original.
    for (label, truncated) in [("tampered", false), ("truncated", true)] {
        let copy = run.join(label);
        hotr::windows_security::create_directory(&copy).unwrap();
        let mut bytes = fs::read(snapshot.join("vault.db")).unwrap();
        if truncated {
            bytes.truncate(bytes.len() / 2);
        } else {
            bytes[4096 + 100] ^= 0x5a;
        }
        let mut file = hotr::windows_security::create_file(&copy.join("vault.db")).unwrap();
        file.write_all(&bytes).unwrap();
        drop(file);
        let mut meta = hotr::windows_security::create_file(&copy.join("backup.json")).unwrap();
        meta.write_all(&fs::read(snapshot.join("backup.json")).unwrap())
            .unwrap();
        drop(meta);
        assert!(
            hotr::backup::restore(&copy, &run.join(format!("{label}-restore")), BACKUP_KEY)
                .is_err()
        );
        assert!(!run.join(format!("{label}-restore")).exists());
        // Even an attacker-updated untrusted outer checksum cannot authenticate damaged pages.
        let mut edited = manifest.clone();
        edited.bytes = bytes.len() as u64;
        edited.ciphertext_sha256 = format!("{:x}", Sha256::digest(&bytes));
        fs::write(
            copy.join("backup.json"),
            serde_json::to_vec(&edited).unwrap(),
        )
        .unwrap();
        assert!(
            hotr::backup::restore(&copy, &run.join(format!("{label}-rehashed")), BACKUP_KEY)
                .is_err()
        );
        assert!(!run.join(format!("{label}-rehashed")).exists());
    }
    assert_eq!(
        before,
        Sha256::digest(fs::read(run.join("vault/vault.db")).unwrap())
    );
    assert_eq!(
        before_wal,
        Sha256::digest(fs::read(run.join("vault/vault.db-wal")).unwrap())
    );
    assert_eq!(
        snapshot_before,
        Sha256::digest(fs::read(snapshot.join("vault.db")).unwrap())
    );
    let restored = run.join("restored");
    fs::create_dir(&restored).unwrap();
    let result = hotr::backup::restore(&snapshot, &restored.join("vault"), BACKUP_KEY).unwrap();
    assert_eq!(result["active_clients"], 0);
    assert_eq!(result["clients_invalidated"], 2);
    let mut restored_server = Server::start(&restored, "restored-owner");
    assert!(
        owner::request(&restored.join("vault"), owner::UNLOCK, BACKUP_KEY)
            .await
            .unwrap()
            .error
            .is_none()
    );
    for mut old in [a, b, c] {
        old.port = restored_server.port;
        assert_eq!(
            api::scoped_request(&old, "GET", "/v1/status", None)
                .await
                .unwrap()
                .0,
            401
        );
    }
    let (fresh, tf) = issue_cli(&restored, "reenrolled", "reader", "alpha");
    let record = api::scoped_request(
        &fresh,
        "POST",
        "/v1/records/get",
        Some(&json!({"namespace":"alpha","id":"accepted"})),
    )
    .await
    .unwrap();
    assert_eq!(record.0, 200);
    assert_eq!(record.1["revision"], 2);
    assert_eq!(record.1["state"], "accepted");
    assert_eq!(record.1["body"], BODY);
    assert_eq!(record.1["sources"], original["record"]["sources"]);
    assert_eq!(
        api::scoped_request(&fresh, "POST", "/v1/records", Some(&original))
            .await
            .unwrap()
            .0,
        403
    );
    assert_eq!(
        api::scoped_request(
            &fresh,
            "POST",
            "/v1/records/count",
            Some(&json!({"namespace":"beta"}))
        )
        .await
        .unwrap()
        .0,
        403
    );
    let searched = api::scoped_request(
        &fresh,
        "POST",
        "/v1/search",
        Some(&json!({"page":page("alpha",5,0),"query":"shared"})),
    )
    .await
    .unwrap();
    assert_eq!(searched.0, 200);
    assert_eq!(searched.1["total"], manifest.watermark.records);
    restored_server.stop(&restored).await;
    let db = hotr::schema::open(&restored.join("vault/vault.db"), BACKUP_KEY).unwrap();
    for (id, sequence) in receipts {
        let found = hotr::schema::revision(&db, "alpha", &id, None).unwrap();
        assert_eq!(
            found.is_some(),
            sequence <= manifest.watermark.audit_sequence,
            "snapshot watermark mismatch"
        );
    }
    assert_eq!(
        db.query_row("SELECT count(*) FROM write_receipts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        manifest.watermark.receipts
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM clients WHERE revoked=1", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        manifest.watermark.clients
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM client_grants WHERE namespace='alpha'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        manifest.watermark.grants + 1
    );
    drop(db);
    server.stop(&run).await;
    scan(
        &run,
        &[&ta, &tb, &tc, &tf, std::str::from_utf8(BACKUP_KEY).unwrap()],
    );
    write_new(&run.join("HOTR-11-recovery.json"),&serde_json::to_vec_pretty(&json!({"result":"PASS","binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),"concurrent_acknowledged_writes":200,"snapshot":manifest,"all_receipts_reconciled_to_snapshot_watermark":true,"different_encryption_key":true,"restored_old_clients_denied":3,"new_reader_reenrolled":true,"original_vault_and_snapshot_preserved_on_bad_restore":true,"rehashed_tamper_and_truncation_denied":true})).unwrap());
}
