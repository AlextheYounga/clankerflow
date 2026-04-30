use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::app::output;
use crate::core::embeds::{copy_kit, place_opencode_config};
use crate::db::connection::connect;

/// # Errors
/// Returns an error if any step of initialization fails.
pub async fn run() -> anyhow::Result<()> {
    let project_root = env::current_dir()?;
    let agents_dir = project_root.join(".agents");
    let is_reinit = agents_dir.exists();

    if is_reinit && !confirm_overwrite()? {
        println!("{} setup cancelled.", output::warning("Warning"));
        return Ok(());
    }

    println!("{} clankerflow in {}", output::action("Setting up"), output::path(&".agents/"));

    // Copy kit files into .agents/
    copy_kit(&project_root, is_reinit)?;

    // Install Node dependencies for the workflow runtime.
    npm_install(&project_root)?;

    // Place .opencode/opencode.json for project-local OpenCode config.
    place_opencode_config(&project_root)?;

    // Initialize the database (creates + migrates)
    connect(&project_root).await?;

    if is_reinit {
        println!("{} clankerflow scaffold refreshed.", output::success("Done"));
    } else {
        println!("{} clankerflow in {}", output::success("Initialized"), output::path(&project_root.display()));
        println!();
        println!("{}", output::action(".agents Tour:"));
        println!("- {} framework scaffold and local project automation", output::path(&".agents/"));
        println!("- {} explain your project to clankerflow", output::path(&".agents/docs/PROJECT.md"));
        println!("- {} project settings", output::path(&".agents/settings.json"));
        println!("- {} sample workflows you can edit and run", output::path(&".agents/workflows/"));
        println!("- {} local planning tickets and notes", output::path(&".agents/tickets/"));
        println!("- {} shared context, roles, and templates", output::path(&".agents/context/"));
        println!("- {} runtime internals and container support", output::path(&".agents/.clankerflow/"));
        println!("- {} OpenCode project config", output::path(&".opencode/opencode.json"));
        println!();
        println!("{}", output::action("Next Steps:"));
        println!("- Update {} to explain your project.", output::path(&".agents/docs/PROJECT.md"));
        println!(
            "- Review {} and {}.",
            output::path(&".agents/settings.json"),
            output::path(&".opencode/opencode.json")
        );
        println!(
            "- Check out {}, {}, and {}.",
            output::path(&".agents/workflows/"),
            output::path(&".agents/tickets/"),
            output::path(&".agents/context/")
        );
        println!("- Run {}.", output::command("clankerflow work <name>"));
    }

    Ok(())
}

fn npm_install(project_root: &Path) -> anyhow::Result<()> {
    let lib_dir = project_root.join(".agents/.clankerflow/lib");
    let status = Command::new("npm")
        .args(["install", "--prefix", lib_dir.to_str().unwrap_or(".")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run npm install: {e}"))?;

    if !status.success() {
        anyhow::bail!("npm install failed in {}", lib_dir.display());
    }

    Ok(())
}

fn confirm_overwrite() -> anyhow::Result<bool> {
    print!(
        "{} {} already exists and will be overwritten. Continue? [y/N]: ",
        output::warning("Warning:"),
        output::path(&".agents")
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let normalized = input.trim().to_ascii_lowercase();

    Ok(matches!(normalized.as_str(), "y" | "yes"))
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_places_opencode_config_at_project_root() {
        let dir = TempDir::new().unwrap();
        copy_kit(dir.path(), false).unwrap();

        place_opencode_config(dir.path()).unwrap();

        assert!(dir.path().join(".opencode/opencode.json").exists());
    }
}
