//! # Terraphim RLM
//!
//! Recursive Language Model (RLM) orchestration for Terraphim AI.
//!
//! This crate provides:
//! - Isolated code execution in Firecracker VMs with sub-500ms allocation
//! - Recursive LLM invocation from within VMs via HTTP bridge
//! - Knowledge graph validation of commands with configurable strictness
//! - Session management with VM affinity, snapshots, and extensions
//! - Dual budget system (tokens + time) for runaway prevention
//!
//! ## Architecture
//!
//! ```text
//! TerraphimRlm (public API)
//!     ├── SessionManager (VM affinity, context, snapshots, extensions)
//!     ├── QueryLoop (command parsing, execution, result handling)
//!     ├── BudgetTracker (token counting, time tracking, depth limits)
//!     └── KnowledgeGraphValidator (term matching, retry, strictness)
//!
//! ExecutionEnvironment trait
//!     ├── FirecrackerExecutor (primary, full isolation)
//!     ├── DockerExecutor (fallback, gVisor/runc)
//!     └── E2bExecutor (cloud option)
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use terraphim_rlm::{TerraphimRlm, RlmConfig};
//!
//! let config = RlmConfig::default();
//! let rlm = TerraphimRlm::new(config).await?;
//!
//! // Create a session
//! let session = rlm.create_session().await?;
//!
//! // Execute Python code
//! let result = rlm.execute_code(&session, "print('Hello, RLM!')").await?;
//! println!("Output: {}", result.stdout);
//!
//! // Execute bash command
//! let result = rlm.execute_command(&session, "ls -la").await?;
//! println!("Output: {}", result.stdout);
//! ```

// Core modules
pub mod config;
pub mod error;
pub mod types;

// Execution environment abstraction
pub mod executor;

// Session and budget management (Phase 2)
pub mod budget;
pub mod llm_bridge;
pub mod session;

// RLM orchestration (to be implemented in later phases)
// pub mod rlm;
// pub mod query_loop;
// pub mod validator;
// pub mod command;
// pub mod preamble;
// pub mod logger;
// pub mod autoscaler;
// pub mod dns_security;
// pub mod operations;
// pub mod mcp_errors;

// Re-exports for convenient access
pub use budget::BudgetTracker;
pub use config::{BackendType, KgStrictness, RlmConfig, SessionModel};
pub use error::RlmError;
pub use executor::{
    Capability, ExecutionContext, ExecutionEnvironment, ExecutionResult, SnapshotId,
    ValidationResult,
};
pub use llm_bridge::{LlmBridge, LlmBridgeConfig, QueryRequest, QueryResponse};
pub use session::{SessionManager, SessionStats};
pub use types::{
    BudgetStatus, Command, CommandHistory, QueryMetadata, SessionId, SessionInfo, SessionState,
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default token budget (100K tokens)
pub const DEFAULT_TOKEN_BUDGET: u64 = 100_000;

/// Default time budget (5 minutes in milliseconds)
pub const DEFAULT_TIME_BUDGET_MS: u64 = 300_000;

/// Default max recursion depth
pub const DEFAULT_MAX_RECURSION_DEPTH: u32 = 10;

/// Default max snapshots per session
pub const DEFAULT_MAX_SNAPSHOTS_PER_SESSION: u32 = 10;

/// Default max inline output bytes before streaming to file (64KB)
pub const DEFAULT_MAX_INLINE_OUTPUT_BYTES: u64 = 65_536;

/// Default VM allocation timeout (500ms)
pub const VM_ALLOCATION_TIMEOUT_MS: u64 = 500;

/// Target boot time for VMs (2 seconds)
pub const TARGET_BOOT_TIME_MS: u64 = 2000;

/// Default DNS allowlist
pub const DEFAULT_DNS_ALLOWLIST: &[&str] = &["pypi.org", "github.com", "raw.githubusercontent.com"];
