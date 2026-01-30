//! Testing utilities

use anyhow::Result;
use std::path::{Path, PathBuf};
use tempfile::{NamedTempFile, TempDir};

/// Create a temporary directory for testing
pub fn create_temp_dir() -> Result<TempDir> {
    Ok(TempDir::new()?)
}

/// Create a temporary file with content
pub fn create_temp_file(content: &str) -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;
    std::io::Write::write_all(&mut file, content.as_bytes())?;
    Ok(file)
}

/// Assert that two files have the same content
pub fn assert_files_equal<P1: AsRef<Path>, P2: AsRef<Path>>(path1: P1, path2: P2) -> Result<()> {
    let content1 = std::fs::read_to_string(path1)?;
    let content2 = std::fs::read_to_string(path2)?;

    if content1 != content2 {
        anyhow::bail!("File contents differ");
    }

    Ok(())
}

/// Create test configuration for validation
pub fn create_test_config() -> crate::orchestrator::ValidationConfig {
    crate::orchestrator::ValidationConfig {
        download_dir: "/tmp/test-downloads".to_string(),
        concurrent_validations: 2,
        timeout_seconds: 300,
        enabled_platforms: vec![
            crate::artifacts::Platform::LinuxX86_64,
            crate::artifacts::Platform::MacOSX86_64,
        ],
        enabled_categories: vec!["download".to_string(), "installation".to_string()],
        notification_webhook: None,
    }
}

/// Create a mock release artifact for testing
pub fn create_mock_release_structure(version: &str) -> Result<PathBuf> {
    let temp_dir = create_temp_dir()?;
    let releases_dir = temp_dir.path().join("releases").join(version);
    std::fs::create_dir_all(&releases_dir)?;

    // Create mock artifacts
    let artifacts: Vec<(&str, &str)> = vec![
        ("terraphim_server-linux-x86_64", "binary"),
        ("terraphim_server-macos-x86_64", "binary"),
        ("terraphim_server-windows-x86_64.exe", "exe"),
        ("terraphim-tui-linux-x86_64", "binary"),
        ("terraphim-tui-macos-x86_64", "binary"),
        ("terraphim-tui-windows-x86_64.exe", "exe"),
    ];

    for (filename, artifact_type) in &artifacts {
        let path = releases_dir.join(filename);
        std::fs::write(&path, format!("Mock {} content", artifact_type))?;
    }

    // Create checksum file
    let checksums_path = releases_dir.join("checksums.txt");
    let checksums_content = artifacts
        .iter()
        .map(|(filename, _)| format!("{}  abc123def456", filename))
        .collect::<Vec<String>>()
        .join("\n");
    std::fs::write(&checksums_path, checksums_content)?;

    Ok(temp_dir.keep().as_path().to_path_buf())
}
