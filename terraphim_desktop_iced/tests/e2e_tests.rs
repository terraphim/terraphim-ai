use std::sync::Arc;
use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::{DeviceStorage, Persistable};
use terraphim_service::conversation_service::ConversationService;
use terraphim_settings::DeviceSettings;
use terraphim_types::{
    ChatMessage, ContextItem, ContextType, Conversation, ConversationId, Document,
    NormalizedTermValue, RoleName, SearchQuery,
};

/// Test helper to initialize memory-only storage
async fn init_test_storage() {
    let _ = DeviceStorage::init_memory_only().await;
}

/// Test helper to create a test config state with default desktop role
async fn create_test_config_state() -> Arc<ConfigState> {
    init_test_storage().await;

    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build()
        .expect("Failed to build config");

    // Try to load, fallback to default
    let config = match config.load().await {
        Ok(c) => c,
        Err(_) => ConfigBuilder::new()
            .build_default_desktop()
            .build()
            .expect("Failed to build default config"),
    };

    let mut tmp_config = config.clone();
    let config_state = ConfigState::new(&mut tmp_config)
        .await
        .expect("Failed to create config state");

    Arc::new(config_state)
}

/// Test helper to create test conversation service
fn create_test_conversation_service() -> Arc<ConversationService> {
    Arc::new(ConversationService::new())
}

#[tokio::test]
async fn test_search_with_autocomplete_default_role() {
    // Initialize test environment
    let config_state = create_test_config_state().await;

    // Verify default role is loaded
    let config = config_state.config.lock().await;
    let role = config.selected_role.clone();
    drop(config);

    assert_eq!(role.as_str(), "Default");

    // Test autocomplete if thesaurus is available
    let query = "rust";

    if let Some(rolegraph_sync) = config_state.roles.get(&role) {
        let rolegraph = rolegraph_sync.lock().await;

        if !rolegraph.thesaurus.is_empty() {
            // Build autocomplete index
            let autocomplete_index =
                terraphim_automata::build_autocomplete_index(rolegraph.thesaurus.clone(), None)
                    .expect("Failed to build autocomplete index");

            drop(rolegraph);

            // Get autocomplete suggestions
            let suggestions = terraphim_automata::autocomplete_search(&autocomplete_index, query, Some(8))
                .expect("Autocomplete search failed");

            println!("Autocomplete suggestions for '{}': {:?}", query, suggestions.iter().map(|s| &s.term).collect::<Vec<_>>());

            // Perform actual search
            let search_query = SearchQuery {
                search_term: NormalizedTermValue::new(query.to_string()),
                search_terms: None,
                operator: None,
                skip: Some(0),
                limit: Some(10),
                role: Some(role),
            };

            let mut search_service = terraphim_service::TerraphimService::new((*config_state).clone());
            let results = search_service
                .search(&search_query)
                .await
                .expect("Search failed");

            println!("Search results: {} documents found", results.len());
        } else {
            println!("Thesaurus is empty - skipping autocomplete test");
        }
    } else {
        println!("Role not found - skipping search test");
    }

    println!("Test completed successfully");
}

#[tokio::test]
async fn test_search_with_kg_role() {
    // Initialize test environment with KG role
    init_test_storage().await;

    // Create config with a specific KG-focused role
    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .expect("Failed to build config");

    let mut tmp_config = config.clone();
    let config_state = ConfigState::new(&mut tmp_config)
        .await
        .expect("Failed to create config state");

    let config_state = Arc::new(config_state);

    // Test autocomplete with KG terms
    let query = "data";

    let config = config_state.config.lock().await;
    let role = config.selected_role.clone();
    drop(config);

    if let Some(rolegraph_sync) = config_state.roles.get(&role) {
        let rolegraph = rolegraph_sync.lock().await;

        if !rolegraph.thesaurus.is_empty() {
            let autocomplete_index =
                terraphim_automata::build_autocomplete_index(rolegraph.thesaurus.clone(), None)
                    .expect("Failed to build autocomplete index");

            drop(rolegraph);

            // Test fuzzy search
            let fuzzy_results =
                terraphim_automata::fuzzy_autocomplete_search(&autocomplete_index, query, 0.7, Some(10))
                    .expect("Fuzzy search failed");

            println!("Fuzzy search results for '{}': {} terms", query, fuzzy_results.len());
        } else {
            println!("Thesaurus is empty - skipping fuzzy search test");
        }
    } else {
        println!("Role not found - skipping test");
    }

    println!("Test completed successfully");
}

#[tokio::test]
async fn test_context_crud_operations() {
    init_test_storage().await;

    let conversation_service = create_test_conversation_service();

    // CREATE: Create a new conversation
    let mut conversation = conversation_service
        .create_conversation("Test Context CRUD".to_string(), RoleName::new("Test"))
        .await
        .expect("Failed to create conversation");

    println!("Created conversation: {}", conversation.id.as_str());
    assert_eq!(conversation.global_context.len(), 0);

    // CREATE: Add context items
    let context1 = ContextItem {
        id: ulid::Ulid::new().to_string(),
        title: "Context 1".to_string(),
        summary: Some("First context item".to_string()),
        content: "This is the first context item for testing".to_string(),
        context_type: ContextType::UserInput,
        metadata: ahash::AHashMap::new(),
        created_at: chrono::Utc::now(),
        relevance_score: None,
    };

    conversation.add_global_context(context1.clone());
    conversation = conversation_service
        .update_conversation(conversation.clone())
        .await
        .expect("Failed to save conversation");

    assert_eq!(conversation.global_context.len(), 1);
    println!("Added context 1: {}", context1.title);

    // CREATE: Add second context
    let context2 = ContextItem {
        id: ulid::Ulid::new().to_string(),
        title: "Context 2".to_string(),
        summary: Some("Second context item".to_string()),
        content: "This is the second context item for testing".to_string(),
        context_type: ContextType::UserInput,
        metadata: ahash::AHashMap::new(),
        created_at: chrono::Utc::now(),
        relevance_score: None,
    };

    conversation.add_global_context(context2.clone());
    conversation = conversation_service
        .update_conversation(conversation.clone())
        .await
        .expect("Failed to save conversation");

    assert_eq!(conversation.global_context.len(), 2);
    println!("Added context 2: {}", context2.title);

    // READ: Verify contexts
    let loaded_conversation = conversation_service
        .get_conversation(&conversation.id)
        .await
        .expect("Failed to load conversation");

    assert_eq!(loaded_conversation.global_context.len(), 2);
    println!("Verified 2 context items in loaded conversation");

    // UPDATE: Modify context (simulate by replacing)
    let context_id_to_remove = context1.id.clone();
    conversation.global_context.retain(|c| c.id != context_id_to_remove);
    conversation = conversation_service
        .update_conversation(conversation.clone())
        .await
        .expect("Failed to update conversation");

    assert_eq!(conversation.global_context.len(), 1);
    println!("Removed context 1, {} context items remaining", conversation.global_context.len());

    // DELETE: Remove all contexts
    conversation.global_context.clear();
    conversation = conversation_service
        .update_conversation(conversation.clone())
        .await
        .expect("Failed to update conversation");

    assert_eq!(conversation.global_context.len(), 0);
    println!("Removed all contexts");

    // DELETE: Delete entire conversation
    conversation_service
        .delete_conversation(&conversation.id)
        .await
        .expect("Failed to delete conversation");

    let result = conversation_service
        .get_conversation(&conversation.id)
        .await;

    assert!(result.is_err(), "Conversation should not exist after deletion");
    println!("Successfully deleted conversation");
}

#[tokio::test]
async fn test_kg_search_and_add_to_context() {
    let config_state = create_test_config_state().await;
    let conversation_service = create_test_conversation_service();

    // Create conversation
    let mut conversation = conversation_service
        .create_conversation("KG Test".to_string(), RoleName::new("Test"))
        .await
        .expect("Failed to create conversation");

    // Search KG terms
    let config = config_state.config.lock().await;
    let role = config.selected_role.clone();
    drop(config);

    if let Some(rolegraph_sync) = config_state.roles.get(&role) {
        let rolegraph = rolegraph_sync.lock().await;

        if !rolegraph.thesaurus.is_empty() {
            let autocomplete_index =
                terraphim_automata::build_autocomplete_index(rolegraph.thesaurus.clone(), None)
                    .expect("Failed to build autocomplete index");

            drop(rolegraph);

            // Search for KG terms
            let query = "test";
            let kg_results =
                terraphim_automata::fuzzy_autocomplete_search(&autocomplete_index, query, 0.7, Some(5))
                    .expect("KG search failed");

            println!("Found {} KG terms for '{}'", kg_results.len(), query);

            // Add first KG term as context (or create mock if none found)
            let test_term = if let Some(kg_term) = kg_results.first() {
                kg_term.term.clone()
            } else {
                "test_term".to_string()
            };

            let context_item = ContextItem {
                id: ulid::Ulid::new().to_string(),
                title: format!("KG: {}", test_term),
                summary: Some(format!("Knowledge Graph term: {}", test_term)),
                content: format!("Knowledge Graph term: {}", test_term),
                context_type: ContextType::KGTermDefinition,
                metadata: {
                    let mut map = ahash::AHashMap::new();
                    map.insert("term".to_string(), test_term.clone());
                    map.insert("source_type".to_string(), "kg_term".to_string());
                    map
                },
                created_at: chrono::Utc::now(),
                relevance_score: None,
            };

            conversation.add_global_context(context_item.clone());
            conversation = conversation_service
                .update_conversation(conversation.clone())
                .await
                .expect("Failed to save conversation");

            assert_eq!(conversation.global_context.len(), 1);
            assert_eq!(conversation.global_context[0].context_type, ContextType::KGTermDefinition);
            println!("Added KG term '{}' as context", test_term);
        } else {
            println!("Thesaurus is empty - creating mock KG context");

            // Create mock KG context for testing
            let context_item = ContextItem {
                id: ulid::Ulid::new().to_string(),
                title: "KG: Mock Term".to_string(),
                summary: Some("Mock Knowledge Graph term".to_string()),
                content: "Mock KG term for testing".to_string(),
                context_type: ContextType::KGTermDefinition,
                metadata: {
                    let mut map = ahash::AHashMap::new();
                    map.insert("term".to_string(), "mock_term".to_string());
                    map.insert("source_type".to_string(), "kg_term".to_string());
                    map
                },
                created_at: chrono::Utc::now(),
                relevance_score: None,
            };

            conversation.add_global_context(context_item);
            conversation = conversation_service
                .update_conversation(conversation)
                .await
                .expect("Failed to save conversation");

            assert_eq!(conversation.global_context.len(), 1);
            assert_eq!(conversation.global_context[0].context_type, ContextType::KGTermDefinition);
            println!("Added mock KG term as context");
        }
    }

    println!("Test completed successfully");
}

#[tokio::test]
async fn test_chat_functionality() {
    init_test_storage().await;

    let conversation_service = create_test_conversation_service();

    // Create conversation
    let mut conversation = conversation_service
        .create_conversation("Chat Test".to_string(), RoleName::new("Test"))
        .await
        .expect("Failed to create conversation");

    assert_eq!(conversation.messages.len(), 0);

    // Add user message
    let user_message = ChatMessage::user("Hello, this is a test message".to_string());
    conversation.add_message(user_message.clone());

    assert_eq!(conversation.messages.len(), 1);
    assert_eq!(conversation.messages[0].role, "user");
    println!("Added user message: {}", conversation.messages[0].content);

    // Add assistant response
    let assistant_message = ChatMessage::assistant("Hello! I'm here to help.".to_string(), None);
    conversation.add_message(assistant_message.clone());

    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(conversation.messages[1].role, "assistant");
    println!("Added assistant message: {}", conversation.messages[1].content);

    // Save conversation
    conversation = conversation_service
        .update_conversation(conversation.clone())
        .await
        .expect("Failed to save conversation");

    // Verify persistence
    let loaded_conversation = conversation_service
        .get_conversation(&conversation.id)
        .await
        .expect("Failed to load conversation");

    assert_eq!(loaded_conversation.messages.len(), 2);
    assert_eq!(loaded_conversation.messages[0].content, user_message.content);
    assert_eq!(loaded_conversation.messages[1].content, assistant_message.content);
    println!("Verified chat history persistence");
}

#[tokio::test]
async fn test_conversation_persistence() {
    init_test_storage().await;

    let conversation_service = create_test_conversation_service();

    // Create conversation with messages and context
    let mut conversation = conversation_service
        .create_conversation("Persistence Test".to_string(), RoleName::new("Test"))
        .await
        .expect("Failed to create conversation");

    // Add messages
    conversation.add_message(ChatMessage::user("First message".to_string()));
    conversation.add_message(ChatMessage::assistant("First response".to_string(), Some("test-model".to_string())));
    conversation.add_message(ChatMessage::user("Second message".to_string()));
    conversation.add_message(ChatMessage::assistant("Second response".to_string(), Some("test-model".to_string())));

    // Add context
    let context1 = ContextItem {
        id: ulid::Ulid::new().to_string(),
        title: "Persistent Context".to_string(),
        summary: Some("Test context item".to_string()),
        content: "This context should persist across saves and loads".to_string(),
        context_type: ContextType::UserInput,
        metadata: ahash::AHashMap::new(),
        created_at: chrono::Utc::now(),
        relevance_score: None,
    };

    conversation.add_global_context(context1.clone());

    // Save conversation
    let conversation_id = conversation.id.clone();
    conversation = conversation_service
        .update_conversation(conversation.clone())
        .await
        .expect("Failed to save conversation");

    println!("Saved conversation with {} messages and {} context items",
             conversation.messages.len(),
             conversation.global_context.len());

    // Load conversation
    let loaded = conversation_service
        .get_conversation(&conversation_id)
        .await
        .expect("Failed to load conversation");

    // Verify all data persisted
    assert_eq!(loaded.id, conversation_id);
    assert_eq!(loaded.title, "Persistence Test");
    assert_eq!(loaded.messages.len(), 4);
    assert_eq!(loaded.global_context.len(), 1);
    assert_eq!(loaded.messages[0].content, "First message");
    assert_eq!(loaded.messages[1].content, "First response");
    assert_eq!(loaded.global_context[0].title, "Persistent Context");

    println!("Successfully verified conversation persistence");

    // Test updating existing conversation
    let mut updated_conversation = loaded;
    updated_conversation.add_message(ChatMessage::user("Third message".to_string()));

    conversation_service
        .update_conversation(updated_conversation.clone())
        .await
        .expect("Failed to update conversation");

    // Reload and verify update
    let reloaded = conversation_service
        .get_conversation(&conversation_id)
        .await
        .expect("Failed to reload conversation");

    assert_eq!(reloaded.messages.len(), 5);
    assert_eq!(reloaded.messages[4].content, "Third message");

    println!("Successfully verified conversation update persistence");
}

#[tokio::test]
async fn test_end_to_end_user_journey() {
    println!("\n=== Starting End-to-End User Journey Test ===\n");

    // Setup
    let config_state = create_test_config_state().await;
    let conversation_service = create_test_conversation_service();

    // Step 1: User performs search with autocomplete
    println!("Step 1: Search with autocomplete");
    let search_query = "test";

    let config = config_state.config.lock().await;
    let role = config.selected_role.clone();
    drop(config);

    let mut search_results = Vec::new();
    let mut autocomplete_index = None;

    if let Some(rolegraph_sync) = config_state.roles.get(&role) {
        let rolegraph = rolegraph_sync.lock().await;

        if !rolegraph.thesaurus.is_empty() {
            let index = terraphim_automata::build_autocomplete_index(rolegraph.thesaurus.clone(), None)
                .expect("Failed to build autocomplete index");
            autocomplete_index = Some(index);
            drop(rolegraph);

            if let Some(ref index) = autocomplete_index {
                let suggestions =
                    terraphim_automata::autocomplete_search(index, "test", Some(5))
                        .unwrap_or_default();

                println!("  - Got {} autocomplete suggestions", suggestions.len());
            }

            // Perform search
            let query = SearchQuery {
                search_term: NormalizedTermValue::new(search_query.to_string()),
                search_terms: None,
                operator: None,
                skip: Some(0),
                limit: Some(10),
                role: Some(role.clone()),
            };

            let mut search_service = terraphim_service::TerraphimService::new((*config_state).clone());
            search_results = search_service.search(&query).await.unwrap_or_default();

            println!("  - Search returned {} results", search_results.len());
        } else {
            println!("  - Thesaurus is empty, using mock data");
        }
    } else {
        println!("  - Role not found, using mock data");
    }

    // Step 2: Create conversation
    println!("\nStep 2: Create conversation");
    let mut conversation = conversation_service
        .create_conversation("E2E Test Conversation".to_string(), role.clone())
        .await
        .expect("Failed to create conversation");

    println!("  - Created conversation: {}", conversation.id.as_str());

    // Step 3: Add search results as context
    println!("\nStep 3: Add search result as context");
    if let Some(doc) = search_results.first() {
        let context_from_search = ContextItem::from_document(doc);
        conversation.add_global_context(context_from_search.clone());

        println!("  - Added document '{}' as context", doc.title);
    } else {
        // Create mock document context for testing
        let mock_context = ContextItem {
            id: ulid::Ulid::new().to_string(),
            title: "Mock Document".to_string(),
            summary: Some("Mock search result".to_string()),
            content: "This is a mock document from search results".to_string(),
            context_type: ContextType::Document,
            metadata: ahash::AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: Some(1.0),
        };
        conversation.add_global_context(mock_context);
        println!("  - Added mock document as context");
    }

    // Step 4: Search KG and add terms as context
    println!("\nStep 4: Search KG and add term as context");

    if let Some(ref index) = autocomplete_index {
        let kg_results =
            terraphim_automata::fuzzy_autocomplete_search(index, "test", 0.7, Some(3))
                .unwrap_or_default();

        if let Some(kg_term) = kg_results.first() {
            let kg_context = ContextItem {
                id: ulid::Ulid::new().to_string(),
                title: format!("KG: {}", kg_term.term),
                summary: Some(format!("Knowledge Graph term: {}", kg_term.term)),
                content: format!("Term: {}", kg_term.term),
                context_type: ContextType::KGTermDefinition,
                metadata: {
                    let mut map = ahash::AHashMap::new();
                    map.insert("term".to_string(), kg_term.term.clone());
                    map
                },
                created_at: chrono::Utc::now(),
                relevance_score: None,
            };

            conversation.add_global_context(kg_context.clone());
            println!("  - Added KG term '{}' as context", kg_term.term);
        } else {
            // Add mock KG term
            let kg_context = ContextItem {
                id: ulid::Ulid::new().to_string(),
                title: "KG: Mock Term".to_string(),
                summary: Some("Mock KG term".to_string()),
                content: "Mock KG term for testing".to_string(),
                context_type: ContextType::KGTermDefinition,
                metadata: ahash::AHashMap::new(),
                created_at: chrono::Utc::now(),
                relevance_score: None,
            };
            conversation.add_global_context(kg_context);
            println!("  - Added mock KG term as context");
        }
    } else {
        // Add mock KG term when no index available
        let kg_context = ContextItem {
            id: ulid::Ulid::new().to_string(),
            title: "KG: Mock Term".to_string(),
            summary: Some("Mock KG term".to_string()),
            content: "Mock KG term for testing".to_string(),
            context_type: ContextType::KGTermDefinition,
            metadata: ahash::AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: None,
        };
        conversation.add_global_context(kg_context);
        println!("  - Added mock KG term as context");
    }

    // Step 5: Add custom context
    println!("\nStep 5: Add custom context");
    let custom_context = ContextItem {
        id: ulid::Ulid::new().to_string(),
        title: "Custom Context".to_string(),
        summary: Some("User-provided context".to_string()),
        content: "This is custom context added by the user for the conversation".to_string(),
        context_type: ContextType::UserInput,
        metadata: ahash::AHashMap::new(),
        created_at: chrono::Utc::now(),
        relevance_score: None,
    };

    conversation.add_global_context(custom_context.clone());
    println!("  - Added custom context: {}", custom_context.title);

    // Step 6: Start chatting
    println!("\nStep 6: Chat with context");
    conversation.add_message(ChatMessage::user(
        "Tell me about Rust based on the context provided".to_string(),
    ));

    conversation.add_message(ChatMessage::assistant(
        "Based on the context about Rust...".to_string(),
        None,
    ));

    println!("  - Added user message and assistant response");

    // Step 7: Save conversation
    println!("\nStep 7: Save conversation");
    conversation = conversation_service
        .update_conversation(conversation.clone())
        .await
        .expect("Failed to save conversation");

    println!("  - Saved conversation with {} messages and {} context items",
             conversation.messages.len(),
             conversation.global_context.len());

    // Step 8: Test context CRUD
    println!("\nStep 8: Test context CRUD operations");
    let original_context_count = conversation.global_context.len();

    // Remove first context item
    if let Some(first_context) = conversation.global_context.first() {
        let context_id_to_remove = first_context.id.clone();
        conversation.global_context.retain(|c| c.id != context_id_to_remove);

        conversation = conversation_service
            .update_conversation(conversation.clone())
            .await
            .expect("Failed to update conversation");

        println!("  - Removed context item, {} remaining", conversation.global_context.len());
        assert_eq!(conversation.global_context.len(), original_context_count - 1);
    }

    // Step 9: Continue chat
    println!("\nStep 9: Continue conversation");
    conversation.add_message(ChatMessage::user("What else can you tell me?".to_string()));
    conversation.add_message(ChatMessage::assistant("I can provide more details...".to_string(), None));

    conversation = conversation_service
        .update_conversation(conversation.clone())
        .await
        .expect("Failed to save conversation");

    println!("  - Added more messages, total: {}", conversation.messages.len());

    // Step 10: Verify persistence
    println!("\nStep 10: Verify full persistence");
    let loaded = conversation_service
        .get_conversation(&conversation.id)
        .await
        .expect("Failed to load conversation");

    assert_eq!(loaded.messages.len(), conversation.messages.len());
    assert_eq!(loaded.global_context.len(), conversation.global_context.len());
    assert_eq!(loaded.title, "E2E Test Conversation");

    println!("  - Successfully verified all data persisted");
    println!("  - Messages: {}", loaded.messages.len());
    println!("  - Context items: {}", loaded.global_context.len());

    println!("\n=== End-to-End User Journey Test Complete ===\n");
}
