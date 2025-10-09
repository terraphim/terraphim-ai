use crate::score::bm25::{BM25FScorer, BM25PlusScorer};
use crate::score::bm25_additional::{
    JaccardScorer, OkapiBM25Scorer, QueryRatioScorer, TFIDFScorer,
};
use crate::score::common::{BM25Params, FieldWeights};
use terraphim_types::Document;

fn create_test_documents() -> Vec<Document> {
    vec![
        Document {
            id: "doc1".to_string(),
            title: "Introduction to Rust Programming".to_string(),
            body: "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety.".to_string(),
            description: Some("A comprehensive guide to Rust programming language".to_string()),
            summarization: None,
            tags: Some(vec!["programming".to_string(), "rust".to_string(), "systems".to_string()]),
            rank: None,
            stub: None,
            url: "https://example.com/doc1".to_string(),
            source_haystack: None,
        },
        Document {
            id: "doc2".to_string(),
            title: "Advanced Rust Concepts".to_string(),
            body: "This document covers advanced Rust concepts including ownership, borrowing, and lifetimes.".to_string(),
            description: Some("Deep dive into advanced Rust programming concepts".to_string()),
            summarization: None,
            tags: Some(vec!["rust".to_string(), "advanced".to_string(), "ownership".to_string()]),
            rank: None,
            stub: None,
            url: "https://example.com/doc2".to_string(),
            source_haystack: None,
        },
        Document {
            id: "doc3".to_string(),
            title: "Systems Programming with Rust".to_string(),
            body: "Systems programming requires careful memory management and performance optimization.".to_string(),
            description: Some("Guide to systems programming using Rust".to_string()),
            summarization: None,
            tags: Some(vec!["systems".to_string(), "programming".to_string(), "performance".to_string()]),
            rank: None,
            stub: None,
            url: "https://example.com/doc3".to_string(),
            source_haystack: None,
        },
    ]
}

#[test]
fn test_bm25_scorer_basic_functionality() {
    let documents = create_test_documents();
    let mut bm25_scorer = OkapiBM25Scorer::new();
    bm25_scorer.initialize(&documents);

    let query = "rust programming";
    let scores: Vec<f64> = documents
        .iter()
        .map(|doc| bm25_scorer.score(query, doc))
        .collect();

    // All documents should have positive scores for "rust programming"
    assert!(scores.iter().all(|&score| score >= 0.0));

    // Document 1 should have the highest score as it contains both "rust" and "programming"
    assert!(scores[0] > scores[2]);
    assert!(scores[0] > scores[1]);
}

#[test]
fn test_bm25f_scorer_field_weights() {
    let documents = create_test_documents();
    let weights = FieldWeights {
        title: 2.0,
        body: 1.0,
        description: 1.5,
        tags: 0.5,
    };
    let params = BM25Params {
        k1: 1.2,
        b: 0.75,
        delta: 1.0,
    };

    let mut bm25f_scorer = BM25FScorer::with_params(params, weights);
    bm25f_scorer.initialize(&documents);

    let query = "rust";
    let scores: Vec<f64> = documents
        .iter()
        .map(|doc| bm25f_scorer.score(query, doc))
        .collect();

    // All documents should have positive scores
    assert!(scores.iter().all(|&score| score >= 0.0));

    // Document with "rust" in title should score higher than document with "rust" only in body
    assert!(scores[0] > scores[2]);
}

#[test]
fn test_bm25plus_scorer_enhanced_parameters() {
    let documents = create_test_documents();
    let params = BM25Params {
        k1: 1.5,
        b: 0.8,
        delta: 1.2,
    };

    let mut bm25plus_scorer = BM25PlusScorer::with_params(params);
    bm25plus_scorer.initialize(&documents);

    let query = "systems programming";
    let scores: Vec<f64> = documents
        .iter()
        .map(|doc| bm25plus_scorer.score(query, doc))
        .collect();

    // All documents should have positive scores
    assert!(scores.iter().all(|&score| score >= 0.0));

    // Document 3 should have the highest score as it contains both "systems" and "programming"
    assert!(scores[2] > scores[0]);
    assert!(scores[2] > scores[1]);
}

#[test]
fn test_tfidf_scorer_traditional_approach() {
    let documents = create_test_documents();
    let mut tfidf_scorer = TFIDFScorer::new();
    tfidf_scorer.initialize(&documents);

    let query = "rust";
    let scores: Vec<f64> = documents
        .iter()
        .map(|doc| tfidf_scorer.score(query, doc))
        .collect();

    // All documents should have positive scores
    assert!(scores.iter().all(|&score| score >= 0.0));

    // Documents with "rust" should have higher scores than those without
    assert!(scores[0] > 0.0);
    assert!(scores[1] > 0.0);
}

#[test]
fn test_jaccard_scorer_similarity_based() {
    let documents = create_test_documents();
    let mut jaccard_scorer = JaccardScorer::new();
    jaccard_scorer.initialize(&documents);

    let query = "rust programming";
    let scores: Vec<f64> = documents
        .iter()
        .map(|doc| jaccard_scorer.score(query, doc))
        .collect();

    // All scores should be between 0.0 and 1.0 (Jaccard similarity)
    assert!(scores.iter().all(|&score| (0.0..=1.0).contains(&score)));

    // Document 1 should have the highest similarity as it contains both terms
    assert!(scores[0] > scores[2]);
}

#[test]
fn test_query_ratio_scorer_term_matching() {
    let documents = create_test_documents();
    let mut query_ratio_scorer = QueryRatioScorer::new();
    query_ratio_scorer.initialize(&documents);

    let query = "rust systems";
    let scores: Vec<f64> = documents
        .iter()
        .map(|doc| query_ratio_scorer.score(query, doc))
        .collect();

    // All scores should be between 0.0 and 1.0 (ratio of matched terms)
    assert!(scores.iter().all(|&score| (0.0..=1.0).contains(&score)));

    // Document 1 should have the highest ratio as it contains both "rust" and "systems"
    assert!(scores[0] > scores[1]);
    assert!(scores[0] > scores[2]);
}

#[test]
fn test_scorer_initialization_with_empty_documents() {
    let empty_documents: Vec<Document> = vec![];

    let mut bm25_scorer = OkapiBM25Scorer::new();
    bm25_scorer.initialize(&empty_documents);

    let mut bm25f_scorer = BM25FScorer::new();
    bm25f_scorer.initialize(&empty_documents);

    let mut bm25plus_scorer = BM25PlusScorer::new();
    bm25plus_scorer.initialize(&empty_documents);

    // Should not panic with empty documents
    // Note: We can't access private fields, so we just verify initialization doesn't panic
    // Test passes by not panicking during initialization
}

#[test]
fn test_scorer_empty_query_handling() {
    let documents = create_test_documents();
    let mut bm25_scorer = OkapiBM25Scorer::new();
    bm25_scorer.initialize(&documents);

    let empty_query = "";
    let scores: Vec<f64> = documents
        .iter()
        .map(|doc| bm25_scorer.score(empty_query, doc))
        .collect();

    // Empty query should return 0.0 scores
    assert!(scores.iter().all(|&score| score == 0.0));
}

#[test]
fn test_scorer_case_insensitive_matching() {
    let documents = create_test_documents();
    let mut bm25_scorer = OkapiBM25Scorer::new();
    bm25_scorer.initialize(&documents);

    let query_lower = "rust programming";
    let query_upper = "RUST PROGRAMMING";

    let scores_lower: Vec<f64> = documents
        .iter()
        .map(|doc| bm25_scorer.score(query_lower, doc))
        .collect();

    let scores_upper: Vec<f64> = documents
        .iter()
        .map(|doc| bm25_scorer.score(query_upper, doc))
        .collect();

    // Case should not affect scores significantly
    for (lower, upper) in scores_lower.iter().zip(scores_upper.iter()) {
        assert!((lower - upper).abs() < 0.001);
    }
}

#[test]
fn test_scorer_parameter_sensitivity() {
    let documents = create_test_documents();
    let query = "rust programming";

    // Test different k1 values
    let params_low_k1 = BM25Params {
        k1: 0.5,
        b: 0.75,
        delta: 1.0,
    };
    let params_high_k1 = BM25Params {
        k1: 2.0,
        b: 0.75,
        delta: 1.0,
    };

    let mut scorer_low = BM25FScorer::with_params(params_low_k1, FieldWeights::default());
    let mut scorer_high = BM25FScorer::with_params(params_high_k1, FieldWeights::default());

    scorer_low.initialize(&documents);
    scorer_high.initialize(&documents);

    let scores_low: Vec<f64> = documents
        .iter()
        .map(|doc| scorer_low.score(query, doc))
        .collect();

    let scores_high: Vec<f64> = documents
        .iter()
        .map(|doc| scorer_high.score(query, doc))
        .collect();

    // Different k1 values should produce different scores
    assert_ne!(scores_low, scores_high);
}
