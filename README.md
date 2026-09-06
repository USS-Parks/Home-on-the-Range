# Home on the Range

A Windows-first encrypted context vault being built for shared access by local AI applications through one service, HTTP, and MCP.

**Status: full PSPR execution has reached HOTR-15, local encrypted semantic indexing, with its full local gate and independent review passed. Exact publication/hosted results are tracked in VERIFICATION. The owner deferred remaining HOTR-12 app integrations and further Lamprey/plugin work. Codex CLI, Claude Code, Lamprey and Hermes retain their actual isolated-profile proof. Everyday enrollment is deferred; semantic retrieval, larger stress/soak campaigns and packaging remain open. The product is not yet installed or deployment-approved.**

The earlier 10,000-record, 18,000-request prototype workload measured search/write p95 of 33/38 ms with no observed errors or lost acknowledged revisions. See the verification ledger for the tested versions and workload; this is not the later scale or soak campaign.

The first usable milestone is an encrypted vault with versioned context, scoped application credentials, exact/keyword retrieval, two demonstrated client integrations, and tested encrypted backup recovery. Later milestones add local semantic retrieval, management, fault testing, and packaging.

One Rust service owns the SQLCipher database. Each app connects through MCP or
HTTP with its own scoped credential. Retrieved records carry their source and
revision into the model's current context; this does not enlarge its native
context window.

The workflow is:

1. Create, start and unlock the vault through the owner controls.
2. Issue a separate reader or contributor credential for each app and namespace.
3. Add the HOTR connection through that app's supported configuration.
4. Ask the app to search or retrieve sourced records, or propose a new record or
   correction. Owner acceptance is a separate step.
5. Revoke an app independently when its access should end; keep encrypted backups.

[Application compatibility and current proof](docs/COMPATIBILITY.md) distinguishes
tested interfaces from detected installations and everyday-profile enrollment.

- [Canonical PSPR](PLANNING/HOME-ON-THE-RANGE-PSPR.md)
- [Local prototype quickstart](docs/QUICKSTART.md)
- [Owner-selected imports and preview/commit workflow](docs/IMPORTS.md)
- [Corrections, retention and permission changes](docs/CONTEXT-LIFECYCLE.md)
- [M1 demonstration and evidence boundaries](docs/M1-DEMO.md)
- [Development log](docs/DEVLOG.md)
- [Verification ledger](docs/VERIFICATION.md)
- [Working agreements](AGENTS.md)
- [Native Windows build and encryption proof](docs/NATIVE-BUILD.md)
- [Verification harness and frozen test targets](docs/VERIFICATION-HARNESS.md)
- [Owner commands and Windows key boundary](docs/OWNER-BOUNDARY.md)
- [Versioned records, provenance, and preservation](docs/RECORD-SCHEMA.md)
- [Atomic revisions and retry outcomes](docs/TRANSACTIONS.md)
- [Application credentials and permission matrix](docs/ACCESS-CONTROL.md)
- [Local API and protected client examples](docs/REST-API.md)
- [Encrypted retrieval and measured prototype workload](docs/RETRIEVAL.md)
- [MCP tools and project connection template](docs/MCP.md)
- [Encrypted backup and fresh-path recovery](docs/BACKUP-AND-RESTORE.md)
- [Actual Codex and Claude integration](docs/INSTALLED-CLIENTS.md)
- [Actual Lamprey integration](docs/LAMPREY-INTEGRATION.md)
- [Actual Hermes integration](docs/HERMES-INTEGRATION.md)
- [Application compatibility and enrollment status](docs/COMPATIBILITY.md)

Repository: https://github.com/USS-Parks/Home-on-the-Range

This project does not claim universal application compatibility, protection from a compromised Windows account, or that local storage prevents an authorized cloud client from sending retrieved text to its provider. Deployment claims will be limited to tested configurations.

Local semantic indexing setup and behavior: [Local indexing](docs/LOCAL-INDEXING.md). HOTR-15 passed its local gate and fresh review; publication is tracked in the verification ledger.
