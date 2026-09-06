# MCP connection contract

Run `hotr mcp --credential <absolute-profile-path>` as the client's stdio command.
The owner first issues a separate reader or contributor profile through the
protected owner CLI. The bridge reads that DPAPI profile; the command line and
application configuration contain its path, never a bearer token or vault key.
It forwards each call through the existing loopback client, which verifies the
actual connected server's Windows identity before decrypting its credential.
The vault service must already be running and unlocked.

| Tool | Arguments | Service operation |
|---|---|---|
| `hotr_health` | Empty object | Credential-scoped status |
| `hotr_search` | `Search`: page and literal query | Current authorized FTS |
| `hotr_get` | Namespace, ID, optional historical revision | Exact current/history read |
| `hotr_create` | `WriteRequest`, expected revision null | Propose a new record |
| `hotr_revise` | `WriteRequest`, positive expected revision | Revise permitted proposed state |

Schemas are generated from the same Rust/Serde types used by REST. The service
applies its field limits, role/namespace checks and accepted-record restrictions
on every call. Read results carry the stored source fields without inventing
provenance; the record contract currently allows an empty sources list. Supplying
sources is part of the documented writing workflow. Search uses its existing
complete-response byte and conservative token budgets.

Successful calls return the same JSON as structured content and a JSON text block
for older clients. Service refusals are MCP tool errors with stable HTTP status
and service error data. Malformed tool arguments and unknown tool names are
protocol errors. A canceled connection or missing reply does not prove a write
rolled back: retry the exact original arguments/idempotency key to reconcile.
The bridge performs no automatic replay and keeps no context cache. Revocation
therefore applies to the next call from an already-running bridge.

The tool list contains no owner, unlock, acceptance, credential issuance, shell,
filesystem, arbitrary SQL, or arbitrary destination operation. Stored text has no
authority over those controls. As elsewhere in HOTR, mutually hostile processes
running as the same Windows user are outside the isolation boundary. A host app
can send text it legitimately receives to its selected model provider.

## Protocol and bounds

The official Rust SDK `rmcp = 3.2.0` handles JSON-RPC messages, initialization,
current-protocol discovery, schemas, error envelopes and cancellation. HOTR wraps
the SDK codec/transport extension points to enforce resource bounds; it does not
implement a competing MCP parser. Enabled SDK features are `server` and
`transport-io`, with default features disabled. HTTP MCP, OAuth, sampling, tasks,
elicitation and provider transports are not enabled or advertised.

Actual process tests cover legacy initialization at 2024-11-05, 2025-03-26 and
2025-11-25, and the 2026-07-28 inline discovery/call lifecycle with required
per-request protocol/capability metadata. Missing current-protocol metadata is
rejected. The SDK performs negotiation rather than HOTR rewriting version strings.

- Incoming JSON-RPC line: at most 256 KiB; request IDs at most 128 display bytes.
- At most 16 active requests, with duplicate active IDs rejected. Admission lives
  in SDK request extensions until the handler ends, including canceled work.
- At most 16 pending output sends, 1 MiB encoded output, five-second output timeout.
- Fifteen-second initial handshake/discovery timeout; service calls retain the
  existing ten-second deadline and can be canceled earlier.
- Malformed/oversized framing and capacity failures close the bridge. Reconnect
  using the same scoped profile; reconcile any uncertain writes before new ones.

Stdout is reserved for protocol traffic. Diagnostics are generic stderr messages;
no SDK tracing subscriber is installed because verbose upstream traces can include
message contents. A dead transport may terminate before a complete reply is read.
On termination the bridge explicitly exits so a Windows blocking stdin read cannot
keep it alive after cancellation or an initialization timeout.

## Project-local template and evidence

[`examples/mcp/stdio.json`](../examples/mcp/stdio.json) is a common MCP-server
configuration fragment. Replace both absolute paths in a newly selected project
configuration. Client-specific schemas/profile isolation and actual named-app
acceptance are HOTR-12 and its compatibility prompts. This template is not applied
to existing settings. One client gets one credential and explicit namespace grants.

The native tests start four actual bridge processes with two credentials against
one actual SQLCipher service. They verify save/recall/revision/replay, independent
roles, accepted-state protection, cross-namespace denial, revocation while another
bridge stays authorized, and reconnect. Every stdout line is parsed and checked
as protocol, with plaintext records retained only in the test process's memory.
The cancellation fixture uses a separate same-owner delayed HTTP listener to
observe socket closure, then tests ping recovery, malformed/oversized frames,
duplicate IDs, admission overflow and initialization timeout. That delayed peer
is not authorization or named-application proof. Raw tokens are scanned against
managed synthetic fixture files. The complete prompt gate repeats these tests
with native/HTTP/owner checks and the actual second authenticated Windows account.

## Upstream provenance

Crate checksum: `42b6914fac0be956fe704a38239c3f44a9f841d1b06a5713d2f638065593f5b5`.
Crate source revision: `51ccb42993d6eb5075399672ce7a0c21a0e55eea`.
Published package license metadata says Apache-2.0; the pinned upstream
[license text](third-party/rmcp-3.2.0-LICENSE.txt) explains the Apache/MIT transition
and documentation terms. Its SHA-256 is
`0382b0057770ca05e9c350a50aa3b1c1fea84da0bc81d723bf00b9aa841be58a`.
No SDK source was copied or modified. Preserve the upstream notices in packaging;
the full dependency/license review remains HOTR-31/33.

Primary references checked for this prompt:

- [Official SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Pinned source and license](https://github.com/modelcontextprotocol/rust-sdk/tree/51ccb42993d6eb5075399672ce7a0c21a0e55eea)
- [SDK transport extension points](https://docs.rs/rmcp/latest/rmcp/transport/index.html)
- [MCP 2026-07-28 specification changes](https://blog.modelcontextprotocol.io/posts/2026-07-28/)
- [Upstream security advisories](https://github.com/modelcontextprotocol/rust-sdk/security/advisories)

The reviewed upstream advisories concern HTTP/OAuth/redirect features omitted
from this stdio bridge. This scoped review does not replace the full locked-tree
dependency gate.
