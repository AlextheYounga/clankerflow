use std::path::Path;

use crate::core::codebase_id;
use crate::core::opencode_config::OpencodeConfig;
use crate::core::project::require_project_root;

const DEFAULT_OPENCODE_URL: &str = "http://127.0.0.1:4096";

/// # Errors
/// Returns an error if the project root is not found, `OpenCode` config fails to load,
/// or the browser fails to open.
pub fn run() -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    open_for_project_root(&project_root)
}

/// # Errors
/// Returns an error if `OpenCode` config parsing fails or the browser fails to open.
pub fn open_for_project_root(project_root: &Path) -> anyhow::Result<()> {
    let config = OpencodeConfig::load_optional(project_root)?;

    let server_url = config
        .as_ref()
        .and_then(|cfg| cfg.server_url.as_deref())
        .unwrap_or(DEFAULT_OPENCODE_URL);

    let url = build_manage_url(server_url, project_root);
    println!("Opening {url}");
    open::that(&url)?;
    Ok(())
}

fn build_manage_url(server_url: &str, project_root: &Path) -> String {
    let encoded = codebase_id::derive(project_root);
    format!("{}/{}/sessions", server_url.trim_end_matches('/'), encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_manage_url_encodes_project_path_as_base64() {
        let url = build_manage_url("http://127.0.0.1:4096", Path::new("/home/alex/project"));

        assert!(url.starts_with("http://127.0.0.1:4096/"));
        assert!(url.ends_with("/sessions"));

        let encoded = codebase_id::derive(Path::new("/home/alex/project"));
        assert_eq!(url, format!("http://127.0.0.1:4096/{encoded}/sessions"));
    }

    #[test]
    fn build_manage_url_strips_trailing_slash_from_server_url() {
        let url = build_manage_url("http://127.0.0.1:4096/", Path::new("/tmp/repo"));

        assert_eq!(url, "http://127.0.0.1:4096/L3RtcC9yZXBv/sessions");
    }

    #[test]
    fn build_manage_url_produces_known_url_for_known_path() {
        let url = build_manage_url("http://127.0.0.1:4096", Path::new("/srv/project"));

        assert_eq!(url, "http://127.0.0.1:4096/L3Nydi9wcm9qZWN0/sessions");
    }

    #[test]
    fn build_manage_url_works_with_custom_server_url() {
        let url = build_manage_url("http://10.0.0.5:8080", Path::new("/tmp/project"));

        assert!(url.starts_with("http://10.0.0.5:8080/"));
        assert!(url.ends_with("/sessions"));
    }
}
