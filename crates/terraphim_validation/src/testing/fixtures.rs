//! Test fixtures for validation system testing

use crate::artifacts::{ArtifactType, Platform, ReleaseArtifact};
use crate::testing::{Result, ValidationResult, ValidationStatus};

/// Create a mock release artifact for testing
pub fn create_test_artifact(
    name: &str,
    version: &str,
    platform: Platform,
    artifact_type: ArtifactType,
) -> ReleaseArtifact {
    ReleaseArtifact::new(
        name.to_string(),
        version.to_string(),
        platform,
        artifact_type,
        format!("https://example.com/releases/{}/{}", version, name),
        "abc123def456".to_string(),
        1024,
    )
}

/// Create a set of test artifacts for different platforms
pub fn create_test_artifact_set(version: &str) -> Vec<ReleaseArtifact> {
    vec![
        create_test_artifact(
            "terraphim_server",
            version,
            Platform::LinuxX86_64,
            ArtifactType::Binary,
        ),
        create_test_artifact(
            "terraphim_server",
            version,
            Platform::MacOSX86_64,
            ArtifactType::Binary,
        ),
        create_test_artifact(
            "terraphim_server",
            version,
            Platform::WindowsX86_64,
            ArtifactType::Exe,
        ),
        create_test_artifact(
            "terraphim_tui",
            version,
            Platform::LinuxX86_64,
            ArtifactType::Binary,
        ),
        create_test_artifact(
            "terraphim_tui",
            version,
            Platform::MacOSX86_64,
            ArtifactType::Binary,
        ),
        create_test_artifact(
            "terraphim_tui",
            version,
            Platform::WindowsX86_64,
            ArtifactType::Exe,
        ),
    ]
}

/// Create a mock validation result for testing
pub fn create_test_result(
    name: &str,
    category: &str,
    status: ValidationStatus,
) -> ValidationResult {
    let mut result = ValidationResult::new(name.to_string(), category.to_string());
    match status {
        ValidationStatus::Passed => result.pass(100),
        ValidationStatus::Failed => {
            result.fail(100, vec![]);
        }
        ValidationStatus::Skipped => result.skip("Test skip".to_string()),
        ValidationStatus::Error => result.error(100, "Test error".to_string()),
        _ => result.start(),
    }
    result
}
