#![cfg(feature = "legacy-components")]

use gpui::*;
use std::sync::Arc;
use terraphim_desktop_gpui::components::{
    ContextComponent, EnhancedChatComponent, EnhancedChatConfig, EnhancedChatEvent,
    EnhancedChatTheme, MessageRenderingConfig, SearchContextBridge, StreamingConfig,
};
use terraphim_types::{
    ChatMessage, ChunkType, ContextItem, ContextType, ConversationId, MessageRole, MessageStatus,
    RenderChunk, RoleName, StreamingChatMessage,
};
use tokio::time::{Duration, sleep};

mod app;

/// Comprehensive tests for the enhanced chat system
///
/// NOTE: This is part of the legacy reusable component test suite.
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
}
