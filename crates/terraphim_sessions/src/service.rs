//! Session Service - High-level API for session management
//!
//! This module provides a unified interface for working with sessions
//! from multiple AI coding assistants.

use crate::connector::{ConnectorRegistry, ConnectorStatus, ImportOptions};
use crate::model::{Session, SessionId};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session service for unified session management
pub struct SessionService {
    /// Connector registry
    registry: ConnectorRegistry,
    /// Cached sessions (in-memory)
    cache: Arc<RwLock<HashMap<SessionId, Session>>>,
}

impl SessionService {
    /// Create a new session service
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: ConnectorRegistry::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create session service with custom registry
    #[must_use]
    pub fn with_registry(registry: ConnectorRegistry) -> Self {
        Self {
            registry,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the connector registry
    #[must_use]
    pub fn registry(&self) -> &ConnectorRegistry {
        &self.registry
    }

    /// Detect available session sources
    pub fn detect_sources(&self) -> Vec<SourceInfo> {
        self.registry
            .detect_all()
            .into_iter()
            .map(|(id, status)| {
                let connector = self.registry.get(id);
                SourceInfo {
                    id: id.to_string(),
                    name: connector.map(|c| c.display_name().to_string()),
                    status,
                }
            })
            .collect()
    }

    /// Import sessions from a specific source
    pub async fn import_from(
        &self,
        source_id: &str,
        options: &ImportOptions,
    ) -> Result<Vec<Session>> {
        let connector = self
            .registry
            .get(source_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown source: {}", source_id))?;

        let sessions = connector.import(options).await?;

        // Update cache
        let mut cache = self.cache.write().await;
        for session in &sessions {
            cache.insert(session.id.clone(), session.clone());
        }

        Ok(sessions)
    }

    /// Import sessions from all available sources
    pub async fn import_all(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let sessions = self.registry.import_all(options).await?;

        // Update cache
        let mut cache = self.cache.write().await;
        for session in &sessions {
            cache.insert(session.id.clone(), session.clone());
        }

        Ok(sessions)
    }

    /// List all cached sessions
    pub async fn list_sessions(&self) -> Vec<Session> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }

    /// Get a session by ID
    pub async fn get_session(&self, id: &SessionId) -> Option<Session> {
        let cache = self.cache.read().await;
        cache.get(id).cloned()
    }

    /// Search sessions by query string
    pub async fn search(&self, query: &str) -> Vec<Session> {
        let cache = self.cache.read().await;
        let query_lower = query.to_lowercase();

        cache
            .values()
            .filter(|session| {
                // Search in title
                if let Some(title) = &session.title {
                    if title.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }

                // Search in project path
                if let Some(path) = &session.metadata.project_path {
                    if path.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }

                // Search in message content
                for msg in &session.messages {
                    if msg.content.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }

                false
            })
            .cloned()
            .collect()
    }

    /// Get sessions by source
    pub async fn sessions_by_source(&self, source: &str) -> Vec<Session> {
        let cache = self.cache.read().await;
        cache
            .values()
            .filter(|s| s.source == source)
            .cloned()
            .collect()
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Clear the session cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Load sessions into cache (for CLI persistence)
    pub async fn load_sessions(&self, sessions: Vec<Session>) {
        let mut cache = self.cache.write().await;
        for session in sessions {
            cache.insert(session.id.clone(), session);
        }
    }

    /// Get summary statistics
    pub async fn statistics(&self) -> SessionStatistics {
        let cache = self.cache.read().await;

        let mut total_messages = 0;
        let mut total_user_messages = 0;
        let mut total_assistant_messages = 0;
        let mut sources: HashMap<String, usize> = HashMap::new();

        for session in cache.values() {
            total_messages += session.message_count();
            total_user_messages += session.user_message_count();
            total_assistant_messages += session.assistant_message_count();

            *sources.entry(session.source.clone()).or_default() += 1;
        }

        SessionStatistics {
            total_sessions: cache.len(),
            total_messages,
            total_user_messages,
            total_assistant_messages,
            sessions_by_source: sources,
        }
    }
}

impl Default for SessionService {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a session source
#[derive(Debug, Clone)]
pub struct SourceInfo {
    /// Source ID
    pub id: String,
    /// Human-readable name
    pub name: Option<String>,
    /// Detection status
    pub status: ConnectorStatus,
}

impl SourceInfo {
    /// Check if source is available
    pub fn is_available(&self) -> bool {
        self.status.is_available()
    }
}

/// Session statistics
#[derive(Debug, Clone, Default)]
pub struct SessionStatistics {
    /// Total number of sessions
    pub total_sessions: usize,
    /// Total number of messages across all sessions
    pub total_messages: usize,
    /// Total user messages
    pub total_user_messages: usize,
    /// Total assistant messages
    pub total_assistant_messages: usize,
    /// Sessions grouped by source
    pub sessions_by_source: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_service_creation() {
        let service = SessionService::new();
        assert_eq!(service.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_detect_sources() {
        let service = SessionService::new();
        let sources = service.detect_sources();

        // Should have at least the native connector
        assert!(!sources.is_empty());
        assert!(sources.iter().any(|s| s.id == "claude-code-native"));
    }

    #[tokio::test]
    async fn test_statistics_empty() {
        let service = SessionService::new();
        let stats = service.statistics().await;

        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.total_messages, 0);
    }

    fn make_test_session(id: &str, source: &str, messages: Vec<crate::model::Message>) -> Session {
        Session {
            id: id.to_string(),
            source: source.to_string(),
            external_id: id.to_string(),
            title: Some(format!("Session {}", id)),
            source_path: std::path::PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages,
            metadata: crate::model::SessionMetadata::default(),
        }
    }

    #[tokio::test]
    async fn test_load_and_list_sessions() {
        let service = SessionService::new();
        let sessions = vec![
            make_test_session("s1", "test", vec![]),
            make_test_session("s2", "test", vec![]),
        ];
        service.load_sessions(sessions).await;

        let listed = service.list_sessions().await;
        assert_eq!(listed.len(), 2);
        assert_eq!(service.session_count().await, 2);
    }

    #[tokio::test]
    async fn test_get_session_by_id() {
        let service = SessionService::new();
        let sessions = vec![make_test_session("s1", "test", vec![])];
        service.load_sessions(sessions).await;

        let found = service.get_session(&"s1".to_string()).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "s1");

        let not_found = service.get_session(&"nonexistent".to_string()).await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_search_by_title() {
        let service = SessionService::new();
        let sessions = vec![
            make_test_session("s1", "test", vec![]),
            make_test_session("s2", "test", vec![]),
        ];
        service.load_sessions(sessions).await;

        let results = service.search("Session s1").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s1");
    }

    #[tokio::test]
    async fn test_search_by_message_content() {
        use crate::model::{Message, MessageRole};
        let service = SessionService::new();
        let sessions = vec![make_test_session(
            "s1",
            "test",
            vec![Message::text(
                0,
                MessageRole::User,
                "How to use Rust async?",
            )],
        )];
        service.load_sessions(sessions).await;

        let results = service.search("rust async").await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_case_insensitive() {
        let service = SessionService::new();
        let sessions = vec![make_test_session("s1", "test", vec![])];
        service.load_sessions(sessions).await;

        let results = service.search("SESSION S1").await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_no_results() {
        let service = SessionService::new();
        let sessions = vec![make_test_session("s1", "test", vec![])];
        service.load_sessions(sessions).await;

        let results = service.search("nonexistent-query").await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_sessions_by_source() {
        let service = SessionService::new();
        let sessions = vec![
            make_test_session("s1", "claude", vec![]),
            make_test_session("s2", "cursor", vec![]),
            make_test_session("s3", "claude", vec![]),
        ];
        service.load_sessions(sessions).await;

        let claude_sessions = service.sessions_by_source("claude").await;
        assert_eq!(claude_sessions.len(), 2);

        let cursor_sessions = service.sessions_by_source("cursor").await;
        assert_eq!(cursor_sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let service = SessionService::new();
        let sessions = vec![make_test_session("s1", "test", vec![])];
        service.load_sessions(sessions).await;
        assert_eq!(service.session_count().await, 1);

        service.clear_cache().await;
        assert_eq!(service.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_statistics_with_data() {
        use crate::model::{Message, MessageRole};
        let service = SessionService::new();
        let sessions = vec![
            make_test_session(
                "s1",
                "claude",
                vec![
                    Message::text(0, MessageRole::User, "Hello"),
                    Message::text(1, MessageRole::Assistant, "Hi"),
                ],
            ),
            make_test_session(
                "s2",
                "cursor",
                vec![Message::text(0, MessageRole::User, "Help")],
            ),
        ];
        service.load_sessions(sessions).await;

        let stats = service.statistics().await;
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_messages, 3);
        assert_eq!(stats.total_user_messages, 2);
        assert_eq!(stats.total_assistant_messages, 1);
        assert_eq!(stats.sessions_by_source.get("claude"), Some(&1));
        assert_eq!(stats.sessions_by_source.get("cursor"), Some(&1));
    }

    #[tokio::test]
    async fn test_load_sessions_deduplicates_by_id() {
        let service = SessionService::new();
        let sessions = vec![
            make_test_session("s1", "test", vec![]),
            make_test_session("s1", "test", vec![]), // duplicate
        ];
        service.load_sessions(sessions).await;
        assert_eq!(service.session_count().await, 1);
    }
}
