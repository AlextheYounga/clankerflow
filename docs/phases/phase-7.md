# Phase 7: Move OpenCode to Node SDK

## Goal

Move OpenCode HTTP/session operations out of Rust and into the workflow Node
runtime using `@opencode-ai/sdk`, while keeping Rust responsible for workflow
orchestration, process lifecycle, cancellation, SQLite persistence, and
container management.

## Brief Intent Summary

Phase 7 is a hard cutover from Rust-side OpenCode capability dispatch to direct
Node SDK usage behind `tools.agent`.

- Workflow syntax in `src/kit/workflows/` does not change.
- `tools.agent` remains a set of wrapper functions; workflows do not receive the
  raw SDK client directly.
- Rust stops making OpenCode HTTP calls.
- Rust keeps ownership of workflow runs, DB state, cancellation, and
  container/runner lifecycle.
- `.agents/settings.json` becomes shared config that both Rust and Node may read.
- `codebase_id` is no longer stored as user-editable config; Rust derives it
  from `project_root` when needed.

## Why This Phase

The current architecture makes Rust the OpenCode client through a capability
bridge:

- Node sends IPC `request` messages for `session_run`,
  `session_messages_list`, `session_cancel`, and `session_events_subscribe`.
- Rust dispatches those requests in `src/core/capabilities.rs` and
  `src/core/opencode.rs`.
- Rust performs the OpenCode HTTP calls in `src/core/opencode/client.rs`.

That design is now the wrong boundary.

We want OpenCode behavior to live where workflow authoring lives: in the Node
runtime. That gives `tools.agent` direct access to the real SDK surface and
removes an unnecessary Rust transport layer, while still preserving Rust as the
system orchestrator.

## Design Decisions

### 1) Hard cutover in this phase

This phase deletes the Rust OpenCode path rather than leaving compatibility
shims behind.

Remove:

- `src/core/opencode.rs`
- `src/core/opencode/client.rs`
- `src/core/capabilities.rs`
- the IPC capability dispatch path in `src/core/runner/ipc_loop.rs`
- capability request/response protocol types that only existed for agent
  capability IPC

The runtime no longer sends OpenCode capability requests to Rust.

### 2) `tools.agent` stays as wrapper functions

Do not expose the raw `@opencode-ai/sdk` client directly to workflows.

Keep `tools.agent` as the workflow-facing API surface, but reimplement it as a
thin Node wrapper over the SDK.

This preserves workflow ergonomics and lets us control the public API shape
without coupling workflows to the SDK directly.

### 3) Shared settings file, no Rust-owned settings schema

`.agents/settings.json` remains the project config file, but it is no longer a
Rust-owned configuration boundary.

Both runtimes may read it directly.

- Node reads it for OpenCode configuration.
- Rust may still read it where convenient, such as `agentctl manage`.

This is acceptable because the file is simple JSON and does not require a
cross-runtime config service.

### 4) `codebase_id` becomes derived runtime metadata

`codebase_id` should not remain in `.agents/settings.json`.

Instead:

- remove it from the settings schema and scaffolded file
- derive it from `project_root` anywhere Rust needs it
- keep the derivation consistent with the OpenCode web UI path encoding already
  used by `manage`

This makes `codebase_id` non-editable and removes a runtime identity concern
from user config.

### 5) Session persistence stays Rust-owned via explicit events

Today Rust persists workflow/OpenCode session correlation by intercepting the
Rust capability response for `session_run`.

Once Node calls the SDK directly, that interception disappears. Replace it with
an explicit IPC event emitted by Node when a session is created or first bound
to a workflow run.

Proposed event:

```json
{
  "name": "agent_session_started",
  "payload": {
    "run_id": 123,
    "session_id": "sess_abc"
  }
}
```

Rust handles that event and calls `create_workflow_session()`.

Duplicate event handling is deferred for now and should be revisited during
implementation if the event can be emitted more than once for the same
run/session pair.

### 6) Node owns OpenCode URL resolution for workflow runs

Since OpenCode HTTP moves to Node, Node should load `opencode.server_url`
directly from `.agents/settings.json`.

Rust no longer needs to forward the OpenCode URL into workflow runs.

For container execution, Node should normalize local loopback URLs so the SDK
can reach the host OpenCode server from inside the container:

- `http://127.0.0.1:4096` -> `http://host.docker.internal:4096`
- `http://localhost:4096` -> `http://host.docker.internal:4096`

Only the Node OpenCode call site changes; the container workflow model stays
the same.

## Scope

### In scope

- Replace Rust-side OpenCode HTTP calls with Node SDK calls.
- Expand `tools.agent` to wrap the relevant OpenCode SDK session operations.
- Keep existing workflow calls working.
- Remove the Rust capability/OpenCode bridge.
- Move OpenCode config consumption for workflow runs into Node.
- Remove `codebase_id` from user settings and derive it in Rust.
- Add the IPC event needed for session persistence.

### Out of scope

- Rewriting workflow examples in `src/kit/workflows/`.
- Changing Rust ownership of workflow orchestration, DB, cancellation, or Docker.
- Exposing the raw SDK client directly to workflow authors.
- Broad config-system redesign beyond removing stored `codebase_id`.

## Runtime API Direction

The exact wrapper surface can evolve during implementation, but the intent is:

- `agent.run(...)` remains the primary compatibility entrypoint.
- `agent.run(...)` preserves the current workflow-facing result shape for now,
  including `ok`, `output`, and `error`, with SDK responses adapted into that
  shape rather than redefining the workflow contract in this phase.
- `agent.messages(sessionId)` becomes a direct SDK-backed call.
- `agent.events(sessionId)` becomes a direct SDK-backed call.
- `agent.cancel(sessionId)` becomes a direct SDK-backed call.

As needed, Phase 7 may also add more wrapper methods that map cleanly to SDK
session behavior, but the runtime should stay intentionally small and explicit.

## Rust Changes

### Remove OpenCode/capability code

Delete:

- `src/core/opencode.rs`
- `src/core/opencode/client.rs`
- `src/core/capabilities.rs`

Update:

- `src/core/runner/ipc_loop.rs`
  - remove the `"request"` arm used for capability dispatch
  - handle `agent_session_started` events and persist workflow session rows
- `src/core/runner.rs`
  - stop loading `opencode.server_url` for workflow execution
  - keep normal run setup/orchestration
  - send the real DB `run_id` to Node in `start_run` instead of `0`
- `src/app/commands/work.rs`
  - stop depending on stored `settings.codebase_id`
  - derive `codebase_id` from the project root instead
- `src/app/commands/containment.rs`
  - derive `codebase_id` instead of reading it from settings
- `src/app/commands/manage.rs`
  - continue reading `.agents/settings.json` directly for `opencode.server_url`

### Derive `codebase_id`

Add a small shared Rust helper for project ID derivation and reuse it in:

- workflow runs
- containment commands
- manage URL construction if appropriate

The result must stay aligned with the existing base64-no-padding behavior.

## Node Runtime Changes

### `runtime/src/tools/agent.ts`

Replace capability IPC calls with SDK-backed wrapper functions.

Responsibilities:

- create/configure the SDK client from `.agents/settings.json`
- preserve `agent.run()` compatibility
- expose wrapper methods for messages/events/cancel
- emit `agent_session_started` once a workflow run is associated with a session

### `runtime/src/runner.ts`

- stop constructing `tools.agent` from `invokeCapability`
- pass the current `run_id`, runtime environment, and any event emitter hooks
  required by the new agent wrapper
- continue to emit normal workflow lifecycle events to Rust

### `runtime/src/protocol.ts`

Remove types that only existed for capability request/response payloads.

### Settings loading in Node

Add a small runtime-side settings reader for `.agents/settings.json`.

Node uses it to:

- resolve `opencode.server_url`
- apply container host rewriting when necessary

## Kit / Distributed Runtime Changes

The runtime source of truth remains `runtime/src/*`.

`build.rs` already copies `runtime/src` into `src/kit/.agentkata/lib/src`, so
Phase 7 must account for the remaining non-copied artifacts as well.

Update:

- `src/kit/.agentkata/lib/package.json`
- `src/kit/.agentkata/lib/types/agentkata.d.ts`

Also update any embedded runtime behavior that is not sourced from
`runtime/src/*`.

## Settings Changes

### `.agents/settings.json`

Remove:

- `codebase_id`

Keep as shared config:

- `git`
- `workflows`
- `opencode`

The Phase 7 intent is not to redesign the full config file, only to remove the
derived runtime ID from user-editable settings and move OpenCode runtime usage
to Node.

## Testing Plan

All tests remain offline.

### Rust

- `src/core/runner/ipc_loop.rs`
  - persists workflow sessions when `agent_session_started` is received
  - no longer depends on capability request dispatch
- `src/core/runner.rs`
  - sends the real `run_id` in `start_run`
- project ID derivation helper
  - produces the expected base64-no-padding ID from project paths
- command handlers
  - `work` and `containment` no longer require `codebase_id` in settings
- `manage`
  - still builds the correct URL from `opencode.server_url`

### Node

- `runtime/tests/*`
  - `tools.agent.run()` preserves compatibility
  - `tools.agent.messages()` uses SDK-backed behavior
  - `tools.agent.events()` uses SDK-backed behavior
  - `tools.agent.cancel()` uses SDK-backed behavior
  - settings loading resolves server URL correctly
  - container mode rewrites loopback OpenCode hosts correctly
  - session-start event emission occurs when a session is established

## Files Expected to Change

### Rust

- `src/app/commands/work.rs`
- `src/app/commands/containment.rs`
- `src/app/commands/manage.rs`
- `src/core/runner.rs`
- `src/core/runner/ipc_loop.rs`
- `src/core/settings.rs`
- new shared helper for project ID derivation

### Rust deleted

- `src/core/opencode.rs`
- `src/core/opencode/client.rs`
- `src/core/capabilities.rs`

### Node runtime

- `runtime/src/tools/agent.ts`
- `runtime/src/runner.ts`
- `runtime/src/protocol.ts`
- runtime tests
- `runtime/package.json`

### Embedded kit runtime

- `src/kit/.agentkata/lib/package.json`
- `src/kit/.agentkata/lib/types/agentkata.d.ts`
- any generated/copied runtime files affected via `build.rs`
- `src/kit/settings.json`

## Acceptance Criteria

- Workflow authors continue using `tools.agent` rather than a raw SDK client.
- OpenCode HTTP calls for workflow runs happen in Node, not Rust.
- `agent.run()`, `agent.messages()`, `agent.events()`, and `agent.cancel()` are
  SDK-backed.
- Existing kit workflows continue to work without modification.
- Rust still owns workflow run lifecycle, DB state, cancellation, and container
  management.
- `workflow_sessions` rows are still created for OpenCode session correlation.
- `start_run` sends the real workflow run id to Node rather than `0`.
- `codebase_id` is no longer stored in `.agents/settings.json`.
- Tests cover the new session event path and Node-side settings/OpenCode logic.
