use std::env;
use std::path::{Path, PathBuf};

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

pub fn require_initialized_project() -> Result<PathBuf> {
    let project_root = require_project_root()?;
    initialize_project_database(&project_root)?;
    Ok(project_root)
}