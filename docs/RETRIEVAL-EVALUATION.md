# Retrieval evaluation

HOTR-17 measures keyword and hybrid retrieval against a synthetic reference corpus. It does not import personal material or establish quality on all future workloads. The corpus, relevance rationales, review, freeze and results are separate artifacts.

The [corpus](../tests/fixtures/hotr17/corpus.json) and [authoring history](../tests/fixtures/hotr17/AUTHORING.md) live with the test fixtures. The author creates varied records, related distractors, revisions, conflicting sources, inactive records and a private namespace. A separate reviewer examines the query labels before any model evaluation. These are LLM-authored and independently LLM-reviewed examples; no human-rating provenance is claimed. The candidate corpus contains 144 distinct queries: 96 development and 48 held-out. Positive record IDs are disjoint between partitions across all categories; independent review and freezing must precede evaluation.

The freeze records SHA-256 over the UTF-8 corpus with actual CRLF line endings normalized to LF. Both corpus and freeze JSON also enter the verifier's source manifest. Missing or mismatched freeze data must fail before an evaluation starts. Development queries run first; only development results may guide tuning. Held-out labels and thresholds are not changed in response to failures. Failed runs and their outcomes remain retained.

The evaluator creates a marked synthetic vault, writes records through the real service, applies corrections and visibility through owner controls, and indexes current records with the installed pinned Ollama model. Each query exercises the keyword and hybrid HTTP endpoints using a reader granted only the shared namespace. Exact-ID cases also check direct retrieval. Responses are checked for current revision, current text, complete source references, namespace access and prohibited IDs.

Acceptance retains the canonical thresholds:

- Exact-ID/current-revision correctness: 100%.
- Held-out authorized paraphrase Recall@5: at least 90%.
- Hybrid improvement: at least 10 percentage points over keyword retrieval, or non-regression if the keyword baseline already reaches 90%.
- Prohibited IDs/text and access leaks: zero.

Recall@5 measures the fraction of labeled relevant records found among the first five candidates, averaged over positive queries. No-answer and access-denial cases are reported separately rather than entering that denominator. The service returns ranked candidates with `rrf_score`; it does not claim an answer or implement a no-answer confidence threshold. No-answer candidate-return rates can therefore be reported, but a threshold false-positive rate is not claimed.

The lexical baseline is HOTR's existing literal-term intersection endpoint with stable authorized ordering. It does not rewrite natural questions into keywords or use a separately tuned BM25 retriever. The benchmark measures this shipped comparison, not a best-in-class lexical baseline.

Results include both improvements and regressions, all request outcomes, endpoint latency, indexing time, vector bytes and encrypted-vault size. Errors and timeouts count as attempted requests and retrieval misses where applicable. Warm-model query measurements are kept distinct from cold model setup. This is a retrieval-quality evaluation; it does not replace the later scale or four-hour soak gates.

After the corpus is reviewed and frozen, the bounded gate is:

```powershell
pwsh -NoProfile -File .cargo/verify-installed-clients.ps1 -Mode evaluation
```

The gate must pass the ordinary product and verifier checks, installed-model regression fixtures, the evaluation itself, and the synthetic plaintext canary scan. See [VERIFICATION](VERIFICATION.md) for actual results and the [canonical PSPR](../PLANNING/HOME-ON-THE-RANGE-PSPR.md) for the fixed acceptance criteria. The first full gate passed; see the [measured results](RETRIEVAL-RESULTS.md) and source-bound evidence. Fresh implementation review and publication remain separately tracked.
