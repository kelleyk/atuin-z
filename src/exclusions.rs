use anyhow::{Context, Result};
use std::path::PathBuf;

/// Resolve the path to the exclusions file.
///
/// Uses `XDG_DATA_HOME` if set, otherwise `~/.local/share/atuin-z/exclusions`.
pub fn exclusions_path() -> Result<PathBuf> {
    let base = if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg)
    } else {
        let home = dirs::home_dir().context("could not determine home directory")?;
        home.join(".local").join("share")
    };
    Ok(base.join("atuin-z").join("exclusions"))
}

/// Load the exclusion list from disk. Returns an empty vec if the file doesn't exist.
pub fn load() -> Result<Vec<String>> {
    let path = exclusions_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read exclusions file: {}", path.display()))?;
    Ok(content
        .lines()
        .map(|l| l.to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// Add a path to the exclusion list. Creates the file and parent directories if needed.
pub fn add(dir: &str) -> Result<()> {
    let path = exclusions_path()?;
    let mut entries = load()?;

    if entries.iter().any(|e| e == dir) {
        // Already excluded
        return Ok(());
    }

    entries.push(dir.to_string());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }

    std::fs::write(&path, entries.join("\n") + "\n")
        .with_context(|| format!("failed to write exclusions file: {}", path.display()))?;

    Ok(())
}

/// Check if a directory is in the exclusion list.
pub fn is_excluded(dir: &str, exclusions: &[String]) -> bool {
    exclusions.iter().any(|e| e == dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_excluded_matches_exact_path() {
        let exclusions = vec!["/home/user/secret".to_string()];
        assert!(is_excluded("/home/user/secret", &exclusions));
    }

    #[test]
    fn is_excluded_no_match() {
        let exclusions = vec!["/home/user/secret".to_string()];
        assert!(!is_excluded("/home/user/public", &exclusions));
    }

    #[test]
    fn is_excluded_empty_list() {
        assert!(!is_excluded("/anything", &[]));
    }

    #[test]
    fn is_excluded_does_not_match_subdirectories() {
        let exclusions = vec!["/home/user".to_string()];
        assert!(!is_excluded("/home/user/child", &exclusions));
    }

    #[test]
    fn is_excluded_does_not_match_partial_path() {
        let exclusions = vec!["/home/user/proj".to_string()];
        assert!(!is_excluded("/home/user/project", &exclusions));
    }
}
