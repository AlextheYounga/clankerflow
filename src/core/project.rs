use std::env;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentCtlError {
    #[error("agentctl is not initialized in {0} or any parent directory (run `agentctl init`)")]
    ProjectNotInitialized(PathBuf),
}

pub type Result<T> = std::result::Result<T, AgentCtlError>;

fn walk_for_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start.to_path_buf());
    while let Some(path) = current {
        if path.join(".agents").is_dir() {
            return Some(path);
        }
        current = path.parent().map(|p| p.to_path_buf());
    }
    None
}

pub fn get_project_root() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    walk_for_project_root(&cwd)
}

fn require_project_root_from(start: &Path) -> Result<PathBuf> {
    walk_for_project_root(start)
        .ok_or_else(|| AgentCtlError::ProjectNotInitialized(start.to_path_buf()))
}

pub fn require_project_root() -> Result<PathBuf> {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("<unknown>"));
    require_project_root_from(&cwd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn finds_project_root_in_start_directory() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".agents")).unwrap();

        let root = walk_for_project_root(dir.path());

        assert_eq!(root, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn finds_project_root_in_parent_directory() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".agents")).unwrap();
        let nested = dir.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();

        let root = walk_for_project_root(&nested);

        assert_eq!(root, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn returns_none_when_project_not_initialized() {
        let dir = TempDir::new().unwrap();

        let root = walk_for_project_root(dir.path());

        assert_eq!(root, None);
    }

    #[test]
    fn require_project_root_returns_project_not_initialized_error() {
        let dir = TempDir::new().unwrap();

        let err = require_project_root_from(dir.path()).unwrap_err();

        assert!(matches!(err, AgentCtlError::ProjectNotInitialized(_)));
    }
}
