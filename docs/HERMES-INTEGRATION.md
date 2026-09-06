# Hermes integration

HOTR uses the installed Hermes Agent CLI's native MCP client. The inspected
release is 0.21.0. Its final three-prompt live acceptance and common verification
passed. Exact publication/hosted status is in VERIFICATION. The owner's active Hermes profile has not yet
been enrolled.

Give Hermes its own HOTR reader or contributor credential, scoped to the
namespaces it needs. Add a `hotr` entry under `mcp_servers` in the intended
Hermes profile's `config.yaml`, preserving all other settings:

```yaml
mcp_servers:
  hotr:
    command: 'C:\approved\hotr.exe'
    args: ['mcp', '--credential', 'C:\approved\hermes.credential']
    trust: full
    connect_timeout: 15
    tool_timeout: 15
    tools:
      include: [hotr_health, hotr_search, hotr_get, hotr_create, hotr_revise]
      resources: false
      prompts: false
```

`trust: full` permits Hermes to invoke this deliberately configured HOTR tool
set. HOTR independently enforces the credential's role and namespaces. The
credential is protected by Windows DPAPI; neither a token nor the vault
passphrase belongs in the YAML or a model prompt. Processes running as the
same Windows user are not isolated from one another by this credential design.

Start and unlock the HOTR service through its owner controls. Run
`hermes mcp test hotr` to check native discovery. In Hermes, request a search
or get for an authorized namespace and ask it to retain the source and current
revision in its answer. A contributor can propose new records and corrections;
owner acceptance is a separate operation. A provider/model change does not
change the Hermes credential's grants.

The isolated acceptance uses `HERMES_HOME` to select a fresh protected profile.
It supplies only the `mcp-hotr` toolset, caps model output at 1,024 tokens and
tool iterations at eight, and disables background model features only in that
profile. It reads actual `role=tool` messages from that new profile's native
session database. Model prose and tool arguments do not count as proof.

This release exposes a native discovery/router layer: `tool_search`,
`tool_describe` and `tool_call`. The verifier validates its five-entry HOTR
catalog and each resolved call/result identity. Hermes's untrusted-result
wrapper is retained in the raw evidence and decoded only for assertions.
The final run made five, two and one HOTR operations across its three prompts;
metadata discovery calls are recorded separately.

Final accepted run: `HOTR-12A-79932-1788680065291303200`, actual fixture
`HOTR-07-50908-1788680074702627400`. Native tool results show revisions 1 and 2,
owner-accepted revision 3 after restart, search total 1 with the same source,
forbidden-scope 403 and revoked-credential 401. The search used 785 estimated
tokens within its 1,024-token context budget. An independent reader retained
access to revision 3. The selected provider/model was Anthropic
`claude-sonnet-5`; all three successful-run CLI processes exited normally.

The final common gate passed 24 product tests, six runner tests and the
4,083-file canary scan. Its source, native inputs and product executable match
the final Hermes app gate. See [source-bound summary](evidence/HOTR-12A-clients.json)
and [actual tool results](evidence/HOTR-12A-application.json). Earlier attempts
and the verification-monitor repair are retained in DEVLOG.

Run the zero-model probe with
`pwsh -NoProfile -File .cargo/verify-installed-clients.ps1 -Mode hermes-preflight`.
Run the three-prompt acceptance with `-Mode hermes-acceptance` after discovery
passes. The shared compatibility counter charges failures before inference.
Existing native libraries and the installed Hermes Python environment are
reused. No additional Python environment, personal-history import or model
download is required.

The test contract covers save/recall/correction and forbidden scope, owner
acceptance, service restart, current sourced get/search in a fresh CLI session,
revocation, and an independent reader. DEVLOG and VERIFICATION record the
actual outcome and publication state. CLI acceptance does not establish a
separate browser or desktop frontend's behavior.
