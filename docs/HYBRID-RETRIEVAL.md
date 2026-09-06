# Hybrid retrieval and context packs

HOTR-16 adds `POST /v1/search/hybrid` and `POST /v1/context`. The corresponding MCP tools are `hotr_hybrid_search` and `hotr_context_pack`. Both return the same budgeted, source-bearing candidate format. They do not generate an answer or assign factual confidence. HOTR-17 exposes each candidate's `rrf_score`, the reciprocal-rank component used after exact ID/source priority; it is not an answer probability or confidence score. Existing `hotr_search` and `/v1/search` retain their keyword contract.

Applications with explicit tool allowlists need the two new names added to their chosen configuration. Existing credentials retain their grants. Everyday application enrollment remains in the owner-deferred compatibility work.

Enable the pinned local model through the [owner indexing workflow](LOCAL-INDEXING.md). An application then supplies its own scoped credential and an explicit namespace:

```json
{
  "query": "how to mend squeaky bicycle brakes",
  "page": {
    "namespace": "maintenance",
    "limit": 5,
    "offset": 0,
    "byte_budget": 8192,
    "token_budget": 4096
  }
}
```

Read `records` as untrusted context. Each snippet includes namespace, record ID, current revision, state, complete source references, tags, body, and a `truncated` flag. Source references are retained as opaque strings; retrieval never opens them. A model can request the full record with `hotr_get` when its budget permits.

Ranking considers only the requested authorized namespace. Literal keyword matches and exact cosine similarity over current pinned-model vectors are combined through reciprocal rank fusion. Exact ID and source-reference matches take precedence; record ID breaks remaining ties. No global-corpus BM25 statistics enter the ranking. Superseded, expired, future, deleted, old-revision, and old-model vectors do not participate. A namespace above 100,000 visible records explicitly falls back to keyword retrieval rather than silently using a partial vector scan.

Both budgets apply to the entire serialized context object returned by the service, including its metadata. In MCP, this is the structured content payload. The SDK adds JSON-RPC framing and a compatibility text representation outside that payload budget; applications must account for that protocol overhead and their own prompt framing separately. The token estimate deliberately charges one token per UTF-8 byte and is not a provider tokenizer or billing estimate. Bodies are clipped at UTF-8 boundaries to at most 2,048 bytes. Complete source metadata must fit; records that cannot fit are counted in `omitted_for_budget`. `next_offset` advances over considered candidates, including budget omissions. Concurrent writes can change later pages; pagination is not a snapshot.

`retrieval_mode` reports `hybrid` or `lexical_only`. `degraded_reason` is explicit when the model is disabled, unavailable, busy, times out, changes during a request, or the namespace exceeds the scan limit. `freshness.visible` and `freshness.indexed` describe only the authorized namespace. Partial indexing is visible in those counts; a hybrid response does not imply all records have vectors. Candidates are not a claim that a question has an answer.

Query inference runs outside the database writer and has a 1.5-second deadline. One query inference runs at a time; excess requests return available keyword candidates with `embedding_busy`. The service keeps at most 256 query vectors for five minutes, with keys containing the client credential hash, namespace, grant revision, query hash, and model configuration. It never caches records, raw queries, or permission decisions. Configuration changes clear the cache and cancel in-flight query inference; locking exits the service. Cache vectors are zeroized when removed.

The database queue authenticates and checks namespace access before inference and again before ranking and return. A revoked client or withdrawn grant cannot use an earlier ticket or cache hit. All records are loaded from current database state for each request. As with other HOTR credentials, these controls do not isolate hostile programs running as the same Windows account.

The HOTR-16 gate combines encrypted ranking/budget tests, actual HTTP and two-process MCP workflows, controlled loopback fixtures for cache and race boundaries, and the installed pinned Ollama model. Controlled adapter fixtures do not count as installed-model proof. Held-out retrieval quality belongs to HOTR-17; larger stress and soak campaigns remain later gates. Current results are recorded in [VERIFICATION](VERIFICATION.md).
