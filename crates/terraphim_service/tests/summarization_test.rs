use anyhow::Result;
use serial_test::serial;
use terraphim_config::Role;
use terraphim_service::summarization_manager::{SummarizationManager, SummarizationManagerBuilder};
use terraphim_service::summarization_queue::QueueConfig;
use terraphim_types::Document;

/// Helper function to create a test role configuration
fn create_test_role() -> Role {
    Role {
        name: "Test Engineer".to_string().into(),
        shortname: Some("test_engineer".to_string()),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        theme: "light".to_string(),
        terraphim_it: false,
        kg: None,
        haystacks: vec![],
        ..Default::default()
    }
}

/// Helper function to create a test document
fn create_test_document(id: &str, body: &str) -> Document {
    Document {
        id: id.to_string(),
        title: format!("Test Document {}", id),
        body: body.to_string(),
        url: format!("https://example.com/{}", id),
        description: None,
        summarization: None,
        stub: None,
        tags: Some(vec!["test".to_string()]),
        rank: Some(100),
        source_haystack: None,
    }
}

/// Helper function to create a comprehensive test document with substantial content
fn create_comprehensive_test_document() -> Document {
    let body = r#"
    Introduction to Advanced Software Architecture

    Software architecture is the fundamental structure of a software system, encompassing the high-level design decisions that shape how components interact, data flows, and the overall system behavior. This comprehensive guide explores the key principles and patterns that enable scalable, maintainable, and robust software systems.

    Key Architectural Patterns

    The most widely adopted patterns include microservices architecture, which breaks down monolithic applications into smaller, independently deployable services. Event-driven architecture enables loose coupling between components through asynchronous messaging. Domain-driven design helps organize complex business logic around business domains and bounded contexts.

    Scalability Considerations

    Modern systems must handle increasing load through horizontal and vertical scaling strategies. Load balancing distributes traffic across multiple instances, while caching reduces database load and improves response times. Database sharding and replication ensure data availability and performance at scale.

    Security and Reliability

    Security must be built into the architecture from the ground up, implementing authentication, authorization, input validation, and secure communication protocols. Reliability requires redundancy, failover mechanisms, circuit breakers, and comprehensive monitoring and alerting systems.

    Performance Optimization

    Performance optimization involves careful consideration of data structures, algorithms, network latency, and resource utilization. Profiling and benchmarking help identify bottlenecks, while caching strategies and content delivery networks improve user experience globally.

    Conclusion

    Successful software architecture requires balancing trade-offs between complexity, performance, maintainability, and cost. The principles and patterns discussed in this guide provide a foundation for making informed architectural decisions that support long-term success.
    "#;

    Document {
        id: "comprehensive-architecture-guide".to_string(),
        title: "Advanced Software Architecture Guide".to_string(),
        body: body.trim().to_string(),
        url: "https://example.com/architecture-guide".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: Some(vec![
            "architecture".to_string(),
            "software".to_string(),
            "guide".to_string(),
        ]),
        rank: Some(200),
        source_haystack: None,
    }
}

#[tokio::test]
#[serial]
async fn test_extract_description_from_short_body() -> Result<()> {
    let body = "This is a short document body with sufficient content.";

    let result = SummarizationManager::extract_description_from_body(body, 200)?;

    assert_eq!(
        result,
        "This is a short document body with sufficient content."
    );
    assert!(result.len() <= 200);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_extract_description_from_long_body() -> Result<()> {
    let body = "This is a very long document body that contains multiple sentences and exceeds the maximum description length specified in the configuration. It should be truncated at an appropriate boundary, preferably at a sentence ending to maintain readability. The system should handle this gracefully and provide a meaningful excerpt.";

    let result = SummarizationManager::extract_description_from_body(body, 150)?;

    assert!(result.len() <= 150);
    assert!(result.contains("This is a very long document"));
    assert!(!result.contains("gracefully")); // Should be truncated before this word

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_extract_description_sentence_boundary() -> Result<()> {
    let body = "First sentence is short. Second sentence contains more detailed information about the topic. Third sentence would exceed the limit and should not be included.";

    let result = SummarizationManager::extract_description_from_body(body, 100)?;

    assert!(result.len() <= 100);
    assert!(result.ends_with("."));
    assert!(result.contains("First sentence is short."));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_extract_description_multiline_content() -> Result<()> {
    let body = r#"

    This is the first substantial paragraph after some empty lines.
    It contains multiple lines with important information.

    This is the second paragraph that should not be included.
    "#;

    let result = SummarizationManager::extract_description_from_body(body, 200)?;

    assert!(result.contains("This is the first substantial paragraph"));
    assert!(!result.contains("second paragraph"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_extract_description_empty_body_error() -> Result<()> {
    let body = "";

    let result = SummarizationManager::extract_description_from_body(body, 200);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_process_document_fields_description_extraction() -> Result<()> {
    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    let mut doc = create_test_document(
        "test-desc",
        "This document needs a description extracted from its body content for testing purposes.",
    );

    let task_id = manager
        .process_document_fields(&mut doc, &role, true, false)
        .await?;

    // Should extract description, no summarization task queued
    assert!(doc.description.is_some());
    assert!(doc.summarization.is_none());
    assert!(task_id.is_none());

    let description = doc.description.as_ref().unwrap();
    assert!(description.contains("This document needs a description"));
    assert!(description.len() <= 200);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_process_document_fields_preserves_existing_description() -> Result<()> {
    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    let mut doc = create_test_document(
        "test-preserve",
        "This document has body content but already has a description.",
    );
    doc.description = Some("Existing description that should be preserved".to_string());

    let _task_id = manager
        .process_document_fields(&mut doc, &role, true, false)
        .await?;

    // Should preserve existing description
    assert_eq!(
        doc.description.as_ref().unwrap(),
        "Existing description that should be preserved"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_process_document_fields_no_description_for_empty_body() -> Result<()> {
    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    let mut doc = create_test_document("test-empty", "");

    let _task_id = manager
        .process_document_fields(&mut doc, &role, true, false)
        .await?;

    // Should not create description for empty body
    assert!(doc.description.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_process_document_fields_queues_summarization_for_long_content() -> Result<()> {
    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    let mut doc = create_comprehensive_test_document(); // Has substantial content > 500 chars

    let task_id = manager
        .process_document_fields(&mut doc, &role, true, true)
        .await?;

    // Should extract description and queue summarization
    assert!(doc.description.is_some());
    assert!(task_id.is_some()); // Summarization task was queued

    let description = doc.description.as_ref().unwrap();
    assert!(description.to_lowercase().contains("software architecture"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_process_document_fields_no_summarization_for_short_content() -> Result<()> {
    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    let mut doc = create_test_document("test-short", "Short content under 500 characters.");

    let task_id = manager
        .process_document_fields(&mut doc, &role, true, true)
        .await?;

    // Should extract description but not queue summarization
    assert!(doc.description.is_some());
    assert!(task_id.is_none()); // No summarization task queued for short content

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_process_documents_batch() -> Result<()> {
    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    let mut documents = vec![
        create_test_document(
            "batch-1",
            "First document in batch processing test with sufficient content.",
        ),
        create_test_document(
            "batch-2",
            "Second document in batch processing test with sufficient content.",
        ),
        create_comprehensive_test_document(), // Long content
    ];

    let task_ids = manager
        .process_documents_batch(&mut documents, &role, true, true)
        .await?;

    // All documents should have descriptions
    for doc in &documents {
        assert!(
            doc.description.is_some(),
            "Document {} should have description",
            doc.id
        );
    }

    // Should return task IDs array with same length
    assert_eq!(task_ids.len(), documents.len());

    // First two documents should not have summarization tasks (short content)
    assert!(task_ids[0].is_none());
    assert!(task_ids[1].is_none());

    // Third document should have summarization task (long content)
    assert!(task_ids[2].is_some());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_process_documents_batch_no_description_extraction() -> Result<()> {
    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    let mut documents = vec![
        create_test_document("no-desc-1", "Document content for testing."),
        create_test_document("no-desc-2", "Another document content."),
    ];

    let task_ids = manager
        .process_documents_batch(&mut documents, &role, false, false)
        .await?;

    // No descriptions should be extracted
    for doc in &documents {
        assert!(
            doc.description.is_none(),
            "Document {} should not have description",
            doc.id
        );
    }

    // No tasks should be queued
    for task_id in &task_ids {
        assert!(task_id.is_none());
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_process_documents_batch_mixed_content_lengths() -> Result<()> {
    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    let mut documents = vec![
        create_test_document("mixed-1", "Short content."), // Under 500 chars
        create_comprehensive_test_document(),              // Over 500 chars
        create_test_document("mixed-3", "Another short content."), // Under 500 chars
    ];

    let task_ids = manager
        .process_documents_batch(&mut documents, &role, true, true)
        .await?;

    // All should have descriptions
    assert_eq!(
        documents.iter().filter(|d| d.description.is_some()).count(),
        3
    );

    // Only the long document should have a summarization task
    let summarization_tasks = task_ids.iter().filter(|t| t.is_some()).count();
    assert_eq!(summarization_tasks, 1);

    // The middle document (comprehensive) should have the task
    assert!(task_ids[0].is_none()); // Short content
    assert!(task_ids[1].is_some()); // Long content
    assert!(task_ids[2].is_none()); // Short content

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_manager_builder_configuration() -> Result<()> {
    let manager = SummarizationManagerBuilder::new()
        .max_queue_size(100)
        .build();

    // Manager should be created and healthy
    assert!(manager.is_healthy());

    // Should be able to get stats
    let stats = manager.get_stats().await?;
    assert_eq!(stats.queue_size, 0);
    assert_eq!(stats.completed_tasks, 0);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_document_fields_integration_with_persistence() -> Result<()> {
    // This test verifies that documents with description and summarization fields
    // can be properly saved and loaded through the persistence layer

    use terraphim_persistence::Persistable;

    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);
    let role = create_test_role();

    // Initialize test persistence
    terraphim_persistence::DeviceStorage::init_memory_only().await?;

    let mut doc = create_comprehensive_test_document();

    // Process the document to add description
    let _task_id = manager
        .process_document_fields(&mut doc, &role, true, false)
        .await?;

    // Add a mock summarization for testing persistence
    doc.summarization = Some("This is a comprehensive guide covering software architecture principles, patterns, and best practices for building scalable systems.".to_string());

    // Save the document
    doc.save().await?;

    // Load the document back
    let mut loaded_doc = Document::new(doc.id.clone());
    loaded_doc = loaded_doc.load().await?;

    // Verify all fields are preserved
    assert_eq!(loaded_doc.id, doc.id);
    assert_eq!(loaded_doc.title, doc.title);
    assert_eq!(loaded_doc.body, doc.body);
    assert_eq!(loaded_doc.description, doc.description);
    assert_eq!(loaded_doc.summarization, doc.summarization);

    // Verify extracted description makes sense
    assert!(loaded_doc
        .description
        .as_ref()
        .unwrap()
        .to_lowercase()
        .contains("software architecture"));

    // Verify summarization is preserved
    assert!(loaded_doc
        .summarization
        .as_ref()
        .unwrap()
        .contains("comprehensive guide"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_summarization_manager_lifecycle() -> Result<()> {
    let config = QueueConfig::default();
    let mut manager = SummarizationManager::new(config);

    // Manager should start healthy
    assert!(manager.is_healthy());

    // Should be able to pause and resume
    manager.pause().await?;
    manager.resume().await?;

    // Should be able to get stats
    let stats = manager.get_stats().await?;
    assert!(stats.queue_size == 0);

    // Should shut down cleanly
    manager.shutdown().await?;
    assert!(!manager.is_healthy());

    Ok(())
}
