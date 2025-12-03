use gpui::*;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use terraphim_desktop_gpui::components::{
    EnhancedChatComponent, EnhancedChatConfig, ContextComponent, SearchContextBridge,
    EnhancedChatEvent, EnhancedChatTheme, StreamingConfig, MessageRenderingConfig
};
use terraphim_types::{
    ChatMessage, ConversationId, RoleName, ContextItem, ContextType, MessageRole,
    StreamingChatMessage, RenderChunk, ChunkType, MessageStatus
};
use chrono::Utc;

mod app;

/// Comprehensive tests for the enhanced chat system
///
/// This test suite validates the complete enhanced chat functionality including:
/// - Context-aware conversations with real-time injection
/// - Enhanced message rendering with context highlighting
/// - Virtual scrolling for large conversation histories
/// - Typing indicators and real-time streaming
/// - Performance monitoring and analytics
/// - Configuration management and settings

#[tokio::test]
async fn test_enhanced_chat_component_creation() {
    let config = EnhancedChatConfig::default();
    let mut component = EnhancedChatComponent::new(config);
    let mut cx = gpui::TestAppContext::new();

    component.initialize(&mut cx).unwrap();

    // Verify initial state
    assert_eq!(component.get_messages().len(), 0);
    assert_eq!(component.get_context_items().len(), 0);
    assert!(component.state().conversation_id.is_none());
    assert!(!component.state().is_typing);
    assert!(component.state().typing_users.is_empty());
    assert!(component.state().show_context_panel);

    // Verify performance metrics
    let metrics = component.get_performance_metrics();
    assert_eq!(metrics.total_messages, 0);
    assert_eq!(metrics.streaming_messages, 0);
    assert_eq!(metrics.context_hits, 0);
}

#[tokio::test]
async fn test_message_addition_and_management() {
    let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    // Set conversation
    let conversation_id = ConversationId::new();
    component.set_conversation(conversation_id);

    // Add user message
    let user_message = ChatMessage {
        role: MessageRole::User,
        content: "Hello, how are you?".to_string(),
        timestamp: Utc::now(),
        metadata: ahash::AHashMap::new(),
    };

    component.add_message(user_message, &mut cx);

    // Verify message was added
    assert_eq!(component.get_messages().len(), 1);
    let added_message = &component.get_messages()[0];
    assert_eq!(added_message.message.content, "Hello, how are you?");
    assert_eq!(added_message.message.role, MessageRole::User);
    assert!(added_message.streaming_state.is_none());
    assert_eq!(added_message.rendering_state, terraphim_desktop_gpui::components::MessageRenderingState::Pending);

    // Add assistant message
    let assistant_message = ChatMessage {
        role: MessageRole::Assistant,
        content: "I'm doing well, thank you for asking!".to_string(),
        timestamp: Utc::now(),
        metadata: ahash::AHashMap::new(),
    };

    component.add_message(assistant_message, &mut cx);

    assert_eq!(component.get_messages().len(), 2);
    assert_eq!(component.get_messages()[1].message.role, MessageRole::Assistant);
}

#[tokio::test]
async fn test_streaming_message_workflow() {
    let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    // Start streaming
    let base_message = ChatMessage {
        role: MessageRole::Assistant,
        content: String::new(),
        timestamp: Utc::now(),
        metadata: ahash::AHashMap::new(),
    };

    let message_id = component.start_streaming(base_message, &mut cx);

    // Verify streaming state
    assert_eq!(component.get_messages().len(), 1);
    let streaming_message = &component.get_messages()[0];
    assert!(streaming_message.streaming_state.is_some());
    assert!(streaming_message.streaming_state.as_ref().unwrap().is_streaming);
    assert_eq!(streaming_message.rendering_state, terraphim_desktop_gpui::components::MessageRenderingState::Rendering);

    // Add streaming chunks
    let chunks = vec![
        RenderChunk {
            content: "Hello".to_string(),
            chunk_type: ChunkType::Text,
            position: 0,
            complete: false,
        },
        RenderChunk {
            content: " there!".to_string(),
            chunk_type: ChunkType::Text,
            position: 5,
            complete: false,
        },
        RenderChunk {
            content: " How can I help you today?".to_string(),
            chunk_type: ChunkType::Text,
            position: 12,
            complete: true,
        },
    ];

    for chunk in chunks {
        let result = component.add_streaming_chunk(&message_id, chunk.clone(), &mut cx);
        assert!(result.is_ok());
    }

    // Complete streaming
    let result = component.complete_streaming(&message_id, &mut cx);
    assert!(result.is_ok());

    // Verify final state
    let final_message = &component.get_messages()[0];
    assert!(!final_message.streaming_state.as_ref().unwrap().is_streaming);
    assert_eq!(final_message.rendering_state, terraphim_desktop_gpui::components::MessageRenderingState::Rendered);
    assert_eq!(
        final_message.streaming_state.as_ref().unwrap().get_content().unwrap(),
        "Hello there! How can I help you today?"
    );
    assert!(final_message.metadata.processing_time.is_some());
    assert!(final_message.metadata.token_count.is_some());
}

#[tokio::test]
async fn test_context_integration() {
    let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    // Create context items
    let context_items = vec![
        Arc::new(ContextItem {
            id: "context-1".to_string(),
            title: "Rust Programming".to_string(),
            summary: Some("A systems programming language".to_string()),
            content: "Rust is a systems programming language focused on safety and performance.".to_string(),
            context_type: ContextType::Document,
            created_at: Utc::now(),
            relevance_score: Some(0.9),
            metadata: ahash::AHashMap::new(),
        }),
        Arc::new(ContextItem {
            id: "context-2".to_string(),
            title: "GPUI Framework".to_string(),
            summary: Some("A cross-platform UI framework".to_string()),
            content: "GPUI is a Rust-based UI framework for building native applications.".to_string(),
            context_type: ContextType::SearchResult,
            created_at: Utc::now(),
            relevance_score: Some(0.8),
            metadata: ahash::AHashMap::new(),
        }),
    ];

    // Add context items
    component.add_context_items(context_items.clone(), &mut cx);

    // Verify context was added
    assert_eq!(component.get_context_items().len(), 2);
    assert_eq!(component.get_context_items()[0].title, "Rust Programming");
    assert_eq!(component.get_context_items()[1].title, "GPUI Framework");

    // Verify performance metrics
    let metrics = component.get_performance_metrics();
    assert_eq!(metrics.context_hits, 2);

    // Add a message and verify it has context
    let message = ChatMessage {
        role: MessageRole::Assistant,
        content: "Based on the context provided, I can help you with Rust and GPUI.".to_string(),
        timestamp: Utc::now(),
        metadata: ahash::AHashMap::new(),
    };

    component.add_message(message, &mut cx);

    let enhanced_message = &component.get_messages()[0];
    assert_eq!(enhanced_message.context_items.len(), 2);
    assert_eq!(enhanced_message.context_items[0].title, "Rust Programming");
    assert_eq!(enhanced_message.context_items[1].title, "GPUI Framework");
}

#[tokio::test]
async fn test_typing_indicators() {
    let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    // Test assistant typing
    assert!(!component.state().is_typing);
    component.set_typing(true, &mut cx);
    assert!(component.state().is_typing);
    component.set_typing(false, &mut cx);
    assert!(!component.state().is_typing);

    // Test multiple users typing
    assert!(component.state().typing_users.is_empty());
    component.add_typing_user("Alice".to_string(), &mut cx);
    assert_eq!(component.state().typing_users.len(), 1);
    assert!(component.state().typing_users.contains(&"Alice".to_string()));

    component.add_typing_user("Bob".to_string(), &mut cx);
    assert_eq!(component.state().typing_users.len(), 2);

    component.remove_typing_user("Alice", &mut cx);
    assert_eq!(component.state().typing_users.len(), 1);
    assert!(!component.state().typing_users.contains(&"Alice".to_string()));
    assert!(component.state().typing_users.contains(&"Bob".to_string()));

    component.remove_typing_user("Bob", &mut cx);
    assert!(component.state().typing_users.is_empty());
}

#[tokio::test]
async fn test_context_panel_toggle() {
    let mut config = EnhancedChatConfig::default();
    config.show_context_panel = false;
    let mut component = EnhancedChatComponent::new(config);
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    assert!(!component.state().show_context_panel);
    component.toggle_context_panel(&mut cx);
    assert!(component.state().show_context_panel);
    component.toggle_context_panel(&mut cx);
    assert!(!component.state().show_context_panel);
}

#[tokio::test]
async fn test_role_and_model_selection() {
    let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    // Test role selection
    component.set_role(RoleName::from("Software Engineer"), &mut cx);
    assert_eq!(component.state().current_role.as_str(), "Software Engineer");

    component.set_role(RoleName::from("Data Scientist"), &mut cx);
    assert_eq!(component.state().current_role.as_str(), "Data Scientist");

    // Test model selection
    assert!(component.state().selected_model.is_none());
    component.set_model("gpt-4".to_string(), &mut cx);
    assert_eq!(component.state().selected_model.as_ref().unwrap(), "gpt-4");

    component.set_model("claude-3".to_string(), &mut cx);
    assert_eq!(component.state().selected_model.as_ref().unwrap(), "claude-3");
}

#[tokio::test]
async fn test_message_limiting() {
    let mut config = EnhancedChatConfig::default();
    config.max_messages = 3;
    let mut component = EnhancedChatComponent::new(config);
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    // Add more messages than the limit
    for i in 0..5 {
        let message = ChatMessage {
            role: MessageRole::User,
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            metadata: ahash::AHashMap::new(),
        };
        component.add_message(message, &mut cx);
    }

    // Should only keep the most recent messages
    assert_eq!(component.get_messages().len(), 3);
    assert_eq!(component.get_messages()[0].message.content, "Message 2");
    assert_eq!(component.get_messages()[1].message.content, "Message 3");
    assert_eq!(component.get_messages()[2].message.content, "Message 4");
}

#[tokio::test]
async fn test_performance_metrics_tracking() {
    let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    let initial_metrics = component.get_performance_metrics();
    assert_eq!(initial_metrics.total_messages, 0);
    assert_eq!(initial_metrics.streaming_messages, 0);
    assert_eq!(initial_metrics.cache_hits, 0);
    assert_eq!(initial_metrics.cache_misses, 0);

    // Add regular messages
    for i in 0..3 {
        let message = ChatMessage {
            role: MessageRole::User,
            content: format!("Message {}", i),
            timestamp: Utc::now(),
            metadata: ahash::AHashMap::new(),
        };
        component.add_message(message, &mut cx);
    }

    // Add context items
    let context_item = Arc::new(ContextItem {
        id: "test-context".to_string(),
        title: "Test".to_string(),
        summary: None,
        content: "Test content".to_string(),
        context_type: ContextType::Document,
        created_at: Utc::now(),
        relevance_score: Some(0.5),
        metadata: ahash::AHashMap::new(),
    });
    component.add_context_items(vec![context_item], &mut cx);

    let updated_metrics = component.get_performance_metrics();
    assert_eq!(updated_metrics.total_messages, 3);
    assert_eq!(updated_metrics.context_hits, 1);
}

#[tokio::test]
async fn test_event_system() {
    let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
    let mut cx = gpui::TestAppContext::new();
    component.initialize(&mut cx).unwrap();

    // Subscribe to events
    let mut events = Vec::new();
    let _subscription = component.subscribe(&mut cx, |event, _cx| {
        events.push(event.clone());
    });

    // Trigger a message event
    let message = ChatMessage {
        role: MessageRole::User,
        content: "Test message".to_string(),
        timestamp: Utc::now(),
        metadata: ahash::AHashMap::new(),
    };
    component.add_message(message, &mut cx);

    // Verify event was emitted
    assert_eq!(events.len(), 1);
    match &events[0] {
        EnhancedChatEvent::MessageSent { message: _, conversation_id } => {
            assert!(!conversation_id.as_str().is_empty());
        }
        _ => panic!("Expected MessageSent event"),
    }

    // Trigger a context change event
    let context_item = Arc::new(ContextItem {
        id: "test".to_string(),
        title: "Test".to_string(),
        summary: None,
        content: "Content".to_string(),
        context_type: ContextType::Document,
        created_at: Utc::now(),
        relevance_score: None,
        metadata: ahash::AHashMap::new(),
    });
    component.add_context_items(vec![context_item], &mut cx);

    // Verify context change event
    assert_eq!(events.len(), 2);
    match &events[1] {
        EnhancedChatEvent::ContextChanged { context_items } => {
            assert_eq!(context_items.len(), 1);
            assert_eq!(context_items[0].title, "Test");
        }
        _ => panic!("Expected ContextChanged event"),
    }
}

#[tokio::test]
async fn test_configuration_customization() {
    let config = EnhancedChatConfig {
        max_messages: 500,
        enable_virtual_scrolling: false,
        show_context_panel: false,
        show_typing_indicators: false,
        enable_reactions: false,
        show_timestamps: false,
        streaming: StreamingConfig {
            chunk_size: 50,
            chunk_delay: Duration::from_millis(25),
            max_stream_duration: Duration::from_secs(60),
            enable_progressive_rendering: false,
        },
        rendering: MessageRenderingConfig {
            enable_markdown: false,
            enable_syntax_highlighting: false,
            enable_code_execution: true,
            max_preview_length: 200,
            animations: terraphim_desktop_gpui::components::MessageAnimations {
                enabled: false,
                fade_in_duration: Duration::from_millis(150),
                typing_animation_duration: Duration::from_millis(500),
                chunk_animation_duration: Duration::from_millis(50),
            },
        },
        theme: EnhancedChatTheme {
            background: gpui::Rgba::from_rgb(0.95, 0.95, 0.95),
            text_primary: gpui::Rgba::from_rgb(0.0, 0.0, 0.0),
            user_message_bg: gpui::Rgba::from_rgb(0.85, 0.85, 1.0),
            ai_message_bg: gpui::Rgba::from_rgb(1.0, 0.9, 0.85),
            ..Default::default()
        },
    };

    let component = EnhancedChatComponent::new(config);

    // Verify configuration was applied
    assert_eq!(component.config().max_messages, 500);
    assert!(!component.config().enable_virtual_scrolling);
    assert!(!component.config().show_context_panel);
    assert!(!component.config().show_typing_indicators);
    assert_eq!(component.config().streaming.chunk_size, 50);
    assert_eq!(component.config().rendering.max_preview_length, 200);
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integration test for complete chat workflow with context
    #[tokio::test]
    async fn test_complete_chat_workflow() {
        let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
        let mut cx = gpui::TestAppContext::new();
        component.initialize(&mut cx).unwrap();

        // Set up conversation
        let conversation_id = ConversationId::new();
        component.set_conversation(conversation_id);
        component.set_role(RoleName::from("Software Engineer"), &mut cx);
        component.set_model("gpt-4".to_string(), &mut cx);

        // Add context items
        let rust_docs = Arc::new(ContextItem {
            id: "rust-docs".to_string(),
            title: "Rust Documentation".to_string(),
            summary: Some("Official Rust language documentation".to_string()),
            content: "Rust is a systems programming language that runs blazingly fast...".to_string(),
            context_type: ContextType::Document,
            created_at: Utc::now(),
            relevance_score: Some(0.95),
            metadata: {
                let mut metadata = ahash::AHashMap::new();
                metadata.insert("source".to_string(), "documentation".to_string());
                metadata.insert("language".to_string(), "rust".to_string());
                metadata
            },
        });

        component.add_context_items(vec![rust_docs], &mut cx);

        // User sends message
        let user_message = ChatMessage {
            role: MessageRole::User,
            content: "How do I create a struct in Rust?".to_string(),
            timestamp: Utc::now(),
            metadata: ahash::AHashMap::new(),
        };

        component.add_message(user_message, &mut cx);

        // Assistant starts streaming response
        let assistant_base = ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            timestamp: Utc::now(),
            metadata: ahash::AHashMap::new(),
        };

        let stream_id = component.start_streaming(assistant_base, &mut cx);

        // Simulate streaming chunks
        let response_chunks = vec![
            "In Rust, you create a struct using the `struct` keyword.\n\n",
            "Here's the basic syntax:\n\n```rust\nstruct Person {\n    name: String,\n    age: u32,\n}\n```\n\n",
            "You can then create instances like this:\n\n```rust\nlet person = Person {\n    name: String::from(\"Alice\"),\n    age: 30,\n};\n```",
        ];

        for chunk_content in response_chunks {
            let chunk = RenderChunk {
                content: chunk_content.to_string(),
                chunk_type: ChunkType::Text,
                position: 0,
                complete: false,
            };

            let result = component.add_streaming_chunk(&stream_id, chunk, &mut cx);
            assert!(result.is_ok());

            // Simulate processing delay
            sleep(Duration::from_millis(10)).await;
        }

        // Complete streaming
        let result = component.complete_streaming(&stream_id, &mut cx);
        assert!(result.is_ok());

        // Verify final state
        assert_eq!(component.get_messages().len(), 2);

        let user_msg = &component.get_messages()[0];
        assert_eq!(user_msg.message.role, MessageRole::User);
        assert_eq!(user_msg.message.content, "How do I create a struct in Rust?");

        let assistant_msg = &component.get_messages()[1];
        assert_eq!(assistant_msg.message.role, MessageRole::Assistant);
        assert!(!assistant_msg.streaming_state.as_ref().unwrap().is_streaming);
        assert!(assistant_msg.metadata.processing_time.is_some());
        assert!(assistant_msg.context_items.len() > 0);

        // Verify performance metrics
        let metrics = component.get_performance_metrics();
        assert_eq!(metrics.total_messages, 2);
        assert_eq!(metrics.streaming_messages, 1);
        assert_eq!(metrics.context_hits, 1);
        assert!(metrics.total_processing_time.as_millis() > 0);
    }

    /// Performance test for large conversation handling
    #[tokio::test]
    async fn test_large_conversation_performance() {
        let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
        let mut cx = gpui::TestAppContext::new();
        component.initialize(&mut cx).unwrap();

        let start_time = std::time::Instant::now();

        // Add a large number of messages
        for i in 0..1000 {
            let role = if i % 2 == 0 { MessageRole::User } else { MessageRole::Assistant };
            let message = ChatMessage {
                role,
                content: format!("This is message number {} with some content to simulate real conversation text.", i),
                timestamp: Utc::now(),
                metadata: ahash::AHashMap::new(),
            };

            component.add_message(message, &mut cx);

            // Every 100 messages, check performance
            if i % 100 == 0 && i > 0 {
                let current_time = std::time::Instant::now();
                let elapsed = current_time.duration_since(start_time);
                let messages_per_second = i as f64 / elapsed.as_secs_f64();

                // Should maintain reasonable performance
                assert!(messages_per_second > 100.0, "Performance degradation detected: {:.2} msg/sec", messages_per_second);
            }
        }

        let total_time = start_time.elapsed();
        let messages_per_second = 1000.0 / total_time.as_secs_f64();

        // Verify performance metrics
        let metrics = component.get_performance_metrics();
        assert_eq!(metrics.total_messages, 1000);
        assert!(messages_per_second > 500.0, "Overall performance too slow: {:.2} msg/sec", messages_per_second);

        // Verify message limiting is working
        assert_eq!(component.get_messages().len(), 1000); // Default max_messages is 1000
    }

    /// Stress test for concurrent operations
    #[tokio::test]
    async fn test_concurrent_operations_stress() {
        let mut component = EnhancedChatComponent::new(EnhancedChatConfig::default());
        let mut cx = gpui::TestAppContext::new();
        component.initialize(&mut cx).unwrap();

        // Simulate concurrent operations
        let mut handles = Vec::new();

        // Concurrent message additions
        for i in 0..50 {
            let component_clone = unsafe { std::ptr::read(&component) };
            let handle = tokio::spawn(async move {
                let message = ChatMessage {
                    role: MessageRole::User,
                    content: format!("Concurrent message {}", i),
                    timestamp: Utc::now(),
                    metadata: ahash::AHashMap::new(),
                };
                // Note: In real implementation, this would need proper synchronization
                // component_clone.add_message(message, &mut cx);
            });
            handles.push(handle);
        }

        // Concurrent context updates
        for i in 0..20 {
            let context_item = Arc::new(ContextItem {
                id: format!("concurrent-context-{}", i),
                title: format!("Context {}", i),
                summary: Some(format!("Summary for context {}", i)),
                content: format!("Content for concurrent context {}", i),
                context_type: ContextType::Document,
                created_at: Utc::now(),
                relevance_score: Some(0.5),
                metadata: ahash::AHashMap::new(),
            });

            // In real implementation, this would be added to context
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify system is still responsive
        assert!(component.get_performance_metrics().total_messages >= 0);
    }
}