# Project Plan

## Purpose

**Name:** `agentctl`
**Goal:** Build an AI workflow framework where workflows are authored in Javascript and orchestrated by a Rust CLI.

## Important Notes
- There are no existing projects. You are free to create everything as if it were pre-launch. 

## Architecture

- Rust-powered CLI and orchestrator.
- Designed as a drop-in agent system for any codebase with hands-free development workflows.
- Ability to create deterministic AI workflows using plain Javascript.
- Typescript runtime that is compiled to Javascript `.agents/lib` folder, containing the runtime assets and helpers for creating deterministic workflows.
- The `dist/` payload is embedded in the Rust binary and emitted during `agentctl init` into `.agents/lib`.
- Clap-powered CLI. Workflow monitoring is handled by the OpenCode web UI (`agentctl manage` opens it in the browser).
- Workflow JavaScript is executed by a managed Node runtime process started by Rust.
- Rust and Node communicate over stdio using structured JSON messages (IPC).
- Transport model is intentionally Tauri-like: TypeScript workflow APIs are thin wrappers that invoke Rust capabilities over IPC.
- Rust owns process lifecycle (start/stop/cancel/timeouts) and state tracking.
- Rust is the source of truth for persistent state (SQLite with SeaORM).
- Tickets are stored as Markdown files in `.agents/tickets/` to remain human-readable and editable. Rust handlers perform filesystem operations for ticket APIs. These are not crucial for workflow operation, just a helper system.
- TypeScript owns helpers and workflow composition, but not Opencode API handling, this goes through Rust in order for more fine-grained control.
- There is no built-in workflow resume concept. 
- Rust communicates with Opencode API. Typescript just owns workflow running and workflow helpers, passing back data to Rust via IPC channel. 
- Workflow examples are stored at `src/kit/workflows` and these should not be changed heavily, as this is the syntax we are aiming for. 

### Container & Runtime
- Container model is per-project (persistent), not per workflow run.
- Container named `workflow-{codebase_id}` using codebase_id from settings. Codebase ID is the base64-encoded (no padding) absolute project path, written to settings.json on `init`. This matches the project identifier used by the OpenCode web UI.
- Containers are not spawned by default, but set on a per workflow basis in the `meta` block in the workflow. The point of contained workflows is so that they can be run safely on dangerous mode (`--yolo` or `--dangerously-bypass-approvals-and-sandbox` in codex-speak. In opencode it's --yolo.)
- Under both host and container workflow runtimes, they should run as a background daemon; do not steal my terminal from me with a blocking process. Monitoring is done via the OpenCode web UI, opened by `agentctl manage`.

## OpenCode Lifecycle Integration
- OpenCode REST API is a primary integration point for agent session lifecycle.
- `docs/references/opencode-sdk.md` is the reference for modeling session create/prompt/events/messages/cancel flows.
- Prefer implementing OpenCode lifecycle orchestration in Rust 
- Prior project behavior may differ; this project should optimize for explicit Rust ownership and consistent lifecycle semantics.

## Workflow Runtime Strategy

- Keep Rust as the core runtime for CLI, daemon orchestration, SQLite, and container management.
- Keep workflows in TypeScript for authoring speed and maintainability.
- Use Node for workflow execution to preserve Node APIs (`fs`, `child_process`, etc.).
- Do not execute workflows through shell strings; spawn Node directly from Rust process APIs.
- Preferred distribution model: ship a bundled Node runtime with `agentctl` releases.
- No fallback: bundled Node is required for releases.
- Dev override: `AGENTCTL_NODE_BIN` can point to a local Node binary.

## User Flow

- User runs `agentctl init` in a new or existing repository.
- A `.agents` folder is created in the workspace. If `.agents` already exists, `init` exits with an error (no overwrite).
- The `init` command also places a boilerplate `.opencode/opencode.json` for project-local OpenCode settings.
- User runs `agentctl work duos` to execute `.agents/workflows/duos.ts`. 
- User runs `agentctl manage` to open the OpenCode web UI for this project in the browser. The web UI shows running and previous sessions.
- User can stop active workflows from the web UI.

## References

The following files are `code2prompt` exports and should be used as implementation references.

Most architecture and behavior questions in this plan are already answered in `docs/references/agentcontainment.md`.
Default approach: search that reference first and reuse those patterns unless there is a clear reason to diverge.

```
docs/references/
├── agentcontainment.md       Old Rust project that is ~95% complete; reuse proven patterns.
├── agentctlv1.md 			  Last project attempt at this. Closer to this approach. 
├── example-js-workflow.md    Example JavaScript workflow, used as a baseline workflow.
├── opencode-docs.md          OpenCode full documentation.
├── opencode-flow-attempt.md  Previous Typescript unsuccessful attempt at making OpenCode plugin, but contains code worth referencing.
├── opencode.md               OpenCode codebase export (very large).
├── opencode-sdk.md           JavaScript OpenCode REST API SDK reference (we can translate to Rust for what we need).
├── plugins.mdx               OpenCode plugins documentation.
└── schema.md                 Slimmed Rust schema from agentcontainment project.
```

## Kit Folder

After `agentctl init`, the framework scaffold is written into the repository as `.agents`, plus `.opencode/opencode.json` for project-local OpenCode configuration.

```
.agents
├── .agentctl                             Internal framework state (gitignored)
│   ├── database.db                       SQLite database
│   ├── docker                            Container runtime support
│   │   ├── agent.docker-compose.yaml
│   │   └── Dockerfile
│   └── lib                   		      Compiled JS runtime helpers and orchestration
├── .worktrees                            Git worktrees (gitignored)
├── context                               Prompts and context assets
│   ├── prompts
│   ├── roles
│   ├── skills
│   └── templates
├── .gitignore
├── README.md
├── settings.json                         Project-level framework settings
├── tickets                               Agent task tickets
└── workflows                             User-authored TypeScript workflows
```

## CLI Commands

### Entrypoint

Usage: `agentctl <COMMAND>`

Commands:
- `init` Initialize agentctl in current directory
- `work` Start a workflow run
- `manage` Open the OpenCode web UI for this project in the browser
- `make` Generate project artifacts
- `help` Print this message or the help of the given subcommand(s)

Options:
- `-h, --help` Print help

### Work

Usage: `agentctl work <NAME>`

Options:
- `-e, --env` Runtime target: `host` or `container` (default: `host`)

### Make

Usage: `agentctl make <COMMAND>`

Commands:
- `ticket` Create a new ticket
- `worktree` Create a git worktree, ticket under .agents/.worktrees on host
- `help` Print this message or the help of the given subcommand(s)

## Must Haves

- Agent CLI for setting up and operating the agentctl framework.
- Workflows authored in Javascript with support for complex, multi-step agent orchestration.
- Workflow runtime support for both host and container environments.
  - OpenCode dangerous mode uses `--yolo` with workflows running in containers.
  - This should be configurable via env choice (`host` vs `container`).
- Tracking of running and previous workflows via SQLite.
- Ability to start and stop workflows.
- Workflow helper APIs (agent, git, tickets, etc.).
- TypeScript/JavaScript workflow linting and validation support.
- Workflows can invoke agents through OpenCode SDK sessions.
- `.agents/settings.json` handles global settings (for example git username/email for automated git operations).
- Per-project drop-in architecture.
- `agentctl manage` opens the OpenCode web UI for the current project in the browser.
- Managed Rust <-> Node IPC protocol for workflow execution events and control messages.

## Future Exploratory Goals

- Long-term agent memory using SQLite.
