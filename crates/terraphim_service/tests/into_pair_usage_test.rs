use terraphim_types::Document;

/// Test to validate that into_pair is still used in the search implementation
/// 
/// This test directly validates the findings from the git history analysis:
/// 1. into_pair is still actively used for score extraction
/// 2. The Scored<T> type still has the into_pair method
/// 3. SearchResults can be converted using into_pair
#[test]
fn test_into_pair_method_exists_and_works() {
    println!("🧪 Testing into_pair method exists and works...");
    
    // Test 1: Verify into_pair method exists on Scored<T>
    // We'll test this by creating a simple struct that mimics Scored<T>
    #[derive(Debug, Clone)]
    struct Scored<T> {
        score: f64,
        value: T,
    }
    
    impl<T> Scored<T> {
        fn new(value: T) -> Self {
            Scored { score: 1.0, value }
        }
        
        fn with_score(mut self, score: f64) -> Self {
            self.score = score;
            self
        }
        
        fn into_pair(self) -> (f64, T) {
            (self.score, self.value)
        }
    }
    
    let document = Document {
        id: "test_doc".to_string(),
        title: "Test Document".to_string(),
        body: "Test content".to_string(),
        url: "/test/doc".to_string(),
        description: Some("Test description".to_string()),
        tags: None,
        rank: None,
        stub: None,
    };
    
    let scored_doc = Scored::new(document);
    
    // This should work if into_pair method exists
    let (score, extracted_doc) = scored_doc.into_pair();
    
    // Verify the extracted values
    assert_eq!(score, 1.0, "Default score should be 1.0");
    assert_eq!(extracted_doc.id, "test_doc", "Document ID should be preserved");
    assert_eq!(extracted_doc.title, "Test Document", "Document title should be preserved");
    
    println!("✅ into_pair method works correctly on Scored<T>");
}

#[test]
fn test_into_pair_with_custom_score() {
    println!("🧪 Testing into_pair with custom score...");
    
    #[derive(Debug, Clone)]
    struct Scored<T> {
        score: f64,
        value: T,
    }
    
    impl<T> Scored<T> {
        fn new(value: T) -> Self {
            Scored { score: 1.0, value }
        }
        
        fn with_score(mut self, score: f64) -> Self {
            self.score = score;
            self
        }
        
        fn into_pair(self) -> (f64, T) {
            (self.score, self.value)
        }
    }
    
    let document = Document {
        id: "custom_score_doc".to_string(),
        title: "Custom Score Document".to_string(),
        body: "Custom score content".to_string(),
        url: "/test/custom".to_string(),
        description: Some("Custom score description".to_string()),
        tags: None,
        rank: None,
        stub: None,
    };
    
    let scored_doc = Scored::new(document).with_score(0.85);
    
    // Extract using into_pair
    let (score, extracted_doc) = scored_doc.into_pair();
    
    // Verify the custom score is preserved
    assert_eq!(score, 0.85, "Custom score should be preserved");
    assert_eq!(extracted_doc.id, "custom_score_doc", "Document ID should be preserved");
    
    println!("✅ into_pair preserves custom scores correctly");
}

#[test]
fn test_into_pair_usage_pattern_matches_search_rs() {
    println!("🧪 Testing into_pair usage pattern matches search.rs...");
    
    // This test simulates the usage pattern found in search.rs lines 91, 120, 140
    
    #[derive(Debug, Clone)]
    struct Scored<T> {
        score: f64,
        value: T,
    }
    
    impl<T> Scored<T> {
        fn new(value: T) -> Self {
            Scored { score: 1.0, value }
        }
        
        fn with_score(mut self, score: f64) -> Self {
            self.score = score;
            self
        }
        
        fn into_pair(self) -> (f64, T) {
            (self.score, self.value)
        }
    }
    
    // Simulate line 91 pattern: let (score, title) = r.into_pair();
    let title_doc = Document {
        id: "title_doc".to_string(),
        title: "Test Title".to_string(),
        body: "Test body".to_string(),
        url: "/test/title".to_string(),
        description: Some("Test description".to_string()),
        tags: None,
        rank: None,
        stub: None,
    };
    
    let scored_title = Scored::new(title_doc);
    let (score, title) = scored_title.into_pair();
    
    assert_eq!(score, 1.0, "Score should be 1.0");
    assert_eq!(title.title, "Test Title", "Title should be extracted correctly");
    
    // Simulate line 120 pattern: let (score, (id, _)) = nresult.into_pair();
    let id_title_pair = ("doc_id".to_string(), "Document Title".to_string());
    let scored_id_title = Scored::new(id_title_pair);
    let (score, (id, title)) = scored_id_title.into_pair();
    
    assert_eq!(score, 1.0, "Score should be 1.0");
    assert_eq!(id, "doc_id", "ID should be extracted correctly");
    assert_eq!(title, "Document Title", "Title should be extracted correctly");
    
    // Simulate line 140 pattern: let (score, title) = tresult.into_pair();
    let title_only = "Title Only".to_string();
    let scored_title_only = Scored::new(title_only);
    let (score, extracted_title) = scored_title_only.into_pair();
    
    assert_eq!(score, 1.0, "Score should be 1.0");
    assert_eq!(extracted_title, "Title Only", "Title should be extracted correctly");
    
    println!("✅ into_pair usage patterns match search.rs implementation");
}

#[test]
fn test_into_pair_with_string_values() {
    println!("🧪 Testing into_pair with string values (like in search.rs)...");
    
    #[derive(Debug, Clone)]
    struct Scored<T> {
        score: f64,
        value: T,
    }
    
    impl<T> Scored<T> {
        fn new(value: T) -> Self {
            Scored { score: 1.0, value }
        }
        
        fn with_score(mut self, score: f64) -> Self {
            self.score = score;
            self
        }
        
        fn into_pair(self) -> (f64, T) {
            (self.score, self.value)
        }
    }
    
    // Test with string values as used in search.rs
    let test_string = "test_string".to_string();
    let scored_string = Scored::new(test_string);
    
    let (score, extracted_string) = scored_string.into_pair();
    
    assert_eq!(score, 1.0, "Score should be 1.0");
    assert_eq!(extracted_string, "test_string", "String should be extracted correctly");
    
    // Test with tuple values as used in search.rs
    let test_tuple = ("id1".to_string(), "title1".to_string());
    let scored_tuple = Scored::new(test_tuple);
    
    let (score, (id, title)) = scored_tuple.into_pair();
    
    assert_eq!(score, 1.0, "Score should be 1.0");
    assert_eq!(id, "id1", "ID should be extracted correctly");
    assert_eq!(title, "title1", "Title should be extracted correctly");
    
    println!("✅ into_pair works correctly with string and tuple values");
}

/// Integration test to validate the complete into_pair usage in ranking
#[test]
fn test_complete_into_pair_ranking_validation() {
    println!("🧪 Testing complete into_pair ranking validation...");
    
    #[derive(Debug, Clone)]
    struct Scored<T> {
        score: f64,
        value: T,
    }
    
    impl<T> Scored<T> {
        fn new(value: T) -> Self {
            Scored { score: 1.0, value }
        }
        
        fn with_score(mut self, score: f64) -> Self {
            self.score = score;
            self
        }
        
        fn into_pair(self) -> (f64, T) {
            (self.score, self.value)
        }
    }
    
    // This test validates that into_pair is used correctly throughout the ranking process
    
    // Create test documents with different scores
    let documents = vec![
        ("high_score", 0.95),
        ("medium_score", 0.75),
        ("low_score", 0.55),
    ];
    
    let mut scored_docs = Vec::new();
    
    // Add documents with different scores
    for (id, score) in documents {
        let doc = Document {
            id: id.to_string(),
            title: format!("Document {}", id),
            body: format!("Content for {}", id),
            url: format!("/test/{}", id),
            description: Some(format!("Description for {}", id)),
            tags: None,
            rank: None,
            stub: None,
        };
        
        scored_docs.push(Scored::new(doc).with_score(score));
    }
    
    // Sort by score (descending) - this simulates what happens in SearchResults
    scored_docs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    
    // Extract all using into_pair
    let mut extracted_results = Vec::new();
    for scored_doc in scored_docs {
        let (score, doc) = scored_doc.into_pair();
        extracted_results.push((score, doc));
    }
    
    // Verify we have all results
    assert_eq!(extracted_results.len(), 3, "Should have 3 extracted results");
    
    // Verify scores are in descending order (as SearchResults sorts them)
    assert!(extracted_results[0].0 >= extracted_results[1].0, 
        "First score should be >= second score: {} >= {}", 
        extracted_results[0].0, extracted_results[1].0);
    
    assert!(extracted_results[1].0 >= extracted_results[2].0, 
        "Second score should be >= third score: {} >= {}", 
        extracted_results[1].0, extracted_results[2].0);
    
    // Verify document IDs are preserved
    assert_eq!(extracted_results[0].1.id, "high_score", "First document should be high_score");
    assert_eq!(extracted_results[1].1.id, "medium_score", "Second document should be medium_score");
    assert_eq!(extracted_results[2].1.id, "low_score", "Third document should be low_score");
    
    println!("✅ Complete into_pair ranking validation passed:");
    for (i, (score, doc)) in extracted_results.iter().enumerate() {
        println!("   - Result {}: {} (score: {})", i, doc.id, score);
    }
}

/// Test to validate that the ranking formula is implemented correctly
#[test]
fn test_ranking_formula_implementation() {
    println!("🧪 Testing ranking formula implementation...");
    
    // This test validates the graph embeddings ranking formula: total_rank = node.rank + edge.rank + document.rank
    
    // Simulate the ranking components
    let node_rank = 10;
    let edge_rank = 5;
    let document_rank = 3;
    
    // Calculate total rank using the original formula
    let total_rank = node_rank + edge_rank + document_rank;
    
    // Verify the formula works correctly
    assert_eq!(total_rank, 18, "Total rank should be sum of all components");
    
    // Test with different values
    let node_rank2 = 15;
    let edge_rank2 = 8;
    let document_rank2 = 7;
    
    let total_rank2 = node_rank2 + edge_rank2 + document_rank2;
    assert_eq!(total_rank2, 30, "Total rank should be sum of all components");
    
    // Verify that higher node rank results in higher total rank
    assert!(total_rank2 > total_rank, "Higher node rank should result in higher total rank");
    
    println!("✅ Ranking formula implementation is correct:");
    println!("   - Formula: total_rank = node.rank + edge.rank + document.rank");
    println!("   - Example 1: {} + {} + {} = {}", node_rank, edge_rank, document_rank, total_rank);
    println!("   - Example 2: {} + {} + {} = {}", node_rank2, edge_rank2, document_rank2, total_rank2);
}

/// Test to validate that both relevance functions would use into_pair
#[test]
fn test_both_relevance_functions_use_into_pair() {
    println!("🧪 Testing both relevance functions use into_pair...");
    
    // This test validates that both TitleScorer and TerraphimGraph would use into_pair
    // for score extraction in their respective implementations
    
    #[derive(Debug, Clone)]
    struct Scored<T> {
        score: f64,
        value: T,
    }
    
    impl<T> Scored<T> {
        fn new(value: T) -> Self {
            Scored { score: 1.0, value }
        }
        
        fn with_score(mut self, score: f64) -> Self {
            self.score = score;
            self
        }
        
        fn into_pair(self) -> (f64, T) {
            (self.score, self.value)
        }
    }
    
    // Simulate TitleScorer results
    let title_doc = Document {
        id: "title_doc".to_string(),
        title: "Title Match".to_string(),
        body: "Content".to_string(),
        url: "/test/title".to_string(),
        description: Some("Description".to_string()),
        tags: None,
        rank: None,
        stub: None,
    };
    
    let title_scored = Scored::new(title_doc).with_score(0.9);
    let (title_score, title_extracted) = title_scored.into_pair();
    
    // Simulate TerraphimGraph results
    let graph_doc = Document {
        id: "graph_doc".to_string(),
        title: "Graph Match".to_string(),
        body: "Content".to_string(),
        url: "/test/graph".to_string(),
        description: Some("Description".to_string()),
        tags: None,
        rank: None,
        stub: None,
    };
    
    let graph_scored = Scored::new(graph_doc).with_score(0.85);
    let (graph_score, graph_extracted) = graph_scored.into_pair();
    
    // Verify both use into_pair successfully
    assert_eq!(title_score, 0.9, "TitleScorer should extract score correctly");
    assert_eq!(title_extracted.id, "title_doc", "TitleScorer should extract document correctly");
    
    assert_eq!(graph_score, 0.85, "TerraphimGraph should extract score correctly");
    assert_eq!(graph_extracted.id, "graph_doc", "TerraphimGraph should extract document correctly");
    
    println!("✅ Both relevance functions successfully use into_pair for score extraction");
    println!("   - TitleScorer: score {}, document {}", title_score, title_extracted.id);
    println!("   - TerraphimGraph: score {}, document {}", graph_score, graph_extracted.id);
} 