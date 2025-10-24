//! # Terraphim Agent Application
//!
//! OTP-style application behavior for the Terraphim agent system, providing
//! system-wide lifecycle management, configuration, and hot code reloading.
//!
//! This crate implements the application layer that coordinates all agent system
//! components following Erlang/OTP application behavior patterns.
//!
//! ## Core Features
//!
//! - **Application Lifecycle**: Startup, shutdown, and restart management
//! - **Supervision Tree Management**: Top-level supervision of all system components
//! - **Hot Code Reloading**: Dynamic agent behavior updates without system restart
//! - **Configuration Management**: System-wide configuration with hot reloading
//! - **Health Monitoring**: System diagnostics and health checks
//! - **Deployment Strategies**: Agent deployment and scaling management

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod application;
pub mod config;
pub mod deployment;
pub mod diagnostics;
pub mod error;
pub mod hot_reload;
pub mod lifecycle;

pub use application::*;
pub use config::*;
pub use deployment::*;
pub use diagnostics::*;
pub use error::*;
pub use hot_reload::*;
pub use lifecycle::*;

// Re-export key types
pub use terraphim_agent_supervisor::{AgentPid, SupervisorId};
pub use terraphim_kg_orchestration::SupervisionTreeOrchestrator;

/// Result type for application operations
pub type ApplicationResult<T> = Result<T, ApplicationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Test that all modules compile and basic types are available
        let _agent_id = AgentPid::new();
        let _supervisor_id = SupervisorId::new();
    }
}
