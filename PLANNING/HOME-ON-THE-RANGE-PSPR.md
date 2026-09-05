# Home on the Range — Canonical Plan / Sequential Prompt Roster

Version: 1.0, 2026-09-05

Status: **APPROVED FOR FULL STS — EXECUTION ACTIVE**

Execution checkpoint: HOTR-01–06 and [HOTR-04-R2](HOTR-04-R2-HOSTED-PIPE-REPAIR.md) passed their local gates; exact R2 commit `eeddaedbd8d92b8fca1220156179d13f21253245` passed hosted Windows run 33995345800. HOTR-06's hosted fixture initialization failed and is tracked by [HOTR-06-R1](HOTR-06-R1-FRESH-FIXTURE-REPAIR.md). HOTR-07/08 execute as the [documented capabilities/REST bundle](HOTR-07-08-LIVE-BOUNDARY-BUNDLE.md), including that bounded fixture repair. HOTR-09 waits for its exact fresh hosted pass. All earlier failures remain retained. See DEVLOG and VERIFICATION for evidence and remaining gates.

Current amendments: [bounded write approval](HOTR-BOUNDED-WRITES-APPROVED-2026-09-05.md), [compatibility expansion](HOTR-STS-APPROVAL-AND-COMPATIBILITY-2026-09-05.md), and [Lamprey Harness integration](HOTR-LAMPREY-HARNESS-INTEGRATION-2026-09-05.md). Original proposal/approval wording below is retained as plan history; these dated user approvals govern execution. Consult DEVLOG and VERIFICATION for actual prompt status.

Initiative: HOTR

Owner: USS-Parks

Publication branch: `main`

## 1. Governance and intended result

Build a small usable context vault on the owner's Windows machine, harden it through measured failures and sustained load, and decide whether the tested result is suitable for local distribution. The product supplies shared, persistent, searchable context to separately authorized applications. It does not run or orchestrate the owner's agents.

Success means an approved application can save a sourced fact, another approved application can retrieve it after restart, a correction supersedes the old fact, an unauthorized application is denied, and an encrypted backup restores the same accepted state. Later gates measure semantic retrieval, resource bounds, failure recovery, and installation outside the development checkout.

“Bullet proof” is an engineering aspiration, not an acceptance label. Claims must name the tested operating system, application versions, dataset size, concurrency, fault model, and remaining limitations.

### Authority, location, and history

- Authoritative repository: https://github.com/USS-Parks/Home-on-the-Range.
- Canonical checkout: `C:\Users\17076\Documents\Codex\Home-on-the-Range`.
- Source of truth: this PSPR, `docs/DEVLOG.md`, `docs/VERIFICATION.md`, approved amendments, then implementation and its evidence. Code/tests establish actual behavior; the plan establishes requested behavior.
- Repository was verified private and empty before the planning commit. The project starts on `main`; there is no imported codebase and no temporary worktree.
- The conversation's earlier SQLCipher recommendation was an architecture proposal. Existing memory projects have documented overlap; no reuse candidate has yet passed local security or integration acceptance.
- Changes to scope, stack, gates, or targets are dated amendments. Keep superseded decisions and failed results visible. Never rewrite an old result to make it look successful.

### Authorization

The current request authorizes preparation and publication of this PSPR and supporting governance files. The user explicitly requests a roster **for STS approval**. Accordingly, HOTR-01 through HOTR-36 are not executed by publishing this document. “Make the small prototype here and now” defines the first executable milestone; explicit STS approval starts its implementation.

Standing authorization from the user: commit and push all in-scope project work to this repository's `main` from now on. This includes approved implementation, tests, documentation, and sanitized evidence. It never means committing private context, vault files, secrets, dependency caches, or raw sensitive logs. No additional commit/push confirmation is required within approved scope. Preserve repository visibility and branch protections; do not force push.

Approval options:

1. **Recommended:** `Run it STS through M1: HOTR-01 through HOTR-12. Commit and push each passed prompt to main.` This ends with a usable small local prototype and a milestone closeout.
2. Approve a later milestone after the preceding milestone passes.
3. `Run the full PSPR STS through HOTR-36.` This authorizes sequential continuation through all listed local work and test campaigns without extra milestone approval. It does not authorize public hosting, public release publication, commercial licensing purchases, whole-machine ingestion, OS-wide security changes, or deletion of unrelated artifacts.

Named-prompt approval is also valid, subject to dependencies. Stop at the end of the approved range, a required user action, or a failed prerequisite that cannot be repaired in scope. Do not silently skip a gate. A user-approved live-gate deferral must be dated and cannot yield a full acceptance claim.

### Verified preparation and unresolved prerequisites

On 2026-09-05, this host reported Windows 11 Home, an AMD Ryzen 7 5800H (8 cores / 16 logical processors), approximately 59.9 GiB usable RAM, and approximately 339 GiB free on C:. Rust/Cargo 1.98.0 and Node 24.15.0 were available. These are preparation observations, not frozen benchmark conditions; remeasure at execution.

GitHub authentication allowed repository inspection and clone. Codex, Claude Code, and Cursor launchers were found. Ollama is running with existing local models; an embedding model has not been selected or verified. An Ollama listener was observed on a wildcard IPv6 address; HOTR must use explicit loopback, and this plan does not change Ollama's global networking. Do not submit private material to any cloud-tagged model.

Agentmemory listeners exist at its expected local ports. Its local skills describe hybrid retrieval, local embeddings, a viewer, and optional shared bearer authentication. The current installation's encryption, authorization isolation, backup guarantees, and actual data path remain unverified. No existing memory content needs to be read during the reuse review.

The restricted shell runner returned Windows error 5; approved execution outside that runner succeeded. Native SQLCipher/OpenSSL compilation, compiler prerequisites, application credentials, a protected administrator IPC channel, private-repository hosted CI availability, and a second Windows security principal remain execution prerequisites. Their absence is a blocker to the relevant claim, not permission to replace its proof with mocks.

## 2. Scope, architecture, and settled defaults

### Scope of the small prototype (M1)

One encrypted vault; one service owns database connections and serializes writes; explicit user unlock; local command-line management; separate application credentials; shared and project namespaces; sourced/versioned text records; exact and keyword search; loopback REST; a stdio MCP bridge; encrypted backup/restore; and live use by two supported applications. No semantic model, desktop wrapper, automatic transcript harvesting, or agent orchestrator is required to make M1 usable.

### Proposed stack, ratified by HOTR-01/02

| Concern | Default | Override boundary |
|---|---|---|
| Implementation | Rust; Tokio/Axum for the small service, Clap/Serde for CLI/contracts | A stack change needs a dated ADR and approval if it expands scope or changes security boundaries |
| Storage | SQLCipher through rusqlite; encrypted indexes and vector blobs in the same vault | Never fall back to plaintext SQLite; verify compiled cipher version and actual file behavior |
| Windows crypto build | Bundled SQLCipher with a supported OpenSSL provider; locked dependencies | Exact crate and native-library versions selected after advisory/build checks |
| Database ownership | Single service with bounded writer queue; SQLite transactions/WAL and `synchronous=FULL` | Read pooling only after measured need; no direct client file access |
| MCP | Official Rust SDK, stdio bridge forwarding into the existing service | Negotiate supported protocol versions; remote/HTTP MCP OAuth is parked |
| App API | Versioned JSON REST bound to `127.0.0.1:47821`; configurable loopback port | Reject wildcard/non-loopback binding in this release; never silently choose a different service port |
| Admin channel | Local Windows named pipe with explicit owner ACL, remote rejection, and server/client identity checks | No unlock, credential issuance, restore, or policy administration exposed to ordinary application tokens |
| Vault key | User passphrase entered through a no-echo local prompt; SQLCipher's supported KDF/key APIs | No custom cipher/key-envelope scheme; no passphrase in arguments, source, logs, environment, browser storage, or agent context |
| Application credentials | Cryptographically random capability tokens; server stores hashes; per-client role and namespace grants | Windows user-scoped credential storage for clients; no shared administrator token in agent configuration |
| Search | FTS5/keyword first; local embedding + exact cosine and reciprocal rank fusion at M2 | Start without an ANN/vector server; tune or add a vetted index only after a measured bottleneck |
| Embeddings | Pinned local model through an explicit loopback Ollama endpoint; model/license selected at HOTR-15 | No cloud fallback; no arbitrary per-request model/provider URL; quantify a model download before it occurs |
| UI | Small locally served management UI at M2; no Electron/Tauri shell for the prototype | Read-only context viewer first; admin changes remain on protected local CLI until their authentication is proven |
| Installation | User-scoped Windows application and optional logon startup; portable build first | Windows service account, VM, Docker, Linux/macOS packaging, and LAN service are parked |
| Data location | `%LOCALAPPDATA%\HomeOnTheRange\vaults\default` once implementation is approved | Explicit alternate local directory allowed; no Git, synced folder, or network filesystem by default |

The same executable may expose `serve`, `mcp`, and administrative CLI subcommands. Do not split into microservices. Client bridges hold their own limited credential and do not receive the vault passphrase. API behavior, not the LLM's judgment, decides authorization and validity.

### Context and permission contract

Records contain a stable ID, namespace, kind, title/body, source reference and optional content hash, creator identity, observed/created/updated timestamps, revision, tags, status, and explicit supersession links. Kinds include fact, preference, decision, plan, task, and lesson. A roadmap is structured content; it is not a separate database engine. Simple relations are tables, not a new graph service.

Application roles are reader and contributor. Contributors can create records and revise records they are permitted to edit; accepted/authoritative records require owner approval to supersede. Credentials, namespace grants, restore, key changes, and retention policy are owner operations. A record saying “grant me access” has no authority. Project access does not automatically include shared/global context; grants are explicit.

Updates use expected revisions and atomic transactions. Repeated writes use principal-scoped idempotency keys plus request hashes. A timeout may leave a committed result; retrying the same request must retrieve that result, never duplicate it. Incompatible retries are rejected. Reads disclose record ID, revision, status, and source. Search, counts, histories, relations, audit views, exports, and caches enforce the same namespace permission boundary.

### Threat boundaries and key lifecycle

Protect against copied locked vaults/backups without their passphrase, unauthorized API clients, hostile web origins, accidental cross-project disclosure, conflicting writers, malformed inputs, interrupted operations, and stale or malicious stored content. Use synthetic secrets to test leakage across the database, WAL/journal, indexes, temporary files, backups, errors, diagnostics, and application logs.

The owner account and operating system are trusted. Per-client tokens and user-scoped DPAPI storage do not isolate hostile processes running as that same Windows user. An administrator, compromised client, debugger, or memory reader can exceed this boundary. Returning plaintext to an authorized cloud-backed client allows that client to send it to its provider. Revocation cannot retract text already returned or undo copies in older backups. Do not market stronger guarantees.

The owner unlocks manually after startup. Lock rejects new requests, drains or cancels work with defined outcomes, clears sensitive caches, closes the database, and terminates the key-holding process. A lock is complete only when that process has exited. Windows paging, hibernation, and system crash dumps are outside application-file encryption guarantees; inspect and document their implications without changing OS policy automatically.

Recovery means a verified encrypted backup plus its passphrase stored separately by the owner, potentially in an existing password manager. There is no password reset that can recover a lost key. Passphrase rotation preserves an old encrypted recovery point until the new vault is verified, then explains which old copies still require the old passphrase. No password-manager integration is required in M1.

### Explicit exclusions

No public endpoint, SaaS tenancy, multi-machine sync, remote MCP OAuth server, agent execution/orchestration, web browsing, arbitrary SQL/shell tools, general document OCR, automatic whole-machine capture, biometric unlock, custom cryptography, invisible model-driven rewrites, automatic deletion of valuable context, or blanket compatibility claim. No commercial product launch decision is inferred from a passing local prototype.

## 3. Reuse ledger

HOTR-01 freezes revisions/licenses and a short capability assessment. Do not spend weeks implementing competing systems merely to compare them. Limit initial executable reuse spikes to two candidates and one bounded day of effort; unresolved questions become explicit findings.

| Candidate/component | Classification | Intended treatment | Evidence required |
|---|---|---|---|
| Existing agentmemory | Reuse/integration candidate | Preserve installation; inspect metadata/contracts and test a synthetic import/export path if suitable | Encryption coverage, per-client grants, local processing, maintenance/licensing, and conflicts with its existing runtime |
| OpenMemory | Existing product to evaluate | Compare setup and cross-client workflow; do not duplicate its whole stack by default | Real local dependencies, model data flow, encryption/auth gaps, Windows setup burden |
| Perseus Vault | Reuse/extraction candidate | Inspect its compact Rust storage/retrieval design and licensed modules | Body-only encryption versus indexes/metadata; token scopes; compiled dependencies; actual Windows behavior |
| Memory-Vault | Reuse/extension candidate | Inspect scoped credentials and local memory workflow | Encryption boundaries, key storage, transport policy, maintenance and migration costs |
| Herdr / herdr-memory | Existing integration seam | Optional future source/client; no Herdr requirement | Supported transcript formats, provenance, ingestion consent, revision behavior |
| SQLCipher / SQLite | Direct reuse | Transactions, encryption, FTS, backup facilities | Actual cipher-enabled build, native version/advisories, integrity checks and license notices |
| Official MCP SDK | Direct reuse | Handshake, framing, schemas, cancellation | Supported negotiated protocol and two real clients |
| Windows ACL/DPAPI APIs | Direct reuse | Protect local administrative IPC and stored app credentials | Real second-user rejection and same-user limitation documentation |
| HOTR record/policy contract | New work at a small service boundary | Namespaces, revisions, accepted-state rules, bounded context response | End-to-end correctness and denial tests |
| Fault/retrieval evidence | New project-specific work | Seeded harnesses and comparison datasets | Reproducible results on the release executable |

If an existing product meets the requirements with a small extension, prefer that extension or a thin integration over rebuilding it. License notices and upstream provenance accompany extracted code. Adoption of a different database/security boundary needs a dated, reviewable amendment before execution continues.

## 4. Verification gates before implementation

### Common prompt gate

Each prompt supplies one focused deliverable and meaningful tests for changed behavior. Before commit: format/lint applicable files, pass focused tests, inspect staged changes for secrets/unrelated files, and run `git diff --check`. Rust implementation prompts run the applicable `cargo fmt --check`, warnings-denied Clippy, and locked tests; broader suites run at milestone or dependency boundaries. Do not repeat expensive tests without a change or unresolved concern.

`cargo xtask verify --prompt HOTR-NN` is the intended gate interface created in HOTR-03, not an existing command today. Prior prompts record their exact manual commands. It records commands, exit status, source/binary hashes, dataset seed, environment, and evidence references without secrets. It must fail when a required command is skipped.

One prompt per focused commit, then push to `main` and verify its exact remote SHA. If a gate fails, retain diagnostics, fix the focused problem, rerun the affected gate, and only then commit. For a hosted gate, local checks permit publishing the candidate commit, but status remains awaiting hosted evidence until its exact run passes. Never claim a local result is hosted proof.

### Invariants that block acceptance

| Gate | Required result |
|---|---|
| G1 — Stored confidentiality | Real SQLCipher build; wrong key fails; ordinary SQLite cannot read it; file-level synthetic canary scan finds no plaintext in application-managed storage, index, journal, backup, temp, or logs; findings investigated rather than excused by encryption branding |
| G2 — Authorization | Every allowed/denied role × namespace × operation case matches policy; direct ID/history/count/export/cache paths included; credentials never appear in responses/logs; owner IPC rejects an actual second Windows principal |
| G3 — Durable truth | Zero lost acknowledged writes in the specified crash campaign; no duplicates from replay; no silent revision loss; integrity check passes after successful recovery; corrupt inputs fail explicitly |
| G4 — Retrieval correctness | Exact lookups return the authorized current record/revision; superseded/deleted/expired records are absent from default retrieval; semantic evaluation meets the frozen thresholds below |
| G5 — Real integration | Two actual applications use separate credentials against one running vault; create/recall/correction/restart/revocation demonstrated; SDK simulations are supplemental |
| G6 — Recovery | Consistent encrypted snapshot while writes occur; restoration into a fresh vault preserves accepted state and policy; restored credentials are invalidated until owner reenrollment; wrong-key/tampered backup fails without replacing the live vault |
| G7 — Bounded operation | Queue, input sizes, result sizes, CPU workers, RAM, disk, timeouts, cancellation, and log growth have enforced limits; overload returns controlled errors, not hangs or partial transactions |
| G8 — Deployment evidence | Clean Windows user/environment installs the packaged build without Rust/Node tooling; exact packaged executable passes smoke and targeted fault checks; source, package hashes, SBOM/licenses, and hosted runs are linked |

### Numerical targets and measurement rules

Targets below are proposed acceptance thresholds, not measurements. Freeze them and the reference machine in HOTR-03; changing a failing target requires an explicit dated amendment and reason. Report warm/cold behavior separately. Include timeouts and rejected requests in totals, not just successes.

- **Prototype corpus:** 10,000 records, 1–4 KiB typical bodies, 10 namespaces, and adversarial Unicode/path-like values. At 8 concurrent clients, 20 aggregate requests/second, 80% reads / 20% writes for 15 minutes: write and keyword-search p95 ≤500 ms, unexpected errors <0.1%, zero correctness/security violations. Key derivation/unlock measured separately.
- **Limits:** default request ≤256 KiB, body ≤64 KiB, result count ≤50, response/context byte ceiling explicit, writer queue ≤256, request deadline ≤10 seconds. A context-packing request also enforces a declared token estimate/budget; token estimates are labeled as estimates, not exact provider accounting.
- **Retrieval evaluation:** at least 120 hand-reviewed queries across exact identifiers, paraphrases, temporal changes, conflicting facts, no-answer cases, and access restrictions. Freeze at least 40 held-out queries. Exact-ID/current-revision correctness 100%; authorized paraphrase Recall@5 ≥90%; no prohibited IDs/text in any result. If a no-answer threshold/abstention feature is offered, false-positive rate on the labeled no-answer subset ≤5%; otherwise expose scored candidates without claiming an answer. Hybrid must improve paraphrase Recall@5 by ≥10 percentage points over lexical search unless the lexical baseline already reaches 90%, in which case non-regression suffices.
- **Scale target:** 100,000 records, 100 namespaces, 32 concurrent clients, 50 aggregate requests/second, 80% reads / 20% writes for 30 minutes. Write/keyword p95 ≤1 second; hybrid p95 ≤2 seconds end to end including local query embedding on a warmed model; p99 ≤5 seconds; unexpected errors <0.1%; zero corruption, lost acknowledgments, duplicates, or policy violations. Report the maximum sustainable rate independently of the target.
- **Soak:** 4 hours with 16 clients at 20 aggregate requests/second; mixed CRUD/search, periodic encrypted backups, and selected safe restarts. Compare private bytes over a fixed-size live corpus after warmup; unexplained retained-memory growth must be <10% and <128 MiB. Separate expected database growth and Windows file cache from a process leak.
- **Crash campaign:** at least 100 reproducible termination/restart cycles around instrumented pre-commit/post-commit/response/checkpoint boundaries. Record the client's durable acknowledgment journal separately from server logs. Process kills do not prove sudden power-loss durability; no deliberate power cuts on this machine.
- **Abuse/race campaign:** at least 10,000 same-record conflicting updates/retries; at least 100,000 generated malformed/permission-mutated API operations; all predefined authorization combinations; 100 credential-revocation races. A request authorized and committed before revocation can complete; no new operation admitted after revocation completion may use that credential. Historical output already delivered is not retractable.
- **Recovery target:** restart/unlock-ready within 60 seconds for the reference 100k corpus, excluding human passphrase entry. Restore 100k records within 5 minutes on this host, including verification. Backup schedule defaults off until an owner chooses a local target; documented recovery point is the last verified snapshot, not continuous backup.

### Resource and fault-test safety envelope

Stress tools operate only on an explicitly created synthetic vault tagged with the current run ID under repository `work/`. They validate canonical paths, reject junction/symlink escapes, and refuse the real vault or existing service paths. Deletion of those exact generated test fixtures is part of the approved test prompt; unrelated cache/worktree removal still requires explicit authorization.

Default test ceilings: 4 CPU worker threads, 8 GiB combined service/model/harness private memory, 20 GiB generated disk data, bounded logs, one corpus and one scratch restore at a time, and abort if C: free space falls below 25 GiB. Reduce concurrency before affecting the owner's interactive work; record any resource adjustment. Do not load the installed large generation models for stress tests. Synthetic model stubs are used for volume; the real local embedding model is exercised in designated passes.

Use injected storage errors or a bounded isolated filesystem fixture for disk-full/permission failures; never fill or damage the host drive. Kill only PIDs created and tracked by the harness. No host reboot, OS account change, firewall change, or persistent startup installation merely to simulate a fault. Where a real isolated-user/login step is required, prepare the exact command and pause for that narrow user action if permissions/tools do not support it.

Local integration acceptance may use at most 12 short synthetic prompts across the owner's already configured AI applications per milestone, with no private context and no new paid account/provider. Cloud-backed clients remain clearly labeled. Load/fuzz/soak tests never spend cloud-model tokens. Model downloads require a recorded size, license, hash, and available capacity; approval of HOTR-15 allows one suitable local embedding model up to 1 GiB.

## 5. Milestones and approval cuts

| Milestone | Prompts | Usable result | Exit |
|---|---|---|---|
| M1 — Small local prototype | HOTR-01–12 | Encrypted shared context through two actual clients, keyword retrieval, scoped credentials, verified restore | Prototype demonstration and exact main SHA |
| M2 — Searchable daily-use vault | HOTR-13–18 | Selected imports, durable corrections, local semantic retrieval, measured relevance, management viewer | Retrieval comparison and UI evidence |
| M3 — Hardened local operation | HOTR-19–22 | Audit visibility, safe key rotation, Windows startup/lock behavior, controlled real-work pilot | Operational and recovery runbook |
| M4 — Stress-tested candidate | HOTR-23–32 | Authorization/fault/race/soak evidence and fixed reproducible defects | Full evidence matrix with no critical/high open findings |
| M5 — Deployment decision | HOTR-33–36 | Installable Windows candidate, clean install, hosted verification, go/no-go report | Limited claim: personal use / local beta / not ready |

Each milestone may be approved independently. Full-roster approval permits automatic sequential continuation; it does not erase required live tests. Version labels are planned: M1 `0.1.0-prototype`, M3 `0.2.0-local`, M5 `0.3.0-rc`. They are not release tags until explicitly included in an approved publication action.

## 6. Ordered sequential prompt roster

Every item below is **NOT STARTED**. HOTR-00 is the separate documentation preparation recorded in the log. Dependencies also include all prior prompts in the approved sequence; the listed dependency highlights the immediate prerequisite.

### M1 — Small local prototype

#### HOTR-01 — Ratify the foundation from machine and reuse evidence

**Depends on:** STS approval.

**Objective:** Choose the smallest defensible implementation route.

**Work:** Read governance, inventory repository/installed memory-service metadata and resources, inspect up to two leading reuse candidates at pinned revisions, and score encryption coverage, authorization, Windows operation, recovery, license, and maintenance. Preserve agentmemory and Ollama.

**Deliverables:** `docs/adr/0001-foundation.md`, completed reuse ledger, blocker list.

**Acceptance:** Every must-have has verified evidence or an explicit gap; no unverified candidate is called secure. Ratify the proposed Rust/SQLCipher route or submit a specific stack amendment before dependent implementation. No vault contents copied.

#### HOTR-02 — Prove native encrypted storage on Windows

**Depends on:** HOTR-01.

**Objective:** Establish that the selected build actually encrypts all database storage.

**Work:** Create the minimal locked Rust package, pin native dependencies, verify compiler/OpenSSL prerequisites and current SQLCipher advisories, then create/read/reopen a synthetic encrypted database with FTS and WAL.

**Deliverables:** Build configuration, native-version manifest, encryption smoke test.

**Acceptance:** Native Windows release build succeeds; runtime reports a real cipher version; correct key works, wrong key and ordinary SQLite fail; canaries absent from DB/WAL/temp/logs. Encryption silently disabled is a hard failure. No product data yet.

#### HOTR-03 — Establish the reproducible verification harness

**Depends on:** HOTR-02.

**Objective:** Make later evidence repeatable and bounded.

**Work:** Add the small `xtask` gate runner, seeded corpus generator, owned-PID/path guards, per-run manifests, secret redaction, timeouts, disk/RAM caps, and a minimal Windows CI build/test workflow. Freeze reference targets and a source/binary hash convention.

**Deliverables:** Gate runner, synthetic fixtures, CI definition, evidence schema.

**Acceptance:** An intentionally failed assertion and timed-out child make the gate fail; outside-root paths and unrelated PIDs are refused; deterministic seed reproduces input; local build/tests pass. Record hosted CI separately when the exact commit is published.

#### HOTR-04 — Implement create, unlock, lock, and local administration

**Depends on:** HOTR-03.

**Objective:** Give the owner exclusive control of the key-holding process.

**Work:** Build no-echo create/unlock prompts, SQLCipher key handling, explicit owner ACLs on files and named pipe, remote-pipe rejection, first-instance protection, identity validation, lifecycle states, and lock-by-process-exit. Handle occupied port/pipe deterministically.

**Deliverables:** Vault lifecycle and owner CLI; key-boundary runbook.

**Acceptance:** Locked startup discloses no context; invalid unlock is bounded and redacted; lock ends key-holding process and connections; duplicate service fails; genuine second Windows principal cannot administer/open protected files. If no second principal is available, this live gate remains blocked.

#### HOTR-05 — Define versioned context records and namespaces

**Depends on:** HOTR-04.

**Objective:** Preserve where context came from and how it changed.

**Work:** Add migrations, kinds, namespaces, source references, revision history, accepted/proposed states, tags, and simple relation tables. Validate size and Unicode handling. Reject unsupported future schemas without touching their files.

**Deliverables:** Schema, migrations, typed record contracts.

**Acceptance:** Create/reopen/migrate known fixtures preserves IDs and history; constraints reject malformed relations and oversized bodies; newer-schema fixture is left byte-for-byte unchanged; source references are returned but never automatically fetched.

#### HOTR-06 — Make writes atomic and retries safe

**Depends on:** HOTR-05.

**Objective:** Prevent silent overwrites and duplicate mutations.

**Work:** Implement the bounded single-writer queue, expected revisions, principal-scoped idempotency keys/request hashes, transaction-level audit events, and explicit canceled/committed/unknown-to-client outcomes.

**Deliverables:** Transaction service and retry contract.

**Acceptance:** Concurrent conflicting updates have one winner; stale writers receive a conflict; identical retries return one result; same key/different body is rejected; crash/retry at an acknowledgment boundary produces no duplicate or missing acknowledged revision.

#### HOTR-07 — Enforce application capabilities centrally

**Depends on:** HOTR-06.

**Objective:** Give each client only its approved operations and namespaces.

**Work:** Owner-only token issuance/revocation; CSPRNG tokens, hashed verification, reader/contributor grants, owner acceptance rules, and Windows user-scoped credential storage. Apply policy in the service before any store/retrieval operation.

**Deliverables:** Credential manager and permission matrix.

**Acceptance:** Complete role/namespace/operation matrix passes on the running process; reader cannot mutate, contributor cannot administer or alter accepted facts; spoofed principal fields are ignored/rejected; revoked credential fails on an existing connection's next request. No raw token in source, CLI arguments, or logs.

#### HOTR-08 — Expose the bounded local REST API

**Depends on:** HOTR-07.

**Objective:** Let authorized local apps use the vault without database-file access.

**Work:** Add typed `/v1` record/revision/status endpoints, credential-derived identity, stable error codes, JSON/body/depth limits, deadlines, no-store responses, allowed Host/Origin policy, and explicit loopback binding. No owner administrative routes.

**Deliverables:** REST implementation and client examples.

**Acceptance:** Real HTTP tests prove authentication, namespace filtering, hostile Origin/Host rejection, malformed/oversized input handling, cancellation, and overload responses. No redirects, permissive CORS, public bind, SQL execution, or internal stack/secret disclosure.

#### HOTR-09 — Deliver useful exact and keyword retrieval

**Depends on:** HOTR-08.

**Objective:** Make the prototype useful without embedding infrastructure.

**Work:** Implement ID lookup, scoped FTS5 queries, exact-name/path boosts, current-revision filtering, pagination, safe query parsing, source-bearing results, and response budgets. Count/list/history obey the same grants.

**Deliverables:** Search endpoint, lexical fixtures, baseline report.

**Acceptance:** Exact lookup/current revision is 100% correct; default search excludes superseded/deleted state; hidden namespaces never appear in hits/counts/errors. Run the 10k-record prototype load target with encrypted FTS and publish measured percentiles.

#### HOTR-10 — Connect MCP through the existing service

**Depends on:** HOTR-09.

**Objective:** Offer model-independent memory tools through a standard client interface.

**Work:** Add the official-SDK stdio bridge exposing health, search, get, create, and permitted revision tools; forward using its own stored credential. Keep the DB and master passphrase out of the bridge. Provide project-local connection templates.

**Deliverables:** MCP bridge, schemas, compatibility contract.

**Acceptance:** Real protocol initialization, tools/list, calls, error mapping, cancellation, and reconnect pass; stdout contains only protocol frames; bridge cannot access owner operations. Separate bridge processes use distinct credentials against one service.

#### HOTR-11 — Recover from an encrypted backup

**Depends on:** HOTR-10.

**Objective:** Prove that useful context survives loss of the active vault file.

**Work:** Use a supported consistent encrypted backup path, verify integrity/manifests, stage restore into a new local vault, validate before switching, and invalidate restored application tokens until owner reenrollment. Do not copy a live WAL database naively.

**Deliverables:** Owner backup/restore commands and recovery instructions.

**Acceptance:** Back up during writes; restore record/revision/policy state to a fresh path; verify the snapshot watermark. Wrong key/tampered/truncated backup fails without changing the active vault; restored previously revoked credentials cannot regain access; canary scan covers backup/staging files.

#### HOTR-12 — Demonstrate and close the small prototype

**Depends on:** HOTR-11.

**Objective:** Prove the complete workflow in two real installed applications.

**Work:** Configure a project-scoped Codex CLI connection and a Claude Code connection (or document a justified available-client replacement). Use synthetic facts: A saves, B recalls, owner accepts/corrects, A recalls current state; restart, revoke A, restore and reenroll. Record exact application versions and tool traces without credentials.

**Deliverables:** M1 demo evidence, quickstart, gate matrix, main publication closeout.

**Acceptance:** Two actual applications succeed with independent credentials; revoked A is denied while B remains allowed; restart/restore retain accepted context. An SDK harness does not count as either app. G1–G6 and prototype bounds pass; report all limitations and retained generated storage. Stop here if only M1 was approved.

### M2 — Searchable daily-use vault

#### HOTR-13 — Import only owner-selected material

**Depends on:** HOTR-12.

**Objective:** Bring useful existing context into the vault without indiscriminate capture.

**Work:** Add explicit file/JSON/Markdown import with dry-run preview, canonical path validation, allowed formats, content hashes, provenance, size limits, deduplication, and atomic batches. Treat imported facts as proposed.

**Deliverables:** Import command and synthetic compatibility fixtures.

**Acceptance:** Preview matches committed import; repeated import creates no duplicate; malformed input leaves batch unchanged; junction/traversal/network-path escapes fail. User selects actual files before personal data is imported. No Codex managed-memory edits or provider uploads.

#### HOTR-14 — Make corrections and retention consistent

**Depends on:** HOTR-13.

**Objective:** Ensure subsequent retrieval reflects explicit corrections and exclusions.

**Work:** Implement owner acceptance/supersession, conflict presentation, validity/expiry, tombstones, permission changes, and revision-aware cache invalidation. Keep history accessible only to authorized callers. Expiry suppresses retrieval; it does not silently erase history.

**Deliverables:** Context lifecycle contract and correction workflow.

**Acceptance:** A correction appears across both clients immediately after commit; old default results disappear; unauthorized writers cannot overwrite accepted state; expiry/deletion/scope changes suppress every retrieval path; old backups' retention implications are documented.

#### HOTR-15 — Build local semantic indexing

**Depends on:** HOTR-14.

**Objective:** Add embeddings without exporting private context.

**Work:** Select one licensed, pinned ≤1 GiB local embedding model; document download hash and dimensions; use explicit loopback Ollama, redirects disabled, timeouts, queue/backpressure, bounded retries, encrypted vector storage, and per-record revision/model generation. No cloud fallback.

**Deliverables:** Embedding adapter, model manifest, encrypted index worker.

**Acceptance:** Observed inference traffic stays on loopback; unexpected model/dimensions/NaN fails safely; model-down state preserves writes and lexical search; worker resume is idempotent; stale embeddings cannot resurrect old revisions. No model downloaded into another project's cache tree.

#### HOTR-16 — Combine semantic and keyword results

**Depends on:** HOTR-15.

**Objective:** Retrieve relevant, current, authorized context within a caller budget.

**Work:** Implement permission-filtered exact cosine search, lexical/vector rank fusion, deterministic tie handling, revision checks, source-bearing snippets, and token/byte-budget packing. Include index freshness and degraded-search status in responses.

**Deliverables:** Hybrid search and context-pack endpoint/tools.

**Acceptance:** Namespace restrictions apply before candidate ranking and again before return; cross-client cache keys cannot leak results; superseded vectors are excluded; lexical-only degraded mode is explicit; response budgets hold for Unicode and large records.

#### HOTR-17 — Measure whether semantic search earns its cost

**Depends on:** HOTR-16.

**Objective:** Demonstrate retrieval improvement on an independent evaluation set.

**Work:** Create/review the 120-query corpus and held-out partition, freeze labels, compare lexical-only and hybrid search, and report latency, index time/size, recall, wrong-revision hits, no-answer behavior, and authorization negatives. Tune only on development queries.

**Deliverables:** Reproducible evaluation corpus and relevance report.

**Acceptance:** Section 4 retrieval thresholds pass on held-out queries with zero access leaks. Any failure is retained with a repair/retest; do not relabel examples or weaken thresholds to manufacture a pass. Report both wins and regressions.

#### HOTR-18 — Make the vault inspectable by its owner

**Depends on:** HOTR-17.

**Objective:** Provide a small usable management viewer.

**Work:** Serve a local UI for search, source/revision history, conflicts, client/grant status, index health, and backup status. Use a short-lived owner-approved viewer session with secure local handling; no credentials in URLs/localStorage. Encode untrusted content, set CSP, and keep privileged changes on the owner CLI.

**Deliverables:** Viewer and local screenshots/interaction evidence.

**Acceptance:** Actual browser tests cover navigation, keyboard use, long text, empty/error/locked states, malicious stored markup, session expiry, origin/CSRF protections as applicable, and cache clearing. M2 closeout links passing retrieval results and main SHA.

### M3 — Hardened local operation

#### HOTR-19 — Make access and changes auditable without leaking content

**Depends on:** HOTR-18.

**Objective:** Let the owner identify which client read or changed context.

**Work:** Record principal, operation, target IDs, revision/watermark, outcome, and time; keep sensitive audit metadata encrypted. Add owner-filtered inspection, bounded log retention, denial/rate-limit counters, and redacted diagnostics. Define behavior when required audit persistence fails.

**Deliverables:** Audit API/CLI, diagnostics, retention contract.

**Acceptance:** Mutations and their audit events commit atomically; authorized retrievals are traceable by record/revision; denial logs contain no bodies or credentials; filling log quota cannot consume unbounded disk. Audit integrity is not described as tamper-proof against the OS owner.

#### HOTR-20 — Rotate vault and client secrets safely

**Depends on:** HOTR-19.

**Objective:** Change credentials without losing the vault or reviving revoked access.

**Work:** Implement owner-driven passphrase rotation using supported SQLCipher operations with a verified pre-rotation recovery point, exclusive maintenance state, and interrupt recovery. Separately rotate app tokens and document backup/key epochs.

**Deliverables:** Rotation commands and recovery matrix.

**Acceptance:** Successful rotation accepts only the new passphrase on the active vault; old backup behavior is explicitly verified; interruption at every instrumented step yields either verified old or new state, never silent loss. Tokens revoked before rotation/restore remain unusable.

#### HOTR-21 — Integrate the Windows process lifecycle

**Depends on:** HOTR-20.

**Objective:** Make startup, lock, shutdown, and recovery predictable during daily use.

**Work:** Provide user-scoped install/start/stop/status and optional logon-start registration, always locked until manual unlock. Handle session lock/suspend by closing the key-holding process; recover stale locks/PIDs safely; prevent startup loops. Prepare unregister commands and preserve vault data on uninstall.

**Deliverables:** Windows lifecycle integration and runbook.

**Acceptance:** Actual login/start, session lock/unlock, sleep/resume where user permits, abnormal exit/restart, and occupied endpoint cases behave as documented. OS-wide changes are not made for tests. Missing live interaction remains an explicit gate, not a simulated pass.

#### HOTR-22 — Run a controlled local-use comparison

**Depends on:** HOTR-21.

**Objective:** Establish whether the vault solves the owner's workflow better than shared files.

**Work:** On user-selected material, execute at least 10 representative handoff/recall/correction tasks across two apps and two service restarts. Compare against a shared Markdown baseline; record setup effort, repeated explanations, stale answers, source accuracy, and owner corrections. Keep personal results local; publish sanitized metrics.

**Deliverables:** M3 pilot report and prioritized defects.

**Acceptance:** The system completes each scripted data-operation invariant, has no unresolved high-impact operational defect, and the report honestly states where it helps or adds overhead. This gate does not establish market demand or willingness to pay. Stop if approval ends at M3.

### M4 — Security, fault, and stress campaign

#### HOTR-23 — Attack the authorization boundaries

**Depends on:** HOTR-22.

**Objective:** Find unauthorized access across every shipped surface.

**Work:** Exercise cross-project IDs, history/relations/counts, viewer sessions, exports, caches, token spoofing/replay/revocation, browser origins, named-pipe ACLs/impersonation, locked state, admin routes, and restore reenrollment. Use a real second Windows principal for OS boundary tests.

**Deliverables:** Exhaustive permission matrix and reproducible denial cases.

**Acceptance:** Zero unauthorized context/metadata disclosures in the declared matrix; all identified bypasses repaired and replayed. Same-user hostile-process limits remain explicit. Source review alone cannot close this gate.

#### HOTR-24 — Fuzz inputs and test malicious context

**Depends on:** HOTR-23.

**Objective:** Prevent malformed requests and stored instructions from taking over service behavior.

**Work:** Run ≥100k generated API/protocol/schema cases with oversized/deep JSON, Unicode, FTS syntax, malformed vectors, hostile markup, forged source/status/policy text, and boundary IDs. Exercise authorized synthetic prompt-injection examples through a local client.

**Deliverables:** Seeded corpus, minimized crash cases, injection report.

**Acceptance:** No service crash, unbounded allocation, shell/file/network execution, or permission change from stored content. API policy is enforced even if the model obeys a malicious note. Do not claim models are universally immune to prompt injection.

#### HOTR-25 — Stress concurrent writers and retries

**Depends on:** HOTR-24.

**Objective:** Prove revision, idempotency, and revocation semantics under contention.

**Work:** Run ≥10k same-record races, duplicate request replays, client cancellations/disconnects, delayed responses, queue saturation, and ≥100 revocation races across distinct client processes.

**Deliverables:** Independent expected-state model and contention report.

**Acceptance:** Actual state matches the reference history; zero lost accepted revisions or duplicated operations; incompatible retries conflict; queue/deadline bounds hold; post-revocation admission follows the defined policy. Controlled overload is measured separately from unexpected failures.

#### HOTR-26 — Interrupt transactions and recover

**Depends on:** HOTR-25.

**Objective:** Prove crash consistency within the process-failure model.

**Work:** Execute ≥100 instrumented crash/restart cycles around transactions, audit commits, result delivery, WAL checkpoints, and graceful shutdown. A separate client journal records received acknowledgments.

**Deliverables:** Crash harness, seed/phase matrix, recovered-state evidence.

**Acceptance:** Every acknowledged write survives; ambiguous requests reconcile by idempotency key; integrity and revision/audit consistency hold. Record process-kill coverage separately from untested hardware/power-loss behavior.

#### HOTR-27 — Fail storage, backup, restore, and rotation

**Depends on:** HOTR-26.

**Objective:** Bound failure when storage becomes unavailable or inconsistent.

**Work:** Inject disk-full/read-only/access-denied/short-write errors using disposable fixtures; test locked files, truncated/tampered DB/WAL/backup, wrong keys, backup-during-write, interrupted restore, and interrupted passphrase rotation. Include future-schema and older-backup cases.

**Deliverables:** Storage/recovery fault matrix and recovery-time report.

**Acceptance:** No silent success after failed persistence; no overwrite of the last verified recovery point; wrong/corrupt data fails closed; restore/rotation cannot resurrect app privileges; documented 100k recovery targets pass. Host filesystem and real vault remain untouched.

#### HOTR-28 — Break the embedding and index pipeline

**Depends on:** HOTR-27.

**Objective:** Keep authoritative context correct when semantic infrastructure fails.

**Work:** Stop/timeout the test embedding endpoint, return malformed values, change model hashes/dimensions, interrupt rebuilds, race corrections/deletions against indexing, and revoke a namespace while results are cached. Do not terminate the owner's unrelated Ollama jobs.

**Deliverables:** Index-failure fixtures and freshness report.

**Acceptance:** Writes/lexical recall remain available within limits; degraded status is visible; old generations never mix; suppressed records do not reappear; reindex resumes without duplicate work or uncontrolled retries. Endpoint substitution cannot send context to a remote host.

#### HOTR-29 — Find the actual load and size ceiling

**Depends on:** HOTR-28.

**Objective:** Measure the service's useful operating range.

**Work:** Run the frozen 10k and 100k workloads, then step concurrency 1/8/32/64 and request rate until controlled overload or resource limits. Within the same 20 GiB/8 GiB ceilings, optionally grow a synthetic corpus toward 1 million records to locate scale failure; no additional success claim is required for that exploratory tier.

**Deliverables:** Latency/throughput/error/resource plots, raw synthetic measurements, supported-envelope statement.

**Acceptance:** Reference 100k targets pass; cold startup/indexing and warm retrieval are separated; overload fails predictably and recovers; no data/policy violations. A failed exploratory tier is reported, not hidden or mistaken for supported scale.

#### HOTR-30 — Run the four-hour bounded soak

**Depends on:** HOTR-29.

**Objective:** Detect leaks and degradation that short runs miss.

**Work:** Run the fixed-size 16-client mixed workload for four hours with bounded periodic backup and controlled restarts. Record memory slope, handles/threads, queue depth, DB/WAL sizes, latency drift, failures, and acknowledgment state. Keep a user-visible cancel path and progress snapshots.

**Deliverables:** Soak manifest, charts, reconciliation report.

**Acceptance:** Memory-growth and error thresholds pass, all acknowledgments reconcile, no policy violation, no runaway files/handles/threads, and service remains responsive afterward. An interrupted run is incomplete; rerun the affected soak after a material fix.

#### HOTR-31 — Review security and dependency exposure

**Depends on:** HOTR-30.

**Objective:** Review implementation risks beyond the exercised test cases.

**Work:** Review auth/IPC/key/storage/backup/import/retrieval boundaries, native SQLCipher/OpenSSL versions, dependency advisories/licenses, build provenance, accidental telemetry, unsafe code, secret logging, and repository hygiene. Use a supported security-review workflow; do not turn a scanner's empty report into proof of security.

**Deliverables:** Threat-to-control traceability, SBOM/advisory report, severity-ranked findings.

**Acceptance:** No unresolved critical/high finding or known exploitable shipped dependency. Medium findings have explicit impact/disposition; any material acceptance of residual risk is presented to the owner. Do not claim an independent external audit unless one actually occurred.

#### HOTR-32 — Repair, regress, and freeze the tested candidate

**Depends on:** HOTR-31.

**Objective:** Close the campaign on a single traceable candidate.

**Work:** Add focused `HOTR-32-R01`, `R02`, etc. repair prompts when defects need independent commits; preserve original failures. Rerun the affected matrix and milestone smoke tests, repeat soak when runtime/memory behavior changed, and freeze source/binary/dependency/model hashes.

**Deliverables:** M4 evidence index, repair ledger, candidate manifest.

**Acceptance:** All mandatory M4 gates pass on the candidate or unchanged components demonstrably covered by the same hashes; no unresolved critical/high risk or missing security/live proof. Report retained storage and main SHA. This is a tested candidate, not yet a distributable product.

### M5 — Packaging and deployment decision

#### HOTR-33 — Package the tested Windows application

**Depends on:** HOTR-32.

**Objective:** Produce an installable local candidate without developer tooling.

**Work:** Build a portable Windows package with the service/CLI/bridge, bundled viewer assets, third-party notices, version manifest, checksums, example client configuration, and optional user-scoped install/uninstall scripts. Keep embeddings optional with explicit download instructions; do not bundle an unrelated large model.

**Deliverables:** Local candidate package and install/recovery documentation.

**Acceptance:** Package contains no keys/vaults/logs, starts without Rust/Node installed in its environment, and uses explicit data paths. Uninstall preserves user vaults and removes only registered project-owned startup entries. Code signing is reported accurately; no unapproved certificate purchase.

#### HOTR-34 — Reproduce installation in a clean Windows environment

**Depends on:** HOTR-33.

**Objective:** Prove the package works beyond the development account.

**Work:** Use a user-approved clean Windows account or suitable isolated Windows environment; install from the package, create a new synthetic vault, connect two clients, import, search, revoke, lock, restore, and exercise an upgrade from a prior fixture. Record setup steps and prerequisite failures.

**Deliverables:** Clean-install evidence and compatibility matrix.

**Acceptance:** G8 passes with the exact packaged executable; selected auth/crash/recovery checks pass there; upgrade preserves context and rollback does not open an incompatible schema. Windows Home does not guarantee Sandbox/Hyper-V availability; absence of a suitable environment is an explicit blocker, not a developer-account substitute.

#### HOTR-35 — Reproduce builds and verify hosted checks

**Depends on:** HOTR-34.

**Objective:** Make the candidate reproducible and reviewable from GitHub main.

**Work:** Finalize pinned Windows build/test workflows, minimal token permissions, locked dependencies, package checksums/SBOM, artifact retention, and redacted synthetic evidence. Compare local/hosted builds and explain nondeterministic binary fields if byte identity is unavailable.

**Deliverables:** Hosted run links, build provenance, artifact manifest.

**Acceptance:** Exact source commit's required hosted jobs pass; downloadable private CI artifact matches its manifest; local acceptance is linked to the tested package hash. No claim of reproducible identical binaries unless verified. CI unavailability leaves this gate blocked.

#### HOTR-36 — Decide local deployability and close the roster

**Depends on:** HOTR-35.

**Objective:** Make an evidence-backed release decision.

**Work:** Reconcile all gates, hashes, findings, client/OS support, measured limits, backup/key responsibilities, install burden, and user-workflow results. Inventory worktrees/generated data and publish the sanitized final report to main.

**Deliverables:** `docs/DEPLOYABILITY.md`, final DEVLOG/verification entries, supported-client matrix, recovery runbook, next-decision list.

**Acceptance:** Choose exactly one verdict: `PERSONAL-USE ONLY`, `READY FOR LIMITED LOCAL BETA`, or `NOT READY`. Any missing mandatory security/recovery/install evidence prevents the beta verdict. Commercial viability remains unproven; public release, hosting, pricing, broad platform work, and promotion require separate authorization.

## 7. Completion and closeout protocol

A prompt is locally verified only when its prescribed checks pass. It is published only after the remote main SHA is checked. A milestone is accepted only when its real-client and other live gates pass and its evidence index identifies the tested source/executable. Keep these states distinct.

At each milestone record: approved range, completed prompts, exact implementation SHA, documentation closeout SHA, remote main SHA, gate results/links, remaining blockers, supported behavior, worktree inventory, dirty/untracked/unpublished work, and approximate generated-data size. Keep one canonical checkout; no temporary worktrees are planned. Preserve local-only work before any later cleanup.

Commit-SHA bookkeeping must not chase itself: evidence names the tested implementation SHA; a following documentation-only closeout commit records it. The final report records the new main SHA externally if needed. Binary behavior need not be retested merely because a documentation-only SHA changed; the candidate manifest must show no runtime inputs changed.

Final completion requires all approved prompts closed honestly, no secrets or private context in Git, an installed/recoverable local result at the approved milestone, and a truthful deployability verdict. If only M1 is approved, M2–M5 remain planned and are not implicitly started.

## 8. Primary references and evidence limits

Checked during preparation on 2026-09-05. Upstream descriptions establish evaluation candidates, not acceptance of their implementation.

- [SQLCipher design](https://www.zetetic.net/sqlcipher/design/): page encryption and journal/WAL behavior. Application key handling, temporary files, and backups still need tests.
- [rusqlite features](https://docs.rs/crate/rusqlite/latest): SQLCipher/OpenSSL build options; resolved native versions require independent inspection.
- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk): reuse protocol machinery; pin a stable supported release rather than a moving main branch.
- [MCP transports](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports): local binding, origin validation, and transport separation; negotiate protocol compatibility for actual clients.
- [Windows named-pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights): default ACLs are insufficient for the owner administration channel.
- [Windows data-protection scope](https://devblogs.microsoft.com/oldnewthing/20240327-00/?p=109580): user-scoped encryption is not app isolation against other processes with that identity.
- [OpenMemory](https://mem0.ai/blog/introducing-openmemory-mcp), [Perseus Vault](https://github.com/Perseus-Computing-LLC/perseus-vault), [Memory-Vault](https://github.com/fusae/Memory-Vault): overlap to evaluate, not reasons to assume security or Windows readiness.
- [Herdr](https://herdr.dev/docs/agents/) and [herdr-memory](https://github.com/jatingargiitk/herdr-memory): potential integration surfaces; not required runtimes for HOTR.

Local preparation also read installed agentmemory architecture/configuration skill guidance and inspected matching listener metadata. No encryption/access-control test of that installation has been performed.

## 9. Amendments and approval record

| Date | Event | Effect |
|---|---|---|
| 2026-09-05 | Initial user-requested PSPR | HOTR-01–36 proposed; standing commit/push authorization to private main recorded; implementation pending explicit STS approval |
| 2026-09-05 | Full STS and stack approved | User selected SQLCipher through rusqlite and expanded named application/provider/OS compatibility; separate addenda preserve prompt history |
| 2026-09-05 | Preservation exception approved | User explicitly approved the bounded project write proposal; original project files snapshotted before edits; outside-scope files remain protected |

Record the user's approved prompt range and any resource/stack amendments here before executing the first prompt. Do not backfill approval that was never given.
