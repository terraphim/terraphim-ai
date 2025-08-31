/// Target management for build configurations
///
/// This module provides functionality for managing build targets in the
/// Terraphim AI project, enabling configuration for cross-compilation,
/// platform-specific settings, and target validation.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Different build targets that can be specified
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum BuildTarget {
    /// Native target for release builds
    NativeRelease,
    /// Native target for debug builds
    #[default]
    NativeDebug,
    /// Cross-compilation target
    CrossCompile(String),
    /// Docker build target
    Docker,
    /// Earthly build target
    Earthly(String),
    /// Custom target with specific triple
    Custom(String),
}

/// Profiles for build targets
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuildProfile {
    /// Debug build profile
    Debug,
    /// Release build profile
    Release,
    /// Dev build profile
    Dev,
}

impl BuildTarget {
    /// Retrieves the target triple string
    pub fn target_triple(&self) -> Option<String> {
        match self {
            BuildTarget::NativeRelease => None, // Default triple
            BuildTarget::NativeDebug => None,   // Default triple
            BuildTarget::CrossCompile(triple) => Some(triple.clone()),
            BuildTarget::Docker => None, // Docker doesn't use triples
            BuildTarget::Earthly(target) => Some(target.clone()),
            BuildTarget::Custom(triple) => Some(triple.clone()),
        }
    }

    /// Retrieves the Earthly target string
    pub fn earthly_target(&self) -> String {
        match self {
            BuildTarget::Earthly(target) => target.clone(),
            _ => "build".to_string(), // Default Earthly target
        }
    }

    /// Retrieves the build profile
    pub fn profile(&self) -> BuildProfile {
        match self {
            BuildTarget::NativeRelease
            | BuildTarget::CrossCompile(_)
            | BuildTarget::Docker
            | BuildTarget::Earthly(_)
            | BuildTarget::Custom(_) => BuildProfile::Release,
            BuildTarget::NativeDebug => BuildProfile::Debug,
        }
    }

    /// Validates the target
    pub fn validate(&self) -> Result<()> {
        match self {
            BuildTarget::CrossCompile(triple) | BuildTarget::Custom(triple) => {
                if !is_valid_target_triple(triple) {
                    return Err(Error::target(format!("Invalid target triple '{}': must follow '<arch>-<vendor>-<sys>-<abi>' format", triple)));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl FromStr for BuildTarget {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "native-release" | "release" => Ok(BuildTarget::NativeRelease),
            "native-debug" | "debug" => Ok(BuildTarget::NativeDebug),
            "docker" => Ok(BuildTarget::Docker),
            other => {
                if let Some(stripped) = other.strip_prefix("cross-") {
                    Ok(BuildTarget::CrossCompile(stripped.to_string()))
                } else if let Some(stripped) = other.strip_prefix("earthly-") {
                    Ok(BuildTarget::Earthly(stripped.to_string()))
                } else {
                    Ok(BuildTarget::Custom(other.to_string()))
                }
            }
        }
    }
}

impl std::fmt::Display for BuildTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildTarget::NativeRelease => write!(f, "native-release"),
            BuildTarget::NativeDebug => write!(f, "native-debug"),
            BuildTarget::CrossCompile(triple) => write!(f, "cross-{}", triple),
            BuildTarget::Docker => write!(f, "docker"),
            BuildTarget::Earthly(target) => write!(f, "earthly-{}", target),
            BuildTarget::Custom(triple) => write!(f, "custom-{}", triple),
        }
    }
}

/// Validates that a target triple follows the '<arch>-<vendor>-<sys>-<abi>' format
/// Note: Some target triples have 3 parts (without ABI), which is also valid
fn is_valid_target_triple(triple: &str) -> bool {
    let parts: Vec<&str> = triple.split('-').collect();
    (parts.len() == 3 || parts.len() == 4) && parts.iter().all(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_triple_validation() {
        assert!(is_valid_target_triple("x86_64-unknown-linux-gnu"));
        assert!(is_valid_target_triple("aarch64-apple-darwin"));
        assert!(!is_valid_target_triple("x86_64"));
        assert!(!is_valid_target_triple(""));
    }

    #[test]
    fn test_build_target_parsing() {
        assert_eq!(
            BuildTarget::from_str("native-release").unwrap(),
            BuildTarget::NativeRelease
        );
        assert_eq!(
            BuildTarget::from_str("release").unwrap(),
            BuildTarget::NativeRelease
        );
        assert_eq!(
            BuildTarget::from_str("native-debug").unwrap(),
            BuildTarget::NativeDebug
        );
        assert_eq!(
            BuildTarget::from_str("debug").unwrap(),
            BuildTarget::NativeDebug
        );
        assert_eq!(
            BuildTarget::from_str("docker").unwrap(),
            BuildTarget::Docker
        );
        assert_eq!(
            BuildTarget::from_str("cross-x86_64-unknown-linux-musl").unwrap(),
            BuildTarget::CrossCompile("x86_64-unknown-linux-musl".to_string())
        );
        assert_eq!(
            BuildTarget::from_str("earthly-build").unwrap(),
            BuildTarget::Earthly("build".to_string())
        );
    }

    #[test]
    fn test_build_target_validation() {
        let valid_target = BuildTarget::CrossCompile("x86_64-unknown-linux-musl".to_string());
        assert!(valid_target.validate().is_ok());

        let invalid_target = BuildTarget::CrossCompile("invalid-triple".to_string());
        assert!(invalid_target.validate().is_err());
    }

    #[test]
    fn test_build_target_display() {
        let target = BuildTarget::CrossCompile("x86_64-unknown-linux-gnu".to_string());
        assert_eq!(target.to_string(), "cross-x86_64-unknown-linux-gnu");

        let target = BuildTarget::NativeDebug;
        assert_eq!(target.to_string(), "native-debug");
    }
}
