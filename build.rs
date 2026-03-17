use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/kit.md");

    if env::var_os("CARGO_FEATURE_TEST_RUNNER_BUNDLE").is_none() {
        return Ok(());
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let test_runtime_dir = target_dir()?.join("test-runtime");
    let bundled_runner = test_runtime_dir.join("runner.cjs");

    bundle_test_runner(&manifest_dir, &bundled_runner)?;
    println!(
        "cargo:rustc-env=CLANKERFLOW_TEST_RUNNER_BUNDLE={}",
        bundled_runner.display()
    );

    Ok(())
}

fn bundle_test_runner(manifest_dir: &str, bundled_runner: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = bundled_runner.parent() {
        fs::create_dir_all(parent)?;
    }

    let esbuild = Path::new(manifest_dir).join("runtime/node_modules/esbuild/bin/esbuild");
    if !esbuild.exists() {
        println!(
            "cargo:warning=Skipping test runner bundle; missing {}",
            esbuild.display()
        );
        return Ok(());
    }

    let entrypoint = Path::new(manifest_dir).join("runtime/src/runner.ts");

    let status = Command::new(esbuild)
        .arg(entrypoint)
        .args(["--bundle", "--platform=node", "--format=cjs"])
        .arg(format!("--outfile={}", bundled_runner.display()))
        .status()?;

    if !status.success() {
        return Err("failed to bundle test runner with esbuild".into());
    }

    Ok(())
}

fn target_dir() -> Result<PathBuf, Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let out_path = Path::new(&out_dir);

    let target = out_path
        .ancestors()
        .nth(4)
        .ok_or("failed to derive Cargo target directory from OUT_DIR")?;

    Ok(target.to_path_buf())
}
