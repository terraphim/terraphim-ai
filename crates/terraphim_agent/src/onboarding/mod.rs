//! CLI Onboarding Wizard for terraphim-agent
//!
//! Provides interactive setup wizard for first-time users, supporting:
//! - Quick start templates (Terraphim Engineer, LLM Enforcer, etc.)
//! - Custom role configuration with haystacks, LLM, and knowledge graphs
//! - Add-role capability to extend existing configuration
//!
//! # Example
//!
//! ```bash
//! # Interactive setup
//! terraphim-agent setup
//!
//! # Apply template directly
//! terraphim-agent setup --template terraphim-engineer
//!
//! # Add role to existing config
//! terraphim-agent setup --add-role
//! ```

mod prompts;
mod templates;
mod validation;
mod wizard;

pub use templates::{ConfigTemplate, TemplateRegistry};
pub use wizard::{apply_template, run_setup_wizard, SetupMode, SetupResult};

use thiserror::Error;

/// Errors that can occur during onboarding
#[derive(Debug, Error)]
pub enum OnboardingError {
    /// User cancelled the setup wizard
    #[error("User cancelled setup")]
    Cancelled,

    /// Requested template was not found
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// Configuration validation failed
    #[error("Validation failed: {0}")]
    Validation(String),

    /// IO error during file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Not running in a TTY - interactive mode requires a terminal
    #[error("Not a TTY - interactive mode requires a terminal. Use --template for non-interactive mode.")]
    NotATty,

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// User went back in wizard navigation
    #[error("User navigated back")]
    NavigateBack,

    /// Dialoguer prompt error
    #[error("Prompt error: {0}")]
    Prompt(String),
}

impl From<dialoguer::Error> for OnboardingError {
    fn from(err: dialoguer::Error) -> Self {
        // Check if the error indicates user cancellation (Ctrl+C)
        if err.to_string().contains("interrupted") {
            OnboardingError::Cancelled
        } else {
            OnboardingError::Prompt(err.to_string())
        }
    }
}

/// List all available templates
pub fn list_templates() -> Vec<ConfigTemplate> {
    TemplateRegistry::new().list().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates_returns_templates() {
        let templates = list_templates();
        assert!(!templates.is_empty(), "Should have at least one template");
    }

    #[test]
    fn test_onboarding_error_display() {
        let err = OnboardingError::Cancelled;
        assert_eq!(err.to_string(), "User cancelled setup");

        let err = OnboardingError::TemplateNotFound("foo".into());
        assert_eq!(err.to_string(), "Template not found: foo");
    }
}
