# Future: Daemon / Background Workflow Execution

## Context

v1 runs workflows synchronously — `agentctl work` blocks the terminal until
the workflow finishes. This is simpler to debug and reason about. This document
captures what needs to change to support background daemon execution later.

The original daemon design was implemented in the first iteration of Phase 3
and then reverted to synchronous for v1. The infrastructure was proven to work,
so most of this is re-introduction rather than new design.

## Why Daemon Mode

- **Don't steal the terminal.** Long-running workflows (multi-step agent
  orchestration, CI-style pipelines) shouldn't hold a terminal hostage.
- **Survive terminal close.** A workflow started in a terminal should keep
  running if the user closes the tab or SSH disconnects.
- **Multiple concurrent workflows.** Users may want to kick off several
  workflows and monitor them all via the OpenCode web UI.

## Architecture

```
agentctl work duos
  │
  ├─ validate project / workflow / settings
  ├─ create workflow + run DB records
  ├─ spawn detached worker: agentctl _run --run-id X --workflow-path Y ...
  │    └─ setsid() makes it a session leader (survives terminal close)
  │    └─ stdin/stdout/stderr → /dev/null
  ├─ print "workflow started (run id: <run_id>)"
  └─ exit immediately

agentctl _run (detached worker)
  │
  ├─ spawn Node (runner.js) with IPC on fd 3
  ├─ IPC loop (same as synchronous, but no terminal output)
  ├─ poll DB for stop requests (cancellation via external process)
  └─ update final status in DB
```

Two key differences from synchronous:

1. **Two-process model.** The CLI re-execs itself as a hidden `_run` command
   that runs detached. The original process exits immediately.


## What Needs to Be Built

### 1. Hidden `_run` CLI command

Add a hidden subcommand to `src/app/cli.rs`:

```rust
#[command(hide = true, name = "_run")]
Run {
    #[arg(long)] run_id: String,
    #[arg(long)] workflow_path: String,
    #[arg(long)] env: String,
    #[arg(long)] project_root: String,
    #[arg(long)] yolo: bool,
}
```

This is the entry point for the detached worker process. It calls
`drive_workflow_runtime()` with the provided args.

### 2. Detached process spawning

In `daemon.rs` (or `daemon/process.rs`), add a `detach_process` function:

- Spawns `agentctl _run` with the run's parameters.
- Uses `pre_exec` + `libc::setsid()` to make the child a session leader.
- Sets stdin/stdout/stderr to `Stdio::null()`.
- The parent does not wait on the child — it exits immediately.

Platform note: on Windows, use `CREATE_NEW_PROCESS_GROUP` instead of `setsid`.

### 3. DB-polled cancellation

Add an `is_stop_requested(db, run_id)` function to `daemon/store.rs`:

- Reads a flag from the workflow run record (e.g. a `stop_requested` boolean
  column, or a status transition to `Cancelling`).
- The IPC loop checks this on each iteration.
- When true, sends `cancel_run` to Node and transitions to `Cancelled`.

This replaces the synchronous model's SIGINT handling for the daemon case.

### 4. `agentctl stop <run_id>` command

A new CLI command that:

1. Looks up the run in the DB.
2. Sets the stop-requested flag.
3. The detached worker picks it up on its next poll cycle.

Alternatively, the OpenCode web UI could set this flag directly if it has
DB access, or via a small HTTP endpoint.

### 5. Node stdout/stderr routing

In daemon mode, Node's stdout/stderr go to `/dev/null` (since the detached
worker has no terminal). Options for preserving output:

- **Log file.** Redirect Node stdout/stderr to a per-run log file under
  `.agents/.agentctl/logs/<run_id>.log`.
- **DB events.** Capture stdout lines as IPC events and persist them.
- **Both.** Log file for raw output, DB events for structured data.

The log file approach is simplest and most debuggable.

### 6. SIGINT handling changes

In daemon mode, SIGINT handling on the `_run` worker is different:

- The worker is a session leader, so terminal SIGINT doesn't reach it.
- Graceful shutdown is driven by DB polling (stop-requested flag), not signals.
- The worker should still handle SIGTERM for system-level shutdown (e.g.
  `kill <pid>`), treating it the same as a cancel request.

## What Doesn't Change

- **IPC transport.** Socketpair + fd 3, same protocol, same message format.
- **Capability dispatch.** Same routing, same handlers.
- **DB schema.** Workflow + run + event tables stay the same. May need a
  `stop_requested` column or equivalent on `workflow_runs`.
- **Node runtime.** `runner.ts`, `ipc.ts`, `context.ts` — all unchanged. Node
  doesn't know if Rust is foreground or background.
- **Build pipeline.** `build.rs` + esbuild bundling is unchanged.

## Migration Path

The synchronous `run_workflow` function and the daemon's
`drive_workflow_runtime` share the same core: spawn Node, run IPC loop, handle
events and capabilities, update DB. The difference is only in:

1. **How the process is launched** (inline vs. detached re-exec).
2. **How cancellation arrives** (SIGINT vs. DB poll).
3. **Where Node output goes** (terminal vs. /dev/null or log file).

A clean approach: keep the core IPC loop as a shared function, and have the
synchronous and daemon code paths differ only in setup and teardown. This could
be a runtime flag (`--foreground` / `--background`) or two thin wrappers around
the same loop.

## Estimated Scope

- `src/app/cli.rs` — add hidden `_run` command (~20 lines).
- `src/core/daemon/process.rs` — `detach_process` + `wait_for_child` (~80 lines).
- `src/core/daemon/store.rs` — `is_stop_requested` (~15 lines).
- `src/app/commands/work.rs` — swap `run_workflow` for `launch_workflow` (~10 lines).
- New: `src/app/commands/stop.rs` — stop command (~40 lines).
- DB migration for `stop_requested` column if needed.
- Tests for detach, stop, and DB polling.

Most of this existed in the original Phase 3 implementation and can be
reintroduced with minimal changes once the synchronous v1 is stable.
