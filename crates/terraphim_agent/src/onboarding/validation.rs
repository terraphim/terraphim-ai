//! Configuration validation utilities
//!
//! Validates roles, haystacks, and knowledge graph configurations
//! before saving to ensure they are well-formed.

use std::path::Path;
use terraphim_config::{Haystack, KnowledgeGraph, Role, ServiceType};
use thiserror::Error;

/// Validation errors that can occur
#[derive(Debug, Error, Clone)]
#[allow(dead_code)]
pub enum ValidationError {
    /// A required field is empty
    #[error("Field '{0}' cannot be empty")]
    EmptyField(String),

    /// Role has no haystacks configured
    #[error("Role must have at least one haystack")]
    MissingHaystack,

    /// Haystack location is invalid
    #[error("Invalid haystack location: {0}")]
    InvalidLocation(String),

    /// Service type requires specific configuration
    #[error("Service {0} requires: {1}")]
    ServiceRequirement(String, String),

    /// URL is malformed
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Knowledge graph configuration is invalid
    #[error("Invalid knowledge graph: {0}")]
    InvalidKnowledgeGraph(String),
}

/// Validate a role configuration
///
/// # Returns
/// - `Ok(())` if validation passes
/// - `Err(Vec<ValidationError>)` if any validations fail
pub fn validate_role(role: &Role) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Role name must not be empty
    if role.name.to_string().trim().is_empty() {
        errors.push(ValidationError::EmptyField("name".into()));
    }

    // Must have at least one haystack
    if role.haystacks.is_empty() {
        errors.push(ValidationError::MissingHaystack);
    }

    // Validate each haystack
    for haystack in &role.haystacks {
        if let Err(e) = validate_haystack(haystack) {
            errors.push(e);
        }
    }

    // Validate knowledge graph if present
    if let Some(ref kg) = role.kg {
        if let Err(e) = validate_knowledge_graph(kg) {
            errors.push(e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a haystack configuration
pub fn validate_haystack(haystack: &Haystack) -> Result<(), ValidationError> {
    // Location must not be empty
    if haystack.location.trim().is_empty() {
        return Err(ValidationError::EmptyField("location".into()));
    }

    // Service-specific validation
    match haystack.service {
        ServiceType::Ripgrep => {
            // For Ripgrep, location should be a path (we don't validate existence here,
            // that's done separately with path_exists check if needed)
            // Just ensure it's not a URL
            if haystack.location.starts_with("http://") || haystack.location.starts_with("https://")
            {
                return Err(ValidationError::InvalidLocation(
                    "Ripgrep requires a local path, not a URL".into(),
                ));
            }
        }
        ServiceType::QueryRs => {
            // QueryRs can be URL or default
            // No specific validation needed
        }
        ServiceType::Quickwit => {
            // Quickwit requires a URL
            if !haystack.location.starts_with("http://")
                && !haystack.location.starts_with("https://")
            {
                return Err(ValidationError::ServiceRequirement(
                    "Quickwit".into(),
                    "URL (http:// or https://)".into(),
                ));
            }
        }
        ServiceType::Atomic => {
            // Atomic requires a URL
            if !haystack.location.starts_with("http://")
                && !haystack.location.starts_with("https://")
            {
                return Err(ValidationError::ServiceRequirement(
                    "Atomic".into(),
                    "URL (http:// or https://)".into(),
                ));
            }
        }
        _ => {
            // Other services - basic validation only
        }
    }

    Ok(())
}

/// Validate knowledge graph configuration
pub fn validate_knowledge_graph(kg: &KnowledgeGraph) -> Result<(), ValidationError> {
    // Must have either automata_path or knowledge_graph_local
    let has_remote = kg.automata_path.is_some();
    let has_local = kg.knowledge_graph_local.is_some();

    if !has_remote && !has_local {
        return Err(ValidationError::InvalidKnowledgeGraph(
            "Must specify either remote automata URL or local knowledge graph path".into(),
        ));
    }

    // Validate local path format if present
    if let Some(ref local) = kg.knowledge_graph_local {
        if local.path.as_os_str().is_empty() {
            return Err(ValidationError::InvalidKnowledgeGraph(
                "Local knowledge graph path cannot be empty".into(),
            ));
        }
    }

    Ok(())
}

/// Check if a path exists on the filesystem
///
/// Handles tilde expansion for home directory
pub fn path_exists(path: &str) -> bool {
    let expanded = expand_tilde(path);
    Path::new(&expanded).exists()
}

/// Expand tilde (~) to home directory
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", home.to_string_lossy().as_ref(), 1);
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Validate that a URL is well-formed
pub fn validate_url(url: &str) -> Result<(), ValidationError> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ValidationError::InvalidUrl(format!(
            "URL must start with http:// or https://: {}",
            url
        )));
    }

    // Basic URL structure check
    if url.len() < 10 {
        return Err(ValidationError::InvalidUrl(format!(
            "URL is too short: {}",
            url
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::RoleName;

    fn create_test_role(name: &str) -> Role {
        let mut role = Role::new(name);
        role.haystacks = vec![Haystack {
            location: "/some/path".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        }];
        role
    }

    #[test]
    fn test_validate_role_valid() {
        let role = create_test_role("Test Role");
        assert!(validate_role(&role).is_ok());
    }

    #[test]
    fn test_validate_role_empty_name() {
        let mut role = create_test_role("");
        // Role::new doesn't allow truly empty names, but we can test with whitespace
        role.name = RoleName::new("   ");
        let result = validate_role(&role);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::EmptyField(_)))
        );
    }

    #[test]
    fn test_validate_role_missing_haystack() {
        let mut role = create_test_role("Test Role");
        role.haystacks.clear();
        let result = validate_role(&role);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::MissingHaystack))
        );
    }

    #[test]
    fn test_validate_haystack_valid_ripgrep() {
        let haystack = Haystack {
            location: "/some/path".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };
        assert!(validate_haystack(&haystack).is_ok());
    }

    #[test]
    fn test_validate_haystack_ripgrep_rejects_url() {
        let haystack = Haystack {
            location: "https://example.com".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };
        let result = validate_haystack(&haystack);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_haystack_quickwit_requires_url() {
        let haystack = Haystack {
            location: "/local/path".to_string(),
            service: ServiceType::Quickwit,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };
        let result = validate_haystack(&haystack);
        assert!(result.is_err());

        // Valid Quickwit config
        let haystack_valid = Haystack {
            location: "http://localhost:7280".to_string(),
            service: ServiceType::Quickwit,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };
        assert!(validate_haystack(&haystack_valid).is_ok());
    }

    #[test]
    fn test_validate_haystack_empty_location() {
        let haystack = Haystack {
            location: "".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };
        let result = validate_haystack(&haystack);
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_tilde() {
        // Test that tilde expansion works (result depends on actual home dir)
        let expanded = expand_tilde("~/Documents");
        assert!(!expanded.starts_with("~") || dirs::home_dir().is_none());
    }

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://example.com/api").is_ok());
        assert!(validate_url("http://localhost:8080").is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("http://").is_err());
    }
}
