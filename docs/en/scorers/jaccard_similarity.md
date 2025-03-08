# Jaccard Similarity Scorer

Jaccard similarity is a statistical measure used to gauge the similarity and diversity of sample sets. In Terraphim AI, it's implemented as a document scoring algorithm that measures the overlap between query terms and document terms.

## Overview

The Jaccard similarity coefficient is defined as the size of the intersection divided by the size of the union of two sets:

```
J(A,B) = |A ∩ B| / |A ∪ B|
```

For document search, this translates to:
- The intersection is the number of terms that appear in both the query and document
- The union is the total number of unique terms in both the query and document

The score always ranges from 0 to 1, where:
- 0 means no overlap (completely different)
- 1 means perfect overlap (identical)

## Implementation

The Jaccard similarity implementation in Terraphim AI follows this approach:

1. Tokenize the query and document into term sets
2. Calculate the intersection (terms present in both sets)
3. Calculate the union (all unique terms from both sets)
4. Compute the Jaccard coefficient as intersection/union

```rust
pub struct JaccardScorer {
    doc_count: usize,
}

impl JaccardScorer {
    pub fn new() -> Self {
        Self {
            doc_count: 0,
        }
    }

    pub fn initialize(&mut self, documents: &[Document]) {
        self.doc_count = documents.len();
    }

    pub fn score(&self, query: &str, doc: &Document) -> f64 {
        let query_terms: Vec<String> = query.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        if query_terms.is_empty() || self.doc_count == 0 {
            return 0.0;
        }
        
        let doc_terms: Vec<String> = doc.body.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        if doc_terms.is_empty() {
            return 0.0;
        }
        
        // Create sets for query and document terms
        let query_set: std::collections::HashSet<&String> = query_terms.iter().collect();
        let doc_set: std::collections::HashSet<&String> = doc_terms.iter().collect();
        
        // Calculate intersection size
        let intersection_size = query_set.intersection(&doc_set).count();
        
        // Calculate union size
        let union_size = query_set.union(&doc_set).count();
        
        // Jaccard similarity formula
        if union_size > 0 {
            intersection_size as f64 / union_size as f64
        } else {
            0.0
        }
    }
}
```

## When to Use Jaccard Similarity

Jaccard similarity is particularly useful when:

1. You want a simple, intuitive similarity measure
2. The presence/absence of terms is more important than their frequency
3. You need a normalized score between 0 and 1
4. You want to give equal weight to all terms

It's well-suited for:
- Comparing short texts or documents
- Measuring similarity between sets of tags or categories
- Finding documents with similar vocabulary, regardless of term frequency

## Testing and Validation

To ensure the Jaccard similarity implementation is working correctly, several test cases have been implemented:

### 1. Basic Validation Test

This test uses documents with predictable term overlap to verify that the Jaccard scores match expected values:

```rust
#[test]
fn test_validate_jaccard_similarity() {
    // Create test documents with predictable term overlap
    let documents = vec![
        Document {
            id: "doc1".to_string(),
            title: "apple banana cherry".to_string(),
            body: "apple banana cherry date".to_string(),
            // ...
        },
        Document {
            id: "doc2".to_string(),
            title: "apple banana".to_string(),
            body: "apple banana elderberry".to_string(),
            // ...
        },
        Document {
            id: "doc3".to_string(),
            title: "cherry date".to_string(),
            body: "cherry date fig".to_string(),
            // ...
        },
    ];
    
    // Initialize Jaccard scorer
    let mut jaccard_scorer = JaccardScorer::new();
    jaccard_scorer.initialize(&documents);
    
    // Test with query "apple banana"
    let query = "apple banana";
    let scores = jaccard_scorer.score_documents(query, &documents);
    
    // Verify scores match expected values
    // For doc1: intersection = 2, union = 4 => 2/4 = 0.5
    // For doc2: intersection = 2, union = 3 => 2/3 = 0.67
    // For doc3: intersection = 0, union = 5 => 0/5 = 0
    
    assert!(scores[0].1 >= 0.45 && scores[0].1 <= 0.55);
    assert!(scores[1].1 >= 0.6 && scores[1].1 <= 0.7);
    assert_eq!(scores[2].1, 0.0);
}
```

### 2. Edge Case Testing

This test verifies that the Jaccard implementation handles edge cases correctly:

```rust
#[test]
fn test_jaccard_edge_cases() {
    // Test with empty documents
    // Test with empty queries
    // Test with exact matches
    // Test with no overlap
}
```

### 3. Visualization Test

This test demonstrates how to manually calculate Jaccard similarity and compare it with the scorer's results:

```rust
#[test]
fn test_visualize_jaccard_similarity() {
    // For each document:
    // 1. Calculate query terms set
    // 2. Calculate document terms set
    // 3. Calculate intersection
    // 4. Calculate union
    // 5. Calculate Jaccard score
    // 6. Compare with scorer's result
}
```

## Running the Tests

To run the Jaccard similarity tests:

```bash
cargo test -p terraphim_service score::bm25_additional_test::tests::test_validate_jaccard_similarity -- --nocapture
cargo test -p terraphim_service score::bm25_additional_test::tests::test_jaccard_edge_cases -- --nocapture
cargo test -p terraphim_service score::bm25_additional_test::tests::test_visualize_jaccard_similarity -- --nocapture
```

## Usage in Terraphim AI

To use Jaccard similarity for document ranking in Terraphim AI:

```rust
let search_query = SearchQuery {
    search_term: NormalizedTermValue::new("your query".to_string()),
    skip: None,
    limit: None,
    role: Some(RoleName::new("your role")),
};

// Use the Jaccard scorer
let documents = score::rescore_documents(&search_query, documents, RelevanceFunction::Jaccard);
```

## Comparison with Other Scorers

| Scorer | Strengths | Weaknesses | When to Use |
|--------|-----------|------------|-------------|
| Jaccard | Simple, normalized (0-1), term presence/absence | Ignores term frequency, position | When term overlap is most important |
| BM25F | Field weighting, term frequency | More complex | When field importance varies |
| BM25+ | Handles rare terms well | More complex | When rare terms are important |
| TFIDF | Considers term frequency and rarity | Not normalized | For classic IR approach |
| QueryRatio | Simple, focuses on query coverage | Ignores document size | When query term coverage is key |

## References

1. Jaccard, P. (1901). Étude comparative de la distribution florale dans une portion des Alpes et des Jura. Bulletin de la Société Vaudoise des Sciences Naturelles, 37, 547-579.
2. Tan, P. N., Steinbach, M., & Kumar, V. (2005). Introduction to Data Mining. Addison-Wesley.
3. Manning, C. D., Raghavan, P., & Schütze, H. (2008). Introduction to Information Retrieval. Cambridge University Press. 