# Versioned context records

HOTR-05 defines encrypted storage and typed record contracts. HOTR-06 adds the
atomic writer. HOTR-07/08 add centrally enforced capabilities and HTTP entry
points; MCP follows in HOTR-10. Accepting a `state` value in an internal Rust
type is not authorization to accept a fact.

## Schema and history

The encrypted database uses SQLite `user_version=4`. Version 0 is the HOTR-04
container with its `hotr_vault` format marker. Migration 1 adds namespaces,
record identities, and revisions. Migration 2 adds revision source references,
tags, and relations. Migration 3 adds mutation audit events and durable retry
receipts. Migration 4 adds hashed clients, permanent revocation, and exact
namespace grants. Needed migrations run in one immediate transaction on an
older vault. Existing version-1 identities and complete revision history survive
the later migrations. Unsupported future versions fail before writable open.

The identity is `(namespace, id)`. IDs and namespaces are bounded ASCII labels,
not paths; namespaces may have slash-separated segments. Empty segments, `.` and
`..` are rejected. A record points at its current revision through a deferred
foreign key. Revisions retain body, kind, proposed/accepted state, and creation
time. Updating or deleting revision rows is rejected by SQL triggers. Advancing
a current pointer must use the next revision; the write service will enforce
expected revisions and authorization before changing it.

Kinds are fact, preference, decision, procedure, roadmap, and note. Source
references are opaque strings accompanied by a label. They are returned with
the revision and never opened as files, URLs, commands, or executable content.
Tags and sources belong to a specific revision. Relations connect existing
records within one namespace and have one of five meanings: supports,
contradicts, depends_on, supersedes, or related. Self-relations, missing endpoints,
and endpoints in another namespace are rejected.

| Field | Enforced bound |
|---|---|
| Namespace and ID | 1–128 bytes each; validated ASCII label syntax |
| Revision | Positive 32-bit unsigned integer |
| Body | 1–65,536 UTF-8 bytes; no embedded NUL |
| Source references | At most 16; each reference 1–2,048 bytes, label 0–256 bytes |
| Tags | At most 32 distinct values; each 1–64 UTF-8 bytes |
| Creation time | Nonnegative signed 64-bit milliseconds since Unix epoch |

Rust contracts reject unknown fields, duplicate tags, invalid labels, oversized
values, and NUL bytes. SQL STRICT tables, CHECK constraints, ordinal bounds,
uniqueness, and foreign keys enforce the storage rules independently. Unicode
bodies, combining marks, emoji sequences, and tags survive JSON and encrypted
database round trips without normalization. Length limits count bytes, not
displayed characters.

## Preservation before migration

A plain read-only SQLite connection can still create WAL sidecars; see the
[SQLite WAL documentation](https://www.sqlite.org/wal.html). The version probe
therefore holds Windows read handles that deny writes and deletion. For a
closed database with no WAL or rollback journal, it uses a percent-encoded
immutable SQLite URI while the write-denying handle is held. It never applies
immutable mode to a database with a WAL, because committed state can be there.

With an existing WAL and shared-memory file, the probe opens the pinned native
implementation with `mode=ro&readonly_shm=1` and holds write-denying handles to
both sidecars. SQLite can reconstruct its WAL index privately without altering
the files. This path is verified against a real WAL left by terminating a
project-owned synthetic writer after commit. A future version committed only
in that WAL is detected, and all database/sidecar hashes remain unchanged.

A rollback journal, or a WAL with a missing/unreadable shared-memory file, is
refused without reconstruction or cleanup. Such an exceptional input requires
an explicit recovery flow in a new destination. It is not permission to erase a
journal or substitute the stale main database. Full crash/recovery campaigns
remain later prompts.

The tests also retain the older failed exclusive-lock experiment: Windows
SQLite rejects write-style exclusive locks on a read-only handle. That path
was replaced with the native read-only shared-memory option. Numeric native
error codes are available for diagnostics; keys and record content are not
included in error messages.

## Evidence

`cargo xtask verify --prompt HOTR-05` requires native build/lint/tests, the actual
encrypted schema fixture suite, canary scanning, and the live separate-account
owner test. SQL migration files participate in source hashing. The schema suite
checks migration/history preservation, exact current/history lookup, Unicode,
SQL and typed bounds, relation constraints, absence of an outbound connection
to a supplied source URL, closed future files, and a future version in a crashed
writer's WAL. Its subprocess helper is normally ignored and is invoked explicitly
by the parent preservation test with a guarded synthetic path.
