use std::fs;
use std::path::{Path, PathBuf};

const TICKET_TEMPLATE: &str = include_str!("../kit/context/templates/ticket-template.md");

/// Create a new ticket in `<project_root>/.agents/tickets/` and return
/// the filename (e.g. `T-001.md`).
///
/// # Errors
/// Returns an error if the tickets directory cannot be created, the ticket
/// file already exists, or writing the file fails.
pub fn create_ticket(project_root: &Path) -> anyhow::Result<String> {
    create_ticket_with_options(project_root, None, None)
}

/// Create a ticket with an optional title override.
///
/// # Errors
/// Returns an error if the tickets directory cannot be created, the ticket
/// file already exists, or writing the file fails.
pub fn create_ticket_with_title(project_root: &Path, title: &str) -> anyhow::Result<String> {
    create_ticket_with_options(project_root, Some(title), None)
}

/// # Errors
/// Returns an error if the tickets directory cannot be created, the ticket
/// file already exists, or writing the file fails.
pub fn create_ticket_with_branch(project_root: &Path, branch: &str) -> anyhow::Result<String> {
    create_ticket_with_options(project_root, None, Some(branch))
}

fn create_ticket_with_options(
    project_root: &Path,
    title: Option<&str>,
    branch: Option<&str>,
) -> anyhow::Result<String> {
    let tickets_dir = dir(project_root);
    fs::create_dir_all(&tickets_dir)?;

    let number = next_ticket_number(&tickets_dir)?;
    let ticket_id = format!("T-{number:03}");
    let filename = format!("{ticket_id}.md");
    let ticket_path = tickets_dir.join(&filename);

    if ticket_path.exists() {
        anyhow::bail!("Ticket already exists: {}", ticket_path.display());
    }

    let content = render_template(&ticket_id, title, branch);
    fs::write(&ticket_path, content)?;

    Ok(filename)
}

#[must_use]
pub fn dir(project_root: &Path) -> PathBuf {
    project_root.join(".agents").join("tickets")
}

fn next_ticket_number(tickets_dir: &Path) -> anyhow::Result<u32> {
    if !tickets_dir.exists() {
        return Ok(1);
    }

    let mut max = 0u32;
    for entry in fs::read_dir(tickets_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(n) = parse_ticket_number(&name) {
            max = max.max(n);
        }
    }

    Ok(max + 1)
}

#[must_use]
pub fn parse_ticket_number(filename: &str) -> Option<u32> {
    filename
        .strip_prefix("T-")
        .and_then(|s| s.strip_suffix(".md"))
        .and_then(|n| n.parse().ok())
}

fn render_template(ticket_id: &str, title: Option<&str>, branch: Option<&str>) -> String {
    let number = ticket_id.strip_prefix("T-").unwrap_or(ticket_id);
    let mut out = TICKET_TEMPLATE.replacen("id: '001'", &format!("id: '{number}'"), 1);

    if let Some(t) = title {
        let t = t.lines().next().unwrap_or("").trim();
        if !t.is_empty() {
            let escaped = t.replace('\\', "\\\\").replace('"', "\\\"");
            out = out.replacen("title: Short Title", &format!("title: \"{escaped}\""), 1);
        }
    }

    if let Some(b) = branch {
        let b = b.trim();
        if !b.is_empty() {
            out = out.replacen("branch: feat/your-branch-name", &format!("branch: {b}"), 1);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        let dir = TempDir::new().unwrap();
        let tickets = dir.path().join(".agents/tickets");
        fs::create_dir_all(&tickets).unwrap();
        dir
    }

    #[test]
    fn creates_first_ticket_as_t001() {
        let dir = setup();
        let filename = create_ticket(dir.path()).unwrap();
        assert_eq!(filename, "T-001.md");
        assert!(dir.path().join(".agents/tickets/T-001.md").exists());
    }

    #[test]
    fn increments_ticket_number() {
        let dir = setup();
        create_ticket(dir.path()).unwrap();
        let filename = create_ticket(dir.path()).unwrap();
        assert_eq!(filename, "T-002.md");
    }

    #[test]
    fn create_ticket_with_title_substitutes_placeholder() {
        let dir = setup();
        create_ticket_with_title(dir.path(), "Add login feature").unwrap();
        let content = fs::read_to_string(dir.path().join(".agents/tickets/T-001.md")).unwrap();
        assert!(content.contains("title: \"Add login feature\""));
    }

    #[test]
    fn create_ticket_with_branch_substitutes_placeholder() {
        let dir = setup();
        create_ticket_with_branch(dir.path(), "feat/my-branch").unwrap();
        let content = fs::read_to_string(dir.path().join(".agents/tickets/T-001.md")).unwrap();
        assert!(content.contains("branch: feat/my-branch"));
    }

    #[test]
    fn parse_ticket_number_handles_valid_filenames() {
        assert_eq!(parse_ticket_number("T-001.md"), Some(1));
        assert_eq!(parse_ticket_number("T-042.md"), Some(42));
    }

    #[test]
    fn parse_ticket_number_rejects_invalid_filenames() {
        assert_eq!(parse_ticket_number("README.md"), None);
        assert_eq!(parse_ticket_number("T-abc.md"), None);
    }
}
