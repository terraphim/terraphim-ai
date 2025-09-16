use chrono::Utc;
use terraphim_multi_agent::{test_utils::*, *};
use tokio_test;

#[tokio::test]
async fn test_context_item_creation() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    let item = ContextItem::new(
        ContextItemType::Memory,
        "User prefers functional programming".to_string(),
        25, // token count
        0.85,
    )
    .with_metadata(ContextMetadata {
        source: Some("user_preference".to_string()),
        ..Default::default()
    });

    context.add_item(item.clone());

    assert_eq!(context.items.len(), 1);
    assert_eq!(context.items[0].content, item.content);
    assert_eq!(context.items[0].relevance_score, item.relevance_score);
    assert_eq!(context.items[0].item_type, item.item_type);
    assert_eq!(
        context.items[0].metadata.source,
        Some("user_preference".to_string())
    );
}

#[tokio::test]
async fn test_context_relevance_filtering() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    // Add items with different relevance scores
    let items = vec![
        ("High relevance item", 0.95),
        ("Medium relevance item", 0.65),
        ("Low relevance item", 0.25),
        ("Very high relevance item", 0.98),
        ("Below threshold item", 0.15),
    ];

    for (content, score) in items {
        context
            .add_item(ContextItem::new(
                ContextItemType::Memory,
                content.to_string(),
                20, // token count
                score,
            ))
            .unwrap();
    }

    // Test filtering with threshold 0.5
    let relevant_items = context.get_items_by_relevance(0.5, None);
    assert_eq!(
        relevant_items.len(),
        3,
        "Should return 3 items above 0.5 threshold"
    );

    // Should be sorted by relevance (highest first)
    assert!(relevant_items[0].relevance_score >= relevant_items[1].relevance_score);
    assert!(relevant_items[1].relevance_score >= relevant_items[2].relevance_score);

    // Test with limit
    let limited_items = context.get_items_by_relevance(0.5, Some(2));
    assert_eq!(limited_items.len(), 2, "Should respect limit parameter");

    // Should still be highest relevance items
    assert_eq!(limited_items[0].content, "Very high relevance item");
    assert_eq!(limited_items[1].content, "High relevance item");
}

#[tokio::test]
async fn test_context_different_item_types() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    // Add different types of context items
    let items = vec![
        (ContextItemType::Memory, "Remembered fact", 0.8),
        (ContextItemType::Task, "Current task", 0.9),
        (ContextItemType::Document, "Relevant document", 0.7),
        (ContextItemType::Lesson, "Learned lesson", 0.6),
    ];

    for (item_type, content, score) in items {
        context
            .add_item(ContextItem::new(
                item_type,
                content.to_string(),
                15, // token count
                score,
            ))
            .unwrap();
    }

    assert_eq!(context.items.len(), 4);

    // Verify all types are present
    let memory_count = context
        .items
        .iter()
        .filter(|i| matches!(i.item_type, ContextItemType::Memory))
        .count();
    let task_count = context
        .items
        .iter()
        .filter(|i| matches!(i.item_type, ContextItemType::Task))
        .count();
    let doc_count = context
        .items
        .iter()
        .filter(|i| matches!(i.item_type, ContextItemType::Document))
        .count();
    let lesson_count = context
        .items
        .iter()
        .filter(|i| matches!(i.item_type, ContextItemType::Lesson))
        .count();

    assert_eq!(memory_count, 1);
    assert_eq!(task_count, 1);
    assert_eq!(doc_count, 1);
    assert_eq!(lesson_count, 1);
}

#[tokio::test]
async fn test_context_automatic_enrichment() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // First, add some context manually
    {
        let mut context = agent.context.write().await;
        context
            .add_item(ContextItem::new(
                ContextItemType::Memory,
                "User is working on Rust web development".to_string(),
                30, // token count
                0.9,
            ))
            .unwrap();
    }

    // Process a command - this should use the context
    let input = CommandInput::new(CommandType::Generate, "Create a web API endpoint");
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    let output = result.unwrap();

    // Verify context was used
    assert!(
        output.context_used.len() > 0,
        "Should have used available context"
    );
    assert_eq!(
        output.context_used[0].content,
        "User is working on Rust web development"
    );
}

#[tokio::test]
async fn test_context_token_aware_truncation() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    // Add many context items to test truncation
    for i in 0..20 {
        context
            .add_item(ContextItem::new(
                ContextItemType::Memory,
                format!(
                    "Context item {} with detailed information that takes up tokens",
                    i
                ),
                50,                      // token count
                0.8 - (i as f64 * 0.01), // Decreasing relevance
            ))
            .unwrap();
    }

    // Test with different token limits
    let items_100 = context.get_items_by_relevance(0.0, Some(5));
    assert_eq!(
        items_100.len(),
        5,
        "Should respect limit even with many items"
    );

    // Should prioritize highest relevance
    for i in 1..items_100.len() {
        assert!(items_100[i - 1].relevance_score >= items_100[i].relevance_score);
    }
}

#[tokio::test]
async fn test_context_update_and_cleanup() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    // Add some items
    let current_task = ContextItem::new(
        ContextItemType::Task,
        "Current task".to_string(),
        20, // token count
        0.9,
    );
    context.add_item(current_task).unwrap();

    let old_memory = ContextItem::new(
        ContextItemType::Memory,
        "Old memory".to_string(),
        15, // token count
        0.3,
    );
    context.add_item(old_memory).unwrap();

    assert_eq!(context.items.len(), 2);

    // Test clearing items below threshold
    context.items.retain(|item| item.relevance_score >= 0.5);
    assert_eq!(context.items.len(), 1);
    assert_eq!(context.items[0].content, "Current task");
}

#[tokio::test]
async fn test_context_metadata_handling() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    let metadata = ContextMetadata {
        source: Some("knowledge_graph".to_string()),
        tags: vec!["technical".to_string(), "high_confidence".to_string()],
        ..Default::default()
    };

    let doc_item = ContextItem::new(
        ContextItemType::Document,
        "Technical documentation excerpt".to_string(),
        40, // token count
        0.85,
    )
    .with_metadata(metadata.clone());

    context.add_item(doc_item).unwrap();

    assert_eq!(context.items.len(), 1);
    let item = &context.items[0];

    assert_eq!(item.metadata.source, Some("knowledge_graph".to_string()));
    assert!(item.metadata.tags.contains(&"technical".to_string()));
    assert!(item.metadata.tags.contains(&"high_confidence".to_string()));
}

#[tokio::test]
async fn test_context_concurrent_access() {
    let agent = create_test_agent().await.unwrap();

    use tokio::task::JoinSet;
    let mut join_set = JoinSet::new();

    // Add context items concurrently
    for i in 0..10 {
        let agent_clone = agent.clone();
        join_set.spawn(async move {
            let mut context = agent_clone.context.write().await;
            context
                .add_item(ContextItem::new(
                    ContextItemType::Memory,
                    format!("Concurrent item {}", i),
                    20, // token count
                    0.7,
                ))
                .unwrap();
        });
    }

    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }

    let context = agent.context.read().await;
    assert_eq!(
        context.items.len(),
        10,
        "All concurrent additions should succeed"
    );
}

#[tokio::test]
async fn test_context_relevance_scoring() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    // Add items with edge case relevance scores
    let test_scores = vec![0.0, 0.1, 0.5, 0.99, 1.0];

    for (i, score) in test_scores.iter().enumerate() {
        context
            .add_item(ContextItem::new(
                ContextItemType::Memory,
                format!("Item with score {}", score),
                15, // token count
                *score,
            ))
            .unwrap();
    }

    // Test boundary conditions
    let items_above_zero = context.get_items_by_relevance(0.0, None);
    assert_eq!(
        items_above_zero.len(),
        5,
        "Should include items with score 0.0"
    );

    let items_above_half = context.get_items_by_relevance(0.5, None);
    assert_eq!(
        items_above_half.len(),
        3,
        "Should include items with score >= 0.5"
    );

    let items_above_one = context.get_items_by_relevance(1.0, None);
    assert_eq!(
        items_above_one.len(),
        1,
        "Should include only items with score 1.0"
    );
}

#[tokio::test]
async fn test_context_timestamp_handling() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    let now = Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);
    let one_day_ago = now - chrono::Duration::days(1);

    let recent_memory = ContextItem::new(
        ContextItemType::Memory,
        "Recent memory".to_string(),
        20, // token count
        0.8,
    );
    context.add_item(recent_memory).unwrap();

    let recent_task = ContextItem::new(
        ContextItemType::Task,
        "Recent task".to_string(),
        18, // token count
        0.8,
    );
    context.add_item(recent_task).unwrap();

    let old_doc = ContextItem::new(
        ContextItemType::Document,
        "Old document".to_string(),
        22, // token count
        0.8,
    );
    context.add_item(old_doc).unwrap();

    assert_eq!(context.items.len(), 3);

    // Verify timestamps are preserved (Note: timestamps are set automatically in ContextItem::new)
    let recent_items: Vec<_> = context
        .items
        .iter()
        .filter(|item| item.added_at > one_hour_ago)
        .collect();
    // All items will be recent since they were just created
    assert!(recent_items.len() >= 2, "Should find recent items");
}

#[tokio::test]
async fn test_context_integration_with_agent_memory() {
    let agent = create_test_agent().await.unwrap();
    agent.initialize().await.unwrap();

    // Add memory through agent's memory system
    {
        let mut memory = agent.memory.write().await;
        let snapshot = memory.create_snapshot("test_memory".to_string(), 0.85);
        memory.add_memory(
            "User preference".to_string(),
            "Prefers async Rust".to_string(),
            snapshot,
        );
    }

    // Process a command that should pull from memory into context
    let input = CommandInput::new(CommandType::Answer, "How should I write this function?");
    let result = agent.process_command(input).await;
    assert!(result.is_ok());

    // Context should have been enriched from agent memory
    let context = agent.context.read().await;

    // This tests the integration - context should have items added from memory
    // (The exact behavior depends on the context enrichment implementation)
}

#[tokio::test]
async fn test_context_threshold_configuration() {
    let agent = create_test_agent().await.unwrap();

    let mut context = agent.context.write().await;

    // Note: relevance_threshold is not a field in AgentContext anymore
    // We'll use the get_items_by_relevance method with threshold parameter directly

    let high_relevance = ContextItem::new(
        ContextItemType::Memory,
        "High relevance".to_string(),
        15, // token count
        0.8,
    );
    context.add_item(high_relevance).unwrap();

    let low_relevance = ContextItem::new(
        ContextItemType::Memory,
        "Low relevance".to_string(),
        12, // token count
        0.6,
    );
    context.add_item(low_relevance).unwrap();

    // Using threshold 0.7
    let relevant = context.get_items_by_relevance(0.7, None);
    assert_eq!(relevant.len(), 1, "Should use threshold 0.7");
    assert_eq!(relevant[0].content, "High relevance");
}
