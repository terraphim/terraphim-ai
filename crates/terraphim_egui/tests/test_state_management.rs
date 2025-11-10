//! End-to-end tests for state management
//!
//! These tests verify thread-safe state management, context operations,
//! and conversation history.

use chrono::Utc;
use std::sync::{Arc, Mutex};
use terraphim_egui::state::{
    ActiveTab, AppSettings, AppState, ChatMessage, ChatMessageRole, ContextManager,
    PanelVisibility, Theme, UIState,
};
use terraphim_types::Document;

/// Test AppState initialization
#[tokio::test]
async fn test_app_state_initialization() {
    let state = AppState::new();

    // Verify initial state
    assert!(!state.get_current_role().name.lowercase.is_empty());

    // Search results should be empty
    let results = state.get_search_results();
    assert!(results.is_empty());

    // Context should be empty
    let context = state.get_context_manager();
    assert!(context.selected_documents.is_empty());
    assert_eq!(context.selected_concepts.len(), 0);
    assert_eq!(context.selected_kg_nodes.len(), 0);

    // Conversation history should be empty
    let history = state.get_conversation_history();
    assert!(history.is_empty());

    // UI state should have defaults
    let ui_state = state.get_ui_state();
    assert_eq!(ui_state.active_tab, ActiveTab::Search);
    assert!(ui_state.panel_visibility.search_panel);
}

/// Test adding and removing documents from context
#[tokio::test]
async fn test_document_context_management() {
    let state = AppState::new();

    // Create a test document
    let doc = Document {
        id: "test-doc-1".to_string(),
        url: "https://example.com".to_string(),
        title: "Test Document".to_string(),
        body: "Test content".to_string(),
        description: Some("Test description".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["test".to_string()]),
        rank: Some(100),
        source_haystack: Some("test".to_string()),
    };

    // Add document to context
    state.add_document_to_context(doc.clone());

    // Verify it's in context
    let context = state.get_context_manager();
    assert_eq!(context.selected_documents.len(), 1);
    assert_eq!(context.selected_documents[0].id, "test-doc-1");

    // Try to add the same document again (should not duplicate)
    state.add_document_to_context(doc.clone());

    let context = state.get_context_manager();
    assert_eq!(context.selected_documents.len(), 1); // Still 1, not 2

    // Remove document
    state.remove_document_from_context("test-doc-1");

    let context = state.get_context_manager();
    assert_eq!(context.selected_documents.len(), 0);
}

/// Test clearing context
#[tokio::test]
async fn test_clear_context() {
    let state = AppState::new();

    // Add multiple documents
    for i in 1..=5 {
        let doc = Document {
            id: format!("test-doc-{}", i),
            url: format!("https://example.com/{}", i),
            title: format!("Test Document {}", i),
            body: format!("Test content {}", i),
            description: Some(format!("Test description {}", i)),
            summarization: None,
            stub: None,
            tags: Some(vec!["test".to_string()]),
            rank: Some(100),
            source_haystack: Some("test".to_string()),
        };
        state.add_document_to_context(doc);
    }

    // Verify documents are in context
    let context = state.get_context_manager();
    assert_eq!(context.selected_documents.len(), 5);

    // Clear context
    state.clear_context();

    // Verify context is empty
    let context = state.get_context_manager();
    assert_eq!(context.selected_documents.len(), 0);
    assert_eq!(context.selected_concepts.len(), 0);
    assert_eq!(context.selected_kg_nodes.len(), 0);
}

/// Test context size tracking
#[tokio::test]
async fn test_context_size_tracking() {
    let state = AppState::new();

    // Add documents with varying sizes
    let large_doc = Document {
        id: "large-doc".to_string(),
        url: "https://example.com/large".to_string(),
        title: "Large Document".to_string(),
        body: "This is a very large document with lots of content. ".repeat(100),
        description: Some("Large document description".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["large".to_string()]),
        rank: Some(100),
        source_haystack: Some("test".to_string()),
    };

    state.add_document_to_context(large_doc);

    let context = state.get_context_manager();
    assert!(context.current_context_size > 0);

    // Clear and verify size is reset
    state.clear_context();
    let context = state.get_context_manager();
    assert_eq!(context.current_context_size, 0);
}

/// Test conversation history
#[tokio::test]
async fn test_conversation_history() {
    let state = AppState::new();

    // Create messages
    let user_msg = ChatMessage {
        id: uuid::Uuid::new_v4(),
        role: ChatMessageRole::User,
        content: "Hello, world!".to_string(),
        timestamp: Utc::now(),
        metadata: None,
    };

    let assistant_msg = ChatMessage {
        id: uuid::Uuid::new_v4(),
        role: ChatMessageRole::Assistant,
        content: "Hello! How can I help you?".to_string(),
        timestamp: Utc::now(),
        metadata: None,
    };

    // Add messages
    state.add_chat_message(user_msg.clone());
    state.add_chat_message(assistant_msg.clone());

    // Verify history
    let history = state.get_conversation_history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "Hello, world!");
    assert_eq!(history[1].content, "Hello! How can I help you?");

    // Clear conversation
    state.clear_conversation();

    let history = state.get_conversation_history();
    assert!(history.is_empty());
}

/// Test search results management
#[tokio::test]
async fn test_search_results_management() {
    let state = AppState::new();

    // Set search results
    let results = create_test_documents();
    state.set_search_results(results.clone());

    // Verify results are set
    let saved_results = state.get_search_results();
    assert_eq!(saved_results.len(), 3);
    assert_eq!(saved_results[0].id, "1");
    assert_eq!(saved_results[1].id, "2");
    assert_eq!(saved_results[2].id, "3");

    // Update results
    let new_results = vec![create_test_document("4", "New Document")];
    state.set_search_results(new_results);

    let updated_results = state.get_search_results();
    assert_eq!(updated_results.len(), 1);
    assert_eq!(updated_results[0].id, "4");
}

/// Test UI state management
#[tokio::test]
async fn test_ui_state_management() {
    let state = AppState::new();

    // Update UI state
    state.update_ui_state(|ui_state| {
        ui_state.active_tab = ActiveTab::Chat;
        ui_state.theme = Theme::Dark;
    });

    // Verify changes
    let ui_state = state.get_ui_state();
    assert_eq!(ui_state.active_tab, ActiveTab::Chat);
    assert_eq!(ui_state.theme, Theme::Dark);

    // Update panel visibility
    state.update_ui_state(|ui_state| {
        ui_state.panel_visibility.search_panel = false;
    });

    let ui_state = state.get_ui_state();
    assert!(!ui_state.panel_visibility.search_panel);
}

/// Test concurrent state access
#[tokio::test]
async fn test_concurrent_state_access() {
    let state = Arc::new(Mutex::new(AppState::new()));

    // Spawn multiple tasks to access state concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let state_clone = Arc::clone(&state);
            tokio::spawn(async move {
                // Read operations
                {
                    let state = state_clone.lock().unwrap();
                    let _results = state.get_search_results();
                }
                {
                    let state = state_clone.lock().unwrap();
                    let _context = state.get_context_manager();
                }

                // Write operation
                let doc = create_test_document(&format!("concurrent-{}", i), &format!("Doc {}", i));
                {
                    let state = state_clone.lock().unwrap();
                    state.add_document_to_context(doc);
                }

                i
            })
        })
        .collect();

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify final state
    let state_guard = state.lock().unwrap();
    let context = state_guard.get_context_manager();
    assert_eq!(context.selected_documents.len(), 10);
}

/// Test role management
#[tokio::test]
async fn test_role_management() {
    let state = AppState::new();

    // Get current role name
    let initial_role = state.get_current_role();
    let initial_name = initial_role.name.clone();

    // Set new role
    let new_role = terraphim_config::Role::new("TestRole");
    state.set_current_role(new_role.clone());

    // Verify role changed
    let current_role = state.get_current_role();
    assert_eq!(current_role.name, "TestRole".into());
    assert_ne!(current_role.name, initial_name);
}

/// Test max context size
#[tokio::test]
async fn test_max_context_size() {
    let mut context = ContextManager::default();

    // Set max context size
    context.max_context_size = 1000;

    assert_eq!(context.max_context_size, 1000);

    // Add documents
    for i in 1..=10 {
        let doc = create_test_document(&format!("{}", i), &format!("Document {}", i));
        context.selected_documents.push(doc);
    }

    // Estimate context size
    let estimated_size: usize = context
        .selected_documents
        .iter()
        .map(|doc| doc.body.len() + doc.title.len())
        .sum();

    context.current_context_size = estimated_size;

    assert!(context.current_context_size > 0);
}

/// Test AppSettings defaults
#[tokio::test]
async fn test_app_settings_defaults() {
    let settings = AppSettings::default();

    assert!(settings.show_autocomplete);
    assert_eq!(settings.autocomplete_debounce_ms, 50);
    assert_eq!(settings.max_autocomplete_results, 5);
    assert_eq!(settings.llm_provider, "ollama");
    assert_eq!(settings.llm_model, "llama3.2:3b");
    assert!(settings.auto_save_conversations);
    assert!(settings.enable_shortcuts);
}

/// Test PanelVisibility defaults
#[tokio::test]
async fn test_panel_visibility_defaults() {
    let visibility = PanelVisibility::default();

    assert!(visibility.search_panel);
    assert!(visibility.chat_panel);
    assert!(visibility.context_panel);
    assert!(visibility.knowledge_graph_panel);
    assert!(visibility.configuration_panel);
    assert!(!visibility.sessions_panel); // Default to false
}

/// Helper functions
fn create_test_documents() -> Vec<Document> {
    vec![
        create_test_document("1", "Document 1"),
        create_test_document("2", "Document 2"),
        create_test_document("3", "Document 3"),
    ]
}

fn create_test_document(id: &str, title: &str) -> Document {
    Document {
        id: id.to_string(),
        url: format!("https://example.com/{}", id),
        title: title.to_string(),
        body: format!("Content of {}", title),
        description: Some(format!("Description of {}", title)),
        summarization: None,
        stub: None,
        tags: Some(vec!["test".to_string()]),
        rank: Some(100),
        source_haystack: Some("test".to_string()),
    }
}
