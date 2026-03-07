use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

pub fn resolve_node_bin() -> Result<PathBuf> {
    let exe = std::env::current_exe().map_err(|e| anyhow!("failed to resolve executable: {e}"))?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| anyhow!("failed to resolve executable directory"))?;
    resolve_node_bin_from(exe_dir.to_path_buf())
}

pub fn resolve_node_bin_from(exe_dir: PathBuf) -> Result<PathBuf> {
    if let Ok(override_bin) = std::env::var("AGENTCTL_NODE_BIN") {
        return Ok(PathBuf::from(override_bin));
    }

    let bundled = bundled_node_path(&exe_dir);
    if bundled.exists() {
        return Ok(bundled);
    }

    Err(anyhow!(
        "bundled Node runtime not found at {}; set AGENTCTL_NODE_BIN to override",
        bundled.display()
    ))
}

fn bundled_node_path(exe_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        exe_dir.join("node").join("node.exe")
    } else {
        exe_dir.join("node").join("bin").join("node")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn env_override_is_respected() {
        // SAFETY: test binary is single-threaded; no parallel env mutation.
        unsafe { std::env::set_var("AGENTCTL_NODE_BIN", "/usr/bin/node") };

        let result = resolve_node_bin_from(PathBuf::from("/tmp/agentctl-test"));

        unsafe { std::env::remove_var("AGENTCTL_NODE_BIN") };
        assert_eq!(result.unwrap(), PathBuf::from("/usr/bin/node"));
    }

    #[test]
    fn errors_when_bundled_binary_missing_and_no_env_override() {
        unsafe { std::env::remove_var("AGENTCTL_NODE_BIN") };

        let err = resolve_node_bin_from(PathBuf::from("/tmp/agentctl-test-nonexistent"))
            .expect_err("should error when bundled node is missing");

        assert!(err.to_string().contains("bundled Node runtime not found"));
    }
}
