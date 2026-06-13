//! Testing framework for validation system
//!
//! This module provides testing utilities and fixtures for validation system.

pub mod desktop_ui;
pub mod fixtures;
pub mod server_api;
pub mod tui;
pub mod utils;

#[allow(ambiguous_glob_reexports)]
pub use desktop_ui::*;
pub use fixtures::*;
#[allow(ambiguous_glob_reexports)]
pub use server_api::*;
#[allow(ambiguous_glob_reexports)]
pub use tui::*;
pub use utils::*;

// Re-export anyhow::Result for testing modules
pub use anyhow::Result;

// Re-export validation types for testing modules
pub use crate::validators::{Severity, ValidationIssue, ValidationResult, ValidationStatus};
