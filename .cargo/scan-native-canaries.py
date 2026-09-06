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
    # Support retained v1 runs plus v2's platform-independent run identifier.
    for seed in [str(run), run.name, "\\\\?\\" + str(run)]:
        key = "synthetic-key-" + hashlib.sha256(seed.encode()).hexdigest()
        canary = "hotrcanary" + hashlib.sha256(key.encode()).hexdigest()
        patterns.extend([key.encode(), canary.encode(), canary.encode("utf-16-le")])
    report = run / "result.json"
    if report.exists() and json.loads(report.read_text())["result"] == "PASS":
        completed += 1

assert completed, "no passing encryption result manifest"
owner_runs = sorted((root / "work/hotr-tests").glob("HOTR-04-*"))
for run in owner_runs:
    assert run.resolve().is_relative_to(root.resolve()), "owner test path escaped project"
    assert (run / "SYNTHETIC-ONLY").read_text().startswith("HOTR-04;"), "unowned owner run"
owner_key = "HOTR-synthetic-owner-secret-649a5bd8"
patterns.extend([owner_key.encode(), owner_key.encode("utf-16-le")])
schema_runs = sorted((root / "work/hotr-tests").glob("HOTR-05-*"))
for run in schema_runs:
    assert run.resolve().is_relative_to(root.resolve()), "schema test path escaped project"
    assert (run / "SYNTHETIC-ONLY").read_text().startswith("HOTR-05;"), "unowned schema run"
for canary in ["HOTR-05-synthetic-key-373a2b7d", "HOTR05canary"]:
    patterns.extend([canary.encode(), canary.encode("utf-16-le")])
checked = 0
writer_runs = sorted((root / "work/hotr-tests").glob("HOTR-06-*"))
for run in writer_runs:
    assert run.resolve().is_relative_to(root.resolve()), "writer test path escaped project"
    assert (run / "SYNTHETIC-ONLY").read_text().startswith("HOTR-06;"), "unowned writer run"
for canary in ["HOTR-06-synthetic-key-493bf46e", "HOTR06canary"]:
    patterns.extend([canary.encode(), canary.encode("utf-16-le")])
overlap = max(map(len, patterns)) - 1
capability_runs = sorted((root / "work/hotr-tests").glob("HOTR-07-*"))
for run in capability_runs:
    assert run.resolve().is_relative_to(root.resolve()), "capability test path escaped project"
    assert (run / "SYNTHETIC-ONLY").read_text().startswith("HOTR-07;"), "unowned capability run"
for canary in ["HOTR-07-synthetic-key-866bc4ad", "HOTR07canary", "HOTR11BackupKey-8bcd49c1-different", "HOTR18BackupKey-different-4e341"]:
    patterns.extend([canary.encode(), canary.encode("utf-16-le")])
hybrid_runs = sorted((root / "work/hotr-tests").glob("HOTR-16-*"))
for run in hybrid_runs:
    assert run.resolve().is_relative_to(root.resolve()), "hybrid test path escaped project"
    assert (run / "SYNTHETIC-ONLY").read_text().startswith("HOTR-16;"), "unowned hybrid run"
for canary in ["HOTR-16-synthetic-key-718634", "HOTR16canary"]:
    patterns.extend([canary.encode(), canary.encode("utf-16-le")])
# Search each distinct short prefix once, then confirm the complete literal.
# This preserves exact byte matching while avoiding a full-file pass per key.
pattern_groups = {}
for pattern in set(patterns):
    pattern_groups.setdefault(pattern[:8], []).append(pattern)


def contains_canary(window):
    for prefix, candidates in pattern_groups.items():
        position = window.find(prefix)
        while position != -1:
            if any(window.startswith(pattern, position) for pattern in candidates):
                return True
            position = window.find(prefix, position + 1)
    return False


# Validate every actual UTF-8/UTF-16 pattern and reject prefix-only false matches.
assert not contains_canary(b"\x00" * 1024), "matcher negative control failed"
for pattern in set(patterns):
    assert contains_canary(b"\x00" + pattern + b"\x00"), "matcher positive control failed"
    assert not contains_canary(pattern[:8] + b"\x00" * 512), "matcher prefix control failed"
overlap = max(map(len, patterns)) - 1
for directory in [*hybrid_runs, *runs, *owner_runs, *schema_runs, *writer_runs, *capability_runs, root / "work/hotr-build/tmp", root / "work/hotr-evidence"]:
    for path in directory.rglob("*"):
        assert not path.is_symlink(), "refusing symlink in test scan"
        assert path.resolve().is_relative_to(root.resolve()), "scan path escaped project"
        if not path.is_file():
            continue
        previous = b""
        with path.open("rb") as handle:
            while chunk := handle.read(1024 * 1024):
                window = previous + chunk
                assert not contains_canary(window), "plaintext canary found"
                previous = window[-overlap:]
        checked += 1
print(json.dumps({"prompt": "HOTR-02", "result": "PASS", "scan_files": checked,
                  "passing_runs": completed, "storage_temp_logs_canary_absent": True,
                  "distinct_patterns": len(set(patterns)), "prefix_groups": len(pattern_groups),
                  "matching": "exact literals with cross-chunk overlap", "matcher_controls": "PASS"}))
