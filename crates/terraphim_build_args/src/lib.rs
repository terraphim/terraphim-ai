pub mod cli;
/// Terraphim Build Argument Management
///
/// This crate provides comprehensive build argument management for the Terraphim AI project,
/// enabling centralized configuration of build features, targets, and deployment options.
///
/// # Key Features
///
/// - Feature flag management across workspace
/// - Build target configuration for multiple architectures
/// - Environment-specific build configurations
/// - Docker/Earthfile build argument generation
/// - Cross-compilation support
/// - Integration with CI/CD pipelines
///
/// # Usage
///
/// ```rust
/// use terraphim_build_args::{BuildConfig, BuildTarget, FeatureSet};
///
/// let config = BuildConfig::builder()
///     .target(BuildTarget::NativeRelease)
///     .features(FeatureSet::from_string("openrouter,typescript"))
///     .environment("production")
///     .build()?;
///
/// println!("Cargo args: {}", config.cargo_args());
/// ```
pub mod config;
pub mod environment;
pub mod error;
pub mod features;
pub mod generator;
pub mod targets;
pub mod validation;

pub use config::*;
pub use environment::*;
pub use error::*;
pub use features::*;
pub use generator::*;
pub use targets::*;
pub use validation::*;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Main build configuration structure that orchestrates all build arguments
/// and generates appropriate commands for different build systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildConfig {
    /// Build target specification (native, cross-compile, docker, etc.)
    pub target: BuildTarget,

    /// Feature flags to enable during build
    pub features: FeatureSet,

    /// Environment-specific configuration (dev, staging, production)
    pub environment: Environment,

    /// Custom build arguments and environment variables
    pub custom_args: IndexMap<String, String>,

    /// Docker/container specific build arguments
    pub docker_args: DockerBuildArgs,

    /// Workspace-specific settings
    pub workspace: WorkspaceConfig,

    /// Build metadata
    pub metadata: BuildMetadata,
}

/// Docker/container build arguments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DockerBuildArgs {
    /// Base image to use for build
    pub base_image: Option<String>,

    /// Build context path
    pub context: Option<String>,

    /// Dockerfile path
    pub dockerfile: Option<String>,

    /// Build-time environment variables
    pub build_args: IndexMap<String, String>,

    /// Target stage in multi-stage build
    pub target_stage: Option<String>,

    /// Platform specification for cross-platform builds
    pub platform: Option<String>,

    /// Cache configuration
    pub cache_from: Vec<String>,
    pub cache_to: Vec<String>,
}

/// Workspace configuration for multi-crate projects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WorkspaceConfig {
    /// Root workspace directory
    pub root: String,

    /// Specific workspace members to build
    pub members: Vec<String>,

    /// Workspace members to exclude from build
    pub exclude: Vec<String>,

    /// Default workspace member if none specified
    pub default_member: Option<String>,

    /// Whether to build all workspace members
    pub build_all: bool,
}

/// Build metadata for tracking and auditing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildMetadata {
    /// Unique build identifier
    pub build_id: String,

    /// Build timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Git commit hash (if available)
    pub commit_hash: Option<String>,

    /// Build initiator (CI, local, etc.)
    pub initiator: String,

    /// Additional build tags
    pub tags: Vec<String>,

    /// Build duration estimate
    pub estimated_duration: Option<std::time::Duration>,
}

impl Default for BuildMetadata {
    fn default() -> Self {
        Self {
            build_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            commit_hash: None,
            initiator: "local".to_string(),
            tags: vec![],
            estimated_duration: None,
        }
    }
}

impl BuildConfig {
    /// Creates a new builder for BuildConfig
    pub fn builder() -> BuildConfigBuilder {
        BuildConfigBuilder::new()
    }

    /// Loads build configuration from file
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::IoError(format!("Failed to read config file {}: {}", path, e)))?;

        if path.ends_with(".toml") {
            toml::from_str(&content)
                .map_err(|e| Error::ParseError(format!("TOML parse error: {}", e)))
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&content)
                .map_err(|e| Error::ParseError(format!("YAML parse error: {}", e)))
        } else if path.ends_with(".json") {
            serde_json::from_str(&content)
                .map_err(|e| Error::ParseError(format!("JSON parse error: {}", e)))
        } else {
            Err(Error::UnsupportedFormat(format!(
                "Unsupported config format: {}",
                path
            )))
        }
    }

    /// Saves build configuration to file
    pub fn to_file(&self, path: &str) -> Result<()> {
        let content = if path.ends_with(".toml") {
            toml::to_string_pretty(self).map_err(|e| {
                Error::SerializationError(format!("TOML serialization error: {}", e))
            })?
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::to_string(self).map_err(|e| {
                Error::SerializationError(format!("YAML serialization error: {}", e))
            })?
        } else if path.ends_with(".json") {
            serde_json::to_string_pretty(self).map_err(|e| {
                Error::SerializationError(format!("JSON serialization error: {}", e))
            })?
        } else {
            return Err(Error::UnsupportedFormat(format!(
                "Unsupported config format: {}",
                path
            )));
        };

        std::fs::write(path, content)
            .map_err(|e| Error::IoError(format!("Failed to write config file {}: {}", path, e)))?;

        Ok(())
    }

    /// Generates Cargo build command with all appropriate flags
    pub fn cargo_args(&self) -> Vec<String> {
        let mut args = vec![];

        // Add build command
        args.push("build".to_string());

        // Add target specification
        if let Some(target_triple) = self.target.target_triple() {
            args.push("--target".to_string());
            args.push(target_triple);
        }

        // Add build profile
        match self.target.profile() {
            BuildProfile::Release => args.push("--release".to_string()),
            BuildProfile::Debug => {} // Default is debug
            BuildProfile::Dev => args.push("--profile".to_string()),
        }

        // Add features
        if !self.features.is_empty() {
            if self.features.default_features {
                // Use default features plus additional ones
                if !self.features.enabled.is_empty() {
                    args.push("--features".to_string());
                    args.push(self.features.enabled.join(","));
                }
            } else {
                // Disable default features
                args.push("--no-default-features".to_string());
                if !self.features.enabled.is_empty() {
                    args.push("--features".to_string());
                    args.push(self.features.enabled.join(","));
                }
            }
        }

        // Add workspace settings
        if self.workspace.build_all {
            args.push("--workspace".to_string());
        } else if !self.workspace.members.is_empty() {
            for member in &self.workspace.members {
                args.push("--package".to_string());
                args.push(member.clone());
            }
        }

        // Add custom arguments
        for (key, value) in &self.custom_args {
            if value.is_empty() {
                args.push(format!("--{}", key));
            } else {
                args.push(format!("--{}", key));
                args.push(value.clone());
            }
        }

        args
    }

    /// Generates Docker build command with all appropriate flags
    pub fn docker_args(&self) -> Vec<String> {
        let mut args = vec!["build".to_string()];

        // Add build context
        if let Some(context) = &self.docker_args.context {
            args.push(context.clone());
        } else {
            args.push(".".to_string());
        }

        // Add dockerfile
        if let Some(dockerfile) = &self.docker_args.dockerfile {
            args.push("--file".to_string());
            args.push(dockerfile.clone());
        }

        // Add target stage
        if let Some(target) = &self.docker_args.target_stage {
            args.push("--target".to_string());
            args.push(target.clone());
        }

        // Add platform
        if let Some(platform) = &self.docker_args.platform {
            args.push("--platform".to_string());
            args.push(platform.clone());
        }

        // Add build arguments
        for (key, value) in &self.docker_args.build_args {
            args.push("--build-arg".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Add cache configuration
        for cache in &self.docker_args.cache_from {
            args.push("--cache-from".to_string());
            args.push(cache.clone());
        }

        for cache in &self.docker_args.cache_to {
            args.push("--cache-to".to_string());
            args.push(cache.clone());
        }

        args
    }

    /// Generates Earthfile build arguments
    pub fn earthly_args(&self) -> Vec<String> {
        let mut args = vec![];

        // Add target
        args.push(format!("+{}", self.target.earthly_target()));

        // Add build arguments
        for (key, value) in &self.docker_args.build_args {
            args.push(format!("--{}", key));
            args.push(value.clone());
        }

        // Add platform if specified
        if let Some(platform) = &self.docker_args.platform {
            args.push("--platform".to_string());
            args.push(platform.clone());
        }

        args
    }

    /// Validates the build configuration
    pub fn validate(&self) -> Result<()> {
        self.target.validate()?;
        self.features.validate()?;
        self.environment.validate()?;
        Ok(())
    }
}

/// Builder pattern for creating BuildConfig instances
#[derive(Debug, Default)]
pub struct BuildConfigBuilder {
    target: Option<BuildTarget>,
    features: Option<FeatureSet>,
    environment: Option<Environment>,
    custom_args: IndexMap<String, String>,
    docker_args: DockerBuildArgs,
    workspace: WorkspaceConfig,
    metadata: Option<BuildMetadata>,
}

impl BuildConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn target(mut self, target: BuildTarget) -> Self {
        self.target = Some(target);
        self
    }

    pub fn features(mut self, features: FeatureSet) -> Self {
        self.features = Some(features);
        self
    }

    pub fn environment(mut self, env: &str) -> Self {
        self.environment = Some(Environment::from_str(env).unwrap_or_default());
        self
    }

    pub fn custom_arg(mut self, key: &str, value: &str) -> Self {
        self.custom_args.insert(key.to_string(), value.to_string());
        self
    }

    pub fn docker_args(mut self, docker_args: DockerBuildArgs) -> Self {
        self.docker_args = docker_args;
        self
    }

    pub fn workspace(mut self, workspace: WorkspaceConfig) -> Self {
        self.workspace = workspace;
        self
    }

    pub fn metadata(mut self, metadata: BuildMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn build(self) -> Result<BuildConfig> {
        Ok(BuildConfig {
            target: self.target.unwrap_or_default(),
            features: self.features.unwrap_or_default(),
            environment: self.environment.unwrap_or_default(),
            custom_args: self.custom_args,
            docker_args: self.docker_args,
            workspace: self.workspace,
            metadata: self.metadata.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_build_config_creation() {
        let config = BuildConfig::builder()
            .target(BuildTarget::NativeRelease)
            .features(FeatureSet::from_slice(&["openrouter", "typescript"]))
            .environment("production")
            .build()
            .unwrap();

        assert_eq!(config.target, BuildTarget::NativeRelease);
        assert!(config.features.enabled.contains(&"openrouter".to_string()));
        assert_eq!(config.environment.name, "production");
    }

    #[test]
    fn test_cargo_args_generation() {
        let config = BuildConfig::builder()
            .target(BuildTarget::NativeRelease)
            .features(FeatureSet::from_slice(&["openrouter"]))
            .build()
            .unwrap();

        let args = config.cargo_args();
        assert!(args.contains(&"build".to_string()));
        assert!(args.contains(&"--release".to_string()));
        assert!(args.contains(&"--features".to_string()));
        assert!(args.contains(&"openrouter".to_string()));
    }

    #[test]
    fn test_config_serialization() {
        let config = BuildConfig::builder()
            .target(BuildTarget::NativeDebug)
            .features(FeatureSet::from_slice(&["typescript"]))
            .build()
            .unwrap();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: BuildConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_file_operations() -> Result<()> {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_config.toml");

        let config = BuildConfig::builder()
            .target(BuildTarget::NativeRelease)
            .features(FeatureSet::from_slice(&["openrouter", "typescript"]))
            .build()?;

        // Save to file
        config.to_file(file_path.to_str().unwrap())?;

        // Load from file
        let loaded_config = BuildConfig::from_file(file_path.to_str().unwrap())?;

        assert_eq!(config.target, loaded_config.target);
        assert_eq!(config.features.enabled, loaded_config.features.enabled);

        Ok(())
    }
}
