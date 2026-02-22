//! End-to-end tests for session management
//!
//! These tests validate the complete session management lifecycle including
//! session creation, updates, context management, cleanup, and Redis integration.

use std::sync::Arc;
use terraphim_llm_proxy::{
    session::{SessionConfig, SessionManager},
    token_counter::{ChatRequest, Message, MessageContent},
};
use uuid::Uuid;

/// Create a test session manager
fn create_test_session_manager(enable_redis: bool) -> SessionManager {
    let config = SessionConfig {
        max_sessions: 100,
        max_context_messages: 5,
        session_timeout_minutes: 1, // Short timeout for testing
        redis_url: if enable_redis {
            Some("redis://localhost:6379".to_string())
        } else {
            None
        },
        enable_redis,
    };
    SessionManager::new(config).unwrap()
}

/// Create a test chat request
fn create_test_request(content: &str, _session_id: Option<&str>) -> ChatRequest {
    ChatRequest {
        model: "test-model".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text(content.to_string()),
            ..Default::default()
        }],
        system: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: None,
        tools: None,
        thinking: None,
        ..Default::default()
    }
}

#[tokio::test]
async fn test_session_lifecycle_basic() {
    let session_manager = Arc::new(create_test_session_manager(false));
    let session_id = Uuid::new_v4().to_string();

    // Test: Create new session
    let session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();
    assert_eq!(session.session_id, session_id);
    assert_eq!(session.request_count, 0);
    assert_eq!(session.total_tokens, 0);
    assert!(session.context.is_empty());

    // Test: Update session with request/response
    session_manager
        .update_session(
            &session_id,
            50,  // request tokens
            100, // response tokens
            "openrouter",
            "claude-3.5-sonnet",
            "This is a test response",
        )
        .await
        .unwrap();

    // Verify session was updated
    let updated_session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();
    assert_eq!(updated_session.request_count, 1);
    assert_eq!(updated_session.total_tokens, 150);
    assert_eq!(updated_session.context.len(), 1);
    assert_eq!(updated_session.context[0].role, "assistant");
    assert!(updated_session
        .provider_preferences
        .contains_key("openrouter"));

    // Test: Multiple updates
    for i in 2..=5 {
        session_manager
            .update_session(
                &session_id,
                30,
                70,
                "openrouter",
                "claude-3.5-sonnet",
                &format!("Response {}", i),
            )
            .await
            .unwrap();
    }

    let final_session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();
    assert_eq!(final_session.request_count, 5);
    assert_eq!(final_session.total_tokens, 550); // 150 + 4 * 100
    assert_eq!(final_session.context.len(), 5); // Should keep all 5 (max_context_messages = 5)

    // Test: Provider preference scoring
    let openrouter_score = final_session
        .provider_preferences
        .get("openrouter")
        .unwrap();
    assert!(*openrouter_score > 0.0);
    assert_eq!(final_session.provider_preferences.len(), 1);
}

#[tokio::test]
async fn test_session_context_limiting() {
    let session_manager = Arc::new(create_test_session_manager(false));
    let session_id = Uuid::new_v4().to_string();

    // Create session
    let _session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Add more messages than the context limit (5)
    for i in 1..=10 {
        session_manager
            .update_session(
                &session_id,
                10,
                20,
                "openrouter",
                "claude-3.5-sonnet",
                &format!("Message {}", i),
            )
            .await
            .unwrap();
    }

    let session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Should only keep the 5 most recent messages
    assert_eq!(session.context.len(), 5);

    // Verify the messages are the most recent ones
    assert_eq!(session.context[0].content, "Message 6");
    assert_eq!(session.context[4].content, "Message 10");

    // Verify timestamps are in ascending order
    for i in 1..session.context.len() {
        assert!(session.context[i].timestamp >= session.context[i - 1].timestamp);
    }
}

#[tokio::test]
async fn test_multiple_provider_preferences() {
    let session_manager = Arc::new(create_test_session_manager(false));
    let session_id = Uuid::new_v4().to_string();

    // Create session
    let _session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Use different providers
    let providers = vec![
        ("openrouter", "claude-3.5-sonnet"),
        ("deepseek", "deepseek-chat"),
        ("openrouter", "claude-3.5-sonnet"),
        ("deepseek", "deepseek-chat"),
        ("openrouter", "claude-3.5-sonnet"),
    ];

    for (provider, model) in providers {
        session_manager
            .update_session(
                &session_id,
                25,
                75,
                provider,
                model,
                "Response from provider",
            )
            .await
            .unwrap();
    }

    let session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Should have preferences for both providers
    assert_eq!(session.provider_preferences.len(), 2);
    assert!(session.provider_preferences.contains_key("openrouter"));
    assert!(session.provider_preferences.contains_key("deepseek"));

    // OpenRouter should have higher score (used 3 times vs 2 times)
    let openrouter_score = session.provider_preferences.get("openrouter").unwrap();
    let deepseek_score = session.provider_preferences.get("deepseek").unwrap();
    assert!(openrouter_score > deepseek_score);

    // Scores should be capped at 1.0
    assert!(openrouter_score <= &1.0);
    assert!(deepseek_score <= &1.0);
}

#[tokio::test]
async fn test_session_cleanup_expired() {
    let session_manager = Arc::new(create_test_session_manager(false));

    // Create multiple sessions
    let session_ids: Vec<String> = (1..=5).map(|_| Uuid::new_v4().to_string()).collect();

    for session_id in &session_ids {
        session_manager
            .get_or_create_session(session_id)
            .await
            .unwrap();
        session_manager
            .update_session(
                session_id,
                10,
                20,
                "openrouter",
                "claude-3.5-sonnet",
                "Test response",
            )
            .await
            .unwrap();
    }

    // Verify all sessions exist
    let stats = session_manager.get_stats();
    assert_eq!(stats.active_sessions, 5);

    // Wait for sessions to expire (timeout is 1 minute, but we'll manually test cleanup)
    // In a real scenario, we'd wait, but for testing we'll simulate expired sessions
    // by creating sessions with old timestamps manually through the implementation

    // For now, let's test the cleanup function runs without error
    let cleaned_count = session_manager.cleanup_expired_sessions().await.unwrap();

    // Should clean up 0 sessions since they're all recent
    assert_eq!(cleaned_count, 0);

    let stats_after = session_manager.get_stats();
    assert_eq!(stats_after.active_sessions, 5);
}

#[tokio::test]
async fn test_session_manager_stats() {
    let session_manager = Arc::new(create_test_session_manager(false));

    // Initial stats
    let stats = session_manager.get_stats();
    assert_eq!(stats.active_sessions, 0);
    assert_eq!(stats.max_sessions, 100);

    // Create some sessions
    let session_ids: Vec<String> = (1..=3).map(|_| Uuid::new_v4().to_string()).collect();

    for session_id in &session_ids {
        let _session = session_manager
            .get_or_create_session(session_id)
            .await
            .unwrap();
    }

    let stats_after = session_manager.get_stats();
    assert_eq!(stats_after.active_sessions, 3);
    assert_eq!(stats_after.max_sessions, 100);
}

#[tokio::test]
async fn test_concurrent_session_access() {
    let session_manager = Arc::new(create_test_session_manager(false));
    let session_id = Uuid::new_v4().to_string();

    // Create session
    let _session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Spawn multiple concurrent tasks to update the same session
    let mut handles = vec![];

    for i in 1..=10 {
        let session_manager_clone = Arc::clone(&session_manager);
        let session_id_clone = session_id.clone();

        let handle = tokio::spawn(async move {
            session_manager_clone
                .update_session(
                    &session_id_clone,
                    10,
                    20,
                    "openrouter",
                    "claude-3.5-sonnet",
                    &format!("Concurrent response {}", i),
                )
                .await
                .unwrap();
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify session state is consistent
    let session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();
    assert_eq!(session.request_count, 10);
    assert_eq!(session.total_tokens, 300); // 10 * (10 + 20)
    assert_eq!(session.context.len(), 5); // Limited by max_context_messages
}

#[tokio::test]
async fn test_session_id_extraction() {
    let session_manager = create_test_session_manager(false);

    // Test request without session information
    let request = create_test_request("Hello", None);
    let session_id = session_manager.extract_or_create_session_id(&request);

    // Should generate a new UUID
    assert!(!session_id.is_empty());
    assert_eq!(session_id.len(), 36); // UUID length

    // Multiple calls should generate different session IDs
    let request2 = create_test_request("Hello again", None);
    let session_id2 = session_manager.extract_or_create_session_id(&request2);

    assert_ne!(session_id, session_id2);
}

#[tokio::test]
async fn test_session_metadata_storage() {
    let session_manager = Arc::new(create_test_session_manager(false));
    let session_id = Uuid::new_v4().to_string();

    // Create session
    let mut session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Add metadata
    session
        .metadata
        .insert("user_agent".to_string(), "test-client/1.0".to_string());
    session
        .metadata
        .insert("ip_address".to_string(), "127.0.0.1".to_string());

    // Note: In the current implementation, metadata isn't persisted through update_session
    // This test documents the current behavior and could be enhanced in the future

    // Update session
    session_manager
        .update_session(
            &session_id,
            15,
            25,
            "deepseek",
            "deepseek-chat",
            "Response with metadata",
        )
        .await
        .unwrap();

    let updated_session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Verify session was updated (metadata would need to be explicitly stored in a real implementation)
    assert_eq!(updated_session.request_count, 1);
    assert_eq!(updated_session.total_tokens, 40);
}

#[tokio::test]
async fn test_session_content_truncation() {
    let session_manager = Arc::new(create_test_session_manager(false));
    let session_id = Uuid::new_v4().to_string();

    // Create session
    let _session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Add a very long response
    let long_content = "A".repeat(1000);
    session_manager
        .update_session(
            &session_id,
            100,
            200,
            "openrouter",
            "claude-3.5-sonnet",
            &long_content,
        )
        .await
        .unwrap();

    let session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();

    // Content should be truncated to 500 characters (as per implementation)
    assert_eq!(session.context.len(), 1);
    assert_eq!(session.context[0].content.len(), 500);
    assert!(session.context[0].content.chars().all(|c| c == 'A'));
}

#[ignore] // Requires Redis to be running
#[tokio::test]
async fn test_session_redis_integration() {
    let session_manager = Arc::new(create_test_session_manager(true));
    let session_id = Uuid::new_v4().to_string();

    // Create session (should store in Redis)
    let session = session_manager
        .get_or_create_session(&session_id)
        .await
        .unwrap();
    assert_eq!(session.session_id, session_id);

    // Update session
    session_manager
        .update_session(
            &session_id,
            50,
            100,
            "openrouter",
            "claude-3.5-sonnet",
            "Redis test response",
        )
        .await
        .unwrap();

    // Create a new session manager instance to test Redis persistence
    let session_manager2 = Arc::new(create_test_session_manager(true));

    // Should retrieve session from Redis
    let retrieved_session = session_manager2
        .get_or_create_session(&session_id)
        .await
        .unwrap();
    assert_eq!(retrieved_session.session_id, session_id);
    assert_eq!(retrieved_session.request_count, 1);
    assert_eq!(retrieved_session.total_tokens, 150);
    assert_eq!(retrieved_session.context.len(), 1);
}

#[test]
fn test_session_config_validation() {
    // Valid configuration
    let valid_config = SessionConfig {
        max_sessions: 100,
        max_context_messages: 10,
        session_timeout_minutes: 60,
        redis_url: None,
        enable_redis: false,
    };
    assert!(SessionManager::new(valid_config).is_ok());

    // Invalid: max_sessions = 0
    let invalid_config = SessionConfig {
        max_sessions: 0,
        ..Default::default()
    };
    assert!(SessionManager::new(invalid_config).is_err());

    // Invalid: Redis enabled but no URL
    let redis_invalid_config = SessionConfig {
        enable_redis: true,
        redis_url: None,
        ..Default::default()
    };
    assert!(SessionManager::new(redis_invalid_config).is_err());
}

#[test]
fn test_session_config_default() {
    let config = SessionConfig::default();
    assert_eq!(config.max_sessions, 1000);
    assert_eq!(config.max_context_messages, 10);
    assert_eq!(config.session_timeout_minutes, 60);
    assert!(!config.enable_redis);
    assert_eq!(config.redis_url, None);
}

#[test]
fn test_session_manager_default() {
    let manager = SessionManager::default();
    let stats = manager.get_stats();
    assert_eq!(stats.active_sessions, 0);
    assert_eq!(stats.max_sessions, 1000);
}
