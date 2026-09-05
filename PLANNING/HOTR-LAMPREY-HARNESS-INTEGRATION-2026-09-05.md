# Lamprey Harness — first-class HOTR integration

Date: 2026-09-05
Status: explicitly requested by the user during approved STS; no Lamprey files changed.

## Target and evidence

Canonical source inspected read-only: C:\Users\17076\Documents\Claude\Lamprey Harness.
Observed source HEAD: dc67b05de22e79170d20baea0d0d2af5f12d965a.
Observed package version: 0.32.0. This does not establish the installed application's version.

Lamprey already has the needed MCP client architecture:
- electron/services/mcp-manager.ts defines McpServerConfig with id, name, transport, command/args/env, auth, and enabled.
- It supports stdio, SSE, and Streamable HTTP transports through the MCP SDK.
- electron/ipc/mcp.ts sanitizes a new server and requests user confirmation before launching a stdio command.
- addServerIfMissing refuses duplicate IDs and writes the MCP server configuration to Lamprey's user-data store.
- Plugin-owned server registrations are transient and rebuilt from plugin configuration; they are not persisted into mcp-servers.json.
- electron/services/chat-tool-dispatch.ts calls mcpManager.callTool through the existing cancellation and post-tool handling path.

Use that existing connection path. Do not add another database, parallel memory manager, bypass tool permissions, or modify the chat pipeline merely to integrate HOTR.

## Connection design

The HOTR bridge executes in stdio mode with a credential reference for a Lamprey-specific reader/contributor identity. The actual bridge is not built yet; no executable path/config entry should be installed until HOTR-10 passes.

MCP transport auth may be "none" for the local stdio link while the bridge authenticates separately to HOTR with its scoped credential. "none" must never be interpreted as an unauthenticated HOTR API. The master vault passphrase never appears in Lamprey settings, model context, or subprocess arguments.

First use an isolated test application profile if its behavior can be verified. Merely launching Lamprey may write its database, default connectors, and other user-data files. Do not launch it against the owner's active profile under the preservation rule. An existing-profile configuration change requires a later exact-path proposal and explicit authorization.

## Dedicated prompt HOTR-12-LAMPREY

Position: immediately after HOTR-12, before HOTR-12A (Hermes). This adds a first-class Lamprey gate without renumbering earlier prompts.

Depends on: HOTR-12 and a verified isolated-profile/write authorization.

Objective: make Lamprey Harness a real client of the same HOTR vault.

Work:
1. Verify installed/built application identity separately from source HEAD.
2. Prepare a HOTR connector through Lamprey's existing connector/plugin contract in an approved new test profile.
3. Preserve Lamprey's local-command confirmation and tool-permission flow.
4. Give Lamprey a distinct scoped credential.
5. Run actual source-bearing save/recall/correction/reconnect/revocation tests against the service shared with Codex or another verified client.
6. Switch the selected provider/model within Lamprey and verify that service access remains attached to the Lamprey application identity, subject to actual tool support.
7. Record credential/model/provider data flow; cloud-backed runs use synthetic context only.

Deliverables: versioned connector template or plugin manifest, isolated-profile procedure, actual application evidence, and a named Lamprey row in the compatibility matrix.

Acceptance:
- Lamprey recalls the same current authorized record/revision as the other client after correction and service restart.
- A forbidden namespace and revoked token are denied by HOTR.
- Tool cancellation/error outcomes use Lamprey's existing dispatch path.
- No existing Lamprey source, active profile, keys.json, lamprey.db, mcp-servers.json, plugin installation, or release artifact is changed without exact authorization.
- Headless SDK tests alone do not count as Lamprey application acceptance.
- Any necessary Lamprey source patch is prepared separately for review and remains outside the proposed HOTR-only write exception.
- Record live proof separately from installed metadata and repository source inspection.

No source code has been copied from Lamprey into HOTR. The existing Lamprey project and its current independent implementation/publication work remain untouched.
