# Application access

HOTR-07 adds two application roles and explicit namespace grants to the encrypted
vault. Every application gets a separate 256-bit token from Windows BCrypt's
system cryptographic generator. Only its SHA-256 hash is stored in SQLCipher.
The token is returned to the owner as user-scoped DPAPI ciphertext and saved in
a new credential file with a protected owner/SYSTEM ACL. Tokens and passphrases
are never command arguments, environment variables, or diagnostic output.

The owner issues credentials through the existing SID-checked named pipe after
unlock. `hotr issue <vault> --credential <new-file> --label <name> --role reader
--namespace <namespace>` enrolls a reader. Use `contributor` for permitted writes;
repeat `--namespace` for each grant, up to 32. There is no implicit shared/global
grant. The parent directory must already exist and existing destinations are
refused. If issuance succeeds but saving fails, inspect `hotr clients <vault>`
and revoke the unused client ID before reenrolling to a fresh destination.

| Operation | Reader | Contributor | Owner pipe |
|---|---|---|---|
| Status (own client ID, role, schema only) | Yes | Yes | Yes |
| Current or historical record in granted namespace | Yes | Yes | Acceptance input only |
| Read or write another namespace | Denied | Denied | Explicit administrative operation |
| Create/revise proposed record in granted namespace | Denied | Yes, with revision check | Use a scoped contributor |
| Create accepted record or revise an accepted record | Denied | Denied | Acceptance only |
| Issue/list/revoke clients; accept a revision | Denied | Denied | Yes, while unlocked |
| Unlock/lock | Denied | Denied | Yes |

`hotr accept <vault> --namespace <namespace> --id <id> --expected-revision <n>
--request-id <stable-key>` adds an accepted revision, retaining the original.
All app writes derive identity from the verified credential; a JSON `principal`
field is rejected. The worker resolves the credential and grant before reading a
retry receipt. An original successful request can retrieve its old receipt after
owner acceptance, but it cannot mutate the accepted record. Revoked clients
cannot retrieve even their old receipts.

`hotr revoke <vault> <client-id>` permanently revokes a credential. Reads, writes,
authorization checks, and revocation use the same serialized database queue.
After the revoke reply, the next operation on an already-open HTTP connection is
denied. Work already authorized and committed may have succeeded. Restart keeps
revocations. There is no authorization cache to invalidate. Enrollment identity,
role, and token hash are immutable; reenroll for a different role. Client listing
currently returns at most 50 entries; owner inventory pagination follows with
the later administrative audit work.

The credential file's DPAPI scope is the current Windows user, never the whole
machine. The supplied client also checks the server-side PID/SID of the actual
established TCP connection before decrypting or sending a token. This avoids a
different Windows account capturing tokens by occupying HOTR's port. It fails
closed when Windows cannot resolve the connection/process identity. It does not
trust a listener check made before connecting.

The owner account and OS are trusted. Other processes running as that same user,
administrators, debuggers, and a compromised authorized client are outside this
isolation boundary. DPAPI does not make a credential transferable to another
account or machine. Authorized returned context is plaintext in the receiving
app; its downstream cloud use and copies are that app's responsibility. Lock
finishes when the key-holding server exits; it cannot recall already-returned
context or erase the operating system's paging/hibernation/crash files.

Evidence is the actual executable's role matrix, persistent-connection revoke
test, encrypted fixture scan, real owner pipe test, and separate authenticated
Windows account probe. SDK/HTTP fixtures are not acceptance of any named LLM
application; those live integrations remain HOTR-12 and its compatibility roster.
