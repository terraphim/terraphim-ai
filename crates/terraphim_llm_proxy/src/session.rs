//! Session management for tracking conversation context and usage
//!
//! This module provides session tracking with LRU cache and Redis integration
//! for distributed caching across multiple proxy instances.

use crate::{ProxyError, Result};
use lru::LruCache;
use redis::Commands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;
use uuid::Uuid;

/// Session information tracking usage and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier
    pub session_id: String,

    /// When the session was created (Unix timestamp)
    pub created_at: u64,

    /// Last activity timestamp (Unix timestamp)
    pub last_activity: u64,

    /// Total tokens used in this session
    pub total_tokens: u64,

    /// Number of requests in this session
    pub request_count: u64,

    /// Provider preferences based on usage patterns
    pub provider_preferences: HashMap<String, f64>,

    /// Conversation context (recent messages)
    pub context: Vec<ConversationMessage>,

    /// Session metadata
    pub metadata: HashMap<String, String>,
}

/// Message in the conversation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Message role (user/assistant)
    pub role: String,

    /// Message content (truncated for storage)
    pub content: String,

    /// Token count for this message
    pub token_count: u32,

    /// When this message was added (Unix timestamp)
    pub timestamp: u64,
}

/// Session manager with local LRU cache and Redis backend
pub struct SessionManager {
    /// Local LRU cache for fast access
    cache: Arc<Mutex<LruCache<String, SessionInfo>>>,

    /// Redis client for distributed caching (optional)
    redis_client: Option<redis::Client>,

    /// Maximum number of sessions to keep in local cache
    max_sessions: usize,

    /// Maximum context messages per session
    max_context_messages: usize,

    /// Session timeout duration
    session_timeout: Duration,
}

/// Configuration for session management
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum sessions in local LRU cache
    pub max_sessions: usize,

    /// Maximum context messages per session
    pub max_context_messages: usize,

    /// Session timeout duration
    pub session_timeout_minutes: u64,

    /// Redis URL (optional)
    pub redis_url: Option<String>,

    /// Enable Redis for distributed caching
    pub enable_redis: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_sessions: 1000,
            max_context_messages: 10,
            session_timeout_minutes: 60,
            redis_url: None,
            enable_redis: false,
        }
    }
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: SessionConfig) -> Result<Self> {
        let cache = LruCache::new(
            NonZeroUsize::new(config.max_sessions)
                .ok_or_else(|| ProxyError::ConfigError("Max sessions must be > 0".to_string()))?,
        );

        let redis_client = if config.enable_redis {
            if let Some(redis_url) = &config.redis_url {
                Some(redis::Client::open(redis_url.as_str()).map_err(|e| {
                    ProxyError::ConfigError(format!("Redis connection failed: {}", e))
                })?)
            } else {
                return Err(ProxyError::ConfigError(
                    "Redis URL must be provided when Redis is enabled".to_string(),
                ));
            }
        } else {
            None
        };

        Ok(Self {
            cache: Arc::new(Mutex::new(cache)),
            redis_client,
            max_sessions: config.max_sessions,
            max_context_messages: config.max_context_messages,
            session_timeout: Duration::from_secs(config.session_timeout_minutes * 60),
        })
    }

    /// Extract session ID from request metadata or generate new one
    pub fn extract_or_create_session_id(
        &self,
        _request: &crate::token_counter::ChatRequest,
    ) -> String {
        // For now, always generate new session ID
        // TODO: Extract from request headers or other metadata when available
        Uuid::new_v4().to_string()
    }

    /// Get session info, creating if it doesn't exist
    pub async fn get_or_create_session(&self, session_id: &str) -> Result<SessionInfo> {
        // Try local cache first
        if let Some(session) = self.get_from_cache(session_id) {
            return Ok(session);
        }

        // Try Redis if available
        if let Some(ref redis_client) = self.redis_client {
            if let Ok(Some(session)) = self.get_from_redis(redis_client, session_id).await {
                // Store in local cache
                self.put_in_cache(session.clone());
                return Ok(session);
            }
        }

        // Create new session
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_session = SessionInfo {
            session_id: session_id.to_string(),
            created_at: now,
            last_activity: now,
            total_tokens: 0,
            request_count: 0,
            provider_preferences: HashMap::new(),
            context: Vec::new(),
            metadata: HashMap::new(),
        };

        // Store in cache and Redis
        self.put_in_cache(new_session.clone());
        if let Some(ref redis_client) = self.redis_client {
            let _ = self.put_in_redis(redis_client, &new_session).await;
        }

        Ok(new_session)
    }

    /// Update session with new request/response data
    pub async fn update_session(
        &self,
        session_id: &str,
        request_tokens: u32,
        response_tokens: u32,
        provider: &str,
        _model: &str,
        message_content: &str,
    ) -> Result<()> {
        let mut session = self.get_or_create_session(session_id).await?;

        // Update session stats
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        session.last_activity = now;
        session.total_tokens += request_tokens as u64 + response_tokens as u64;
        session.request_count += 1;

        // Update provider preferences (simple scoring based on usage)
        let current_score = session.provider_preferences.get(provider).unwrap_or(&0.0);
        let new_score = current_score + 0.1; // Simple increment, could be more sophisticated
        session
            .provider_preferences
            .insert(provider.to_string(), new_score.min(1.0));

        // Add to context (keep only recent messages)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        session.context.push(ConversationMessage {
            role: "assistant".to_string(),
            content: message_content.chars().take(500).collect(), // Truncate for storage
            token_count: response_tokens,
            timestamp: now,
        });

        // Keep only recent context messages
        if session.context.len() > self.max_context_messages {
            session.context = session
                .context
                .into_iter()
                .rev()
                .take(self.max_context_messages)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
        }

        // Update cache and Redis
        self.put_in_cache(session.clone());
        if let Some(ref redis_client) = self.redis_client {
            let _ = self.put_in_redis(redis_client, &session).await;
        }

        Ok(())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let mut to_remove = Vec::new();

        // Find expired sessions in cache (hold lock briefly)
        {
            let cache = self.cache.lock().unwrap();
            for (session_id, session) in cache.iter() {
                if now - session.last_activity > self.session_timeout.as_secs() {
                    to_remove.push(session_id.clone());
                }
            }
        }

        // Remove from cache (hold lock briefly again)
        {
            let mut cache = self.cache.lock().unwrap();
            for session_id in &to_remove {
                cache.pop(session_id);
            }
        }

        // Remove from Redis if available
        if let Some(ref redis_client) = self.redis_client {
            for session_id in &to_remove {
                let _ = self.delete_from_redis(redis_client, session_id).await;
            }
        }

        info!("Cleaned up {} expired sessions", to_remove.len());
        Ok(to_remove.len())
    }

    /// Get session statistics
    pub fn get_stats(&self) -> SessionStats {
        let cache = self.cache.lock().unwrap();
        SessionStats {
            active_sessions: cache.len(),
            max_sessions: self.max_sessions,
        }
    }

    // Private helper methods

    fn get_from_cache(&self, session_id: &str) -> Option<SessionInfo> {
        self.cache.lock().unwrap().get(session_id).cloned()
    }

    fn put_in_cache(&self, session: SessionInfo) {
        let mut cache = self.cache.lock().unwrap();
        // Note: LRU cache will automatically evict old entries when full
        cache.put(session.session_id.clone(), session);
    }

    async fn get_from_redis(
        &self,
        client: &redis::Client,
        session_id: &str,
    ) -> redis::RedisResult<Option<SessionInfo>> {
        let mut conn = client.get_connection()?;
        let key = format!("session:{}", session_id);

        match conn.get::<&str, String>(&key) {
            Ok(data) => {
                match serde_json::from_str::<SessionInfo>(&data) {
                    Ok(session) => {
                        // Check if session is expired
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        if now - session.last_activity > self.session_timeout.as_secs() {
                            let _ = self.delete_from_redis(client, session_id).await;
                            Ok(None)
                        } else {
                            Ok(Some(session))
                        }
                    }
                    Err(_) => Ok(None),
                }
            }
            Err(_) => Ok(None),
        }
    }

    async fn put_in_redis(
        &self,
        client: &redis::Client,
        session: &SessionInfo,
    ) -> redis::RedisResult<()> {
        let mut conn = client.get_connection()?;
        let key = format!("session:{}", session.session_id);
        let data = serde_json::to_string(session).unwrap_or_default();

        // Set with expiration
        let _: () = conn.set_ex(&key, &data, self.session_timeout.as_secs())?;
        Ok(())
    }

    async fn delete_from_redis(
        &self,
        client: &redis::Client,
        session_id: &str,
    ) -> redis::RedisResult<()> {
        let mut conn = client.get_connection()?;
        let key = format!("session:{}", session_id);
        let _: () = conn.del(&key)?;
        Ok(())
    }
}

/// Session statistics
#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub active_sessions: usize,
    pub max_sessions: usize,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(SessionConfig::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new(SessionConfig {
            max_sessions: 10,
            ..Default::default()
        })
        .unwrap();

        let session_id = "test-session-123";
        let session = manager.get_or_create_session(session_id).await.unwrap();

        assert_eq!(session.session_id, session_id);
        assert_eq!(session.request_count, 0);
        assert_eq!(session.total_tokens, 0);
    }

    #[tokio::test]
    async fn test_session_update() {
        let manager = SessionManager::new(SessionConfig {
            max_sessions: 10,
            ..Default::default()
        })
        .unwrap();

        let session_id = "test-session-456";
        manager
            .update_session(
                session_id,
                100,
                200,
                "openrouter",
                "claude-3.5-sonnet",
                "Hello world",
            )
            .await
            .unwrap();

        let session = manager.get_or_create_session(session_id).await.unwrap();
        assert_eq!(session.request_count, 1);
        assert_eq!(session.total_tokens, 300);
        assert!(session.provider_preferences.contains_key("openrouter"));
    }

    #[tokio::test]
    async fn test_session_context_management() {
        let manager = SessionManager::new(SessionConfig {
            max_sessions: 10,
            max_context_messages: 3,
            ..Default::default()
        })
        .unwrap();

        let session_id = "test-session-789";

        // Add multiple messages
        for i in 0..5 {
            manager
                .update_session(
                    session_id,
                    10,
                    20,
                    "openrouter",
                    "claude-3.5-sonnet",
                    &format!("Message {}", i),
                )
                .await
                .unwrap();
        }

        let session = manager.get_or_create_session(session_id).await.unwrap();
        assert_eq!(session.context.len(), 3); // Should be limited to max_context_messages
    }
}
