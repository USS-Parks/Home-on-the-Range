# M2: local retrieval and owner inspection

Status: HOTR-13–17 accepted; HOTR-18 passed its complete local gate, including actual browser acceptance. Fresh independent review returned ship with no findings; main publication and its exact hosted gate remain open. The owner requested standby after HOTR-18 publication. This is not a full STS, daily-use enrollment or deployment verdict.

The milestone combines selected-file imports with preview/commit, sourced current revisions and owner correction, local encrypted embeddings, bounded hybrid retrieval, a frozen retrieval-quality benchmark, and a read-only owner viewer. The source builds in the single canonical Windows checkout; no additional worktree or application profile is required for the synthetic demonstration.

1. Follow the [local quickstart](QUICKSTART.md) to create, serve and unlock a vault.
2. Import only deliberately selected files through [preview/commit](IMPORTS.md). Review original source references and deduplication results.
3. Issue separate application credentials and use the [REST/MCP context workflow](HYBRID-RETRIEVAL.md). The returned record IDs, current revisions and source references are evidence for the consuming model.
4. Use the [owner viewer](OWNER-VIEWER.md) to search, inspect retained/current records and historical revisions, compare an expected revision, and review clients, grants, indexing and backup observations. Changes stay on the protected owner CLI.
5. Make and verify an [encrypted backup](BACKUP-AND-RESTORE.md). A viewer receipt is not a new restore test.

The [HOTR-17 retrieval results](RETRIEVAL-RESULTS.md) passed on the frozen 144-query synthetic corpus at implementation [5524505](https://github.com/USS-Parks/Home-on-the-Range/commit/5524505c9128b4d794b151de764929e3084d7fa5), published with [main closeout af906f7](https://github.com/USS-Parks/Home-on-the-Range/commit/af906f7e06378f5988ef3b334f36cf88cc79d9cf). Held-out paraphrase Recall@5 was 24/24 versus 0/24 for the existing literal-term keyword baseline, with zero prohibited results or wrong revisions. No abstention claim is made: negative queries still receive scored candidates. These are synthetic local measurements, not a universal relevance guarantee.

The [HOTR-18 evidence](evidence/HOTR-18.json) binds the final 85 source/input hashes, both binaries, 52 ordinary product tests, six harness tests, three installed-model fixtures, 34 actual browser assertions and the 10,724-file plaintext scan. Its accepted implementation/main SHA will follow fresh review and publication. Later access auditing, rotation, daily use, large-scale faults, sustained load/soak, independent security review and installation/packaging remain in the approved roster. The owner has deferred the remaining HOTR-12 compatibility and further Lamprey/plugin work; existing isolated-profile app proofs do not mean every installed application is enrolled.
