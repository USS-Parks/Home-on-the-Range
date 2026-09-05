# Full STS approval and compatibility expansion

Date: 2026-09-05
Status: user-approved initiative and stack; preservation restriction remains active.

## Approval and authority

The user approved full STS in this session, selected SQLCipher through rusqlite, accepted the other proposed defaults, and required Hermes, Qwen, DeepSeek, Unsloth, OpenCode, OpenRouter, NVIDIA NOOA, Google AI Studio/DeepMind, Gemma, Docker, VS Code, and relevant installed applications.

This records approval of HOTR-01 through HOTR-36 and the explicitly requested compatibility work below. The prior "pending STS" markers are historical; this later addendum supersedes them. No existing file was overwritten to record the approval. The no-delete/no-overwrite addendum still applies. It explicitly requires a bounded write exception before builds, mutable database tests, ordinary file edits, or Git publication.

## Architecture remains small

One authoritative SQLCipher vault through rusqlite; one service; MCP and REST adapters. Shared memory does not enlarge a model's native context window. Retrieval supplies selected, source-bearing context within each client's budget. No additional agent orchestration platform is implied.

A model family, API provider, agent framework, editor, container runtime, and desktop application have different integration surfaces. Native app support and an API-based fallback are separate rows. A successful DeepSeek API call is not proof that DeepSeek's web chat can access localhost. A successful Qwen Code connection is not proof of Qwen Desktop compatibility. Gemini API support is not a browser-based AI Studio integration.

## Required compatibility matrix

Initial metadata inventory is read-only. "Detected" does not mean authenticated, operational, or integrated.

| Target | Category / proposed route | Preparation observation | Required live evidence |
|---|---|---|---|
| Codex CLI | MCP stdio bridge | Package 0.144.5 detected | Actual save/recall/current-revision/revocation; preserve existing config |
| Claude Code | MCP stdio bridge | Package 2.1.220 detected | Actual second-client flow |
| Hermes | MCP configuration plus selectable model provider | Launcher detected | Profile-scoped connection, independent credential, scoped recall |
| Qwen Code | MCP stdio | Not detected on PATH | Installation/config route verified before adding; actual supported release |
| Qwen Desktop | Discover documented app capability; explicit import/context-pack fallback if no connector | Installed-app metadata reports 1.0.3 | Native route tested, or clearly labeled manual fallback; not conflated with Qwen Code |
| Qwen model family | Local model through Ollama or a compatible host | Existing local Qwen model previously listed | Live local tool/context cycle with pinned model identity |
| DeepSeek | Local supported model or provider tool-calling client | Requested; no authenticated route verified | Provider/local runtime tested separately; web-chat limitations explicit |
| Unsloth Studio | Local inference endpoint/client adapter | Desktop metadata reports 0.1.801-beta | Actual endpoint/version and retrieve-then-infer workflow |
| Unsloth training | Owner-approved sourced dataset export only | No training run requested or started | Export manifest/provenance; no automatic training or claim that training is retrieval memory |
| OpenCode | Version-specific project MCP config | 1.18.22 detected | Installed V1 config tested; do not apply incompatible V2 schema |
| OpenRouter | Existing capable host or bounded client-side tool loop | Requested; credentials not inspected | Exact selected tool-capable model/provider route; no model substitutions |
| NVIDIA NOOA | Typed Python method/client adapter | Official NVIDIA-NeMo/labs-OO-Agents identified | Isolated framework example exercises limited REST client; no arbitrary generated code on primary filesystem |
| Google AI Studio / Gemini | Local SDK tool loop; separate browser product route | AI Studio app registration detected | SDK/CLI proof distinct from browser app support; cloud service never receives master vault credential |
| DeepMind | Organization/product umbrella | Exact usable surfaces are Gemini/Gemma unless another is specified | Product-specific rows, no invented generic "DeepMind connector" |
| Gemma | Local runtime/context injection; tool support assessed per selected model | Existing Gemma model previously listed | Actual runtime/version/model and bounded recall workflow |
| Docker | Containerized MCP/REST client or service, explicit auth/network boundary | Docker Desktop 4.80.0 detected | No database file shared concurrently across hosts; no Docker socket, host-home, or unrelated volume mount |
| VS Code | Built-in supported MCP surface or an installed extension | 1.127.0 detected | Actual chosen feature and version; credentials isolated |
| Continue | Extension-specific tool/context adapter | 2.0.0 Windows extension directory detected | Actual configured extension proof |
| Cline | MCP client | 4.0.11 and 4.0.6 extension directories detected | Determine active version without deleting older directory; prove live flow |
| Cursor | MCP client | 3.17.8 desktop metadata detected | Actual project-scoped connection |
| Ollama | Local inference/embedding API | Existing service; do not change its bind/global config | Explicit loopback-only adapter; real model test separate from volume stubs |
| Chatbox, OpenWork, Grok Bot, Lamprey | Additional relevant app inventory | Installation directory names detected only | Verify identity/version and supported connector before a capability claim |
| Other relevant apps | Reusable MCP/REST/context-pack route | Discover narrowly from installed-app metadata | Add individual rows; no whole-machine content scan |

Statuses must be one of DOCUMENTED, DETECTED, CONFIGURED, VERIFIED LIVE, BLOCKED, or UNSUPPORTED NATIVE (with an explicit alternative). No global "all supported" label unless every claimed row has matching live proof.

## Added prompt roster

Original HOTR-01–36 identifiers remain stable. Execute these compatibility prompts after HOTR-12 and before HOTR-13, sequentially. Their implementation is authorized by the user's explicit scope expansion, subject to the preservation restriction. Apply the canonical common gate, one focused passed-prompt commit, and credential/data-flow rules to each.

### HOTR-12A — Hermes

Depends on HOTR-12. Objective: connect the installed Hermes application with its own scoped credential. Work: inspect its supported isolated-profile config, prepare the MCP entry, test save/recall/correction/denial. Deliverables: adapter/template and versioned evidence. Acceptance: actual Hermes process succeeds on allowed namespaces and fails on forbidden ones without changing existing profiles.

### HOTR-12B — Qwen clients and model

Depends on HOTR-12A. Objective: supply honest separate routes for Qwen Code, Qwen Desktop, and a local Qwen model. Work: version-specific MCP integration, documented desktop capability assessment, local model retrieval test. Deliverables: three matrix rows and supported adapters. Acceptance: each claimed native route has live evidence; unavailable native routes remain explicitly blocked/unsupported with a reviewed context-pack alternative.

### HOTR-12C — OpenCode

Depends on HOTR-12B. Objective: integrate the installed OpenCode release. Work: use its actual V1 schema, new project/profile configuration, scoped bridge token, reconnect test. Deliverables: versioned template and live trace. Acceptance: no V2-only config assumptions; real save/recall and revoked-token denial.

### HOTR-12D — VS Code, Continue, Cline, and Cursor

Depends on HOTR-12C. Objective: connect each relevant editor surface without conflating them. Work: determine active versions, create separate test workspaces/profiles when supported, install no duplicate extensions, exercise independent client credentials. Deliverables: per-extension/editor evidence rows. Acceptance: each claimed surface independently recalls current sourced context and rejects forbidden namespaces; existing settings/extensions remain unchanged.

### HOTR-12E — Local models and Unsloth

Depends on HOTR-12D. Objective: support local Qwen/Gemma/DeepSeek models through actual available runtimes and Unsloth inference. Work: reuse modest existing models, verify endpoints, implement client-side retrieval or tool loop appropriate to capabilities. Deliverables: pinned model/runtime matrix and examples. Acceptance: observed private context traffic remains local; absent models remain blocked rather than automatically downloading large weights. Training exports are explicit separate actions.

### HOTR-12F — OpenRouter and DeepSeek provider routes

Depends on HOTR-12E. Objective: connect provider-backed model workflows through a local client. Work: implement bounded tool-call dispatch to HOTR, validate names/arguments, keep HOTR credentials local, and select only user-approved existing provider configurations. Deliverables: adapter and provider-specific evidence. Acceptance: actual selected provider/model completes synthetic recall; no cloud SDK is given a localhost URL expecting provider servers to reach it; no private context or unsolicited fallback/routing.

### HOTR-12G — Google AI Studio / Gemini

Depends on HOTR-12F. Objective: establish a Gemini SDK/CLI route and assess the actual AI Studio app separately. Work: use official tool-calling support through a local runner; inspect supported browser integration constraints. Deliverables: API/CLI evidence, browser-support row, source-bearing manual context-pack fallback if necessary. Acceptance: model retrieves a synthetic record and source via the intended local client; browser support is never inferred from the API success.

### HOTR-12H — NVIDIA NOOA

Depends on HOTR-12G. Objective: expose a small typed HOTR client to NOOA. Work: pin NOOA, use an isolated execution environment, provide only narrow search/get/write methods with a restricted token, and do not expose the vault key or host filesystem. Deliverables: example and containment description. Acceptance: actual framework flow succeeds within grants; forbidden requests fail at HOTR regardless of generated code. Research-framework AST checks are not counted as OS containment.

### HOTR-12I — Docker boundary

Depends on HOTR-12H. Objective: support a container client without opening the vault to a network or mounting it into every app. Work: prove an explicit authenticated connection using a local bridge or isolated container service; define host-versus-container loopback behavior. Deliverables: minimal compose/run template and boundary tests. Acceptance: no wildcard host exposure, no Docker socket or user-home mount, and one database owner. A changed bind architecture requires a documented security review.

### HOTR-12J — Remaining installed clients

Depends on HOTR-12I. Objective: assess and connect other relevant detected clients. Work: identify Chatbox/OpenWork/Grok Bot/Lamprey and any additional documented installations; use existing MCP/REST adapters where possible. Deliverables: individual compatibility records and only needed small adapters. Acceptance: actual supported integrations pass the same cross-client contract; unsupported native surfaces get explicit alternatives and do not become blanket compatibility claims.

### HOTR-12K — Cross-client consistency

Depends on HOTR-12J. Objective: prove all verified routes share one current truth. Work: circulate a synthetic sourced record through verified clients, correct/expire it, restart, revoke one identity, and test context budgets. Deliverables: compatibility evidence index. Acceptance: all claimed verified clients receive the same current authorized revision; differing application/provider caches and blocked rows are documented.

## Operating-system expansion

"Across all platforms" includes portable contracts and explicit Windows, Linux/container, and macOS support rows. Windows remains the first native implementation. The Unix adapter must use OS-appropriate IPC/credential handling rather than pretending DPAPI/named pipes are portable.

Add HOTR-35A after HOTR-35: build and test Linux/container operation, encrypted database behavior, Unix IPC/file permissions, and two relevant client processes. Add HOTR-35B after HOTR-35A: equivalent macOS keychain/IPC/build/client checks. HOTR-36 depends on these expanded support results when claiming those OSes. Hosted compilation is not native client proof; unavailable OS/client environments stay BLOCKED. No unsupported Windows-only component is labeled cross-platform.

## Evidence and provider limits

No existing personal transcript, API key, browser cookie, vault, or global settings file was read to produce this inventory. The current provider smoke budget remains the original 12 short synthetic prompts per milestone; exceeding it needs a specific budget amendment. Load/fault/soak runs remain local and synthetic. Public hosting, training jobs, model purchases, large downloads, and cloud context ingestion are not implicit in connector compatibility.

## Primary references

- Hermes MCP: https://hermes-agent.nousresearch.com/docs/reference/mcp-config-reference/
- Qwen Code: https://qwenlm.github.io/qwen-code-docs/en/users/features/mcp/
- OpenCode versioned docs: https://opencode.ai/v2/docs/mcp-servers (V2 differs from the installed V1)
- VS Code MCP: https://code.visualstudio.com/docs/agent-customization/mcp-servers
- NOOA: https://github.com/NVIDIA-NeMo/labs-OO-Agents
- Unsloth: https://unsloth.ai/docs/basics/inference-and-deployment
- DeepSeek: https://api-docs.deepseek.com/guides/tool_calls/
- OpenRouter: https://openrouter.ai/docs/agent-sdk/call-model/tools
- Gemini: https://ai.google.dev/gemini-api/docs/function-calling
