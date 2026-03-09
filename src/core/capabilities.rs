use crate::core::opencode;

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::core::ipc::Message;

pub struct CapabilityRequest<'a> {
    pub capability: &'a str,
    pub params: &'a Value,
}

/// # Errors
/// Returns an error if the required parameter is absent or not a string.
pub fn require_str<'a>(params: &'a Value, key: &str, capability: &str) -> Result<&'a str> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("capability '{capability}' missing required param: {key}"))
}

#[must_use]
pub fn dispatch(request_id: &str, request: &CapabilityRequest<'_>) -> Message {
    let domain = request
        .capability
        .split('_')
        .next()
        .unwrap_or(request.capability);

    let result = match domain {
        "session" => opencode::dispatch(request.capability, request.params),
        _ => Err(anyhow!("unknown capability domain: {domain}")),
    };

    match result {
        Ok(payload) => Message::response(request_id, "capability_response", payload),
        Err(error) => Message::error_response(request_id, "capability_response", error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_domain_returns_error_response() {
        let request = CapabilityRequest {
            capability: "bogus_thing",
            params: &serde_json::json!({}),
        };

        let response = dispatch("req_1", &request);

        assert_eq!(response.kind, "error");
        assert!(response.payload["error"]
            .as_str()
            .unwrap_or("")
            .contains("unknown capability domain"));
    }

    #[test]
    fn require_str_returns_value_when_present() {
        let params = serde_json::json!({ "prompt": "hello" });

        let value = require_str(&params, "prompt", "session_run").unwrap();

        assert_eq!(value, "hello");
    }

    #[test]
    fn require_str_errors_when_key_missing() {
        let params = serde_json::json!({});

        let err = require_str(&params, "prompt", "session_run").unwrap_err();

        assert!(err.to_string().contains("prompt"));
    }
}
