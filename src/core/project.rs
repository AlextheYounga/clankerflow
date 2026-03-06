use std::path::{Path, PathBuf};

pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join(".agents").is_dir() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

pub fn require_project_root(cwd: &Path) -> Result<std::path::PathBuf> {
    find_project_root(cwd).ok_or_else(|| AgentCtlError::ProjectNotInitialized(cwd.to_path_buf()))
}

pub fn require_initialized_project(cwd: &Path) -> Result<std::path::PathBuf> {
    let project_root = require_project_root(cwd)?;
    initialize_project_database(&project_root)?;
    Ok(project_root)
}