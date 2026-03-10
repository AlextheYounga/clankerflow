use std::fs;
use std::path::Path;

use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/kit/"]
struct Kit;

/// Write every embedded kit file into `<project_root>/.agents/`.
///
/// # Errors
/// Returns an error if a file cannot be created or written to disk, or if an
/// expected embedded asset is missing.
pub fn copy_kit(project_root: &Path, is_reinit: bool) -> anyhow::Result<()> {
    copy_kit_into(project_root, is_reinit)
}

fn copy_kit_into(project_root: &Path, _is_reinit: bool) -> anyhow::Result<()> {
    let agents_dir = project_root.join(".agents");

    for path in Kit::iter() {
        let rel: &str = &path;

        // opencode.json is placed separately via place_opencode_config.
        if rel == "opencode.json" {
            continue;
        }

        let dest = agents_dir.join(rel);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let file =
            Kit::get(rel).ok_or_else(|| anyhow::anyhow!("embedded kit file missing: {rel}"))?;
        fs::write(&dest, file.data)?;
    }

    // Always write `.agents/.gitignore` from the embedded `gitignore.example`.
    let gitignore_dest = agents_dir.join(".gitignore");
    let file = Kit::get("gitignore.example")
        .ok_or_else(|| anyhow::anyhow!("embedded asset 'gitignore.example' is missing"))?;
    if let Some(parent) = gitignore_dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&gitignore_dest, file.data)?;

    Ok(())
}

/// Write the embedded `opencode.json` to `<project_root>/.opencode/opencode.json`.
/// Skips if the file already exists (idempotent).
///
/// # Errors
/// Returns an error if the directory cannot be created or the file cannot be written.
pub fn place_opencode_config(project_root: &Path) -> anyhow::Result<()> {
    let dest = project_root.join(".opencode/opencode.json");
    if dest.exists() {
        return Ok(());
    }

    let file = Kit::get("opencode.json")
        .ok_or_else(|| anyhow::anyhow!("embedded asset 'opencode.json' is missing"))?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&dest, file.data)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn fresh_init_writes_expected_scaffold_files() {
        let dir = TempDir::new().unwrap();

        copy_kit_into(dir.path(), false).unwrap();

        assert!(dir.path().join(".agents/settings.json").exists());
        assert!(dir.path().join(".agents/workflows/default.js").exists());
        assert!(dir.path().join(".agents/.gitignore").exists());
    }

    #[test]
    fn reinit_overwrites_existing_scaffold_files() {
        let dir = TempDir::new().unwrap();
        copy_kit_into(dir.path(), false).unwrap();
        let settings_path = dir.path().join(".agents/settings.json");
        fs::write(&settings_path, "custom settings").unwrap();

        copy_kit_into(dir.path(), true).unwrap();

        let settings = fs::read_to_string(settings_path).unwrap();
        assert_ne!(settings, "custom settings");
    }

    #[test]
    fn reinit_restores_missing_scaffold_files() {
        let dir = TempDir::new().unwrap();
        copy_kit_into(dir.path(), false).unwrap();
        let workflow_path = dir.path().join(".agents/workflows/default.js");
        fs::remove_file(&workflow_path).unwrap();

        copy_kit_into(dir.path(), true).unwrap();

        assert!(workflow_path.exists());
    }

    #[test]
    fn reinit_rewrites_gitignore_file() {
        let dir = TempDir::new().unwrap();
        copy_kit_into(dir.path(), false).unwrap();
        let gitignore_path = dir.path().join(".agents/.gitignore");
        fs::write(&gitignore_path, "custom-ignore").unwrap();

        copy_kit_into(dir.path(), true).unwrap();

        let gitignore = fs::read_to_string(gitignore_path).unwrap();
        assert_ne!(gitignore.trim(), "custom-ignore");
    }

    #[test]
    fn copy_kit_excludes_opencode_json_from_agents_dir() {
        let dir = TempDir::new().unwrap();

        copy_kit_into(dir.path(), false).unwrap();

        assert!(!dir.path().join(".agents/opencode.json").exists());
    }

    #[test]
    fn place_opencode_config_writes_file_on_fresh_init() {
        let dir = TempDir::new().unwrap();

        place_opencode_config(dir.path()).unwrap();

        let dest = dir.path().join(".opencode/opencode.json");
        assert!(dest.exists());
        let content = fs::read_to_string(dest).unwrap();
        assert!(content.contains("opencode.ai"));
    }

    #[test]
    fn place_opencode_config_skips_if_already_exists() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join(".opencode/opencode.json");
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, "custom config").unwrap();

        place_opencode_config(dir.path()).unwrap();

        let content = fs::read_to_string(dest).unwrap();
        assert_eq!(content, "custom config");
    }
}
