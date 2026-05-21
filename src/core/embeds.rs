use glob::glob;
use rust_embed::RustEmbed;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

const OPENCODE_CONFIG: &str = include_str!("../../kit/opencode.json");

#[derive(RustEmbed)]
#[folder = "kit/"]
#[exclude = ".clankerflow/lib/node_modules/*"]
#[exclude = "opencode.json"]
struct EmbeddedKit;

/// Write every embedded kit file into `<project_root>/.agents/`.
///
/// # Errors
/// Returns an error if a file cannot be created or written to disk, or if an
/// expected embedded asset is missing.
pub fn copy_kit(project_root: &Path, is_reinit: bool) -> anyhow::Result<()> {
    copy_kit_into(project_root, is_reinit)
}

fn copy_kit_into(project_root: &Path, is_reinit: bool) -> anyhow::Result<()> {
    let agents_dir = project_root.join(".agents");
    fs::create_dir_all(&agents_dir)?;

    for file_path in EmbeddedKit::iter() {
        let Some(contents) = EmbeddedKit::get(file_path.as_ref()) else {
            anyhow::bail!("missing embedded kit asset: {file_path}");
        };

        let target_path = agents_dir.join(file_path.as_ref());
        if !is_reinit && target_path.exists() {
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(target_path, contents.data.as_ref())?;
    }

    enable_gitignore(&agents_dir);
    clear_gitkeeps(&agents_dir);

    Ok(())
}

/// Write the embedded `opencode.json` to `<project_root>/.opencode/opencode.json`.
/// Existing configs are updated in place to deny `OpenCode` question prompts.
///
/// # Errors
/// Returns an error if the directory cannot be created or the file cannot be written.
pub fn place_opencode_config(project_root: &Path) -> anyhow::Result<()> {
    let dest = project_root.join(".opencode/opencode.json");
    if !dest.exists() {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, OPENCODE_CONFIG)?;
        return Ok(());
    }

    let raw = fs::read_to_string(&dest)?;
    let mut config: Value = serde_json::from_str(&raw)?;
    ensure_question_denied(&mut config)?;
    fs::write(&dest, serde_json::to_string_pretty(&config)?)?;

    Ok(())
}

fn ensure_question_denied(config: &mut Value) -> anyhow::Result<()> {
    let Some(root) = config.as_object_mut() else {
        anyhow::bail!("OpenCode config must be a JSON object");
    };

    let permission = root.entry("permission").or_insert_with(|| Value::Object(Map::new()));
    let Some(permission_object) = permission.as_object_mut() else {
        anyhow::bail!("OpenCode config permission must be a JSON object");
    };

    permission_object.insert("question".to_string(), Value::String("deny".to_string()));
    Ok(())
}

fn enable_gitignore(agents_dir: &Path) {
    let sample_gitignore = agents_dir.join(".example.gitignore");
    let target_path = agents_dir.join(".gitignore");

    if sample_gitignore.exists() {
        fs::rename(sample_gitignore, target_path).unwrap_or_else(|e| {
            eprintln!("Failed to rename gitignore: {e}");
        });
    }
}

fn clear_gitkeeps(agents_dir: &Path) {
    let pattern = agents_dir.join("**/.gitkeep").to_string_lossy().to_string();
    let paths = match glob(&pattern) {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!("Failed to read glob pattern: {err}");
            return;
        }
    };

    for entry in paths {
        match entry {
            Ok(path) => {
                if let Err(err) = fs::remove_file(&path) {
                    eprintln!("Failed to remove {}: {err}", path.display());
                }
            }
            Err(err) => eprintln!("Failed to read glob entry: {err}"),
        }
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn fresh_init_writes_expected_scaffold_files() {
        let dir = TempDir::new().unwrap();

        copy_kit_into(dir.path(), false).unwrap();

        assert!(dir.path().join(".agents/settings.json").exists());
        assert!(dir.path().join(".agents/workflows/default.ts").exists());
        assert!(dir.path().join(".agents/.clankerflow/lib/index.ts").exists());
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
        let workflow_path = dir.path().join(".agents/workflows/default.ts");
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
    fn copy_kit_excludes_node_modules_from_agents_dir() {
        let dir = TempDir::new().unwrap();

        copy_kit_into(dir.path(), false).unwrap();

        assert!(!dir.path().join(".agents/.clankerflow/lib/node_modules").exists());
    }

    #[test]
    fn copy_kit_writes_expected_gitignore_entries() {
        let dir = TempDir::new().unwrap();

        copy_kit_into(dir.path(), false).unwrap();

        let gitignore = fs::read_to_string(dir.path().join(".agents/.gitignore")).unwrap();
        assert!(gitignore.contains(".clankerflow"));
        assert!(gitignore.contains(".worktrees"));
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
    fn place_opencode_config_errors_on_malformed_existing_config() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join(".opencode/opencode.json");
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, "custom config").unwrap();

        let result = place_opencode_config(dir.path());

        assert!(result.is_err());
        let content = fs::read_to_string(dest).unwrap();
        assert_eq!(content, "custom config");
    }

    #[test]
    fn place_opencode_config_adds_question_deny_to_existing_config() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join(".opencode/opencode.json");
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(
            &dest,
            r#"{
  "$schema": "https://opencode.ai/config.json",
  "theme": "system",
  "autoupdate": false,
  "model": "opencode/big-pickle"
}"#,
        )
        .unwrap();

        place_opencode_config(dir.path()).unwrap();

        let content = fs::read_to_string(dest).unwrap();
        assert!(content.contains("\"question\": \"deny\""));
        assert!(content.contains("\"theme\": \"system\""));
    }
}
