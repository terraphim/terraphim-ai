#![cfg(feature = "ollama")]

use ahash::AHashMap;
use serial_test::serial;
use std::sync::Arc;
use terraphim_config::{ConfigState, Role};
use terraphim_service::{context::ContextManager, llm};
use terraphim_types::{ContextItem, ContextType, ConversationId, RoleName};

/// Integration tests for context management with Ollama LLM integration
/// Tests context operations with AI-powered summarization and content processing
#[tokio::test]
#[serial]
async fn ollama_context_management_integration() {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    // Test 1: Context creation with AI summarization
    test_context_creation_with_ai_summarization(&base_url).await;

    // Test 2: Context update operations
    test_context_update_operations(&base_url).await;

    // Test 3: Context deletion and management
    test_context_deletion_and_management(&base_url).await;

    // Test 4: Bulk context operations with AI processing
    test_bulk_context_operations(&base_url).await;

    // Test 5: AI-powered context content analysis
    test_ai_powered_context_analysis(&base_url).await;
}

/// Test 1: Context creation with AI-powered summarization
async fn test_context_creation_with_ai_summarization(base_url: &str) {
    println!("🧪 Testing context creation with AI summarization");

    // Test Ollama connectivity first
    test_ollama_connectivity(base_url).await;

    // Create role with Ollama configuration
    let mut role = create_test_role_with_ollama(base_url, "ContextCreationTest");

    // Create context manager
    let mut context_manager = ContextManager::new();
    let conversation_id = ConversationId::new("test-conv-ai-summary");

    // Create LLM client for AI processing
    let llm_client = llm::build_llm_from_role(&role).expect("Failed to build LLM client");

    // Test content that would benefit from summarization
    let test_content = r#"
    Context management is a critical aspect of modern AI systems. It involves storing, organizing, 
    and retrieving contextual information that helps AI models understand and respond appropriately 
    to user queries. Effective context management enables AI systems to maintain coherent conversations, 
    remember important details across interactions, and provide more personalized and relevant responses.

    Key components of context management include:
    1. Context storage mechanisms for persisting information across sessions
    2. Context retrieval systems that can quickly access relevant information
    3. Context ranking algorithms that prioritize the most important information
    4. Context updating mechanisms that allow for modification and refinement
    5. Context deletion capabilities for managing storage and privacy

    In the Terraphim AI system, context management is implemented through a sophisticated 
    architecture that supports multiple context types including documents, search results, 
    user inputs, and system-generated content. The system uses advanced indexing and retrieval 
    mechanisms to ensure fast access to contextual information.
    "#;

    // Generate AI summary using Ollama
    let ai_summary = llm_client
        .summarize(test_content, llm::SummarizeOptions { max_length: 150 })
        .await
        .expect("AI summarization should succeed");

    // Create context item with AI-generated summary
    let context_item = ContextItem {
        id: "ctx-ai-summary-test".to_string(),
        context_type: ContextType::Document,
        title: "AI Context Management Guide".to_string(),
        summary: Some(ai_summary.clone()),
        content: test_content.to_string(),
        metadata: {
            let mut metadata = AHashMap::new();
            metadata.insert("ai_generated_summary".to_string(), "true".to_string());
            metadata.insert("llm_model".to_string(), "llama3.2:3b".to_string());
            metadata
        },
        created_at: chrono::Utc::now(),
        relevance_score: Some(0.95),
    };

    // Add context to conversation
    context_manager
        .add_context_to_conversation(&conversation_id, context_item.clone())
        .expect("Adding context should succeed");

    // Retrieve and verify context
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Conversation should exist")
        .expect("Conversation should be found");

    assert_eq!(conversation.global_context.len(), 1);
    let retrieved_context = &conversation.global_context[0];

    assert_eq!(retrieved_context.title, "AI Context Management Guide");
    assert!(retrieved_context.summary.is_some());
    assert!(!ai_summary.trim().is_empty());
    assert!(retrieved_context
        .metadata
        .contains_key("ai_generated_summary"));

    println!("✅ Context creation with AI summarization test passed");
    println!("📝 Generated summary: {}", ai_summary);
}

/// Test 2: Context update operations with AI enhancement
async fn test_context_update_operations(base_url: &str) {
    println!("🧪 Testing context update operations with AI enhancement");

    let mut context_manager = ContextManager::new();
    let conversation_id = ConversationId::new("test-conv-updates");

    // Create initial context
    let initial_context = ContextItem {
        id: "ctx-update-test".to_string(),
        context_type: ContextType::UserInput,
        title: "Initial Context Item".to_string(),
        summary: Some("Initial summary".to_string()),
        content: "Initial content for update testing".to_string(),
        metadata: AHashMap::new(),
        created_at: chrono::Utc::now(),
        relevance_score: None,
    };

    context_manager
        .add_context_to_conversation(&conversation_id, initial_context)
        .expect("Adding initial context should succeed");

    // Create LLM client for AI-enhanced updates
    let role = create_test_role_with_ollama(base_url, "ContextUpdateTest");
    let llm_client = llm::build_llm_from_role(&role).expect("Failed to build LLM client");

    // Updated content that needs new summarization
    let updated_content = r#"
    Updated context management now includes advanced features such as:
    - Real-time context synchronization across distributed systems
    - Semantic understanding of context relationships
    - Automated context relevance scoring using machine learning
    - Dynamic context pruning to maintain optimal performance
    - Integration with vector databases for similarity-based retrieval
    - Support for multi-modal context including text, images, and structured data
    
    These enhancements significantly improve the AI system's ability to maintain 
    coherent and contextually appropriate conversations while scaling to handle 
    large volumes of contextual information efficiently.
    "#;

    // Generate new AI summary for updated content
    let updated_summary = llm_client
        .summarize(updated_content, llm::SummarizeOptions { max_length: 120 })
        .await
        .expect("AI summarization for update should succeed");

    // Create updated context item
    let updated_context = ContextItem {
        id: "ctx-update-test".to_string(),
        context_type: ContextType::Document,
        title: "Enhanced Context Management System".to_string(),
        summary: Some(updated_summary.clone()),
        content: updated_content.to_string(),
        metadata: {
            let mut metadata = AHashMap::new();
            metadata.insert("updated_with_ai".to_string(), "true".to_string());
            metadata.insert(
                "update_timestamp".to_string(),
                chrono::Utc::now().to_rfc3339(),
            );
            metadata
        },
        created_at: chrono::Utc::now(),
        relevance_score: Some(0.98),
    };

    // Update the context
    let updated_result = context_manager
        .update_context(&conversation_id, "ctx-update-test", updated_context)
        .expect("Context update should succeed");

    // Verify updates
    assert_eq!(updated_result.title, "Enhanced Context Management System");
    assert_eq!(updated_result.context_type, ContextType::Document);
    assert!(updated_result.summary.is_some());
    assert_eq!(updated_result.summary.as_ref().unwrap(), &updated_summary);
    assert!(updated_result.metadata.contains_key("updated_with_ai"));

    println!("✅ Context update operations test passed");
    println!("📝 Updated summary: {}", updated_summary);
}

/// Test 3: Context deletion and management
async fn test_context_deletion_and_management(base_url: &str) {
    println!("🧪 Testing context deletion and management operations");

    let mut context_manager = ContextManager::new();
    let conversation_id = ConversationId::new("test-conv-deletion");

    // Add multiple context items
    let context_items = vec![
        create_test_context_item("ctx-del-1", "First Context", "Content for first context"),
        create_test_context_item("ctx-del-2", "Second Context", "Content for second context"),
        create_test_context_item("ctx-del-3", "Third Context", "Content for third context"),
    ];

    for item in context_items {
        context_manager
            .add_context_to_conversation(&conversation_id, item)
            .expect("Adding context should succeed");
    }

    // Verify all contexts were added
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Conversation should exist")
        .expect("Conversation should be found");
    assert_eq!(conversation.global_context.len(), 3);

    // Test deletion of middle context
    context_manager
        .delete_context(&conversation_id, "ctx-del-2")
        .expect("Context deletion should succeed");

    // Verify deletion
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Conversation should exist")
        .expect("Conversation should be found");
    assert_eq!(conversation.global_context.len(), 2);

    let remaining_ids: Vec<&str> = conversation
        .global_context
        .iter()
        .map(|ctx| ctx.id.as_str())
        .collect();
    assert!(remaining_ids.contains(&"ctx-del-1"));
    assert!(!remaining_ids.contains(&"ctx-del-2"));
    assert!(remaining_ids.contains(&"ctx-del-3"));

    // Test deletion of non-existent context
    let delete_result = context_manager.delete_context(&conversation_id, "non-existent");
    assert!(
        delete_result.is_err(),
        "Deleting non-existent context should fail"
    );

    println!("✅ Context deletion and management test passed");
}

/// Test 4: Bulk context operations with AI processing
async fn test_bulk_context_operations(base_url: &str) {
    println!("🧪 Testing bulk context operations with AI processing");

    let mut context_manager = ContextManager::new();
    let conversation_id = ConversationId::new("test-conv-bulk");
    let role = create_test_role_with_ollama(base_url, "BulkContextTest");
    let llm_client = llm::build_llm_from_role(&role).expect("Failed to build LLM client");

    // Test content for bulk processing
    let test_contents = vec![
        "Rust ownership system prevents memory leaks and ensures thread safety through compile-time checks.",
        "WebAssembly enables running Rust code in web browsers with near-native performance.",
        "Async programming in Rust uses futures and the tokio runtime for concurrent execution.",
        "Cargo is Rust's package manager and build system that handles dependencies automatically.",
        "Rust's type system prevents null pointer dereferences and data races at compile time.",
    ];

    // Process contexts with AI-generated summaries
    for (i, content) in test_contents.iter().enumerate() {
        // Generate AI summary
        let summary = llm_client
            .summarize(content, llm::SummarizeOptions { max_length: 80 })
            .await
            .expect("AI summarization should succeed");

        let context_item = ContextItem {
            id: format!("ctx-bulk-{}", i),
            context_type: ContextType::Document,
            title: format!("Rust Concept {}", i + 1),
            summary: Some(summary),
            content: content.to_string(),
            metadata: {
                let mut metadata = AHashMap::new();
                metadata.insert("batch_id".to_string(), "bulk-test-1".to_string());
                metadata.insert("ai_processed".to_string(), "true".to_string());
                metadata
            },
            created_at: chrono::Utc::now(),
            relevance_score: Some(0.8 + (i as f64 * 0.02)),
        };

        context_manager
            .add_context_to_conversation(&conversation_id, context_item)
            .expect("Adding bulk context should succeed");
    }

    // Verify all contexts were added
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Conversation should exist")
        .expect("Conversation should be found");
    assert_eq!(conversation.global_context.len(), test_contents.len());

    // Verify all contexts have AI-generated summaries
    for context in &conversation.global_context {
        assert!(context.summary.is_some());
        assert!(!context.summary.as_ref().unwrap().trim().is_empty());
        assert_eq!(
            context.metadata.get("ai_processed"),
            Some(&"true".to_string())
        );
        assert_eq!(
            context.metadata.get("batch_id"),
            Some(&"bulk-test-1".to_string())
        );
    }

    println!("✅ Bulk context operations with AI processing test passed");
    println!(
        "📊 Processed {} contexts with AI summaries",
        test_contents.len()
    );
}

/// Test 5: AI-powered context content analysis
async fn test_ai_powered_context_analysis(base_url: &str) {
    println!("🧪 Testing AI-powered context content analysis");

    let role = create_test_role_with_ollama(base_url, "ContextAnalysisTest");
    let llm_client = llm::build_llm_from_role(&role).expect("Failed to build LLM client");

    // Test different types of content for analysis
    let test_cases = vec![
        (
            "Technical documentation",
            r#"
            Rust's borrow checker ensures memory safety by enforcing ownership rules at compile time.
            The system prevents data races and null pointer dereferences without runtime overhead.
            Key concepts include ownership, borrowing, and lifetimes that work together to guarantee safety.
            "#,
        ),
        (
            "User conversation",
            r#"
            I'm having trouble understanding how to implement async/await in Rust.
            Could you help me understand the difference between async fn and regular functions?
            Also, what is the tokio runtime and when should I use it?
            "#,
        ),
        (
            "Code example",
            r#"
            ```rust
            async fn fetch_data(url: &str) -> Result<String, reqwest::Error> {
                let response = reqwest::get(url).await?;
                let text = response.text().await?;
                Ok(text)
            }
            ```
            "#,
        ),
    ];

    for (content_type, content) in test_cases {
        // Generate AI summary
        let summary = llm_client
            .summarize(content, llm::SummarizeOptions { max_length: 100 })
            .await
            .expect("AI analysis should succeed");

        // Verify summary characteristics
        assert!(
            !summary.trim().is_empty(),
            "Summary should not be empty for {}",
            content_type
        );

        // AI summaries should be shorter than original content
        if content.len() > 200 {
            assert!(
                summary.len() < content.len(),
                "Summary should be shorter than original content for {}",
                content_type
            );
        }

        println!(
            "✅ AI analysis passed for {}: {} chars -> {} chars",
            content_type,
            content.len(),
            summary.len()
        );
        println!("📝 Summary: {}", summary);
    }

    println!("✅ AI-powered context content analysis test passed");
}

/// Helper function to test Ollama connectivity
async fn test_ollama_connectivity(base_url: &str) {
    let http = terraphim_service::http_client::create_default_client()
        .unwrap_or_else(|_| reqwest::Client::new());
    let health_url = format!("{}/api/tags", base_url.trim_end_matches('/'));

    let response = http
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if !resp.status().is_success() {
                panic!("❌ Ollama returned non-success status: {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!(
                "❌ Ollama not reachable at {} — skipping test: {}",
                base_url, e
            );
            // Skip the test if Ollama is not available
            return;
        }
    }
}

/// Helper function to create a test role with Ollama configuration
fn create_test_role_with_ollama(base_url: &str, role_name: &str) -> Role {
    let mut role = Role {
        shortname: Some(role_name.into()),
        name: format!("{} Role", role_name).into(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        extra: AHashMap::new(),
        ..Default::default()
    };

    role.extra
        .insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra
        .insert("llm_model".into(), serde_json::json!("llama3.2:3b"));
    role.extra
        .insert("llm_base_url".into(), serde_json::json!(base_url));
    role.extra
        .insert("llm_auto_summarize".into(), serde_json::json!(true));

    role
}

/// Helper function to create a test context item
fn create_test_context_item(id: &str, title: &str, content: &str) -> ContextItem {
    ContextItem {
        id: id.to_string(),
        context_type: ContextType::Document,
        title: title.to_string(),
        summary: Some(format!("Summary for {}", title)),
        content: content.to_string(),
        metadata: AHashMap::new(),
        created_at: chrono::Utc::now(),
        relevance_score: Some(0.85),
    }
}

/// Stress test for context operations with Ollama
#[tokio::test]
#[serial]
async fn ollama_context_stress_test() {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    println!("🧪 Running context management stress test with Ollama");

    // Test connectivity first
    test_ollama_connectivity(&base_url).await;

    let mut context_manager = ContextManager::new();
    let conversation_id = ConversationId::new("stress-test-conv");
    let role = create_test_role_with_ollama(&base_url, "StressTest");
    let llm_client = llm::build_llm_from_role(&role).expect("Failed to build LLM client");

    let start_time = std::time::Instant::now();
    let context_count = 10;
    let mut successful_operations = 0;

    // Create multiple contexts with AI processing
    for i in 0..context_count {
        let content = format!(
            "Context item {} contains information about Rust programming concepts. \
            This includes ownership, borrowing, lifetimes, and memory safety features. \
            Rust's unique approach to system programming makes it ideal for performance-critical applications.",
            i + 1
        );

        // Generate AI summary (this is the expensive operation)
        match llm_client
            .summarize(&content, llm::SummarizeOptions { max_length: 80 })
            .await
        {
            Ok(summary) => {
                let context_item = ContextItem {
                    id: format!("stress-ctx-{}", i),
                    context_type: ContextType::Document,
                    title: format!("Stress Test Context {}", i + 1),
                    summary: Some(summary),
                    content,
                    metadata: {
                        let mut metadata = AHashMap::new();
                        metadata.insert("stress_test".to_string(), "true".to_string());
                        metadata.insert("index".to_string(), i.to_string());
                        metadata
                    },
                    created_at: chrono::Utc::now(),
                    relevance_score: Some(0.9),
                };

                match context_manager.add_context_to_conversation(&conversation_id, context_item) {
                    Ok(_) => successful_operations += 1,
                    Err(e) => println!("❌ Failed to add context {}: {}", i, e),
                }
            }
            Err(e) => println!("❌ Failed to generate summary for context {}: {}", i, e),
        }
    }

    let duration = start_time.elapsed();
    let success_rate = (successful_operations as f64 / context_count as f64) * 100.0;

    // Verify final state
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Conversation should exist")
        .expect("Conversation should be found");

    println!("📊 Stress test results:");
    println!("   Total contexts: {}", context_count);
    println!("   Successful operations: {}", successful_operations);
    println!(
        "   Final context count: {}",
        conversation.global_context.len()
    );
    println!("   Success rate: {:.1}%", success_rate);
    println!("   Total time: {:?}", duration);
    println!(
        "   Average time per operation: {:?}",
        duration / context_count as u32
    );

    assert!(
        successful_operations > 0,
        "At least one context operation should succeed"
    );
    assert!(
        success_rate >= 70.0,
        "Success rate should be at least 70% (got {:.1}%)",
        success_rate
    );
    assert_eq!(
        conversation.global_context.len(),
        successful_operations,
        "Conversation should contain all successfully added contexts"
    );

    println!("✅ Context management stress test passed");
}

/// Test concurrent context operations with Ollama
#[tokio::test]
#[serial]
async fn ollama_context_concurrent_operations_test() {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    println!("🧪 Testing concurrent context operations with Ollama");

    // Test connectivity first
    test_ollama_connectivity(&base_url).await;

    let context_manager = Arc::new(tokio::sync::Mutex::new(ContextManager::new()));
    let conversation_id = ConversationId::new("concurrent-test-conv");
    let role = create_test_role_with_ollama(&base_url, "ConcurrentTest");
    let llm_client = Arc::new(llm::build_llm_from_role(&role).expect("Failed to build LLM client"));

    // Create concurrent tasks
    let mut tasks = vec![];
    let concurrent_ops = 5;

    for i in 0..concurrent_ops {
        let context_manager = Arc::clone(&context_manager);
        let conversation_id = conversation_id.clone();
        let llm_client = Arc::clone(&llm_client);

        let task = tokio::spawn(async move {
            let content = format!(
                "Concurrent operation {} testing Rust async programming with tokio runtime. \
                This demonstrates how Rust handles concurrent operations safely and efficiently.",
                i + 1
            );

            // Generate AI summary
            let summary = llm_client
                .summarize(&content, llm::SummarizeOptions { max_length: 60 })
                .await
                .map_err(|e| format!("Summary generation failed: {}", e))?;

            let context_item = ContextItem {
                id: format!("concurrent-ctx-{}", i),
                context_type: ContextType::Document,
                title: format!("Concurrent Context {}", i + 1),
                summary: Some(summary),
                content,
                metadata: {
                    let mut metadata = AHashMap::new();
                    metadata.insert("concurrent_test".to_string(), "true".to_string());
                    metadata.insert("task_id".to_string(), i.to_string());
                    metadata
                },
                created_at: chrono::Utc::now(),
                relevance_score: Some(0.88),
            };

            // Add to context manager (with mutex protection)
            let mut manager = context_manager.lock().await;
            manager
                .add_context_to_conversation(&conversation_id, context_item)
                .map_err(|e| format!("Context addition failed: {}", e))?;

            Ok::<_, String>(())
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;

    // Count successful operations
    let mut successful = 0;
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(Ok(())) => {
                successful += 1;
                println!("✅ Concurrent operation {} succeeded", i + 1);
            }
            Ok(Err(e)) => println!("❌ Concurrent operation {} failed: {}", i + 1, e),
            Err(e) => println!("❌ Concurrent task {} panicked: {}", i + 1, e),
        }
    }

    // Verify final state
    let manager = context_manager.lock().await;
    let conversation = manager
        .get_conversation(&conversation_id)
        .expect("Conversation should exist")
        .expect("Conversation should be found");

    let success_rate = (successful as f64 / concurrent_ops as f64) * 100.0;

    println!("📊 Concurrent operations results:");
    println!("   Total operations: {}", concurrent_ops);
    println!("   Successful operations: {}", successful);
    println!(
        "   Final context count: {}",
        conversation.global_context.len()
    );
    println!("   Success rate: {:.1}%", success_rate);

    assert!(
        successful > 0,
        "At least one concurrent operation should succeed"
    );
    assert_eq!(
        conversation.global_context.len(),
        successful,
        "Context count should match successful operations"
    );

    // Verify all contexts have required metadata
    for context in &conversation.global_context {
        assert!(context.summary.is_some());
        assert_eq!(
            context.metadata.get("concurrent_test"),
            Some(&"true".to_string())
        );
    }

    println!("✅ Concurrent context operations test passed");
}
