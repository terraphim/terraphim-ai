use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectDiscoveryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Not a directory: {0}")]
    NotDirectory(PathBuf),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub global_shortcut: Option<String>,
    #[serde(default)]
    pub roles: std::collections::HashMap<String, crate::Role>,
}

impl ProjectConfig {
    pub fn from_file(path: &Path) -> Result<Self, ProjectDiscoveryError> {
        let content = std::fs::read_to_string(path)?;
        let config: ProjectConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}

pub fn discover(start_dir: Option<&Path>) -> Result<Option<PathBuf>, ProjectDiscoveryError> {
    let start_dir = match start_dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()?,
    };

    let mut current = Some(start_dir);

    while let Some(dir) = current {
        let terraphim_dir = dir.join(".terraphim");
        if terraphim_dir.is_dir() {
            let canonical = terraphim_dir.canonicalize()?;
            return Ok(Some(canonical));
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_dir_with_structure(base: &TempDir, structure: &[&str]) -> PathBuf {
        let base_path = base.path().to_path_buf();
        for path in structure {
            let full_path = base_path.join(path);
            if path.ends_with('/') {
                fs::create_dir_all(&full_path).unwrap();
            } else {
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&full_path, "{}").unwrap();
            }
        }
        base_path
    }

    #[test]
    fn test_discover_finds_terraphim_dir() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &["work/", "work/.terraphim/", "work/src/"]);
        let result = discover(Some(&base.join("work/src"))).unwrap();
        let expected = std::fs::canonicalize(base.join("work/.terraphim")).unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_discover_not_found() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &["src/", "src/main.rs"]);
        let result = discover(Some(&base.join("src"))).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_discover_from_current_dir() {
        let original_dir = std::env::current_dir().unwrap();
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &[".terraphim/"]);
        std::env::set_current_dir(&base).unwrap();
        let result = discover(None).unwrap();
        let expected = std::fs::canonicalize(base.join(".terraphim")).unwrap();
        assert_eq!(result, Some(expected));
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_discover_upwards_search() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(
            &temp,
            &["project/", "project/.terraphim/", "project/src/main.rs"],
        );
        let result = discover(Some(&base.join("project/src"))).unwrap();
        let expected = std::fs::canonicalize(base.join("project/.terraphim")).unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_discover_multiple_levels_up() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(
            &temp,
            &[
                "a/",
                "a/b/",
                "a/b/c/",
                "a/b/c/.terraphim/",
                "a/b/c/src/main.rs",
            ],
        );
        let result = discover(Some(&base.join("a/b/c/src"))).unwrap();
        let expected = std::fs::canonicalize(base.join("a/b/c/.terraphim")).unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_project_config_from_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.json");
        let json = r#"{"global_shortcut": "Ctrl+Shift+T", "roles": {}}"#;
        fs::write(&config_path, json).unwrap();
        let config = ProjectConfig::from_file(&config_path).unwrap();
        assert_eq!(config.global_shortcut, Some("Ctrl+Shift+T".to_string()));
    }

    #[test]
    fn test_project_config_from_file_empty() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();
        let config = ProjectConfig::from_file(&config_path).unwrap();
        assert_eq!(config.global_shortcut, None);
        assert!(config.roles.is_empty());
    }

    #[test]
    fn test_discover_returns_none_for_missing() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &["src/", "src/main.rs"]);
        let result = discover(Some(&base.join("src"))).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_discover_root_finds_terraphim() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &[".terraphim/"]);
        let result = discover(Some(&base)).unwrap();
        let expected = std::fs::canonicalize(base.join(".terraphim")).unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_discover_symlink_to_real_dir() {
        let temp = TempDir::new().unwrap();
        let real = temp.path().join("real");
        let linked = temp.path().join("linked");
        fs::create_dir_all(real.join(".terraphim")).unwrap();
        fs::create_dir_all(real.join("src")).unwrap();
        std::os::unix::fs::symlink(&real, &linked).unwrap();
        let canonical = std::fs::canonicalize(real.join(".terraphim")).unwrap();
        let result = discover(Some(&linked.join("src"))).unwrap();
        assert_eq!(result, Some(canonical));
    }
}
