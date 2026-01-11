//! Execution environment abstraction for RLM.
//!
//! This module defines the `ExecutionEnvironment` trait and related types that
//! provide a unified interface for different execution backends (Firecracker, Docker, E2B).
//!
//! ## Architecture
//!
//! ```text
//! ExecutionEnvironment trait
//!     ├── FirecrackerExecutor (full VM isolation, requires KVM)
//!     ├── DockerExecutor (container isolation, gVisor/runc)
//!     └── E2bExecutor (cloud-hosted Firecracker)
//! ```
//!
//! ## Backend Selection
//!
//! Backends are selected based on:
//! 1. User preference order in `RlmConfig::backend_preference`
//! 2. Availability (KVM for Firecracker, API key for E2B, Docker daemon)
//! 3. Fallback to next available backend if preferred is unavailable

mod context;
mod firecracker;
mod ssh;
mod r#trait;

pub use context::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
pub use firecracker::FirecrackerExecutor;
pub use ssh::SshExecutor;
pub use r#trait::ExecutionEnvironment;

use crate::config::{BackendType, RlmConfig};
use crate::error::RlmError;

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
/// ```rust,ignore
/// use terraphim_rlm::{RlmConfig, select_executor};
///
/// let config = RlmConfig::default();
/// let executor = select_executor(&config).await?;
/// ```
pub async fn select_executor(
    config: &RlmConfig,
) -> Result<Box<dyn ExecutionEnvironment<Error = RlmError> + Send + Sync>, RlmError> {
    let backends = if config.backend_preference.is_empty() {
        vec![
            BackendType::Firecracker,
            BackendType::E2b,
            BackendType::Docker,
        ]
    } else {
        config.backend_preference.clone()
    };

    let mut tried = Vec::new();

    for backend in backends {
        match backend {
            BackendType::Firecracker if is_kvm_available() => {
                log::info!("Selected Firecracker backend (KVM available)");
                return Ok(Box::new(FirecrackerExecutor::new(config.clone())?));
            }
            BackendType::Firecracker => {
                log::debug!("Firecracker unavailable: KVM not present");
                tried.push("firecracker (no KVM)".to_string());
            }

            BackendType::E2b if config.e2b_api_key.is_some() => {
                log::info!("Selected E2B backend");
                // E2B executor will be implemented in later phase
                tried.push("e2b (not yet implemented)".to_string());
            }
            BackendType::E2b => {
                log::debug!("E2B unavailable: no API key configured");
                tried.push("e2b (no API key)".to_string());
            }

            BackendType::Docker if is_docker_available() => {
                log::info!(
                    "Selected Docker backend (gVisor: {})",
                    is_gvisor_available()
                );
                // Docker executor will be implemented in later phase
                tried.push("docker (not yet implemented)".to_string());
            }
            BackendType::Docker => {
                log::debug!("Docker unavailable: daemon not running");
                tried.push("docker (not available)".to_string());
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
}
