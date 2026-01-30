//! Terraphim AI Release Validation System
//!
//! This crate provides comprehensive validation capabilities for Terraphim AI releases,
//! including download testing, installation validation, functional verification, and
//! security scanning across multiple platforms and package formats.

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

    #[tokio::test]
    async fn test_validation_system_creation() {
        let system = ValidationSystem::new().unwrap();
        assert!(true); // Basic creation test
    }
}
