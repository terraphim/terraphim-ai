//! Session Connectors for multi-source session management
//!
//! This module provides a unified interface for importing sessions
//! from various AI coding assistants.

mod native;

pub use native::NativeClaudeConnector;

use crate::model::Session;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Status of a connector's detection
#[derive(Debug, Clone)]
pub enum ConnectorStatus {
    /// Connector found with estimated session count
    Available {
        path: PathBuf,
        sessions_estimate: Option<usize>,
    },
    /// Connector's data directory not found
    NotFound,
    /// Connector found but has errors
    Error(String),
}

impl ConnectorStatus {
    /// Check if connector is available
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }
}

/// Options for importing sessions
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Override the default path
    pub path: Option<PathBuf>,
    /// Only import sessions after this timestamp
    pub since: Option<jiff::Timestamp>,
    /// Only import sessions before this timestamp
    pub until: Option<jiff::Timestamp>,
    /// Maximum sessions to import
    pub limit: Option<usize>,
    /// Skip sessions already imported (for incremental updates)
    pub incremental: bool,
}

impl ImportOptions {
    /// Create new import options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set path override
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Enable incremental mode
    pub fn incremental(mut self) -> Self {
        self.incremental = true;
        self
    }
}

/// Trait for session connectors
#[async_trait]
pub trait SessionConnector: Send + Sync {
    /// Unique identifier for this connector
    fn source_id(&self) -> &str;

    /// Human-readable name
    fn display_name(&self) -> &str;

    /// Check if this connector's data source is available
    fn detect(&self) -> ConnectorStatus;

    /// Get the default data path for this connector
    fn default_path(&self) -> Option<PathBuf>;

    /// Import sessions from this source
    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>>;
}

/// Registry of available connectors
pub struct ConnectorRegistry {
    connectors: Vec<Box<dyn SessionConnector>>,
}

impl ConnectorRegistry {
    /// Create a new registry with all available connectors
    #[must_use]
    #[allow(clippy::vec_init_then_push)] // Feature-gated conditional pushes prevent using vec![]
    pub fn new() -> Self {
        let mut connectors: Vec<Box<dyn SessionConnector>> = Vec::new();

        // Add native Claude Code connector (always available)
        connectors.push(Box::new(NativeClaudeConnector));

        // Add CLA-based connectors if feature enabled
        #[cfg(feature = "claude-log-analyzer")]
        {
            connectors.push(Box::new(crate::cla::ClaClaudeConnector::default()));

            #[cfg(feature = "cla-full")]
            connectors.push(Box::new(crate::cla::ClaCursorConnector::default()));
        }

        Self { connectors }
    }

    /// Get all registered connectors
    #[must_use]
    pub fn connectors(&self) -> &[Box<dyn SessionConnector>] {
        &self.connectors
    }

    /// Find a connector by source ID
    #[must_use]
    pub fn get(&self, source_id: &str) -> Option<&dyn SessionConnector> {
        self.connectors
            .iter()
            .find(|c| c.source_id() == source_id)
            .map(|c| c.as_ref())
    }

    /// Detect all available connectors
    pub fn detect_all(&self) -> Vec<(&str, ConnectorStatus)> {
        self.connectors
            .iter()
            .map(|c| (c.source_id(), c.detect()))
            .collect()
    }

    /// Get all available (detected) connectors
    pub fn available(&self) -> Vec<&dyn SessionConnector> {
        self.connectors
            .iter()
            .filter(|c| c.detect().is_available())
            .map(|c| c.as_ref())
            .collect()
    }

    /// Import sessions from all available connectors
    pub async fn import_all(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let mut all_sessions = Vec::new();

        for connector in self.available() {
            match connector.import(options).await {
                Ok(mut sessions) => {
                    all_sessions.append(&mut sessions);
                }
                Err(e) => {
                    tracing::warn!("Failed to import from {}: {}", connector.display_name(), e);
                }
            }

            // Apply global limit if specified
            if let Some(limit) = options.limit {
                if all_sessions.len() >= limit {
                    all_sessions.truncate(limit);
                    break;
                }
            }
        }

        Ok(all_sessions)
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_registry_creation() {
        let registry = ConnectorRegistry::new();
        assert!(!registry.connectors().is_empty());
    }

    #[test]
    fn test_import_options_builder() {
        let options = ImportOptions::new()
            .with_path(PathBuf::from("/test"))
            .with_limit(10)
            .incremental();

        assert_eq!(options.path, Some(PathBuf::from("/test")));
        assert_eq!(options.limit, Some(10));
        assert!(options.incremental);
    }
}
