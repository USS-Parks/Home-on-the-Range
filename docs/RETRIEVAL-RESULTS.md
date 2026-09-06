# HOTR-17 retrieval quality results

Local gate PASS: `HOTR-17-86548-1788720435627956200`. The frozen 100-record corpus contains 96 development and 48 held-out queries; all 144 ran once in the first evaluation attempt. No ranking tuning or post-freeze label changes were needed. Corpus SHA-256: `6b1d878b950d11fe15fa8af58faaa1106a58f14f3aaa5d9010a96ab0975aa74f`.

| Held-out category | Queries | Keyword Recall@5 | Hybrid Recall@5 |
|---|---:|---:|---:|
| Exact identifiers |8|100%|100%|
| Paraphrases |24|0%|100%|
| Current temporal facts |6|0%|100%|
| Conflicting sources |4|0%|100%|

All 24 held-out paraphrases improved; none regressed. Development paraphrases were 48/48 for hybrid and 0/48 for keyword. Exact search and direct retrieval agreed on all 24 exact queries across both partitions. All 288 search responses and 24 direct lookups passed; 12 search calls returned the expected 403 across six restricted-namespace questions. There were zero unexpected errors, prohibited results, wrong revisions or source/text contract violations. Parent recomputation independently checked all query recalls, result IDs and lifecycle revisions.

No-answer behavior remains a limitation: hybrid returned candidates for all 12 no-answer questions; keyword returned none. The service exposes scores and does not assert answers or implement abstention. A threshold false-positive rate is therefore not applicable, and callers must not treat a candidate as proof that the corpus answers their question.

| Measurement | Development | Held-out |
|---|---:|---:|
| Hybrid end-to-end p95 |114.722 ms|150.649 ms|
| Keyword p95 |3.559 ms|4.443 ms|
| Search attempts per endpoint, including denials |96|48|

The warmed-model query timings include local embedding and retrieval. Query-vector cache hits are not observable; the corpus has no repeated exact query strings. The separate model-process start took 3,100 ms and first direct embedding 373 ms. This used a fresh owned model process; OS file-cache state was not reset. Indexing 92 current records across both namespaces took 13,026 ms. The index held 282,624 vector bytes; the encrypted vault was 876,544 bytes after service stop. A read-only audit found 92 index rows and integrity `ok`.

The model was installed Ollama 0.32.6 with the pinned 768-dimensional `nomic-embed-text:v1.5` manifest `0a109f422b47e3a30ba2b10eca18548e944e8a23073ee3f3e947efcf3c45e59f`. All model manifest/blob hashes remained unchanged after inference. The test-owned model process exited; the pre-existing Ollama process remained running.

The complete gate passed 50 ordinary product tests, six runner tests, three installed-model fixtures, both format/strict-Clippy checks, release build and a 7,531-file plaintext canary scan. Product SHA-256: `72e04907bdaa086e843d83cef9055776299850d0f9f4fb21b4ae02e35125a0ea`. Runner SHA-256: `ac4842b835e85b7eeffa68da1c1704c9c1a428f7b5153f444c0e1f84da86f274`. All 79 source/input hashes match. The earlier corpus rejections, repairs and review chain are retained in the authoring history and freeze.

This is a synthetic LLM-authored and independently LLM-reviewed benchmark, not a human-rated or personal-data evaluation. The keyword comparator is the existing literal-term intersection endpoint; it does not rewrite questions or represent a tuned BM25 baseline. The later stress, soak, cross-principal, installation and deployment gates remain open.

See the [method and reproduction command](RETRIEVAL-EVALUATION.md), [corpus and review provenance](../tests/fixtures/hotr17/AUTHORING.md), [frozen labels](../tests/fixtures/hotr17/corpus.json), [freeze](../tests/fixtures/hotr17/freeze.json), and [full sanitized evidence](evidence/HOTR-17.json). Fresh implementation review and main publication are tracked in [VERIFICATION](VERIFICATION.md).
