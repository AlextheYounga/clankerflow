# Phase 5: OpenCode Client Integration + `.opencode/opencode.json` Scaffold

## Goal

Two things:

1. Wire the real OpenCode REST API into `src/core/opencode.rs`, replacing all
   stubs with working HTTP calls via `ureq`.
2. Have `agentctl init` emit a boilerplate `.opencode/opencode.json` for
   project-local OpenCode configuration.

## `.opencode/opencode.json`

OpenCode reads project-local config from `.opencode/opencode.json` (among other
locations). `agentctl init` should place one there if it doesn't already exist.

A minimal template:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "anthropic/claude-sonnet-4-5"
}
```

### Implementation

- Add `opencode.json` as an embedded file at `src/kit/.opencode/opencode.json`.
- In `src/core/embeds.rs`, add a `place_opencode_config(project_root)` function
  that writes `.opencode/opencode.json` to the project root. Skip if it already
  exists (idempotent — same pattern as `place_gitignore`).
- Call `place_opencode_config` from `src/app/commands/init.rs` after `copy_kit`.
- Re-init is also idempotent: if `.opencode/opencode.json` already exists, leave
  it alone.

## OpenCode HTTP Client

### `src/core/opencode_client.rs` (new file)

A thin synchronous HTTP client wrapping `ureq`. Reads the server URL from
`OpencodeSettings::server_url` (defaulting to `http://127.0.0.1:4096`).

Required operations (matching the existing capability stubs):

| Capability | HTTP call |
|---|---|
| `session_run` | `POST /session` to create a session, then `POST /session/{id}/message` with the prompt |
| `session_messages_list` | `GET /session/{id}/message` |
| `session_cancel` | `POST /session/{id}/abort` |
| `session_events_subscribe` | `GET /event` (SSE stream — see note below) |

The client struct:

```rust
pub struct OpencodeClient {
    base_url: String,
}

impl OpencodeClient {
    pub fn new(base_url: &str) -> Self { ... }
    pub fn create_session(&self) -> Result<Session> { ... }
    pub fn chat(&self, session_id: &str, prompt: &str) -> Result<AssistantMessage> { ... }
    pub fn messages(&self, session_id: &str) -> Result<Vec<Message>> { ... }
    pub fn abort(&self, session_id: &str) -> Result<()> { ... }
}
```

Keep response types as plain structs with only the fields we need — no need to
model the full SDK surface.

### SSE / `session_events_subscribe`

The OpenCode `/event` endpoint is an SSE stream. `ureq` supports streaming
responses. For this phase, `session_events_subscribe` should open the stream and
return a subscription handle that Node can poll via subsequent IPC requests, or
alternatively, Rust can relay individual events back to Node as IPC `event`
messages. Exact relay design to be decided during implementation — the key
constraint is no blocking the IPC loop.

### Wiring into `src/core/opencode.rs`

Replace each stub with a call to `OpencodeClient`:

- `session_run` — create session + chat, return `{ session_id, message_id }`.
- `session_messages_list` — return the message array.
- `session_cancel` — abort and return `{ session_id }`.
- `session_events_subscribe` — open event stream (design TBD).

The client is constructed per-call using the server URL from settings, passed
in as a param or resolved from the project root.

### `workflow_sessions` table

Once `session_run` creates a real OpenCode session, write the `opencode_session_id`
into the `workflow_sessions` table so runs can be correlated with sessions in the
web UI.

## Tests

All tests must be offline — no real OpenCode server.

- `src/core/opencode_client.rs` — mock `ureq` responses or use a local HTTP
  test server (e.g. `mockito` crate) to cover: session create, chat, messages
  list, abort.
- `src/core/opencode.rs` — update existing stubs tests to assert real return
  shapes once the stubs are replaced.
- `src/core/embeds.rs` — extend existing embed tests to assert
  `.opencode/opencode.json` is written on fresh init and skipped if already
  present.
- `src/app/commands/init.rs` — assert `.opencode/opencode.json` exists after
  `run()`.

## Files Added

- `src/kit/.opencode/opencode.json`
- `src/core/opencode_client.rs`

## Files Changed

- `src/core/embeds.rs` — add `place_opencode_config`.
- `src/core/opencode.rs` — replace stubs with `OpencodeClient` calls.
- `src/app/commands/init.rs` — call `place_opencode_config`.
- `src/core.rs` — export `opencode_client` module.
- `Cargo.toml` — add `mockito` to `[dev-dependencies]` for HTTP mocking in tests.

## Out of Scope

- Container lifecycle management.
- Full SSE event relay design (deferred to a follow-up phase).
