# HOTR-04-R1 — Hosted Windows ACL verification repair

Date: 2026-09-05. Status: implementation repair under the existing full STS scope.

## Objective and dependency

Repair the HOTR-04 hosted owner-ACL failure before advancing beyond HOTR-05.
The local HOTR-04 acceptance, including the real second-account proof, passed at
`1e049d27ec1f915dd54498fc44f6231ec934cee7`. Its separate Windows hosted run
33992714760 failed both owner lifecycle and console creation at the protected
ACL check. Native encryption passed. The complete failed workflow log and test
artifact are retained under `work/hotr-evidence`.

The initial verifier compared a serialized SDDL string to a hand-built string.
Windows can express trustees with aliases and reorder equivalent access entries.
The repair compares the actual owner SID, protected/non-null DACL, exact count
of two plain allow ACEs, full file-access masks, required inheritance flags, and
the exact owner/SYSTEM trustee set. Extra grants, duplicate/unexpected trustees,
wrong ownership, incomplete rights, inherited/unprotected ACLs, and null DACLs
remain rejected. No ACL on user data or any OS account is changed by this repair.

## Work and deliverables

Replace only the ACL inspection implementation, add alias/order and denial
contracts, rerun actual Windows owner and distinct-account tests, and retain the
new source/binary evidence plus the original failure. The requested permissions
on newly created vault objects remain the same.

## Acceptance

1. Structural permission contracts and warnings-denied native checks pass.
2. Actual create/no-echo/unlock/lock and real second authenticated account error-5
   denial pass on the repaired executable.
3. The exact published repair commit passes the hosted Windows workflow. A local
   pass does not satisfy this hosted repair gate.

Publication is a tightly scoped bundle with HOTR-05: the hosted failure arrived
after HOTR-05 had passed locally but before it was published, and both changes
affect vault opening. One final executable and source manifest verify the paired
changes. This uses the PSPR's allowed justified bundle, preserves all earlier
evidence, and avoids manufacturing a second checkout or discarding pending work.
HOTR-06 waits for the hosted repair result. This repair does not waive a gate,
expand filesystem authorization, or mark the original failed run successful.
