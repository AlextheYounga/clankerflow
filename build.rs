use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=runtime/src");
    println!("cargo:rerun-if-changed=runtime/workflows");
    println!("cargo:rerun-if-changed=runtime/package.json");
    println!("cargo:rerun-if-changed=runtime/package-lock.json");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let src = Path::new(&manifest_dir).join("runtime/src");
    let dst = Path::new(&manifest_dir).join("src/kit/.clankerflow/lib/src");
    let workflows_src = Path::new(&manifest_dir).join("runtime/workflows");
    let workflows_dst = Path::new(&manifest_dir).join("src/kit/workflows");
    let test_runtime_dir = target_dir()?.join("test-runtime");
    let bundled_runner = test_runtime_dir.join("runner.cjs");

    copy_dir_all(&src, &dst)?;
    copy_dir_all(&workflows_src, &workflows_dst)?;
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

fn target_dir() -> Result<std::path::PathBuf, Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let out_path = Path::new(&out_dir);

    let target = out_path
        .ancestors()
        .nth(4)
        .ok_or("failed to derive Cargo target directory from OUT_DIR")?;

    Ok(target.to_path_buf())
}

// Recursively copy all files from src into dst, creating directories as needed.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
