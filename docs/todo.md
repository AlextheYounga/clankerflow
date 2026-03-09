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
- [x] `src/core/runner.rs` — `run_workflow()` spawns Node, runs IPC loop, returns final status
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

## Remaining

- [~] `src/core/opencode.rs` — Session capability handlers exist but return stub errors
- [ ] Create `src/kit/.opencode/opencode.json` with minimal template (`$schema`, `model`)
- [ ] `src/core/embeds.rs` — Add `place_opencode_config(project_root)` function
- [ ] `src/app/commands/init.rs` — Call `place_opencode_config` after `copy_kit`
- [ ] Idempotent: skip if `.opencode/opencode.json` already exists
- [ ] Tests: assert `.opencode/opencode.json` written on fresh init, skipped if present
- [ ] `src/core/opencode_client.rs` — New file: `OpencodeClient` struct wrapping `ureq`
- [ ] `OpencodeClient::create_session()` — `POST /session`
- [ ] `OpencodeClient::chat(session_id, prompt)` — `POST /session/{id}/message`
- [ ] `OpencodeClient::messages(session_id)` — `GET /session/{id}/message`
- [ ] `OpencodeClient::abort(session_id)` — `POST /session/{id}/abort`
- [ ] Response types as plain structs (only fields we need)
- [ ] `src/core.rs` — Export `opencode_client` module
- [ ] `session_events_subscribe` — Open SSE stream on `GET /event`
- [ ] Design event relay to Node (relay as IPC `event` messages, or subscription polling — TBD)
- [ ] SSE relay must not block the IPC loop
- [ ] Replace `session_run` stub with `create_session` + `chat` calls
- [ ] Replace `session_messages_list` stub with `messages` call
- [ ] Replace `session_cancel` stub with `abort` call
- [ ] Replace `session_events_subscribe` stub with SSE stream
- [ ] Write `opencode_session_id` to `workflow_sessions` table when `session_run` creates a session
- [ ] `src/core/opencode_client.rs` — Mock HTTP tests via `mockito`
- [ ] `src/core/opencode.rs` — Update stub tests to assert real return shapes
- [ ] `src/core/embeds.rs` — Extend embed tests for `.opencode/opencode.json`
- [ ] `src/app/commands/init.rs` — Assert `.opencode/opencode.json` exists after `run()`
- [ ] `Cargo.toml` — Add `mockito` to `[dev-dependencies]`
- [ ] Container lifecycle management (`--env container`, Docker Compose lifecycle)
- [ ] Background/daemon execution (design in `docs/future/daemon.md`)
- [ ] Long-term agent memory via SQLite
- [ ] Bundled Node runtime distribution for releases
