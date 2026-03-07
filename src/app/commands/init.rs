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

fn stamp_codebase_id(project_root: &std::path::Path) -> anyhow::Result<()> {
    let mut settings = Settings::load(project_root)?;
    if settings.codebase_id.is_empty() {
        settings.codebase_id = new_codebase_id();
        settings.save(project_root)?;
    }
    Ok(())
}

fn new_codebase_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut h = DefaultHasher::new();
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut h);
    std::process::id().hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::embeds::copy_kit;
    use crate::core::settings::Settings;
    use tempfile::TempDir;

    #[test]
    fn stamp_codebase_id_fills_empty_value_after_reinit_overwrite() {
        let dir = TempDir::new().unwrap();
        copy_kit(dir.path(), false).unwrap();

        let mut settings = Settings::load(dir.path()).unwrap();
        settings.codebase_id.clear();
        settings.save(dir.path()).unwrap();

        stamp_codebase_id(dir.path()).unwrap();

        let updated = Settings::load(dir.path()).unwrap();
        assert!(!updated.codebase_id.is_empty());
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
