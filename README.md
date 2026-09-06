# Home on the Range

A Windows-first encrypted context vault being built for shared access by local AI applications through one service, HTTP, and MCP.

**Status: full PSPR execution is active. HOTR-01–12 passed locally. Actual Codex CLI 0.153.4 and Claude Code 2.1.220 shared a sourced fact through correction, acceptance, restart, revocation, encrypted restore and reenrollment. The earlier 10,000-record, 18,000-request prototype workload measured search/write p95 of 33/38 ms with no observed errors or lost acknowledged revisions. HOTR-11's repair passed its exact clean hosted Windows build. Lamprey and the expanded compatibility roster, semantic retrieval, larger stress/soak campaigns, and packaging remain required. The product is not yet installed or deployment-approved.**

The first usable milestone is an encrypted vault with versioned context, scoped application credentials, exact/keyword retrieval, two demonstrated client integrations, and tested encrypted backup recovery. Later milestones add local semantic retrieval, management, fault testing, and packaging.

- [Canonical PSPR](PLANNING/HOME-ON-THE-RANGE-PSPR.md)
- [Local prototype quickstart](docs/QUICKSTART.md)
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

Repository: https://github.com/USS-Parks/Home-on-the-Range

This project does not claim universal application compatibility, protection from a compromised Windows account, or that local storage prevents an authorized cloud client from sending retrieved text to its provider. Deployment claims will be limited to tested configurations.
