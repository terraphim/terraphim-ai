//! Terraphim Judge Evaluator
//!
//! Multi-agent code quality assessment using Knowledge Graph and tiered LLM routing.
//!
//! This crate provides:
//! - **SimpleAgent**: Knowledge Graph lookup for context enrichment
//! - **JudgeModelRouter**: Tier-based LLM model selection (quick/deep/tiebreaker/oracle)
//! - **JudgeAgent**: Supervised agent implementing the full evaluation pipeline

pub mod judge_agent;
pub mod model_router;
pub mod simple_agent;

pub use simple_agent::{KgMatch, SimpleAgent};

use thiserror::Error;

/// Errors specific to the judge evaluator
#[derive(Error, Debug)]
pub enum JudgeError {
    #[error("Failed to load model mapping configuration: {0}")]
    ConfigLoadError(String),

    #[error("Unknown judge tier: {0}")]
    UnknownTier(String),

    #[error("Unknown profile: {0}")]
    UnknownProfile(String),

    #[error("Knowledge Graph lookup failed: {0}")]
    KgLookupError(String),

    #[error("Model dispatch failed: {0}")]
    DispatchError(String),

    #[error("Failed to parse verdict: {0}")]
    VerdictParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for judge operations
pub type JudgeResult<T> = Result<T, JudgeError>;
