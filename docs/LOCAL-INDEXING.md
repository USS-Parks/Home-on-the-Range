# Local semantic indexing

Indexing is disabled in a new or restored vault. The owner enables it through
Windows owner IPC; application credentials cannot select an embedding endpoint.
HOTR uses only numeric IPv4 loopback and the pinned local model. Cloud fallback,
redirects, DNS names and proxy routing are absent from the inference adapter.

Prepare the [pinned model](EMBEDDING-MODEL.md) once in this project's cache:

~~~powershell
pwsh -NoProfile -File .cargo/prepare-embedding.ps1 -Install
~~~

Start a separate Ollama server from a dedicated PowerShell process. These
settings belong to that process, not user or machine environment settings:

~~~powershell
$env:OLLAMA_HOST = '127.0.0.1:47822'
$env:OLLAMA_MODELS = Join-Path (Get-Location) 'work/hotr-models'
$env:OLLAMA_NO_CLOUD = 'true'
$env:OLLAMA_DEBUG = 'false'
$env:OLLAMA_DEBUG_LOG_REQUESTS = 'false'
$env:OLLAMA_NOPRUNE = 'true'
$env:OLLAMA_NOHISTORY = 'true'
$env:OLLAMA_NUM_PARALLEL = '1'
$env:OLLAMA_MAX_LOADED_MODELS = '1'
$env:OLLAMA_MAX_QUEUE = '4'
$env:OLLAMA_LOAD_TIMEOUT = '30s'
ollama serve
~~~

With the vault service running and unlocked, inspect its generation, then enable
indexing on the chosen port. Replace VAULT with the existing vault directory.

~~~powershell
hotr embedding-status VAULT
hotr embedding-configure VAULT --port 47822 --expected-generation 0
hotr embedding-status VAULT
~~~

Use the returned generation for each later change. Omitting --port disables
indexing; configuring the same port again explicitly starts a new generation
and retries exhausted records. A stale expected generation is rejected. Lock
and successful reconfiguration cancel the current inference task before the
owner acknowledgement. They cannot undo a request the local model already
received. Restoring a backup disables inference until the owner enables it again.

The worker selects one visible current record at a time. A durable database
claim spends one of three attempts before inference; a crash therefore cannot
reset the retry budget. Failed attempts wait one then five seconds; interrupted
claims wait for their 65-second lease to expire. Three attempts exhaust automatic
retries for that record revision and generation. New revisions or an explicit
new generation are eligible again. There is no unbounded in-memory work queue.

Inference has a 60-second total deadline and 30-second per-request deadline.
The adapter checks the exact model name and digest before and after inference,
plus model dimensions and metadata. Documents are split at UTF-8 boundaries
into at most 33 chunks, each at most 2,048 bytes including the search_document:
prefix. Truncation is disabled. Each chunk vector is validated and normalized;
their normalized sum forms one record vector. This is the initial indexing
algorithm; semantic ranking and measured retrieval quality are HOTR-16 and
HOTR-17 gates. HOTR-15 adds no semantic search endpoint.

Vectors are 768 finite normalized f32 values stored as little-endian encrypted
SQLCipher BLOBs. Each carries the record revision, configuration generation and
model digest. Completion rechecks current revision, visibility and configuration
inside the database transaction. The current-vector view repeats these filters.
Corrections, expiry, tombstones and supersession cannot make an obsolete vector
current. Derived vectors from old revisions may remain encrypted until replaced;
older encrypted backups keep their original retained content.

Status reports visible, indexed, pending and exhausted counts plus a safe error
code and observed loopback peer. An unavailable or rejected model leaves record
writes and keyword search available. No raw model response or context body is
included in error reporting. A same-Windows-user process can impersonate a local
model server; model metadata is a consistency check, not hostile-process
attestation. The pinned installer verifies actual model bytes; the adapter
verifies the endpoint's reported identity on every embedding.

Run the complete bounded HOTR-15 gate from this checkout:

~~~powershell
pwsh -NoProfile -File .cargo/verify-installed-clients.ps1 -Mode embedding
~~~

It includes native tests, formatting, strict Clippy, actual installed Ollama
inference in a separate owned process, encrypted persistence/restart checks and
a final plaintext-canary scan. It does not reuse another project's model cache
or call a cloud provider. The live fixture currently requires installed Ollama
0.32.6 and a pre-existing Ollama identity; it refuses to create a user identity.
