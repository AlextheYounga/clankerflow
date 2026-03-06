# Phase 2: Rust Testing

## Goal

Add fast, offline Rust tests for the current CLI and core modules.

## Rules

- Use in-file `#[cfg(test)]` tests.
- No network.
- Do not run real `git` commands in tests.
- Do not add tests for `new_codebase_id()` uniqueness.

## Test Targets

### `src/core/project.rs`
- Project root found in current dir.
- Project root found in parent dir.
- Missing `.agents/` returns error.

Refactor: extract a pure helper that does not depend on global CWD.

### `src/core/settings.rs`
- Load valid JSON.
- Save and round-trip JSON.
- Missing file errors.
- Malformed JSON errors.

### `src/core/embeds.rs`
- Fresh init writes scaffold files.
- Re-init overwrites existing scaffold files after confirmation.
- Re-init restores missing scaffold files.
- `.agents/.gitignore` is rewritten during overwrite.

Behavior:
- If `.agents/` exists, warn that init will overwrite it.
- If confirmed, overwrite the scaffold.

### `src/app/commands/work.rs`
- Resolves `<name>.js`.
- Resolves `<name>.ts` if `.js` is missing.
- Prefers `.js` when both exist.
- Error includes workflows directory.
- Rejects unsafe names like `/`, `\\`, or `..`.

### `src/app/commands/make.rs`
- `validate_branch_name` accepts valid names.
- Rejects empty, `..`, leading `-`, and spaces.

### `src/app/cli.rs` (optional)
- `agentctl work duos` uses default args.
- `--env container` parses.
- Invalid `--env` rejects.
- `agentctl make worktree <branch>` parses.

## Nice-to-have

- Extra `src/core/tickets.rs` edge cases.
- DB smoke tests if `connect()` is refactored to accept an explicit project root.

## Priority

1. `src/core/project.rs`
2. `src/core/embeds.rs`
3. `src/app/commands/work.rs`
4. `src/core/settings.rs`
5. `src/app/commands/make.rs`
6. `src/app/cli.rs`
