use std::path::Path;

use serde::{Deserialize, Serialize};

const SETTINGS_PATH: &str = ".agents/settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSettings {
    pub user_name: String,
    pub user_email: String,
    pub default_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSettings {
    pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub codebase_id: String,
    pub git: GitSettings,
    pub workflows: WorkflowSettings,
}

impl Settings {
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let path = project_root.join(SETTINGS_PATH);
        let raw = std::fs::read_to_string(&path)?;
        let settings = serde_json::from_str(&raw)?;
        Ok(settings)
    }

    pub fn save(&self, project_root: &Path) -> anyhow::Result<()> {
        let path = project_root.join(SETTINGS_PATH);
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
