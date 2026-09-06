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
    if args.as_slice() == ["hermes-preflight"] {
        return hermes_check(false);
    }
    if args.as_slice() == ["hermes-acceptance"] {
        return hermes_check(true);
    }
    if args.as_slice() == ["lamprey-smoke"] {
        return lamprey_check("smoke");
    }
    if args.as_slice() == ["lamprey-preflight"] {
        return lamprey_check("preflight");
    }
    if args.as_slice() == ["lamprey-acceptance"] {
        return lamprey_check("acceptance");
    }
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
                "HOTR-02"
                    | "HOTR-03"
                    | "HOTR-04"
                    | "HOTR-05"
                    | "HOTR-06"
                    | "HOTR-07"
                    | "HOTR-08"
                    | "HOTR-09"
                    | "HOTR-10"
                    | "HOTR-11"
                    | "HOTR-12"
                    | "HOTR-13"
                    | "HOTR-04-R2"
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
        if matches!(
            prompt,
            "HOTR-04"
                | "HOTR-05"
                | "HOTR-06"
                | "HOTR-07"
                | "HOTR-08"
                | "HOTR-09"
                | "HOTR-10"
                | "HOTR-11"
                | "HOTR-12"
                | "HOTR-04-R2"
        ) {
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
        if prompt == "HOTR-12" {
            let apps = run(
                &guard,
                &directory,
                "installed-applications",
                &cargo,
                &[
                    "test",
                    "--release",
                    "--locked",
                    "--test",
                    "api_capabilities",
                    "actual_codex_and_claude_shared_memory",
                    "--",
                    "--ignored",
                    "--nocapture",
                ],
                Duration::from_secs(1800),
            )?;
            let pass = apps.ensure_pass();
            outcomes.push(apps);
            pass?;
            hotr_xtask::ensure_required(&outcomes, &["installed-applications"])?;
        }
        if prompt == "HOTR-09" {
            let load = run(
                &guard,
                &directory,
                "prototype-load",
                &cargo,
                &[
                    "test",
                    "--release",
                    "--locked",
                    "--test",
                    "api_capabilities",
                    "prototype_10k_load_15_minutes",
                    "--",
                    "--ignored",
                    "--nocapture",
                ],
                Duration::from_secs(1800),
            )?;
            let pass = load.ensure_pass();
            outcomes.push(load);
            pass?;
            hotr_xtask::ensure_required(&outcomes, &["prototype-load"])?;
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

fn lamprey_check(mode: &str) -> io::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let guard = Guard::new(&root)?;
    let cpu_rate = limits::install_job_limits()?;
    let smoke = mode == "smoke";
    let prompt = if mode == "acceptance" {
        "HOTR-12-LAMPREY"
    } else if smoke {
        "HOTR-12-LAMPREY-SMOKE"
    } else {
        "HOTR-12-LAMPREY-PREFLIGHT"
    };
    let directory = guard.new_run(prompt)?;
    let source = hotr_xtask::snapshot(&guard)?;
    let budget_test = run(
        &guard,
        &directory,
        "prompt-budget",
        Path::new("node"),
        &["--test", "integrations/clients/prompt_budget.test.cjs"],
        Duration::from_secs(30),
    )?;
    budget_test.ensure_pass()?;
    let test = if mode == "acceptance" {
        "actual_lamprey_acceptance"
    } else if smoke {
        "actual_lamprey_smoke"
    } else {
        "installed_lamprey_preflight"
    };
    let outcome = run(
        &guard,
        &directory,
        "installed-lamprey-smoke",
        Path::new("cargo"),
        &[
            "test",
            "--release",
            "--locked",
            "--test",
            "api_capabilities",
            test,
            "--",
            "--ignored",
            "--nocapture",
        ],
        Duration::from_secs(if mode == "acceptance" { 1500 } else { 360 }),
    )?;
    let result = outcome.ensure_pass().and_then(|()| {
        if hotr_xtask::snapshot(&guard)? == source {
            Ok(())
        } else {
            Err(io::Error::other("source changed during smoke test"))
        }
    });
    write_new(
        &directory.join("manifest.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({
            "prompt":prompt, "result":if result.is_ok(){"PASS"}else{"FAIL"},
        "scope":if mode == "acceptance" { "Actual installed Lamprey save, recall, correction, owner acceptance, restart, selected-model switch, cancellation, recovery, forbidden namespace and revocation; independent scoped reader" } else if smoke { "One actual-app save/recall/correction/forbidden-scope smoke; full Lamprey prompt remains open" } else { "Installed application connection and schema preflight only; zero model prompts; full Lamprey gate remains open" },
        "source":source, "commands":[budget_test,outcome], "cpu_rate_per_10000":cpu_rate, "memory_limit_bytes":limits::MEMORY_BYTES,
            "product_sha256":hash(&root.join("work/hotr-build/target/release/hotr.exe"))?, "runner_sha256":hash(&std::env::current_exe()?)?
        }))?,
    )?;
    println!("Lamprey smoke evidence: {}", directory.display());
    result
}

fn hermes_check(full: bool) -> io::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let guard = Guard::new(&root)?;
    let cpu_rate = limits::install_job_limits()?;
    let prompt = if full {
        "HOTR-12A"
    } else {
        "HOTR-12A-PREFLIGHT"
    };
    let directory = guard.new_run(prompt)?;
    let source = hotr_xtask::snapshot(&guard)?;
    let mut outcomes = Vec::new();
    let result = (|| -> io::Result<()> {
        for (label, arguments) in [
            (
                "native-result-contract",
                vec!["--test", "integrations/clients/hermes_results.test.cjs"],
            ),
            (
                "driver-syntax",
                vec!["--check", "integrations/clients/hermes_cli.cjs"],
            ),
            (
                "prompt-budget",
                vec!["--test", "integrations/clients/prompt_budget.test.cjs"],
            ),
        ] {
            let outcome = run(
                &guard,
                &directory,
                label,
                Path::new("node"),
                &arguments,
                Duration::from_secs(30),
            )?;
            let pass = outcome.ensure_pass();
            outcomes.push(outcome);
            pass?;
        }
        let test = if full {
            "actual_hermes_acceptance"
        } else {
            "installed_hermes_preflight"
        };
        let outcome = run(
            &guard,
            &directory,
            "installed-hermes",
            Path::new("cargo"),
            &[
                "test",
                "--release",
                "--locked",
                "--test",
                "api_capabilities",
                test,
                "--",
                "--ignored",
                "--nocapture",
            ],
            Duration::from_secs(if full { 900 } else { 360 }),
        )?;
        let pass = outcome.ensure_pass();
        outcomes.push(outcome);
        pass?;
        if hotr_xtask::snapshot(&guard)? != source {
            return Err(io::Error::other("source changed during Hermes gate"));
        }
        Ok(())
    })();
    write_new(
        &directory.join("manifest.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({
            "prompt":prompt,"result":if result.is_ok(){"PASS"}else{"FAIL"},"failure":result.as_ref().err().map(ToString::to_string),
            "source":source,"commands":outcomes,"cpu_rate_per_10000":cpu_rate,"memory_limit_bytes":limits::MEMORY_BYTES,
            "product_sha256":hash(&root.join("work/hotr-build/target/release/hotr.exe"))?,"runner_sha256":hash(&std::env::current_exe()?)?,
            "scope":if full {"Actual installed Hermes save/recall/correction, forbidden namespace, owner acceptance, restart/search, revocation and independent reader"} else {"Installed Hermes native MCP discovery; zero model prompts"}
        }))?,
    )?;
    println!("Hermes evidence: {}", directory.display());
    result
}
