# ADR 0001 — SQLCipher through rusqlite

Date: 2026-09-05
Decision: APPROVED BY USER.
Prompt: HOTR-01 — foundation review completed; publication pending preservation-scope clarification.

## Decision

Use SQLCipher through rusqlite for the authoritative context database. Build a small Rust service and reuse supported storage, protocol, crypto, and OS primitives. Keep existing agentmemory and model/application installations untouched. Do not adopt another memory product's complete stack merely because it overlaps the feature list.

The user explicitly selected this route after requesting a reuse review. This is a fit decision, not a claim that our unbuilt implementation is better or safer than existing products.

## Reuse review at pinned revisions

| Candidate | Revision | Verified preparation | Gap / decision |
|---|---|---|---|
| Perseus Vault | 01367a85117e248cd010753eaf32f3339dc0dbfc | README fetched at this SHA; license endpoint reports MIT; describes local Rust/SQLite, encryption, history, recall | No executable tests were run. Whole-file/index encryption and our Windows owner/per-client boundary are not established. Reuse design knowledge; no code copied. |
| Memory-Vault | cb19259894b547048f710bcdc17b097d2a00298e | README fetched at this SHA; license endpoint reports MIT; describes SQLite/vector/Ollama, hashed principals, namespace grants, audit and per-space encryption | Broader worker/gateway/team workflow than the first prototype. Windows IPC and complete at-rest coverage unverified. Reuse contract ideas; no code copied. |
| Existing agentmemory | Installed skill guidance plus previous listener metadata; executable revision unknown | Architecture/config guidance documents local hybrid retrieval, optional bearer auth, viewer | Live security, per-client grants, encrypted persistence, and backup correctness unverified. Preserve runtime and data; optional future selected import/export adapter. |
| Shared SDK depot | Existing SDK Fetch catalog and guidance read | Existing dependency storage is present; guidance favors official SDKs and credential-free smoke tests | Read-only reuse where feasible. No bootstrap, update, install, or execution of provider clients occurred. |

## Requirement disposition

| Must-have | Route | Evidence status |
|---|---|---|
| Whole-vault encryption, including derived indexes | SQLCipher file/page encryption | Upstream design documented; native canary proof still required by HOTR-02 |
| Shared concurrent clients | One DB-owning service, bounded writer queue | Planned; HOTR-06/08/25 live gates |
| Per-client permissions | Credential-derived identity and namespace policy | Planned; HOTR-07/23 gates |
| No master key given to models | Owner-only protected IPC and local passphrase entry | Planned; HOTR-04 live second-principal gate |
| MCP / API portability | Official SDK bridge plus versioned REST contract | Planned; HOTR-10/12 and compatibility additions |
| Local semantic retrieval | Loopback local embeddings, encrypted vectors, hybrid ranking | Planned; HOTR-15–17 |
| Recoverability | Encrypted consistent backup, fresh-path restore, credential reenrollment | Planned; HOTR-11/20/27 |
| Non-destructive development | Additive files until explicit bounded write exception | Current preservation addendum active; build/publication blocked |
| Broad ecosystem support | Per-application, per-provider, per-OS compatibility matrix | Metadata/documented routes only; none yet VERIFIED LIVE |

## Native build finding

Cached rusqlite 0.40.1 and libsqlite3-sys 0.38.1 are present. The cached libsqlite3-sys SQLCipher amalgamation identifies itself as 4.14.0. That cache is not accepted automatically: Zetetic's current release notes identify later fixes, including a defensive-mode/export fix, updated SQLite security fixes, and a Windows logging/memory-security crash fix.

HOTR-02 must resolve and pin a supported current native SQLCipher build (4.18.0 as listed during this review, subject to exact source verification) through rusqlite, inspect the actual linked native version, and run real encryption checks. If the current rusqlite bundled feature still embeds an older version, use its supported external-SQLCipher link path or a narrowly documented source-build override with provenance. Never silently use ordinary SQLite, an unverified prebuilt binary, or an older cache to pass the gate.

Visual Studio Build Tools 18 with the C++ component is installed. Git's Perl exists; native-build compatibility is untested. NASM and standalone native Perl were not found in the checked locations. That does not prove they are absent everywhere. Build prerequisites can only be exercised after a bounded write scope is authorized.

## Status and next step

HOTR-01's read-only decision evidence is complete and the selected architecture is ratified. No source implementation, mutable vault, build, dependency install, service restart, or application configuration change has occurred. The prompt cannot be marked published/fully closed until its focused commit can be made under an explicit exception for normal project/Git writes.

Next dependent prompt: HOTR-02. Do not skip the encryption build proof.

Sources: https://www.zetetic.net/sqlcipher/design/ ; https://www.zetetic.net/blog/ ; https://docs.rs/crate/rusqlite/latest .
