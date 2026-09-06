use hotr::{owner, windows_security};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SECRET: &str = "HOTR-synthetic-owner-secret-649a5bd8";

fn run_dir() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("work/hotr-tests");
    fs::create_dir_all(&base).unwrap();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = base.join(format!("HOTR-04-{}-{stamp}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    fs::write(
        directory.join("SYNTHETIC-ONLY"),
        b"HOTR-04; synthetic owner boundary fixtures\n",
    )
    .unwrap();
    directory
}

struct Server {
    child: Child,
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
    fn start(directory: &Path, label: &str, port: u16) -> (Self, serde_json::Value) {
        let stderr = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(
                directory
                    .parent()
                    .unwrap()
                    .join(format!("{label}.stderr.txt")),
            )
            .unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_hotr"))
            .arg("serve")
            .arg(directory)
            .args(["--port", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .unwrap();
        let mut server = Self { child };
        let stdout = server.child.stdout.take().unwrap();
        let (send, receive) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut line = String::new();
            let _ = BufReader::new(stdout).read_line(&mut line);
            let _ = send.send(line);
        });
        let line = receive
            .recv_timeout(Duration::from_secs(10))
            .expect("server readiness timeout");
        let ready = serde_json::from_str(&line).unwrap_or_else(|_| {
            panic!("server startup failed; inspect retained {label}.stderr.txt")
        });
        (server, ready)
    }
    fn wait_exit(&mut self) {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(7),
                "key holder did not exit"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn actual_owner_lifecycle_and_preservation() {
    let run = run_dir();
    let vault = run.join("vault");
    owner::create(&vault, SECRET.as_bytes()).unwrap();
    windows_security::verify_file_owner(&vault, true).unwrap();
    windows_security::verify_file_owner(&vault.join("vault.db"), false).unwrap();
    let original = Sha256::digest(fs::read(vault.join("vault.db")).unwrap());
    assert!(owner::create(&vault, b"different-synthetic-passphrase").is_err());
    assert_eq!(
        original,
        Sha256::digest(fs::read(vault.join("vault.db")).unwrap())
    );
    let (mut server, ready) = Server::start(&vault, "first", 0);
    assert_eq!(ready["state"], "locked");
    let locked = owner::request(&vault, owner::STATUS, &[]).await.unwrap();
    assert_eq!(locked.state, "locked");
    assert_eq!(locked.pid, server.child.id());
    let duplicate = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .arg("serve")
        .arg(&vault)
        .args(["--port", "0"])
        .output()
        .unwrap();
    assert!(
        !duplicate.status.success(),
        "duplicate named pipe instance accepted"
    );
    fs::write(run.join("duplicate.stderr.txt"), &duplicate.stderr).unwrap();
    let collision = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .arg("serve")
        .arg(&vault)
        .args(["--port", &ready["port"].as_u64().unwrap().to_string()])
        .output()
        .unwrap();
    assert!(
        !collision.status.success(),
        "occupied loopback port accepted"
    );
    fs::write(run.join("port-collision.stderr.txt"), &collision.stderr).unwrap();
    let start = Instant::now();
    let wrong = owner::request(&vault, owner::UNLOCK, b"deliberately-wrong-passphrase")
        .await
        .unwrap();
    assert_eq!(wrong.state, "locked");
    assert!(wrong.error.is_some());
    assert!(start.elapsed() < Duration::from_secs(5));
    for (length, body) in [
        (0, vec![]),
        ((hotr::api::MAX_REQUEST + 2) as u32, vec![]),
        (1, vec![255]),
        (2, vec![owner::STATUS, 0]),
    ] {
        let mut client = owner::connect(&vault).await.unwrap();
        client.write_u32_le(length).await.unwrap();
        client.write_all(&body).await.unwrap();
        let reply_length = client.read_u32_le().await.unwrap();
        assert!(reply_length <= 4096);
        let mut bytes = vec![0; reply_length as usize];
        client.read_exact(&mut bytes).await.unwrap();
        let reply: owner::Reply = serde_json::from_slice(&bytes).unwrap();
        client.write_u8(1).await.unwrap();
        assert!(reply.error.is_some());
        assert_eq!(reply.state, "locked");
    }
    // Keep a consumed client endpoint alive briefly after acknowledgement.
    // Windows can retain that instance until both sides have actually closed.
    let mut retiring = owner::connect(&vault).await.unwrap();
    retiring.write_u32_le(1).await.unwrap();
    retiring.write_u8(owner::STATUS).await.unwrap();
    let length = retiring.read_u32_le().await.unwrap();
    let mut bytes = vec![0; length as usize];
    retiring.read_exact(&mut bytes).await.unwrap();
    retiring.write_u8(1).await.unwrap();
    let (during_retirement, ()) = tokio::join!(owner::request(&vault, owner::STATUS, &[]), async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        drop(retiring);
    });
    assert_eq!(during_retirement.unwrap().state, "locked");
    for _ in 0..4096 {
        assert_eq!(
            owner::request(&vault, owner::STATUS, &[])
                .await
                .unwrap()
                .state,
            "locked"
        );
    }
    let unlocked = owner::request(&vault, owner::UNLOCK, SECRET.as_bytes())
        .await
        .unwrap();
    assert!(unlocked.error.is_none());
    assert_eq!(unlocked.state, "unlocked");
    let closed = owner::request(&vault, owner::LOCK, &[]).await.unwrap();
    assert!(closed.closing);
    server.wait_exit();
    assert!(
        fs::read(run.join("first.stderr.txt")).unwrap().is_empty(),
        "owner diagnostics must not emit native wrong-key errors"
    );
    assert!(owner::request(&vault, owner::STATUS, &[]).await.is_err());
    let (mut restarted, ready) = Server::start(&vault, "restart", 0);
    assert_eq!(ready["state"], "locked");
    owner::request(&vault, owner::LOCK, &[]).await.unwrap();
    restarted.wait_exit();
    fs::write(
        run.join("lifecycle.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "result":"PASS", "pid":locked.pid, "restart_pid":ready["pid"],
            "owner_sid":windows_security::current_sid().unwrap(), "preservation":true,
            "wrong_unlock_bounded":true, "lock_process_exit":true, "duplicate_pipe_rejected":true,
            "occupied_port_rejected":true, "second_authenticated_principal":"NOT RUN"
        }))
        .unwrap(),
    )
    .unwrap();
}

/// Run separately while the reviewed probe executes under a different account.
/// A missing probe times out and fails; this test is never a mock substitute.
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a live second authenticated Windows account"]
async fn live_second_principal_boundary() {
    let run = run_dir();
    let vault = run.join("vault");
    owner::create(&vault, SECRET.as_bytes()).unwrap();
    let (mut server, _) = Server::start(&vault, "principal", 0);
    owner::request(&vault, owner::UNLOCK, SECRET.as_bytes())
        .await
        .unwrap();
    let enrolled = owner::admin(
        &vault,
        &owner::AdminRequest::Issue(hotr::capabilities::NewClient {
            label: "synthetic DPAPI boundary".into(),
            role: hotr::capabilities::Role::Reader,
            namespaces: vec!["dpapi-proof".into()],
        }),
    )
    .await
    .unwrap();
    assert!(enrolled.error.is_none());
    let profile: hotr::credentials::CredentialProfile =
        serde_json::from_value(enrolled.data.unwrap()).unwrap();
    let token = hotr::credentials::unprotect(&profile).unwrap();
    assert!(hotr::credentials::token_hash(&token).is_some());
    let original = Sha256::digest(fs::read(vault.join("vault.db")).unwrap());
    let owner_sid = windows_security::current_sid().unwrap();
    let challenge = serde_json::json!({
        "owner_sid": owner_sid, "server_pid": server.child.id(),
        "directory": vault, "database": vault.join("vault.db"),
        "marker": vault.join(".hotr-vault"), "pipe": owner::pipe_name(&vault).unwrap(),
        "receipt": run.join("second-principal.json"),
        "protected_token": profile.protected_token,
        "fake_tcp_port": run.join("fake-tcp-port.json")
    });
    fs::write(
        run.join("challenge.json"),
        serde_json::to_vec_pretty(&challenge).unwrap(),
    )
    .unwrap();
    println!(
        "SECOND_PRINCIPAL_CHALLENGE={}",
        run.join("challenge.json").display()
    );
    let start = Instant::now();
    let receipt_path = run.join("second-principal.json");
    let fake_port_path = run.join("fake-tcp-port.json");
    let mut fake_peer_attempted = false;
    while !receipt_path.exists() {
        if !fake_peer_attempted && fake_port_path.exists() {
            let port: u16 = serde_json::from_slice(&fs::read(&fake_port_path).unwrap()).unwrap();
            let mut fake_profile = profile.clone();
            fake_profile.port = port;
            let error = hotr::api::scoped_request(&fake_profile, "GET", "/v1/status", None)
                .await
                .unwrap_err();
            assert!(
                matches!(error.kind(), std::io::ErrorKind::PermissionDenied),
                "foreign TCP peer must be denied before authentication: {error}"
            );
            fake_peer_attempted = true;
        }
        assert!(
            start.elapsed() < Duration::from_secs(180),
            "live second-principal evidence missing"
        );
        assert!(
            server.child.try_wait().unwrap().is_none(),
            "owner exited during probe"
        );
        thread::sleep(Duration::from_millis(100));
    }
    // The probe publishes only after closing its complete JSON file.
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(receipt_path).unwrap()).unwrap();
    assert_eq!(receipt["authenticated"], true);
    assert!(fake_peer_attempted);
    assert_eq!(receipt["fake_tcp"]["accepted"], true);
    assert_eq!(receipt["fake_tcp"]["application_bytes"], 0);
    assert_ne!(receipt["sid"].as_str().unwrap(), owner_sid);
    assert_eq!(
        receipt["dpapi"]["denied"], true,
        "copied credential must reject a different authenticated account"
    );
    for name in ["directory", "database", "marker", "pipe"] {
        assert_eq!(
            receipt[name]["win32_error"], 5,
            "{name} must fail specifically with access denied"
        );
        assert_eq!(receipt[name]["denied"], true);
    }
    assert_eq!(
        original,
        Sha256::digest(fs::read(vault.join("vault.db")).unwrap())
    );
    assert_eq!(
        owner::request(&vault, owner::STATUS, &[])
            .await
            .unwrap()
            .state,
        "unlocked"
    );
    owner::request(&vault, owner::LOCK, &[]).await.unwrap();
    server.wait_exit();
    fs::write(
        run.join("boundary.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
        "result":"PASS", "owner_sid":owner_sid, "peer_sid":receipt["sid"],
        "binary_sha256":format!("{:x}", Sha256::digest(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap())),
            "server_pid":server.child.id(), "authenticated_peer":true,
            "files_access_denied":true, "pipe_access_denied":true,
            "vault_unchanged":true, "owner_still_unlocked_after_probe":true,
            "key_holder_exited":true
            ,"copied_dpapi_credential_denied":true,
            "foreign_tcp_peer_rejected_before_authentication":true
        }))
        .unwrap(),
    )
    .unwrap();
}

fn console_command(
    run: &Path,
    label: &str,
    arguments: &[&std::ffi::OsStr],
    prompts: &[&str],
) -> String {
    let system = native_pty_system();
    let pair = system
        .openpty(PtySize {
            rows: 24,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_hotr"));
    for arg in arguments {
        command.arg(arg);
    }
    command.cwd(env!("CARGO_MANIFEST_DIR"));
    let mut child = pair.slave.spawn_command(command).unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (send, receive) = mpsc::sync_channel(32);
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(length) => {
                    if send.send(buffer[..length].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let mut output = Vec::new();
    let mut next_prompt = 0;
    let mut cursor_answered = false;
    let start = Instant::now();
    let result = loop {
        if let Ok(chunk) = receive.recv_timeout(Duration::from_millis(20)) {
            output.extend_from_slice(&chunk);
        }
        if !cursor_answered && output.windows(4).any(|window| window == b"\x1b[6n") {
            // ConPTY requests the initial terminal cursor location before launch.
            writer.write_all(b"\x1b[1;1R").unwrap();
            writer.flush().unwrap();
            cursor_answered = true;
        }
        if output.len() > 64 * 1024 || start.elapsed() > Duration::from_secs(20) {
            let _ = child.kill();
            let _ = child.wait();
            break false;
        }
        if next_prompt < prompts.len()
            && String::from_utf8_lossy(&output).contains(prompts[next_prompt])
        {
            writer.write_all(SECRET.as_bytes()).unwrap();
            writer.write_all(b"\r").unwrap();
            writer.flush().unwrap();
            next_prompt += 1;
        }
        if let Some(status) = child.try_wait().unwrap() {
            break status.success();
        }
    };
    while let Ok(chunk) = receive.try_recv() {
        output.extend_from_slice(&chunk);
    }
    let text = String::from_utf8_lossy(&output).into_owned();
    fs::write(
        run.join(format!("{label}.console.txt")),
        text.replace(SECRET, "[REDACTED SYNTHETIC SECRET]"),
    )
    .unwrap();
    assert!(
        result,
        "console command failed; inspect retained {label}.console.txt"
    );
    assert_eq!(next_prompt, prompts.len());
    assert!(
        !text.contains(SECRET),
        "passphrase echoed to a real console"
    );
    text
}

#[tokio::test(flavor = "current_thread")]
async fn actual_windows_console_never_echoes_passphrases() {
    let run = run_dir();
    let vault = run.join("console-vault");
    let output = console_command(
        &run,
        "create",
        &["create".as_ref(), vault.as_os_str()],
        &["New vault passphrase:", "Confirm passphrase:"],
    );
    assert!(output.contains("Vault created and locked."));
    let (mut server, _) = Server::start(&vault, "console-server", 0);
    let unlocked = console_command(
        &run,
        "unlock",
        &["unlock".as_ref(), vault.as_os_str()],
        &["Vault passphrase:"],
    );
    assert!(unlocked.contains("unlocked"));
    let snapshot = run.join("console-snapshot");
    let backed_up = console_command(
        &run,
        "backup",
        &["backup".as_ref(), vault.as_os_str(), snapshot.as_os_str()],
        &["New backup passphrase:", "Confirm backup passphrase:"],
    );
    assert!(backed_up.contains("ciphertext_sha256"));
    let restored = run.join("console-restored");
    let recovered = console_command(
        &run,
        "restore",
        &[
            "restore".as_ref(),
            snapshot.as_os_str(),
            restored.as_os_str(),
        ],
        &["Backup passphrase:"],
    );
    assert!(recovered.contains("reenrollment_required"));
    owner::validate(&restored).unwrap();
    owner::request(&vault, owner::LOCK, &[]).await.unwrap();
    server.wait_exit();
    fs::write(
        run.join("console.json"),
        b"{\"result\":\"PASS\",\"transport\":\"Windows ConPTY\",\"passphrases_echoed\":false}",
    )
    .unwrap();
}
