# Phase 3: Node IPC Runtime (Synchronous)

## Goal

Wire `agentctl work` to a real workflow runtime:

- Rust spawns Node directly and owns process lifecycle.
- Rust and Node communicate over line-delimited JSON IPC.
- Run lifecycle events are persisted to SQLite.
- `work` blocks the terminal until the workflow completes (or the user hits Ctrl+C).
- Node stdout/stderr pass through to the user's terminal for visibility.

v1 intentionally avoids daemon/background execution to keep the debugging
surface simple. See `docs/future/daemon.md` for the upgrade path.

## Execution Model

```
agentctl work duos
  │
  ├─ validate project / workflow / settings
  ├─ create workflow + run DB records
  ├─ spawn Node (runner.js) as a child process
  │    ├─ IPC on fd 3 (socketpair)
  │    ├─ stdout → inherited (user terminal)
  │    └─ stderr → inherited (user terminal)
  ├─ send start_run over IPC
  ├─ enter IPC read loop (blocks until terminal event or EOF)
  │    ├─ handle request messages (capability dispatch)
  │    ├─ handle event messages (DB persist + status transitions)
  │    └─ on SIGINT → send cancel_run, await graceful shutdown
  ├─ send shutdown, wait with grace period, kill if needed
  └─ print final status and exit
```

No hidden `_run` command. No detached process. No `setsid`. The `work`
command itself drives the entire lifecycle in a single process.

## What Changes (relative to the previous daemon-based Phase 3)

### Removed: detached worker model

- `launch_workflow` no longer re-execs itself as a hidden `_run` worker.
- The hidden `_run` CLI command is removed from `src/app/cli.rs`.
- `daemon/process.rs` (`detach_process`, `setsid`) is removed.
- `is_stop_requested` DB polling for cancellation is removed — cancellation
  is now handled in-process via SIGINT.

### Changed: `work.rs` drives the runtime directly

`commands::work::run()` calls a function (e.g. `run_workflow`) that:

1. Creates DB records (workflow + run).
2. Spawns Node directly (no intermediate detached process).
3. Runs the IPC loop to completion.
4. Updates final run status in DB.
5. Prints a summary and exits.

### Changed: Node stdout/stderr are inherited

In the daemon model, the detached worker's own stdio was `/dev/null`, so Node
output was invisible. In the synchronous model, `Stdio::inherit()` is used for
both stdout and stderr on the Node child, so `console.log` and error output
appear in the user's terminal.

### Added: SIGINT handling

Register a signal handler (or use `tokio::signal`) so Ctrl+C during a running
workflow:

1. Sends `cancel_run` to Node over IPC.
2. Waits for Node to acknowledge with a `run_finished` or `run_failed` event
   (with a grace period timeout).
3. If Node doesn't exit within the grace period, sends `shutdown` then kills.
4. Sets run status to `Cancelled` in DB.
5. Exits with a non-zero code.

A second Ctrl+C during the grace period should force-kill immediately.

### Simplified: daemon.rs restructure

The daemon module collapses. The sub-module split (`process.rs`, `protocol.rs`,
`store.rs`) made sense for the daemon model where process management was a
first-class concern. In the synchronous model:

- `process.rs` is removed entirely (no detach, no setsid).
- `protocol.rs` and `store.rs` remain as they are — IPC serialization and DB
  operations are still needed.
- The top-level `daemon.rs` is renamed or restructured to reflect that it no
  longer manages a daemon. A name like `workflow_runner.rs` or keeping
  `daemon.rs` with clearer function names both work.

## What Stays

### IPC transport (socketpair + fd 3)

Unchanged. Rust creates a Unix socketpair, dup2s the child end to fd 3, sets
`NODE_CHANNEL_FD=3` and `NODE_CHANNEL_SERIALIZATION_MODE=json`. Node uses
`process.send` / `process.on("message")`. Newline-delimited JSON frames.

### IPC message protocol

Unchanged. Same `IpcMessage` struct, same `kind` variants (`command`,
`response`, `request`, `event`, `error`), same `v1` protocol version.

### Capability dispatch

Unchanged. `src/core/capabilities.rs` routes `request` messages to capability
handlers. `src/core/opencode.rs` handles OpenCode capabilities (still stubs
until Phase 5).

### Build pipeline

Unchanged. `build.rs` bundles `runtime/src/runner.ts` and
`runtime/src/helpers.ts` via esbuild into `src/kit/.agentctl/lib/`.

### DB persistence

Unchanged. Workflow + run records are created before Node is spawned. Events
are appended during the IPC loop. Terminal status is set on completion.

### Node runtime code

Unchanged. `runtime/src/runner.ts`, `ipc.ts`, `context.ts`, `protocol.ts`,
`loader.ts`, and all helpers remain as-is. The Node side doesn't know or care
whether Rust is a daemon or a foreground process.

### protocol.rs (IPC serialization)

Unchanged. `LoopControl`, `parse_capability_request_payload`, `write_message`,
`send_cancel`, `send_shutdown` all remain.

### store.rs (DB operations)

Mostly unchanged. `upsert_workflow`, `create_run`, `set_status`,
`append_run_event` all remain. `is_stop_requested` is removed since
cancellation is now in-process via SIGINT rather than DB polling.

## Files Changed

- `src/app/cli.rs`
  - Remove the hidden `_run` command and its args.
- `src/app/commands/work.rs`
  - Replace `launch_workflow` call with direct `run_workflow` call that blocks.
- `src/core/daemon.rs` (or renamed)
  - Remove `launch_workflow` (the detach + re-exec function).
  - Collapse `drive_workflow_runtime` into a simpler `run_workflow` that
    spawns Node, runs the IPC loop, and returns the final status.
  - Remove `detach_process` / `setsid` usage.
  - Add SIGINT handler wiring.
- `src/core/daemon/process.rs`
  - Remove entirely.
- `src/core/daemon/store.rs`
  - Remove `is_stop_requested`.

## Tests

### Updated

- `src/core/daemon.rs` tests: remove detach-related assertions, add tests for
  the synchronous run flow (spawn → IPC loop → completion).
- `src/app/commands/work.rs` tests: update to reflect blocking behavior.

### Unchanged

- `src/core/ipc.rs` tests: message shape + round-trip.
- `src/core/runtime.rs` tests: node binary resolution.
- `src/core/capabilities.rs` tests: routing + param validation.
- `src/core/opencode.rs` tests: capability name validation.
- `runtime/tests/context.test.ts`: `ctx.exec(...)` behavior.

## Out of Scope

- Background/daemon execution (see `docs/future/daemon.md`).
- Real OpenCode client integration (capabilities are stubs until Phase 5).
- Container lifecycle management.
- `make validate` workflow linting.
