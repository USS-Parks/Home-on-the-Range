"""Scan only project-owned synthetic storage, temp, and evidence; print no secrets."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).absolute().parent.parent
runs = sorted((root / "work/hotr-tests").glob("HOTR-02-*"))
assert runs, "encryption gate has not produced a synthetic run"
patterns = []
completed = 0
for run in runs:
    assert run.resolve().is_relative_to(root.resolve()), "test path escaped project"
    assert (run / "SYNTHETIC-ONLY").read_text().startswith("HOTR-02;"), "unowned run"
    key = "synthetic-key-" + hashlib.sha256(str(run).encode()).hexdigest()
    canary = "hotrcanary" + hashlib.sha256(key.encode()).hexdigest()
    patterns.extend([key.encode(), canary.encode(), canary.encode("utf-16-le")])
    report = run / "result.json"
    if report.exists() and json.loads(report.read_text())["result"] == "PASS":
        completed += 1

assert completed, "no passing encryption result manifest"
checked = 0
overlap = max(map(len, patterns)) - 1
for directory in [*runs, root / "work/hotr-build/tmp", root / "work/hotr-evidence"]:
    for path in directory.rglob("*"):
        assert not path.is_symlink(), "refusing symlink in test scan"
        assert path.resolve().is_relative_to(root.resolve()), "scan path escaped project"
        if not path.is_file():
            continue
        previous = b""
        with path.open("rb") as handle:
            while chunk := handle.read(1024 * 1024):
                window = previous + chunk
                assert not any(pattern in window for pattern in patterns), "plaintext canary found"
                previous = window[-overlap:]
        checked += 1
print(json.dumps({"prompt": "HOTR-02", "result": "PASS", "scan_files": checked,
                  "passing_runs": completed, "storage_temp_logs_canary_absent": True}))
