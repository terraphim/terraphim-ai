//! Agent runner for Symphony.
//!
//! Wraps the Codex app-server session lifecycle and provides an
//! orchestrator-facing API with event channels.

pub mod protocol;
pub mod session;

pub use protocol::{AgentEvent, TokenCounts, TokenTotals};
pub use session::{CodexSession, WorkerOutcome};
