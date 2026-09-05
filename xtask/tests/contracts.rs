use hotr_xtask::{FIXTURE_SECRET, Guard, check_budget, limits, redact, seeded_record};
use std::path::{Path, PathBuf};

#[test]
fn generation_is_reproducible_and_seed_sensitive() {
    assert_eq!(seeded_record(47821, 9), seeded_record(47821, 9));
    assert_ne!(seeded_record(47821, 9), seeded_record(47822, 9));
    assert_ne!(seeded_record(47821, 9), seeded_record(47821, 10));
    assert!(seeded_record(47821, 9).len() >= 1024);
    assert!(
        seeded_record(47821, 9)
            .contains("0541df8cee13319a19ba41e46b0a3424bfba2b1d8720ce20b124f9b5ded82389")
    );
}

#[test]
fn traversal_absolute_paths_and_non_work_paths_fail() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let guard = Guard::new(&root).unwrap();
    for path in ["../other", "work/../other", "C:/Windows", "src/lib.rs"] {
        assert!(guard.checked(Path::new(path)).is_err());
    }
    assert!(
        guard
            .checked(Path::new("work/hotr-tests/new-fixture"))
            .is_ok()
    );
}

#[test]
fn logs_redact_registered_credentials_and_drop_truncated_lines() {
    let raw = format!("first\ncredential={FIXTURE_SECRET}\nAuthorization: Bearer private\n");
    let clean = redact(raw.as_bytes(), &[FIXTURE_SECRET], false);
    assert!(!clean.contains(FIXTURE_SECRET));
    assert!(!clean.contains("Bearer private"));
    assert!(clean.contains("REDACTED"));
    assert_eq!(
        redact(b"safe\npartial-secret", &["partial-secret-longer"], true),
        "safe"
    );
}

#[test]
fn disk_limits_fail_closed_at_the_recorded_thresholds() {
    assert!(check_budget(limits::MAX_DISK_BYTES, limits::MIN_FREE_BYTES).is_err());
    assert!(check_budget(0, limits::MIN_FREE_BYTES - 1).is_err());
    assert!(check_budget(1, limits::MIN_FREE_BYTES).is_ok());
}

#[test]
fn skipped_required_commands_cannot_pass() {
    assert!(hotr_xtask::ensure_required(&[], &["required-build"]).is_err());
}
