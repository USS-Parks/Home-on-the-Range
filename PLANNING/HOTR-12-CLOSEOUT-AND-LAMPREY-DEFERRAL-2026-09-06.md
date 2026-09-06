# HOTR-12 publication closeout and Lamprey deferral

Status: AUTHORIZED by the user's current instruction, 2026-09-06 UTC.

> No, commit HOTR 12 and merge with main. Worry about Lamprey later.

HOTR-12's implementation is already committed directly to main at
`ce1f8f7a8a72780aaf69f6bbf7a2d324f563518f`, with the same SHA verified on remote
main. No separate feature branch or merge remains. Exact Windows CI run
[34004546514](https://github.com/USS-Parks/Home-on-the-Range/actions/runs/34004546514)
passed; its downloaded native artifact confirms that commit and a clean source
tree. Actual Codex/Claude integration evidence is recorded separately in
`docs/evidence/HOTR-12-applications.json` and `docs/M1-DEMO.md`.

The current publication cut is HOTR-12. Publish this documentation closeout to
public main with the verified implementation unchanged. Preserve all unfinished
Lamprey source, tests, drivers, budget proposal and local evidence in place.
Do not stage that implementation, delete it, reset it, or claim it passed.

HOTR-12-LAMPREY is explicitly DEFERRED by the owner. Its unfinished live gate,
formatting request and proposed additional model budget do not block HOTR-12
publication. The 72-prompt budget proposal is not approved by this instruction.
No additional model calls or Lamprey formatting are part of this closeout.
The earlier Lamprey checkpoint remains as history for later resumption.

This deferral supersedes earlier instructions to finish Lamprey before closing
HOTR-12. It does not convert Lamprey or any remaining PSPR prompt into a pass,
and it does not establish installation, full STS or deployment acceptance.

Use the existing main checkout. Verify the final remote SHA after publication.
Retain the sole checkout and its generated state for later approved work; no
worktree creation or discretionary cleanup is needed.
