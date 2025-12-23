//! Session management for VM-based workflow execution
//!
//! This module provides:
//! - VM allocation and lifecycle management (manager.rs)
//! - Session tracking per workflow execution

pub mod manager;

pub use manager::{
    MockVmProvider, Session, SessionManager, SessionManagerConfig, SessionState, SessionStats,
    VmProvider,
};
