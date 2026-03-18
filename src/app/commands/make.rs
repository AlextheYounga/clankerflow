use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::project::require_project_root;
use crate::core::tickets;

/// # Errors
/// Returns an error if the project root is not found or ticket creation fails.
pub fn ticket() -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let filename = tickets::create_ticket(&project_root)?;
    println!(
        "Created {}",
        tickets::dir(&project_root).join(&filename).display()
    );
    Ok(())
}

/// # Errors
/// Returns an error if the branch name is invalid, the project root is not
/// found, the worktree path already exists, or `git worktree add` fails.
pub fn worktree(branch: &str) -> anyhow::Result<()> {
    validate_branch_name(branch)?;
    let project_root = require_project_root()?;

    let worktree_path = worktree_path(&project_root, branch);
    if worktree_path.exists() {
        anyhow::bail!("Worktree path already exists: {}", worktree_path.display());
    }

    let parent = worktree_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("worktree path has no parent directory"))?;
    fs::create_dir_all(parent)?;
    create_git_worktree(&project_root, branch, &worktree_path)?;

    println!("Created worktree {}", worktree_path.display());
    Ok(())
}

fn worktree_path(project_root: &Path, branch: &str) -> PathBuf {
    project_root.join(".agents").join(".worktrees").join(branch)
}

fn validate_branch_name(branch: &str) -> anyhow::Result<()> {
    if branch.is_empty() {
        anyhow::bail!("Branch name cannot be empty");
    }
    if branch.contains("..") || branch.starts_with('-') || branch.contains(' ') {
        anyhow::bail!("Invalid branch name: '{branch}'");
    }
    Ok(())
}

fn create_git_worktree(
    project_root: &Path,
    branch: &str,
    worktree_path: &Path,
) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            branch,
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(project_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!("Failed to create git worktree: {stderr}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn validate_branch_name_accepts_valid_names() {
        assert!(validate_branch_name("feat/new-feature").is_ok());
        assert!(validate_branch_name("fix-bug-123").is_ok());
        assert!(validate_branch_name("release/v1.0").is_ok());
    }

    #[test]
    fn validate_branch_name_rejects_empty_name() {
        assert!(validate_branch_name("").is_err());
    }

    #[test]
    fn validate_branch_name_rejects_double_dot() {
        assert!(validate_branch_name("feat..bad").is_err());
    }

    #[test]
    fn validate_branch_name_rejects_leading_dash() {
        assert!(validate_branch_name("-bad").is_err());
    }

    #[test]
    fn validate_branch_name_rejects_spaces() {
        assert!(validate_branch_name("bad branch").is_err());
    }

    #[test]
    fn constructs_worktree_path_under_agents_worktrees() {
        let root = PathBuf::from("/tmp/project");

        let path = worktree_path(&root, "feat/new-feature");

        assert_eq!(
            path,
            PathBuf::from("/tmp/project/.agents/.worktrees/feat/new-feature")
        );
    }
}
