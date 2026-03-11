use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;

#[must_use]
pub fn derive(project_root: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        STANDARD_NO_PAD.encode(project_root.as_os_str().as_bytes())
    }

    #[cfg(not(unix))]
    {
        STANDARD_NO_PAD.encode(project_root.to_string_lossy().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_base64_without_padding() {
        let id = derive(Path::new("/srv/project"));

        assert_eq!(id, "L3Nydi9wcm9qZWN0");
        assert!(!id.contains('='));
    }

    #[test]
    fn derivation_is_deterministic() {
        let path = Path::new("/tmp/my-project");

        let first = derive(path);
        let second = derive(path);

        assert_eq!(first, second);
    }
}
