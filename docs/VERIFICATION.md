# Verification ledger

Current execution: HOTR-14 passed its full local gate, independent review, main publication and exact hosted Windows run `34048039160` at `d1903b7dbf708d7c9caf919350e0cf2769f1ec8b`. HOTR-15 local semantic indexing passed its local gate and independent review and is published at `e2efb97ab86e17967800991459d904ede79d8ab2`; exact hosted [Windows run `34049617780`](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34049617780) passed. HOTR-16 passed its full local gate; fresh independent review returned ship; publication is pending. Remaining HOTR-12 compatibility and further Lamprey/plugin work stay deferred by the owner.

HOTR-01–12 local gates have passed, including the actual Codex CLI and Claude Code shared-memory workflow. Installed Lamprey acceptance is published with exact Windows CI PASS. HOTR-12A's final Hermes and common gates passed, including the HOTR-12A-R1 monitor repair. These results accept the tested isolated-profile workflows; everyday installation, the remaining compatibility roster, semantic retrieval and later STS gates remain open. Hosted evidence is tracked separately.

| Gate | Required evidence | Status |
|---|---|---|
| HOTR-00 | Plan structure, approval boundary, clean diff, authorized main publication | PASS — `b2c900519569bc2288d4d4f4e18c6cc2a6171f1a` verified on origin/main |
| HOTR-01 | Pinned reuse review, requirement gaps, user-ratified SQLCipher architecture, bounded-write approval, baseline preservation | PASS — `b6a2ae04332d6dac5700aba4659697730e575e37` verified on origin/main |
| HOTR-02 | Native release build, actual cipher/provider versions, FTS/WAL/reopen/integrity, wrong-key/keyless/plain-SQLite/tamper rejection, storage/temp/log scan | PASS — `dcc54f2c0becc8b0542cdec3dd1c95535a5298a7` verified on origin/main; [native evidence](evidence/HOTR-02-native.json) |
| HOTR-03 | Failed assertion/timeout/log-flood refusal, live Windows resource limits, path/PID guards, seed/redaction/required-command tests, exact source/binary hashes, minimal CI | PASS locally and hosted — `c95bffb8e068abd0f345762ca806fce49402a875` verified on origin/main; [harness evidence](evidence/HOTR-03-harness.json); [exact-commit Windows run](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33991495729) |
| HOTR-04 | Actual no-echo ConPTY create/unlock, protected ACLs, bounded wrong/malformed requests, duplicate pipe/port refusal, lock/process exit, real second authenticated Windows principal denial | PASS locally at `1e049d27ec1f915dd54498fc44f6231ec934cee7` — [owner evidence](evidence/HOTR-04-owner.json); [hosted run 33992714760](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33992714760) FAIL at literal ACL comparison |
| HOTR-04-R1 | Structural SID/ACE verification, alias/order and deny cases, repeated real owner/two-account gate, exact-commit Windows CI | PASS locally; hosted ACL and console tests passed at `437f77887b512f3d7f1fcfcd0f922a42fb2d6719`, but [full run 33993684320](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33993684320) FAIL on owner reconnect; see R2 |
| HOTR-04-R2 | Bounded actual pipe-slot retirement, 4,096 reconnects, retained client overlap, silent wrong-key errors, full owner/two-account/encryption gate, hosted run | PASS locally and hosted at `eeddaedbd8d92b8fca1220156179d13f21253245` — [repair evidence](evidence/HOTR-04-R2-pipe.json); [exact hosted run 33995345800](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33995345800) |
| HOTR-05 | Versioned encrypted records, migrations preserving history, Unicode/byte limits, relation constraints, opaque sources, future closed/WAL files untouched | PASS locally — [final combined evidence](evidence/HOTR-05-and-04-R1.json); [earlier schema checkpoint](evidence/HOTR-05-schema.json) retained before the ACL repair |
| HOTR-06 | Atomic revisions/audit/receipts, concurrent winner, principal-scoped replay, bounded queue, cancellation/unknown outcomes, actual crash/ack replay, repeated owner boundary | PASS locally at `edf926ce8b5d29d75dc6e147b42217e4799fd8af` — [transaction evidence](evidence/HOTR-06-transactions.json); [hosted run 33996469717](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33996469717) FAIL creating the fresh fixture root; see R1 |
| HOTR-06-R1 | Create the validated absent synthetic fixture root before unit tests; exact fresh hosted pass | PASS locally and on exact fresh [Windows run 33998187706](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33998187706) |
| HOTR-07/08 | Actual HTTP role/namespace/operation matrix, DPAPI, separate-account denial, revoke on existing connection, typed limits, safe client, overload and deadlines | PASS locally and hosted at `b4fe15bd19d439cb34a5522d99c119d29ab2dc16` — [combined evidence](evidence/HOTR-07-08-capabilities-rest.json); [Windows run](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33998187706); separate-account proof is local |
| HOTR-09 | Scoped encrypted FTS/current/history/count/list, literal queries and budgets, native query deadlines, 10k records/8 clients/20 requests per second for 15 minutes, independent durable reconciliation and canaries | PASS locally and hosted at `006428c3525fdb011e69a7e2bb948afd96451ea4` — [source-bound gate](evidence/HOTR-09-retrieval.json), [measured local load](evidence/HOTR-09-prototype-load.json), [exact hosted run](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34000552038) |
| HOTR-10 | Official SDK, real bridge initialization/discovery/tool calls, independent credentials, role/namespace denial, current sourced reads, replay/revoke/reconnect, cancellation and frame/admission/startup limits | PASS locally and hosted at `51f0effef83cfabd71db29e2bc8f411850407e49` — [source-bound MCP gate](evidence/HOTR-10-mcp.json), [live exported schemas](evidence/HOTR-10-tools.json), [exact Windows run](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34001324657); this is not named-app acceptance |
| HOTR-11 | Encrypted snapshot during writes, different key, snapshot watermark reconciliation, fresh restore with state/grants, old-token invalidation, bad-key/tamper/truncation refusal, ConPTY and separate account, canary scan | PASS locally at `91c12e2333ea4482cd3cd9a5c621b6f03f12464b` — [source-bound gate](evidence/HOTR-11-backup.json), [actual recovery](evidence/HOTR-11-recovery.json); [hosted run 34002264699](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34002264699) FAIL before backup due to absent fresh fixture parent; see R1 |
| HOTR-11-R1 | Validate/create the backup fixture parent, native multi-step copy and full rebuilt native/two-account gate, exact clean hosted run | PASS locally and on exact clean [hosted run 34003325869](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34003325869) at `cf4fa8ed1373431733e01c9f3faa1229f4e5c9fa` — [local repair evidence](evidence/HOTR-11-R1-fixture.json) identifies preserved HOTR-12 helpers; hosted manifest independently confirms the clean commit |
| HOTR-12 | Actual independent Codex/Claude clients, current sourced correction/acceptance, restart, revoke, restore/reenroll, two-account check and final scanner | PASS locally — [source-bound gate](evidence/HOTR-12-clients.json), [actual application tool results](evidence/HOTR-12-applications.json); 8 successful-run prompts, 11 total including failures; publication/hosted closeout tracked below |
| M1 / HOTR-01–12 | Actual encrypted Windows service, scoped access, keyword retrieval, two real clients, restored encrypted backup | Local prototype gates PASS; [demonstration and scope limits](M1-DEMO.md); exact main publication closeout follows |
| HOTR-12-LAMPREY | Actual Lamprey memory flow, model switch, cancellation/recovery, restart and denial | PASS locally and on exact Windows run 34017610460 at `344b7a0a1ae37efe18ca19ca8b768d85e0b2788b` |
| HOTR-12A / HOTR-12A-R1 | Actual Hermes native MCP flow and nonblocking resource-monitor repair | Final local common/app gates PASS; [source-bound evidence](evidence/HOTR-12A-clients.json), [actual tool results](evidence/HOTR-12A-application.json); exact publication/hosted closeout follows |
| HOTR-12B–12K | Remaining individual applications, provider/runtime routes and cross-client consistency | Deferred by the owner; not established by the four completed app proofs |
| M2 / HOTR-13–18 | Selected imports, correction consistency, local embeddings, hybrid retrieval evaluation, management UI | HOTR-13–15 local gates passed; HOTR-16 local gate passed; publication and HOTR-17–18 pending |
| M3 / HOTR-19–22 | Auditing, key rotation, Windows lifecycle, controlled daily-use comparison | Not started |
| M4 / HOTR-23–32 | Security, malformed inputs, races, crashes, storage faults, model faults, scale, soak, dependency review, repairs | Not started |
| M5 / HOTR-33–36 | Packaging, clean installation, hosted builds, deployability decision | Not started |

For each run record: prompt/gate ID, UTC timestamp, source SHA and dirty status, executable SHA-256, dependency/model versions and hashes, OS and resource allocation, dataset and seed, command/arguments with secrets excluded, sample counts, percentiles, errors/timeouts, correctness assertions, limits/abort conditions, evidence paths, and pass/fail/blocked result.

Use `PASS`, `FAIL`, `BLOCKED`, or `NOT RUN`. Do not convert skipped checks into passes. Preserve complete failure output locally with restricted access; produce a redacted public-to-the-repository summary. Local/private and synthetic evidence have separate retention rules.

Artifacts are planned under `docs/evidence/` for sanitized summaries and `work/evidence/` for local raw synthetic logs. Runtime private data is excluded from Git regardless of repository visibility.

Historical HOTR-00 checkpoint: verified 36 unique sequential prompts, all required fields, dependency ordering, relative links, approval boundary, staged governance files, credential-pattern scan, whitespace, and repository hooks. At that planning-only checkpoint no implementation gate had run. Current implementation results are listed above.

2026-09-06 closeout update: HOTR-12 is published at
`ce1f8f7a8a72780aaf69f6bbf7a2d324f563518f`; exact clean hosted Windows run
34004546514 PASS. HOTR-12-LAMPREY is IN PROGRESS: real installed-app connection
and scoped denial proved, write/correction smoke FAILED on provider schema
compatibility. The local inline-schema repair passed focused/native tests and
the final zero-model app preflight, but actual model-driven repair acceptance,
remaining Lamprey gates, formatting, commit and push are still open. Full
evidence and pending input boundaries: [Lamprey checkpoint](LAMPREY-INTEGRATION-PROGRESS.md).

Owner-directed publication update, 2026-09-06 UTC: HOTR-12 is complete and on
public main at implementation commit `ce1f8f7a8a72780aaf69f6bbf7a2d324f563518f`,
with exact hosted Windows CI PASS. HOTR-12-LAMPREY is now **DEFERRED by the
owner**, superseding its earlier in-progress priority. Its unfinished code is
preserved locally and excluded from the HOTR-12 documentation closeout. No
Lamprey completion or full STS acceptance is implied. See the
[authorized deferral](../PLANNING/HOTR-12-CLOSEOUT-AND-LAMPREY-DEFERRAL-2026-09-06.md).

Resumption update, 2026-09-06 UTC: the owner's subsequent full compatibility
authorization supersedes that deferral. Installed Lamprey 0.32.0 passed its
six-prompt actual-application acceptance, including owner acceptance, restart,
model switch, cancellation/recovery, namespace denial and revocation. See
[sanitized app evidence](evidence/HOTR-12-LAMPREY-clients.json) and the
[integration workflow](LAMPREY-INTEGRATION.md). The first common gate failed the
existing idle MCP deadline test; its complete failure is retained locally as
`HOTR-03-40840-1788666049692772100`. The unchanged-test common rerun, final binary
app verification and exact publication remain in progress. Active-profile
enrollment and the remaining application rows are not yet complete.

Final local closeout: common gate `HOTR-03-78452-1788676083262650400` PASS
(24 product tests, five runner tests, both formatting/Clippy gates, negative
controls and 3,415-file canary scan). Final actual Lamprey acceptance
`HOTR-12-LAMPREY-77184-1788676590328991300` PASS, with six successful synthetic
prompts and normal application exit. Both gates used identical source and
native-library hash maps. [Published source evidence](evidence/HOTR-12-LAMPREY-source.json)
records those maps and the distinct executable hashes; current files match.
The original timing failure remains retained and its cause is not established;
the unchanged deadline test passed at approximately 15.04 seconds in the rerun.
Hosted verification and active-profile enrollment remain separate gates.

Hermes final local closeout, 2026-09-06 UTC: common gate
`HOTR-03-88344-1788679798730446900` and actual application gate
`HOTR-12A-79932-1788680065291303200` PASS on identical source/native inputs and
product SHA-256 `7536254f1471e810982da5d8288db4578bdf3213f2b069ed0e4b363144384aea`.
Twenty-four product tests, six runner tests, both format/Clippy gates, negative
controls and a 4,083-file canary scan passed. The final installed-app flow used
three prompts; Hermes consumed seven including earlier attempts and verification
after the monitor repair. The shared compatibility total is 19 of 72.
Actual tool results confirm sourced revisions 1/2/3, restart search/current
recall, 403 outside scope and 401 after revocation; an independent reader still
received revision 3. The earlier owner timeout remains retained with cause
unestablished. The subsequent scanner timeout is addressed by HOTR-12A-R1;
neither failed manifest was relabeled. Publication and exact hosted verification
follow this local acceptance. Everyday-profile enrollment remains pending.


## HOTR-13 — owner-selected imports

Local PASS on `HOTR-13-68176-1788708184429672500`: 27 product tests and six runner tests, both format/strict-Clippy checks, release build and canary scan of 4,347 files. The [source-bound evidence](evidence/HOTR-13.json) includes 62 source hashes, native/binary hashes and three actual import fixture reports.

The actual CLI and owner service proved exact preview/committed-record agreement, proposed state and provenance, no duplicate inserts, owner acceptance preservation, identical restart receipts, stale-file/revision rejection, malformed/hash/collision rejection, cross-vault digest refusal, and atomic revision/audit/receipt/FTS rollback under a forced storage failure. Actual Windows path tests refused a junction escape and a junction root, plus traversal, UNC, device, alternate-stream, format/size/UTF-8 and concurrent-writer cases. Original selected files and the junction target retained their bytes.

The reused owner channel, encryption, schema migration, backup/restore, HTTP/MCP permissions, cancellation and lifecycle regression tests passed. Separate-account and model-driven app workflows retain their prior evidence; they were not rerun or relabeled as fresh HOTR-13 proof. No personal imports or cloud calls were made. Full stress/soak and later roster gates remain outstanding.

Hermes publication `5f6c6481af2892ad7da5ba499e7efc74cf8b4eac` has exact [Windows CI PASS](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34041363996). HOTR-13 publication and exact hosted result follow its passed local gate; do not infer hosted success from this local evidence.


Final combined-source HOTR-13 gate PASS: `HOTR-13-79532-1788708735741015700`, covering implementation `7a7db6070b4acb6f4fa317ac60fc57cfcacf451e` above preserved upstream `bb1977dbde1894180069cdad87b69cf79992a984`. Again, all 27 product tests and six verifier tests, both format/strict-Clippy checks, release build and final canary scan passed; the scan covered 4,574 files. Product SHA-256 `a75c10b89c97a4768702eb542b28727f8dd3088837afc51f7932269be94dd5c5`; runner SHA-256 `fc0dbf0283ec0395b959a93d9abcd6fb8d862625674c9fdfce738d94fc4a1ee1`. The three new import fixtures match this exact product binary. Earlier failures and the prior passing gate remain retained in [the evidence](evidence/HOTR-13.json).


## HOTR-14 — corrections, retention and grants

Local PASS: `HOTR-14-72804-1788710656448614500`; 28 product tests and six runner tests, release build, both format/strict-Clippy gates and canary scan of 5,201 files. [Source-bound evidence](evidence/HOTR-14.json) records all 65 source hashes and the actual two-bridge fixture. All current retrieval paths enforce suppression and current scope; explicit history remains authorized and labeled historical. Actual owner correction, supersession, validity/expiry/tombstones, role downgrade, grant removal and restart receipts passed. Earlier failures remain retained.

The owner requested main publication and immediate standby. Exact hosted CI is separate; named-app compatibility and later stress, semantic and deployment gates remain outstanding. Next prompt after resumption: HOTR-15.


Resumed parent gate `HOTR-14-85332-1788713354061792200` PASS on identical 65-source hash map: 28 product tests, six runner tests and 5,442-file scan. The [evidence](evidence/HOTR-14.json) now binds the final binary `ec4aab64f85c0a9d9f23e54568092b9bace9daf649a8f9dddbf6377302c862df` to the fresh two-bridge fixture and retains the earlier passed binary separately. The owner resumed execution; fresh review precedes HOTR-14 main publication.


Final repaired HOTR-14 local gate `HOTR-14-84644-1788714278331656100` PASS: 29 product tests, six runner tests, both format/strict-Clippy gates and 5,917-file scan. [Evidence](evidence/HOTR-14.json) binds current source and executable to real schema-5/6 restored services and two lifecycle bridges. Old backup bytes and context/receipt/audit state are preserved; old credentials return 401 after migration and fresh credentials recall revision 1. Earlier schema-bound backup failure is repaired; initial synthetic ACL failure is retained. Fresh review and exact main/hosted verification remain separate acceptance steps.


Fresh independent ASTRA REVIEW returned `ship` with no findings after the repaired gate. Parent staged-source, preserved-baseline and document-link checks also PASS. HOTR-14 is locally accepted for main publication; exact hosted CI remains pending until observed.


HOTR-14 publication closeout: implementation commit `5ed982f2ecf35ca8ffac937038d9972375d7406e` passed the final local gate `HOTR-14-84644-1788714278331656100` and fresh independent review (`ship`). All 65 normalized source hashes match the passed gate. This documentation-only closeout records the implementation SHA; it does not change runtime sources. Non-force main publication and the exact hosted result are tracked separately. One canonical checkout is retained, with no linked worktrees; approximately 4.18 GiB of generated cache/evidence is retained. No cleanup or personal import occurred. Continue the approved roster at HOTR-15.


## HOTR-15 — local gate passed

The complete gate `HOTR-15-73280-1788715935249086000` PASS: 41 ordinary product tests, six verifier tests, one actual installed-Ollama test, format/release build/strict Clippy, and the 6,250-file canary scan. Product SHA-256 `d231621e5657b14f3cd2aa207a9bc6286a81db314a293d0e15756eff15c6bcd8`; runner SHA-256 `437916df872c6147182b3da45fcf5a11f91e99f90435a718e6a1c3e4d63f0847`. All 71 current normalized source hashes match. Real Ollama 0.32.6 supplied 768-dimensional vectors from the pinned project-only model; observed TCP peer was numeric loopback. Actual service tests proved outage-safe writes/keyword search, three-attempt exhaustion across restart, owner configuration conflicts, in-flight cancellation, revision replacement, no duplicate reindex on restart, and restored-vault indexing disabled. Native encrypted tests additionally rejected stale revision/generation/visibility completions; actual loopback HTTP fixtures rejected model changes, wrong dimensions, malformed vectors, redirects and oversized/truncated responses.

The two failed Clippy gates and slow pre-download installer attempt remain recorded. [Sanitized source-bound evidence](evidence/HOTR-15.json) separates this local PASS from fresh review, publication, hosted verification, and later search/quality/stress gates. Preservation: eight baseline copies match, one canonical main checkout with no linked worktrees, 5,580,787,832 bytes generated and 297,781,182,464 bytes free at closeout. Cache/model/evidence are retained; no files were deleted or personal context imported.

API-EQUIVALENT COST RECEIPT: unavailable. Parent and native subagent per-call token usage is not exposed, so no cost or savings estimate is supported. Requested Sol/high and Terra/high routes, native task identities/status, and unobservable runtime settings are recorded in the evidence.


Fresh independent HOTR-15 review `/root/hotr15_review` returned `ship`, matching all 71 source hashes, product/runner hashes, raw gate logs and all three fixture reports. The sole P3 documentation finding was repaired: README and model docs now reflect the passed gate, and model task prefixes are distinguished from MCP tool names. No runtime source changed after verification. A separate read-only post-inference check found exactly the five pinned model files with unchanged manifest/blob hashes. Requested reviewer route Sol/high; actual model/effort/token telemetry remains unobservable. The authorized focused main commit and exact publication checks follow.


HOTR-15 implementation commit `3f6b232af870244c27bb798d05852895cbd5d4d7` records the source verified by `HOTR-15-73280-1788715935249086000` and accepted by the fresh `ship` review. This documentation-only closeout records that SHA; runtime sources are unchanged. Authorized non-force publication to main and exact hosted verification are tracked separately. HOTR-16 is the next approved prompt.


HOTR-16 final local gate `HOTR-16-19596-1788718053792654100` PASS: 46 product tests, six verifier tests, two actual installed-Ollama fixtures and a 7,093-file plaintext canary scan. All 76 source hashes match; product SHA-256 `c5de23315bbf1b0afe3091d9c15c2599b52663272bd21d19c24b34745228c8b0`, runner SHA-256 `b2d1bf1a1a51691e7d16d23a70214049ef6c11a917234090750d6b93d5000d44`. [Source-bound hybrid evidence](evidence/HOTR-16.json) separates real HTTP/MCP/model proof from controlled adapter race fixtures. Fresh independent review, main publication, exact hosted verification and HOTR-17 quality remain pending.


HOTR-16 implementation `4640559ef66a00a73d914420b3439b32a6e60077`: full local gate and fresh independent review PASS. Main publication and exact hosted verification remain separately tracked. No runtime source changed for this SHA closeout; HOTR-17 held-out evaluation is next.


HOTR-16 exact published main head cb89ac9a6362c1805b93a813883fbf8662123f8c passed hosted Windows run [34051365651](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34051365651). Installed-model proof remains separately recorded by the local gate. HOTR-17 corpus/metric development continues; no retrieval-quality result has yet been measured.


## HOTR-17 — retrieval quality local PASS

`HOTR-17-86548-1788720435627956200` passed all 50 product tests, six runner tests, three installed-model fixtures and 7,531-file canary scan on 79 matching source/input hashes. [Results](RETRIEVAL-RESULTS.md) and [sanitized source-bound evidence](evidence/HOTR-17.json) record the independently frozen 144-query corpus. Held-out paraphrase Recall@5 is 24/24 for hybrid versus 0/24 for literal keyword retrieval; exact/current/conflict cases pass with zero prohibited results or wrong revisions. Held-out hybrid p95 is 150.649 ms. No-answer inputs still return candidates without an answer/abstention claim. Fresh implementation review and main publication remain pending.


Fresh independent ASTRA REVIEW /root/hotr17_implementation_review returned ship with no actionable findings. It matched all79 current/staged source hashes, both binaries, the model manifest/four blobs, five raw proof reports and all288 search plus24 direct-get measurements. Gate logs confirm all11 commands passed. Requested reviewer Sol/high; actual settings and token telemetry unobservable. Parent accepts HOTR-17 for authorized main publication. Exact hosted verification remains separately tracked; HOTR-18 owner viewer follows publication.

API-EQUIVALENT COST RECEIPT: unavailable. No observed parent/delegate token telemetry is exposed; no cost or savings claim is supported.


HOTR-17 implementation commit `5524505c9128b4d794b151de764929e3084d7fa5` contains the source accepted by full gate `HOTR-17-86548-1788720435627956200` and fresh independent `ship` review. This documentation-only closeout records its SHA; all 79 runtime/source/input hashes remain unchanged. Non-force main publication and exact hosted verification follow. The single canonical checkout retains approximately 5.49 GiB of shared cache/model/synthetic evidence and 298,479 bytes of rejected corpus drafts; no cleanup or new worktree. HOTR-18 is next.
