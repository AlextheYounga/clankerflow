use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub v: String,
    pub id: String,
    pub kind: String,
    pub name: String,
    pub payload: Value,
}

impl Message {
    pub fn command(id: impl Into<String>, name: impl Into<String>, payload: Value) -> Self {
        Self {
            v: "v1".to_string(),
            id: id.into(),
            kind: "command".to_string(),
            name: name.into(),
            payload,
        }
    }

    pub fn response(
        request_id: impl Into<String>,
        name: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            v: "v1".to_string(),
            id: request_id.into(),
            kind: "response".to_string(),
            name: name.into(),
            payload,
        }
    }

    pub fn error_response(
        request_id: impl Into<String>,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            v: "v1".to_string(),
            id: request_id.into(),
            kind: "error".to_string(),
            name: name.into(),
            payload: serde_json::json!({ "error": message.into() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_constructor_sets_correct_fields() {
        let msg = Message::command(
            "cmd_1",
            "start_run",
            serde_json::json!({ "workflow_path": "/tmp/duos.js" }),
        );

        assert_eq!(msg.v, "v1");
        assert_eq!(msg.id, "cmd_1");
        assert_eq!(msg.kind, "command");
        assert_eq!(msg.name, "start_run");
        assert_eq!(msg.payload["workflow_path"], "/tmp/duos.js");
    }

    #[test]
    fn response_constructor_echoes_request_id() {
        let msg = Message::response("req_abc", "capability_request", serde_json::json!({}));

        assert_eq!(msg.id, "req_abc");
        assert_eq!(msg.kind, "response");
        assert_eq!(msg.name, "capability_request");
    }

    #[test]
    fn error_response_carries_error_message() {
        let msg = Message::error_response("req_xyz", "capability_request", "something failed");

        assert_eq!(msg.id, "req_xyz");
        assert_eq!(msg.kind, "error");
        assert_eq!(msg.payload["error"], "something failed");
    }

    #[test]
    fn round_trips_through_json_line_without_newlines() {
        let original = Message::command(
            "cmd_42",
            "cancel_run",
            serde_json::json!({ "reason": "user_requested" }),
        );

        let line = serde_json::to_string(&original).expect("should serialize");
        assert!(!line.contains('\n'), "IPC line must not contain newlines");

        let parsed: Message = serde_json::from_str(&line).expect("should deserialize");
        assert_eq!(parsed.v, "v1");
        assert_eq!(parsed.id, "cmd_42");
        assert_eq!(parsed.payload["reason"], "user_requested");
    }

    #[test]
    fn ignores_unknown_fields_on_deserialize() {
        let json = r#"{
            "v": "v1",
            "id": "msg_1",
            "kind": "event",
            "name": "run_started",
            "payload": {},
            "future_field": "ignored"
        }"#;

        let msg: Message = serde_json::from_str(json).expect("unknown fields should be ignored");
        assert_eq!(msg.name, "run_started");
    }
}
