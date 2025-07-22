/// Feature flag management for Terraphim build configurations
///
/// This module handles feature flags that can be enabled or disabled during
/// build processes, providing validation and consistent feature set management.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Set of feature flags that can be enabled during build
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FeatureSet {
    /// List of explicitly enabled features
    pub enabled: Vec<String>,
    
    /// Whether to include default features from Cargo.toml
    pub default_features: bool,
    
    /// Feature dependencies and constraints
    pub constraints: FeatureConstraints,
}

/// Feature constraints for validation and dependency management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FeatureConstraints {
    /// Features that are mutually exclusive (cannot be enabled together)
    pub mutually_exclusive: Vec<Vec<String>>,
    
    /// Required dependencies for specific features
    pub dependencies: HashMap<String, Vec<String>>,
    
    /// Features that require specific target architectures
    pub target_specific: HashMap<String, Vec<String>>,
    
    /// Features that are only available in certain environments
    pub environment_specific: HashMap<String, Vec<String>>,
}

/// Well-known feature categories for Terraphim project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureCategory {
    /// Core application features
    Core,
    /// AI and machine learning features
    AI,
    /// User interface features
    UI,
    /// Backend service features
    Backend,
    /// Development and debugging features
    Development,
    /// Integration features
    Integration,
    /// Platform-specific features
    Platform,
    /// Experimental features
    Experimental,
}

impl FeatureSet {
    /// Creates a new empty feature set
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Creates a feature set with default features enabled
    pub fn with_defaults() -> Self {
        Self {
            enabled: Vec::new(),
            default_features: true,
            constraints: FeatureConstraints::default(),
        }
    }
    
    /// Creates a feature set from a slice of feature names
    pub fn from_slice(features: &[&str]) -> Self {
        Self {
            enabled: features.iter().map(|s| s.to_string()).collect(),
            default_features: true,
            constraints: FeatureConstraints::default(),
        }
    }
    
    /// Creates a feature set from a comma-separated string
    pub fn from_str(features: &str) -> Result<Self> {
        if features.trim().is_empty() {
            return Ok(Self::new());
        }
        
        let enabled: Vec<String> = features
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        Ok(Self {
            enabled,
            default_features: true,
            constraints: FeatureConstraints::default(),
        })
    }
    
    /// Adds a feature to the enabled set
    pub fn enable<S: Into<String>>(&mut self, feature: S) {
        let feature = feature.into();
        if !self.enabled.contains(&feature) {
            self.enabled.push(feature);
        }
    }
    
    /// Removes a feature from the enabled set
    pub fn disable<S: AsRef<str>>(&mut self, feature: S) {
        let feature = feature.as_ref();
        self.enabled.retain(|f| f != feature);
    }
    
    /// Checks if a feature is enabled
    pub fn is_enabled<S: AsRef<str>>(&self, feature: S) -> bool {
        self.enabled.contains(&feature.as_ref().to_string())
    }
    
    /// Returns true if no features are explicitly enabled
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }
    
    /// Returns the number of enabled features
    pub fn len(&self) -> usize {
        self.enabled.len()
    }
    
    /// Validates the feature set against its constraints
    pub fn validate(&self) -> Result<()> {
        // Check for mutually exclusive features
        for exclusive_group in &self.constraints.mutually_exclusive {
            let enabled_in_group: Vec<_> = exclusive_group
                .iter()
                .filter(|&f| self.is_enabled(f))
                .collect();
            
            if enabled_in_group.len() > 1 {
                return Err(Error::feature(format!(
                    "Mutually exclusive features enabled: {:?}",
                    enabled_in_group
                )));
            }
        }
        
        // Check feature dependencies
        for enabled_feature in &self.enabled {
            if let Some(deps) = self.constraints.dependencies.get(enabled_feature) {
                for dep in deps {
                    if !self.is_enabled(dep) {
                        return Err(Error::feature(format!(
                            "Feature '{}' requires '{}' to be enabled",
                            enabled_feature, dep
                        )));
                    }
                }
            }
        }
        
        // Validate feature names (no special characters, lowercase, etc.)
        for feature in &self.enabled {
            if !is_valid_feature_name(feature) {
                return Err(Error::feature(format!(
                    "Invalid feature name '{}': must be lowercase alphanumeric with hyphens or underscores",
                    feature
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validates features against a target architecture
    pub fn validate_for_target(&self, target: &str) -> Result<()> {
        for feature in &self.enabled {
            if let Some(supported_targets) = self.constraints.target_specific.get(feature) {
                if !supported_targets.iter().any(|t| target.contains(t)) {
                    return Err(Error::feature(format!(
                        "Feature '{}' is not supported on target '{}'",
                        feature, target
                    )));
                }
            }
        }
        Ok(())
    }
    
    /// Validates features against an environment
    pub fn validate_for_environment(&self, environment: &str) -> Result<()> {
        for feature in &self.enabled {
            if let Some(supported_envs) = self.constraints.environment_specific.get(feature) {
                if !supported_envs.contains(&environment.to_string()) {
                    return Err(Error::feature(format!(
                        "Feature '{}' is not supported in '{}' environment",
                        feature, environment
                    )));
                }
            }
        }
        Ok(())
    }
    
    /// Returns features by category
    pub fn features_by_category(&self, category: FeatureCategory) -> Vec<&String> {
        self.enabled
            .iter()
            .filter(|&feature| get_feature_category(feature) == category)
            .collect()
    }
    
    /// Merges another feature set into this one
    pub fn merge(&mut self, other: &FeatureSet) {
        for feature in &other.enabled {
            self.enable(feature.clone());
        }
        
        // Merge constraints
        self.constraints.mutually_exclusive.extend_from_slice(&other.constraints.mutually_exclusive);
        self.constraints.dependencies.extend(other.constraints.dependencies.clone());
        self.constraints.target_specific.extend(other.constraints.target_specific.clone());
        self.constraints.environment_specific.extend(other.constraints.environment_specific.clone());
    }
    
    /// Creates a feature set optimized for a specific environment
    pub fn for_environment(environment: &str) -> Self {
        let mut features = Self::with_defaults();
        
        // Add environment-specific features
        match environment {
            "development" | "dev" => {
                features.enable("dev-tools");
                features.enable("debug-info");
            }
            "testing" | "test" => {
                features.enable("test-utils");
                features.enable("mock-services");
            }
            "staging" => {
                features.enable("analytics");
                features.enable("monitoring");
            }
            "production" | "prod" => {
                features.enable("optimizations");
                features.enable("telemetry");
                features.default_features = false; // Minimize features in production
            }
            _ => {} // Keep defaults
        }
        
        features
    }
    
    /// Returns well-known Terraphim features
    pub fn terraphim_features() -> HashMap<String, String> {
        let mut features = HashMap::new();
        
        // Core features
        features.insert("typescript".to_string(), "TypeScript bindings for WASM".to_string());
        features.insert("native".to_string(), "Native platform optimizations".to_string());
        features.insert("remote-loading".to_string(), "Remote resource loading".to_string());
        
        // AI features
        features.insert("openrouter".to_string(), "OpenRouter AI integration".to_string());
        features.insert("llm".to_string(), "Large Language Model support".to_string());
        
        // Backend features
        features.insert("server".to_string(), "Server components".to_string());
        features.insert("api".to_string(), "HTTP API endpoints".to_string());
        features.insert("database".to_string(), "Database connectivity".to_string());
        
        // Development features
        features.insert("dev-tools".to_string(), "Development utilities".to_string());
        features.insert("debug-info".to_string(), "Debug information".to_string());
        features.insert("test-utils".to_string(), "Testing utilities".to_string());
        features.insert("mock-services".to_string(), "Mock service implementations".to_string());
        
        // Integration features
        features.insert("docker".to_string(), "Docker container support".to_string());
        features.insert("kubernetes".to_string(), "Kubernetes integration".to_string());
        features.insert("prometheus".to_string(), "Prometheus metrics".to_string());
        
        // Platform features
        features.insert("cross-compile".to_string(), "Cross-compilation support".to_string());
        features.insert("wasm".to_string(), "WebAssembly target".to_string());
        
        // Experimental features
        features.insert("experimental".to_string(), "Experimental functionality".to_string());
        features.insert("unstable".to_string(), "Unstable APIs".to_string());
        
        features
    }
}

impl FromStr for FeatureSet {
    type Err = Error;
    
    fn from_str(s: &str) -> Result<Self> {
        Self::from_str(s)
    }
}

impl std::fmt::Display for FeatureSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.enabled.is_empty() {
            write!(f, "(none)")
        } else {
            write!(f, "{}", self.enabled.join(","))
        }
    }
}

/// Validates that a feature name follows conventions
fn is_valid_feature_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    
    // Feature names should be lowercase, alphanumeric, with optional hyphens or underscores
    name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && !name.starts_with('-')
        && !name.starts_with('_')
        && !name.ends_with('-')
        && !name.ends_with('_')
}

/// Gets the category of a feature based on its name
fn get_feature_category(feature: &str) -> FeatureCategory {
    match feature {
        "typescript" | "native" | "wasm" => FeatureCategory::Core,
        "openrouter" | "llm" | "ai" => FeatureCategory::AI,
        "ui" | "frontend" | "svelte" | "tauri" => FeatureCategory::UI,
        "server" | "api" | "database" | "backend" => FeatureCategory::Backend,
        "dev-tools" | "debug-info" | "test-utils" | "mock-services" => FeatureCategory::Development,
        "docker" | "kubernetes" | "prometheus" | "telemetry" => FeatureCategory::Integration,
        "cross-compile" | "linux" | "windows" | "macos" => FeatureCategory::Platform,
        "experimental" | "unstable" => FeatureCategory::Experimental,
        _ => FeatureCategory::Core, // Default category
    }
}

impl FeatureConstraints {
    /// Creates default constraints for Terraphim project
    pub fn terraphim_constraints() -> Self {
        let mut constraints = Self::default();
        
        // Mutually exclusive features
        constraints.mutually_exclusive = vec![
            vec!["debug-info".to_string(), "optimizations".to_string()],
            vec!["mock-services".to_string(), "production".to_string()],
        ];
        
        // Feature dependencies
        constraints.dependencies.insert(
            "openrouter".to_string(),
            vec!["api".to_string(), "network".to_string()],
        );
        constraints.dependencies.insert(
            "kubernetes".to_string(),
            vec!["docker".to_string()],
        );
        constraints.dependencies.insert(
            "test-utils".to_string(),
            vec!["dev-tools".to_string()],
        );
        
        // Target-specific features
        constraints.target_specific.insert(
            "native".to_string(),
            vec!["linux".to_string(), "windows".to_string(), "macos".to_string()],
        );
        constraints.target_specific.insert(
            "wasm".to_string(),
            vec!["wasm32".to_string()],
        );
        
        // Environment-specific features
        constraints.environment_specific.insert(
            "debug-info".to_string(),
            vec!["development".to_string(), "testing".to_string()],
        );
        constraints.environment_specific.insert(
            "optimizations".to_string(),
            vec!["production".to_string(), "staging".to_string()],
        );
        constraints.environment_specific.insert(
            "mock-services".to_string(),
            vec!["development".to_string(), "testing".to_string()],
        );
        
        constraints
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature_set_creation() {
        let features = FeatureSet::from_slice(&["openrouter", "typescript"]);
        assert!(features.is_enabled("openrouter"));
        assert!(features.is_enabled("typescript"));
        assert!(!features.is_enabled("debug"));
        assert_eq!(features.len(), 2);
    }
    
    #[test]
    fn test_feature_set_from_string() {
        let features = FeatureSet::from_str("openrouter,typescript,native").unwrap();
        assert!(features.is_enabled("openrouter"));
        assert!(features.is_enabled("typescript"));
        assert!(features.is_enabled("native"));
        assert_eq!(features.len(), 3);
    }
    
    #[test]
    fn test_feature_validation() {
        let mut features = FeatureSet::new();
        features.enable("valid-feature");
        features.enable("another_feature");
        assert!(features.validate().is_ok());
        
        // Test invalid feature name
        features.enable("Invalid-Feature-Name!");
        assert!(features.validate().is_err());
    }
    
    #[test]
    fn test_feature_constraints() {
        let mut features = FeatureSet::new();
        features.constraints.mutually_exclusive = vec![
            vec!["debug-info".to_string(), "optimizations".to_string()],
        ];
        
        features.enable("debug-info");
        assert!(features.validate().is_ok());
        
        features.enable("optimizations");
        assert!(features.validate().is_err()); // Should fail due to mutual exclusion
    }
    
    #[test]
    fn test_feature_dependencies() {
        let mut features = FeatureSet::new();
        features.constraints.dependencies.insert(
            "openrouter".to_string(),
            vec!["api".to_string()],
        );
        
        features.enable("openrouter");
        assert!(features.validate().is_err()); // Should fail due to missing dependency
        
        features.enable("api");
        assert!(features.validate().is_ok()); // Should pass with dependency satisfied
    }
    
    #[test]
    fn test_feature_categories() {
        let features = FeatureSet::from_slice(&["openrouter", "typescript", "server"]);
        
        let ai_features = features.features_by_category(FeatureCategory::AI);
        assert_eq!(ai_features.len(), 1);
        assert!(ai_features.contains(&&"openrouter".to_string()));
        
        let core_features = features.features_by_category(FeatureCategory::Core);
        assert!(core_features.contains(&&"typescript".to_string()));
    }
    
    #[test]
    fn test_environment_specific_features() {
        let dev_features = FeatureSet::for_environment("development");
        assert!(dev_features.is_enabled("dev-tools"));
        assert!(dev_features.is_enabled("debug-info"));
        
        let prod_features = FeatureSet::for_environment("production");
        assert!(prod_features.is_enabled("optimizations"));
        assert!(prod_features.is_enabled("telemetry"));
        assert!(!prod_features.default_features);
    }
    
    #[test]
    fn test_feature_merge() {
        let mut features1 = FeatureSet::from_slice(&["openrouter"]);
        let features2 = FeatureSet::from_slice(&["typescript", "native"]);
        
        features1.merge(&features2);
        
        assert!(features1.is_enabled("openrouter"));
        assert!(features1.is_enabled("typescript"));
        assert!(features1.is_enabled("native"));
        assert_eq!(features1.len(), 3);
    }
    
    #[test]
    fn test_valid_feature_names() {
        assert!(is_valid_feature_name("valid-feature"));
        assert!(is_valid_feature_name("valid_feature"));
        assert!(is_valid_feature_name("feature123"));
        assert!(is_valid_feature_name("f"));
        
        assert!(!is_valid_feature_name(""));
        assert!(!is_valid_feature_name("-invalid"));
        assert!(!is_valid_feature_name("invalid-"));
        assert!(!is_valid_feature_name("_invalid"));
        assert!(!is_valid_feature_name("invalid_"));
        assert!(!is_valid_feature_name("Invalid"));
        assert!(!is_valid_feature_name("feature!"));
        assert!(!is_valid_feature_name("feature with spaces"));
    }
}
