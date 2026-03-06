use std::fs;
use std::path::Path;

use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/kit/"]
struct Kit;

/// Write every embedded kit file into `<project_root>/.agents/`.
pub fn copy_kit(project_root: &Path, is_reinit: bool) -> anyhow::Result<()> {
    copy_kit_into(project_root, is_reinit)
}

fn copy_kit_into(project_root: &Path, _is_reinit: bool) -> anyhow::Result<()> {
    let agents_dir = project_root.join(".agents");

    for path in Kit::iter() {
        let rel: &str = &path;
        let dest = agents_dir.join(rel);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = Kit::get(rel).expect("embedded file must exist");
        fs::write(&dest, file.data)?;
    }

    // Always write `.agents/.gitignore` from the embedded `gitignore.example`.
    let gitignore_dest = agents_dir.join(".gitignore");
    let file = Kit::get("gitignore.example").expect("gitignore.example must be embedded");
    if let Some(parent) = gitignore_dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&gitignore_dest, file.data)?;

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
}
