use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Deserialize;

const OPENCODE_CONFIG_PATH: &str = ".opencode/opencode.json";

#[derive(Debug, Clone, Deserialize)]
pub struct OpencodeConfig {
    pub server_url: Option<String>,
}

impl OpencodeConfig {
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let path = project_root.join(OPENCODE_CONFIG_PATH);
        let raw = fs::read_to_string(&path)?;
        let config = serde_json::from_str(&raw)?;
        Ok(config)
    }

    /// # Errors
    /// Returns an error if the config exists but cannot be read or parsed.
    pub fn load_optional(project_root: &Path) -> anyhow::Result<Option<Self>> {
        let path = project_root.join(OPENCODE_CONFIG_PATH);
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        let config = serde_json::from_str(&raw)?;
        Ok(Some(config))
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".opencode")).unwrap();
        dir
    }

    #[test]
    fn load_reads_server_url() {
        let dir = setup();
        let json = r#"{
  "server_url": "http://10.0.0.5:8080"
}"#;
        fs::write(dir.path().join(".opencode/opencode.json"), json).unwrap();

        let config = OpencodeConfig::load(dir.path()).unwrap();

        assert_eq!(config.server_url.as_deref(), Some("http://10.0.0.5:8080"));
    }

    #[test]
    fn load_handles_missing_server_url() {
        let dir = setup();
        let json = r#"{
  "$schema": "https://opencode.ai/config.json",
  "model": "opencode/big-pickle"
}"#;
        fs::write(dir.path().join(".opencode/opencode.json"), json).unwrap();

        let config = OpencodeConfig::load(dir.path()).unwrap();

        assert!(config.server_url.is_none());
    }

    #[test]
    fn load_errors_when_file_is_missing() {
        let dir = TempDir::new().unwrap();

        let result = OpencodeConfig::load(dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn load_optional_returns_none_when_file_is_missing() {
        let dir = TempDir::new().unwrap();

        let result = OpencodeConfig::load_optional(dir.path()).unwrap();

        assert!(result.is_none());
    }
}
