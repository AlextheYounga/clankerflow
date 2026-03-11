# Project Todo

Legend: `[x]` done, `[ ]` not started, `[~]` partially done / stub exists.

## Done

- [x] `src/core/embeds.rs` — `copy_kit()` and `place_gitignore()` via `rust-embed`
- [x] `src/core/settings.rs` — `Settings` struct with `GitConfig`, `WorkflowConfig`, `OpencodeConfig`; load/save JSON
- [x] `src/core/tickets.rs` — Ticket creation with auto-increment `T-NNN`, template rendering, branch/title substitution
- [x] `src/core/project.rs` — Walk-up project root discovery (`.agents/` marker)
- [x] `agentctl init` — Fresh init + re-init (preserves user files), codebase_id stamping, DB migration
- [x] `agentctl work <name>` — Workflow file resolution (`.js`/`.ts`), path safety validation
- [x] `agentctl make ticket` — Create ticket via `core::tickets`
- [x] `agentctl make worktree <branch>` — Branch validation, `git worktree add`, paired ticket creation
- [x] `agentctl manage` — Opens OpenCode web UI in default browser via `open` crate
- [x] URL built from base64-encoded project path (`http://{server}/{base64(path)}/sessions`)
- [x] `codebase_id` changed from random hex to base64-encoded project path
- [x] `OpencodeSettings` struct with optional `server_url` (default `http://127.0.0.1:4096`)
- [x] `settings.json` template includes `opencode` block
- [x] `agentctl work` drives the entire lifecycle in a single foreground process (no daemon)
- [x] `src/core/runner.rs` — `WorkflowRunner::run()` spawns Node, runs IPC loop, returns final status
- [x] `src/core/runner/protocol.rs` — `parse_capability_request_payload`, `write_message`, `send_cancel`, `send_shutdown`
- [x] `src/core/runner/store.rs` — `upsert_workflow`, `create_run`, `set_status`, `append_run_event`
- [x] `src/core/ipc.rs` — `IpcMessage` struct, `command()`, `response()`, `error_response()` constructors
- [x] `src/core/runtime.rs` — Node binary resolution (`AGENTCTL_NODE_BIN` env override, bundled fallback)
- [x] `src/core/capabilities.rs` — Domain-prefix routing for capability requests
- [x] First Ctrl+C sends `cancel_run` to Node over IPC, grace period, second Ctrl+C force-kills
- [x] `Stdio::inherit()` — Node stdout/stderr appear in user's terminal
- [x] Unix socketpair IPC with fd 3, newline-delimited JSON frames
- [x] `build.rs` bundles `runtime/src/runner.ts` and `runtime/src/helpers.ts` via esbuild
- [x] `runtime/src/runner.ts` — Runner class with start/cancel/shutdown command handling
- [x] `runtime/src/ipc.ts` — `IpcTransport` + `IpcRouter` with request/response correlation and abort
- [x] `runtime/src/context.ts` — `createContext()` with agent, exec, log, sleep, signal
- [x] `runtime/src/protocol.ts` — v1 message format and parser
- [x] `runtime/src/loader.ts` — Dynamic workflow module import with meta/default validation
- [x] `runtime/src/helpers.ts` — `sleepWithSignal`, `runExec`, re-exports (fs, git, tickets)
- [x] `runtime/src/helpers/fs.ts` — Sandboxed filesystem context with path escape prevention
- [x] `runtime/src/helpers/git.ts` — Git operations via `simple-git`
- [x] `runtime/src/helpers/tickets/` — Full ticket CRUD (parse, scan, lookup, update, comment)
- [x] DB migrations: workflows, workflow_runs, workflow_sessions, events
- [x] DB entities: `workflow`, `workflow_run`, `workflow_session`, `event`
- [x] Rust tests across 14 modules (63+ tests total)
- [x] Node tests: runner, loader, context, fs, tickets
- [x] `src/kit/opencode.json` — minimal OpenCode config template (`$schema`, `model`)
- [x] `src/core/embeds.rs` — `place_opencode_config(project_root)` — idempotent, skips if already exists
- [x] `src/app/commands/init.rs` — calls `place_opencode_config` after `copy_kit`
- [x] `src/core/opencode/client.rs` — `OpencodeClient` wrapping `reqwest`: `create_session`, `chat`, `messages`, `abort`
- [x] `src/core/opencode.rs` — session capability handlers wired to real client (`session_run`, `session_messages_list`, `session_cancel`, `session_events_subscribe`)
- [x] `opencode_session_id` written to `workflow_sessions` table when `session_run` creates a session
- [x] SSE event stream subscribed and relayed to Node as IPC `event` messages without blocking the IPC loop
- [x] `src/core/docker.rs` — `Docker` struct: `is_available`, `build`, `up`, `down`, `is_running`, `get_container_id`, `ensure_running` (all async via `tokio::process::Command`)
- [x] `src/core/runner/env.rs` — `spawn_host_runner` and `spawn_container_runner`; container path calls `Docker::ensure_running` then `docker exec`
- [x] Socketpair/fd-3 IPC replaced with TCP: Rust binds `127.0.0.1:0`, passes port via `AGENTCTL_IPC_PORT`, Node connects back
- [x] `runtime/src/ipc.ts` — `IpcTransport` rewritten to use `net.createConnection`; container mode connects to `host.docker.internal`; `"error"` handler logs connection failures; `AGENTCTL_IPC_PORT` guarded with clear exit on missing
- [x] `src/app/cli.rs` — `--containment` flag (shorthand for `--env container --yolo`); clap-level `conflicts_with = "yolo"`; runtime guard for `--env container`
- [x] `agentctl containment up/down` subcommands — `src/app/commands/containment.rs`
- [x] `agent.docker-compose.yaml` — `extra_hosts: host.docker.internal:host-gateway` for Linux container-to-host TCP
- [x] `listener.accept()` wrapped with 30-second timeout and actionable error message
- [x] Node tests: `runtime/tests/ipc.test.ts` — TCP transport, line framing, disconnect handling

## Remaining

- [ ] Revisit duplicate `agent_session_started` handling and decide whether Node guarantees single emission or Rust persists sessions idempotently
- [ ] Background/daemon execution (design in `docs/future/daemon.md`)
- [ ] Long-term agent memory via SQLite
- [ ] Bundled Node runtime distribution for releases
