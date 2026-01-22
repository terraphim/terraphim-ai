//! Shared session connector interfaces and normalized session model.
//!
//! This crate provides:
//! - `SessionConnector` trait (sync import API)
//! - normalized session/message data model
//! - reusable connectors for specific sources (feature-gated)

use anyhow::Result;
use std::path::PathBuf;

#[cfg(feature = "aider")]
pub mod aider;

#[cfg(feature = "opencode")]
pub mod opencode;

/// Status of a connector's detection.
#[derive(Debug, Clone)]
pub enum ConnectorStatus {
    /// Connector found with estimated session count.
    Available {
        path: PathBuf,
        sessions_estimate: Option<usize>,
    },
    /// Connector's data directory not found.
    NotFound,
    /// Connector found but has errors.
    Error(String),
}

impl ConnectorStatus {
    /// Check if connector is available.
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }
}

/// Options for importing sessions.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Override the default path.
    pub path: Option<PathBuf>,
    /// Only import sessions after this timestamp.
    pub since: Option<jiff::Timestamp>,
    /// Only import sessions before this timestamp.
    pub until: Option<jiff::Timestamp>,
    /// Maximum sessions to import.
    pub limit: Option<usize>,
    /// Skip sessions already imported (for incremental updates).
    pub incremental: bool,
}

/// Normalized session from any connector.
#[derive(Debug, Clone)]
pub struct NormalizedSession {
    /// Connector source ID.
    pub source: String,
    /// Original session ID from the source.
    pub external_id: String,
    /// Session title or description.
    pub title: Option<String>,
    /// Path to source file/database.
    pub source_path: PathBuf,
    /// Session start time.
    pub started_at: Option<jiff::Timestamp>,
    /// Session end time.
    pub ended_at: Option<jiff::Timestamp>,
    /// Normalized messages.
    pub messages: Vec<NormalizedMessage>,
    /// Additional metadata.
    pub metadata: serde_json::Value,
}

/// Normalized message from any connector.
#[derive(Debug, Clone)]
pub struct NormalizedMessage {
    /// Message index in session.
    pub idx: usize,
    /// Role: user, assistant, or system.
    pub role: String,
    /// Author identifier (model name, user, etc.).
    pub author: Option<String>,
    /// Message content.
    pub content: String,
    /// Message timestamp.
    pub created_at: Option<jiff::Timestamp>,
    /// Additional fields.
    pub extra: serde_json::Value,
}

/// Trait for session connectors.
pub trait SessionConnector: Send + Sync {
    /// Unique identifier for this connector.
    fn source_id(&self) -> &str;

    /// Human-readable name.
    fn display_name(&self) -> &str;

    /// Check if this connector's data source is available.
    fn detect(&self) -> ConnectorStatus;

    /// Get the default data path for this connector.
    fn default_path(&self) -> Option<PathBuf>;

    /// Import sessions from this source.
    fn import(&self, options: &ImportOptions) -> Result<Vec<NormalizedSession>>;
}
