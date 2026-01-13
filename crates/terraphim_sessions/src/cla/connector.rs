//! TSA-based session connectors
//!
//! These connectors wrap terraphim-session-analyzer's connectors
//! to provide enhanced parsing capabilities.

use super::from_normalized_session;
use crate::connector::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::Session;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use terraphim_session_analyzer::connectors::{
    ImportOptions as ClaImportOptions, SessionConnector as ClaSessionConnector,
};

/// TSA-powered Claude Code connector
///
/// Uses terraphim-session-analyzer for enhanced parsing with agent attribution,
/// tool tracking, and detailed analytics.
#[derive(Debug, Default)]
pub struct ClaClaudeConnector {
    inner: terraphim_session_analyzer::connectors::ClaudeCodeConnector,
}

#[async_trait]
impl SessionConnector for ClaClaudeConnector {
    fn source_id(&self) -> &str {
        "claude-code"
    }

    fn display_name(&self) -> &str {
        "Claude Code (CLA)"
    }

    fn detect(&self) -> ConnectorStatus {
        match self.inner.detect() {
            terraphim_session_analyzer::connectors::ConnectorStatus::Available {
                path,
                sessions_estimate,
            } => ConnectorStatus::Available {
                path,
                sessions_estimate,
            },
            terraphim_session_analyzer::connectors::ConnectorStatus::NotFound => {
                ConnectorStatus::NotFound
            }
            terraphim_session_analyzer::connectors::ConnectorStatus::Error(e) => {
                ConnectorStatus::Error(e)
            }
        }
    }

    fn default_path(&self) -> Option<PathBuf> {
        self.inner.default_path()
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let cla_options = to_cla_options(options);

        // CLA import is synchronous, wrap in blocking task
        // Create a new connector inside the blocking task since it's stateless
        let sessions = tokio::task::spawn_blocking(move || {
            let connector = terraphim_session_analyzer::connectors::ClaudeCodeConnector;
            connector.import(&cla_options)
        })
        .await??;

        Ok(sessions
            .into_iter()
            .map(|ns| from_normalized_session(ns, "cla"))
            .collect())
    }
}

/// CLA-powered Cursor IDE connector
///
/// Uses claude-log-analyzer's Cursor connector for SQLite parsing.
#[cfg(feature = "tsa-full")]
#[derive(Debug, Default)]
pub struct ClaCursorConnector {
    inner: terraphim_session_analyzer::connectors::cursor::CursorConnector,
}

#[cfg(feature = "tsa-full")]
#[async_trait]
impl SessionConnector for ClaCursorConnector {
    fn source_id(&self) -> &str {
        "cursor"
    }

    fn display_name(&self) -> &str {
        "Cursor IDE"
    }

    fn detect(&self) -> ConnectorStatus {
        match self.inner.detect() {
            terraphim_session_analyzer::connectors::ConnectorStatus::Available {
                path,
                sessions_estimate,
            } => ConnectorStatus::Available {
                path,
                sessions_estimate,
            },
            terraphim_session_analyzer::connectors::ConnectorStatus::NotFound => {
                ConnectorStatus::NotFound
            }
            terraphim_session_analyzer::connectors::ConnectorStatus::Error(e) => {
                ConnectorStatus::Error(e)
            }
        }
    }

    fn default_path(&self) -> Option<PathBuf> {
        self.inner.default_path()
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let cla_options = to_cla_options(options);

        // CLA import is synchronous, wrap in blocking task
        // Create a new connector inside the blocking task since it's stateless
        let sessions = tokio::task::spawn_blocking(move || {
            let connector = terraphim_session_analyzer::connectors::cursor::CursorConnector;
            connector.import(&cla_options)
        })
        .await??;

        Ok(sessions
            .into_iter()
            .map(|ns| from_normalized_session(ns, "cursor"))
            .collect())
    }
}

/// Convert our ImportOptions to CLA's ImportOptions
fn to_cla_options(options: &ImportOptions) -> ClaImportOptions {
    ClaImportOptions {
        path: options.path.clone(),
        since: options.since,
        until: options.until,
        limit: options.limit,
        incremental: options.incremental,
    }
}

// Placeholder for when tsa-full is not enabled
#[cfg(not(feature = "tsa-full"))]
#[derive(Debug, Default)]
pub struct ClaCursorConnector;

#[cfg(not(feature = "tsa-full"))]
#[async_trait]
impl SessionConnector for ClaCursorConnector {
    fn source_id(&self) -> &str {
        "cursor-stub"
    }

    fn display_name(&self) -> &str {
        "Cursor IDE (requires tsa-full feature)"
    }

    fn detect(&self) -> ConnectorStatus {
        ConnectorStatus::Error("Cursor support requires tsa-full feature".to_string())
    }

    fn default_path(&self) -> Option<PathBuf> {
        None
    }

    async fn import(&self, _options: &ImportOptions) -> Result<Vec<Session>> {
        anyhow::bail!("Cursor support requires tsa-full feature")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cla_claude_connector() {
        let connector = ClaClaudeConnector::default();
        assert_eq!(connector.source_id(), "claude-code");
        assert_eq!(connector.display_name(), "Claude Code (CLA)");
    }

    #[cfg(feature = "tsa-full")]
    #[test]
    fn test_cla_cursor_connector() {
        let connector = ClaCursorConnector::default();
        assert_eq!(connector.source_id(), "cursor");
        assert_eq!(connector.display_name(), "Cursor IDE");
    }
}
