# kata

`kata` is a Rust CLI for running AI workflows in your repository.

Workflows are authored in TypeScript and executed by a managed Node runtime, while Rust owns orchestration, state, and OpenCode lifecycle calls.

## What it does

- Initializes a project-local workflow scaffold under `.agents/`.
- Runs workflows on host or in a containment container.
- Persists workflow runs/events in SQLite.
- Integrates with OpenCode sessions from Rust (run/messages/events/cancel/command).
- Opens the OpenCode web UI for the current project when you start a workflow.

## Requirements

- Rust toolchain (edition 2024 compatible).
- Node + npm (used during `kata init` to install runtime dependencies).
- Docker (only if you use containment mode).
- OpenCode local server reachable at `http://127.0.0.1:4096` by default.

You can override the OpenCode URL in `.agents/settings.json`:

```json
{
  "opencode": {
    "server_url": "http://127.0.0.1:4096"
  }
}
```

## Install and build

```bash
cargo build
```

Run without installing globally:

```bash
cargo run -- <command>
```

After build, the CLI binary is named `kata`.

## Quick start

From the repository you want to automate:

```bash
kata init
```

Then run a workflow:

```bash
kata work duos
```

`kata work` now also opens the OpenCode manage URL in your browser automatically.

## CLI commands

- `kata init` - initialize or refresh `.agents` scaffold.
- `kata work <name>` - run workflow `.agents/workflows/<name>.ts`.
  - `--env host|container`
  - `--yolo`
  - `--containment` (shorthand for container + yolo)
- `kata manage` - open OpenCode sessions UI for current project.
- `kata make ticket` - create a new ticket markdown file.
- `kata make worktree <branch>` - create a git worktree under `.agents/.worktrees/<branch>`.
- `kata containment up|down` - start/stop containment container.

## Workflow runtime notes

- Workflow files live at `.agents/workflows/*.ts`.
- Runtime helpers are scaffolded under `.agents/.agentkata/lib`.
- Rust and Node communicate over structured JSON IPC.
- Run monitoring is done in the OpenCode web UI.

## Agent API (workflow side)

Current workflow `agent` tool surface includes:

- `agent.run(...)`
- `agent.command(...)`
- `agent.messages(sessionId)`
- `agent.events(sessionId)`
- `agent.cancel(sessionId)`

Example:

```ts
const started = await agent.run({ prompt: "Draft implementation plan" });

if (started.session_id) {
  await agent.command({
    session_id: started.session_id,
    command: "/review",
  });
}
```

## Development

Run Rust tests:

```bash
cargo test
```

Run runtime tests:

```bash
cd runtime
npm test
```
