use crate::core::project::require_project_root;
use std::fs;

use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/kit/"]
struct Kit;

/// Write every embedded kit file into `<project_root>/.agents/`.
///
/// On re-init the tickets directory and settings.json are preserved
/// (files that already exist are skipped).
pub fn copy_kit(is_reinit: bool) -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let agents_dir = project_root.join(".agents");

    for path in Kit::iter() {
        let rel: &str = &path;
        let dest = agents_dir.join(rel);

        if is_reinit && should_preserve(rel) {
            continue;
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = Kit::get(rel).expect("embedded file must exist");
        fs::write(&dest, file.data)?;
    }

    // Write `.agents/.gitignore` from the embedded `gitignore.example`.
    // Skips if the file already exists so user edits are not clobbered.
    let gitignore_dest = agents_dir.join(".gitignore");
    if !gitignore_dest.exists() {
        let file = Kit::get("gitignore.example").expect("gitignore.example must be embedded");
        if let Some(parent) = gitignore_dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&gitignore_dest, file.data)?;
    }

    Ok(())
}

/// Returns true for paths that must survive a re-init.
fn should_preserve(rel: &str) -> bool {
    rel.starts_with("tickets/")
        || rel == "settings.json"
        || rel.starts_with("context/")
        || rel == "AGENTS.md"
        || rel == "README.md"
}
