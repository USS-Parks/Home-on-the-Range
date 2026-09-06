# HOTR-11-R1 — create the fresh backup fixture root

Status: local full gate PASS; exact clean hosted Windows run PASS at
`cf4fa8ed1373431733e01c9f3faa1229f4e5c9fa`, run 34003325869.

Exact HOTR-11 commit `91c12e2333ea4482cd3cd9a5c621b6f03f12464b` passed its local
gate but failed hosted run 34002264699. The first library backup test tried to
create its run directory before the fresh runner had `work/hotr-tests`. It failed
with Windows error 3 at `src/backup.rs:398`, before opening a database. Complete
hosted logs/artifact remain under `work/hotr-evidence/HOTR-11-hosted-34002264699/`.

Objective: validate and create that parent in the fixture before creating its
unique run. Scope is two lines inside `#[cfg(test)]`; runtime backup, encryption,
service permissions and other applications remain unchanged.

Gates: format, warnings-denied Clippy and the actual native multi-step backup unit
test, followed by the full HOTR-11 native/owner/two-account gate. The rebuilt
executable hash differs despite the source edit being restricted to `cfg(test)`;
the prior executable's test result is therefore not reused for this build. The
local full gate includes preserved, uncommitted HOTR-12 test helpers (its live
application test is ignored). Publish only this focused repair and require its
exact clean hosted Windows run before closing M1.
Local execution on a populated checkout is not fresh-run hosted evidence.

HOTR-12's source remains preserved and uncommitted in the canonical checkout.
Its first full local gate completed its ordinary native tests but was deliberately
interrupted at the separate-account stage before any model prompt. Evidence is
retained under `work/hotr-evidence/HOTR-12-78964-1788656332064645300/`; this is not a
passing HOTR-12 gate. Resume its complete gate after the repair's local checks,
track the prerequisite's hosted result, and do not repeat or discard its work.

Hosted closeout, 2026-09-06T01:31:01Z: the exact commit's clean runner completed
the native gate successfully. Its manifest reports PASS, `source.dirty=false`,
and the matching SHA above. Full logs and artifact are retained under
`work/hotr-evidence/HOTR-11-R1-hosted-34003325869/`. This closes the fresh-fixture
prerequisite; it does not replace HOTR-12's actual installed-client proof.
