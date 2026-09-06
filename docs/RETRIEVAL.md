# Encrypted keyword retrieval

HOTR-09 keeps FTS5's index and shadow tables inside the same SQLCipher vault.
Each successful revision transaction replaces that record's derived index entry
along with its record pointer, sources, tags, audit event, and retry receipt.
An injected audit failure also rolls back the index change. Old revisions remain
in encrypted history; they are absent from the current index.

The app must have an explicit grant for the request's namespace. Authorization
runs on the same database queue as the query and revocation. Search, list, count,
current get and history all enforce it. A denied namespace returns the same
generic forbidden result without counts or matching IDs.

| POST endpoint | Request |
|---|---|
| `/v1/search` | `page` object plus `query` |
| `/v1/records/list` | Page object |
| `/v1/records/count` | `namespace` |
| `/v1/records/history` | `page` object plus `id` |
| `/v1/records/get` | `namespace`, `id`, optional historical `revision` |

Example keyword request:

```json
{
  "page": {
    "namespace": "project/demo",
    "limit": 10,
    "offset": 0,
    "byte_budget": 65536,
    "token_budget": 32768
  },
  "query": "encrypted backup"
}
```

Pipe UTF-8 JSON to `hotr request --credential <credential-file> --method POST
--endpoint /v1/search`. The credential stays protected and the client verifies
the actual server's Windows identity before sending it. Sources are returned as
opaque references; search does not open them.

The baseline uses Unicode61 tokenization and literal intersection: each of up to
32 whitespace-separated query phrases must match. Queries are at most 512 UTF-8
bytes, with no NUL. Quotes are escaped for FTS; operators, column selectors,
wildcards and SQL-like input are treated as text rather than query instructions.
IDs, current bodies, tags, and source references are indexed. Exact ID and exact
source-reference matches get fixed ordering boosts, then IDs provide stable
ordering. This first lexical baseline does not rank using global-corpus BM25
statistics, which could make one namespace's results depend on hidden data.
Semantic and hybrid ranking remain later prompts.

Search/list results contain complete sourced revisions and an authorized `total`,
`next_offset`, `omitted_for_budget`, and explicit budget fields. Defaults: ten
records, offset zero, 64 KiB bytes, 32,768 estimated tokens. Limits: 1–50 records,
offset at most 100,000, byte budget 1,024–262,144, token budget 512–262,144.
The conservative token estimate counts one per serialized UTF-8 byte plus an
envelope reserve; it is not a provider tokenizer or a billing count. Both bounds
apply to the complete response, including metadata. Whole records that do not
fit are omitted and counted; increase budgets or use a smaller page to retrieve
them. No source or body is silently clipped. Pagination advances across examined
authorized candidates, including omissions. Pages reflect current data, so
concurrent writes can change offset positions; this is not a snapshot cursor.

Schema 5 adds a `record_visibility` envelope and a shared visible-record view.
Default current get/search/list/count exclude tombstoned, expired and explicitly
superseded identities. History and explicitly requested historical revisions
remain available to granted clients. HOTR-09 tests these filters by seeding only
its stopped synthetic vault. Owner operations for retirement, expiry, correction
and retention policy are HOTR-14; this prompt exposes no deletion operation and
does not erase a historical record. Future metadata changes use that existing
filter seam.

Database commands install a progress handler with their queued deadline and the
owner stop flag. Long SQLite work can be interrupted; the handler is cleared
before the next command. The native deadline test interrupts a long encrypted
connection query and then proves the same worker accepts its next command.
The diagnostic query exists only in the test build, with no HTTP/CLI SQL route.

## Prototype load gate

`cargo xtask verify --prompt HOTR-09` requires all native/HTTP/schema/owner gates,
the live separate-account probe, and the full 15-minute workload. The load test
is ignored by ordinary `cargo test` and explicitly required by this prompt's
runner; skipping it cannot produce a HOTR-09 passing manifest.

Four bounded seed clients create 10,000 records through the actual authenticated
API in ten namespaces. Bodies are 1,024/1,792/2,560/3,328 UTF-8 bytes, with opaque
sources and 25 keyword topics. Seed 47821 fixes the corpus and schedule. Eight
independent scoped clients then issue 18,000 scheduled operations over 900 seconds:
3,600 writes, 7,200 keyword searches, 3,600 current gets, and 3,600 counts. Each
five-operation group revises a record before querying revised data. The hot write
set contains 2,000 of the 10,000 live records; this is not a uniform random workload.

Latency starts at scheduled arrival and ends after the complete response, so
scheduling delay and failures are included. There is no unbounded producer queue:
only eight clients can be active. Unexpected failures or correctness violations
abort the campaign and retain a failed report. The first query after seeding is
reported separately and is explicitly not an OS-cache-cold measurement.

After the mixed workload, the server locks/exits. An independent encrypted read
checks all 10,000 expected current revisions, exactly 13,600 durable receipts,
10,000 stable record identities, SQLite integrity and FTS integrity. Storage and
logs are scanned for the synthetic body/key and actual generated tokens. Minute
progress is appended to a new synthetic-run JSONL file; numerical results are
created as a new JSON file. Nothing is written into an existing user vault.

Passing requires the frozen write/keyword p95 ≤500 ms, all scheduled requests,
the full 900 seconds, no lost acknowledged revisions, no duplicates, and zero
observed correctness/security violations. The harness retains its four-CPU,
8 GiB memory, 20 GiB project-work and 25 GiB minimum-free-space limits. Results
are recorded in the verification ledger when measured; these targets are not
claimed results.

## Measured prototype result

The final source-bound HOTR-09 run passed on this Windows x86-64 host at
2026-09-06 UTC (2026-09-05 local time). It completed 18,000 of 18,000 scheduled
requests over 900.017 seconds, with zero unexpected errors, correctness violations
or acknowledgment mismatches. All 10,000 final revisions and 13,600 retry receipts
reconciled after service exit; SQLite and FTS integrity passed.

| Operation | Samples | p50 ms | p95 ms | p99 ms | Maximum ms |
|---|---:|---:|---:|---:|---:|
| Keyword | 7,200 | 22.172 | 32.957 | 257.133 | 1,080.844 |
| Write | 3,600 | 20.993 | 38.458 | 326.709 | 1,194.625 |
| Current get/count | 7,200 | 17.632 | 26.045 | 33.125 | 384.736 |

Seeding took 158.809 seconds. The first query after seeding took 26.047 ms;
this is not a cold-cache result. The p95 acceptance target passed; the maximums
show occasional longer stalls. These measurements establish this prototype
workload only, not the later 100k-record or four-hour soak requirements.
See the [complete numerical result](evidence/HOTR-09-prototype-load.json) and
[source/binary gate manifest](evidence/HOTR-09-retrieval.json).
