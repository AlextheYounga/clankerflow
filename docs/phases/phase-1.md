# Phase 1: CLI Commands

## Goal

Stand up all four top-level CLI commands (`init`, `work`, `manage`, `make`) with enough real logic to be useful immediately, and lay the supporting core modules they depend on.

## Completed Work

### Core modules

#### `src/core/embeds.rs`
Embeds the entire `src/kit/` directory into the binary at compile time via `rust-embed`. Exposes two functions used by `init`:

- `copy_kit(project_root, is_reinit)` — writes every embedded kit file into `.agents/`. On re-init, files under `tickets/`, `context/`, `settings.json`, `AGENTS.md`, and `README.md` are skipped so user edits survive.
- `place_gitignore(project_root)` — writes `.agents/.gitignore` from the embedded `gitignore.example`; skipped if the file already exists.

#### `src/core/settings.rs`
Typed `Settings` struct (with nested `GitSettings` and `WorkflowSettings`) backed by `.agents/settings.json`. Provides `Settings::load` and `Settings::save`.

#### `src/core/tickets.rs`
Ticket creation logic. Reads the embedded `ticket-template.md` at compile time (`include_str!`) and renders it with:

- Auto-incremented `T-NNN` numbering (scans existing filenames in `.agents/tickets/`).
- Optional title and branch substitutions.

Public surface: `create_ticket`, `create_ticket_with_title`, `create_ticket_with_branch`, `parse_ticket_number`, `tickets_dir`.

6 unit tests cover: first ticket numbering, increment, title substitution, branch substitution, and filename parsing (valid and invalid).

---

### Commands

#### `agentctl init` — `src/app/commands/init.rs`

1. Detects whether `.agents/` already exists (re-init vs. fresh init).
2. Calls `copy_kit` and `place_gitignore` from `core::embeds`.
3. On a fresh init, generates a `codebase_id` (hex string from timestamp + PID hash) and writes it into `settings.json` via `core::settings`.
4. Connects to the SQLite database, running all pending migrations.
5. Prints a contextual success message (re-init vs. fresh init).

Re-init preserves: `tickets/`, `settings.json`, `context/`, `AGENTS.md`, `README.md`.

#### `agentctl work <name>` — `src/app/commands/work.rs`

1. Requires an initialized project (`require_project_root`).
2. Resolves `.agents/workflows/<name>.js` or `.agents/workflows/<name>.ts`.
3. Prints the resolved path and intended env/yolo flags.
4. **Stub**: bails with "workflow execution not yet implemented" — Node IPC runtime is Phase 2.

#### `agentctl make ticket` — `src/app/commands/make.rs`

Calls `core::tickets::create_ticket`, prints the full path of the created file.

#### `agentctl make worktree <branch>` — `src/app/commands/make.rs`

1. Validates the branch name (no `..`, no leading `-`, no spaces).
2. Checks that `.agents/.worktrees/<branch>` does not already exist.
3. Runs `git worktree add -b <branch> .agents/.worktrees/<branch>`.
4. Creates a paired ticket in `.agents/tickets/` with the branch pre-filled.
5. Prints both paths.

#### `agentctl manage` — `src/app/commands/manage.rs`

Requires an initialized project, connects to the DB. **Stub**: prints "TUI not yet implemented" — Ratatui wiring is Phase 2.

---

## What is not yet done (Phase 2+)

- Node runtime spawning and Rust ↔ Node IPC protocol (`work` command).
- Ratatui TUI for `manage`.
- OpenCode session lifecycle (create / prompt / events / cancel) via Rust HTTP client.
- Container support (`--env container`, Docker compose lifecycle).
- Workflow JS runtime helpers (`src/kit/.agentctl/lib/`).
