# Encrypted backup and recovery

HOTR-11 provides an owner-only snapshot command and restoration into a new local
directory. The source vault must already be unlocked. The CLI reads passphrases
from the terminal without echo; keys are never command-line arguments or SQL text.

```powershell
hotr backup C:\approved\vault C:\approved\snapshot-001
hotr restore C:\approved\snapshot-001 C:\approved\restored-vault-001
```

These are examples, not permission to write outside the owner's selected paths.
Both destinations must be absent and their local parents must exist. Existing
files/directories are refused. No active vault, profile, or service is switched
automatically. The new backup can use a different passphrase from the source.
The restored vault uses the backup passphrase.

## Snapshot contract

The single database worker serializes the snapshot with ordinary mutations.
It records schema, record/revision/receipt/client/grant counts and the largest
mutation audit sequence. SQLite's online backup API copies between two SQLCipher
connections; both sides are encrypted. The destination is keyed through the native
API. It is never a plaintext intermediate or a naive copy of a live WAL file.

SQLCipher 4.18.0 returns `cipher_page_size` as text. Size validation parses that
explicit value before multiplying by page count. Native copy steps are 128 pages.
The destination receives SQL, cipher, foreign-key and FTS integrity checks and
must match the source watermark. It is closed without WAL/journal sidecars before
the ciphertext is hashed. `backup.json` is written last with exclusive creation
and a file flush. The snapshot has no ordinary vault marker and cannot be served
as an active vault.

`backup.json` contains format/version, a random snapshot identifier, ciphertext
size/SHA-256 and numeric watermarks. It contains no record bodies, sources, tokens
or keys, but its counts are metadata. Both it and the encrypted database receive
the protected owner/SYSTEM ACL. The manifest is an untrusted corruption check;
SQLCipher authentication and database integrity checks still run on restore.

## Restore contract

Restore holds read handles denying writes/deletion while inspecting the closed
snapshot. It rejects sidecars, unsupported formats/schemas, bad size/hash, invalid
key, corrupt pages, foreign-key errors and watermark mismatch before creating its
destination. It uses the same supported encrypted backup API to populate that
new directory, verifies it, then permanently revokes every active client copied
from the snapshot. Format-1 snapshots from schema 5 through the current schema
are supported. Only the new copy is migrated to the current schema; original
counts and audit watermark must remain intact. The report distinguishes the
original watermark's schema from `restored_schema_version`. Integrity checks
run again after migration. The ordinary vault marker is written and flushed last.

Start the service against the new vault path and unlock using the backup
passphrase. Enroll each intended client again with a new credential/profile.
Every old token remains invalid, including one revoked after the snapshot was
taken. A recovered backup therefore cannot resurrect client access. The owner
must explicitly choose when applications should use the restored service.

The active source vault and original backup remain in place. Failed new staging
files remain for inspection and have no completed vault marker; subsequent
attempts must choose a new destination. There is no automatic deletion, retention
policy, overwrite, directory replacement or fallback to an older snapshot.

## Current bounds and recovery expectations

This prototype bounds a snapshot to 1 GiB and four seconds of copy/check/hash
work, with deadline checks between steps and during SQL integrity checks. This is
an upper size bound, not a promise that a 1 GiB vault finishes in four seconds.
The owner transport has a five-second deadline and the worker queue is bounded.
Ordinary writes wait behind the snapshot. An owner transport timeout can leave
an uncertain result: inspect the selected destination's completed manifest before
choosing a new path; never overwrite or automatically replay an uncertain command.

Disk or process failure may leave incomplete newly created files. Full disk,
power-loss, crash and long-duration fault campaigns remain later PSPR gates.
Historical backups retain historical encrypted context, and their passphrases
must remain available to the owner. This command does not establish an off-device
backup, a recovery-key escrow or protection from a hostile process running as the
same Windows account.

## Upstream basis

- [SQLCipher maintainer's updated online-backup guidance](https://discuss.zetetic.net/t/using-the-sqlite-online-backup-api/2631): SQLCipher 4.3.0 onward supports encrypted-to-encrypted backup; mixed plaintext/encrypted backup is excluded.
- [rusqlite backup module](https://docs.rs/rusqlite/0.40.2/rusqlite/backup/index.html) and [stepped backup API](https://docs.rs/rusqlite/0.40.2/rusqlite/backup/struct.Backup.html).
- [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/) for key handling and cipher integrity checks.

Verification is recorded in the HOTR-11 development/evidence ledger. A backup
fixture or SDK client does not constitute named-application integration proof.
