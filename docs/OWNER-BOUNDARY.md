# Owner lifecycle and key boundary

HOTR-04 supplies the Windows owner CLI. HOTR-05–08 add versioned records,
serialized transactions, scoped credentials, and REST operations; see
ACCESS-CONTROL.md and REST-API.md. MCP and startup installation remain later
prompts in the approved sequential roster.

## Owner commands

After the native build, the executable is
`work/hotr-build/target/release/hotr.exe`. Use a new directory under an already
existing parent. The current session's allowed test locations are marked runs
under `work/hotr-tests`; a real user vault outside that scope needs its own exact
installation approval.

```powershell
hotr create PATH_TO_NEW_VAULT
hotr serve PATH_TO_VAULT --port 47821
# In a second owner terminal:
hotr status PATH_TO_VAULT
hotr unlock PATH_TO_VAULT
hotr lock PATH_TO_VAULT
```

Create and unlock read the passphrase from a Windows console with echo disabled.
Create asks for confirmation. The passphrase must be 16 to 1024 UTF-8 bytes and
is not accepted as an argument, environment variable, or configuration value.
There is no default password. Losing it currently means losing access to the
encrypted vault; later backup and rotation prompts do not invent a recovery key.

Creation requires an absent destination and uses exclusive OS file/directory
creation. Existing paths are refused. A failed creation retains its new files
for inspection and requires a fresh destination for a later attempt. Local disk
paths are required; traversal, UNC paths, and existing reparse ancestors are
rejected. The service validates protected ACLs and the format marker before
opening the database.

## Process and IPC boundary

Every start is locked. Status exposes lifecycle metadata only. The process
owns a loopback port and an owner-specific Windows named pipe. An occupied
port or duplicate pipe prevents startup. Since HOTR-08 the TCP listener serves
the scoped API, returning locked until the owner unlocks.

Vault directories have a protected ACL granting full control to their creating
user and SYSTEM, with child inheritance. The database and format marker also
have explicit protected ACLs. The administration pipe has explicit owner/SYSTEM
permissions, rejects remote clients, and is created with first-instance
protection. Clients inspect the live server's Windows SID before sending a key.
The service impersonates a pipe client only long enough to read its token SID,
reverts synchronously, and rejects a different identity. No impersonation spans
an asynchronous operation.

Unlock frames remain limited to 1025 bytes. Since HOTR-07, typed owner
administration frames permit at most 256 KiB of JSON plus the operation byte;
reply frames are at most 1 MiB. Frame read/reply deadlines remain five seconds.
Incorrect unlock attempts return one generic error and incur a 500 ms delay.
One request is processed while one pipe connection can wait. The service keeps
a successor instance available to avoid a disconnect/reconnect race. A hung
owner client can delay progress until those deadlines; later load gates measure
the complete application API.

Unlock holds the SQLCipher connection in one process. Passphrase buffers use
zeroizing storage. Lock closes the connection, acknowledges the request, and
ends the key-holding process, including its pipe and TCP handles. A missing
acknowledgement cannot prevent exit beyond the reply deadline. Restart requires
another unlock. No background service or auto-unlock credential is installed.

These controls separate Windows identities. They do not isolate hostile
programs sharing the owner's account, privileged administrators, or SYSTEM.
The implementation does not claim forensic erasure of OS paging, crash dumps,
or physical RAM. The distinct application capability layer is documented in
ACCESS-CONTROL.md.

## Reproduce the live gate

`cargo xtask verify --prompt HOTR-04` runs the normal native build, lint,
encryption, lifecycle, real ConPTY no-echo tests, and a required separate-account
test. The last test writes a new `challenge.json` in its marked synthetic run,
then waits at most 180 seconds for a probe. Missing evidence is a failure.

From a genuinely different authenticated Windows account, run:

```powershell
powershell.exe -NoProfile -File .cargo/owner-principal-probe.ps1 -Challenge PATH_TO_CHALLENGE
```

The probe checks its actual token identity, directly attempts read access to
the directory/database/marker, and tries to connect to the administration pipe.
Every check must fail specifically with Windows error 5. A missing path,
timeout, or same-account token is not accepted as evidence. It publishes a new
receipt atomically in that synthetic run without writing vault data. The owner
test checks that the live vault remained unlocked and unchanged, then locks it
and confirms process exit.

Since HOTR-07/08 the same probe also attempts to decrypt copied DPAPI ciphertext
under the second account. It then opens a synthetic loopback listener as that
account; the production client must connect but send zero application bytes
before rejecting its server identity. Neither a missing connection nor a timeout
counts as this proof. The probe closes only its own listener and client.

On this host, the existing Codex sandbox account is a separate authenticated
principal. Selecting the system `cmd.exe` runner makes that account usable;
the default user-installed PowerShell path had failed before command launch.
No account, password, security setting, application profile, or OS-wide ACL was
changed to run the proof. Hosted CI currently exercises the normal owner tests;
the local two-account result is recorded separately and is not inferred from CI.

The original hosted HOTR-04 run failed the literal SDDL comparison. HOTR-04-R1
replaces string equality with structural owner SID, protected DACL, exact ACE
count, trustee, mask, and inheritance checks. Equivalent SID aliases and ACE
order are accepted; broader permissions are rejected. The final local proof is
paired with HOTR-05, and hosted repair acceptance is tracked in VERIFICATION.
