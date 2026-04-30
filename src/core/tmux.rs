use std::env;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};

const SESSION_PREFIX: &str = "clankerflow__";
const RUN_WINDOW_PREFIX: &str = "run__";
const SESSION_WINDOW_PREFIX: &str = "opencode__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionWindow {
    pub session_name: String,
    pub window_name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowTarget<'a> {
    pub session_name: &'a str,
    pub run_id: i64,
    pub work_dir: &'a Path,
}

#[must_use]
pub fn project_session_name(project_key: &str) -> String {
    format!("{SESSION_PREFIX}{}", sanitize_token(project_key))
}

#[must_use]
pub fn run_window_name(run_id: i64, workflow_name: &str) -> String {
    format!("{RUN_WINDOW_PREFIX}{run_id}__{}", sanitize_token(workflow_name))
}

#[must_use]
pub fn opencode_window_name(run_id: i64, session_id: &str) -> String {
    format!("{SESSION_WINDOW_PREFIX}{run_id}__{}", sanitize_token(session_id))
}

/// # Errors
/// Returns an error when tmux fails to create the window.
pub fn create_run_window(workflow_name: &str, target: WindowTarget<'_>, command: &str) -> Result<String> {
    let window_name = run_window_name(target.run_id, workflow_name);
    create_window(target.session_name, &window_name, target.work_dir, command)?;
    Ok(window_name)
}

/// # Errors
/// Returns an error when tmux fails to create the window.
pub fn create_opencode_window(opencode_session_id: &str, target: WindowTarget<'_>, base_url: &str) -> Result<String> {
    let window_name = opencode_window_name(target.run_id, opencode_session_id);
    let command = format!("opencode attach {base_url} -s {}", shell_escape(opencode_session_id));
    create_window(target.session_name, &window_name, target.work_dir, &command)?;
    Ok(window_name)
}

/// # Errors
/// Returns an error when tmux fails to attach or switch the client.
pub fn attach_session(session_name: &str) -> Result<()> {
    let command = if env::var("TMUX").is_ok() { "switch-client" } else { "attach-session" };

    let status = Command::new("tmux")
        .args([command, "-t", session_name])
        .status()
        .context("failed to attach to clankerflow tmux session")?;

    if !status.success() {
        return Err(anyhow!("tmux failed to attach to session {session_name}"));
    }

    Ok(())
}

/// # Errors
/// Returns an error when tmux fails to attach or select the window.
pub fn attach_window(session_name: &str, window_name: &str) -> Result<()> {
    let target = format!("{session_name}:{window_name}");
    let status = if env::var("TMUX").is_ok() {
        Command::new("tmux")
            .args(["switch-client", "-t", session_name])
            .args([";", "select-window", "-t", &target])
            .status()
            .context("failed to switch to clankerflow window")?
    } else {
        Command::new("tmux")
            .args(["attach-session", "-t", session_name])
            .args([";", "select-window", "-t", &target])
            .status()
            .context("failed to attach clankerflow window")?
    };

    if !status.success() {
        return Err(anyhow!("tmux failed to attach window {target}"));
    }

    Ok(())
}

/// # Errors
/// Returns an error when tmux fails to list sessions or windows.
pub fn list_project_windows(project_key: &str) -> Result<Vec<SessionWindow>> {
    let session_name = project_session_name(project_key);
    if !session_exists(&session_name)? {
        return Ok(Vec::new());
    }

    let output = Command::new("tmux")
        .args(["list-windows", "-t", &session_name])
        .args(["-F", "#{window_name}\x1f#{window_active}"])
        .output()
        .context("failed to list clankerflow tmux windows")?;

    if !output.status.success() {
        return Err(anyhow!("tmux failed to list windows for {session_name}"));
    }

    let output = String::from_utf8(output.stdout).context("tmux window output was not valid UTF-8")?;
    let mut windows = Vec::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let (window_name, active) =
            line.split_once('\x1f').ok_or_else(|| anyhow!("failed to parse tmux window line: {line}"))?;
        windows.push(SessionWindow {
            session_name: session_name.clone(),
            window_name: window_name.to_string(),
            active: active == "1",
        });
    }

    Ok(windows)
}

fn create_window(session_name: &str, window_name: &str, work_dir: &Path, command: &str) -> Result<()> {
    if window_exists(session_name, window_name)? {
        return Ok(());
    }

    let status = if session_exists(session_name)? {
        Command::new("tmux")
            .args(["new-window", "-d", "-t", session_name, "-n", window_name, "-c"])
            .arg(work_dir)
            .arg(command)
            .status()
            .with_context(|| format!("failed to create tmux window {window_name}"))?
    } else {
        Command::new("tmux")
            .args(["new-session", "-d", "-s", session_name, "-n", window_name, "-c"])
            .arg(work_dir)
            .arg(command)
            .status()
            .with_context(|| format!("failed to create tmux session {session_name}"))?
    };

    if !status.success() {
        return Err(anyhow!("tmux failed to create window {window_name} in {session_name}"));
    }

    set_remain_on_exit(session_name, window_name)?;

    Ok(())
}

fn session_exists(session_name: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()
        .context("failed to inspect tmux sessions")?;

    Ok(output.status.success())
}

fn window_exists(session_name: &str, window_name: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .args(["list-windows", "-t", session_name, "-F", "#{window_name}"])
        .output()
        .context("failed to inspect tmux windows")?;

    if !output.status.success() {
        return Ok(false);
    }

    let output = String::from_utf8(output.stdout).context("tmux window output was not valid UTF-8")?;
    Ok(output.lines().any(|line| line.trim() == window_name))
}

fn set_remain_on_exit(session_name: &str, window_name: &str) -> Result<()> {
    let target = format!("{session_name}:{window_name}");
    let status = Command::new("tmux")
        .args(["set-window-option", "-t", &target, "remain-on-exit", "on"])
        .status()
        .with_context(|| format!("failed to configure remain-on-exit for {target}"))?;

    if !status.success() {
        return Err(anyhow!("tmux failed to enable remain-on-exit for {target}"));
    }

    Ok(())
}

fn sanitize_token(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => character,
            _ => '_',
        })
        .collect::<String>();

    if sanitized.is_empty() { "unknown".to_string() } else { sanitized }
}

fn shell_escape(value: &str) -> String {
    let escaped = value.replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_session_name_uses_prefix() {
        assert_eq!(project_session_name("abc123"), "clankerflow__abc123".to_string());
    }

    #[test]
    fn run_window_name_includes_run_id_and_workflow_name() {
        assert_eq!(run_window_name(42, "duos"), "run__42__duos".to_string());
    }

    #[test]
    fn opencode_window_name_includes_session_id() {
        assert_eq!(opencode_window_name(42, "sess_abc"), "opencode__42__sess_abc".to_string());
    }

    #[test]
    fn sanitize_token_replaces_unsafe_characters() {
        assert_eq!(sanitize_token("sess:abc/123"), "sess_abc_123".to_string());
    }
}
