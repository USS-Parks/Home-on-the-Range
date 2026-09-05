# Proposed bounded write exception — approval required

Date: 2026-09-05
Status: PROPOSAL ONLY. This does not grant itself permission.

The user approved full STS and the stack. The earlier preservation addendum explicitly says STS does not waive its prohibition on overwrites, including database writes, build outputs, Git metadata, and test fixtures. Implementation therefore needs a small, explicit exception. All existing user data and installed-app settings remain protected.

## Exact proposed boundary

Root: C:\Users\17076\Documents\Codex\Home-on-the-Range

| Paths below that root | Proposed allowed action |
|---|---|
| Cargo.toml, Cargo.lock, rust-toolchain.toml, build.rs; src/**, tests/**, examples/**, xtask/**, integrations/**, ui/**, .cargo/**, .github/** | Create and edit HOTR-owned source/build configuration only; retain proposed changes in version control |
| README.md, AGENTS.md, .gitignore, .gitattributes, PLANNING/**, docs/** | Maintain this project's plan, source guidance, logs and sanitized evidence; preserve a new baseline copy before the first approved edit |
| .git/** | Ordinary add/commit/fetch/push bookkeeping and transient Git lock-file lifecycle; no reset, force push, clean, gc, prune, history rewrite, or worktree removal |
| work/hotr-build/** | Compiler/cache/temp outputs for this project; tools may update/recreate their own new outputs and remove their own transient files |
| work/hotr-tests/** | Create and mutate specifically marked synthetic vaults/WAL/index/backup fixtures for the approved tests; permit SQLite's journal/WAL lifecycle; preserve fault evidence |
| work/hotr-client-profiles/** | New isolated application profiles for integration proof only; no edits to existing global application profiles |
| work/hotr-tool-cache/** | Project-owned native/dependency build cache if existing shared caches cannot be reused read-only; no copying huge unrelated caches |
| work/hotr-evidence/** | Append or create run logs/results/manifests; no replacing historical runs |
| work/hotr-baselines/** | New immutable copies/hash manifest of existing project files before their first edit; do not replace prior snapshots |

All resolved write paths must remain inside the canonical root. Reject symlink/junction escapes and paths pointing at actual user vaults, another repository, existing app profiles, or model directories. Run from one checkout. Do not create a staging duplicate of the repository or another worktree.

## Still prohibited under this proposed exception

- Deleting, moving, or renaming any pre-existing project/user file.
- Removing retained build caches or completed test evidence as discretionary cleanup.
- Deleting actual memories, projects, model weights, installed apps, or user data.
- Changing anything in another repository, Codex-managed memories, user home settings, Windows global configuration, or existing agentmemory/Ollama stores.
- Docker prune, container/volume deletion outside a specifically approved fixture, host-home/Docker-socket mounts.
- Installing logon startup, creating another OS user, modifying existing app MCP config, writing real vaults under LocalAppData, or downloading outside the project without a later exact-path proposal.
- Mutating existing .git hooks/config to weaken checks, bypassing protections, or force publishing.

If a tool requires an unavoidable write outside these paths, stop that action and name the path/operation. Do not reinterpret this exception broadly.

## Concrete next operation if approved

1. Snapshot/hash the eight project files that predate the current implementation turn into new files under work/hotr-baselines/.
2. Commit the preservation addendum, full-STS/compatibility addendum, and HOTR-01 ADR to main, preserving their history.
3. Create the minimal HOTR-02 Rust package, build against a verified SQLCipher source version through rusqlite, and run encryption tests only inside a new owned work/hotr-tests/HOTR-02-* run directory.
4. Use the same bounded source/build/test/Git permissions for subsequent approved prompts; request no repetitive approval for operations already covered.
5. Stop at explicit outside-boundary actions or unavailable live gates. No automatic cleanup.

This exception authorizes ordinary project development and purpose-built synthetic tests, not destruction of existing data. The user may approve a smaller subset instead.
