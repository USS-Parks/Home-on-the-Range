# Owner-selected imports

HOTR imports individually selected UTF-8 `.txt`, `.md` and `.json` files into an
unlocked vault. The owner CLI reads those files; the service receives a bounded
batch through its existing owner-only named pipe. Applications have no import
HTTP route or MCP tool.

Create/start/unlock the vault using the [quickstart](QUICKSTART.md). Preview a
selection from one local directory:

```powershell
hotr import C:\Vaults\context --root C:\SelectedContext --file roadmap.md --file facts.json --namespace personal
```

The JSON response's `data` contains `outcome: "preview"`, `preview_digest`, and
each record's exact body, type, proposed state, source SHA-256, source reference,
ID, current revision and `insert`/`duplicate` action. Preview changes no records,
audit events or import receipts. It prints selected content to the owner's
terminal; it does not save an unencrypted preview file automatically.

Review that response, then repeat the same selection with its digest:

```powershell
hotr import C:\Vaults\context --root C:\SelectedContext --file roadmap.md --file facts.json --namespace personal --commit <preview_digest>
```

The CLI rereads the selected files. The service reparses and hashes the received
bytes, checks the digest against this vault and the current record revisions,
and commits the complete batch in one transaction. A changed file or stale
preview is refused. Run a new preview to review the changed batch. A successful
receipt reports `inserted`, `duplicates` and record IDs/revisions. Applications
with a matching namespace grant can immediately retrieve the proposed records.
Owner acceptance remains a separate [owner operation](ACCESS-CONTROL.md).

Text and Markdown each produce one note. Links and commands remain text. JSON
uses this exact shape; unknown fields are rejected:

```json
{
  "records": [
    {"kind": "fact", "body": "The selected project uses SQLCipher.", "tags": ["architecture"]},
    {"kind": "decision", "body": "Keep this record proposed until owner review."}
  ]
}
```

Kinds are `fact`, `preference`, `decision`, `procedure`, `roadmap` and `note`.
`tags` is optional. Input cannot set IDs, namespaces, acceptance, grants or
source references. The owner selects the namespace on the CLI. Provenance
contains the canonical file URI, exact file SHA-256 and zero-based record index;
it is stored encrypted with the revision and returned to authorized readers.

Limits per invocation are 16 files, 64 records and 128 KiB total raw input.
Each body is nonempty, NUL-free and at most 64 KiB; existing tag limits apply.
The serialized owner request must also fit 256 KiB, including JSON escaping.
There is no automatic chunking, directory scan, wildcard expansion, provider
upload or managed-memory edit. Original selected files stay in place.

Deduplication uses namespace, canonical source URI, exact file hash and record
index. Repeating an unchanged selection inserts nothing. A changed file or
different source path creates new proposals; it does not revise or remove older
records. Reimport never resets a later owner acceptance/correction or restores
a suppressed record. If another writer occupies an import ID with different
original content, the whole batch is refused as a conflict.

Commit receipts are durable and encrypted. After a timeout or lost response,
retry the same selection and digest to reconcile the original transaction,
including after service restart. A matching receipt describes that historical
commit; retrieve the record separately for its current revision. Changed source
bytes cannot reuse an old receipt. An independent vault rejects its digest.

Selected paths must be normal relative file paths under the specified local
fixed/removable-drive root. UNC, mapped network drives, device paths, alternate
data streams, traversal and reparse points—including junction ancestors—are
refused. Native handles pin the directory chain against rename/delete and deny
source-file writes while capturing the batch; the final opened handle must
resolve under the selected root. These controls prevent path escapes in this
workflow; they do not isolate hostile processes sharing the owner's Windows
identity. See [Microsoft's handle-path contract](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew)
and [Rust's Windows sharing options](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html).

Schema v6 adds a vault import identity and immutable batch receipts. The existing
single writer, revision/source/tag insertion, audit and FTS indexing are reused.
The pinned URL parser already present in the lockfile is used directly for
source URIs. No database encryption or native-library substitution is involved.

Verification uses synthetic source files and actual owner/CLI processes, with
checks for exact preview/record agreement, FTS, retries, stale previews, malformed
input, forced transaction rollback, cross-vault digests, ID collisions, input
bounds, concurrent writers, an actual Windows junction and original-file hashes.
Results and source/binary hashes are recorded in [VERIFICATION](VERIFICATION.md).
