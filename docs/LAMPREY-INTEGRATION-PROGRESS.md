# Lamprey acceptance checkpoint — 2026-09-06 UTC

Current status: **DEFERRED by the owner**. The user requested HOTR-12 publication
and said "Worry about Lamprey later." The checkpoint below is retained history.
Its open live tests, formatting request and unapproved budget proposal do not
block HOTR-12 closeout. See the
[dated deferral](../PLANNING/HOTR-12-CLOSEOUT-AND-LAMPREY-DEFERRAL-2026-09-06.md).
The Lamprey implementation remains uncommitted and preserved for later work.

Full STS remains in progress. HOTR-12-LAMPREY is not complete or published.
The repository is intentionally PUBLIC. Published main and origin/main both
resolve to `ce1f8f7a8a72780aaf69f6bbf7a2d324f563518f`.

## HOTR-12 hosted closeout

Windows run [34004546514](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34004546514)
passed at that exact SHA; updated 2026-09-06T01:52:58Z. The downloaded native
manifest `HOTR-03-1832-1788659284885520900` reports PASS and `dirty=false`.
Full log and artifact are retained in `work/hotr-evidence/HOTR-12-hosted-34004546514/`.
The actual Codex/Claude provider runs remain the separately recorded local
HOTR-12 proof. Hosted CI is not a claim of hosted account or GUI acceptance.

## Installed Lamprey proof and defect

Actual installed application: Lamprey 0.32.0. Executable SHA-256
`0533fc603293f1860e10213e008291bbddda45ff04c6a439506ae537e008b26a`.
Installed main-script SHA-256
`d9d08465d0216032058109243dce5d30bb069be23437ca5aecd3594aef4857be`.
The debugger verifies that complete script before redirecting paths and resuming
its first executable line. It does not replace permissions, providers or MCP
handlers. All windows remain hidden; profile/session/log/crash paths are new
protected HOTR test paths. Default connectors/plugins and updates are disabled.

Initial connection-only preflight passed in
`work/hotr-tests/HOTR-07-18392-1788659680110124000/`. Its native renderer saw all
five MCP descriptors, a connected service and an intact Lamprey database.
A supported JavaScript pre-tool hook permits only the HOTR tool names. Its
editor tests allow HOTR get and reject read_file. This is an application hook,
not an OS sandbox or a claim that Lamprey fails closed on every hook-store fault.
Lamprey's skill allowedTools metadata alone is advisory and is not used as a
security boundary. No existing Lamprey file or active profile was changed.

The one actual model turn used claude-opus-5 through the already-configured
Anthropic account and a Lamprey-specific scoped contributor identity. It used
only synthetic data. The normal tool-approval IPC was exercised with two
scope-once approvals. The actual tool results were 404 for the absent demo record
and 403 for a forbidden namespace. The write/correction smoke failed because
Lamprey's Anthropic normalizer dropped search/create/revise schemas containing
`$ref`; discovery of five descriptors had not proved five model-visible tools.
Full failure: `work/hotr-evidence/HOTR-12-LAMPREY-SMOKE-70820-1788660115289174000/`;
application trace: `work/hotr-tests/HOTR-07-76676-1788660131670882700/`.
The app exited with code 0; the failed test was not relabeled as accepted.

## Local repair and checks

HOTR now uses the pinned Schemars generator's inline_subschemas option. It
preserves typed deserialization and service authorization. The added regression
checks nested record/source fields, enums, nullable revision and unknown-field
rejection. The app preflight now rejects structural schema incompatibilities
before model inference. The actual dispatch driver understands Lamprey's
`Error: {JSON}` tool-result envelope; model prose is not accepted as evidence.

Passed: focused schema regression; actual MCP separate-credential/reconnect/
denial and cancellation/limits/recovery tests; strict Clippy for both crates;
3,050-file native canary scan. MCP regression log:
`work/hotr-evidence/HOTR-LAMPREY-mcp-regression-1788660374314.txt`.

Final frozen-source, zero-model preflight PASS:
`work/hotr-evidence/HOTR-12-LAMPREY-PREFLIGHT-5440-1788660457263273700/`;
actual app fixture: `work/hotr-tests/HOTR-07-80868-1788660461091052000/`.
Its Windows job enforces 8 GiB combined memory and a 25% CPU hard cap on this
16-logical-processor host, equivalent to four logical processors.
Product SHA-256: `4a827f3853ea9afe95477dfed4135da217676c2c0e40ea7edd8d576fc2843f24`.
Runner SHA-256: `aefbce0ff89b99dddb7f7ba87e55cce38d1d612b8de5c71bca0aa412e60de8c8`.
These identify the uncommitted repair tree, not published main.

Earlier preflight failures remain retained: extended Windows path parsing,
the initial strict-mode directive pause, and run
`HOTR-12-LAMPREY-PREFLIGHT-68928-1788660332992370400`, whose app test passed but
whose manifest correctly failed because a helper edit changed source during
the check. The final frozen-source run supersedes that non-result.

## Required next actions

All twelve original M1 prompts are now used, including failures. The proposed
shared allowance of 72 additional synthetic prompts remains unapproved; see
`PLANNING/HOTR-COMPATIBILITY-PROMPT-BUDGET-PROPOSAL-2026-09-06.md`. The driver
still enforces twelve. Do not rerun live inference until the amendment is
actually approved and recorded. Read-only metadata/model-list calls and
connection-only checks spent no model prompts.

Automatic approval review twice rejected source formatting under the original
no-overwrite instruction, including a narrowed retry citing the bounded-write
approval. Formatting has not been applied. A review-only diff is prepared using
rustfmt --emit stdout with source hashes checked unchanged. Final formatting
scope is tests/support/lamprey.rs, xtask/src/main.rs and src/mcp.rs; the last file
was added to this scope by the subsequent schema repair. Explicit confirmation
of this exact scope is needed to resolve the automatic-review block.

After those inputs: format; rerun the actual installed-app save/recall/correction
and model-switch tests; complete restart, revocation and cancellation acceptance;
run the full source-bound gate; update ledgers; commit and push the passed
prompt to main; verify its exact remote SHA. Then resume HOTR-12A onward in
dependency order. No skipped live gate, semantic retrieval, personal-data pilot,
100k workload, four-hour soak or deployment acceptance is implied.

One canonical checkout, no linked worktrees, no unpublished commits; current
Lamprey changes are uncommitted. Approximately 4,155,751,885 bytes of generated
state are retained (3.87 GiB), with 341,009,088,512 bytes free at inventory.
No discretionary cleanup or existing-file deletion. The owner-selected
personal-file question is still unanswered; no personal material was imported.
# Superseding execution update — 2026-09-06 UTC

The owner resumed all application integrations and approved the necessary
permissions and shared compatibility prompt budget. The deferred checkpoint
below is historical. Actual installed Lamprey's six-turn acceptance now passes;
the common gate and final app rerun now also pass. The earlier idle-MCP timing
failure remains retained without a claimed root cause. Current evidence and status are in `DEVLOG.md`, `VERIFICATION.md`
and `evidence/HOTR-12-LAMPREY-clients.json`. No active-profile enrollment is
claimed. Preserve the earlier failures and checkpoints below.
