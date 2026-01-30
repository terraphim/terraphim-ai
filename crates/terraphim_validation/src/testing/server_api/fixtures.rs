//! Test fixtures for server API testing
//!
//! This module provides test data and helper functions for creating
//! realistic test scenarios.

use ahash::AHashMap;
use terraphim_config::{Config, Role};
use terraphim_types::{ChatMessage, Document, NormalizedTermValue, RoleName, SearchQuery};

/// Test fixtures for API testing
pub struct TestFixtures;

impl TestFixtures {
    /// Create a sample document for testing
    pub fn sample_document() -> Document {
        Document {
            id: "test-doc-1".to_string(),
            url: "file:///test/doc1.md".to_string(),
            title: "Test Document".to_string(),
            body: "# Test Document\n\nThis is a test document for API validation.".to_string(),
            description: Some("A test document for validation".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["test".to_string(), "api".to_string()]),
            rank: Some(1),
            source_haystack: None,
        }
    }

    /// Create a large document for performance testing
    pub fn large_document() -> Document {
        let mut large_content = "# Large Test Document\n\n".to_string();
        for i in 0..1000 {
            large_content.push_str(&format!(
                "This is paragraph {} with some test content.\n\n",
                i
            ));
        }

        Document {
            id: "large-doc-1".to_string(),
            url: "file:///test/large.md".to_string(),
            title: "Large Test Document".to_string(),
            body: large_content,
            description: Some("A large document for performance testing".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["large".to_string(), "test".to_string()]),
            rank: Some(1),
            source_haystack: None,
        }
    }

    /// Create a search query for testing
    pub fn search_query(query: &str) -> SearchQuery {
        SearchQuery {
            search_term: NormalizedTermValue::from(query.to_string()),
            search_terms: None,
            operator: None,
            role: Some(RoleName::new("TestRole")),
            skip: Some(0),
            limit: Some(10),
        }
    }

    /// Create a test role configuration
    pub fn test_role_config() -> terraphim_config::Role {
        terraphim_config::Role {
            name: RoleName::new("TestRole"),
            shortname: Some("test".to_string()),
            theme: "default".to_string(),
            ..Default::default()
        }
    }

    /// Create a test configuration
    pub fn test_config() -> Config {
        let mut roles = AHashMap::new();
        roles.insert(RoleName::new("TestRole"), Self::test_role_config());

        Config {
            selected_role: RoleName::new("TestRole"),
            roles,
            ..Default::default()
        }
    }

    /// Create a chat message for testing
    pub fn chat_message(content: &str) -> ChatMessage {
        ChatMessage::user(content.to_string())
    }

    /// Create multiple sample documents
    pub fn sample_documents(count: usize) -> Vec<Document> {
        (0..count)
            .map(|i| Document {
                id: format!("test-doc-{}", i),
                url: format!("file:///test/doc{}.md", i),
                title: format!("Test Document {}", i),
                body: format!(
                    "# Test Document {}\n\nThis is test document number {}.",
                    i, i
                ),
                description: Some(format!("Test document {}", i)),
                summarization: None,
                stub: None,
                tags: Some(vec!["test".to_string(), format!("doc{}", i)]),
                rank: Some(1),
                source_haystack: None,
            })
            .collect()
    }

    /// Create a document with malicious content for security testing
    pub fn malicious_document() -> Document {
        Document {
            id: "malicious-doc-1".to_string(),
            url: "file:///test/malicious.md".to_string(),
            title: "<script>alert('xss')</script>".to_string(),
            body: "Document content with <script>alert('xss')</script> malicious content"
                .to_string(),
            description: Some("A document with malicious content".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["malicious".to_string(), "test".to_string()]),
            rank: Some(1),
            source_haystack: None,
        }
    }

    /// Create a document with special characters for edge case testing
    pub fn special_characters_document() -> Document {
        Document {
            id: "special-doc-1".to_string(),
            url: "file:///test/special.md".to_string(),
            title: "Special Characters Document".to_string(),
            body: "!@#$%^&*()_+-=[]{}|;':\",./<>?".to_string(),
            description: Some("Document with special characters".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["special".to_string(), "test".to_string()]),
            rank: Some(1),
            source_haystack: None,
        }
    }
}
