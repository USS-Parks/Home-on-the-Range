# Verification ledger

HOTR-01 foundation and HOTR-02 native Windows encryption gates have passed. The shared service is not installed or accepted.

| Gate | Required evidence | Status |
|---|---|---|
| HOTR-00 | Plan structure, approval boundary, clean diff, authorized main publication | PASS — `b2c900519569bc2288d4d4f4e18c6cc2a6171f1a` verified on origin/main |
| HOTR-01 | Pinned reuse review, requirement gaps, user-ratified SQLCipher architecture, bounded-write approval, baseline preservation | PASS — `b6a2ae04332d6dac5700aba4659697730e575e37` verified on origin/main |
| HOTR-02 | Native release build, actual cipher/provider versions, FTS/WAL/reopen/integrity, wrong-key/keyless/plain-SQLite/tamper rejection, storage/temp/log scan | PASS locally; [native evidence](evidence/HOTR-02-native.json); publication follows |
| M1 / HOTR-01–12 | Actual encrypted Windows service, scoped access, keyword retrieval, two real clients, restored encrypted backup | In progress; HOTR-01/02 passed, HOTR-03 next |
| M2 / HOTR-13–18 | Selected imports, correction consistency, local embeddings, hybrid retrieval evaluation, management UI | Not started |
| M3 / HOTR-19–22 | Auditing, key rotation, Windows lifecycle, controlled daily-use comparison | Not started |
| M4 / HOTR-23–32 | Security, malformed inputs, races, crashes, storage faults, model faults, scale, soak, dependency review, repairs | Not started |
| M5 / HOTR-33–36 | Packaging, clean installation, hosted builds, deployability decision | Not started |

For each run record: prompt/gate ID, UTC timestamp, source SHA and dirty status, executable SHA-256, dependency/model versions and hashes, OS and resource allocation, dataset and seed, command/arguments with secrets excluded, sample counts, percentiles, errors/timeouts, correctness assertions, limits/abort conditions, evidence paths, and pass/fail/blocked result.

Use `PASS`, `FAIL`, `BLOCKED`, or `NOT RUN`. Do not convert skipped checks into passes. Preserve complete failure output locally with restricted access; produce a redacted public-to-the-repository summary. Local/private and synthetic evidence have separate retention rules.

Artifacts are planned under `docs/evidence/` for sanitized summaries and `work/evidence/` for local raw synthetic logs. Runtime private data is excluded from Git regardless of repository visibility.

HOTR-00 verified 36 unique sequential prompts, all required fields, dependency ordering, relative links, explicit approval boundary, seven staged governance/documentation files, credential-pattern scan, whitespace, and repository commit/push hooks. No implementation, encryption, integration, security, stress, recovery, or hosted build gate has run. See DEVLOG for the initial document-check corrections and retained diagnostic path.
