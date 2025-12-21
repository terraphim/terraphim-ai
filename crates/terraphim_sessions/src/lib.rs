//! Terraphim Sessions - AI Coding Assistant History Management
//!
//! This crate provides session management for AI coding assistant history,
//! supporting multiple sources including Claude Code and Cursor IDE.
//!
//! ## Features
//!
//! - `claude-log-analyzer`: Enable CLA integration for enhanced Claude Code parsing
//! - `cla-full`: CLA with Cursor connector support
//! - `enrichment`: Enable knowledge graph enrichment via terraphim
//! - `full`: All features enabled
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use terraphim_sessions::{SessionService, ConnectorRegistry};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let service = SessionService::new();
//!     let sessions = service.list_sessions().await?;
//!
//!     for session in sessions {
//!         println!("{}: {} messages", session.id, session.messages.len());
//!     }
//!     Ok(())
//! }
//! ```

pub mod connector;
pub mod model;
pub mod service;

#[cfg(feature = "claude-log-analyzer")]
pub mod cla;

#[cfg(feature = "enrichment")]
pub mod enrichment;

// Re-exports for convenience
pub use connector::{ConnectorRegistry, ConnectorStatus, ImportOptions, SessionConnector};
pub use model::{ContentBlock, Message, MessageRole, Session, SessionMetadata};
pub use service::SessionService;

#[cfg(feature = "enrichment")]
pub use enrichment::{
    ConceptMatch, ConceptOccurrence, EnrichmentConfig, EnrichmentResult, SessionConcepts,
    SessionEnricher, find_related_sessions, search_by_concept,
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
