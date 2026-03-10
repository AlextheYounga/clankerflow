use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=runtime/src");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let src = Path::new(&manifest_dir).join("runtime/src");
    let dst = Path::new(&manifest_dir).join("src/kit/.agentkata/lib/src");

    copy_dir_all(&src, &dst)?;

    Ok(())
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
