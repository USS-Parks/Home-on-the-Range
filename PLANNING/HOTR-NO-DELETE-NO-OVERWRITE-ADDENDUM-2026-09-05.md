# Home on the Range — Preservation Addendum

Date: 2026-09-05
Status: ACTIVE USER CONSTRAINT; implementation still pending STS approval.

## Authority

The user instructed: "Do not delete any files or overwrite anything." The user specifically expressed concern about autonomous agents wiping data.

This later instruction overrides conflicting permissions in the initial PSPR and project AGENTS.md, including implicit cleanup of disposable test fixtures, in-place corrections, replacement of restored vaults, key rotation of existing vault files, retention purges, and cleanup during uninstall. No existing plan or file has been edited to record this addendum.

## Mandatory working rule

- Do not delete, truncate, replace, overwrite, or modify any existing file without explicit authorization for the exact path and action. Do not treat an implementation/STS instruction alone as an exception to this preservation rule.
- Do not run cleanup, reset, restore-over, force checkout, force push, garbage collection, cache eviction, worktree removal, recursive deletion, or similar operations as discretionary housekeeping.
- Do not move or rename existing user files as a way around this rule. Do not assume version control, a backup, an ignored status, a generated filename, or a temporary directory grants permission to remove or replace something.
- New documents and other new files may be created only with create-new/no-clobber semantics. If the chosen path exists, stop that write and choose an explicitly new destination; never replace the existing path.
- Changes to an existing file are prepared as a separate proposed patch or a new version at a new path. Show the exact paths and effects and obtain the user's explicit authorization before applying them.
- Inspect tool side effects before execution. Builds, package managers, Git operations, database writes, test tools, installers, and services may modify existing files even when their primary purpose sounds harmless. Do not run such a tool if its necessary writes violate this rule.
- Read-only inspection and additive drafting may continue. The earlier standing commit/push authorization does not override this newer preservation instruction; Git publication is pending clarification/explicit authorization because it changes existing repository metadata and branch references.

## Consequences for the current PSPR

1. This addendum is active immediately. HOTR-01–36 remain unstarted.
2. The existing private repository and all existing local files remain untouched by this policy update.
3. Synthetic test data may not be deleted, corrupted, rekeyed, restored over, or overwritten merely because it is synthetic. Those actions require an exact, reviewed sandbox scope and explicit user authorization first.
4. Stress runs must stop at their storage ceiling. They must not silently purge fixtures, caches, logs, models, or old results to make room.
5. Fault campaigns, mutable vault development, dependency installation, builds, upgrades, and normal runtime persistence need a clearly bounded write scope approved by the user before those dependent steps can begin.
6. Until that approval exists, prepare changes additively and leave potentially conflicting operations unexecuted. No blanket exception is inferred.

## Recording method

This is a newly created, uncommitted addendum. The existing PSPR, AGENTS.md, README, logs, source-control state, vaults, caches, and other files were not modified to record it. The author verifies hashes of previously tracked files after creation.
