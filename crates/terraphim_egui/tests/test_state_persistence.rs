//! State persistence tests
//!
//! These tests verify basic state saving and loading.

use chrono::Utc;
use std::sync::{Arc, Mutex};
use terraphim_egui::state::{AppState, ChatMessage, ChatMessageRole};
use terraphim_types::Document;

/// Helper to create a test document
fn create_test_document(id: &str, title: &str) -> Document {
    Document {
        id: id.to_string(),
        url: format!("https://example.com/{}", id),
        title: title.to_string(),
        body: format!("Content of {}", title),
        description: Some(format!("Description for {}", title)),
        summarization: None,
        stub: None,
        tags: Some(vec!["test".to_string()]),
        rank: Some(100),
        source_haystack: Some("test".to_string()),
    }
}

/// Test that context can be modified
#[tokio::test]
async fn test_context_modification() {
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_arc);

    // Add document to context
    {
        let mut state = state_ref.lock().unwrap();
        state.add_document_to_context(create_test_document("doc1", "Document 1"));
    }

    // Verify document is in context
    let count = {
        let state = state_ref.lock().unwrap();
        let count = state.get_context_manager().selected_documents.len();
        count
    };

    assert_eq!(count, 1, "Should have 1 document in context");
}

/// Test that conversation can be modified
#[tokio::test]
async fn test_conversation_modification() {
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_arc);

    // Add a message
    let message = ChatMessage {
        id: uuid::Uuid::new_v4(),
        role: ChatMessageRole::User,
        content: "Test message".to_string(),
        timestamp: Utc::now(),
        metadata: None,
    };

    state_ref.lock().unwrap().add_chat_message(message);

    // Verify message is in history
    let count = {
        let state = state_ref.lock().unwrap();
        let count = state.get_conversation_history().len();
        count
    };

    assert_eq!(count, 1, "Should have 1 message in history");
}

/// Test context clearing
#[tokio::test]
async fn test_context_clearing() {
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_arc);

    // Add document
    {
        let mut state = state_ref.lock().unwrap();
        state.add_document_to_context(create_test_document("doc1", "Document 1"));
    }

    // Clear context
    state_ref.lock().unwrap().clear_context();

    // Verify context is empty
    let count = {
        let state = state_ref.lock().unwrap();
        let count = state.get_context_manager().selected_documents.len();
        count
    };

    assert_eq!(count, 0, "Context should be empty after clearing");
}
