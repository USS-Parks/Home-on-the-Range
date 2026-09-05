# Atomic writes and retries

HOTR-06 moves the unlocked connection into one named database worker thread.
It accepts at most 256 waiting requests plus the current operation. Admission
uses a nonblocking bounded channel: a full queue returns `overloaded`. A request
has a ten-second deadline. Owner lock stops admission, cancels uncommitted work,
waits for the worker and connection to end, and then ends the owner process.

## Revisions and receipts

A write contains a validated record, an expected revision, and an idempotency
key of 1–128 ASCII identifier bytes. An absent expected revision creates a new
ID only; a positive expected revision updates that exact current version.
Stale writers receive `revision_conflict`. Every update appends one immutable
revision and advances the current pointer by one.

The trusted service supplies the principal separately from request JSON.
Application tokens and accepted-state policy are HOTR-07; the internal writer
API alone does not grant permission to a client. Unknown JSON fields, including
a forged principal, are rejected by the typed write contract.

An immediate SQLCipher transaction stores the revision, source references,
tags, current pointer, mutation audit event, and retry receipt together. The
receipt is keyed by principal and idempotency key, with SHA-256 of the serialized
typed request. Object field ordering is normalized by typed deserialization;
list ordering and string bytes remain significant. A retry returns its original
receipt even after later revisions. Reusing the same key for a changed request
returns `idempotency_conflict`. Another principal has an independent key space.
Receipts and mutation audit rows reject updates and deletion.

## Cancellation and uncertainty

The worker atomically arbitrates cancellation against beginning COMMIT:

| Outcome | Meaning | Client action |
|---|---|---|
| `committed` | A durable receipt identifies the resulting revision and audit sequence | Retain the receipt |
| `canceled` | This attempt did not begin commit; staged work rolls back | Retry intentionally if still needed |
| `unknown_to_client` | Commit began or the completed response was unavailable | Retry the identical request with the same principal/key |
| Typed conflict/rejection | The request was rejected without a new committed mutation | Correct the request; use a new key for changed intent |

Canceling a retry cannot undo an earlier committed attempt. A disconnect or
timeout after COMMIT begins never claims rollback. A commit error is conservatively
unknown and must be reconciled after recovery. This is process-failure recovery,
not proof against sudden hardware power loss.

## Verification

Actual encrypted tests cover concurrent native-thread submissions, stable
receipts across later revisions, independent principal key spaces, stale and
incompatible retries, full queue refusal, dropped/expired requests, and an
injected audit-insert failure rolling back the entire mutation. An owned child
is terminated before commit, after commit before reply, and after the client
persists its received acknowledgment with a separate `sync_all` journal. Replay
reconciles all three boundaries without duplicate or missing revisions. These
three focused cycles do not replace HOTR-26's later 100-cycle campaign.

The instrumentation exists only in the test binary; production has no crash
environment variables or fault hooks. The full HOTR-06 gate also repeats real
owner lifecycle/ConPTY and actual second-account access denial. All generated
fixtures are new marked directories under the approved project test root.
