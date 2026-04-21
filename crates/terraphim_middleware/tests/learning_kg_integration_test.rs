#![cfg(feature = "kg-integration")]

//! Integration tests for knowledge graph learning integration

use terraphim_middleware::learning_indexer::{index_learning, LearningIndexerConfig};
use terraphim_middleware::learning_query::query_with_learnings;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::shared_learning::{LearningSource, SharedLearning, TrustLevel};
use terraphim_types::{
    Layer, NormalizedTerm, NormalizedTermValue, RoleName, SearchQuery, Thesaurus,
};

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

    let query = SearchQuery {
        search_term: NormalizedTermValue::from("rust"),
        search_terms: None,
        operator: None,
        skip: None,
        limit: Some(10),
        role: Some(RoleName::new("test")),
        layer: Layer::default(),
        include_pinned: false,
    };

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

    let query = SearchQuery {
        search_term: NormalizedTermValue::from("rust"),
        search_terms: None,
        operator: None,
        skip: None,
        limit: Some(10),
        role: Some(RoleName::new("test")),
        layer: Layer::default(),
        include_pinned: false,
    };

    let results = query_with_learnings(&graph, &query, true);

    assert_eq!(results.len(), 2);
    // High quality should be first (higher rank)
    assert!(results[0].rank >= results[1].rank);
}
