#[cfg(test)]
mod logical_operators_fix_validation_tests {
    use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
    use terraphim_service::TerraphimService;
    use terraphim_types::{Document, LogicalOperator, NormalizedTermValue, RoleName, SearchQuery};

    async fn setup_test_service() -> TerraphimService {
        let mut config = ConfigBuilder::new_with_id(ConfigId::Embedded)
            .build_default_embedded()
            .build()
            .unwrap();
        let config_state = ConfigState::new(&mut config).await.unwrap();
        TerraphimService::new(config_state)
    }

    fn create_test_documents() -> Vec<Document> {
        vec![
            Document {
                id: "1".to_string(),
                title: "Rust programming".to_string(),
                body: "This document covers Rust programming concepts".to_string(),
                url: "http://example.com/rust".to_string(),
                description: Some("A guide to Rust".to_string()),
                rank: None,
                tags: None,
                summarization: None,
                stub: None,
            },
            Document {
                id: "2".to_string(),
                title: "Async programming in Rust".to_string(),
                body: "Learn about async and await in Rust programming language".to_string(),
                url: "http://example.com/async".to_string(),
                description: Some("Async Rust tutorial".to_string()),
                rank: None,
                tags: None,
                summarization: None,
                stub: None,
            },
            Document {
                id: "3".to_string(),
                title: "JavaScript async patterns".to_string(),
                body: "Modern async patterns in JavaScript development".to_string(),
                url: "http://example.com/js".to_string(),
                description: Some("JS async guide".to_string()),
                rank: None,
                tags: None,
                summarization: None,
                stub: None,
            },
            Document {
                id: "4".to_string(),
                title: "Python programming".to_string(),
                body: "Introduction to Python programming concepts and syntax".to_string(),
                url: "http://example.com/python".to_string(),
                description: Some("Python basics".to_string()),
                rank: None,
                tags: None,
                summarization: None,
                stub: None,
            },
        ]
    }

    #[tokio::test]
    async fn test_get_all_terms_no_duplication_and_operator() {
        let mut service = setup_test_service().await;
        let documents = create_test_documents();

        // Test query: "rust AND async" - should match only document 2
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("rust"),
            search_terms: Some(vec![
                NormalizedTermValue::from("rust"),
                NormalizedTermValue::from("async"),
            ]),
            operator: Some(LogicalOperator::And),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        // Test get_all_terms to ensure no duplication
        let all_terms = query.get_all_terms();
        let terms_as_strings: Vec<String> =
            all_terms.iter().map(|t| t.as_str().to_string()).collect();

        // Should contain both terms but no duplicates
        assert_eq!(
            terms_as_strings.len(),
            2,
            "Should have exactly 2 terms, got: {:?}",
            terms_as_strings
        );
        assert!(
            terms_as_strings.contains(&"rust".to_string()),
            "Should contain 'rust'"
        );
        assert!(
            terms_as_strings.contains(&"async".to_string()),
            "Should contain 'async'"
        );

        // Test the filtering with documents
        let result = service
            .apply_logical_operators_to_documents(&query, documents)
            .await;
        assert!(result.is_ok(), "Filtering should not fail");

        let filtered_docs = result.unwrap();
        // Should match only document 2 (contains both "rust" and "async")
        assert_eq!(
            filtered_docs.len(),
            1,
            "Should match exactly 1 document for 'rust AND async'"
        );
        assert_eq!(
            filtered_docs[0].id, "2",
            "Should match the async Rust document"
        );
    }

    #[tokio::test]
    async fn test_get_all_terms_no_duplication_or_operator() {
        let mut service = setup_test_service().await;
        let documents = create_test_documents();

        // Test query: "rust OR python" - should match documents 1, 2, and 4
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("rust"),
            search_terms: Some(vec![
                NormalizedTermValue::from("rust"),
                NormalizedTermValue::from("python"),
            ]),
            operator: Some(LogicalOperator::Or),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        // Test get_all_terms to ensure no duplication
        let all_terms = query.get_all_terms();
        let terms_as_strings: Vec<String> =
            all_terms.iter().map(|t| t.as_str().to_string()).collect();

        assert_eq!(
            terms_as_strings.len(),
            2,
            "Should have exactly 2 terms, got: {:?}",
            terms_as_strings
        );
        assert!(
            terms_as_strings.contains(&"rust".to_string()),
            "Should contain 'rust'"
        );
        assert!(
            terms_as_strings.contains(&"python".to_string()),
            "Should contain 'python'"
        );

        // Test the filtering
        let result = service
            .apply_logical_operators_to_documents(&query, documents)
            .await;
        assert!(result.is_ok(), "Filtering should not fail");

        let filtered_docs = result.unwrap();
        // Should match documents 1, 2 (rust), and 4 (python)
        assert_eq!(
            filtered_docs.len(),
            3,
            "Should match exactly 3 documents for 'rust OR python'"
        );

        let matched_ids: Vec<&String> = filtered_docs.iter().map(|d| &d.id).collect();
        assert!(
            matched_ids.contains(&&"1".to_string()),
            "Should match document 1 (Rust programming)"
        );
        assert!(
            matched_ids.contains(&&"2".to_string()),
            "Should match document 2 (Async Rust)"
        );
        assert!(
            matched_ids.contains(&&"4".to_string()),
            "Should match document 4 (Python)"
        );
    }

    #[tokio::test]
    async fn test_single_term_query_backward_compatibility() {
        let mut service = setup_test_service().await;
        let _documents = create_test_documents();

        // Test single term query (no operator)
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("async"),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        // Test get_all_terms for single term
        let all_terms = query.get_all_terms();
        let terms_as_strings: Vec<String> =
            all_terms.iter().map(|t| t.as_str().to_string()).collect();

        assert_eq!(
            terms_as_strings.len(),
            1,
            "Single term query should have exactly 1 term"
        );
        assert_eq!(
            terms_as_strings[0], "async",
            "Should contain the single search term"
        );

        // Test search functionality
        let result = service.search(&query).await;
        assert!(result.is_ok(), "Single term search should not fail");

        let filtered_docs = result.unwrap();
        // In test environment might have no documents, just verify it doesn't crash
        println!(
            "Single term search returned {} documents",
            filtered_docs.documents.len()
        );
    }

    #[tokio::test]
    async fn test_word_boundary_matching() {
        let mut service = setup_test_service().await;

        // Create documents with partial word matches that should NOT match
        let documents = vec![
            Document {
                id: "1".to_string(),
                title: "JavaScript programming".to_string(),
                body: "Learn JavaScript fundamentals".to_string(),
                url: "http://example.com/js".to_string(),
                description: Some("JS guide".to_string()),
                rank: None,
                tags: None,
                summarization: None,
                stub: None,
            },
            Document {
                id: "2".to_string(),
                title: "Java programming".to_string(),
                body: "Object-oriented programming in Java language".to_string(),
                url: "http://example.com/java".to_string(),
                description: Some("Java tutorial".to_string()),
                rank: None,
                tags: None,
                summarization: None,
                stub: None,
            },
        ];

        // Search for "java" should match document 2 but NOT document 1 (JavaScript)
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("java"),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        let result = service
            .apply_logical_operators_to_documents(&query, documents)
            .await;
        assert!(result.is_ok(), "Word boundary filtering should not fail");

        let filtered_docs = result.unwrap();
        // Should match only document 2 (Java), not document 1 (JavaScript)
        assert_eq!(
            filtered_docs.len(),
            1,
            "Word boundary matching should be precise"
        );
        assert_eq!(
            filtered_docs[0].id, "2",
            "Should match only the Java document, not JavaScript"
        );
    }

    #[tokio::test]
    async fn test_multiple_terms_and_operator_strict_matching() {
        let mut service = setup_test_service().await;
        let documents = create_test_documents();

        // Test query with 3 terms: "rust AND async AND programming"
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("rust"),
            search_terms: Some(vec![
                NormalizedTermValue::from("rust"),
                NormalizedTermValue::from("async"),
                NormalizedTermValue::from("programming"),
            ]),
            operator: Some(LogicalOperator::And),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        // Test get_all_terms
        let all_terms = query.get_all_terms();
        assert_eq!(all_terms.len(), 3, "Should have exactly 3 terms");

        // Test the filtering - should match only document 2 (has all three terms)
        let result = service
            .apply_logical_operators_to_documents(&query, documents)
            .await;
        assert!(result.is_ok(), "Multi-term AND filtering should not fail");

        let filtered_docs = result.unwrap();
        assert_eq!(
            filtered_docs.len(),
            1,
            "Should match exactly 1 document with all 3 terms"
        );
        assert_eq!(
            filtered_docs[0].id, "2",
            "Should match the async Rust document"
        );
    }

    #[tokio::test]
    async fn test_multiple_terms_or_operator_inclusive_matching() {
        let mut service = setup_test_service().await;
        let documents = create_test_documents();

        // Test query: "rust OR javascript OR python"
        let query = SearchQuery {
            search_term: NormalizedTermValue::from("rust"),
            search_terms: Some(vec![
                NormalizedTermValue::from("rust"),
                NormalizedTermValue::from("javascript"),
                NormalizedTermValue::from("python"),
            ]),
            operator: Some(LogicalOperator::Or),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        // Test get_all_terms
        let all_terms = query.get_all_terms();
        assert_eq!(all_terms.len(), 3, "Should have exactly 3 terms");

        // Test the filtering - should match documents 1, 2, 3, 4
        let result = service
            .apply_logical_operators_to_documents(&query, documents)
            .await;
        assert!(result.is_ok(), "Multi-term OR filtering should not fail");

        let filtered_docs = result.unwrap();
        assert_eq!(
            filtered_docs.len(),
            4,
            "Should match all 4 documents (each contains at least one term)"
        );
    }
}
