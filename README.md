<p align="center">
  <img src="docs/images/clankerflow.png" alt="clankerflow logo" width="160" />
</p>

# clankerflow

`clankerflow` is a Rust CLI for running AI workflows in your repository.

Workflows are authored in TypeScript and executed by a managed Node runtime, while Rust owns orchestration, state, and OpenCode lifecycle calls.

```ts
async function showcase(ctx, { agent, tickets, git }) {
  await agent.run({ title: "Planner", prompt: "Create one small feature ticket." });

  const ticket = ctx.ticket ?? (await tickets.getNext({ status: "OPEN" })).ticket;
  if (!ticket) throw new Error("No open ticket found");

  await git.checkoutBranch(ticket.branch ?? `ticket-${ticket.ticketId}`, "master");
  await tickets.updateStatus({ id: ticket.ticketId, status: "IN_PROGRESS" });

  await agent.run({ title: "Dev", prompt: `Implement ${ticket.filePath}` });
  await agent.run({ title: "QA", prompt: `Review ${ticket.filePath}` });
}
```

## What it does

- Initializes a project-local workflow scaffold under `.agents/`.
- Runs workflows on host or in a containment container.
- Persists workflow runs/events in SQLite.
- Integrates with OpenCode sessions from Rust (run/messages/events/cancel/command).
- Opens the OpenCode web UI for the current project when you start a workflow.

## Requirements

- Rust toolchain (edition 2024 compatible).
- Node + npm (used during `clankerflow init` to install runtime dependencies).
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

After build, the CLI binary is named `clankerflow`.

## Quick start

From the repository you want to automate:

```bash
clankerflow init
```

Then run a workflow:

```bash
clankerflow work duos
```

`clankerflow work` now also opens the OpenCode manage URL in your browser automatically.

## CLI commands

- `clankerflow init` - initialize or refresh `.agents` scaffold.
- `clankerflow work <name>` - run workflow `.agents/workflows/<name>.ts`.
  - `--env host|container`
  - `--yolo`
  - `--containment` (shorthand for container + yolo)
- `clankerflow manage` - open OpenCode sessions UI for current project.
- `clankerflow make ticket` - create a new ticket markdown file.
- `clankerflow make worktree <branch>` - create a git worktree under `.agents/.worktrees/<branch>`.
- `clankerflow containment up|down` - start/stop containment container.

## Workflow runtime notes

- Workflow files live at `.agents/workflows/*.ts`.
- Runtime helpers are scaffolded under `.agents/.clankerflow/lib`.
- Rust and Node communicate over structured JSON IPC.
- Run monitoring is done in the OpenCode web UI.

More short examples live in `docs/examples.md`.

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
