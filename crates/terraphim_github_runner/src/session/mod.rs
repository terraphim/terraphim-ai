//! Session management for VM-based workflow execution
//!
//! This module provides:
//! - VM allocation and lifecycle management (manager.rs)
//! - Session tracking per workflow execution

pub mod fcctl_provider;
pub mod manager;

pub use fcctl_provider::FcctlWebProvider;
pub use manager::{
    HostVmProvider, MockVmProvider, Session, SessionManager, SessionManagerConfig,
    SessionStartSpec, SessionState, SessionStats, VmProvider,
};
