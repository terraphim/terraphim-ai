#![cfg(feature = "release-integration-tests")]

use crate::{
    artifacts::{ArtifactType, Platform, ReleaseArtifact},
    orchestrator::ValidationOrchestrator,
    testing::{create_mock_release_structure, create_temp_dir, create_test_artifact},
};
use anyhow::Result;

#[tokio::test]
async fn test_artifact_creation() {
    let artifact = create_test_artifact(
        "test-artifact",
        "1.0.0",
        Platform::LinuxX86_64,
        ArtifactType::Binary,
    );

    assert_eq!(artifact.name, "test-artifact");
    assert_eq!(artifact.version, "1.0.0");
    assert_eq!(artifact.platform, Platform::LinuxX86_64);
    assert_eq!(artifact.artifact_type, ArtifactType::Binary);
    assert_eq!(artifact.checksum, "abc123def456");
    assert_eq!(artifact.size_bytes, 1024);
    assert!(!artifact.is_available_locally());
}

#[tokio::test]
async fn test_orchestrator_creation() {
    let result = ValidationOrchestrator::new();
    assert!(result.is_ok());

    let orchestrator = result.unwrap();
    let config = orchestrator.get_config();
    assert_eq!(config.concurrent_validations, 4);
    assert_eq!(config.timeout_seconds, 1800);
}

#[tokio::test]
async fn test_mock_release_structure() -> Result<()> {
    let release_path = create_mock_release_structure("1.0.0")?;

    // Verify directory structure
    assert!(release_path.exists());
    let releases_dir = release_path.join("releases").join("1.0.0");
    assert!(releases_dir.exists());

    // Verify artifact files
    let artifacts = vec![
        "terraphim_server-linux-x86_64",
        "terraphim_server-macos-x86_64",
        "terraphim_server-windows-x86_64.exe",
        "terraphim-tui-linux-x86_64",
        "terraphim-tui-macos-x86_64",
        "terraphim-tui-windows-x86_64.exe",
    ];

    for artifact in artifacts {
        let path = releases_dir.join(artifact);
        assert!(path.exists(), "Artifact {} should exist", artifact);
    }

    // Verify checksums file
    let checksums_path = releases_dir.join("checksums.txt");
    assert!(checksums_path.exists());
    let checksums_content = std::fs::read_to_string(&checksums_path)?;
    assert!(checksums_content.contains("abc123def456"));

    Ok(())
}

#[tokio::test]
async fn test_validation_categories() -> Result<()> {
    let orchestrator = ValidationOrchestrator::new()?;

    // Test with valid categories
    let result = orchestrator
        .validate_categories(
            "1.0.0",
            vec!["download".to_string(), "installation".to_string()],
        )
        .await;

    assert!(result.is_ok());

    let report = result.unwrap();
    assert_eq!(report.version, "1.0.0");

    // Test with unknown category (should not fail)
    let result = orchestrator
        .validate_categories("1.0.0", vec!["unknown".to_string()])
        .await;

    assert!(result.is_ok());
}

#[test]
fn test_platform_string_representation() {
    assert_eq!(Platform::LinuxX86_64.as_str(), "x86_64-unknown-linux-gnu");
    assert_eq!(Platform::MacOSX86_64.as_str(), "x86_64-apple-darwin");
    assert_eq!(Platform::WindowsX86_64.as_str(), "x86_64-pc-windows-msvc");
}

#[test]
fn test_platform_families() {
    use crate::artifacts::PlatformFamily;

    assert_eq!(Platform::LinuxX86_64.family(), PlatformFamily::Linux);
    assert_eq!(Platform::LinuxAarch64.family(), PlatformFamily::Linux);
    assert_eq!(Platform::MacOSX86_64.family(), PlatformFamily::MacOS);
    assert_eq!(Platform::WindowsX86_64.family(), PlatformFamily::Windows);
}
