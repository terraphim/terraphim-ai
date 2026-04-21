#![cfg(all(feature = "kg-integration", feature = "feedback-loop"))]

//! Integration tests for knowledge graph learning integration

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use terraphim_middleware::feedback_loop::{
    record_graph_query, record_learning_application, FeedbackConfig, GraphTouchStore,
};
use terraphim_middleware::learning_indexer::{index_learning, LearningIndexerConfig};
use terraphim_middleware::learning_query::query_with_learnings;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::shared_learning::{LearningSource, SharedLearning, TrustLevel};
use terraphim_types::Document;
use terraphim_types::{
    Layer, NormalizedTerm, NormalizedTermValue, RoleName, SearchQuery, Thesaurus,
};

#[derive(Clone, Default)]
struct MockTouchStore {
    touched: Arc<Mutex<Vec<String>>>,
}

impl MockTouchStore {
    fn touched_ids(&self) -> Vec<String> {
        self.touched.lock().unwrap().clone()
    }
}

impl GraphTouchStore for MockTouchStore {
    fn record_graph_touch<'a>(
        &'a self,
        learning_id: &'a str,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), terraphim_types::shared_learning::StoreError>>
                + Send
                + 'a,
        >,
    > {
        let touched = self.touched.clone();
        let id = learning_id.to_string();
        Box::pin(async move {
            touched.lock().unwrap().push(id);
            Ok(())
        })
    }
}

fn create_test_graph() -> RoleGraph {
    let mut thesaurus = Thesaurus::new("test".to_string());
    thesaurus.insert(
        NormalizedTermValue::from("rust"),
        NormalizedTerm::with_auto_id(NormalizedTermValue::from("rust programming language")),
    );
    thesaurus.insert(
        NormalizedTermValue::from("error"),
        NormalizedTerm::with_auto_id(NormalizedTermValue::from("error handling")),
    );

    RoleGraph::new_sync(RoleName::new("test"), thesaurus).unwrap()
}

fn create_test_learning(
    title: &str,
    keywords: Vec<&str>,
    trust_level: TrustLevel,
) -> SharedLearning {
    let mut learning = SharedLearning::new(
        title.to_string(),
        format!("Content for {}", title),
        LearningSource::Manual,
        "test-agent".to_string(),
    );
    learning.keywords = keywords.iter().map(|k| k.to_string()).collect();
    learning.trust_level = trust_level;
    learning
}

fn create_query(term: &str) -> SearchQuery {
    SearchQuery {
        search_term: NormalizedTermValue::from(term),
        search_terms: None,
        operator: None,
        skip: None,
        limit: Some(10),
        role: Some(RoleName::new("test")),
        layer: Layer::default(),
        include_pinned: false,
    }
}

#[test]
fn test_learning_indexing() {
    let mut graph = create_test_graph();
    let learning =
        create_test_learning("Rust Error Handling", vec!["rust", "error"], TrustLevel::L2);

    let config = LearningIndexerConfig::default();
    let result = index_learning(&mut graph, &learning, &config);

    assert!(result.is_ok());
    let indexed = result.unwrap();
    assert_eq!(indexed.id, learning.id);
    assert!(indexed.rank > 0);
}

#[test]
fn test_l1_learning_filtered_by_default() {
    let mut graph = create_test_graph();
    let learning = create_test_learning("Rust Basics", vec!["rust"], TrustLevel::L1);

    let config = LearningIndexerConfig::default();
    let result = index_learning(&mut graph, &learning, &config);

    assert!(result.is_err());
}

#[test]
fn test_learning_appears_in_query() {
    let mut graph = create_test_graph();
    let learning =
        create_test_learning("Rust Error Patterns", vec!["rust", "error"], TrustLevel::L2);

    let config = LearningIndexerConfig::default();
    index_learning(&mut graph, &learning, &config).unwrap();

    let query = create_query("rust");

    let results = query_with_learnings(&graph, &query, true);

    assert!(!results.is_empty());
    assert!(results.iter().any(|doc| doc.id == learning.id));
}

#[test]
fn test_learning_ranked_by_quality() {
    let mut graph = create_test_graph();

    let mut low_quality =
        create_test_learning("Low Quality Learning", vec!["rust"], TrustLevel::L2);
    low_quality.quality.success_rate = Some(0.3);

    let mut high_quality =
        create_test_learning("High Quality Learning", vec!["rust"], TrustLevel::L2);
    high_quality.quality.success_rate = Some(0.9);

    let config = LearningIndexerConfig::default();
    index_learning(&mut graph, &low_quality, &config).unwrap();
    index_learning(&mut graph, &high_quality, &config).unwrap();

    let query = create_query("rust");

    let results = query_with_learnings(&graph, &query, true);

    assert_eq!(results.len(), 2);
    // High quality should be first (higher rank)
    assert!(results[0].rank >= results[1].rank);
}

#[test]
fn test_query_with_learnings_preserves_regular_documents() {
    let mut graph = create_test_graph();
    let learning = create_test_learning("Rust Learning", vec!["rust"], TrustLevel::L2);
    index_learning(&mut graph, &learning, &LearningIndexerConfig::default()).unwrap();

    let regular = Document {
        id: "regular-rust-doc".to_string(),
        url: "file:///tmp/rust.md".to_string(),
        title: "Rust Guide".to_string(),
        body: "rust error handling guide".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: Some(vec!["rust".to_string(), "error".to_string()]),
        rank: Some(50),
        source_haystack: None,
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    };
    let regular_id = regular.id.clone();
    graph.insert_document(&regular_id, regular);

    let query = create_query("rust");
    let results = query_with_learnings(&graph, &query, true);
    assert!(results.iter().any(|doc| doc.id == learning.id));
    assert!(results.iter().any(|doc| doc.id == "regular-rust-doc"));

    let results_without_learnings = query_with_learnings(&graph, &query, false);
    assert!(results_without_learnings
        .iter()
        .all(|doc| doc.id != learning.id));
    assert!(results_without_learnings
        .iter()
        .any(|doc| doc.id == "regular-rust-doc"));
}

#[test]
fn test_feedback_loop_only_touches_learning_documents_and_updates_rank() {
    let mut graph = create_test_graph();
    let learning = create_test_learning("Rust Learning", vec!["rust"], TrustLevel::L2);
    let indexed = index_learning(&mut graph, &learning, &LearningIndexerConfig::default()).unwrap();

    let regular = Document {
        id: "regular-rust-doc".to_string(),
        url: "file:///tmp/rust.md".to_string(),
        title: "Rust Guide".to_string(),
        body: "rust error book".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: Some(vec!["rust".to_string(), "error".to_string()]),
        rank: Some(20),
        source_haystack: None,
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    };
    let regular_id = regular.id.clone();
    graph.insert_document(&regular_id, regular);

    let store = MockTouchStore::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime
        .block_on(record_graph_query(
            &graph,
            &store,
            "rust",
            &FeedbackConfig::default(),
        ))
        .unwrap();

    assert_eq!(store.touched_ids(), vec![learning.id.clone()]);

    let original_rank = graph.get_document(&indexed.id).unwrap().rank;
    runtime
        .block_on(record_learning_application(
            &mut graph,
            &store,
            &indexed.id,
            true,
            &FeedbackConfig::default(),
        ))
        .unwrap();
    let boosted_rank = graph.get_document(&indexed.id).unwrap().rank;
    assert!(boosted_rank > original_rank);
}
