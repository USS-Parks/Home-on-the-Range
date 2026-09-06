# Corrections, retention and access

`hotr lifecycle <vault>` accepts one bounded owner JSON request on stdin. A
durable `idempotency_key` reconciles retries, including after restart. The change
and receipt commit atomically through the existing single writer.

```json
{"idempotency_key":"correct-1","action":{"operation":"correct","expected_revision":1,"record":{"namespace":"project","id":"decision","kind":"decision","body":"The corrected decision.","state":"accepted","sources":[{"reference":"owner-review:decision","label":"Owner correction"}],"tags":[]}}}
```

Correction requires an existing record at its expected revision and creates an
accepted revision. Contributor writes cannot revise accepted or suppressed
records. Stale revisions fail without merging. `hotr accept` remains available.

```powershell
hotr inspect C:\Vaults\context --namespace project --id decision --expected-revision 1
```

Inspection shows current content, retention, visibility, relations and a revision
conflict flag. It does not claim to detect semantic contradictions automatically.
Additional lifecycle actions use the same request envelope:

```json
{"idempotency_key":"retire-1","action":{"operation":"visibility","namespace":"project","id":"decision","expected_revision":2,"tombstoned":true,"valid_from_ms":null,"expires_at_ms":null}}
```

```json
{"idempotency_key":"replace-1","action":{"operation":"supersede","namespace":"project","old_id":"old-decision","old_revision":1,"replacement_id":"new-decision","replacement_revision":1}}
```

```json
{"idempotency_key":"grants-1","action":{"operation":"grants","client_id":"enrolled-client-id","expected_revision":0,"role":"reader","namespaces":["project"]}}
```

Visibility replaces all three policy fields and advances the record revision.
Times are Unix milliseconds. A start must precede an end. Future validity,
expiry, tombstones and supersession suppress ordinary get (including explicit
revisions), search, list and count, plus MCP get/search. History is preserved
through the explicit, namespace-authorized history endpoint and labeled
`historical`. Revoking scope denies history as well as current retrieval.

Supersession requires two currently visible records in the same namespace and
both expected revisions. It accepts the replacement, advances both revisions
and adds the relation atomically. Hidden replacements and cycles are refused.
Suppressing the replacement later does not automatically resurrect the old record.

Grants replace a client's role and namespace set using `grant_revision` from
`hotr clients`. Empty scope is permitted; revoked credentials cannot be revived.
Changes apply to existing bridges/connections because each operation rechecks
policy on the database queue. There is no response cache in this version.
Current revisions and SQL time filters enforce the next operation's view;
revision changes also supply the generation boundary for later indexing.

Tombstones do not physically erase history, original files or older backups.
An old backup retains older policy/content. Restore disables old credentials;
review the restored policy before enrolling new clients. HOTR cannot retract
text already returned to another application's conversation.

Format-1 backups from schema 5 through the current schema can be restored.
Restore verifies the original schema, ciphertext and watermark before creating
the new destination, revokes copied credentials, and migrates only that new copy.
Original backup bytes remain unchanged. Unsupported or future schemas are refused;
a failed copy or migration remains unmarked and cannot be opened as a normal vault.

Actual tests use two MCP bridge processes, independent credentials, the owner
CLI and HTTP clients. They exercise corrections, suppression across retrieval
paths, authorized history, stale revisions, accepted/suppressed writer denial,
live grant withdrawal and restart/retry behavior. See [VERIFICATION](VERIFICATION.md).
