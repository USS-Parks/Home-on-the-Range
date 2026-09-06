# Application compatibility

Current owner instruction: defer remaining HOTR-12 integration work, further Lamprey/plugin work and everyday-profile enrollment; continue at HOTR-13. The inventory below is retained as evidence, not an active app queue. See [the resumption record](../PLANNING/HOTR-13-RESUMPTION-2026-09-06.md).

Snapshot: 2026-09-06 UTC. `VERIFIED LIVE` describes the named interface in a
protected synthetic test profile. It does not mean the owner's everyday profile
has been enrolled. The common gate, exact published commit and hosted result are
recorded separately in VERIFICATION. Existing app settings remain preserved.

| Surface | Status | Current evidence / remaining work |
|---|---|---|
| Codex CLI 0.153.4 | VERIFIED LIVE | Scoped save/current recall, correction, restart, revocation, backup restore and reenrollment; HOTR-12 |
| Claude Code 2.1.220 | VERIFIED LIVE | Independent client in HOTR-12's shared-memory sequence |
| Lamprey 0.32.0 desktop | VERIFIED LIVE | Six-turn acceptance, model switch and cancellation/recovery; published `344b7a0`, exact Windows CI PASS |
| Hermes Agent 0.21.0 CLI | VERIFIED LIVE | Final three-turn native MCP save/correct/restart/search/revoke and common gate PASS; exact publication/hosted closeout tracked in VERIFICATION |
| Qwen Desktop 1.0.3 | DETECTED | Installed code exposes native MCP configuration and tool-call IPC; live acceptance deferred |
| Qwen Code | DOCUMENTED | Not detected on PATH; assess supported installation/local-provider route separately |
| Local Qwen 3:4b | DETECTED | Existing Ollama model listed; inference/tool cycle not yet verified |
| OpenCode 1.18.22 | DETECTED | Installed V1 release; use its V1 MCP schema |
| VS Code 1.127.0 | DETECTED | Native MCP surface requires its own actual-app proof |
| Continue 2.0.0 | DETECTED | Installed VS Code extension; separate extension acceptance required |
| Cline | DETECTED | 4.0.11 and 4.0.6 directories present; determine active extension version |
| Cursor 3.17.8 | DETECTED | Separate actual-editor proof required |
| Ollama | DETECTED | Existing service returned model inventory; inference acceptance remains open |
| Gemma 4:26b | DETECTED | Existing model listed; validate resources and retrieval/inference route |
| Local DeepSeek | DOCUMENTED | No DeepSeek model in the inspected Ollama listing; do not download weights implicitly |
| DeepSeek provider | DOCUMENTED | Existing authenticated route still to be checked; separate from local models and web chat |
| Unsloth Studio 0.1.801-beta | DETECTED | Installation metadata only; verify the actual inference runtime and endpoint |
| Unsloth training | DOCUMENTED | Dataset export/provenance is separate from retrieval; no training job requested |
| OpenRouter | DOCUMENTED | Verify an existing selected model/provider through a bounded local client |
| Gemini SDK / CLI | DOCUMENTED | Verify an authenticated local tool loop; not proof of AI Studio browser access |
| Google AI Studio browser/app | DETECTED | App registration observed; native/browser capability assessment remains open |
| DeepMind | DOCUMENTED | Product-specific Gemini/Gemma rows; no generic connector claim |
| NVIDIA NOOA | DOCUMENTED | Typed client and actual framework containment proof remain open |
| Docker | BLOCKED | CLI 29.6.1 detected; the inspected Desktop Linux engine was stopped |
| Chatbox 1.20.3 | DETECTED | Installed native MCP schema found; actual-app proof remains open |
| OpenWork 0.14.0 | DETECTED | Native MCP code and supported test-profile path found; actual-app proof remains open |
| Grok Bot 0.43.0 | DETECTED | Installed application identified; native connection/acceptance remains open |
| Linux/container service | DOCUMENTED | Windows credential/IPC code is not portable proof; later OS gate required |
| macOS service | DOCUMENTED | Native macOS environment and keychain/IPC/client proof required |

Everyday-profile enrollment is deferred for all rows. Future installation requires a
real owner-created vault and distinct application credentials, preserving
existing configurations in protected backups and adding only HOTR entries.
The synthetic test vaults and their public test passphrases are not production
vaults and must not be promoted into everyday use.

The current vault performs exact and keyword retrieval. Semantic retrieval is
a later approved milestone. An app may keep its own copy of retrieved context;
HOTR's database encryption does not encrypt that app's separate history.

See the [approved compatibility roster](../PLANNING/HOTR-STS-APPROVAL-AND-COMPATIBILITY-2026-09-05.md),
[verification ledger](VERIFICATION.md), [Lamprey workflow](LAMPREY-INTEGRATION.md),
and [Hermes workflow](HERMES-INTEGRATION.md).
