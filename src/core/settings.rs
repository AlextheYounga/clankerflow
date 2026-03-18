use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const SETTINGS_PATH: &str = ".agents/settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub user_name: String,
    pub user_email: String,
    pub default_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub git: GitConfig,
    pub workflows: WorkflowConfig,
}

impl Settings {
    /// # Errors
    /// Returns an error if the settings file cannot be read or parsed.
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let path = project_root.join(SETTINGS_PATH);
        let raw = fs::read_to_string(&path)?;
        let settings = serde_json::from_str(&raw)?;
        Ok(settings)
    }

    /// # Errors
    /// Returns an error if the settings cannot be serialized or the file
    /// cannot be written.
    pub fn save(&self, project_root: &Path) -> anyhow::Result<()> {
        let path = project_root.join(SETTINGS_PATH);
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn sample_settings() -> Settings {
        Settings {
            git: GitConfig {
                user_name: "Alex".to_string(),
                user_email: "alex@example.com".to_string(),
                default_branch: "main".to_string(),
            },
            workflows: WorkflowConfig {
                default: "duos".to_string(),
            },
        }
    }

    fn setup() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".agents")).unwrap();
        dir
    }

    #[test]
    fn load_reads_valid_settings_json() {
        let dir = setup();
        let json = r#"{
  "git": {
    "user_name": "Alex",
    "user_email": "alex@example.com",
    "default_branch": "main"
  },
  "workflows": {
    "default": "duos"
  }
}"#;
        fs::write(dir.path().join(".agents/settings.json"), json).unwrap();

        let settings = Settings::load(dir.path()).unwrap();

        assert_eq!(settings.git.user_name, "Alex");
        assert_eq!(settings.workflows.default, "duos");
    }

    #[test]
    fn save_writes_json_that_round_trips() {
        let dir = setup();
        let settings = sample_settings();

        settings.save(dir.path()).unwrap();
        let loaded = Settings::load(dir.path()).unwrap();

        assert_eq!(loaded.git.user_email, settings.git.user_email);
        assert_eq!(loaded.workflows.default, settings.workflows.default);
    }

    #[test]
    fn load_errors_when_settings_file_is_missing() {
        let dir = setup();

        let result = Settings::load(dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn load_errors_when_json_is_malformed() {
        let dir = setup();
        fs::write(dir.path().join(".agents/settings.json"), "{not-valid-json").unwrap();

        let result = Settings::load(dir.path());

        assert!(result.is_err());
    }
}
