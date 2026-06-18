//! Terraphim AI Release Validation System
//!
//! This crate provides comprehensive validation capabilities for Terraphim AI releases,
//! including download testing, installation validation, functional verification, and
//! security scanning across multiple platforms and package formats.

// The testing module uses glob re-exports so its sub-modules are accessible via
// `crate::testing::*`. Multiple sub-modules export types with the same name
// (e.g. `Result`), which triggers this lint. The ambiguity is intentional.
#![allow(ambiguous_glob_reexports)]

pub mod artifacts;
pub mod orchestrator;
pub mod performance;
pub mod reporting;
pub mod testing;
pub mod validators;

// Re-export core components for easier access
pub use artifacts::{ArtifactType, Platform, ReleaseArtifact};
pub use orchestrator::ValidationOrchestrator;
pub use reporting::{ReportFormat, ValidationReport};
pub use validators::{ValidationResult, ValidationStatus};

use anyhow::Result;

/// Main validation system entry point
pub struct ValidationSystem {
    orchestrator: ValidationOrchestrator,
}

impl ValidationSystem {
    /// Create a new validation system instance
    pub fn new() -> Result<Self> {
        let orchestrator = ValidationOrchestrator::new()?;
        Ok(Self { orchestrator })
    }

    /// Run complete validation for a release
    pub async fn validate_release(&self, version: &str) -> Result<ValidationReport> {
        self.orchestrator.validate_release(version).await
    }

    /// Run specific validation categories
    pub async fn validate_categories(
        &self,
        version: &str,
        categories: Vec<String>,
    ) -> Result<ValidationReport> {
        self.orchestrator
            .validate_categories(version, categories)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::ValidationStatus;

    #[tokio::test]
    async fn test_validation_system_creation() {
        let system = ValidationSystem::new();
        assert!(system.is_ok(), "ValidationSystem::new() must succeed");
    }

    #[test]
    fn test_validation_status_is_success() {
        assert!(ValidationStatus::Passed.is_success());
        assert!(!ValidationStatus::Failed.is_success());
        assert!(!ValidationStatus::Pending.is_success());
    }

    #[test]
    fn test_validation_status_is_failure() {
        assert!(ValidationStatus::Failed.is_failure());
        assert!(ValidationStatus::Error.is_failure());
        assert!(!ValidationStatus::Passed.is_failure());
    }

    #[test]
    fn test_validation_status_is_final() {
        assert!(ValidationStatus::Passed.is_final());
        assert!(ValidationStatus::Failed.is_final());
        assert!(ValidationStatus::Skipped.is_final());
        assert!(ValidationStatus::Error.is_final());
        assert!(!ValidationStatus::Pending.is_final());
        assert!(!ValidationStatus::InProgress.is_final());
    }

    #[test]
    fn test_validation_status_display() {
        assert_eq!(ValidationStatus::Passed.to_string(), "Passed");
        assert_eq!(ValidationStatus::Failed.to_string(), "Failed");
        assert_eq!(ValidationStatus::Pending.to_string(), "Pending");
        assert_eq!(ValidationStatus::InProgress.to_string(), "InProgress");
        assert_eq!(ValidationStatus::Skipped.to_string(), "Skipped");
        assert_eq!(ValidationStatus::Error.to_string(), "Error");
    }

    #[test]
    fn test_validation_categories_accepts_empty() {
        // validate_categories is an async fn on the system; verify the type compiles and
        // the system can be constructed before calling it.
        let system = ValidationSystem::new().expect("system creation");
        let _f = system.validate_categories("0.0.0", vec![]);
        // The future is created without panic — sufficient for a compilation-time check.
    }
}
