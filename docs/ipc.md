# IPC System

Communication between the Rust CLI and Node.js workflow runtime over Node's built-in child IPC channel (`process.send` / `process.on("message")`).

## Overview

Rust spawns Node.js as a child process with `NODE_CHANNEL_FD=3` and a bidirectional Unix socket endpoint attached to fd 3. Node then enables its native IPC API automatically.

- **Rust** writes/reads newline-delimited JSON frames on the fd 3 channel.
- **Node** uses `process.send` and `process.on("message")`; Node handles JSON serialization/deserialization for the JavaScript side.

stdin/stdout/stderr are free for regular logging. `console.log` never collides with IPC.

## Rust Side

`src/core/daemon.rs` — `spawn_runner`:
```rust
// Create one bidirectional unix socket pair
let (parent_ipc, child_ipc) = socketpair(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC);

Command::new(node_bin)
    .arg(runner_path)
    .env("NODE_CHANNEL_FD", "3")
    .env("NODE_CHANNEL_SERIALIZATION_MODE", "json")
    .stdin(Stdio::null())
    .stdout(Stdio::inherit())   // free for console.log
    .stderr(Stdio::inherit())
    // dup2 child_ipc endpoint into fd 3 in the child
    .pre_exec(move || { /* dup2(child_ipc, 3) */ })
    .spawn()
```

Rust holds one endpoint (read + write) and splits it into read/write halves for the event loop.

## Disconnect behavior

- If the IPC channel disconnects, `runtime/src/ipc.ts` notifies the runner.
- `runtime/src/runner.ts` then performs graceful shutdown: abort active runs, wait for drain, then exit.
- This keeps run lifecycle behavior consistent with explicit `shutdown` command handling.

## Node Side

`runtime/src/ipc.ts` — `IpcTransport`:
```javascript
process.on("message", (message) => {
  // message is already parsed JSON object
});

process.send({
  v: "v1",
  id: "msg_1",
  kind: "event",
  name: "run_started",
  payload: {},
});
```

```
Rust process                         Node process
┌──────────────────┐                 ┌──────────────────┐
│ socketpair end A │◄── fd 3 channel ►│ socketpair end B │
│ (read + write)   │   byte stream    │ (read + write)   │
└──────────────────┘                 └──────────────────┘
```

## Message Format

On the Rust side, frames are newline-delimited JSON (one object per line, no embedded newlines):

```json
{"v":"v1","id":"cmd_1","kind":"command","name":"start_run","payload":{}}
```

| Field    | Type   | Description                          |
|----------|--------|--------------------------------------|
| `v`      | string | Protocol version ("v1")             |
| `id`     | string | Unique message identifier            |
| `kind`   | string | `command`, `response`, `request`, `event`, `error` |
| `name`   | string | Action name                          |
| `payload`| object | Data                                 |

## Message Flow

1. Rust spawns Node with `NODE_CHANNEL_FD=3` and `dup2(child_ipc, 3)` in `pre_exec`
2. Rust writes a `command` frame to the IPC channel
3. Node receives it via `process.on("message")`
4. Node sends `request`/`event` via `process.send(...)`
5. Rust reads frames, executes capabilities, and writes `response`/`error` frames back

## Message Types

### command (Rust → Node)

| Name          | Description                    |
|---------------|--------------------------------|
| `start_run`   | Start a workflow              |
| `cancel_run`  | Cancel an active run           |
| `shutdown`    | Graceful shutdown              |

### request (Node → Rust)

Node requests a Rust capability:

```json
{"kind":"request","payload":{"request_id":"req_1","capability":"session_run","params":{}}}
```

### response / error (Rust → Node)

```json
{"kind":"response","id":"req_1","payload":{...}}
{"kind":"error","id":"req_1","payload":{"error":"message"}}
```

### event (Node → Rust)

| Name            | Description              |
|-----------------|--------------------------|
| `run_started`   | Workflow started        |
| `run_finished`  | Workflow completed       |
| `run_failed`    | Workflow failed         |

## Capabilities

Node can invoke Rust functionality via the `capability` field. Currently supports:

- **`agent_*`** - OpenCode agent operations (delegated to `opencode::dispatch`)

## Implementation

| File                          | Role                                        |
|-------------------------------|---------------------------------------------|
| `src/core/ipc.rs`             | `IpcMessage` struct + constructors          |
| `src/core/daemon.rs`          | Spawns Node, wires `NODE_CHANNEL_FD`, runs event loop |
| `src/core/daemon/protocol.rs` | Message serialization and sending           |
| `src/core/capabilities.rs`   | Capability dispatch                         |
| `runtime/src/ipc.ts`         | `IpcTransport` (`process.send`/`message`) + `IpcRouter` |
| `runtime/src/runner.ts`      | Run lifecycle, delegates IPC to `IpcRouter` |

## Protocol Rules

- Rust channel framing: one JSON object per line
- Node API surface: parsed message objects via `process.on("message")`
- Unknown fields ignored (forward compatible)
- All messages include `v: "v1"`
- `kind` field acts as the dispatch switch on both sides
