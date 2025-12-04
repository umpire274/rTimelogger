//! Path utilities: resolve config path, expand ~, validate absolute paths, etc.

use std::path::PathBuf;

pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(path.trim_start_matches("~/"));
    }
    PathBuf::from(path)
}

pub fn is_absolute(path: &str) -> bool {
    PathBuf::from(path).is_absolute()
}
