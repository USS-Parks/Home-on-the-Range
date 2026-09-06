# Home on the Range

A Windows-first encrypted context vault being built for shared access by local AI applications through one service, HTTP, and MCP.

**Status: HOTR-18 passed the complete local owner-viewer gate, including 34 actual Chrome assertions and scans for temporary credentials. Independent review returned ship with no findings; publication is tracked in VERIFICATION. Work stops after HOTR-18 publication until the owner resumes. HOTR-17 retrieval quality and exact hosted Windows CI passed. Remaining HOTR-12 integrations and further Lamprey/plugin work are owner-deferred. Existing app proofs use isolated profiles; everyday enrollment, later stress/soak and packaging remain open. The product is not installed or deployment-approved.**

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
- [M2 retrieval and owner-viewer demonstration](docs/M2-DEMO.md)
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
- [Hybrid search and budgeted context packs](docs/HYBRID-RETRIEVAL.md)
- [Frozen retrieval-quality evaluation](docs/RETRIEVAL-EVALUATION.md)
- [Measured retrieval results](docs/RETRIEVAL-RESULTS.md)
- [Read-only owner viewer](docs/OWNER-VIEWER.md)
- [MCP tools and project connection template](docs/MCP.md)
- [Encrypted backup and fresh-path recovery](docs/BACKUP-AND-RESTORE.md)
- [Actual Codex and Claude integration](docs/INSTALLED-CLIENTS.md)
- [Actual Lamprey integration](docs/LAMPREY-INTEGRATION.md)
- [Actual Hermes integration](docs/HERMES-INTEGRATION.md)
- [Application compatibility and enrollment status](docs/COMPATIBILITY.md)

Repository: https://github.com/USS-Parks/Home-on-the-Range

This project does not claim universal application compatibility, protection from a compromised Windows account, or that local storage prevents an authorized cloud client from sending retrieved text to its provider. Deployment claims will be limited to tested configurations.

Local semantic indexing setup and behavior: [Local indexing](docs/LOCAL-INDEXING.md). HOTR-15 passed its local gate and fresh review; publication is tracked in the verification ledger.
