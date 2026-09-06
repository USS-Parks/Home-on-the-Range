# Local prototype quickstart

HOTR runs as one Windows process owning one encrypted vault. Each client launches
its own small MCP bridge with a different scoped credential. Starting the service
does not unlock it. This is the development prototype; packaging and deployment
acceptance are later PSPR gates.

## Prepare a new local location

Build the pinned executable using [NATIVE-BUILD.md](NATIVE-BUILD.md). It is
`work/hotr-build/target/release/hotr.exe`; `hotr native-info` reports the linked
SQLCipher, SQLite and OpenSSL versions. The examples below assume that executable
is on the terminal's PATH.

Choose an existing local parent and an **absent** vault directory. In the examples,
`C:\approved` is a placeholder for that owner-selected parent, not an installed
location. Existing destinations, network paths and reparse-point ancestors are
refused. The automated acceptance runs use only new synthetic project fixtures.

```powershell
hotr create C:\approved\vault
hotr serve C:\approved\vault --port 47821
```

Create prompts twice for a 16–1024-byte passphrase without echo. Keep it separately
from the vault, for example in your password manager. There is no password reset.
Leave `serve` running. From a second terminal under the same Windows account:

```powershell
hotr status C:\approved\vault
hotr unlock C:\approved\vault
hotr issue C:\approved\vault --credential C:\approved\codex.credential --label codex --role contributor --namespace demo
hotr issue C:\approved\vault --credential C:\approved\claude.credential --label claude --role reader --namespace demo
```

Unlock prompts locally. Credential files are created exclusively and protected
with the owner's Windows identity and DPAPI. Keep their paths out of Git. There
is no implicit access to other namespaces; repeat `--namespace` for each intended
grant. The current credential records the chosen service port, so use that port
again after restart.

## Connect the applications

Use the [Codex and Claude connection fragments](INSTALLED-CLIENTS.md) in a new
isolated profile or an explicitly approved existing configuration. Set the real
absolute executable and credential paths. Each application starts:

```text
hotr mcp --credential C:\approved\APPLICATION.credential
```

The bridge offers `hotr_health`, `hotr_search`, `hotr_get`, `hotr_create` and
`hotr_revise`. Reader credentials cannot write. Neither client receives the vault
passphrase or owner operations. Retrieved text can enter the application's model
context, including a cloud provider when that is the application's selected
route. Test with synthetic facts before selecting personal material.

Ask the contributor application to create a sourced proposed fact in `demo`,
then ask the reader to retrieve its ID and report the returned revision and
source. Review the content before owner acceptance:

```powershell
hotr accept C:\approved\vault --namespace demo --id YOUR_RECORD_ID --expected-revision 1 --request-id YOUR_UNIQUE_ACCEPTANCE_ID
```

Use the actual current revision. Acceptance creates another revision. A stale
revision is rejected; contributors cannot change accepted content. The broader
owner correction, expiry and supersession workflow belongs to HOTR-14.

## Lock, revoke and recover

```powershell
hotr clients C:\approved\vault
hotr revoke C:\approved\vault CLIENT_ID_FROM_LIST
hotr backup C:\approved\vault C:\approved\new-snapshot
hotr lock C:\approved\vault
```

Revocation is permanent for that credential. Backup prompts for a separate
passphrase and confirmation and requires a new destination. Lock completes when
the key-holding process exits. Restart the same `serve` command and unlock again
to resume; startup and automatic unlock are not installed.

To recover a closed snapshot into another new location:

```powershell
hotr restore C:\approved\new-snapshot C:\approved\recovered-vault
hotr serve C:\approved\recovered-vault --port 47821
```

Restore prompts for the backup passphrase; the recovered vault uses that key.
Unlock it and issue fresh credentials. Restored old credentials are invalidated.
The active vault is never replaced automatically. See
[backup limits and verification](BACKUP-AND-RESTORE.md) before relying on a
snapshot. Retain the original and recovery artifacts until the owner explicitly
decides what to remove.

## Evidence and limits

[VERIFICATION.md](VERIFICATION.md) names the exact passing and pending gates.
Windows account isolation is tested separately from application token scopes;
tokens do not isolate mutually hostile programs running as the same owner.
Encryption covers HOTR's vault and snapshots, not third-party transcripts or OS
paging. Semantic retrieval, universal client support, sustained stress and
distribution readiness require their remaining acceptance gates.
