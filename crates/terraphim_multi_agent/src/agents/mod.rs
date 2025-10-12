//! Specialized Agent Implementations
//!
//! This module contains specialized agents that build on top of the core TerraphimAgent
//! to provide specific functionality like summarization, chat, and other domain-specific tasks.

pub mod chat_agent;
pub mod summarization_agent;

pub use chat_agent::*;
pub use summarization_agent::*;

// Re-export commonly used types
pub use crate::{GenAiLlmClient, MultiAgentError, MultiAgentResult, TerraphimAgent};
