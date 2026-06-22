//! Execution environment abstraction for RLM.
//!
//! This module defines the `ExecutionEnvironment` trait and related types that
//! provide a unified interface for different execution backends (Firecracker, Docker, E2B, Local).
//!
//! ## Architecture
//!
//! ```text
//! ExecutionEnvironment trait
//!     ├── FirecrackerExecutor (full VM isolation, requires KVM)
//!     ├── DockerExecutor (container isolation, gVisor/runc)
//!     ├── E2bExecutor (cloud-hosted Firecracker)
//!     └── LocalExecutor (local process execution, no isolation)
//! ```
//!
//! ## Backend Selection
//!
//! Backends are selected based on:
//! 1. User preference order in `RlmConfig::backend_preference`
//! 2. Availability (KVM for Firecracker, API key for E2B, Docker daemon)
//! 3. Fallback to next available backend if preferred is unavailable

mod context;
#[cfg(feature = "docker-backend")]
mod docker;
#[cfg(feature = "firecracker")]
mod firecracker;
mod local;
mod ssh;
mod r#trait;

pub use context::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
#[cfg(feature = "docker-backend")]
pub use docker::DockerExecutor;
#[cfg(feature = "firecracker")]
pub use firecracker::FirecrackerExecutor;
pub use local::LocalExecutor;
pub use ssh::SshExecutor;
pub use r#trait::ExecutionEnvironment;

use crate::config::{BackendType, RlmConfig};
use crate::error::RlmError;
use crate::validator::{KnowledgeGraphValidator, ValidatorConfig};
use std::sync::Arc;

/// Build a `KnowledgeGraphValidator` from config for injection into executors.
fn build_validator_for_executor(config: &RlmConfig) -> Option<Arc<KnowledgeGraphValidator>> {
    if config.thesaurus.is_none() && config.kg_strictness == crate::config::KgStrictness::Permissive
    {
        return None;
    }
    let mut vcfg = match config.kg_strictness {
        crate::config::KgStrictness::Permissive => ValidatorConfig::permissive(),
        crate::config::KgStrictness::Normal => ValidatorConfig::default(),
        crate::config::KgStrictness::Strict => ValidatorConfig::strict(),
    };
    vcfg.max_retries = config.kg_max_retries;
    let mut validator = KnowledgeGraphValidator::new(vcfg);
    if let Some(ref thesaurus) = config.thesaurus {
        validator = validator.with_thesaurus(thesaurus.clone());
    }
    Some(Arc::new(validator))
}

/// Check if KVM is available on this system.
pub fn is_kvm_available() -> bool {
    std::path::Path::new("/dev/kvm").exists()
}

/// Check if Docker is available.
pub fn is_docker_available() -> bool {
    // Simple check - could be enhanced to actually ping Docker daemon
    std::process::Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if gVisor (runsc) is available.
pub fn is_gvisor_available() -> bool {
    std::process::Command::new("runsc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Select and create an appropriate executor based on configuration.
///
/// Tries backends in preference order, falling back to next available.
///
/// # Example
///
/// ```rust,no_run
/// use terraphim_rlm::{RlmConfig, executor::select_executor};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = RlmConfig::default();
///     let executor = select_executor(&config).await?;
///     Ok(())
/// }
/// ```
pub async fn select_executor(
    config: &RlmConfig,
) -> Result<Box<dyn ExecutionEnvironment<Error = RlmError> + Send + Sync>, RlmError> {
    let backends = if config.backend_preference.is_empty() {
        vec![
            BackendType::Firecracker,
            BackendType::E2b,
            BackendType::Docker,
            BackendType::Local,
        ]
    } else {
        config.backend_preference.clone()
    };

    let validator = build_validator_for_executor(config);

    // Cache the docker availability probe across loop iterations to avoid
    // repeating the (~50-100 ms) shell-out to `docker --version`.
    #[cfg(feature = "docker-backend")]
    let docker_available = is_docker_available();
    let mut tried = Vec::new();

    for backend in backends {
        match backend {
            #[cfg(feature = "firecracker")]
            BackendType::Firecracker if is_kvm_available() => {
                log::info!("Selected Firecracker backend (KVM available)");
                let executor = FirecrackerExecutor::new(config.clone(), validator.clone())?;
                if let Err(e) = executor.initialize().await {
                    log::warn!(
                        "Failed to initialize FirecrackerExecutor: {}. Trying next backend.",
                        e
                    );
                    tried.push(format!("firecracker (init failed: {})", e));
                    continue;
                }
                return Ok(Box::new(executor));
            }
            #[cfg(feature = "firecracker")]
            BackendType::Firecracker => {
                log::debug!("Firecracker unavailable: KVM not present");
                tried.push("firecracker (no KVM)".to_string());
            }
            #[cfg(not(feature = "firecracker"))]
            BackendType::Firecracker => {
                log::debug!("Firecracker backend disabled at compile time");
                tried.push("firecracker (compile-time disabled)".to_string());
            }

            BackendType::E2b if config.e2b_api_key.is_some() => {
                // E2B backend is declared in BackendType but not yet wired up.
                // Previously this arm logged "Selected E2B backend" then fell
                // through, misleading operators. Now we explicitly skip and
                // try the next backend.
                log::debug!("E2B backend not yet implemented; trying next backend");
                tried.push("e2b (not implemented)".to_string());
            }
            BackendType::E2b => {
                log::debug!("E2B unavailable: no API key configured");
                tried.push("e2b (no API key)".to_string());
            }

            #[cfg(feature = "docker-backend")]
            BackendType::Docker if docker_available => {
                match DockerExecutor::new(config.clone(), validator.clone()) {
                    Ok(executor) => {
                        log::info!("Selected Docker backend (container isolation)");
                        return Ok(Box::new(executor));
                    }
                    Err(e) => {
                        log::warn!("DockerExecutor init failed: {}. Trying next backend.", e);
                        tried.push(format!("docker (init failed: {})", e));
                    }
                }
            }
            #[cfg(feature = "docker-backend")]
            BackendType::Docker => {
                log::debug!("Docker unavailable: CLI not present");
                tried.push("docker (not available)".to_string());
            }
            #[cfg(not(feature = "docker-backend"))]
            BackendType::Docker => {
                log::debug!("Docker backend disabled at compile time");
                tried.push("docker (compile-time disabled)".to_string());
            }

            BackendType::Local => {
                log::warn!(
                    "Falling back to LocalExecutor (NO ISOLATION). Tried: {:?}",
                    tried
                );
                return Ok(Box::new(
                    LocalExecutor::new()
                        .with_validator(validator)
                        .with_kg_strictness(config.kg_strictness),
                ));
            }
        }
    }

    Err(RlmError::NoBackendAvailable { tried })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kvm_check() {
        // This test just verifies the function doesn't panic
        let _ = is_kvm_available();
    }

    #[test]
    fn test_docker_check() {
        // This test just verifies the function doesn't panic
        let _ = is_docker_available();
    }

    #[test]
    fn test_gvisor_check() {
        // This test just verifies the function doesn't panic
        let _ = is_gvisor_available();
    }

    #[tokio::test]
    async fn test_select_executor_local_preference_returns_local() {
        // backend_preference=[Local] forces selection of LocalExecutor
        // regardless of which other backends are available, exercising the
        // warn-log path on the Local arm.
        let config = RlmConfig {
            backend_preference: vec![BackendType::Local],
            ..Default::default()
        };

        let executor = select_executor(&config).await.expect("should select Local");
        assert_eq!(executor.backend_type(), BackendType::Local);
    }

    #[tokio::test]
    async fn test_select_executor_e2b_unimplemented_falls_through_to_local() {
        // With an E2B api key set but no Firecracker/Docker available,
        // selector should not stall on the E2B arm and should reach Local.
        let config = RlmConfig {
            backend_preference: vec![BackendType::E2b, BackendType::Local],
            e2b_api_key: Some("dummy".to_string()),
            ..Default::default()
        };

        let executor = select_executor(&config)
            .await
            .expect("should fall through to Local");
        assert_eq!(executor.backend_type(), BackendType::Local);
    }
}
