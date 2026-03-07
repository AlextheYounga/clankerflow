# Phase 3: Node IPC Runtime

## Goal

Wire `agentctl work` to a real background runtime:

- Rust launches Node and owns process lifecycle.
- Rust and Node communicate over line-delimited JSON IPC.
- Run lifecycle events are persisted to SQLite.
- `work` returns immediately with a `run_id` while a detached worker continues in the background.

## What We Implemented

### Detached worker model

`agentctl work <name>` now:

1. Validates project/workflow/settings.
2. Creates workflow + run records.
3. Re-execs itself as hidden `_run` worker.
4. Prints `workflow started: <run_id>` and exits.

The `_run` worker calls `pump_workflow`, which owns the Node child and the full IPC loop.

### IPC runtime bridge

Rust (`src/core/daemon.rs`) now:

- spawns Node runner (`.agents/.agentctl/lib/runner.js`),
- sends `start_run`,
- handles incoming `event` and `request` messages,
- maps run terminal events to DB status updates,
- sends `cancel_run` when stop is requested,
- sends `shutdown` and waits with a grace period before kill.

Capability requests are parsed from `payload.capability`, `payload.params`, and
`payload.request_id` to match `runtime/src/runner.ts`.

### Build pipeline for runtime assets

`build.rs` bundles:

- `runtime/src/runner.ts` -> `src/kit/.agentctl/lib/runner.js`
- `runtime/src/helpers.ts` -> `src/kit/.agentctl/lib/helpers.js`

and reruns when `runtime/src/`, `runtime/package.json`, or `runtime/package-lock.json` changes.

### Node-side command execution

`exec` moved to the Node runtime context.

- `runtime/src/context.ts` now exposes `ctx.exec(command, args?)`.
- It uses `resolveExecSpec(...)` + `runExec(...)` in Node.
- Rust no longer exposes an `exec` IPC capability.

## Files Added

- `build.rs`
- `src/core/ipc.rs`
- `src/core/runtime.rs`
- `src/core/capabilities.rs`
- `src/core/opencode.rs`
- `src/db/migration/m0005_add_run_id_to_workflow_runs.rs`

## Files Changed

- `src/app/cli.rs`
  - Added hidden `_run` command with `--run-id`, `--workflow-path`, `--env`, `--project-root`, `--yolo`.
- `src/app/commands/work.rs`
  - Replaced stub with real launch flow.
- `src/app/commands/init.rs`
  - Always ensures `codebase_id` is stamped, including re-init.
- `src/core.rs`
  - Added module exports for `daemon`, `ipc`, `runtime`, `capabilities`, `opencode`.
- `src/core/daemon.rs`
  - Full process + IPC pump implementation.
- `src/db/db.rs`
  - Unified DB connection errors to `anyhow::Result`.
- `src/db/entities/workflow_run.rs`
  - Added `run_id` column to entity model.
- `src/db/migration.rs`
  - Registered migration `m0005_add_run_id_to_workflow_runs`.
- `runtime/src/context.ts`
  - Added Node-side `exec` helper.
- `runtime/tests/context.test.ts`
  - Added `ctx.exec(...)` behavior test.

## Naming/Structure Updates

- `handlers` renamed to `capabilities`.
- `handlers/agent.rs` renamed/moved to `opencode.rs`.
- No `mod.rs` files remain.

## Tests Added/Updated

- `src/core/ipc.rs` tests: message shape + round-trip behavior.
- `src/core/runtime.rs` tests: node binary resolution.
- `src/core/daemon.rs` tests: capability request payload parsing.
- `src/core/capabilities.rs` tests: routing + required param validation.
- `src/core/opencode.rs` tests: capability name validation + subscribe ack.
- `src/app/commands/init.rs` tests: `codebase_id` regeneration/preservation.
- `runtime/tests/context.test.ts`: `ctx.exec(...)` host execution.

## Out of Scope for Phase 3

- Real OpenCode client integration (capabilities are still stubs returning structured errors).
- Full container lifecycle management details.
- `manage` TUI runtime controls.
- `make validate` workflow linting.
