use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::core::capabilities::require_str;

/// Session capability dispatcher.
///
/// OpenCode client integration is not yet implemented (Phase 5).
/// These stubs return structured errors so the workflow gets a clear failure
/// rather than a silent hang.
pub fn dispatch(capability: &str, params: &Value) -> Result<Value> {
    match capability {
        "session_run" => run(params),
        "session_messages_list" => messages_list(params),
        "session_cancel" => cancel(params),
        "session_events_subscribe" => events_subscribe(params),
        _ => Err(anyhow!("unknown session capability: {capability}")),
    }
}

fn run(params: &Value) -> Result<Value> {
    let _prompt = require_str(params, "prompt", "session_run")?;
    // Phase 5: call OpenCodeClient::create_session + chat
    Err(anyhow!("session_run: OpenCode client not yet implemented"))
}

fn messages_list(params: &Value) -> Result<Value> {
    let session_id = require_str(params, "session_id", "session_messages_list")?;
    // Phase 5: call OpenCodeClient::list_messages
    Err(anyhow!(
        "session_messages_list: OpenCode client not yet implemented (session: {session_id})"
    ))
}

fn cancel(params: &Value) -> Result<Value> {
    let session_id = require_str(params, "session_id", "session_cancel")?;
    // Phase 5: call OpenCodeClient::abort_session
    Err(anyhow!(
        "session_cancel: OpenCode client not yet implemented (session: {session_id})"
    ))
}

fn events_subscribe(params: &Value) -> Result<Value> {
    let session_id = require_str(params, "session_id", "session_events_subscribe")?;
    // Phase 5: wire to SSE event stream
    Ok(json!({
        "subscription_id": format!("sub_{session_id}"),
        "session_id": session_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_session_capability_returns_error() {
        let err = dispatch("session_teleport", &serde_json::json!({})).unwrap_err();
        assert!(err.to_string().contains("unknown session capability"));
    }

    #[test]
    fn session_run_requires_prompt_param() {
        let err = dispatch("session_run", &serde_json::json!({})).unwrap_err();
        assert!(err.to_string().contains("prompt"));
    }

    #[test]
    fn events_subscribe_returns_subscription_ack() {
        let params = serde_json::json!({ "session_id": "sess_abc" });
        let result = dispatch("session_events_subscribe", &params).unwrap();
        assert_eq!(result["session_id"], "sess_abc");
        assert!(
            result["subscription_id"]
                .as_str()
                .unwrap()
                .starts_with("sub_")
        );
    }
}
