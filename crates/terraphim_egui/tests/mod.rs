//! Test utilities and helpers
//!
//! This module provides common utilities and helper functions used across
//! the test suite.

use std::sync::{Arc, Mutex};
use terraphim_egui::state::AppState;
use terraphim_types::Document;

/// Create a test document with default values
pub fn create_document(id: &str, title: &str, body: &str, rank: u64) -> Document {
    Document {
        id: id.to_string(),
        url: format!("https://example.com/{}", id),
        title: title.to_string(),
        body: body.to_string(),
        description: Some(format!("Description of {}", title)),
        summarization: None,
        stub: Some(format!("Stub of {}", title)),
        tags: Some(vec!["test".to_string(), "auto-generated".to_string()]),
        rank: Some(rank),
        source_haystack: Some("test-source".to_string()),
    }
}

/// Create a list of test documents with varying properties
pub fn create_test_document_set() -> Vec<Document> {
    vec![
        create_document("1", "Rust Programming Guide", "Complete guide to Rust", 100),
        create_document("2", "Egui UI Framework", "Modern UI framework for Rust", 95),
        create_document("3", "WebAssembly Tutorial", "Learn WebAssembly", 90),
        create_document(
            "4",
            "Async Programming",
            "Asynchronous programming patterns",
            85,
        ),
        create_document(
            "5",
            "Knowledge Graphs",
            "Understanding knowledge graphs",
            80,
        ),
    ]
}

/// Create test documents with specific tags
pub fn create_documents_with_tags(tags: Vec<&str>, count: usize) -> Vec<Document> {
    (0..count)
        .map(|i| {
            let tag_list = tags.iter().map(|s| s.to_string()).collect();
            Document {
                id: format!("tag-test-{}", i),
                url: format!("https://example.com/tag-test/{}", i),
                title: format!("Tagged Document {}", i),
                body: format!("Body of tagged document {}", i),
                description: Some(format!("Tagged document {}", i)),
                summarization: None,
                stub: None,
                tags: Some(tag_list),
                rank: Some(100 - i as u64),
                source_haystack: Some("tag-test-source".to_string()),
            }
        })
        .collect()
}

/// Create test documents with specific sources
pub fn create_documents_with_sources(sources: Vec<&str>) -> Vec<Document> {
    sources
        .iter()
        .enumerate()
        .map(|(i, source)| Document {
            id: format!("source-test-{}", i),
            url: format!("https://example.com/source-test/{}", i),
            title: format!("Document from {}", source),
            body: format!("Content from {}", source),
            description: Some(format!("Description from {}", source)),
            summarization: None,
            stub: None,
            tags: Some(vec!["source-test".to_string()]),
            rank: Some(90 - i as u64),
            source_haystack: Some(source.to_string()),
        })
        .collect()
}

/// Initialize a test AppState with mock data
pub fn setup_test_state() -> Arc<Mutex<AppState>> {
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));

    // Add some test data
    {
        let mut state = state_arc.lock().unwrap();
        let documents = create_test_document_set();
        state.set_search_results(documents);
    }

    state_arc
}

/// Create a large dataset for performance testing
pub fn create_large_document_set(size: usize) -> Vec<Document> {
    (0..size)
        .map(|i| {
            let content = format!("Content {}", i).repeat(10);
            Document {
                id: format!("large-{}", i),
                url: format!("https://example.com/large/{}", i),
                title: format!("Large Document {}", i),
                body: content,
                description: Some(format!("Description of large document {}", i)),
                summarization: None,
                stub: None,
                tags: Some(vec!["large".to_string(), "performance".to_string()]),
                rank: Some((size - i) as u64),
                source_haystack: Some("performance-test".to_string()),
            }
        })
        .collect()
}

/// Assert that two document lists are equal (ignoring order)
pub fn assert_documents_equal_unordered(docs1: &[Document], docs2: &[Document]) {
    assert_eq!(
        docs1.len(),
        docs2.len(),
        "Document lists should have the same length"
    );

    for doc1 in docs1 {
        assert!(
            docs2.iter().any(|doc2| doc2.id == doc1.id),
            "Document with ID {} not found in second list",
            doc1.id
        );
    }
}

/// Assert that a document list contains a specific document ID
pub fn assert_contains_document(docs: &[Document], id: &str) {
    assert!(
        docs.iter().any(|doc| doc.id == id),
        "Document list should contain document with ID {}",
        id
    );
}

/// Assert that a document list does not contain a specific document ID
pub fn assert_not_contains_document(docs: &[Document], id: &str) {
    assert!(
        !docs.iter().any(|doc| doc.id == id),
        "Document list should not contain document with ID {}",
        id
    );
}

/// Wait for a condition to become true (with timeout)
/// This is useful for testing async operations
pub async fn wait_for_condition<F, Fut>(mut check: F, max_wait_ms: u64)
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(max_wait_ms);

    while !check() {
        if start.elapsed() > timeout {
            panic!("Timeout waiting for condition after {}ms", max_wait_ms);
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

/// Simulate user input delay (for testing debounce behavior)
pub async fn simulate_user_input_delay() {
    // Simulate 100ms delay between keystrokes
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

/// Compare documents by a specific field
pub fn compare_documents_by<F, T: Ord>(docs: &mut [Document], compare_fn: F)
where
    F: Fn(&Document) -> T,
{
    // Sort in-place using the comparison function
    docs.sort_by(|a, b| compare_fn(a).cmp(&compare_fn(b)));
}

/// Get document IDs from a list
pub fn get_document_ids(docs: &[Document]) -> Vec<String> {
    docs.iter().map(|doc| doc.id.clone()).collect()
}

/// Check if all documents have required fields
pub fn validate_documents(docs: &[Document]) -> Result<(), String> {
    for (i, doc) in docs.iter().enumerate() {
        if doc.id.is_empty() {
            return Err(format!("Document at index {} has empty ID", i));
        }
        if doc.url.is_empty() {
            return Err(format!("Document {} has empty URL", doc.id));
        }
        if doc.title.is_empty() {
            return Err(format!("Document {} has empty title", doc.id));
        }
        if doc.rank.is_none() {
            return Err(format!("Document {} has no rank", doc.id));
        }
    }
    Ok(())
}
