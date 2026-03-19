use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use opencode_sdk::Client;
use opencode_sdk::types::event::Event;
use opencode_sdk::types::message::{CommandRequest, Message, PromptPart, PromptRequest};
use opencode_sdk::types::project::ModelRef;
use opencode_sdk::types::session::CreateSessionRequest;
use serde_json::{Value, json};
use tokio::time::timeout;

pub mod server;

const RUN_TIMEOUT: Duration = Duration::from_secs(300);
const EVENTS_WINDOW: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct Gateway {
    client: Client,
}

impl Gateway {
    /// # Errors
    /// Returns an error if the `OpenCode` client cannot be constructed.
    pub fn from_project_root(project_root: &Path) -> Result<Self> {
        let client = Client::builder()
            .base_url(server::DEFAULT_BASE_URL)
            .directory(project_root.to_string_lossy().to_string())
            .build()
            .map_err(|error| anyhow!("failed to build OpenCode client: {error}"))?;

        Ok(Self { client })
    }

    /// # Errors
    /// Returns an error when request payload validation fails or when the
    /// `OpenCode` API call fails.
    pub async fn dispatch(&self, request_name: &str, payload: &Value) -> Result<Value> {
        match request_name {
            "opencode_run" => self.run(payload).await,
            "opencode_command" => self.command(payload).await,
            "opencode_messages" => self.messages(payload).await,
            "opencode_events" => self.events(payload).await,
            "opencode_cancel" => self.cancel(payload).await,
            _ => Err(anyhow!("unknown OpenCode request: {request_name}")),
        }
    }

    async fn run(&self, payload: &Value) -> Result<Value> {
        let prompt = require_prompt(payload)?;
        let create = CreateSessionRequest {
            title: read_trimmed_string(payload, "title"),
            ..Default::default()
        };

        let session = self.client.sessions().create(&create).await?;

        let request = PromptRequest {
            parts: vec![PromptPart::Text {
                text: prompt,
                synthetic: None,
                ignored: None,
                metadata: None,
            }],
            message_id: None,
            model: resolve_model(payload),
            agent: None,
            no_reply: None,
            system: read_string(payload, "system"),
            variant: None,
        };

        self.client
            .messages()
            .prompt_async(&session.id, &request)
            .await?;

        let output = self
            .client
            .wait_for_idle_text(&session.id, RUN_TIMEOUT)
            .await?;
        let messages = self.client.messages().list(&session.id).await?;
        let message_id = latest_assistant_message_id(&messages);

        Ok(json!({
            "session_id": session.id,
            "message_id": message_id,
            "output": output,
        }))
    }

    async fn messages(&self, payload: &Value) -> Result<Value> {
        let session_id = require_string(payload, "session_id", "opencode_messages")?;
        let messages = self.client.messages().list(session_id).await?;

        Ok(json!({
            "session_id": session_id,
            "messages": messages,
        }))
    }

    async fn command(&self, payload: &Value) -> Result<Value> {
        let session_id = require_string(payload, "session_id", "opencode_command")?;
        let command = require_command(payload)?;
        let args = read_optional_json(payload, "args");
        let request = CommandRequest { command, args };
        let response = self.client.messages().command(session_id, &request).await?;

        Ok(json!({
            "session_id": session_id,
            "response": response,
        }))
    }

    async fn cancel(&self, payload: &Value) -> Result<Value> {
        let session_id = require_string(payload, "session_id", "opencode_cancel")?;
        self.client.sessions().abort(session_id).await?;

        Ok(json!({
            "session_id": session_id,
            "result": true,
        }))
    }

    async fn events(&self, payload: &Value) -> Result<Value> {
        let session_id = require_string(payload, "session_id", "opencode_events")?;
        let mut subscription = self.client.subscribe_session(session_id).await?;
        let mut stream = Vec::new();
        let deadline = Instant::now() + EVENTS_WINDOW;

        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Some(event) = timeout(remaining, subscription.recv()).await.ok().flatten() else {
                break;
            };

            let should_stop = matches!(event, Event::SessionIdle { .. });
            stream.push(serde_json::to_value(event)?);
            if should_stop {
                break;
            }
        }

        subscription.close();

        Ok(json!({
            "session_id": session_id,
            "stream": stream,
        }))
    }
}

fn require_prompt(payload: &Value) -> Result<String> {
    let prompt = require_string(payload, "prompt", "opencode_run")?;
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("opencode_run requires a non-empty prompt"));
    }

    Ok(trimmed.to_string())
}

fn require_string<'a>(payload: &'a Value, key: &str, request_name: &str) -> Result<&'a str> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{request_name} requires payload.{key} string field"))
}

fn require_command(payload: &Value) -> Result<String> {
    let raw = require_string(payload, "command", "opencode_command")?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("opencode_command requires a non-empty command"));
    }

    let normalized = trimmed.strip_prefix('/').unwrap_or(trimmed).trim();
    if normalized.is_empty() {
        return Err(anyhow!("opencode_command requires a non-empty command"));
    }

    Ok(normalized.to_string())
}

fn read_optional_json(payload: &Value, key: &str) -> Option<Value> {
    payload.get(key).filter(|value| !value.is_null()).cloned()
}

fn read_string(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_string)
}

fn read_trimmed_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn resolve_model(payload: &Value) -> Option<ModelRef> {
    if let Some(model) = payload.get("model")
        && let Some(resolved) = model_from_value(model)
    {
        return Some(resolved);
    }

    model_from_value(payload)
}

fn model_from_value(value: &Value) -> Option<ModelRef> {
    let provider_id = read_model_id(value, &["providerID", "provider_id"])?;
    let model_id = read_model_id(value, &["modelID", "model_id"])?;

    Some(ModelRef {
        provider_id,
        model_id,
    })
}

fn read_model_id(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn latest_assistant_message_id(messages: &[Message]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.role() == "assistant")
        .map(|message| message.id().to_string())
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn require_prompt_rejects_empty_string() {
        let error = require_prompt(&json!({ "prompt": "   " })).unwrap_err();
        assert!(error.to_string().contains("non-empty prompt"));
    }

    #[test]
    fn resolve_model_reads_nested_model_object() {
        let payload = json!({
            "model": {
                "providerID": "anthropic",
                "modelID": "claude-sonnet"
            }
        });

        let model = resolve_model(&payload).unwrap();

        assert_eq!(model.provider_id, "anthropic");
        assert_eq!(model.model_id, "claude-sonnet");
    }

    #[test]
    fn resolve_model_reads_snake_case_top_level_fields() {
        let payload = json!({
            "provider_id": "openai",
            "model_id": "gpt-5"
        });

        let model = resolve_model(&payload).unwrap();

        assert_eq!(model.provider_id, "openai");
        assert_eq!(model.model_id, "gpt-5");
    }

    #[test]
    fn require_command_strips_leading_slash() {
        let command = require_command(&json!({ "command": " /review " })).unwrap();

        assert_eq!(command, "review");
    }

    #[test]
    fn require_command_rejects_empty_command() {
        let error = require_command(&json!({ "command": " /   " })).unwrap_err();

        assert!(error.to_string().contains("non-empty command"));
    }
}
