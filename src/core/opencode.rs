mod client;

use anyhow::{Result, anyhow};
use client::Client;
use serde_json::{Value, json};

use crate::core::capabilities::require_str;

/// Session capability dispatcher.
///
/// Delegates to the `OpenCode` HTTP client for session lifecycle operations.
/// SSE event streaming (`session_events_subscribe`) returns a stub acknowledgment;
/// full relay design is deferred to a follow-up phase.
///
/// # Errors
/// Returns an error for unknown capabilities, missing parameters, or HTTP failures.
pub fn dispatch(capability: &str, params: &Value, server_url: &str) -> Result<Value> {
    match capability {
        "session_run" => run(params, server_url),
        "session_messages_list" => messages_list(params, server_url),
        "session_cancel" => cancel(params, server_url),
        "session_events_subscribe" => events_subscribe(params),
        _ => Err(anyhow!("unknown session capability: {capability}")),
    }
}

fn run(params: &Value, server_url: &str) -> Result<Value> {
    let prompt = require_str(params, "prompt", "session_run")?;
    let client = Client::new(server_url);

    let session = client.create_session()?;
    let message = client.chat(&session.id, prompt)?;

    Ok(json!({
        "session_id": session.id,
        "message_id": message.id,
    }))
}

fn messages_list(params: &Value, server_url: &str) -> Result<Value> {
    let session_id = require_str(params, "session_id", "session_messages_list")?;
    let client = Client::new(server_url);

    let entries = client.messages(session_id)?;
    let messages: Vec<Value> = entries
        .iter()
        .map(|entry| {
            json!({
                "id": entry.info.id,
                "role": entry.info.role,
            })
        })
        .collect();

    Ok(json!({ "messages": messages }))
}

fn cancel(params: &Value, server_url: &str) -> Result<Value> {
    let session_id = require_str(params, "session_id", "session_cancel")?;
    let client = Client::new(server_url);

    client.abort(session_id)?;

    Ok(json!({ "session_id": session_id }))
}

fn events_subscribe(params: &Value) -> Result<Value> {
    let session_id = require_str(params, "session_id", "session_events_subscribe")?;
    // SSE relay design deferred — return a stub acknowledgment.
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
        let err = dispatch("session_teleport", &json!({}), "http://unused").unwrap_err();
        assert!(err.to_string().contains("unknown session capability"));
    }

    #[test]
    fn session_run_requires_prompt_param() {
        let err = dispatch("session_run", &json!({}), "http://unused").unwrap_err();
        assert!(err.to_string().contains("prompt"));
    }

    #[test]
    fn events_subscribe_returns_subscription_ack() {
        let params = json!({ "session_id": "sess_abc" });
        let result = dispatch("session_events_subscribe", &params, "http://unused").unwrap();
        assert_eq!(result["session_id"], "sess_abc");
        assert!(
            result["subscription_id"]
                .as_str()
                .unwrap_or("")
                .starts_with("sub_")
        );
    }

    #[test]
    fn session_run_creates_session_and_chats() {
        let mut server = mockito::Server::new();
        let _session_mock = server
            .mock("POST", "/session")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id": "sess_new"}"#)
            .create();
        let _chat_mock = server
            .mock("POST", "/session/sess_new/message")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id": "msg_1", "sessionID": "sess_new", "role": "assistant"}"#)
            .create();

        let params = json!({ "prompt": "Do something" });
        let result = dispatch("session_run", &params, &server.url()).unwrap();

        assert_eq!(result["session_id"], "sess_new");
        assert_eq!(result["message_id"], "msg_1");
    }

    #[test]
    fn session_messages_list_returns_messages() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/session/sess_1/message")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"info": {"id": "m1", "role": "user"}}, {"info": {"id": "m2", "role": "assistant"}}]"#)
            .create();

        let params = json!({ "session_id": "sess_1" });
        let result = dispatch("session_messages_list", &params, &server.url()).unwrap();

        let messages = result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[1]["role"], "assistant");
    }

    #[test]
    fn session_cancel_aborts_session() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/session/sess_1/abort")
            .with_status(200)
            .with_body("true")
            .create();

        let params = json!({ "session_id": "sess_1" });
        let result = dispatch("session_cancel", &params, &server.url()).unwrap();

        assert_eq!(result["session_id"], "sess_1");
        mock.assert();
    }
}
