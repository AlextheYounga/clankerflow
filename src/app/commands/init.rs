use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;

use crate::core::embeds::{copy_kit, place_opencode_config};
use crate::core::settings::Settings;
use crate::db::connection::connect;

/// # Errors
/// Returns an error if any step of initialization fails.
pub async fn run() -> anyhow::Result<()> {
    let project_root = env::current_dir()?;
    let agents_dir = project_root.join(".agents");
    let is_reinit = agents_dir.exists();

    if is_reinit && !confirm_overwrite()? {
        println!("Initialization cancelled.");
        return Ok(());
    }

    // Copy kit files into .agents/
    copy_kit(&project_root, is_reinit)?;

    // Install Node dependencies for the workflow runtime.
    npm_install(&project_root)?;

    // Place .opencode/opencode.json for project-local OpenCode config.
    place_opencode_config(&project_root)?;

    // Ensure settings always have a codebase_id, including after re-init.
    stamp_codebase_id(&project_root)?;

    // Initialize the database (creates + migrates)
    connect().await.map_err(|e| anyhow::anyhow!("{e}"))?;

    if is_reinit {
        println!("Kit refreshed successfully.");
    } else {
        println!("Initialized kata in {}", project_root.display());
        println!("  .agents/                 framework scaffold");
        println!("  .agents/settings.json    project settings");
        println!("  .agents/workflows/       put your workflows here");
        println!();
        println!("Next: edit .agents/settings.json, then run `kata work <name>`.");
    }

    Ok(())
}

fn npm_install(project_root: &Path) -> anyhow::Result<()> {
    let lib_dir = project_root.join(".agents/.agentkata/lib");
    let status = Command::new("npm")
        .args(["install", "--prefix", lib_dir.to_str().unwrap_or(".")])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run npm install: {e}"))?;

    if !status.success() {
        anyhow::bail!("npm install failed in {}", lib_dir.display());
    }

    Ok(())
}

fn confirm_overwrite() -> anyhow::Result<bool> {
    print!("Warning: .agents already exists and will be overwritten. Continue? [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let normalized = input.trim().to_ascii_lowercase();

    Ok(matches!(normalized.as_str(), "y" | "yes"))
}

fn stamp_codebase_id(project_root: &Path) -> anyhow::Result<()> {
    let mut settings = Settings::load(project_root)?;
    if settings.codebase_id.is_empty() {
        settings.codebase_id = codebase_id_for(project_root);
        settings.save(project_root)?;
    }
    Ok(())
}

fn codebase_id_for(project_root: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        STANDARD_NO_PAD.encode(project_root.as_os_str().as_bytes())
    }
    #[cfg(not(unix))]
    {
        STANDARD_NO_PAD.encode(project_root.to_string_lossy().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::Settings;
    use tempfile::TempDir;

    #[test]
    fn codebase_id_for_encodes_path_as_base64_no_padding() {
        let id = codebase_id_for(Path::new("/srv/project"));

        assert_eq!(id, "L3Nydi9wcm9qZWN0");
        assert!(!id.contains('='));
    }

    #[test]
    fn codebase_id_for_is_deterministic() {
        let path = Path::new("/tmp/my-project");
        let a = codebase_id_for(path);
        let b = codebase_id_for(path);

        assert_eq!(a, b);
    }

    #[test]
    fn stamp_codebase_id_fills_empty_value_after_reinit_overwrite() {
        let dir = TempDir::new().unwrap();
        copy_kit(dir.path(), false).unwrap();

        let mut settings = Settings::load(dir.path()).unwrap();
        settings.codebase_id.clear();
        settings.save(dir.path()).unwrap();

        stamp_codebase_id(dir.path()).unwrap();

        let updated = Settings::load(dir.path()).unwrap();
        let expected = codebase_id_for(dir.path());
        assert_eq!(updated.codebase_id, expected);
    }

    #[test]
    fn stamp_codebase_id_preserves_existing_value() {
        let dir = TempDir::new().unwrap();
        copy_kit(dir.path(), false).unwrap();

        let mut settings = Settings::load(dir.path()).unwrap();
        settings.codebase_id = "existing-id".to_string();
        settings.save(dir.path()).unwrap();

        stamp_codebase_id(dir.path()).unwrap();

        let updated = Settings::load(dir.path()).unwrap();
        assert_eq!(updated.codebase_id, "existing-id");
    }

    #[test]
    fn init_places_opencode_config_at_project_root() {
        let dir = TempDir::new().unwrap();
        copy_kit(dir.path(), false).unwrap();

        place_opencode_config(dir.path()).unwrap();

        assert!(dir.path().join(".opencode/opencode.json").exists());
    }
}
