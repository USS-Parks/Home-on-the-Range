# Installed application acceptance

HOTR-12's full local gate passed with Codex CLI 0.153.4 and Claude Code 2.1.220.
The eight actual application turns proved sourced create/recall, an owner-directed
correction and acceptance, restart, revocation, encrypted restore and reenrollment.
The milestone consumed eleven user prompts including earlier failed attempts.
See [actual tool-result evidence](evidence/HOTR-12-applications.json) and the
[source-bound gate](evidence/HOTR-12-clients.json). This is CLI acceptance on the
tested Windows host; the desktop UIs and other named clients need their own gates.

## Connection contract

Each application launches `hotr mcp --credential <absolute-credential-path>` as
its own stdio process. The bridge contacts the existing owner service. Give each
application a different credential and only its intended namespaces/role. A model
change inside an application does not change that service identity.

Issue a credential using the owner CLI, with a new selected output path:

```powershell
hotr issue C:\approved\vault --credential C:\approved\codex.credential --label codex --role contributor --namespace demo
hotr issue C:\approved\vault --credential C:\approved\claude.credential --label claude --role reader --namespace demo
```

These example paths are placeholders. Installation/configuration of an existing
application profile is a separate owner-selected action; do not replace an
existing file with these examples.

For Codex, add an MCP fragment to an approved configuration:

```toml
[mcp_servers.hotr]
command = 'C:\approved\hotr.exe'
args = ['mcp', '--credential', 'C:\approved\codex.credential']
required = true
startup_timeout_sec = 15
tool_timeout_sec = 15
```

For Claude Code, the corresponding MCP configuration is:

```json
{
  "mcpServers": {
    "hotr": {
      "command": "C:\\approved\\hotr.exe",
      "args": ["mcp", "--credential", "C:\\approved\\claude.credential"]
    }
  }
}
```

These are connection fragments, not replacements for application settings.
The master vault passphrase never belongs in either configuration.
For the tested Claude 2.1.220 automation, set `ENABLE_TOOL_SEARCH=false` and
`MCP_CONNECTION_NONBLOCKING=0` in the isolated client process. Otherwise a
configuration with no built-in tools can also remove the deferred tool-discovery
mechanism. The driver additionally checks the actual CLI's `mcp_status` before
submitting a user prompt, and applies 15-second MCP startup/tool deadlines.

## Acceptance driver

`integrations/clients/live_cli.py` invokes the actual installed executables; their
own MCP implementations launch the bridge. It does not implement another MCP
client or substitute an SDK harness for either application. The native ignored
test controls the synthetic service, owner operations, independent credentials,
restart, backup/restore and durable state assertions.

The eight successful model prompts covered Codex save and sourced recall;
Claude recall; an owner-directed correction proposed by Codex and accepted by the
owner; Codex current accepted recall; revoked Codex denial; Claude recall after
restart; old Claude denial after restore; and reenrolled Claude recall. Actual
MCP tool-result events must contain the required record/revision/state/source or
service denial. Final model prose does not establish acceptance.

The driver records each started attempt in a durable milestone budget ledger,
including failures, with a twelve-prompt ceiling. There is no automatic retry or
model substitution. Each call has a 180-second limit and bounded output. Only
HOTR tools are selected. Shell/file mutation tools, plugins, web search and agent
delegation are excluded from the fixture's intended workflow. Codex uses read-only
sandbox policy. Claude uses its explicit HOTR tool allowlist. Its credential
scrubbing hardening forces the effective permission mode to `default` even when
`dontAsk` is requested; this is retained in the application trace, and the
hardening is not disabled to avoid the warning.

Claude's control initialization/status messages do not invoke a model. The
driver sends the synthetic user message only after this same running CLI reports
one connected server with exactly the five expected HOTR tools. It then consumes
one durable prompt reservation. A live negative control with a missing credential
failed before any prompt or budget change. The readiness control path uses the
CLI wire protocol used by Anthropic's SDK; the application's own MCP implementation
still executes every tool. No SDK server or replacement MCP client is supplied.

Both profiles are new protected directories under `work/hotr-client-profiles/`.
Codex uses the owner's selected model and an isolated native `auth.json` holding
only the existing short-lived session credentials, with no copied refresh token.
This is Codex's native credential format, protected by the profile ACL; it is not
SQLCipher-encrypted. The existing auth file's hash is checked after the test.
Expired/invalid auth fails the test without a new login or provider fallback.
Claude's bare mode uses its existing Anthropic API credential in the process
environment. Provider credentials are redacted from retained evidence; subprocess
credential scrubbing is enabled for Claude's MCP children.

HOTR application credentials themselves remain DPAPI-protected. Test master keys
never enter either app's arguments, model context or configuration. Cloud-backed
models receive only the deliberately synthetic fact/source used by this gate.
The vault's encryption does not encrypt a third-party application's own
transcripts or provider credential store.

The npm Codex CLI 0.144.5 rejected the already-selected `gpt-6-astra` model with
an explicit upgrade-required error. The existing desktop installation also
contains Codex CLI 0.153.4, which completed the sourced create/get. That binary is
used without upgrading the installation or substituting a model. Claude Code
is 2.1.220. Final evidence must report both executable hashes. The older installed
Codex schema marks external-token login as internal/unstable, so that route is
not used by this driver.

## Primary references

- [OpenAI MCP configuration](https://learn.chatgpt.com/docs/mcp).
- [Codex state/profile locations](https://learn.chatgpt.com/docs/config-file/config-advanced).
- [Claude Code configuration/data directories](https://code.claude.com/docs/en/claude-directory).
- [Claude Code authentication](https://code.claude.com/docs/en/team).
- [Claude MCP startup and tool availability](https://code.claude.com/docs/en/mcp).
- [Claude MCP environment controls](https://code.claude.com/docs/en/env-vars).
- [Anthropic SDK control protocol](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/query.py).

Raw redacted application streams remain in the marked synthetic run; published
evidence includes the actual tool results, versions, executable hashes and usage.
The final scanner checked 2,883 files across 21 passing native runs. The actual
second authenticated account was denied files, pipe and copied DPAPI data;
its false endpoint received zero application bytes. See the verification ledger
for exact local/hosted source identity and the remaining full PSPR gates.
