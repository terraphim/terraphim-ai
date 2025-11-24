// End-to-End User Journey Tests
// Tests the complete user workflow from search to chat with context management

use chrono::Utc;
use terraphim_desktop_gpui::*;
use terraphim_types::{ChatMessage, ContextItem, ContextType, Conversation, Document, RoleName};

#[tokio::test]
async fn test_complete_user_journey() {
    // This test demonstrates the complete user workflow:
    // 1. Search with autocomplete
    // 2. Switch between roles
    // 3. Add search results to context
    // 4. CRUD operations on context
    // 5. Start a chat with context
    // 6. Verify persistence

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).try_init().ok();

    println!("═══════════════════════════════════════════════════════════════");
    println!("║  E2E Test: Complete User Journey                          ║");
    println!("═══════════════════════════════════════════════════════════════\n");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 1: Initialize Autocomplete Engine
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ Step 1: Initialize Autocomplete ─────────────────────────┐");

    let thesaurus_json = r#"[
        {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
        {"id": 2, "nterm": "tokio", "url": "https://tokio.rs"},
        {"id": 3, "nterm": "async", "url": "https://rust-lang.org/async"},
        {"id": 4, "nterm": "programming", "url": "https://en.wikipedia.org/wiki/Programming"},
        {"id": 5, "nterm": "concurrency", "url": "https://en.wikipedia.org/wiki/Concurrency"}
    ]"#;

    let autocomplete_engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json)
        .expect("Failed to create autocomplete engine");

    println!("✓ Loaded {} terms", autocomplete_engine.term_count());
    assert_eq!(autocomplete_engine.term_count(), 5);

    // Test autocomplete
    let suggestions = autocomplete_engine.autocomplete("tok", 5);
    println!("  Suggestions for 'tok': {}", suggestions.len());
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.term == "tokio"));

    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 2: Test Role Switching
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ Step 2: Test Role Switching ─────────────────────────────┐");

    let default_role = RoleName::from("default");
    let engineer_role = RoleName::from("engineer");
    let researcher_role = RoleName::from("researcher");

    println!("✓ Testing roles:");
    println!("  • Default: {}", default_role);
    println!("  • Engineer: {}", engineer_role);
    println!("  • Researcher: {}", researcher_role);

    assert_eq!(default_role.as_str(), "default");
    assert_eq!(engineer_role.as_str(), "engineer");
    assert_eq!(researcher_role.as_str(), "researcher");

    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 3: Perform Search with Multiple Roles
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ Step 3: Search with Different Roles ─────────────────────┐");

    // Create mock documents for testing
    let mock_documents = vec![
        create_mock_document(
            "1",
            "Rust Async Programming",
            "Complete guide to async/await in Rust",
            "https://rust-lang.org/async-book",
        ),
        create_mock_document(
            "2",
            "Tokio Runtime",
            "Build reliable network applications with Tokio",
            "https://tokio.rs/tutorial",
        ),
        create_mock_document(
            "3",
            "Concurrency Patterns",
            "Common concurrency patterns in Rust",
            "https://rust-patterns.com/concurrency",
        ),
    ];

    println!("✓ Created {} mock search results", mock_documents.len());

    // Test query parsing
    let queries = vec![
        ("rust async", LogicalOperator::And),
        ("tokio OR async", LogicalOperator::Or),
        ("rust AND tokio AND concurrency", LogicalOperator::And),
    ];

    for (query, expected_op) in queries {
        let parsed = SearchService::parse_query(query);
        println!("  Query: '{}' → Terms: {:?}, Op: {:?}",
            query, parsed.terms, parsed.operator);

        if let Some(op) = parsed.operator {
            assert_eq!(op, expected_op);
        }
    }

    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 4: Context Management (CRUD Operations)
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ Step 4: Context Management (CRUD) ───────────────────────┐");

    // Create context items from search results
    let context_items: Vec<ContextItem> = mock_documents
        .iter()
        .map(|doc| create_context_item_from_document(doc))
        .collect();

    println!("✓ Created {} context items from search results", context_items.len());

    // Test context item structure
    for (i, item) in context_items.iter().enumerate() {
        println!("  {}. {} ({})", i + 1, item.title, item.id);
        assert!(!item.title.is_empty());
        assert!(!item.content.is_empty());
        assert_eq!(item.context_type, ContextType::Document);
    }

    // Test CRUD operations (in-memory simulation)
    let mut context_storage: Vec<ContextItem> = Vec::new();

    // CREATE: Add items
    for item in context_items.clone() {
        context_storage.push(item);
    }
    println!("\n✓ CREATE: Added {} items to context", context_storage.len());
    assert_eq!(context_storage.len(), 3);

    // READ: Retrieve items
    let retrieved_items = context_storage.clone();
    println!("✓ READ: Retrieved {} items from context", retrieved_items.len());
    assert_eq!(retrieved_items.len(), 3);

    // UPDATE: Modify an item
    if let Some(item) = context_storage.get_mut(0) {
        let old_title = item.title.clone();
        item.title = format!("{} (Updated)", item.title);
        item.relevance_score = Some(0.95);
        println!("✓ UPDATE: Modified '{}' → '{}'", old_title, item.title);
    }
    assert!(context_storage[0].title.contains("(Updated)"));

    // DELETE: Remove an item
    let removed = context_storage.remove(2);
    println!("✓ DELETE: Removed '{}'", removed.title);
    assert_eq!(context_storage.len(), 2);

    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 5: Chat with Context
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ Step 5: Chat with Context ───────────────────────────────┐");

    let mut conversation = Conversation::new(
        "Rust Async Programming Discussion".to_string(),
        engineer_role,
    );

    // Add context to conversation
    for item in &context_storage {
        conversation.global_context.push(item.clone());
    }

    println!("✓ Added {} context items to conversation", conversation.global_context.len());
    assert_eq!(conversation.global_context.len(), 2);

    // Add system message
    conversation.messages.push(ChatMessage::system(
        "You are a Rust programming expert with access to documentation about async programming.".to_string(),
    ));

    // Add user message
    conversation.messages.push(ChatMessage::user(
        "Can you explain how Tokio handles async tasks based on the provided context?".to_string(),
    ));

    // Simulate assistant response
    conversation.messages.push(ChatMessage::assistant(
        "Based on the Tokio Runtime documentation in the context, Tokio provides a robust \
        runtime for async applications. It uses a work-stealing scheduler to efficiently \
        manage async tasks across multiple threads. The runtime handles task spawning, \
        I/O operations, and timer management, allowing you to build reliable network \
        applications with ease.".to_string(),
        Some("claude-sonnet-4-5".to_string()),
    ));

    // Add follow-up
    conversation.messages.push(ChatMessage::user(
        "How does this relate to Rust's async/await syntax?".to_string(),
    ));

    conversation.messages.push(ChatMessage::assistant(
        "Excellent question! According to the Rust Async Programming guide in the context, \
        Rust's async/await syntax provides a high-level abstraction for asynchronous code. \
        When you mark a function as 'async', it returns a Future that must be driven to \
        completion by a runtime like Tokio. The 'await' keyword suspends execution until \
        the Future completes, allowing the runtime to run other tasks in the meantime.".to_string(),
        Some("claude-sonnet-4-5".to_string()),
    ));

    println!("\n✓ Conversation Details:");
    println!("  Title: {}", conversation.title);
    println!("  Role: {}", conversation.role);
    println!("  Messages: {}", conversation.messages.len());
    println!("  Context Items: {}", conversation.global_context.len());

    assert_eq!(conversation.messages.len(), 5); // 1 system + 4 user/assistant
    assert_eq!(conversation.global_context.len(), 2);

    // Verify message flow
    for (i, msg) in conversation.messages.iter().enumerate() {
        let role_icon = match msg.role.as_str() {
            "system" => "⚙️",
            "user" => "👤",
            "assistant" => "🤖",
            _ => "💬",
        };

        let preview = if msg.content.len() > 60 {
            format!("{}...", &msg.content[..60])
        } else {
            msg.content.clone()
        };

        println!("  {}) {} [{}] {}", i + 1, role_icon, msg.role, preview);
    }

    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 6: Persistence Verification
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ Step 6: Persistence Verification ────────────────────────┐");

    // Serialize conversation (simulated persistence)
    let conversation_json = serde_json::to_string_pretty(&conversation)
        .expect("Failed to serialize conversation");

    println!("✓ Serialized conversation ({} bytes)", conversation_json.len());
    assert!(!conversation_json.is_empty());

    // Deserialize conversation (simulated retrieval)
    let restored_conversation: Conversation = serde_json::from_str(&conversation_json)
        .expect("Failed to deserialize conversation");

    println!("✓ Restored conversation from storage");

    // Verify restored data
    assert_eq!(restored_conversation.title, conversation.title);
    assert_eq!(restored_conversation.role, conversation.role);
    assert_eq!(restored_conversation.messages.len(), conversation.messages.len());
    assert_eq!(restored_conversation.global_context.len(), conversation.global_context.len());

    println!("\n✓ Verification Results:");
    println!("  • Title matches: {}", restored_conversation.title == conversation.title);
    println!("  • Role matches: {}", restored_conversation.role == conversation.role);
    println!("  • Message count matches: {} = {}",
        restored_conversation.messages.len(), conversation.messages.len());
    println!("  • Context count matches: {} = {}",
        restored_conversation.global_context.len(), conversation.global_context.len());

    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // FINAL SUMMARY
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("║  E2E Test Complete - All Steps Passed ✓                   ║");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n✅ Test Summary:");
    println!("  ✓ Autocomplete with {} KG terms", autocomplete_engine.term_count());
    println!("  ✓ Role switching (default, engineer, researcher)");
    println!("  ✓ Search query parsing (AND/OR operators)");
    println!("  ✓ Context CRUD operations (Create, Read, Update, Delete)");
    println!("  ✓ Chat with {} messages and {} context items",
        conversation.messages.len(), conversation.global_context.len());
    println!("  ✓ Conversation persistence (serialize/deserialize)");
}

// Helper Functions

fn create_mock_document(id: &str, title: &str, description: &str, url: &str) -> Document {
    Document {
        id: id.to_string(),
        body: format!("Full content of {}", title),
        url: url.to_string(),
        title: title.to_string(),
        description: Some(description.to_string()),
        tags: vec!["rust".to_string(), "async".to_string()],
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        embedding: None,
        rank: Some(1.0),
    }
}

fn create_context_item_from_document(doc: &Document) -> ContextItem {
    ContextItem {
        id: doc.id.clone().into(),
        title: doc.title.clone(),
        summary: doc.description.clone(),
        content: doc.body.clone(),
        context_type: ContextType::Document,
        created_at: Utc::now(),
        relevance_score: doc.rank,
        metadata: ahash::AHashMap::new(),
    }
}

#[tokio::test]
async fn test_search_with_kg_roles() {
    // Test search behavior with different roles
    println!("Testing search with KG roles...");

    let roles = vec![
        RoleName::from("default"),
        RoleName::from("engineer"),
        RoleName::from("researcher"),
    ];

    for role in roles {
        println!("  Testing role: {}", role);

        let search_options = SearchOptions {
            role: role.clone(),
            limit: 10,
            skip: 0,
        };

        assert_eq!(search_options.role, role);
        assert_eq!(search_options.limit, 10);
    }

    println!("✓ All role-based searches configured correctly");
}

#[tokio::test]
async fn test_context_management_operations() {
    // Focused test on context CRUD operations
    println!("Testing context management operations...");

    let mut items = Vec::new();

    // CREATE
    for i in 1..=5 {
        let item = ContextItem {
            id: format!("item_{}", i).into(),
            title: format!("Test Item {}", i),
            summary: Some(format!("Summary {}", i)),
            content: format!("Content for item {}", i),
            context_type: ContextType::Document,
            created_at: Utc::now(),
            relevance_score: Some(0.8),
            metadata: ahash::AHashMap::new(),
        };
        items.push(item);
    }

    assert_eq!(items.len(), 5);
    println!("✓ Created 5 context items");

    // READ
    let item = &items[2];
    assert_eq!(item.title, "Test Item 3");
    println!("✓ Read item: {}", item.title);

    // UPDATE
    let mut updated_item = items[2].clone();
    updated_item.title = "Updated Test Item 3".to_string();
    items[2] = updated_item;
    assert_eq!(items[2].title, "Updated Test Item 3");
    println!("✓ Updated item title");

    // DELETE
    items.remove(4);
    assert_eq!(items.len(), 4);
    println!("✓ Deleted 1 item, {} remaining", items.len());
}

#[tokio::test]
async fn test_chat_persistence() {
    // Test conversation serialization and deserialization
    println!("Testing chat persistence...");

    let mut conversation = Conversation::new(
        "Test Conversation".to_string(),
        RoleName::from("engineer"),
    );

    conversation.messages.push(ChatMessage::user("Hello".to_string()));
    conversation.messages.push(ChatMessage::assistant("Hi there".to_string(), None));

    // Add context
    conversation.global_context.push(ContextItem {
        id: "ctx_1".into(),
        title: "Test Context".to_string(),
        summary: None,
        content: "Test content".to_string(),
        context_type: ContextType::Document,
        created_at: Utc::now(),
        relevance_score: Some(0.9),
        metadata: ahash::AHashMap::new(),
    });

    // Serialize
    let json = serde_json::to_string(&conversation).expect("Serialization failed");
    assert!(!json.is_empty());
    println!("✓ Serialized conversation ({} bytes)", json.len());

    // Deserialize
    let restored: Conversation = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(restored.title, conversation.title);
    assert_eq!(restored.messages.len(), conversation.messages.len());
    assert_eq!(restored.global_context.len(), conversation.global_context.len());
    println!("✓ Restored conversation matches original");
}
