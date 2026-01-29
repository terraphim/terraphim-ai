use terraphim_agent::client::*;
use terraphim_types::{Document, NormalizedTermValue, RoleName, SearchQuery};

/// Test ApiClient construction and basic properties
#[test]
fn test_api_client_construction() {
    let base_url = "http://localhost:8000";
    let client = ApiClient::new(base_url);

    // Client should be constructible and Clone-able
    let cloned_client = client.clone();

    // Both clients should be valid (we can't test internal state directly)
    // But we can verify they don't panic when created
    assert_eq!(
        std::mem::size_of_val(&client),
        std::mem::size_of_val(&cloned_client)
    );
}

/// Test SearchQuery construction and serialization
#[test]
fn test_search_query_serialization() {
    let query = SearchQuery {
        search_term: NormalizedTermValue::from("test query"),
        skip: Some(10),
        limit: Some(5),
        role: Some(RoleName::new("TestRole")),
        ..Default::default()
    };

    // Should be serializable to JSON
    let json_result = serde_json::to_string(&query);
    assert!(json_result.is_ok(), "SearchQuery should be serializable");

    let json_str = json_result.unwrap();
    assert!(json_str.contains("test query"));
    assert!(json_str.contains("10")); // skip value
    assert!(json_str.contains("5")); // limit value
    assert!(json_str.contains("TestRole"));

    // Should be deserializable from JSON
    let deserialized: Result<SearchQuery, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok(), "SearchQuery should be deserializable");

    let deserialized_query = deserialized.unwrap();
    assert_eq!(deserialized_query.search_term, query.search_term);
    assert_eq!(deserialized_query.skip, query.skip);
    assert_eq!(deserialized_query.limit, query.limit);
    assert_eq!(deserialized_query.role, query.role);
}

/// Test SearchResponse deserialization
#[test]
fn test_search_response_deserialization() {
    let json_response = r#"{
        "status": "Success",
        "results": [
            {
                "id": "doc1",
                "title": "Test Document",
                "body": "Test content",
                "url": "http://example.com/doc1",
                "description": "A test document",
                "summarization": null,
                "stub": null,
                "tags": ["test", "example"],
                "rank": 95
            }
        ],
        "total": 1
    }"#;

    let response: Result<SearchResponse, _> = serde_json::from_str(json_response);
    assert!(response.is_ok(), "SearchResponse should be deserializable");

    let search_response = response.unwrap();
    assert_eq!(search_response.status, "Success");
    assert_eq!(search_response.total, 1);
    assert_eq!(search_response.results.len(), 1);

    let doc = &search_response.results[0];
    assert_eq!(doc.id, "doc1");
    assert_eq!(doc.title, "Test Document");
    assert_eq!(doc.body, "Test content");
    assert_eq!(doc.rank, Some(95));
}

/// Test ConfigResponse deserialization
#[test]
fn test_config_response_deserialization() {
    let json_response = r#"{
        "status": "Success",
        "config": {
            "id": "Embedded",
            "selected_role": "Default",
            "default_role": "Default",
            "global_shortcut": "Ctrl+Space",
            "roles": {
                "Default": {
                    "shortname": "Default",
                    "name": "Default",
                    "relevance_function": "bm25",
                    "terraphim_it": false,
                    "theme": "default",
                    "kg": null,
                    "haystacks": [],
                    "extra": {}
                }
            }
        }
    }"#;

    let response: Result<ConfigResponse, _> = serde_json::from_str(json_response);
    assert!(
        response.is_ok(),
        "ConfigResponse should be deserializable: {:?}",
        response.as_ref().err().map(|e| e.to_string())
    );

    let config_response = response.unwrap();
    assert_eq!(config_response.status, "Success");
    assert_eq!(config_response.config.selected_role.to_string(), "Default");
    assert_eq!(config_response.config.global_shortcut, "Ctrl+Space");
    assert!(config_response
        .config
        .roles
        .contains_key(&RoleName::new("Default")));
}

/// Test ChatRequest serialization
#[test]
fn test_chat_request_serialization() {
    let chat_request = ChatRequest {
        role: "TestRole".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
        }],
        model: Some("gpt-3.5-turbo".to_string()),
    };

    let json_result = serde_json::to_string(&chat_request);
    assert!(json_result.is_ok(), "ChatRequest should be serializable");

    let json_str = json_result.unwrap();
    assert!(json_str.contains("TestRole"));
    assert!(json_str.contains("user"));
    assert!(json_str.contains("Hello, world!"));
    assert!(json_str.contains("gpt-3.5-turbo"));
}

/// Test ChatResponse deserialization
#[test]
fn test_chat_response_deserialization() {
    let json_response = r#"{
        "status": "Success",
        "message": "Hello! How can I help you?",
        "model_used": "gpt-3.5-turbo",
        "error": null
    }"#;

    let response: Result<ChatResponse, _> = serde_json::from_str(json_response);
    assert!(response.is_ok(), "ChatResponse should be deserializable");

    let chat_response = response.unwrap();
    assert_eq!(chat_response.status, "Success");
    assert_eq!(chat_response.message.unwrap(), "Hello! How can I help you?");
    assert_eq!(chat_response.model_used.unwrap(), "gpt-3.5-turbo");
    assert!(chat_response.error.is_none());
}

/// Test SummarizeRequest serialization
#[test]
fn test_summarize_request_serialization() {
    let document = Document {
        id: "test-doc".to_string(),
        title: "Test Document".to_string(),
        body: "This is a test document with some content.".to_string(),
        url: "http://example.com/test".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
    };

    let summarize_request = SummarizeRequest {
        document: document.clone(),
        role: Some("TestRole".to_string()),
    };

    let json_result = serde_json::to_string(&summarize_request);
    assert!(
        json_result.is_ok(),
        "SummarizeRequest should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("test-doc"));
    assert!(json_str.contains("Test Document"));
    assert!(json_str.contains("TestRole"));
}

/// Test ThesaurusResponse deserialization
#[test]
fn test_thesaurus_response_deserialization() {
    let json_response = r#"{
        "status": "Success",
        "terms": [
            {
                "id": "term1",
                "nterm": "machine learning",
                "url": "http://example.com/ml"
            },
            {
                "id": "term2",
                "nterm": "artificial intelligence",
                "url": null
            }
        ],
        "total": 2
    }"#;

    let response: Result<ThesaurusResponse, _> = serde_json::from_str(json_response);
    assert!(
        response.is_ok(),
        "ThesaurusResponse should be deserializable"
    );

    let thesaurus_response = response.unwrap();
    assert_eq!(thesaurus_response.status, "Success");
    assert_eq!(thesaurus_response.total, 2);
    assert_eq!(thesaurus_response.terms.len(), 2);

    let term1 = &thesaurus_response.terms[0];
    assert_eq!(term1.id, "term1");
    assert_eq!(term1.nterm, "machine learning");
    assert_eq!(term1.url.as_ref().unwrap(), "http://example.com/ml");

    let term2 = &thesaurus_response.terms[1];
    assert_eq!(term2.id, "term2");
    assert_eq!(term2.nterm, "artificial intelligence");
    assert!(term2.url.is_none());
}

/// Test AutocompleteResponse deserialization
#[test]
fn test_autocomplete_response_deserialization() {
    let json_response = r#"{
        "status": "Success",
        "suggestions": [
            {
                "text": "machine learning",
                "score": 0.95
            },
            {
                "text": "machine intelligence",
                "score": 0.87
            }
        ]
    }"#;

    let response: Result<AutocompleteResponse, _> = serde_json::from_str(json_response);
    assert!(
        response.is_ok(),
        "AutocompleteResponse should be deserializable"
    );

    let autocomplete_response = response.unwrap();
    assert_eq!(autocomplete_response.status, "Success");
    assert_eq!(autocomplete_response.suggestions.len(), 2);

    let suggestion1 = &autocomplete_response.suggestions[0];
    assert_eq!(suggestion1.text, "machine learning");
    assert_eq!(suggestion1.score, 0.95);

    let suggestion2 = &autocomplete_response.suggestions[1];
    assert_eq!(suggestion2.text, "machine intelligence");
    assert_eq!(suggestion2.score, 0.87);
}

/// Test RoleGraphResponseDto deserialization
#[test]
fn test_rolegraph_response_deserialization() {
    let json_response = r#"{
        "status": "Success",
        "nodes": [
            {
                "id": 1,
                "label": "machine learning",
                "rank": 100
            },
            {
                "id": 2,
                "label": "artificial intelligence",
                "rank": 95
            }
        ],
        "edges": [
            {
                "source": 1,
                "target": 2,
                "rank": 50
            }
        ]
    }"#;

    let response: Result<RoleGraphResponseDto, _> = serde_json::from_str(json_response);
    assert!(
        response.is_ok(),
        "RoleGraphResponseDto should be deserializable"
    );

    let rolegraph_response = response.unwrap();
    assert_eq!(rolegraph_response.status, "Success");
    assert_eq!(rolegraph_response.nodes.len(), 2);
    assert_eq!(rolegraph_response.edges.len(), 1);

    let node1 = &rolegraph_response.nodes[0];
    assert_eq!(node1.id, 1);
    assert_eq!(node1.label, "machine learning");
    assert_eq!(node1.rank, 100);

    let edge = &rolegraph_response.edges[0];
    assert_eq!(edge.source, 1);
    assert_eq!(edge.target, 2);
    assert_eq!(edge.rank, 50);
}

/// Test TaskStatusResponse deserialization with different states
#[test]
fn test_task_status_response_deserialization() {
    let test_cases = vec![
        (
            r#"{
            "status": "Success",
            "task_id": "task-123",
            "state": "pending",
            "progress": null,
            "result": null,
            "error": null,
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2023-01-01T00:00:00Z"
        }"#,
            "pending",
        ),
        (
            r#"{
            "status": "Success",
            "task_id": "task-456",
            "state": "processing",
            "progress": 0.5,
            "result": null,
            "error": null,
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2023-01-01T00:01:00Z"
        }"#,
            "processing",
        ),
        (
            r#"{
            "status": "Success",
            "task_id": "task-789",
            "state": "completed",
            "progress": 1.0,
            "result": "Task completed successfully",
            "error": null,
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2023-01-01T00:05:00Z"
        }"#,
            "completed",
        ),
        (
            r#"{
            "status": "Success",
            "task_id": "task-000",
            "state": "failed",
            "progress": null,
            "result": null,
            "error": "Task failed due to error",
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2023-01-01T00:02:00Z"
        }"#,
            "failed",
        ),
    ];

    for (json_response, expected_state) in test_cases {
        let response: Result<TaskStatusResponse, _> = serde_json::from_str(json_response);
        assert!(
            response.is_ok(),
            "TaskStatusResponse should be deserializable for state {}",
            expected_state
        );

        let task_response = response.unwrap();
        assert_eq!(task_response.status, "Success");
        assert_eq!(task_response.state, expected_state);
        assert!(task_response.task_id.starts_with("task-"));
    }
}

/// Test QueueStatsResponse deserialization
#[test]
fn test_queue_stats_response_deserialization() {
    let json_response = r#"{
        "status": "Success",
        "pending_tasks": 5,
        "processing_tasks": 2,
        "completed_tasks": 100,
        "failed_tasks": 3,
        "total_tasks": 110
    }"#;

    let response: Result<QueueStatsResponse, _> = serde_json::from_str(json_response);
    assert!(
        response.is_ok(),
        "QueueStatsResponse should be deserializable"
    );

    let stats_response = response.unwrap();
    assert_eq!(stats_response.status, "Success");
    assert_eq!(stats_response.pending_tasks, 5);
    assert_eq!(stats_response.processing_tasks, 2);
    assert_eq!(stats_response.completed_tasks, 100);
    assert_eq!(stats_response.failed_tasks, 3);
    assert_eq!(stats_response.total_tasks, 110);

    // Verify totals add up correctly
    let sum = stats_response.pending_tasks
        + stats_response.processing_tasks
        + stats_response.completed_tasks
        + stats_response.failed_tasks;
    assert_eq!(sum, stats_response.total_tasks);
}

/// Test BatchSummarizeRequest serialization
#[test]
fn test_batch_summarize_request_serialization() {
    let documents = vec![
        Document {
            id: "doc1".to_string(),
            title: "Document 1".to_string(),
            body: "Content 1".to_string(),
            url: "".to_string(),
            description: None,
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
            source_haystack: None,
        },
        Document {
            id: "doc2".to_string(),
            title: "Document 2".to_string(),
            body: "Content 2".to_string(),
            url: "".to_string(),
            description: None,
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
            source_haystack: None,
        },
    ];

    let batch_request = BatchSummarizeRequest {
        documents: documents.clone(),
        role: Some("TestRole".to_string()),
    };

    let json_result = serde_json::to_string(&batch_request);
    assert!(
        json_result.is_ok(),
        "BatchSummarizeRequest should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("doc1"));
    assert!(json_str.contains("doc2"));
    assert!(json_str.contains("Document 1"));
    assert!(json_str.contains("Document 2"));
    assert!(json_str.contains("TestRole"));
}

/// Test error response handling
#[test]
fn test_error_response_deserialization() {
    let error_responses = vec![
        r#"{
            "status": "Error",
            "message": null,
            "error": "Role not found"
        }"#,
        r#"{
            "status": "Failed",
            "results": [],
            "total": 0
        }"#,
    ];

    for json_response in error_responses {
        // Try to parse as different response types
        let chat_result: Result<ChatResponse, _> = serde_json::from_str(json_response);
        let search_result: Result<SearchResponse, _> = serde_json::from_str(json_response);

        // At least one should parse successfully or fail gracefully
        if let Ok(chat_response) = chat_result {
            assert!(!chat_response.status.is_empty());
            assert!(chat_response.status == "Error" || chat_response.status == "Failed");
        }

        if let Ok(search_response) = search_result {
            assert!(!search_response.status.is_empty());
            assert!(search_response.status == "Error" || search_response.status == "Failed");
        }
    }
}

/// Test edge cases and boundary values
#[test]
fn test_boundary_values() {
    // Test with empty strings and zero values
    let query = SearchQuery {
        search_term: NormalizedTermValue::from(""),
        skip: Some(0),
        limit: Some(0),
        role: None,
        ..Default::default()
    };

    let json_result = serde_json::to_string(&query);
    assert!(json_result.is_ok(), "Should handle empty/zero values");

    // Test with very large values
    let large_query = SearchQuery {
        search_term: NormalizedTermValue::from("test"),
        skip: Some(999999),
        limit: Some(999999),
        role: Some(RoleName::new("test")),
        ..Default::default()
    };

    let large_json_result = serde_json::to_string(&large_query);
    assert!(large_json_result.is_ok(), "Should handle large values");

    // Test document with all fields
    let complete_doc = Document {
        id: "complete".to_string(),
        title: "Complete Document".to_string(),
        body: "Full content here".to_string(),
        url: "http://example.com".to_string(),
        description: Some("Description here".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        rank: Some(100),
        source_haystack: None,
    };

    let doc_json = serde_json::to_string(&complete_doc);
    assert!(doc_json.is_ok(), "Should serialize complete document");

    // Test document with minimal fields
    let minimal_doc = Document {
        id: "minimal".to_string(),
        title: "Minimal".to_string(),
        body: "".to_string(),
        url: "".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
    };

    let minimal_doc_json = serde_json::to_string(&minimal_doc);
    assert!(
        minimal_doc_json.is_ok(),
        "Should serialize minimal document"
    );
}

/// Test Clone implementations
#[test]
fn test_clone_implementations() {
    let client = ApiClient::new("http://test.com");
    let cloned_client = client.clone();

    // Should not panic and should be usable
    drop(cloned_client);
    drop(client);

    let response = SearchResponse {
        status: "Success".to_string(),
        results: vec![],
        total: 0,
    };
    let cloned_response = response.clone();
    assert_eq!(response.status, cloned_response.status);
    assert_eq!(response.total, cloned_response.total);
}

/// Test Debug implementations
#[test]
fn test_debug_implementations() {
    let client = ApiClient::new("http://test.com");
    let debug_str = format!("{:?}", client);
    assert!(!debug_str.is_empty(), "Debug should produce output");

    let response = SearchResponse {
        status: "Success".to_string(),
        results: vec![],
        total: 0,
    };
    let debug_response = format!("{:?}", response);
    assert!(debug_response.contains("Success"));
    assert!(debug_response.contains("0"));
}
