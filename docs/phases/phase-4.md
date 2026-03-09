# Phase 4: Manage Pivot — OpenCode Web UI

## Decision

`agentctl manage` will not implement a Ratatui TUI. OpenCode already ships a
web UI at `http://127.0.0.1:4096` that surfaces sessions, events, and messages
for a given project. We leverage that instead of building a parallel monitoring
surface.

## How It Works

OpenCode identifies a project by the base64-encoded (no padding) absolute path
of the project root. The URL structure is:

```
http://{server}/{base64(project_root)}/sessions
```

For example, `/home/alex/Work/rust/agentctl` encodes to
`L2hvbWUvYWxleC9Xb3JrL3J1c3QvYWdlbnRjdGw`, giving:

```
http://127.0.0.1:4096/L2hvbWUvYWxleC9Xb3JrL3J1c3QvYWdlbnRjdGw/sessions
```

`agentctl manage` constructs this URL from the project root and opens it in the
default browser via the `open` crate.

The server address defaults to `http://127.0.0.1:4096` and can be overridden
per-project in `.agents/settings.json`:

```json
{
  "opencode": {
    "server_url": "http://127.0.0.1:4096"
  }
}
```

## codebase_id

`codebase_id` in `settings.json` was previously a random hex string (timestamp
+ PID hash). It is now the base64-encoded project path — the same value OpenCode
uses as its project identifier — making them consistent without any coordination
overhead.

The field is still preserved on re-init: if a non-empty `codebase_id` already
exists, it is left unchanged.

## Files Changed

- `src/app/commands/manage.rs` — replaced DB-connect + TUI stub with
  `build_manage_url` + `open::that`.
- `src/app/commands/init.rs` — `codebase_id_for` now base64-encodes the project
  path instead of hashing time + PID.
- `src/core/settings.rs` — added `OpencodeSettings` struct with optional
  `server_url`; `Settings.opencode` is `Option<OpencodeSettings>` so existing
  settings files without the field deserialize cleanly.
- `src/kit/settings.json` — template now includes the `opencode` block.
- `Cargo.toml` — added `base64` and `open` crates; removed `ratatui`.
