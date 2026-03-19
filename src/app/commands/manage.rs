use crate::core::opencode::server::DEFAULT_BASE_URL;

/// # Errors
/// Returns an error if the browser fails to open.
pub fn run() -> anyhow::Result<()> {
    open()
}

/// # Errors
/// Returns an error if the browser fails to open.
pub fn open() -> anyhow::Result<()> {
    println!("Opening {DEFAULT_BASE_URL}/");
    open::that(format!("{DEFAULT_BASE_URL}/"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_url_is_root_server_path() {
        let url = format!("{DEFAULT_BASE_URL}/");
        assert_eq!(url, "http://127.0.0.1:4096/");
    }
}
