# HOTR-04-R2 — Bounded native pipe retirement

Date: 2026-09-05. Status: implementation under the approved full STS scope.

The exact HOTR-05/R1 commit `437f77887b512f3d7f1fcfcd0f922a42fb2d6719`
passed locally, including actual second-account denial. Its hosted Windows run
[33993684320](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33993684320)
passed structural ACL verification and ConPTY creation/unlock, but failed a
rapid owner reconnect with `UnexpectedEof`. The prior ACL defect is repaired;
the required complete hosted acceptance is still open. Original evidence is
retained. HOTR-06 waits for the repaired exact commit's hosted pass.

## Objective and repair

Keep the existing owner pipe alive across ordinary rapid reconnects. Tokio/Mio
can retain a dropped native handle until canceled I/O completion is dispatched.
A successor creation now yields and retries only native error 231 for at most
one second. It retains the two-instance bound and does not retry the initial
first-instance reservation or replay a transmitted operation. A consumed reply
is acknowledged before disconnect. The original hosted artifact did not retain
server stderr, so native slot retirement is a tested failure mode and repair
hypothesis, not a claimed reconstruction of that unobserved error code.

Move SQLCipher log suppression before key-dependent version queries so wrong
keys return generic errors without native pager diagnostics. Hosted artifacts
add synthetic server stderr and redacted console transcripts, excluding vault
files and credentials.

## Acceptance

1. Actual Windows pipe tests prove full-slot timeout and delayed-slot recovery
   within fixed bounds. Actual owner test performs 4,096 reconnects, a retained
   acknowledged client overlap, wrong-key rejection without stderr, and restart.
2. Full local format/build/Clippy/tests, ConPTY, encryption/canary scan, and actual
   distinct authenticated Windows principal denial pass on one source/binary.
3. Publish one focused repair commit to private main and require its exact hosted
   Windows workflow to pass. No skipped or failed test counts as acceptance.

The user renewed full stem-to-stern execution in direct response to the stated
project-edit permission block. Project-only formatting then succeeded. Existing
user data, application profiles, baseline copies, and historical evidence remain
preserved; this repair adds no OS configuration or discretionary cleanup.
