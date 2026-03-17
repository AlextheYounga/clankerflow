use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use mdpack::{PackOptions, pack_to_path};

#[derive(Parser)]
#[command(name = "cargo-kit", about = "Developer utilities for kit bundling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Sync runtime files into kit/ and generate src/kit.md.
    Bundle,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Bundle => bundle_kit(),
    }
}

fn bundle_kit() -> anyhow::Result<()> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let root = Path::new(&manifest_dir);

    let runtime_src = root.join("runtime/src");
    let runtime_workflows = root.join("runtime/workflows");

    let lib_src_dst = root.join("kit/.clankerflow/lib/src");
    let workflows_dst = root.join("kit/workflows");
    let bundle_out = root.join("src/kit.md");

    sync_dir(&runtime_src, &lib_src_dst)?;
    sync_dir(&runtime_workflows, &workflows_dst)?;
    copy_file(
        &root.join("runtime/package.json"),
        &root.join("kit/.clankerflow/lib/package.json"),
    )?;
    copy_file(
        &root.join("runtime/package-lock.json"),
        &root.join("kit/.clankerflow/lib/package-lock.json"),
    )?;

    pack_to_path(
        &root.join("kit"),
        &bundle_out,
        PackOptions {
            include_hidden: true,
            include_ignored: true,
        },
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    println!("Bundled kit to {}", bundle_out.display());
    Ok(())
}

fn sync_dir(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }

    copy_dir_all(src, dst)
}

fn copy_file(src: &Path, dst: &Path) -> anyhow::Result<()> {
    let parent = dst
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination path has no parent: {}", dst.display()))?;
    fs::create_dir_all(parent)?;
    fs::copy(src, dst)?;
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path: PathBuf = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
