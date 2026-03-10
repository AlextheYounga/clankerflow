use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

pub struct Client {
    base_url: String,
}

// -- Response types (only the fields we need) --

#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantMessage {
    pub id: String,
}

/// A single entry from the messages list endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageEntry {
    pub info: MessageInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    pub role: String,
}

// -- Request types --

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatRequest {
    parts: Vec<TextPart>,
}

#[derive(Debug, Serialize)]
struct TextPart {
    r#type: &'static str,
    text: String,
}

impl Client {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        let url = base_url.trim_end_matches('/');
        Self {
            base_url: url.to_string(),
        }
    }

    /// Create a new `OpenCode` session.    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub fn create_session(&self) -> Result<Session> {
        let url = format!("{}/session", self.base_url);
        let mut response = ureq::post(&url)
            .send_empty()
            .map_err(|e| anyhow!("create_session request failed: {e}"))?;

        response
            .body_mut()
            .read_json::<Session>()
            .map_err(|e| anyhow!("create_session response parse failed: {e}"))
    }

    /// Send a prompt to an existing session and wait for the assistant response.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub fn chat(&self, session_id: &str, prompt: &str) -> Result<AssistantMessage> {
        let url = format!("{}/session/{}/message", self.base_url, session_id);
        let body = ChatRequest {
            parts: vec![TextPart {
                r#type: "text",
                text: prompt.to_string(),
            }],
        };

        let mut response = ureq::post(&url)
            .send_json(&body)
            .map_err(|e| anyhow!("chat request failed: {e}"))?;

        response
            .body_mut()
            .read_json::<AssistantMessage>()
            .map_err(|e| anyhow!("chat response parse failed: {e}"))
    }

    /// List all messages in a session.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub fn messages(&self, session_id: &str) -> Result<Vec<MessageEntry>> {
        let url = format!("{}/session/{}/message", self.base_url, session_id);

        let mut response = ureq::get(&url)
            .call()
            .map_err(|e| anyhow!("messages request failed: {e}"))?;

        response
            .body_mut()
            .read_json::<Vec<MessageEntry>>()
            .map_err(|e| anyhow!("messages response parse failed: {e}"))
    }

    /// Abort a running session.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails.
    pub fn abort(&self, session_id: &str) -> Result<()> {
        let url = format!("{}/session/{}/abort", self.base_url, session_id);

        ureq::post(&url)
            .send_empty()
            .map_err(|e| anyhow!("abort request failed: {e}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_trailing_slash() {
        let client = Client::new("http://localhost:4096/");
        assert_eq!(client.base_url, "http://localhost:4096");
    }

    #[test]
    fn create_session_returns_session_id() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/session")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id": "sess_abc123"}"#)
            .create();

        let client = Client::new(&server.url());
        let session = client.create_session().unwrap();

        assert_eq!(session.id, "sess_abc123");
        mock.assert();
    }

    #[test]
    fn create_session_errors_on_server_failure() {
        let mut server = mockito::Server::new();
        let _mock = server.mock("POST", "/session").with_status(500).create();

        let client = Client::new(&server.url());
        let result = client.create_session();

        assert!(result.is_err());
    }

    #[test]
    fn chat_sends_prompt_and_returns_message() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/session/sess_1/message")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id": "msg_42", "sessionID": "sess_1", "role": "assistant"}"#)
            .create();

        let client = Client::new(&server.url());
        let msg = client.chat("sess_1", "Hello").unwrap();

        assert_eq!(msg.id, "msg_42");
        mock.assert();
    }

    #[test]
    fn chat_errors_on_server_failure() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/session/sess_1/message")
            .with_status(422)
            .create();

        let client = Client::new(&server.url());
        let result = client.chat("sess_1", "Hello");

        assert!(result.is_err());
    }

    #[test]
    fn messages_returns_list() {
        let mut server = mockito::Server::new();
        let body = r#"[
            {"info": {"id": "msg_1", "role": "user"}},
            {"info": {"id": "msg_2", "role": "assistant"}}
        ]"#;
        let mock = server
            .mock("GET", "/session/sess_1/message")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();

        let client = Client::new(&server.url());
        let msgs = client.messages("sess_1").unwrap();

        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].info.id, "msg_1");
        assert_eq!(msgs[0].info.role, "user");
        assert_eq!(msgs[1].info.id, "msg_2");
        assert_eq!(msgs[1].info.role, "assistant");
        mock.assert();
    }

    #[test]
    fn abort_sends_post_to_abort_endpoint() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/session/sess_1/abort")
            .with_status(200)
            .with_body("true")
            .create();

        let client = Client::new(&server.url());
        client.abort("sess_1").unwrap();

        mock.assert();
    }

    #[test]
    fn abort_errors_on_server_failure() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/session/sess_1/abort")
            .with_status(500)
            .create();

        let client = Client::new(&server.url());
        let result = client.abort("sess_1");

        assert!(result.is_err());
    }
}
