# Phase 6: Container Lifecycle

## Goal

Enable workflows to run inside Docker containers with a single `--containment`
flag that implies `--env container` and `--yolo`. Replace the socketpair/fd 3
IPC transport with TCP for both host and container modes, providing a unified
communication path. Add `agentctl containment up/down` subcommands for manual
container management.

## Design Decisions

### `--containment` flag

`agentctl work duos --containment` is shorthand for
`agentctl work duos --env container --yolo`. The explicit `--env` and `--yolo`
flags remain and still work independently. Conflicting combinations (e.g.
`--containment --env host`) produce a clear error at parse time.

### TCP replaces socketpair for IPC

The current IPC transport uses a Unix socketpair with `NODE_CHANNEL_FD=3` and
Node's built-in `process.send`/`process.on("message")`. This cannot cross the
Docker container boundary (the socketpair is a kernel object local to one
machine, and Docker Desktop on macOS uses a VM that prevents Unix socket files
from working on bind-mounted volumes).

TCP solves this cleanly:
- Works identically on host and inside containers (via Docker port mapping).
- Single bidirectional connection — same semantics as the socketpair.
- stdout/stderr remain free for console output (no mixing IPC with `console.log`).
- Both sides use the same transport — no conditional code paths for host vs container.

### Container model

Containers are per-project (persistent), not per workflow run. Named
`agent-{codebase_id}` using the codebase ID from settings. The workspace is
bind-mounted at `/workspace`. When `--containment` is used, Rust ensures the
container is running (building if needed), then spawns Node inside it via
`docker exec`, with IPC over TCP through a mapped port.

### Synchronous execution

Container workflows block the terminal, consistent with the Phase 3 synchronous
model. Daemon/background execution is a future phase.

## IPC Transport Change: Socketpair → TCP

### New flow (both host and container modes)

```
Rust                                    Node (host or container)
────                                    ────────────────────────
1. Bind TCP listener on 127.0.0.1:0
   (OS assigns a random available port)
2. Spawn Node (or docker exec Node)
   with env AGENTCTL_IPC_PORT=<port>
3. Accept one connection              → 4. Connect to 127.0.0.1:<port>
   ─── TCP stream established ───
5. Send start_run as JSON line        → 6. Receive, parse, dispatch
7. Read JSON lines from stream        ← 8. Send events/requests as JSON lines
   ... IPC loop continues ...
```

The wire format is unchanged: newline-delimited JSON, same `Message` /
`IpcMessage` structs, same `kind` variants (`command`, `response`, `request`,
`event`, `error`), same `v1` protocol version.

### Rust side changes

#### `spawn_runner()` — `src/core/runner.rs`

Replace socketpair + fd 3 setup with TCP:

```rust
async fn spawn_runner(project_root: &Path, env: RuntimeEnv, codebase_id: &str) -> Result<NodeRunner> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let child = match env {
        RuntimeEnv::Host => spawn_host_node(project_root, port)?,
        RuntimeEnv::Container => spawn_container_node(project_root, codebase_id, port).await?,
    };

    let (stream, _) = listener.accept().await?;

    Ok(NodeRunner { child, ipc: Some(stream) })
}
```

- `spawn_host_node()`: spawns `node runner.js` directly with
  `AGENTCTL_IPC_PORT=<port>`. No `NODE_CHANNEL_FD`, no `pre_exec`, no
  socketpair.
- `spawn_container_node()`: ensures container is running, then spawns
  `docker exec -e AGENTCTL_IPC_PORT=<port> <container_id> node /workspace/.agents/.agentctl/lib/runner.js`.
  The port is accessible because the docker-compose port mapping makes it
  available inside the container.

Remove `make_socketpair()` entirely.

#### `NodeRunner` struct

```rust
struct NodeRunner {
    child: Child,
    ipc: Option<TcpStream>,
}
```

`TcpStream` replaces `TokioFile`. Both implement `AsyncRead + AsyncWrite`, so
the split into read/write halves works identically.

#### `drive_ipc_loop()` — `src/core/runner.rs`

Change the signature from:

```rust
async fn drive_ipc_loop(ctx, ipc_write, ipc_read: tokio_io::ReadHalf<TokioFile>)
```

to:

```rust
async fn drive_ipc_loop(ctx, ipc_write, ipc_read: tokio_io::ReadHalf<TcpStream>)
```

Or make it generic over `AsyncRead + Unpin`. The body is unchanged.

#### `run_workflow()` — `src/core/runner.rs`

Receives `RuntimeEnv` and `codebase_id` so it can pass them to `spawn_runner()`.
The rest of the function (setup_run, send_start_run, drive_ipc_loop,
send_shutdown, wait_for_child) is unchanged.

#### Functions that do not change

All write-path functions already accept `impl AsyncWrite + Unpin`:
- `write_message()`
- `send_cancel()`
- `send_shutdown()`
- `send_start_run()`
- `handle_runner_line()`

No changes needed.

### Node side changes

#### `IpcTransport` — `runtime/src/ipc.ts`

Replace `process.send`/`process.on("message")` with TCP:

```typescript
import net from "net";

class IpcTransport {
  private socket: net.Socket | null = null;
  private messageHandler: ((msg: IpcMessage) => void) | null = null;
  private disconnectHandler: (() => void) | null = null;

  start(): void {
    const port = parseInt(process.env.AGENTCTL_IPC_PORT!, 10);
    this.socket = net.createConnection({ host: "127.0.0.1", port });

    // Line-delimited JSON framing
    let buffer = "";
    this.socket.on("data", (chunk) => {
      buffer += chunk.toString();
      const lines = buffer.split("\n");
      buffer = lines.pop()!; // keep incomplete line in buffer
      for (const line of lines) {
        if (line.trim()) {
          this.messageHandler?.(JSON.parse(line));
        }
      }
    });

    this.socket.on("close", () => this.disconnectHandler?.());
  }

  send(message: IpcMessage): void {
    this.socket?.write(JSON.stringify(message) + "\n");
  }

  onMessage(handler: (msg: IpcMessage) => void): void {
    this.messageHandler = handler;
  }

  onDisconnect(handler: () => void): void {
    this.disconnectHandler = handler;
  }
}
```

Remove all references to `NODE_CHANNEL_FD`, `NODE_CHANNEL_SERIALIZATION_MODE`,
`process.send`, and `process.on("message")`.

#### Files that do not change

- `IpcRouter` — operates on `IpcMessage` objects, fully decoupled from transport.
- `protocol.ts` — message format is transport-independent.
- `context.ts` — invokes capabilities through `IpcRouter`, no transport awareness.
- `runner.ts` — creates `IpcTransport` and `IpcRouter` at one construction site.
  The construction call is unchanged; `IpcTransport.start()` handles the new
  transport internally.
- `loader.ts` — loads workflow modules, no IPC involvement.

## Docker Module — `src/core/docker.rs`

A new module providing Docker Compose operations. All methods shell out to
`docker compose` (no Rust Docker SDK — keeps dependencies minimal and
transparent).

```rust
pub struct Docker;

impl Docker {
    pub fn is_available() -> bool;
    pub fn build(project_root: &Path, codebase_id: &str) -> Result<()>;
    pub fn up(project_root: &Path, codebase_id: &str) -> Result<()>;
    pub fn down(project_root: &Path, codebase_id: &str) -> Result<()>;
    pub fn is_running(project_root: &Path, codebase_id: &str) -> Result<bool>;
    pub fn get_container_id(project_root: &Path, codebase_id: &str) -> Result<String>;
    pub fn ensure_running(project_root: &Path, codebase_id: &str) -> Result<String>;
}
```

### Key methods

- `compose_file_path(project_root)` — returns
  `{project_root}/.agents/.agentctl/docker/agent.docker-compose.yaml`.
- `compose_args(project_root)` — returns
  `["-f", compose_file_path, ...]` used by all compose commands.
- `build()` — runs `docker compose -f <file> build` with `CODEBASE_ID` env var.
- `up()` — runs `docker compose -f <file> up -d` with `CODEBASE_ID` env var.
  Ensures `~/.config/opencode` exists first (prevents Docker from creating it as
  root-owned).
- `down()` — runs `docker compose -f <file> down` with `CODEBASE_ID` env var.
- `is_running()` — runs `docker compose ps -q --status=running`, returns true if
  non-empty output.
- `get_container_id()` — runs `docker compose ps -q --status=running agent`,
  returns the container ID string.
- `ensure_running()` — idempotent: checks `is_running()`, if false calls
  `build()` then `up()`, then returns `get_container_id()`.

All methods use `std::process::Command` (synchronous). Docker compose operations
are inherently blocking and short-lived; async adds no value here.

## Docker Compose Port Mapping

Update `src/kit/.agentctl/docker/agent.docker-compose.yaml` to use host
networking so the container can reach the host's TCP listener:

```yaml
services:
  agent:
    # ... existing config
    extra_hosts:
      - "host.docker.internal:host-gateway"
```

When Rust binds a TCP listener on the host, Node inside the container connects
to `host.docker.internal:<port>` instead of `127.0.0.1:<port>`. This works on
both Linux (Docker 20.10+) and macOS Docker Desktop.

Node's `IpcTransport.start()` detects container mode via an env var
(e.g. `AGENTCTL_CONTAINER=1`) and connects to `host.docker.internal` instead of
`127.0.0.1`.

## CLI Changes — `src/app/cli.rs`

### `--containment` flag on `work`

```rust
Work {
    name: String,
    #[arg(short, long, value_enum, default_value_t = RuntimeEnv::Host)]
    env: RuntimeEnv,
    #[arg(long, default_value_t = false)]
    yolo: bool,
    /// Run inside a container with dangerous mode enabled
    #[arg(long, default_value_t = false)]
    containment: bool,
},
```

In `run()`, resolve the effective env and yolo:

```rust
Commands::Work { name, env, yolo, containment } => {
    let (effective_env, effective_yolo) = if containment {
        if env != RuntimeEnv::Host || yolo {
            // --containment conflicts with explicit --env or --yolo
            anyhow::bail!("--containment cannot be combined with --env or --yolo");
        }
        (RuntimeEnv::Container, true)
    } else {
        (env, yolo)
    };
    commands::work::run(name, effective_env, effective_yolo).await
}
```

### `containment` subcommand

```rust
#[derive(Debug, Subcommand)]
pub enum Commands {
    // ... existing variants
    /// Manage containment containers
    Containment {
        #[command(subcommand)]
        command: ContainmentCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum ContainmentCommands {
    /// Build and start the agent container
    Up,
    /// Stop and remove the agent container
    Down,
}
```

### Command handlers

#### `src/app/commands/containment.rs`

```rust
pub fn up() -> Result<()> {
    let project_root = require_project_root()?;
    let settings = Settings::load(&project_root)?;
    Docker::ensure_running(&project_root, &settings.codebase_id)?;
    println!("container ready");
    Ok(())
}

pub fn down() -> Result<()> {
    let project_root = require_project_root()?;
    let settings = Settings::load(&project_root)?;
    Docker::down(&project_root, &settings.codebase_id)?;
    println!("container stopped");
    Ok(())
}
```

## Container Workflow Execution — `src/core/runner.rs`

When `env == RuntimeEnv::Container`:

1. `Docker::ensure_running()` — build and start if not already running.
2. `Docker::get_container_id()` — get the running container's ID.
3. Bind a TCP listener on `127.0.0.1:0` (random port).
4. Spawn `docker exec` with the container ID, running
   `node /workspace/.agents/.agentctl/lib/runner.js` with
   `AGENTCTL_IPC_PORT=<port>` and `AGENTCTL_CONTAINER=1`.
5. Accept the TCP connection from Node inside the container.
6. Proceed with the normal IPC loop — identical from this point on.

```rust
fn spawn_container_node(
    project_root: &Path,
    codebase_id: &str,
    port: u16,
) -> Result<Child> {
    let container_id = Docker::ensure_running(project_root, codebase_id)?;
    let runner_path = "/workspace/.agents/.agentctl/lib/runner.js";

    Command::new("docker")
        .args(["exec"])
        .args(["-e", &format!("AGENTCTL_IPC_PORT={port}")])
        .args(["-e", "AGENTCTL_CONTAINER=1"])
        .arg(&container_id)
        .args(["node", runner_path])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("failed to exec in container: {e}"))
}
```

stdout/stderr are inherited so `console.log()` in workflows appears in the
user's terminal. IPC is on the separate TCP channel — no mixing.

## Files Added

- `docs/phases/phase-6.md` — this document.
- `src/core/docker.rs` — Docker Compose operations module.
- `src/app/commands/containment.rs` — `up`/`down` command handlers.

## Files Changed

- `src/core.rs` — export `docker` module.
- `src/core/runner.rs` — replace socketpair with TCP listener/accept; add
  `spawn_container_node()`; remove `make_socketpair()`; update `NodeRunner` to
  use `TcpStream`; pass `RuntimeEnv` and `codebase_id` through to
  `spawn_runner()`.
- `src/app/cli.rs` — add `--containment` flag on `work`; add `Containment`
  subcommand with `Up`/`Down`; add conflict validation.
- `src/app/types.rs` — add `ContainmentCommands` enum.
- `src/app/commands.rs` — export `containment` module.
- `runtime/src/ipc.ts` — replace `process.send`/`process.on("message")` with
  `net.createConnection()` and line-delimited JSON framing over TCP.
- `src/kit/.agentctl/docker/agent.docker-compose.yaml` — add
  `host.docker.internal` extra host mapping.
- `Cargo.toml` — add `tokio` TCP features if not already present (likely
  already included via existing tokio dependency).

## Tests

All tests must be offline — no Docker daemon required.

### Rust

- `src/core/docker.rs` — test `compose_file_path()` returns correct path; test
  `compose_args()` builds correct argument vector. Docker command execution is
  not unit-tested (requires Docker daemon); covered by integration tests.
- `src/core/runner.rs` — test TCP listener binds and accepts a connection
  (localhost, no Docker); test that `NodeRunner` struct holds `TcpStream`
  correctly.
- `src/app/cli.rs` — test `--containment` parses; test `--containment`
  conflicts with explicit `--env container` or `--yolo`; test `containment up`
  and `containment down` parse.

### Node

- `runtime/tests/ipc.test.ts` — test `IpcTransport` connects to a TCP server
  and exchanges JSON messages; test line-delimited framing handles partial
  chunks; test disconnect fires on server close.

## Out of Scope

- Background/daemon execution (future phase).
- Shipping agentctl binary inside the container image.
- tmux session management inside containers.
- Container health checks or restart policies.
- Multi-container workflows.
