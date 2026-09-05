# HOTR-06-R1 — Fresh-checkout fixture initialization

Date: 2026-09-05. Repair under the existing full STS approval.

HOTR-06 passed locally and was published as
`edf926ce8b5d29d75dc6e147b42217e4799fd8af`. Its exact hosted run
[33996469717](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/33996469717)
failed all five transaction tests while canonicalizing their absent shared test
root, before any transaction fixture was created. Unit tests run before the
encryption integration test that previously created that directory. The complete
failure is retained in the downloaded hosted artifact under work/hotr-evidence.

Make transaction fixtures create their approved project test root after validating
its absolute path and ancestors. Existing directories are retained; no deletion,
replacement, dependency on test order, or operating-system configuration is added.

Acceptance: full local transaction/owner/encryption/security gates pass; the exact
published candidate passes a fresh hosted Windows build. This is bundled with the
already active HOTR-07/08 boundary work, whose pending changes remain preserved.
The repair is a fixture initialization change; one combined source manifest and
hosted candidate verify it without discarding pending work or duplicating the
checkout. HOTR-09 waits for that fresh hosted pass. The original HOTR-06 hosted
failure remains a failed run and is never relabeled.
