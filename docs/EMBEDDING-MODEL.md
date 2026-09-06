# HOTR local embedding model

HOTR-15 selects `nomic-embed-text:v1.5` for local semantic indexing. The
selection is Apache-2.0 licensed, produces 768-dimensional vectors, and has a
pinned model layer of 274,290,656 bytes. The complete provenance contract is
[`models/nomic-embed-text-v1.5.json`](../models/nomic-embed-text-v1.5.json).
The immutable OCI manifest SHA-256 is
`0a109f422b47e3a30ba2b10eca18548e944e8a23073ee3f3e947efcf3c45e59f`; its
verified descriptors pin every downloaded config and layer blob. The expected
model layer is SHA-256
`970aa74c0a90ef7482477cf803618e776e173c007bf957f635f1015bfcfef0e6`.

Install only when a HOTR-15 native-model gate specifically calls for it:

```powershell
& .\.cargo\prepare-embedding.ps1 -Install
```

The installer does not call `ollama pull`, start Ollama, alter a user profile,
or write to an existing Ollama cache. It writes only the project-owned
`work/hotr-models` cache, using Ollama's native `manifests/` and `blobs/`
layout. It requires at least 25 GiB free after the bounded operation, limits
all generated `work/` state to 20 GiB, limits the declared model payload to 1
GiB, hashes files while streaming, and never overwrites or deletes a file.
Existing files are reused only after exact size and SHA-256 verification;
unique staging files are retained for inspection on a failed transfer.

Downloads begin only at `https://registry.ollama.ai`, use no proxy or automatic
redirect, and reject private, loopback, link-local, multicast, or special IP
addresses. A registry-initiated blob redirect is limited to one HTTPS hop and
pins one proven blob host for the rest of the installation. The pinned manifest
is fetched without redirects. These restrictions ensure private HOTR content is
never an installer input or upload.

The model runtime is a separate concern from installation. The HOTR-15 native
fixture or an explicitly owner-operated operational setup starts an owned
Ollama process on its fixed numeric loopback endpoint with `OLLAMA_MODELS` set
only in that child process environment to the project cache. It does not set
user or machine environment variables, change global Ollama networking, or use
a cloud fallback. The embedding adapter accepts only the selected model and
exactly 768 finite values with a non-zero vector norm; a malformed response,
wrong model, or unavailable server fails closed while ordinary writes and
lexical search continue. Request bodies are not logged.

The embedding task prefixes are `search_document: ` for stored documents and
`search_query: ` for retrieval queries. They are model inputs, not MCP tool
names. HOTR-15 uses the document prefix for indexing; the query path is ready
for HOTR-16 hybrid retrieval, whose endpoints and tools remain separate work.

The full local gate `HOTR-15-73280-1788715935249086000` passed with installed
Ollama 0.32.6, real inference, encrypted persistence, restart, revision replacement
and restored-vault indexing disabled. All five model-cache files retained their
pinned hashes after inference. See [source-bound evidence](evidence/HOTR-15.json)
for exact binaries, commands and the fresh review. Hosted verification and later
retrieval-quality/stress gates are separate results.

Sources: [Ollama model page](https://ollama.com/library/nomic-embed-text:v1.5),
[Ollama embedding API](https://docs.ollama.com/api/embed), and
[upstream model card](https://huggingface.co/nomic-ai/nomic-embed-text-v1.5).
