use crate::core::embeds::copy_kit;
use crate::core::project::require_project_root;
use crate::core::settings::Settings;
use crate::db::db::connect;

pub async fn run() -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let agents_dir = project_root.join(".agents");
    let is_reinit = agents_dir.exists();

    if is_reinit {
        println!("Refreshing kit files (preserving tickets, settings, context, AGENTS.md)...");
    }

    // Copy kit files into .agents/
    copy_kit(is_reinit)?;

    // Stamp a new codebase_id only on a fresh init
    if !is_reinit {
        stamp_codebase_id(&project_root)?;
    }

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
