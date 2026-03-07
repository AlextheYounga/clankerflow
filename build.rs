use std::env;
use std::process::Command;

const NODE_EXTERNALS: &[&str] = &[
    "fs",
    "path",
    "child_process",
    "readline",
    "util",
    "os",
    "events",
    "crypto",
    "node:*",
];

fn main() {
    println!("cargo:rerun-if-changed=runtime/src");
    println!("cargo:rerun-if-changed=runtime/package.json");
    println!("cargo:rerun-if-changed=runtime/package-lock.json");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    bundle_ts(
        &manifest_dir,
        "runtime/src/runner.ts",
        "src/kit/.agentctl/lib/runner.js",
    );
    bundle_ts(
        &manifest_dir,
        "runtime/src/helpers.ts",
        "src/kit/.agentctl/lib/helpers.js",
    );
}

fn bundle_ts(manifest_dir: &str, entry: &str, output: &str) {
    let entry_path = format!("{manifest_dir}/{entry}");
    let output_path = format!("{manifest_dir}/{output}");
    let runtime_prefix = format!("{manifest_dir}/runtime");

    let mut args = vec![
        "--prefix".to_string(),
        runtime_prefix,
        "esbuild".to_string(),
        entry_path,
        "--bundle".to_string(),
        "--platform=node".to_string(),
        "--format=esm".to_string(),
        "--target=node22".to_string(),
        "--banner:js=import { createRequire } from 'module'; const require = createRequire(import.meta.url);".to_string(),
        format!("--outfile={output_path}"),
    ];

    for module in NODE_EXTERNALS {
        args.push(format!("--external:{module}"));
    }

    let status = Command::new("npx")
        .args(&args)
        .status()
        .expect("failed to run esbuild; ensure `npm install` has been run in runtime/");

    assert!(status.success(), "esbuild failed for {entry}");
}
