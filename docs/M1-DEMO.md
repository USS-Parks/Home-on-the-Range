# M1: tested local prototype

HOTR-12 passed its complete local gate on 2026-09-06 UTC. Two real installed
applications launched independent MCP bridges into the same encrypted service.
The test used new protected application profiles and synthetic content only.
No user vault, active application profile, existing setting, or startup entry
was installed or replaced.

| Step | Actual application or owner action | Required observed result |
|---|---|---|
| 1 | Codex CLI 0.153.4 creates and gets a sourced fact | Blue, proposed revision 1 |
| 2 | Claude Code 2.1.220 gets the same ID | Same body, revision and opaque source |
| 3 | Codex makes the owner-directed correction | Green proposal, revision 2 |
| 4 | Owner accepts; Codex gets current state | Green, accepted revision 3 |
| 5 | Owner backs up, restarts and revokes Codex; Codex gets | Actual HTTP 401 in its MCP result |
| 6 | Claude gets after restart | Same accepted revision 3 |
| 7 | Owner locks and restores into a fresh path; old Claude gets | Actual HTTP 401 after restored-token invalidation |
| 8 | Owner reenrolls Claude with a new reader credential; Claude gets | Same accepted revision 3 and source |

[Published actual tool results](evidence/HOTR-12-applications.json) establish
what each application received; final model prose alone was not accepted.
Independent native API checks also verified durable state and denials.
The eight-step run took 96.81 seconds. Eight user prompts were sent in this
successful run; eleven were sent across M1, including the preserved failed
attempts. Claude also reports its own auxiliary model usage; a user prompt is
not a claim of exactly one underlying provider request.

## Gate matrix

| Invariant | Prototype evidence | Remaining broader gate |
|---|---|---|
| G1 stored confidentiality | Real pinned SQLCipher, keyless/wrong-key/tamper refusal, encrypted WAL/index/backup; final scanner passed 2,883 files across 21 native runs | Later key rotation, storage faults and packaged build |
| G2 authorization | Real HTTP/MCP role and namespace matrix; same-connection revocation; actual separate Windows account denied files/pipe/DPAPI and received no token at its endpoint | Expanded clients, audits, grant changes and revocation race campaign |
| G3 durable writes | Atomic revision/receipt/audit transactions, native interruption/replay tests; all 18,000 prototype requests reconciled | Prescribed 100 crashes and 10,000 conflicting-update campaign |
| G4 current retrieval | Encrypted FTS, current/history/count/list filtering, literal query parsing and response budgets; two real clients received revision 3 | Owner lifecycle completion, embeddings and held-out relevance evaluation |
| G5 application integration | Eight-step actual Codex/Claude workflow with independent credentials | Lamprey, the additional compatibility roster and other OSes |
| G6 recovery | Snapshot during 200 concurrent acknowledged writes; verified watermark, different backup key, rejected bad restores; actual app restore/reenrollment | 100k recovery target, storage faults and clean package install |

The 10k/15-minute workload is the earlier HOTR-09 measured executable, linked by
its own source/binary manifest. Its retrieval path is unchanged; it was not
repeated or relabeled as a new HOTR-12 benchmark. The complete HOTR-12 native
gate and installed-client workflow are tied to product SHA-256
`af39b4096fee4b3f7a831ac985e55ab42b5f07ac01acb5825c70dea45fa8f774`.
The [gate manifest](evidence/HOTR-12-clients.json) records all 44 source hashes,
native libraries, runner hash, commands, limits and exit results.

## Reproduction and retained failures

Use [QUICKSTART.md](QUICKSTART.md) for an owner-operated workflow and
[INSTALLED-CLIENTS.md](INSTALLED-CLIENTS.md) for client configuration. Automated
proof uses `cargo xtask verify --prompt HOTR-12` with the explicit installed
executable/auth paths and `HOTR_RUN_CLIENTS=1`; the second-account challenge is
mandatory. Do not rerun the model gate without accounting for the durable
twelve-prompt milestone budget.

The first application preflight rejected a Codex flag before any model request.
The npm Codex 0.144.5 then rejected the selected model as requiring a newer CLI.
The already-installed 0.153.4 worked. Claude's first model turn saw no tools
because its MCP startup was deferred; blocking startup, upfront tools and an
actual same-process readiness check fixed that path. A missing-credential
negative control still fails before sending any model prompt. All failures and
redacted original streams remain locally retained.

## Completion boundary

This milestone demonstrates a Windows local prototype. The full STS roster
continues immediately with Lamprey and the expanded application work. Semantic
search, owner lifecycle, real-work pilot, four-hour soak, larger stress/fault
campaigns, clean installation and deployment assessment remain open. No universal
compatibility or deployment claim follows from these two CLI results.
