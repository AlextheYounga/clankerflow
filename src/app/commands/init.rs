use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;

use crate::core::embeds::copy_kit;
use crate::core::settings::Settings;
use crate::db::db::connect;
use std::io::{self, Write};

pub async fn run() -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let agents_dir = project_root.join(".agents");
    let is_reinit = agents_dir.exists();

    if is_reinit {
        if !confirm_overwrite()? {
            println!("Initialization cancelled.");
            return Ok(());
        }
    }

    // Copy kit files into .agents/
    copy_kit(&project_root, is_reinit)?;

    // Ensure settings always have a codebase_id, including after re-init.
    stamp_codebase_id(&project_root)?;

    // Initialize the database (creates + migrates)
    connect().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    if is_reinit {
        println!("Kit refreshed successfully.");
    } else {
        println!("Initialized agentctl in {}", project_root.display());
        println!("  .agents/                 framework scaffold");
        println!("  .agents/settings.json    project settings");
        println!("  .agents/workflows/       put your workflows here");
        println!();
        println!("Next: edit .agents/settings.json, then run `agentctl work <name>`.");
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
    use crate::core::embeds::copy_kit;
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
}
