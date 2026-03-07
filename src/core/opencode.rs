use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::core::capabilities::require_str;

/// Agent capability dispatcher.
///
/// OpenCode client integration is not yet implemented (Phase 4).
/// These stubs return structured errors so the workflow gets a clear failure
/// rather than a silent hang.
pub fn dispatch(capability: &str, params: &Value) -> Result<Value> {
    match capability {
        "agent_run" => run(params),
        "agent_messages_list" => messages_list(params),
        "agent_cancel" => cancel(params),
        "agent_events_subscribe" => events_subscribe(params),
        _ => Err(anyhow!("unknown agent capability: {capability}")),
    }
}

fn run(params: &Value) -> Result<Value> {
    let _prompt = require_str(params, "prompt", "agent_run")?;
    // Phase 4: call OpenCodeClient::create_session + chat
    Err(anyhow!("agent_run: OpenCode client not yet implemented"))
}

fn messages_list(params: &Value) -> Result<Value> {
    let session_id = require_str(params, "session_id", "agent_messages_list")?;
    // Phase 4: call OpenCodeClient::list_messages
    Err(anyhow!(
        "agent_messages_list: OpenCode client not yet implemented (session: {session_id})"
    ))
}

fn cancel(params: &Value) -> Result<Value> {
    let session_id = require_str(params, "session_id", "agent_cancel")?;
    // Phase 4: call OpenCodeClient::abort_session
    Err(anyhow!(
        "agent_cancel: OpenCode client not yet implemented (session: {session_id})"
    ))
}

fn events_subscribe(params: &Value) -> Result<Value> {
    let session_id = require_str(params, "session_id", "agent_events_subscribe")?;
    // Phase 4: wire to SSE event stream
    Ok(json!({
        "subscription_id": format!("sub_{session_id}"),
        "session_id": session_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_agent_capability_returns_error() {
        let err = dispatch("agent_teleport", &serde_json::json!({})).unwrap_err();
        assert!(err.to_string().contains("unknown agent capability"));
    }

    #[test]
    fn agent_run_requires_prompt_param() {
        let err = dispatch("agent_run", &serde_json::json!({})).unwrap_err();
        assert!(err.to_string().contains("prompt"));
    }

    #[test]
    fn events_subscribe_returns_subscription_ack() {
        let params = serde_json::json!({ "session_id": "sess_abc" });
        let result = dispatch("agent_events_subscribe", &params).unwrap();
        assert_eq!(result["session_id"], "sess_abc");
        assert!(
            result["subscription_id"]
                .as_str()
                .unwrap()
                .starts_with("sub_")
        );
    }
}
