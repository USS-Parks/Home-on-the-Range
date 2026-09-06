# Lamprey Harness integration

Lamprey 0.32.0 uses its existing stdio MCP connection to HOTR. No Lamprey source
patch or second memory database is required. HOTR generates inline request
schemas because this installed Lamprey release drops model tools containing
JSON Schema references, even when MCP discovery reports those tools.

The installed-app acceptance gate passed on 2026-09-06 UTC:
`HOTR-12-LAMPREY-77184-1788676590328991300`. Its actual application fixture is
`HOTR-07-79188-1788676737638771600`. Six synthetic prompts used the existing
Anthropic account, with a distinct scoped contributor credential and an
independent native reader. Actual tool results proved:

1. Create and recall a sourced blue proposal, revise it to green, and deny a
   forbidden namespace with HTTP 403.
2. Owner acceptance creates revision 3. After service restart, Lamprey's
   selected Opus 5 model retrieves the accepted current record and source.
3. Switching that conversation to Sonnet 5 retrieves the same accepted record.
4. Cancel through Lamprey's existing chat path, then successfully retrieve on
   the next turn.
5. Revoke Lamprey's credential: its next real tool call returns HTTP 401 while
   the independent reader still retrieves revision 3.

The stock executable and installed main script are hashed before startup.
Protected synthetic profiles, hidden windows, ordinary tool approvals and a
HOTR-only pre-tool hook are used. Neither the installed application source nor
the owner's active profile was changed by this acceptance run. This is actual
application acceptance, not yet active-profile enrollment or packaging proof.

HOTR executable SHA-256 for this run:
`0c4f87449f254bbe0cc46cdbfed1ecd0841f1e2a4c6d313ac4716e7a0fceeb16`.
The common gate and this app gate used identical source and native-library
hashes, with separately recorded executable hashes in
`evidence/HOTR-12-LAMPREY-source.json`. Both passed. Two successful six-prompt
runs consumed 12 of the shared 72-prompt compatibility allowance.
Common checks and exact publication are recorded separately in DEVLOG and
VERIFICATION. Earlier failed runs remain retained and are not relabeled.

Connection entry in Lamprey's existing MCP configuration (paths are examples):

```json
{
  "id": "hotr",
  "name": "Home on the Range",
  "transport": "stdio",
  "command": "C:\\approved\\hotr.exe",
  "args": ["mcp", "--credential", "C:\\approved\\lamprey.credential"],
  "auth": "none",
  "enabled": true
}
```

Here `auth: none` describes local stdio transport. The HOTR bridge separately
authenticates every service operation using the DPAPI-protected credential.
The master vault key never belongs in this configuration or model context.
Add only this entry through the native connector flow; preserve other entries.

Build native prerequisites once with `.cargo/prepare-native.ps1`, then run
`pwsh -NoProfile -File .cargo/verify-installed-clients.ps1 -Mode lamprey-acceptance`.
The runner re-enters the prepared MSVC environment and enforces the shared
72-prompt allowance, per-app ceilings, resource limits and source snapshot.
Rebuild native prerequisites when their inputs or toolchain change. Do not
repeat cloud acceptance solely for documentation edits.
