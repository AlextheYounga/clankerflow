use std::env;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentCtlError {
    #[error("agentctl is not initialized in {0} or any parent directory (run `agentctl init`)")]
    ProjectNotInitialized(PathBuf),
}

pub type Result<T> = std::result::Result<T, AgentCtlError>;

pub fn get_project_root() -> Option<PathBuf> {
    let mut current = env::current_dir().ok();
    while let Some(path) = current {
        if path.join(".agents").is_dir() {
            return Some(path);
        }
        current = path.parent().map(|p| p.to_path_buf());
    }
    None
}

pub fn require_project_root() -> Result<PathBuf> {
    get_project_root().ok_or_else(|| {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("<unknown>"));
        AgentCtlError::ProjectNotInitialized(cwd)
    })
}
