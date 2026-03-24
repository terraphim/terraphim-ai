//! Agent runner for Symphony.
//!
//! Wraps the Codex app-server session lifecycle and provides an
//! orchestrator-facing API with event channels.

pub mod claude_code;
pub mod protocol;
pub mod session;

pub use claude_code::ClaudeCodeSession;
pub use protocol::{
    AdfEnvelope, AgentEvent, FindingCategory, FindingSeverity, ReviewAgentOutput, ReviewFinding,
    TokenCounts, TokenTotals, deduplicate_findings,
};
pub use session::{CodexSession, WorkerOutcome};
