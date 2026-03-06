use crate::core::project::require_project_root;
use std::path::Path;
use std::process::Command;

pub async fn ticket() -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let filename = crate::core::tickets::create_ticket(&project_root)?;
    println!(
        "Created {}",
        project_root
            .join(".agents/tickets")
            .join(&filename)
            .display()
    );
    Ok(())
}

pub async fn worktree(branch: String) -> anyhow::Result<()> {
    validate_branch_name(&branch)?;
    let project_root = require_project_root()?;
	
    let worktree_path = project_root
        .join(".agents")
        .join(".worktrees")
        .join(&branch);
    if worktree_path.exists() {
        anyhow::bail!("Worktree path already exists: {}", worktree_path.display());
    }

    std::fs::create_dir_all(worktree_path.parent().unwrap())?;
    create_git_worktree(&project_root, &branch, &worktree_path)?;

    println!("Created worktree {}", worktree_path.display());
    Ok(())
}

fn validate_branch_name(branch: &str) -> anyhow::Result<()> {
    if branch.is_empty() {
        anyhow::bail!("Branch name cannot be empty");
    }
    if branch.contains("..") || branch.starts_with('-') || branch.contains(' ') {
        anyhow::bail!("Invalid branch name: '{}'", branch);
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
        anyhow::bail!("Failed to create git worktree: {}", stderr);
    }

    Ok(())
}
