# Verification ledger

HOTR-01–05 local gates have passed. HOTR-04-R1 awaits its hosted repair gate. The shared context service is not installed or accepted; hosted CI is tracked separately.

| Gate | Required evidence | Status |
|---|---|---|
| HOTR-00 | Plan structure, approval boundary, clean diff, authorized main publication | PASS — `b2c900519569bc2288d4d4f4e18c6cc2a6171f1a` verified on origin/main |
| HOTR-01 | Pinned reuse review, requirement gaps, user-ratified SQLCipher architecture, bounded-write approval, baseline preservation | PASS — `b6a2ae04332d6dac5700aba4659697730e575e37` verified on origin/main |
| HOTR-02 | Native release build, actual cipher/provider versions, FTS/WAL/reopen/integrity, wrong-key/keyless/plain-SQLite/tamper rejection, storage/temp/log scan | PASS — `dcc54f2c0becc8b0542cdec3dd1c95535a5298a7` verified on origin/main; [native evidence](evidence/HOTR-02-native.json) |
| HOTR-03 | Failed assertion/timeout/log-flood refusal, live Windows resource limits, path/PID guards, seed/redaction/required-command tests, exact source/binary hashes, minimal CI | PASS locally and hosted — `c95bffb8e068abd0f345762ca806fce49402a875` verified on origin/main; [harness evidence](evidence/HOTR-03-harness.json); [exact-commit Windows run](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33991495729) |
| HOTR-04 | Actual no-echo ConPTY create/unlock, protected ACLs, bounded wrong/malformed requests, duplicate pipe/port refusal, lock/process exit, real second authenticated Windows principal denial | PASS locally at `1e049d27ec1f915dd54498fc44f6231ec934cee7` — [owner evidence](evidence/HOTR-04-owner.json); [hosted run 33992714760](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33992714760) FAIL at literal ACL comparison |
| HOTR-04-R1 | Structural SID/ACE verification, alias/order and deny cases, repeated real owner/two-account gate, exact-commit Windows CI | PASS locally — [combined evidence](evidence/HOTR-05-and-04-R1.json); hosted repair acceptance PENDING |
| HOTR-05 | Versioned encrypted records, migrations preserving history, Unicode/byte limits, relation constraints, opaque sources, future closed/WAL files untouched | PASS locally — [final combined evidence](evidence/HOTR-05-and-04-R1.json); [earlier schema checkpoint](evidence/HOTR-05-schema.json) retained before the ACL repair |
| M1 / HOTR-01–12 | Actual encrypted Windows service, scoped access, keyword retrieval, two real clients, restored encrypted backup | In progress; await HOTR-04-R1 hosted acceptance, then HOTR-06 |
| M2 / HOTR-13–18 | Selected imports, correction consistency, local embeddings, hybrid retrieval evaluation, management UI | Not started |
| M3 / HOTR-19–22 | Auditing, key rotation, Windows lifecycle, controlled daily-use comparison | Not started |
| M4 / HOTR-23–32 | Security, malformed inputs, races, crashes, storage faults, model faults, scale, soak, dependency review, repairs | Not started |
| M5 / HOTR-33–36 | Packaging, clean installation, hosted builds, deployability decision | Not started |

For each run record: prompt/gate ID, UTC timestamp, source SHA and dirty status, executable SHA-256, dependency/model versions and hashes, OS and resource allocation, dataset and seed, command/arguments with secrets excluded, sample counts, percentiles, errors/timeouts, correctness assertions, limits/abort conditions, evidence paths, and pass/fail/blocked result.

Use `PASS`, `FAIL`, `BLOCKED`, or `NOT RUN`. Do not convert skipped checks into passes. Preserve complete failure output locally with restricted access; produce a redacted public-to-the-repository summary. Local/private and synthetic evidence have separate retention rules.

Artifacts are planned under `docs/evidence/` for sanitized summaries and `work/evidence/` for local raw synthetic logs. Runtime private data is excluded from Git regardless of repository visibility.

Historical HOTR-00 checkpoint: verified 36 unique sequential prompts, all required fields, dependency ordering, relative links, approval boundary, staged governance files, credential-pattern scan, whitespace, and repository hooks. At that planning-only checkpoint no implementation gate had run. Current implementation results are listed above.
