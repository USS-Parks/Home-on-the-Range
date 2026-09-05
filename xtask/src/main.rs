use hotr_xtask::{FIXTURE_SECRET, Guard, hash, limits, run, seeded_record, write_new};
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

fn main() {
    if let Err(error) = execute() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn execute() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("fixture") {
        match args.get(1).map(String::as_str) {
            Some("assertion") => {
                eprintln!("intentional synthetic assertion failure");
                std::process::exit(17);
            }
            Some("timeout") => std::thread::sleep(Duration::from_secs(60)),
            Some("secret") => println!("fixture credential: {FIXTURE_SECRET}"),
            Some("spam") => {
                for _ in 0..20_000 {
                    println!("{}", "x".repeat(1024));
                }
            }
            _ => return Err(io::Error::other("unknown fixture")),
        }
        return Ok(());
    }
    let fault = args.first().map(String::as_str) == Some("fault");
    if !fault
        && (args.len() != 3
            || args[0] != "verify"
            || args[1] != "--prompt"
            || !matches!(
                args[2].as_str(),
                "HOTR-02" | "HOTR-03" | "HOTR-04" | "HOTR-05" | "HOTR-06" | "HOTR-04-R2"
            ))
    {
        return Err(io::Error::other(
            "Usage: cargo xtask verify --prompt HOTR-02|HOTR-03|HOTR-04|HOTR-05",
        ));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let guard = Guard::new(&root)?;
    let cpu_rate = limits::install_job_limits()?;
    let prompt = if fault { "HOTR-03-fault" } else { &args[2] };
    let directory = guard.new_run(prompt)?;
    let executable = std::env::current_exe()?;
    let source = hotr_xtask::snapshot(&guard)?;
    let mut outcomes = Vec::new();
    let result = (|| -> io::Result<()> {
        if fault {
            let kind = args.get(1).map(String::as_str).unwrap_or("assertion");
            if !matches!(kind, "assertion" | "timeout") {
                return Err(io::Error::other("unknown fault"));
            }
            let outcome = run(
                &guard,
                &directory,
                "injected-fault",
                &executable,
                &["fixture", kind],
                Duration::from_millis(250),
            )?;
            let pass = outcome.ensure_pass();
            outcomes.push(outcome);
            return pass;
        }
        if prompt == "HOTR-03" {
            for (label, kind, seconds) in [
                ("reject-assertion", "assertion", 10),
                ("reject-timeout", "timeout", 1),
                ("reject-log-flood", "spam", 10),
            ] {
                let outcome = run(
                    &guard,
                    &directory,
                    label,
                    &executable,
                    &["fixture", kind],
                    Duration::from_secs(seconds),
                )?;
                if outcome.ensure_pass().is_ok() {
                    return Err(io::Error::other("negative control incorrectly passed"));
                }
                let expected = match kind {
                    "assertion" => outcome.exit_code == Some(17) && outcome.failure.is_none(),
                    "timeout" => outcome.failure.as_deref() == Some("timeout"),
                    "spam" => outcome.failure.as_deref() == Some("log limit"),
                    _ => false,
                };
                if !expected {
                    return Err(io::Error::other(
                        "negative control failed for an unexpected reason",
                    ));
                }
                if std::fs::metadata(directory.join(format!("{label}.txt")))?.len()
                    > hotr_xtask::LOG_LIMIT as u64
                {
                    return Err(io::Error::other("stored log exceeded its byte ceiling"));
                }
                outcomes.push(outcome);
            }
            let secret = run(
                &guard,
                &directory,
                "redaction",
                &executable,
                &["fixture", "secret"],
                Duration::from_secs(10),
            )?;
            secret.ensure_pass()?;
            outcomes.push(secret);
            if std::fs::read_to_string(directory.join("redaction.txt"))?.contains(FIXTURE_SECRET) {
                return Err(io::Error::other("redaction failed"));
            }
            if guard.checked(Path::new("../outside")).is_ok()
                || guard.checked(Path::new("work/../../outside")).is_ok()
            {
                return Err(io::Error::other("path guard failed"));
            }
            let mut child = Command::new(&executable)
                .args(["fixture", "timeout"])
                .spawn()?;
            let id = child.id();
            let refused = hotr_xtask::terminate_owned(&mut child, std::process::id()).is_err();
            hotr_xtask::terminate_owned(&mut child, id)?;
            if !refused {
                return Err(io::Error::other("PID guard failed"));
            }
            let sample: Vec<_> = (0..100).map(|i| seeded_record(47821, i)).collect();
            write_new(
                &directory.join("seeded-fixtures.json"),
                serde_json::to_vec(&sample)?.as_slice(),
            )?;
        }
        let cargo = std::env::var_os("CARGO")
            .map(PathBuf::from)
            .unwrap_or_else(|| "cargo".into());
        let specs: Vec<(&str, Vec<&str>)> = vec![
            ("format", vec!["fmt", "--check"]),
            ("build", vec!["build", "--release", "--locked"]),
            (
                "clippy",
                vec![
                    "clippy",
                    "--release",
                    "--locked",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            ),
            (
                "tests",
                vec!["test", "--release", "--locked", "--", "--test-threads=1"],
            ),
            (
                "harness-format",
                vec!["fmt", "--manifest-path", "xtask/Cargo.toml", "--check"],
            ),
            (
                "harness-clippy",
                vec![
                    "clippy",
                    "--manifest-path",
                    "xtask/Cargo.toml",
                    "--release",
                    "--locked",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            ),
            (
                "harness-tests",
                vec![
                    "test",
                    "--lib",
                    "--manifest-path",
                    "xtask/Cargo.toml",
                    "--release",
                    "--locked",
                    "--",
                    "--test-threads=1",
                ],
            ),
        ];
        for (label, command) in specs {
            let outcome = run(
                &guard,
                &directory,
                label,
                &cargo,
                &command,
                Duration::from_secs(600),
            )?;
            let pass = outcome.ensure_pass();
            outcomes.push(outcome);
            pass?;
        }
        if matches!(prompt, "HOTR-04" | "HOTR-05" | "HOTR-06" | "HOTR-04-R2") {
            let boundary = run(
                &guard,
                &directory,
                "owner-boundary",
                &cargo,
                &[
                    "test",
                    "--release",
                    "--locked",
                    "--test",
                    "owner_lifecycle",
                    "live_second_principal_boundary",
                    "--",
                    "--ignored",
                    "--nocapture",
                ],
                Duration::from_secs(240),
            )?;
            let pass = boundary.ensure_pass();
            outcomes.push(boundary);
            pass?;
            hotr_xtask::ensure_required(&outcomes, &["owner-boundary"])?;
        }
        let scan = run(
            &guard,
            &directory,
            "canary-scan",
            Path::new("python"),
            &["-I", "-B", ".cargo/scan-native-canaries.py"],
            Duration::from_secs(60),
        )?;
        let pass = scan.ensure_pass();
        outcomes.push(scan);
        pass?;
        hotr_xtask::ensure_required(
            &outcomes,
            &[
                "format",
                "build",
                "clippy",
                "tests",
                "harness-format",
                "harness-clippy",
                "harness-tests",
                "canary-scan",
            ],
        )
    })();
    let result = result.and_then(|()| {
        if hotr_xtask::snapshot(&guard)? == source {
            Ok(())
        } else {
            Err(io::Error::other("source changed during gate"))
        }
    });
    let report = serde_json::json!({"schema_version":1, "prompt":prompt, "result":if result.is_ok(){"PASS"}else{"FAIL"},
        "failure":result.as_ref().err().map(ToString::to_string), "source":source,
        "seed":47821, "runner_sha256":hash(&executable)?, "product_sha256":hash(&root.join("work/hotr-build/target/release/hotr.exe")).ok(),
        "memory_limit_bytes":limits::MEMORY_BYTES, "disk_limit_bytes":limits::MAX_DISK_BYTES, "minimum_free_bytes":limits::MIN_FREE_BYTES,
        "cpu_rate_per_10000":cpu_rate, "cpu_workers":4, "command_log_limit_bytes":hotr_xtask::LOG_LIMIT,
        "platform":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH},
        "commands":outcomes, "source_hash_policy":"SHA-256 of UTF-8 source normalized to LF and exact native/binary bytes; unchanged source required during gate", "hosted_ci":"NOT RUN"});
    write_new(
        &directory.join("manifest.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "{}: {}",
        prompt,
        if result.is_ok() { "PASS" } else { "FAIL" }
    );
    println!("Evidence: {}", directory.display());
    result
}
