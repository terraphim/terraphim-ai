//! VM execution subsystem for sandboxed code execution via fcctl.

/// HTTP client for the fcctl-web VM execution API.
pub mod client;
/// Utilities for extracting code blocks from LLM text responses.
pub mod code_extractor;
/// Helpers for building `VmExecutionConfig` from role configuration.
pub mod config_helpers;
/// Bridge for tracking execution history and triggering rollbacks.
pub mod fcctl_bridge;
/// Pre- and post-execution hook framework for validation and logging.
pub mod hooks;
/// Shared request/response and error types for VM execution.
pub mod models;
/// Long-lived session management adapter for the fcctl session API.
pub mod session_adapter;

pub use client::*;
pub use code_extractor::*;
pub use config_helpers::*;
pub use fcctl_bridge::*;
pub use hooks::*;
pub use models::*;
pub use session_adapter::*;
