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
