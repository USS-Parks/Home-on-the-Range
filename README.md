# Home on the Range

A proposed Windows-first context vault shared by local AI applications through a local service, MCP, and an HTTP API.

**Status: full STS executing. HOTR-01–05 passed locally, including encrypted records/history and a real second-account denial test. HOTR-03 passed hosted Windows CI. A hosted HOTR-04 ACL failure has a locally verified HOTR-04-R1 repair; its exact-commit hosted check must pass before HOTR-06. Application integrations remain pending.**

The first usable milestone is an encrypted vault with versioned context, scoped application credentials, exact/keyword retrieval, two demonstrated client integrations, and tested encrypted backup recovery. Later milestones add local semantic retrieval, management, fault testing, and packaging.

- [Canonical PSPR](PLANNING/HOME-ON-THE-RANGE-PSPR.md)
- [Development log](docs/DEVLOG.md)
- [Verification ledger](docs/VERIFICATION.md)
- [Working agreements](AGENTS.md)
- [Native Windows build and encryption proof](docs/NATIVE-BUILD.md)
- [Verification harness and frozen test targets](docs/VERIFICATION-HARNESS.md)
- [Owner commands and Windows key boundary](docs/OWNER-BOUNDARY.md)
- [Versioned records, provenance, and preservation](docs/RECORD-SCHEMA.md)

Repository: https://github.com/USS-Parks/Home-on-the-Range

This project does not claim universal application compatibility, protection from a compromised Windows account, or that local storage prevents an authorized cloud client from sending retrieved text to its provider. Deployment claims will be limited to tested configurations.
