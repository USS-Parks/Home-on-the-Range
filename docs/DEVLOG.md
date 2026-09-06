# Development log

## 2026-09-05 — HOTR-00 — Plan publication

Status: drafting/publishing governance; implementation awaiting STS approval.

User request: produce a granular PSPR, first build a small local prototype, then harden and stress test it, then evaluate deployment. Project codename and authorized repository are Home on the Range / `USS-Parks/Home-on-the-Range`. Standing publication authorization targets `main`.

Observed preparation:

- Authenticated `gh repo view` confirmed the repository is private and initially empty.
- One canonical checkout was cloned to `C:\Users\17076\Documents\Codex\Home-on-the-Range`; its unborn branch is `main`. No temporary worktrees were created.
- Windows shell startup failed in the restricted runner with Windows error 5. Approved execution outside that failing runner succeeded. This was an execution-environment problem, not a product test failure.
- Rust/Cargo, Node, Git/GitHub CLI, Ollama, and Codex/Claude/Cursor launchers were found. Finding a launcher does not prove a working application integration.
- Local listeners matched agentmemory's documented ports. Its configuration guidance describes optional bearer authentication and local hybrid retrieval; encryption and per-client authorization have not been verified. No existing memory content or secret configuration was read or changed.
- The local machine has capacity for the bounded test campaign in the plan. No build, model download, stress run, service installation, or client configuration change has occurred.

Files: README, project AGENTS, ignore/line-ending rules, canonical PSPR, this log, and verification ledger.

Local document check passed: 36 unique sequential prompt IDs, all five required fields on every prompt, and the explicit pending-approval marker. Repository publication checks also validate dependency order, relative documentation links, staged-file hygiene, and whitespace. Publication SHA is recorded in the closeout entry after the push is verified.

Next: obtain STS approval, then start HOTR-01. Recommended first authorization is through HOTR-12 (the usable local prototype).

### HOTR-00 publication closeout

- Plan/governance commit: `b2c900519569bc2288d4d4f4e18c6cc2a6171f1a`.
- Published to `origin/main`; `git ls-remote origin refs/heads/main` returned the identical SHA. GitHub confirmed private visibility and default branch `main`.
- Document validation: PASS for 36 unique sequential IDs, five required fields per prompt, dependency order, relative links, pending-approval marker, staged-file allowlist, credential-pattern scan, and staged whitespace. The repository's no-slop hooks passed for both commit and push.
- Preparation corrections: the dependency checker was updated to trim Markdown hard-break spaces; staged whitespace checking then rejected those spaces, which were replaced with ordinary paragraph separation. Complete whitespace failure output is retained at ignored `work/evidence/HOTR-00/whitespace-before-fix.txt`. The final rerun passed.
- Checkout inventory: one canonical checkout on `main`, clean after initial publication, zero ahead/behind commits, no temporary worktrees. Approximately 26 KiB of generated diagnostic evidence is retained for traceability. No `target` or `node_modules` tree exists; nothing needs worktree retirement.
- This following documentation-only closeout records the already-verified planning commit. Its own final remote SHA is verified at publication and reported in the task response.
- No application runtime, database, model download, client integration, or product test has been installed/executed. HOTR-01–36 remain NOT STARTED pending explicit STS approval.

## 2026-09-05 — HOTR-01 — Foundation and approved execution boundary

Full STS was approved by the user, with SQLCipher through rusqlite and an expanded application/provider/OS roster including Lamprey Harness. A later explicit "Yes, I approve and authorize now" approved the exact bounded-write proposal. See the dated approval/addenda in PLANNING; no further confirmation is needed for the operations they authorize.

Eight original project files were preserved with matching SHA-256 hashes under `work/hotr-baselines/2026-09-05-before-STS/`. No files were deleted and no outside application profiles or data were changed.

HOTR-01 gate: PASS for the read-only foundation decision. Two reuse candidates were inspected at pinned revisions with MIT license metadata; every must-have has a route and a stated evidence gap in `docs/adr/0001-foundation.md`. The user-selected Rust/SQLCipher architecture is ratified. Installed client metadata and live Lamprey source interfaces informed the dedicated compatibility prompts. None of those clients is yet integrated.

Native finding for HOTR-02: the existing libsqlite3-sys 0.38.1 cache bundles SQLCipher 4.14.0; a later current SQLCipher source is required for the native gate. Public metadata reports libsqlite3-sys 0.38.2 and SQLCipher v4.18.0. Versions are evidence, not permission to reuse unverified binaries. Native build/tests remain next.

Files: recorded approval and compatibility/preservation/Lamprey addenda, foundation ADR, current-authority pointers, and ledgers. Before publication, verify Markdown links, diff whitespace, staged-file hygiene, and repository hooks. The prompt's commit/remote SHA is recorded in the following closeout entry without circular SHA rewriting.

### HOTR-01 publication closeout

Commit `b6a2ae04332d6dac5700aba4659697730e575e37` was pushed to private origin/main; remote SHA matched exactly. Repository commit/push hooks passed. No temporary worktree was created.

## 2026-09-05 — HOTR-02 — Native Windows encrypted storage

Gate: PASS locally. The minimal release executable uses rusqlite 0.40.2 with externally linked SQLCipher 4.18.0 (SQLite 3.53.4) and statically linked OpenSSL 4.0.2. Native dependencies are pinned and described in NATIVE-BUILD.md; locked native compilation completed in 10m46s on the reference host. Build, dependency, and temporary outputs remain project-confined.

Final evidence directory: `work/hotr-evidence/HOTR-02-gate-20260905T203216769Z/`. Release build, warnings-denied release Clippy across product targets, locked release tests, metadata-only CLI, and independent canary scan passed. The real SQLCipher test verifies FTS and WAL, correct-key reopening, database/cipher integrity, wrong-key/keyless/ordinary Python SQLite rejection, tampered-file rejection, non-file/memory-alias rejection, synchronous FULL, and compile-time memory-only temporary tables. Live DB/WAL/SHM and retained closed files were scanned for generated UTF-8/UTF-16 canaries and synthetic key bytes. The post-run scan checked 20 stored/temp/log files across two retained passing synthetic runs.

Failures retained and repaired: the first root compilation lacked a native library before rusqlite's build, fixed with an explicit native prerequisite; the first metadata-only command queried a provider without a keyed database, fixed with actual linked-library version APIs and a process-level regression check. The first link also reported missing intermediate OpenSSL PDBs; release builds now explicitly use `/DEBUG:NONE`, consistent with their no-debug-information profile. The final release linker log contains no warning. Complete gate logs are retained; the early build-order diagnostic is recorded separately.

Executable SHA-256: `8a20c1d29daed3441d5b25c2044ac4bbb0c59a93e5a742f440cc816a6ba94e93`, size 4,247,040 bytes. Native/source hashes and commands are in `docs/evidence/HOTR-02-native.json`. Native helper formatting changes did not change the pinned C input or compiler options; product sources were rebuilt and tested after the final code change.

Files: Rust package/lockfiles, native build/link configuration, storage boundary, version CLI, encryption and independent canary tests, build runbook, and evidence/ledgers. No real vault, listener, startup entry, external app configuration, or provider connection was created. Retained generated state is approximately 1,175 MiB in one canonical checkout; no duplicate worktree or dependency tree was created. It contains build prerequisites, compiler outputs, preserved baselines, and synthetic evidence and has not been cleaned up.

Next: HOTR-03, the bounded reproducible gate harness. Full STS remains active. This prompt's exact commit/remote SHA is recorded in the next closeout entry.

### HOTR-02 publication closeout

Commit `dcc54f2c0becc8b0542cdec3dd1c95535a5298a7` was pushed to private origin/main; the remote SHA matched. Commit/push hooks passed. Staged whitespace checking first found three extra trailing blank lines; they were corrected and source hashes refreshed before publication. No behavioral gate was waived.

## 2026-09-05 — HOTR-03 — Bounded verification harness

Gate: PASS locally. Final run: `work/hotr-evidence/HOTR-03-76420-1788641524308666500/`; published sanitized manifest: `docs/evidence/HOTR-03-harness.json`. JSON Schema validation and all 20 staged source hashes matched the manifest using the existing SDK Python environment read-only. Native libraries are hashed independently. The runner requires unchanged source through the gate and records failure outcomes alongside passing required commands.

Live evidence: an intentional assertion returned exit 17; an owned sleeping process was terminated at its deadline; an output flood was stopped at the log cap; explicit mismatched-PID termination was refused. Windows accepted and reported the runner's own Job Object memory/process limits; the applied CPU rate was 2500/10000 on this 16-thread host. The job encloses the runner and its ordinary descendants from process creation. Per-command and final source checks passed. Five harness contracts passed, including independently anchored deterministic generation, path rejection, credential redaction, disk thresholds, and omitted-command rejection. Product encryption tests and the post-run native canary scan also passed.

Retained repairs: initial Windows binding namespace correction; a failed full run that attempted to replace its running harness executable, fixed by running the same contract suite as library tests; and a manual review correction reserving the saved log framing within the exact 8 MiB cap. An intermediate run passed its earlier checks before that final log-boundary strengthening. Final largest stored log: 8,388,607 bytes. Earlier manifests/logs remain retained and are superseded by this final evidence.

The native canary protocol now derives new synthetic values from the run basename so Windows canonical-path spelling cannot change the independent scan. The scanner also checks retained older derivations. No real credentials or user memories are involved.

Runner SHA-256: `c50362d9ef9c4e98e2993c4c801094c60eb83101d1e3b9340a965e5f83f2e30e`. Product SHA-256 for this run: `f8eefb44b568b6303a107baf62d132bfcaf0cf3a83cb22d25ba4382e0ad7ede2`. Source identity is based on normalized LF source hashes plus exact native/binary bytes; recompilation can change binary hashes without changing the native dependency versions.

The frozen PSPR workload targets, monitored disk limits, kernel resource limits, and evidence schema are documented. A minimal Windows workflow uses pinned action commits, locked native inputs, and retained synthetic evidence. Hosted status is NOT RUN locally and must be checked for the exact published commit. This gate does not prove client integration, a second-user owner boundary, recovery campaigns, or deployment.

One canonical checkout remains; no temporary worktrees, source clones, or extra dependency trees were created. Project-owned prerequisites, build artifacts, baseline copies, failed diagnostics, and synthetic evidence are retained. No real vault, external application profile, OS account, startup entry, or provider connection was changed.

Next: HOTR-04 owner lifecycle and local administration. Full STS continues. Commit/remote SHA and the exact hosted run are recorded after publication.

### HOTR-03 publication and hosted closeout

Commit `c95bffb8e068abd0f345762ca806fce49402a875` was pushed to private origin/main and the remote SHA matched. Commit/push hooks passed. The exact commit's Windows workflow [33991495729](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33991495729) passed: job `101374424104`, completed 2026-09-05 21:05:11 UTC, including native preparation, the HOTR-03 gate, and evidence upload. This supersedes the earlier pending hosted status without changing its historical local result.

## 2026-09-05 — HOTR-04 — Owner lifecycle and Windows administration

Gate: PASS locally, including the required genuine second-principal proof. Final bounded run: `work/hotr-evidence/HOTR-04-80084-1788642932680279900/`. The source-bound sanitized manifest is `docs/evidence/HOTR-04-owner.json`; the separate-account receipt and preserved synthetic fixture are in `work/hotr-tests/HOTR-04-41464-1788643027165455900/`.

Implemented create/serve/status/unlock/lock, real no-echo Windows console prompts, zeroizing passphrase buffers, explicit protected owner/SYSTEM file and pipe ACLs, remote-pipe rejection, first-instance reservation, peer SID validation, bounded framing, deterministic port collision refusal, and lock through process exit. Existing destinations are refused without replacement. A successor pipe instance prevents a reconnect gap. The loopback port is reserved only; context REST operations remain HOTR-08.

Live gate evidence: ConPTY create and unlock did not echo synthetic passphrases; wrong-key responses were bounded and generic; malformed/oversized frames were rejected; 32 successive reconnects succeeded; duplicate pipe and occupied port failed; locking ended the key holder and its connections; restart was locked. Existing-vault creation refusal preserved the database hash. The actual Codex sandbox account had a distinct authenticated Windows SID and received error 5 on direct directory, database, marker, and administration-pipe access while the owner stayed unlocked. The owner then locked and exited. This was neither a mock nor a restricted same-user token. Selecting system cmd.exe made the existing sandbox account usable after the default user-installed PowerShell launcher had failed. No OS account/security change was needed.

Release build, warnings-denied all-target Clippy, encryption tests, owner tests, harness contracts, and canary scan passed. The normal test suite explicitly ignores the interactive two-account test, and the HOTR-04 runner separately requires and executes it; missing evidence fails after 180 seconds. The independent scan checked 175 storage/temp/log files across retained native and owner runs, with no plaintext native canary or synthetic owner key. Windows job limits enclose the gate and its descendants.

Retained repairs: initial Windows API import correction; first owner run exposed a pipe reconnect race and an unhandled ConPTY cursor-position handshake in the test driver, both repaired and retested; initial gate registration missed the second allowlist; warnings-denied Clippy rejected a redundant unit expression. Failures remain visible in local logs. No required check was waived.

Final product SHA-256: `aa1296d320e0e6891ab3b93eed57b66e0fcb1ddd66b4dab212dca8ef0d548fd4`. Runner SHA-256: `d0e5b9274e2b1566930cbceed186d17099f234afd985e48c2331cf43dce1c0b4`. The separate-account fixture records the identical product hash. Source and native hashes are in the manifest and are checked against the staged source before publication.

Files: owner and Windows security modules, CLI/dependencies, actual-process/ConPTY tests, scoped second-account probe, harness registration and canary extension, owner runbook, and evidence/ledgers. One canonical checkout remains, no worktrees, approximately 1,809.5 MiB of retained build prerequisites/artifacts, synthetic fixtures, baselines and evidence. No real vault, other application's profile, startup entry, or existing user file was deleted. Hosted normal owner tests are checked after publication; the two-account proof is local evidence.

Next: HOTR-05 versioned records and namespaces. Full STS continues. This prompt's commit and exact remote SHA are recorded after publication.

### HOTR-04 publication closeout

Commit `1e049d27ec1f915dd54498fc44f6231ec934cee7` was pushed to private origin/main and the exact remote SHA matched. Twenty-four staged source hashes matched the gate manifest, JSON Schema validation passed, and commit/push hooks passed. The separate hosted Windows run is [33992714760](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33992714760); its result is recorded when complete. The local two-account proof remains independently identified.

## 2026-09-05 — HOTR-05 with HOTR-04-R1 — Records and owner ACL repair

Local gates: PASS. Final combined run: `work/hotr-evidence/HOTR-05-76320-1788644198965839500/`; sanitized manifest: `docs/evidence/HOTR-05-and-04-R1.json`. Earlier schema-only pass `HOTR-05-55244-1788643797194364000` and `docs/evidence/HOTR-05-schema.json` remain retained and are superseded for the final source by the combined result.

HOTR-05 adds typed records, namespaces, six kinds including roadmaps, proposed/accepted states, immutable revisions, current-revision pointers, migration 0-to-1-to-2, source references, tags, and same-namespace relations. SQL STRICT/CHECK/foreign-key/trigger rules enforce storage constraints alongside typed byte limits and unknown-field rejection. Exact current/history lookups return opaque sources without fetching them. Actual encrypted tests preserve version-1 IDs/history through migration and reopen, retain combining Unicode/emoji, reject oversized/NUL data and invalid relations, and observe no connection to a supplied local source URL.

The version probe uses write-denying Windows handles and a read-only native connection. Closed files without journals use an encoded immutable URI; WAL files use the pinned native read-only shared-memory path with both sidecars held against writes. A committed future version left by an owned writer crash is read correctly. Newer closed and crash-WAL fixtures remain byte-for-byte identical, including their sidecars. A hot rollback journal or missing WAL index is refused for explicit later recovery instead of being reconstructed during the version probe. Initial compiler integer-type errors and the Windows read-only exclusive-lock rejection were repaired; the numeric SQLite diagnostic and complete test failures are retained. No wrong-version write occurred in the accepted implementation.

While HOTR-05 was ready for publication, exact HOTR-04 hosted run 33992714760 completed with failure at ACL checking in both owner creation tests; native encryption had passed. Its job was `101377672638`, completed 21:30:58 UTC. Full failed workflow output and downloaded artifact are retained at `work/hotr-evidence/HOTR-04-hosted-33992714760/` and the timestamped hosted-failure log. The original literal SDDL comparison was replaced by owner SID, protected/non-null DACL, exact two-ACE trustee/mask/inheritance checks. Native descriptor tests accept equivalent aliases/order while rejecting extra grants, duplicates, wrong owners, partial rights, null DACLs, and unprotected/inherited entries. This is the bounded HOTR-04-R1 repair documented in PLANNING; unchanged requested permissions are verified structurally.

Final validation: native release build, warnings-denied all-target Clippy, native encryption, schema/migration/future-preservation tests, actual ConPTY and owner lifecycle, five harness contracts, and a repeated genuine second-account error-5 denial all passed. The probe at 21:37:32 UTC ran against the same final executable and left the unlocked owner unchanged before lock/process exit. The independent scan checked 339 retained storage/temp/log files with no native, owner, or schema canary plaintext. SQL files now participate in source hashing.

Final product SHA-256: `013fb00bb49888266108b79f0e3886f9c9cd28847ce1855a94a9dffc80e56a83`. Runner SHA-256: `33a96817b63b2e89e3ce2d1fbc1233d18996aed9b55164ef9752bc5b6e960562`. The original pre-repair schema binary/hash remains in its earlier evidence. Source hashes are verified against the final staged tree before publication.

The paired commit is a justified bundle: the hosted prerequisite defect arrived before schema publication and both changes affect the same vault-opening boundary. One final source and executable are tested together; no pending work is discarded and no extra checkout is created. One canonical checkout remains, with roughly 1.83 GiB of retained project build/cache, synthetic fixtures, baselines, and evidence. No real vault, application profile, OS account, or startup configuration was changed. No existing user file was deleted.

Next: publish this candidate and require its exact hosted Windows run to pass HOTR-04-R1 before starting HOTR-06. Full STS remains active; publication/hosted closeout records the actual outcome.

## 2026-09-05 — HOTR-04-R2 — Native pipe reconnect repair

The HOTR-05/R1 bundle was published as `437f77887b512f3d7f1fcfcd0f922a42fb2d6719`; origin/main matched and hooks passed. Hosted run 33993684320 passed the structural ACL and real console tests, but failed rapid owner reconnect with UnexpectedEof. Its original logs/artifact remain retained under work/hotr-evidence/HOTR-04-R1-hosted-33993684320. This does not count as full hosted acceptance.

R2's local gate PASS: `work/hotr-evidence/HOTR-04-R2-57956-1788646338714247800/`; sanitized source/binary manifest `docs/evidence/HOTR-04-R2-pipe.json`. Native slot exhaustion times out after one second; releasing a held slot permits successor creation. The repair retries only successor-creation error 231 while yielding for native I/O retirement, keeps the two-instance cap and first-instance protection, and never replays a transmitted operation. The earlier hosted server's exact native error was not captured, so the retirement cause remains a tested hypothesis until exact hosted acceptance.

The actual owner processed 4,096 rapid reconnects and an acknowledged client retained during the next request. Wrong-key native logging is suppressed before schema-dependent queries; retained owner stderr is empty. Release format/build/all-target warnings-denied Clippy, native encryption, schema/migration, ConPTY, runner contracts and actual second-account denial all passed. Probe UTC 22:13:11.3390491Z; authenticated peer received error 5 on directory/database/marker/pipe, owner remained unlocked, and lock ended the key holder. Canary scan checked 429 files across ten passing native fixtures without a plaintext match.

Files changed: owner transport and actual-process tests, key initialization, focused R2 plan, runner allowlists, evidence schema, workflow diagnostic artifact inclusion, and ledgers. Hosted artifacts now retain only synthetic stderr and redacted console transcripts in addition to gate evidence; no database files are uploaded.

The user renewed stem-to-stern execution in direct response to the project-edit permission block; formatting then succeeded. Existing user data, other application profiles, original baseline copies, and historical evidence remain preserved. No extra checkout, OS change, or cleanup occurred. Publish the tested candidate, require its exact hosted pass, then continue HOTR-06 under full STS.
### HOTR-04-R2 publication and hosted closeout

Focused repair commit `eeddaedbd8d92b8fca1220156179d13f21253245` was pushed to private main; the exact remote SHA matched. All 28 staged source hashes matched the local gate and hooks passed. Hosted Windows run [33995345800](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33995345800), job 101384814792, completed successfully at 2026-09-05 22:26:29 UTC. Its downloaded manifest confirms PASS on that exact commit, including actual owner reconnect/ConPTY, encryption, schema, and runner negative controls. Evidence is retained under `work/hotr-evidence/HOTR-04-R2-hosted-33995345800/`. The separate second-account proof remains local. The R2 prerequisite passed before HOTR-06 implementation began. GitHub reported a non-failing Node 20 action-runtime deprecation annotation; actions executed under its forced Node 24 runtime. Review pinned action updates at HOTR-31/35.

## 2026-09-05 — HOTR-06 — Atomic writes and retry outcomes

Gate: PASS locally. Final source-bound run `work/hotr-evidence/HOTR-06-51268-1788647693459640700/`; sanitized manifest `docs/evidence/HOTR-06-transactions.json`. Product SHA-256 `644f1e3ab63c16aadb98aebb942de8eb41aecf79bdf1166da916097a8c1d13e8`; runner SHA-256 `de7b8e905dda782070e40aeaf5909d93a1a0cfb3e7bf19d008843eed229d5773`.

The unlocked connection now belongs to one database worker with 256 waiting slots, nonblocking admission, ten-second request deadlines, stop/cancel handling, and owner lock waiting for the worker to close before process exit. Schema 3 adds immutable mutation audit and principal-scoped retry receipts. An immediate transaction stores the record revision, provenance/tags, pointer, audit and receipt together. Expected revisions prevent silent conflicts; canonical typed request hashes reject incompatible key reuse. Identical retries return the original receipt even after later revisions. Cancellation is arbitrated against COMMIT; after that boundary, missing or failed delivery is explicitly unknown-to-client and reconciled by retry.

Actual encrypted validation: four native submitter threads produced one revision-2 winner and three conflicts; stale requests and changed-body retries were rejected; different principals had independent idempotency key spaces. The 256-slot queue refused overflow. Expired/dropped/canceled attempts left no new committed record. An audit-insert failure rolled back all mutation tables. Receipt/audit fields agreed. Owned subprocess terminations before commit, after commit before reply, and after a separate client durably journaled its received acknowledgment reconciled to three unique revisions with no lost acknowledgment. These are three focused cycles, not the later HOTR-26 100-cycle acceptance. Crash hooks exist only in the test binary.

Retained failures: initial usize-to-SQL ordinal compiler rejection; full gate `HOTR-06-70932-1788647589606934000` then caught a fixture stdout framing mismatch because single-threaded libtest prefixes nocapture output with the test name. The client parser now accepts that exact known prefix and retains bounded child stdout. The original timeout remains recorded; the fixed full gate passed without waiving any assertion.

Release format/build/all-target warnings-denied Clippy, all encrypted/native/schema/owner/ConPTY tests, harness contracts, and the actual second authenticated Windows account denial passed. The final probe at 22:36:13.8506888Z used the same binary, received error 5 on all four boundaries, left the owner unlocked, and then proved lock/process exit. The independent canary scan checked 584 storage/temp/log files across eleven passing native fixtures without plaintext matches.

Files: transaction worker/tests, schema-3 migration and registration, owner worker lifecycle, Tokio sync feature, runner/scanner registration, transaction/schema documentation, source-bound evidence and ledgers. Existing user data, other applications, and retained baselines/evidence remain preserved. Next: publish HOTR-06 to main, record its exact hosted run separately, then HOTR-07 application capabilities under full STS.

### HOTR-06 publication and fresh-checkout failure

Commit `edf926ce8b5d29d75dc6e147b42217e4799fd8af` was pushed to private main;
the exact remote SHA matched, 31 source hashes matched the staged candidate,
evidence schema validation and hooks passed. Hosted run
[33996469717](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33996469717),
job 101387776367, failed at 22:47:13 UTC. All five transaction unit tests tried to
canonicalize an absent `work/hotr-tests` root before any fixture. The prior local
tests had already created it. Complete output is retained under
`work/hotr-evidence/HOTR-06-hosted-33996469717/`.

HOTR-06-R1 validates the path and creates the missing approved synthetic root,
retaining existing directories. It is verified with the already-active
HOTR-07/08 source in a single focused boundary bundle. This follows the PSPR's
explicit bundle allowance: capabilities need actual process calls, and REST is
their planned transport. Neither gate is waived. HOTR-09 waits for the exact
fresh hosted pass. The failed earlier hosted run stays failed.

## 2026-09-05 — HOTR-07/08 with HOTR-06-R1 — Capabilities and bounded REST

Local full gate: PASS. Final run
`work/hotr-evidence/HOTR-08-78444-1788649852989493200/`; sanitized manifest
`docs/evidence/HOTR-07-08-capabilities-rest.json`. Product SHA-256
`dbc6e0343288c45009c8d170556a948a94ed5cff1d3ee406edfb51b34fa62e22`;
runner SHA-256 `17ba7fbcfa13b47b58fba68e9c83c6b1d5da4dcd8f567968f65d97726414c8d2`.

Schema 4 adds immutable client identities, hashed 256-bit BCrypt tokens, exact
namespace grants, and permanent revocation. Owner-only enrollment returns a
user-scoped DPAPI profile to a new owner/SYSTEM-protected file. Reader/contributor
policy, credential-derived identity, accepted-record protection, historical
lookup, original receipt replay and permanent revocation execute on the same
bounded worker queue as writes. The application API has no owner routes.

The actual Hyper/Axum loopback server exposes typed status, get/history, and
create/revise endpoints, with fixed Host/Origin rules, no-store responses,
256 KiB request/1 MiB response/32-level JSON bounds, five-second header and
ten-second handler limits, 128 connections and 64 active requests. Overload
returns controlled 429/503. The provided scoped Rust/CLI client checks the
server-side identity of its established TCP connection before decrypting a token;
it does not follow redirects, consult proxies, or automatically replay writes.
Generic reqwest use is confined to test dependencies.

Real process evidence: separate contributor/reader/other-namespace credentials,
spoofed principal and accepted-state rejection, source-bearing current/history
reads, owner acceptance, durable receipt replay, revocation over the same TCP
connection, and restart-persistent revocation passed. The safe CLI and Rust client
completed actual requests. Invalid Hosts/Origins, malformed/oversized/deep JSON,
record limits, slow headers/bodies, handler saturation, and connection saturation
passed. Original paths were not replaced, including refused credential writes.

The actual authenticated second Windows account probe ran at
2026-09-05T23:13:11.0856166Z. Directory/database/marker/pipe access returned error
5. Copied DPAPI ciphertext could not be decrypted. A listener owned by that
account accepted a connection from the production client and received zero
application bytes before client identity rejection. The owner stayed unlocked,
the vault hash was unchanged, and lock exited the key holder. Probe and product
hashes match. No new OS account or security configuration was needed.

Full release build, warnings-denied all-target Clippy, native encryption,
transaction/crash/replay tests, schema/future-file preservation, ConPTY/owner
lifecycle, runner contracts and the mandatory distinct-account probe passed.
The independent storage/temp/log scan checked 808 files across 12 passing native
runs; the app matrix additionally scanned generated raw tokens in memory against
every managed fixture file. No plaintext canary/token appeared in those files.

Retained local failures: the initial full gate
`HOTR-08-64780-1788649017112763500` observed an HTTP uploader reset after early
oversize rejection. The exact 413 assertion now sends an oversized declared
length and reads the refusal before uploading the rejected body. The next gate
`HOTR-08-77512-1788649752724521100` failed Clippy's collapsible-if requirement in
the TCP identity lookup; its control flow was corrected. Complete failed logs
remain retained. The passing final run supersedes neither failure historically.

Publication preflight unexpectedly observed GitHub visibility PUBLIC, conflicting
with AGENTS.md's private-repository requirement. The cause/time of that change is
unknown. Before publishing this candidate, visibility was restored and verified
PRIVATE using the existing repository administration authorization. This does
not retract any prior public copies; no repository history was rewritten.

One canonical checkout remains, no linked worktrees. All eight preserved original
baseline hashes match. Retained project build/cache, synthetic fixtures, baseline
copies and evidence total approximately 2.893 GiB, with 331.77 GiB free. They are
retained for reproducibility; no cleanup or pre-existing-file deletion occurred.
Named application acceptance, search, MCP, backups, stress/soak and deployment
remain later gates. Publish this locally passed bundle and require its exact
fresh hosted pass before HOTR-09. Full STS remains active.

### HOTR-07/08 publication and user visibility correction

Commit `b4fe15bd19d439cb34a5522d99c119d29ab2dc16` is verified on origin/main.
All 36 staged source hashes and the product hash matched the passing manifest;
JSON Schema validation and repository hooks passed. Its fresh Windows run is
[33998187706](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33998187706).
Hosted status is recorded separately when the run completes.

The user then clarified: "No, I changed it to public, it's fine." The earlier
public setting was intentional. Repository visibility was immediately restored
and verified PUBLIC. Active AGENTS.md/PSPR instructions now preserve PUBLIC;
historical private observations remain unchanged. Source and sanitized evidence
continue to main; no private vault/credential content is included. This explicit
user choice supersedes the prior private default. The visibility correction is
documentation only; the exact code verification run above remains required.

### HOTR-07/08 and HOTR-06-R1 hosted acceptance

The exact code commit `b4fe15bd19d439cb34a5522d99c119d29ab2dc16` passed Windows
run 33998187706, job 101392266139, completed 2026-09-05T23:35:13Z. Downloaded
artifact manifest confirms PASS and the same source commit. Full hosted output
and artifact are retained under `work/hotr-evidence/HOTR-07-08-hosted-33998187706/`
and its adjacent watch/log files. Fresh-root transaction tests, actual HTTP
roles/limits, native encryption, owner/ConPTY, schema and harness controls passed.
The real separate-account DPAPI/pipe/TCP proof remains explicitly local. The
user's public-visibility documentation commit is
`876521aff04fd353350dc85bf58939cc30982320`, verified on origin/main; only Markdown
changed from the tested code. No duplicate native workflow was required for that
documentation-only correction. HOTR-09 may now proceed under full STS.

## 2026-09-05 local / 2026-09-06 UTC — HOTR-09 — Encrypted retrieval and prototype load

Full local gate PASS: `work/hotr-evidence/HOTR-09-80236-1788651975924143000/`.
Sanitized source manifest `docs/evidence/HOTR-09-retrieval.json` and complete
numerical result `docs/evidence/HOTR-09-prototype-load.json`. Product SHA-256
`55cf95cbf94b7679b8dcb1cced4aa64ca4895d8a0b981d7551cc75af129c722f`;
runner SHA-256 `14c828f9a97efb6e592e01d24a3a6dd3a948a62bdbf98f84660c447481d992e1`.

Schema 5 adds encrypted FTS5 and visibility metadata. Each revision updates its
derived index atomically with provenance, audit and retry receipt; injected audit
failure rolls back the index too. Exact get/search/list/count use a common current
visibility view; explicit authorized history preserves older/retired revisions.
Literal bounded queries, fixed exact-ID/source boosts, stable ordering, pagination,
whole sourced records and complete-response byte/token budgets are implemented.
No global BM25 statistics influence one namespace's ordering. Retirement metadata
is seeded only in stopped synthetic fixtures; owner lifecycle operations remain
HOTR-14 and no contributor route exposes those changes.

Database command progress handlers enforce the queued deadline and owner stop
flag. An actual long SQLite query is interrupted and the same worker subsequently
handles its next command. Diagnostic SQL exists only in the test binary. Real
HTTP tests cover current versus old terms, exact path/ID boosts, expired/tombstoned/
superseded filtering, pagination, history, hidden-namespace denials, quotes,
operators, malformed limits, revocation and budget omissions. Initial focused
compilation corrected SQL COUNT extraction to SQLite's signed integer type before
the final gate. No gate assertion was waived.

The mandatory prototype campaign seeded 10,000 1–4 KiB records through four
bounded real API clients across ten namespaces, then used eight independently
credentialed clients at 20 requests/second for 900.017 seconds. All 18,000 requests
completed: 3,600 writes, 7,200 keyword queries, 3,600 gets and 3,600 counts. Keyword
p50/p95/p99 were 22.172/32.957/257.133 ms; write 20.993/38.458/326.709 ms.
Maximum keyword/write latencies were 1,080.844/1,194.625 ms and remain in the report.
Latency includes scheduled-arrival delay; no samples or failures were dropped.
Seeding took 158.809 seconds; first post-seed query was 26.047 ms, not cold-cache.

After lock/process exit, the independent encrypted read reconciled all 10,000
current revisions and 13,600 durable receipts with zero acknowledgment mismatches,
duplicates, unexpected errors or correctness violations. SQLite and FTS integrity
passed. The timed hot write set is 2,000 identities; this result is not a 100k load
or four-hour soak claim. Raw fixture and minute progress remain under
`work/hotr-tests/HOTR-07-67272-1788652127852627800/`; the shared helper retains its
earlier prefix, while its load reports explicitly identify HOTR-09.

Release format/build, warnings-denied all-target Clippy, native encryption,
transactions/crash/replay, migrations, HTTP/owner/ConPTY and harness contracts all
passed. The actual separate authenticated Windows principal at
2026-09-05T23:48:43.9844121Z received error 5 for directory/database/marker/pipe;
copied DPAPI ciphertext was rejected and its fake TCP endpoint received zero
application bytes. Owner state/vault hashes stayed unchanged and lock exited the
key holder. The final storage/temp/log scan checked 997 files across 13 passing
native runs, with no canary plaintext. Generated client tokens were separately
scanned against all managed files in each actual API fixture.

All 38 normalized staged source hashes must match the passing manifest before
commit. All eight original baseline copies match. One canonical checkout, no
linked worktrees; approximately 3.125 GiB of retained build/cache, synthetic data,
evidence and baselines, with 323.22 GiB free at closeout. They remain for ongoing
approved verification; no existing user file, application profile or OS setting
was changed or deleted. Public visibility was reverified per the user's choice.
Next: publish this focused checkpoint, track its exact hosted run separately, and
continue HOTR-10 under full STS.

### HOTR-09 publication and hosted closeout

Focused commit `006428c3525fdb011e69a7e2bb948afd96451ea4` was pushed to PUBLIC
main; local and remote SHAs matched and the checkout was clean. All 38 staged
normalized source hashes, product hash, evidence schema, diff check and hooks
passed. Exact hosted Windows run
[34000552038](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34000552038),
job 101398521272, completed PASS at 2026-09-06T00:22:45Z. The downloaded manifest
confirms PASS on that same commit. Complete output and artifact remain under
`work/hotr-evidence/HOTR-09-hosted-34000552038/`.

Hosted verification runs the native/HTTP/retrieval/schema/owner/ConPTY/harness
suite and canary scan. The 15-minute load and genuinely separate authenticated
Windows principal remain local evidence; this closeout does not claim they ran
on the hosted worker. HOTR-10 proceeds under the existing full STS approval.

## 2026-09-05 local / 2026-09-06 UTC — HOTR-10 — Official-SDK MCP bridge

Full local gate PASS: `work/hotr-evidence/HOTR-10-25060-1788654017109858200/`.
Sanitized manifest `docs/evidence/HOTR-10-mcp.json`; actual exported schemas
`docs/evidence/HOTR-10-tools.json`. Product SHA-256
`dc317f36d05ed7a64e24e8c49cd0887601fdc4ecd7ce7e2ec79da6433e073cf5`;
runner SHA-256 `62d0732b2a7203d81262c3f8c52d61a772f9dac6ab5adf1f634bd91913ed9637`.

The pinned official rmcp 3.2.0 SDK supplies protocol handling. `hotr mcp` forwards
five fixed tools through its own DPAPI credential and the existing server-identity-
checking HTTP client. Rust/Serde contracts generate schemas for health, search,
get, create and permitted revision. No vault path/passphrase, owner operation,
arbitrary URL, shell or SQL argument is available through those tools. Requests
are authorized at the service, not by model output. Context and source fields
return verbatim; no cache or automatic write retry is introduced.

The SDK codec is bounded to 256 KiB frames; admission to 16 active requests and
128-byte displayed IDs. Request-extension guards release admission after handler
completion/cancellation. Output has a 1 MiB serialization check, 16-send cap and
five-second deadline; initialization/discovery has a fifteen-second deadline.
Stdout contains protocol traffic; generic stderr avoids request/credential traces.
Explicit process exit prevents an uninterruptible Windows stdin read from keeping
an otherwise-ended bridge alive. Transport loss leaves write outcomes uncertain
and callers reconcile with the original idempotency key/arguments.

Actual final fixture `work/hotr-tests/HOTR-07-68788-1788654207969556100/` starts
four real bridges against one real encrypted service, using two credentials.
Save/recall/current revision/source/replay, reader denial, accepted-state protection,
forbidden namespace, unknown owner tools, revoke while another client remains
authorized, and reconnect passed. Legacy protocols 2024-11-05/2025-03-26/2025-11-25
and current 2026-07-28 discovery/metadata/calls passed. Five live-exported schemas
also passed independent Draft 2020-12 validation. The new template's normalized
SHA-256 is `e459a0603174b9127b043ed39a23b8dfdb1f63421eeabe94cc47945e0e2e335e`.

A real bridge connected to a separate same-owner delayed HTTP listener so native
cancellation could be observed: cancel closed the forwarded socket, then ping
worked. Malformed/oversized frames, 17th active request, duplicate active IDs and
startup timeout with stdin kept open were rejected. This delayed peer is explicitly
a fault fixture, not authorization or named-app proof. Every stdout line is checked
as protocol; synthetic plaintext responses stay in test-process memory, while
managed fixture storage/logs are scanned for body/key/token leakage.

The initial focused compile log `HOTR-10-focused-1788653657978.txt` records a test
attempt to call a crate-private credential helper. The fixture now uses actual
owner CLI enrollment and a new synthetic profile for its delayed peer, preserving
the helper's visibility. The corrected focused run and current-protocol expansion
passed before the final full gate; all original diagnostics remain retained.

Full release format/build, warnings-denied all-target Clippy, native/HTTP/schema/
transaction/crash/owner/ConPTY tests and harness contracts passed. At
2026-09-06T00:24:37.4695623Z the actual distinct authenticated Windows account was
denied directory/database/marker/pipe access with error 5, could not decrypt copied
DPAPI ciphertext, and received zero application bytes at its fake TCP endpoint.
Owner state/vault hash stayed unchanged and lock ended the key holder. The final
scan passed on 1,251 files across 14 passing native runs. All 40 normalized source
hashes and the final executable must match the staged candidate before publication.

SDK crate checksum, exact upstream source revision and unmodified license text
are recorded in `docs/MCP.md`; the package's Apache metadata and upstream Apache/MIT
transition are distinguished. Only stdio/server features are enabled; reviewed
upstream HTTP/OAuth/redirect advisories do not describe this enabled transport.
The complete locked dependency/license review remains a later gate.

One canonical checkout, no linked worktree. Approximately 3.763 GiB of project
build/cache, synthetic fixtures and retained evidence remains, with 321.48 GiB
free. No existing user file or other application profile was deleted or replaced.
No real app was configured and no user vault or startup service was installed.
Publish the passed MCP checkpoint, track its exact hosted run, then continue
HOTR-11 encrypted backup/restore under full STS.

## HOTR-10 hosted closeout — 2026-09-06 UTC

Commit `51f0effef83cfabd71db29e2bc8f411850407e49` was verified on origin/main.
Exact hosted Windows run 34001324657 passed at 2026-09-06T00:44:04Z, job
101400601074. Full logs, status and the native artifact remain in the new
`work/hotr-evidence/HOTR-10-hosted-34001324657/` directory. The artifact's manifest
is under `windows-native-evidence/hotr-evidence/HOTR-03-7936-1788655025816636500/`.
Hosted verification is separate from the local two-account and fifteen-minute
load evidence. No named-application acceptance is implied.

## HOTR-11 — encrypted snapshots and fresh recovery — 2026-09-06 UTC

Local gate: PASS. `cargo xtask verify --prompt HOTR-11` retained complete evidence
under `work/hotr-evidence/HOTR-11-79624-1788655280543162700/` and sanitized results
in `docs/evidence/HOTR-11-backup.json` and `HOTR-11-recovery.json`.

Owner-only `backup` copies the unlocked worker connection into an exclusively new
SQLCipher database with a separately entered key. The native stepped online
backup API, closed-file hash, integrity checks and numeric watermark define the
completed snapshot. `restore` authenticates/checks a closed snapshot before
creating a new destination, copies encrypted-to-encrypted, invalidates every
copied active client, verifies integrity, and writes the ordinary vault marker
last. Existing paths are refused; failed new staging is retained. REST/MCP gain
no backup or filesystem operation. See `docs/BACKUP-AND-RESTORE.md` for bounds,
uncertain transport outcomes and explicit owner switching/reenrollment.

Changed source: `src/backup.rs`, owner/main/lib/capabilities dispatch, the shared
API test's recovery module, real Windows console tests, gate registration and
the native canary scanner. No schema or plaintext-storage fallback was added.

The initial real-service test failed before snapshot schema creation. Direct
native backup succeeded, isolating the failure to size preflight. Inspection of
pinned SQLCipher source showed `page_size`/`cipher_page_size` returns TEXT when a
codec is attached. Preflight now explicitly parses `cipher_page_size`; a native
multi-step regression and the complete recovery flow pass. All initial focused
and native diagnostic failure logs remain under `work/hotr-evidence/`.

Final fixture `work/hotr-tests/HOTR-07-70192-1788655468301571300/` ran four clients
writing 200 acknowledged mutations while a snapshot captured audit sequence 14,
13 records and 14 revisions/receipts. Every final acknowledged ID was checked for
presence exactly according to that watermark. Backup encryption used a different
key. Accepted revision/source data and namespace/reader restrictions survived;
all three old clients were denied and a newly enrolled reader succeeded. One
old token had been revoked before backup and another only after backup.

Wrong key, existing destination, modified ciphertext and truncation failed while
preserving the active database/WAL and the original snapshot. Updated outer
checksums did not make corrupted encrypted copies restorable. The ordinary
console gate exercised actual ConPTY backup/restore prompts without passphrase
echo. A malformed owner frame and a locked-vault backup were rejected.

Release format/build, warnings-denied Clippy, all native/HTTP/MCP/schema/owner
tests and harness checks passed. At 2026-09-06T00:45:47.4420395Z the actual distinct
authenticated Windows account received error 5 on directory/database/marker/pipe,
could not decrypt copied DPAPI ciphertext, and received zero application bytes at
its fake endpoint. Owner state and vault hash remained intact; lock ended the key
holder. The final scanner passed on 1,577 files across 15 native passing runs.

Product SHA-256: `a83c6c5adc090bc227e3ebe82fbeca4493b22bac41aa3a43bb192af7392a3d68`.
Runner SHA-256: `cd91a10bb59825506c1729f2dcdf4e5bd2d1189cf1cffcce02f10df651655ea3`.
All 42 normalized source hashes match the passed candidate. Before publication,
validate the staged versions against the same manifest and retain hook results.

One canonical checkout, no linked worktrees. Retained build/cache, synthetic
fixtures and evidence total approximately 3.783 GiB; free capacity is 320.19 GiB.
They remain for approved ongoing tests; no discretionary cleanup occurred.
No user vault, application profile, startup entry or other project was changed.
Publish this passed prompt, then continue HOTR-12 actual Codex/Claude application
proof, followed by Lamprey and the rest of full STS. Larger crash/fault/soak and
deployment gates remain open.

## HOTR-11 publication and HOTR-11-R1 — 2026-09-06 UTC

HOTR-11 commit `91c12e2333ea4482cd3cd9a5c621b6f03f12464b` passed hooks and was
verified on origin/main. Exact hosted run 34002264699 failed before the first
backup unit test could open a database: Windows error 3 at `src/backup.rs:398`.
The fresh runner lacked `work/hotr-tests`; the unit test assumed it existed.
Full logs and artifact remain under `work/hotr-evidence/HOTR-11-hosted-34002264699/`.

Repair scope: two lines inside the backup `cfg(test)` module validate/create
that parent. No runtime backup, storage, owner/API authorization or credential
behavior was edited. Targeted format/Clippy/native-copy checks passed, retained in
`work/hotr-evidence/HOTR-11-R1-1788656713452.txt`. The rebuilt executable hash
changed, so the complete HOTR-11 native gate was repeated rather than reusing the
previous binary's evidence.

Full local gate PASS: `work/hotr-evidence/HOTR-11-80132-1788656858761724100/`;
sanitized record `docs/evidence/HOTR-11-R1-fixture.json`. Format/build/strict lint,
native backup/restore and multi-step copy, HTTP/MCP, schema/crash, real ConPTY,
harness and second-account checks passed. At 2026-09-06T01:10:45.3410156Z the actual
separate authenticated account was denied four protected surfaces with error 5,
could not decrypt DPAPI ciphertext and received zero application bytes at its
fake endpoint. Owner state/vault stayed intact and lock exited the key holder.
The scanner passed 1,981 files across 17 passing native encryption runs.

Product SHA-256: `23fe71a07d6970ee1006a667480aca8e2ae4d7a0ab40faf0bc557399012f8190`.
Runner SHA-256: `b8d77294a4f124aca935a5d22676af4f184c189b9bd402a4f8d23499ed4ba6f5`.
The 44-source working-tree manifest includes five explicitly listed HOTR-12
driver/test/harness files that remain uncommitted. The application test was
ignored, and no model prompt was sent. This focused repair publishes only its
two source lines, plan/log updates and evidence. Do not call the local working
tree a clean exact-commit test; the repair's hosted run must establish that.

HOTR-12's initial local gate `HOTR-12-78964-1788656332064645300` was intentionally
interrupted at its separate-account stage after ordinary native checks, before
any provider request, to handle this prerequisite. Its source and complete
available logs/interruption record are preserved. Continue its full gate after
the repair's local publication and track clean hosted acceptance before M1
closeout. No existing application settings or user vault was changed.

### HOTR-11-R1 hosted closeout

Repair commit `cf4fa8ed1373431733e01c9f3faa1229f4e5c9fa` is verified on
origin/main. Exact Windows run 34003325869 passed, completed
2026-09-06T01:31:01Z. The downloaded `HOTR-03-3304-1788657847770867100` manifest
reports PASS, that exact SHA and `dirty=false`. Full log, run metadata and native
artifact are retained under `work/hotr-evidence/HOTR-11-R1-hosted-34003325869/`.
The fresh hosted fixture prerequisite is closed. Actual two-account evidence
remains the separately recorded local proof; no second account is inferred from
this hosted run. HOTR-12 continues in the same checkout.

## HOTR-12 — actual Codex/Claude prototype demonstration — 2026-09-06 UTC

Full local gate PASS: `work/hotr-evidence/HOTR-12-54916-1788658332732025800/`.
The actual application fixture is `work/hotr-tests/HOTR-07-80936-1788658498428104400/`.
Published records: `docs/evidence/HOTR-12-clients.json` and
`docs/evidence/HOTR-12-applications.json`, including actual tool results rather
than accepting final model prose. See `docs/M1-DEMO.md` and `docs/QUICKSTART.md`.

Changes: new `tests/support/apps.rs` drives the real service, owner CLI, protected
isolated profiles and two installed application binaries. `tests/api_capabilities.rs`
adds this module and an explicit-port restart helper. New
`integrations/clients/live_cli.py` runs the installed CLIs with bounded output,
deadlines, a durable model-prompt budget and actual event inspection. The xtask
registers HOTR-12 and refuses completion without installed-app and owner-boundary
checks. Documentation covers configuration, recovery and the tested boundaries.
No production storage/API/MCP implementation changed for this prompt.

Successful sequence (96.81 seconds): Codex creates/gets blue proposed revision 1;
Claude recalls that exact sourced record; Codex makes the owner-directed green
proposal at revision 2; owner acceptance creates revision 3; Codex recalls that
accepted revision. The owner snapshots, restarts on the same port and revokes
Codex; its actual MCP result is HTTP 401 while Claude still recalls revision 3.
After lock/restore into a fresh path, old Claude is denied with HTTP 401; a new
reader credential recalls the same accepted sourced revision. Native API checks
independently agree. The original Codex auth file's hash remained unchanged.

Actual versions/hashes: Codex CLI 0.153.4,
`a1cf6360ca71918d5466bc3a32d9f18b7044c9128756d1949e715d277b88c9b6`;
Claude Code 2.1.220,
`af5bf1f1b2aadffc768eccd787084c6fdf9ba81624cbe96c1c6d9ac1a1550231`.
Codex kept the selected gpt-6-astra model; Claude used its default Opus 5 route
with the existing Anthropic API credential. Only synthetic facts entered model
context. Claude's own auxiliary-model usage is reported separately by its CLI.
Eight successful-run user prompts and eleven total M1 prompts were recorded;
one of the twelve authorized slots remains. No automatic model fallback occurred.

Retained failures: `HOTR-12-78312-1788657202982975600` failed because npm Codex
does not accept `--strict-config` on `mcp list` (zero prompts). Run
`HOTR-12-79304-1788657568270048800` reached Codex 0.144.5, which rejected
gpt-6-astra as needing a newer CLI (one prompt). Run
`HOTR-12-72056-1788657912042237400` used the already-installed desktop CLI 0.153.4
successfully; Claude then saw a pending MCP server and no tools (two prompts).
The final driver disables Claude tool deferral, requests blocking startup and
queries that same CLI's control protocol until exactly five HOTR tools are ready
before reserving/sending its user prompt. Positive and missing-credential
negative preflights were exercised with zero model prompts. The missing
credential failed closed and left the budget unchanged. All traces remain in
their original synthetic directories; no failure was overwritten or relabeled.

Format, locked release build/tests, strict Clippy, harness tests, actual encrypted
backup/restore, ConPTY, HTTP/MCP and separate-account checks passed. At
2026-09-06T01:34:51.9752450Z a different authenticated Windows account was denied
directory/database/marker/pipe access with error 5, could not decrypt copied
DPAPI ciphertext, and received zero application bytes at its false endpoint.
The owner stayed unlocked with unchanged vault state, then lock exited the key
holder. The final independent scanner passed 2,883 files across 21 native runs.

Product SHA-256: `af39b4096fee4b3f7a831ac985e55ab42b5f07ac01acb5825c70dea45fa8f774`.
Runner SHA-256: `b8d77294a4f124aca935a5d22676af4f184c189b9bd402a4f8d23499ed4ba6f5`.
All 44 normalized source hashes were checked against the final passing manifest;
staged-source comparison is required before commit. The earlier HOTR-09 load
measurement remains tied to its own executable; it was not rerun or relabeled.

Preservation closeout: all eight original baseline copies still match their
manifest hashes. One canonical main checkout, no linked worktrees or duplicate
dependency trees. Approximately 3.846 GiB of project-generated files are retained
for ongoing STS and failure evidence; free space was 317.951 GiB. No discretionary
cleanup, existing-file deletion, user-vault install or active-profile change.
Publish this passed prompt to public main, verify the exact remote SHA and track
hosted CI, then continue HOTR-12-LAMPREY and the rest of full STS. The pending
owner-selected personal-file question remains unanswered; no personal import is
authorized by a timeout or preselected option.

## HOTR-12 hosted closeout; HOTR-12-LAMPREY in progress — 2026-09-06 UTC

Published main and origin/main remain
`ce1f8f7a8a72780aaf69f6bbf7a2d324f563518f`. The user explicitly confirmed PUBLIC
visibility. Exact Windows run 34004546514 passed; its downloaded native manifest
reports that SHA and `dirty=false`. Full logs/artifact are retained under
`work/hotr-evidence/HOTR-12-hosted-34004546514/`.

Lamprey is not complete. See `docs/LAMPREY-INTEGRATION-PROGRESS.md` for exact
application/source hashes, all preserved failures, the local repair, remaining
live gates and the approval-review block. New files: three installed-app CJS
drivers, `tests/support/lamprey.rs`, the detailed checkpoint, and the proposed
additional compatibility prompt budget. Modified source: `src/mcp.rs` inlines
generated request schemas; `tests/api_capabilities.rs` registers the Lamprey
tests; `xtask/src/lib.rs` includes CJS/JS in source evidence and registers the
bounded probes; `xtask/src/main.rs` runs those explicitly limited checks.

Actual installed Lamprey 0.32.0 connected from a protected isolated profile,
using its existing renderer IPC, native pre-tool hook and permission path.
The one live claude-opus-5 turn reached HOTR and proved a real forbidden-scope
403. Its write/correction smoke FAILED: Lamprey's provider normalizer dropped
search/create/revise because their schemas contained `$ref`. The failed run
`HOTR-12-LAMPREY-SMOKE-70820-1788660115289174000` and application events in
`HOTR-07-76676-1788660131670882700` remain retained.

The HOTR-side inline-schema repair passed its constraints regression, both
actual MCP integration tests, strict Clippy for both crates, and the 3,050-file
canary scan. Final frozen-source zero-model installed-app preflight PASS:
`HOTR-12-LAMPREY-PREFLIGHT-5440-1788660457263273700`; actual fixture
`HOTR-07-80868-1788660461091052000`. Product hash:
`4a827f3853ea9afe95477dfed4135da217676c2c0e40ea7edd8d576fc2843f24`.
The preceding preflight's source-change guard correctly failed its manifest;
the final run is the accepted preflight. No model-driven repair acceptance is
claimed. These source changes are uncommitted, with no prompt-completion claim.

Twelve of twelve original M1 user prompts are now used. The requested shared
72-prompt compatibility amendment is pending an actual reply. Automatic
approval review also rejected broad and then narrowly scoped formatting under
the no-overwrite instruction, despite the documented bounded-write approval.
No rejected formatting was applied. A corrected review-only diff exists at
`work/hotr-evidence/Lamprey-format-review-v2-1788660647805525800.patch`; all source
hashes remained unchanged while generating it. Final formatting scope includes
the two initially requested files and src/mcp.rs from the subsequent repair.
The first review-patch draft included rustfmt's filename header and is retained
as an invalid draft, not a patch to apply.

All eight immutable baseline copies still match. One canonical checkout, no
linked worktrees, no unpublished commits. Approximately 3.87 GiB generated
state retained; 317.58 GiB free at inventory. No discretionary cleanup, existing
user-file deletion, real vault installation, active-profile edit or personal
import. Continue the exact Lamprey live gate after required inputs, then the
remaining full STS roster in dependency order; do not stop at M1 or describe the
project as deployed.

## HOTR-12 publication complete; Lamprey deferred by owner — 2026-09-06 UTC

The user instructed: "No, commit HOTR 12 and merge with main. Worry about
Lamprey later." HOTR-12 implementation is already on local and remote main at
`ce1f8f7a8a72780aaf69f6bbf7a2d324f563518f`. Exact Windows CI run 34004546514 PASS
was verified again. No separate branch or merge remains. Publish this focused
documentation closeout directly to main with the implementation unchanged.

Changed documents: this log, VERIFICATION, the PSPR's current-status addendum,
the retained Lamprey checkpoint, and the new dated closeout/deferral record.
Check the recorded HOTR-12 source hashes against committed main, the staged
documentation-only scope, whitespace, repository hooks and exact remote SHA.
No model retest or source formatting is necessary for this documentation cut.

HOTR-12-LAMPREY is DEFERRED, not passed. Its four tracked source changes, three
new CJS drivers, new Rust test, unapproved budget proposal and raw evidence stay
in place and outside this commit's implementation scope. The earlier open
approval requests do not block HOTR-12. No files are deleted or reset, no new
worktree is created, and no whole-roster or deployability claim is made.

One canonical main checkout remains. Deferred work intentionally makes it
dirty; there were no unpublished commits before this closeout. Approximately
3.87 GiB of generated state is retained for evidence and later continuation.
The immutable original baselines remain preserved. Final closeout verifies
the new documentation commit on remote main and the deferred source hashes.

## HOTR-12-LAMPREY resumed under full compatibility authorization — 2026-09-06 UTC

The owner resumed all pertinent installed-app integrations and explicitly
pre-approved the necessary project permissions. The Lamprey deferral and the
pending compatibility-budget question are superseded by
`PLANNING/HOTR-COMPATIBILITY-RESUMPTION-APPROVED-2026-09-06.md`. Execution remains
sequential, with passed prompt commits on public main and no deletion of user
files or replacement of existing application profiles.

The retained inline-schema repair is exercised through the actual installed
Lamprey 0.32.0 application. A durable shared budget reserves each synthetic
prompt before inference, including failures. Its tests verify the shared
72-prompt limit, per-application ceilings, retained failed reservations, corrupt
ledger rejection and confinement to project work. The original M1 allowance
is unchanged. Six compatibility prompts have been consumed at this checkpoint.

Installed-app acceptance PASS:
`HOTR-12-LAMPREY-75412-1788665111194096200`, application fixture
`HOTR-07-74104-1788665251180908500`. Actual tool events prove create/recall/revise,
owner acceptance, service restart, an Opus 5 to Sonnet 5 conversation switch,
cancellation and recovery, forbidden-namespace 403 and revoked-credential 401.
An independent reader still retrieves accepted revision 3. This proof uses a
protected synthetic profile; the owner's active Lamprey profile is not enrolled.

Common gate `HOTR-03-40840-1788666049692772100` FAILED the existing MCP idle
initialization deadline test. Five other API integration tests passed, as did
the release build, strict product Clippy and eleven library tests. The retained
idle bridge stderr shows rejection after about 19 seconds, beyond the test's
18-second bound. The production 15-second initialization timeout and test bound
have not been relaxed. A complete unchanged-test rerun is in progress; this
entry does not claim the common gate, publication or entire roster passed.

The prepared-native environment helper avoids rebuilding SQLCipher/OpenSSL
for every client probe. It validates existing inputs and compiler timestamps,
uses the same bounded project cache and enters the installed x64 MSVC shell.
The first helper attempt referenced a nonexistent optional toolchain file;
that reference was removed before the next gate. No native library was replaced
by that environment-only repair. Failure evidence and original baselines remain.

The complete common rerun `HOTR-03-78452-1788676083262650400` PASS: 24 product
tests and five verification-runner tests; both formatting and strict Clippy
gates; release build; negative assertion/timeout/log-flood controls; and final
canary scan. Source and native-library snapshots matched at gate start/end.
The unchanged idle-MCP test passed; its rejection log was written about 15,037
ms after fixture stderr creation. This observation does not establish the cause
of the earlier timing overrun, which remains retained. The tested common-gate
binary was `977fa9b73f902511755729bd586d4e6227c1a31fba695c4f9fe21aa3a1f3a814`.
Final installed-Lamprey acceptance PASS:
`HOTR-12-LAMPREY-77184-1788676590328991300`, actual fixture
`HOTR-07-79188-1788676737638771600`. Six more successful prompts bring the shared
compatibility counter to 12 of 72 and Lamprey to its 12-prompt ceiling. The
application exited normally. The final app binary is
`0c4f87449f254bbe0cc46cdbfed1ecd0841f1e2a4c6d313ac4716e7a0fceeb16`.
Both gates have identical normalized source and native-library hash maps;
the separate executable hashes are retained in the sanitized source evidence.
Current source files match that evidence. Commit and exact hosted verification
follow; active-profile enrollment and remaining app prompts are still open.

Publication: `344b7a0a1ae37efe18ca19ca8b768d85e0b2788b` was committed directly
to main and pushed. GitHub's main commit matched exactly; the working tree was
clean immediately afterward. Existing staged/full-tree repository hooks passed.
Exact Windows CI run `34017610460` is in progress, not yet a hosted PASS.

## HOTR-12A — installed Hermes — implementation in progress

The installed Hermes 0.21.0 Python runtime and native MCP configuration were
inspected. The new driver uses its real CLI, restricted `mcp-hotr` toolset,
per-profile `HERMES_HOME`, an independent scoped credential, and the existing
Anthropic account. Title generation, background review, fallback providers and
unrelated memory features are disabled only in the synthetic profile. Each
model response is capped at 1,024 tokens, each turn at eight tool iterations,
and each application process at a bounded deadline under the Windows job.

The acceptance gate reads native Hermes session-database tool results. Model
prose and requested arguments do not qualify as evidence. Planned live sequence:
save/recall/correct/forbidden namespace, owner acceptance and service restart,
current sourced get/search in a fresh CLI session, revocation, independent
reader. Native discovery and driver/budget validation run before inference.
The owner’s existing Hermes configuration, credentials and history remain
outside the test profile. No Hermes acceptance or active enrollment is claimed
until the actual gate passes.

Hermes discovery PASS: `HOTR-12A-PREFLIGHT-30100-1788678037645678100`, fixture
`HOTR-07-29872-1788678195986607200`. Installed version 0.21.0, native MCP connected
in 5,594 ms and discovered exactly five tools; zero model prompts. The earlier
probe `HOTR-12A-PREFLIGHT-54872-1788677824165908800` failed in the new driver's
verbatim Windows-path ancestor walk before launching Hermes. The repaired guard
normalizes the drive-prefix spelling while checking every ancestor for links.

First model acceptance `HOTR-12A-66368-1788678264753739900` FAILED the verifier:
the actual Hermes turn successfully completed all five HOTR operations, but
Hermes exposes them through native `tool_search`/`tool_describe`/`tool_call`
routing and wraps returned context in an untrusted-data envelope. The original
verifier expected direct MCP function names and bare JSON. Its complete actual
trace remains at `HOTR-07-30004-1788678284120312800`; one prompt was charged.
No shell/file tools were in its five-entry discovered catalog. The installed
Hermes MCP implementation also emitted an unawaited-coroutine warning; its source
was not modified and the warning remains visible in the retained trace.

The repaired verifier checks the discovery catalog, pairs native call/result
IDs, validates the resolved HOTR tool name, and decodes only native tool-result
rows. It retains raw events alongside normalized evidence. Regression checks
reject foreign dispatch/catalog entries, mismatched or duplicate IDs and broken
wrappers. Replay of the retained real trace resolved five HOTR operations and
two metadata calls, including both sourced revisions and the actual 403 denial.
A fresh three-prompt acceptance is running.

The launcher now gives Cargo the same verbatim cache path as the bounded runner.
The following client gate reused its already compiled native test executable
(`cargo test` preparation 0.57 seconds in the first full attempt), replacing the
previous two-minute registry-dependency rebuilds. Existing native libraries and
cache contents were retained. A metadata inventory found 3.72 GiB readable
generated data; 390 restricted entries were not readable through that inventory
surface, so this is a lower bound. No linked worktrees were registered. No
permissions were changed for the inventory and no files were deleted.

Exact Lamprey Windows CI run `34017610460` completed successfully for
`344b7a0a1ae37efe18ca19ca8b768d85e0b2788b`. This closes its hosted gate; active
profile enrollment remains part of the broader installation work.

Hermes actual acceptance PASS: `HOTR-12A-77928-1788678502682238600`, fixture
`HOTR-07-78464-1788678526765346300`, installed main SHA-256
`cfcc631b3bb13b38e408d9e26c7a8ff981dabbb2393dfa1987427c5347b015da`.
Three successful Anthropic `claude-sonnet-5` prompts; five/two/one native HOTR
operations plus separately retained metadata discovery. All CLI processes exited
normally. The actual search result was also explicitly checked: total 1, one
accepted record at revision 3 with its original source, 785 estimated tokens
within a 1,024-token budget. Revocation returned 401; an independent reader
still read revision 3. The verifier's earlier failed attempt remains charged:
Hermes has used four of its eight allowed prompts, shared total 16 of 72.

Accepted app-gate executable SHA-256:
`11c715a88d8cce1ad4e85512acd08fd27ad326912fa6f67bb06bc26e3de36606`.
The native-result regression, driver syntax and prompt-budget checks all passed.
The required common gate is now running against the frozen source before the
focused HOTR-12A commit. No active Hermes profile change is claimed.

Common gate `HOTR-03-46516-1788678642414242100` FAILED in
`actual_owner_lifecycle_and_preservation`: the valid unlock following 4,096
successful status reconnects exceeded the existing five-second owner request
deadline. Build, strict product Clippy, eleven library tests, six ordinary API
tests, encryption and the separate console test passed. Full output is retained.
No owner implementation or timeout was changed. One complete unchanged rerun,
`HOTR-03-88712-1788679260865882200`, is checking reproducibility before closeout.
The actual Hermes acceptance remains a separate passed result; no model prompts
are being repeated for this native rerun.

## HOTR-12A-R1 — verification monitor repair

The unchanged rerun passed all 24 product tests, all five runner contracts,
both format/Clippy checks and the release build. Its scanner printed PASS for
3,891 files and exited zero, but the runner recorded `failure=timeout` at
69,596 ms. The complete gate therefore remains FAIL. `run` performed recursive
disk accounting on its deadline/pipe-draining thread; retained cache files were
reopened for metadata every five seconds. A process exit during that scan could
not be observed promptly. The earlier owner unlock timeout is not attributed
to this issue without further evidence.

The focused repair uses non-following Windows directory-entry metadata and
one background disk audit at a time. Child deadline polling and output draining
continue during audits; an outstanding audit is joined and any failure retained
before command completion. Resource thresholds, deadlines, path/reparse checks,
log ceilings and process ownership remain unchanged. A native accounting test
covers nested files and subsequent growth. The full gate and installed Hermes
acceptance will be repeated with the repaired runner and frozen source. This
small repair is bundled with HOTR-12A because it blocks that prompt's gate;
no unrelated product feature is included.

API behavior checked against Rust's primary reference:
https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.metadata.

Repaired common gate PASS: `HOTR-03-88344-1788679798730446900`.
All 24 product tests and six runner tests passed, including live owner unlock
after 4,096 reconnects and the new file-growth accounting regression. Both
format/strict-Clippy gates and all negative controls passed. The scanner passed
4,083 files in 25,815 ms; the command has no timeout failure. Frozen source and
native-library hashes matched. Product SHA-256:
`7536254f1471e810982da5d8288db4578bdf3213f2b069ed0e4b363144384aea`;
runner SHA-256:
`fb3c5a1abdcc362b62583103cb7847bf0b6731b39da7d0d925a460ccad579a0a`.
The final installed Hermes acceptance now verifies the same repaired source
before staged-source comparison and publication.

Final Hermes acceptance PASS: `HOTR-12A-79932-1788680065291303200`, fixture
`HOTR-07-50908-1788680074702627400`. Its product and runner hashes equal the
passing common gate above. All 59 normalized source hashes and native-library
hashes match across both gates. Actual get/search returned the accepted sourced
revision 3 within the 1,024-token context budget; forbidden scope returned 403,
revocation returned 401 and the independent reader retained revision 3. Three
normal CLI exits, seven total Hermes prompts and 19 of 72 shared prompts used.

The read-only closeout validator matched all 59 staged and working source files
against both manifests, checked local document links and staged credential
patterns, and verified all eight preserved originals. One canonical main
checkout, no linked worktrees or unpublished commits before this publication.
Retained generated state: 4,255,176,471 bytes (about 3.96 GiB), for build reuse
and complete synthetic failure/success evidence; free space 296,945,201,152 bytes
(about 276.55 GiB). No files were deleted. This focused 19-file commit includes
Hermes integration, its necessary monitor repair, evidence/workflow documents,
the current compatibility matrix, and Lamprey's exact hosted closeout. Publish
to main, record the resulting SHA/hosted run, and continue HOTR-12B.


## HOTR-13 — owner-selected import; approved resumption

The owner explicitly deferred remaining HOTR-12 compatibility and further Lamprey/plugin work and directed the next canonical prompt. Follow `PLANNING/HOTR-13-RESUMPTION-2026-09-06.md`; the earlier instruction to continue HOTR-12B is superseded.

The passed Hermes checkpoint was committed and pushed to public main as `5f6c6481af2892ad7da5ba499e7efc74cf8b4eac`. Exact Windows run `34041363996` was in progress at the latest check; no hosted PASS yet.

HOTR-13 adds explicit bounded UTF-8 text, Markdown and typed JSON imports through the owner channel. Default preview binds source hashes and current revisions to a vault-specific digest; commit reparses and verifies the exact batch, creates proposed records and stores its durable receipt atomically with revisions, audit and FTS. Repeated source imports preserve subsequent acceptance/corrections. Synthetic native tests exercise CLI, actual owner service, restart, stale previews, malformed data, persistence rollback, ID collisions, path limits and an actual Windows junction. Implementation is in progress; gates and publication remain pending. No personal files have been imported.

Initial HOTR-13 attempts retained: the verifier first refused the unregistered prompt before starting its gate. After allowlist registration, `HOTR-13-87380-1788708083643005200` failed to build because the URL type was referenced through a dev-only HTTP dependency. A patch-preparation mismatch caused an unchanged repeat, `HOTR-13-86504-1788708148787170900`, to retain the same failure. The fix uses the existing locked URL 2.5.8 parser directly; no native dependency version changed. The bounded gate `HOTR-13-68176-1788708184429672500` is in progress. Release build and strict product Clippy have passed at this checkpoint.


HOTR-13 full local gate PASS: `HOTR-13-68176-1788708184429672500`. All 27 product tests, six runner tests, both format/strict-Clippy gates, release build and the 4,347-file canary scan passed. The three new native tests used actual CLI/service processes and a real Windows junction. Source hashes stayed unchanged throughout the gate. The sanitized [evidence](evidence/HOTR-13.json) names exact fixture IDs and checks. Product SHA-256: `cf7d2c077b6387229da7ff2cbb99020ae3f46aeda3cdeb29b6f30d318602259a`; runner SHA-256: `7e0b866c1b027db837e2fb35ae4f5afd3d7fefc58f23fd7d28b9f7cea239fc36`.

Read-only closeout matched all 62 runtime/test/build source hashes and all eight preserved original baselines. One canonical main checkout; no linked worktrees, no unpublished commits before this prompt publication. Retained generated state: 4,435,000,055 bytes (about 4.13 GiB), for native cache reuse and synthetic evidence. Free space: 310,093,770,752 bytes (about 288.8 GiB). No pre-existing files were deleted, no personal content was imported, and no provider prompts were run. The new junction fixture alone was removed nonrecursively after verifying it was test-owned; its external-to-selection synthetic target remained intact. Small reviewed edit drafts and exact preceding-source backups are retained in the task workspace.

Hermes exact hosted Windows run `34041363996` completed successfully for `5f6c6481af2892ad7da5ba499e7efc74cf8b4eac`, closing that checkpoint. HOTR-13 is locally passed; stage/hash verification, focused commit, push, remote SHA and exact hosted result are recorded during publication. The next canonical prompt is HOTR-14. Remaining HOTR-12 app integrations and further Lamprey/plugin work stay deferred.


Publication integration: local commit `542f8ac` passed both repository hooks, but its non-force push was rejected because remote main advanced to `bb1977dbde1894180069cdad87b69cf79992a984`. The upstream commit only repairs missing synthetic fixture parents in owner, schema and Hermes tests. The unpublished HOTR-13 commit was rebased cleanly on that inspected change, preserving both. No user files were reset and no force push was used. A complete bounded native rerun now validates the combined source; earlier passing evidence remains retained.


Final combined-source HOTR-13 gate PASS: `HOTR-13-79532-1788708735741015700`, covering implementation `7a7db6070b4acb6f4fa317ac60fc57cfcacf451e` above preserved upstream `bb1977dbde1894180069cdad87b69cf79992a984`. Again, all 27 product tests and six verifier tests, both format/strict-Clippy checks, release build and final canary scan passed; the scan covered 4,574 files. Product SHA-256 `a75c10b89c97a4768702eb542b28727f8dd3088837afc51f7932269be94dd5c5`; runner SHA-256 `fc0dbf0283ec0395b959a93d9abcd6fb8d862625674c9fdfce738d94fc4a1ee1`. The three new import fixtures match this exact product binary. Earlier failures and the prior passing gate remain retained in [the evidence](evidence/HOTR-13.json).

Final read-only preservation/source inventory: all 62 source hashes and eight original baselines match; one canonical checkout, no linked worktrees. Generated state is 4,442,195,331 bytes (about 4.14 GiB), retained for cache reuse and evidence. The single unpublished implementation commit is ready for the authorized main push with this documentation-only closeout. No runtime code changed after the final gate. Publication must confirm exact remote SHA; hosted status remains a separate result.


## HOTR-14 — remaining full STS resumed

The owner instructed: "STS remaining HOTR prompts now. Do not report back until verified complete." Continue HOTR-14–36 sequentially under the existing bounded-write/publish authorization. Remaining HOTR-12 compatibility and further Lamprey/plugin work remain deferred. HOTR-13 implementation/closeout are published on main at `a11d1a4e7ebba9f45ab8132bbad9ece3c63ca837`; exact Windows run `34042917271` PASS verified.

HOTR-14 implements owner corrections, visibility intervals/tombstones, atomic supersession, versioned role/grant replacement, durable lifecycle receipts and conflict inspection. It reuses revision/audit insertion and the single writer. Ordinary get now enforces suppression even when a revision is supplied; explicit authorized history is labeled historical. Current reads remain uncached and recheck live policy. New actual CLI/two-bridge tests are pending; no completion claim until the gate passes. The HOTR-22 user-selected-material question is pending while earlier prompts proceed with synthetic fixtures.


Owner-directed standby (2026-09-06): "Get to a logical stopping point and standby so I can restart the app with the new astra-advisor plug-in." Finish the current HOTR-14 verification/publication checkpoint, preserve all work and stop. Do not start HOTR-15 until the owner resumes. HOTR-15 preparation was read-only: installed Ollama path and primary embedding-model documentation were inspected; no model download, inference or runtime/configuration change occurred.


HOTR-14 initial gate `HOTR-14-57708-1788710249573899900` failed the new lifecycle fixture: it called nonexistent `memory_get`/`memory_search` tool names instead of the established `hotr_get`/`hotr_search` catalog. The bridge correctly rejected the request. Build, strict product Clippy, all eleven ordinary library tests and nine other API integration tests passed. The fixture names are corrected; product protocol names and policy are unchanged. Full failure output remains retained, and a bounded complete rerun will establish the standby checkpoint.


The second gate `HOTR-14-89092-1788710486499592800` passed the correction/supersession/retention checks through two real bridges, then failed owner grant replacement. Schema v4 classified role as immutable credential identity, so the new explicit owner role update was rejected by its trigger. Schema v7 now preserves immutable client ID/token hash and permanent revocation while allowing owner-governed role policy updates. The live fixture additionally verifies that a downgraded reader cannot write in its newly allowed namespace. This is a migration repair within HOTR-14; the failure is retained and no prompt completion is claimed yet.


## HOTR-14 — verified standby checkpoint

The complete bounded local gate `HOTR-14-72804-1788710656448614500` PASS: 28 product tests, six runner tests, both format/strict-Clippy gates, release build and the 5,201-file canary scan. The actual owner CLI, HTTP and two MCP bridge processes proved current corrections, accepted/suppressed writer denial, supersession, future validity, expiry, tombstones, authorized historical access, immediate grant withdrawal, role downgrade, stale revision refusal and durable restart receipts. [Sanitized evidence](evidence/HOTR-14.json) binds these results to all 65 source hashes and product SHA-256 `d8ce5ecdde7e409ab06a20a72e64c2f97b3c8ba178548f9c4a2de062e7efeb26`; runner SHA-256 `1b66c1f587e9cb394e0fa5ba0e954d43167e4327ffc187bd68b10ad83c914b5d`. Both earlier failed runs remain retained. Named application/model workflows and later stress/soak gates were not rerun.

All eight preserved originals and current source hashes match. One canonical main checkout, no linked worktrees; no unpublished commits before this checkpoint publication. Generated state is 4,466,795,326 bytes (about 4.16 GiB), retained for native cache reuse and synthetic evidence; free space 309,655,142,400 bytes. No pre-existing files were deleted, no personal imports or provider calls occurred.

Latest owner direction: "Merge with main and stop." This checkout already uses main. Publish this focused passed checkpoint, verify the remote SHA and stop for the app restart; exact hosted verification remains separately tracked. On resumption, check publication/CI status and continue at HOTR-15. Do not start HOTR-15 during standby. Remaining HOTR-12 and further Lamprey/plugin work stay deferred.

Post-restart resumption: the owner directed continuation on the same trajectory using `astra-advisor:orchestration`. The interrupted commit did not complete; the 21-file HOTR-14 checkpoint remained staged on main at `a11d1a4`. The parent inspected the complete implementation/documentation diff and is repeating the bounded native gate before a fresh read-only reviewer. Parent model/effort are unobservable through available runtime metadata; requested reviewer is native `gpt-5.6-sol` / `high`, selected for owner-policy and transaction review. No reviewer completion or runtime configuration is inferred from the requested settings. Continue HOTR-15 only after HOTR-14 review and publication; previous standby is superseded.


Astra parent verification PASS: `HOTR-14-85332-1788713354061792200`, with 28 product tests, six verifier tests, both format/strict-Clippy gates and 5,442-file scan. All 65 source hashes match the earlier passing gate. Rebuilt product SHA-256 is `ec4aab64f85c0a9d9f23e54568092b9bace9daf649a8f9dddbf6377302c862df`; the fresh actual two-bridge fixture `HOTR-07-78416-1788713470101338000` matches it. The prior result is retained separately; bit-for-bit binary reproducibility is not claimed. Fresh native reviewer `/root/hotr14_review` was requested as `gpt-5.6-sol` / `high`; dispatch metadata exposes task identity/status but not observed model/effort or token usage. Review is pending.


ASTRA REVIEW `/root/hotr14_review` returned `fix-first`: the current-version-only backup validator rejected schema-5/6 backups before migration. The parent confirmed the path and repaired restore to authenticate the original manifest/schema/ciphertext and watermark before creating a new destination, revoke copied clients, migrate only the new copy under the existing deadline, recheck preserved counts/current schema and publish the vault marker last. A new real encrypted schema-5/6 fixture starts restored service processes, proves old-token 401 and fresh-client recall, verifies original backup bytes/receipts/audit, and rejects unsupported/future/mismatched schemas before destination creation. No existing backup is migrated in place. Parent verification and a new fresh review are required. Reviewer model/effort and tokens remained unobservable; no API-equivalent price is inferred.


Repair gate `HOTR-14-78016-1788713993335812800` failed only the new legacy fixture: its manually written `backup.json` inherited permissions instead of using the protected file ACL required by real backup output. Restore correctly rejected that synthetic input. The fixture now creates and flushes every snapshot/manifest file through the existing protected-file helper; the refusal checks therefore reach their intended schema boundary. Build, strict Clippy, eleven library tests and ten other API tests passed. Complete failure output remains retained; the full gate will rerun.


Final repaired HOTR-14 parent gate PASS: `HOTR-14-84644-1788714278331656100`. All 29 product tests, six verifier tests, both format/strict-Clippy gates, release build and 5,917-file canary scan passed. Actual schema-5/6 restore and lifecycle fixtures match product SHA-256 `e2fab29f42d9dab48cd3de89d388c01b6be66b00473a23d07aadbd6101057c50`; runner SHA-256 `1b66c1f587e9cb394e0fa5ba0e954d43167e4327ffc187bd68b10ad83c914b5d`. All 65 current source hashes and eight preserved originals match. The new legacy fixture proves original backup preservation, migration to schema 7, old-client 401, fresh-client sourced recall, preserved receipts/audit and schema refusal before destination creation. Prior gates/failures remain retained in [evidence](evidence/HOTR-14.json).

One canonical main checkout, no linked worktrees and no unpublished commits before publication. Generated state: 4,490,791,398 bytes (about 4.18 GiB), retained for build reuse and synthetic evidence. Free space: 308,813,549,568 bytes. No pre-existing file deletion, personal import or model/provider call occurred. Fresh reviewer `/root/hotr14_final_review` is checking the repaired full diff; no ship verdict is assumed.


Fresh ASTRA REVIEW `/root/hotr14_final_review`: `ship`, no findings. The reviewer independently matched all 65 normalized source hashes and inspected both legacy restore and lifecycle boundaries against the final passing gate. Native task metadata confirms identity/completion; actual model/effort and per-call token usage remain unobservable (requested `gpt-5.6-sol` / `high`). Read-only behavior was instructed and reported; sandbox-enforced read-only isolation is not claimed. API-equivalent cost and same-token comparisons are unavailable, not zero. Parent accepts HOTR-14 for the authorized focused main publication; exact hosted CI remains separate. Continue the approved sequence with HOTR-15.
