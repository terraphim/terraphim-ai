#[cfg(test)]
mod tests {
    use super::super::bm25::{BM25FScorer, BM25PlusScorer};
    use super::super::bm25_additional::{OkapiBM25Scorer, TFIDFScorer, JaccardScorer, QueryRatioScorer};
    use terraphim_types::Document;
    use std::collections::HashSet;

    // Test documents for all tests
    fn get_test_documents() -> Vec<Document> {
        vec![
            Document {
                id: "1".to_string(),
                url: "http://example.com/1".to_string(),
                title: "Rust Programming Language".to_string(),
                body: "Rust is a systems programming language focused on safety, speed, and concurrency.".to_string(),
                description: Some("Learn about Rust programming".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string(), "systems".to_string()]),
                rank: None,
            },
            Document {
                id: "2".to_string(),
                url: "http://example.com/2".to_string(),
                title: "Python Programming Tutorial".to_string(),
                body: "Python is a high-level programming language known for its readability.".to_string(),
                description: Some("Learn Python programming".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string(), "tutorial".to_string()]),
                rank: None,
            },
            Document {
                id: "3".to_string(),
                url: "http://example.com/3".to_string(),
                title: "JavaScript for Web Development".to_string(),
                body: "JavaScript is a scripting language that enables interactive web pages.".to_string(),
                description: Some("Learn JavaScript for web development".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string(), "web".to_string()]),
                rank: None,
            },
        ]
    }

    #[test]
    fn test_compare_bm25plus_with_okapi_bm25() {
        let documents = get_test_documents();
        
        // Initialize BM25+ scorer
        let mut bm25plus_scorer = BM25PlusScorer::new();
        bm25plus_scorer.initialize(&documents);
        
        // Initialize Okapi BM25 scorer
        let mut okapi_bm25_scorer = OkapiBM25Scorer::new();
        okapi_bm25_scorer.initialize(&documents);
        
        // Test queries
        let queries = vec![
            "rust programming",
            "python tutorial",
            "javascript web",
            "programming language",
        ];
        
        for query in queries {
            println!("Query: {}", query);
            
            // Score documents with BM25+
            let mut bm25plus_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = bm25plus_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Sort by score in descending order
            bm25plus_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            // Score documents with Okapi BM25
            let mut okapi_bm25_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = okapi_bm25_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Sort by score in descending order
            okapi_bm25_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            println!("BM25+ ranking: {:?}", bm25plus_scores);
            println!("Okapi BM25 ranking: {:?}", okapi_bm25_scores);
            
            // Check if the top document is the same for both scorers
            // This is a basic validation that the scorers are producing similar results
            assert_eq!(
                bm25plus_scores.first().unwrap().0,
                okapi_bm25_scores.first().unwrap().0,
                "Top document should be the same for BM25+ and Okapi BM25 for query: {}",
                query
            );
        }
    }

    #[test]
    fn test_compare_bm25f_with_tfidf() {
        let documents = get_test_documents();
        
        // Initialize BM25F scorer
        let mut bm25f_scorer = BM25FScorer::new();
        bm25f_scorer.initialize(&documents);
        
        // Initialize TFIDF scorer
        let mut tfidf_scorer = TFIDFScorer::new();
        tfidf_scorer.initialize(&documents);
        
        // Test queries
        let queries = vec![
            "rust programming",
            "python tutorial",
            "javascript web",
            "programming language",
        ];
        
        for query in queries {
            println!("Query: {}", query);
            
            // Score documents with BM25F
            let mut bm25f_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = bm25f_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Sort by score in descending order
            bm25f_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            // Score documents with TFIDF
            let mut tfidf_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = tfidf_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Sort by score in descending order
            tfidf_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            println!("BM25F ranking: {:?}", bm25f_scores);
            println!("TFIDF ranking: {:?}", tfidf_scores);
            
            // We don't assert equality here because BM25F and TFIDF can produce different rankings
            // Instead, we just print the rankings for manual inspection
        }
    }

    #[test]
    fn test_jaccard_scorer() {
        let documents = get_test_documents();
        
        // Initialize Jaccard scorer
        let mut jaccard_scorer = JaccardScorer::new();
        jaccard_scorer.initialize(&documents);
        
        // Test queries
        let queries = vec![
            "rust programming",
            "python tutorial",
            "javascript web",
            "programming language",
        ];
        
        for query in queries {
            println!("Query: {}", query);
            
            // Score documents with Jaccard
            let mut jaccard_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = jaccard_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Sort by score in descending order
            jaccard_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            println!("Jaccard ranking: {:?}", jaccard_scores);
            
            // Verify that scores are between 0 and 1
            for (_, score) in &jaccard_scores {
                assert!(*score >= 0.0 && *score <= 1.0, "Jaccard score should be between 0 and 1");
            }
            
            // Verify that the top document contains at least one of the query terms
            let top_doc_id = jaccard_scores.first().unwrap().0.clone();
            let top_doc = documents.iter().find(|doc| doc.id == top_doc_id).unwrap();
            
            let query_terms: Vec<&str> = query.split_whitespace().collect();
            let doc_contains_query_term = query_terms.iter().any(|term| {
                top_doc.body.to_lowercase().contains(&term.to_lowercase()) ||
                top_doc.title.to_lowercase().contains(&term.to_lowercase())
            });
            
            assert!(doc_contains_query_term, "Top document should contain at least one query term");
        }
    }

    #[test]
    fn test_query_ratio_scorer() {
        let documents = get_test_documents();
        
        // Initialize QueryRatio scorer
        let mut query_ratio_scorer = QueryRatioScorer::new();
        query_ratio_scorer.initialize(&documents);
        
        // Test queries
        let queries = vec![
            "rust programming",
            "python tutorial",
            "javascript web",
            "programming language",
        ];
        
        for query in queries {
            println!("Query: {}", query);
            
            // Score documents with QueryRatio
            let mut query_ratio_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = query_ratio_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Sort by score in descending order
            query_ratio_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            println!("QueryRatio ranking: {:?}", query_ratio_scores);
            
            // Verify that scores are between 0 and 1
            for (_, score) in &query_ratio_scores {
                assert!(*score >= 0.0 && *score <= 1.0, "QueryRatio score should be between 0 and 1");
            }
            
            // Verify that the top document contains at least one of the query terms
            let top_doc_id = query_ratio_scores.first().unwrap().0.clone();
            let top_doc = documents.iter().find(|doc| doc.id == top_doc_id).unwrap();
            
            let query_terms: Vec<&str> = query.split_whitespace().collect();
            let doc_contains_query_term = query_terms.iter().any(|term| {
                top_doc.body.to_lowercase().contains(&term.to_lowercase()) ||
                top_doc.title.to_lowercase().contains(&term.to_lowercase())
            });
            
            assert!(doc_contains_query_term, "Top document should contain at least one query term");
        }
    }

    #[test]
    fn test_all_scorers_with_same_query() {
        let documents = get_test_documents();
        let query = "programming language";
        
        // Initialize all scorers
        let mut bm25f_scorer = BM25FScorer::new();
        bm25f_scorer.initialize(&documents);
        
        let mut bm25plus_scorer = BM25PlusScorer::new();
        bm25plus_scorer.initialize(&documents);
        
        let mut okapi_bm25_scorer = OkapiBM25Scorer::new();
        okapi_bm25_scorer.initialize(&documents);
        
        let mut tfidf_scorer = TFIDFScorer::new();
        tfidf_scorer.initialize(&documents);
        
        let mut jaccard_scorer = JaccardScorer::new();
        jaccard_scorer.initialize(&documents);
        
        let mut query_ratio_scorer = QueryRatioScorer::new();
        query_ratio_scorer.initialize(&documents);
        
        // Score documents with all scorers
        let mut bm25f_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = bm25f_scorer.score(query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        let mut bm25plus_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = bm25plus_scorer.score(query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        let mut okapi_bm25_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = okapi_bm25_scorer.score(query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        let mut tfidf_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = tfidf_scorer.score(query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        let mut jaccard_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = jaccard_scorer.score(query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        let mut query_ratio_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = query_ratio_scorer.score(query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        // Sort all scores by score in descending order
        bm25f_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        bm25plus_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        okapi_bm25_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        tfidf_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        jaccard_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        query_ratio_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Print all rankings
        println!("Query: {}", query);
        println!("BM25F ranking: {:?}", bm25f_scores);
        println!("BM25+ ranking: {:?}", bm25plus_scores);
        println!("Okapi BM25 ranking: {:?}", okapi_bm25_scores);
        println!("TFIDF ranking: {:?}", tfidf_scores);
        println!("Jaccard ranking: {:?}", jaccard_scores);
        println!("QueryRatio ranking: {:?}", query_ratio_scores);
        
        // Verify that all scorers return non-zero scores for documents containing query terms
        for doc in &documents {
            if doc.body.to_lowercase().contains("programming") || 
               doc.title.to_lowercase().contains("programming") ||
               doc.body.to_lowercase().contains("language") || 
               doc.title.to_lowercase().contains("language") {
                
                let bm25f_score = bm25f_scores.iter().find(|(id, _)| id == &doc.id).unwrap().1;
                let bm25plus_score = bm25plus_scores.iter().find(|(id, _)| id == &doc.id).unwrap().1;
                let okapi_bm25_score = okapi_bm25_scores.iter().find(|(id, _)| id == &doc.id).unwrap().1;
                let tfidf_score = tfidf_scores.iter().find(|(id, _)| id == &doc.id).unwrap().1;
                let jaccard_score = jaccard_scores.iter().find(|(id, _)| id == &doc.id).unwrap().1;
                let query_ratio_score = query_ratio_scores.iter().find(|(id, _)| id == &doc.id).unwrap().1;
                
                // Check if the document contains both terms or just one term
                let contains_both_terms = (doc.body.to_lowercase().contains("programming") || 
                                          doc.title.to_lowercase().contains("programming")) &&
                                         (doc.body.to_lowercase().contains("language") || 
                                          doc.title.to_lowercase().contains("language"));
                
                // For documents containing both terms, all scorers should return positive scores
                if contains_both_terms {
                    assert!(bm25f_score > 0.0, "BM25F score should be positive for document containing both query terms");
                    assert!(bm25plus_score > 0.0, "BM25+ score should be positive for document containing both query terms");
                    assert!(okapi_bm25_score > 0.0, "Okapi BM25 score should be positive for document containing both query terms");
                    assert!(tfidf_score > 0.0, "TFIDF score should be positive for document containing both query terms");
                    assert!(jaccard_score > 0.0, "Jaccard score should be positive for document containing both query terms");
                    assert!(query_ratio_score > 0.0, "QueryRatio score should be positive for document containing both query terms");
                } else {
                    // For documents containing only one term, some scorers might return zero scores
                    // depending on their implementation, so we don't assert anything here
                    println!("Document {} contains only one query term", doc.id);
                    println!("BM25F score: {}", bm25f_score);
                    println!("BM25+ score: {}", bm25plus_score);
                    println!("Okapi BM25 score: {}", okapi_bm25_score);
                    println!("TFIDF score: {}", tfidf_score);
                    println!("Jaccard score: {}", jaccard_score);
                    println!("QueryRatio score: {}", query_ratio_score);
                }
            }
        }
    }

    #[test]
    fn test_validate_jaccard_similarity() {
        // Create test documents with predictable term overlap
        let documents = vec![
            Document {
                id: "doc1".to_string(),
                url: "http://example.com/1".to_string(),
                title: "apple banana cherry".to_string(),
                body: "apple banana cherry date".to_string(),
                description: None,
        summarization: None,
                stub: None,
                tags: None,
                rank: None,
            },
            Document {
                id: "doc2".to_string(),
                url: "http://example.com/2".to_string(),
                title: "apple banana".to_string(),
                body: "apple banana elderberry".to_string(),
                description: None,
        summarization: None,
                stub: None,
                tags: None,
                rank: None,
            },
            Document {
                id: "doc3".to_string(),
                url: "http://example.com/3".to_string(),
                title: "cherry date".to_string(),
                body: "cherry date fig".to_string(),
                description: None,
        summarization: None,
                stub: None,
                tags: None,
                rank: None,
            },
        ];
        
        // Initialize Jaccard scorer
        let mut jaccard_scorer = JaccardScorer::new();
        jaccard_scorer.initialize(&documents);
        
        // Test with query "apple banana"
        let query = "apple banana";
        let scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = jaccard_scorer.score(query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        // Calculate expected scores manually
        // For doc1: intersection = 2 (apple, banana), union = 4 (apple, banana, cherry, date) => 2/4 = 0.5
        // For doc2: intersection = 2 (apple, banana), union = 3 (apple, banana, elderberry) => 2/3 = 0.67
        // For doc3: intersection = 0, union = 5 (apple, banana, cherry, date, fig) => 0/5 = 0
        
        println!("Query: {}", query);
        println!("Jaccard scores: {:?}", scores);
        
        // Verify scores are within expected ranges
        assert!(scores[0].1 >= 0.45 && scores[0].1 <= 0.55, "Doc1 score should be around 0.5");
        assert!(scores[1].1 >= 0.6 && scores[1].1 <= 0.7, "Doc2 score should be around 0.67");
        assert_eq!(scores[2].1, 0.0, "Doc3 score should be 0");
        
        // Verify ranking order
        let mut ranked_scores = scores.clone();
        ranked_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(ranked_scores[0].0, "doc2", "Doc2 should be ranked first");
        assert_eq!(ranked_scores[1].0, "doc1", "Doc1 should be ranked second");
        assert_eq!(ranked_scores[2].0, "doc3", "Doc3 should be ranked third");
    }

    #[test]
    fn test_compare_jaccard_with_other_measures() {
        let documents = get_test_documents(); // Use existing test documents
        
        // Initialize scorers
        let mut jaccard_scorer = JaccardScorer::new();
        let mut query_ratio_scorer = QueryRatioScorer::new();
        let mut tfidf_scorer = TFIDFScorer::new();
        
        jaccard_scorer.initialize(&documents);
        query_ratio_scorer.initialize(&documents);
        tfidf_scorer.initialize(&documents);
        
        // Test queries with different characteristics
        let queries = vec![
            "rare unique terms", // Query with rare terms
            "common frequent words", // Query with common terms
            "programming language", // Query with terms in the documents
        ];
        
        for query in queries {
            println!("\nQuery: {}", query);
            
            // Score with Jaccard
            let jaccard_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = jaccard_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Score with QueryRatio
            let query_ratio_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = query_ratio_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Score with TFIDF
            let tfidf_scores: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = tfidf_scorer.score(query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            println!("Jaccard scores: {:?}", jaccard_scores);
            println!("QueryRatio scores: {:?}", query_ratio_scores);
            println!("TFIDF scores: {:?}", tfidf_scores);
            
            // Verify Jaccard scores are between 0 and 1
            for (_, score) in &jaccard_scores {
                assert!(*score >= 0.0 && *score <= 1.0, "Jaccard score should be between 0 and 1");
            }
        }
    }

    #[test]
    fn test_jaccard_edge_cases() {
        let documents = vec![
            Document {
                id: "empty".to_string(),
                url: "http://example.com/empty".to_string(),
                title: "".to_string(),
                body: "".to_string(),
                description: None,
        summarization: None,
                stub: None,
                tags: None,
                rank: None,
            },
            Document {
                id: "identical".to_string(),
                url: "http://example.com/identical".to_string(),
                title: "test query".to_string(),
                body: "test query".to_string(),
                description: None,
        summarization: None,
                stub: None,
                tags: None,
                rank: None,
            },
            Document {
                id: "no_overlap".to_string(),
                url: "http://example.com/no_overlap".to_string(),
                title: "completely different content".to_string(),
                body: "absolutely no overlap with search terms".to_string(),
                description: None,
        summarization: None,
                stub: None,
                tags: None,
                rank: None,
            },
        ];
        
        let mut jaccard_scorer = JaccardScorer::new();
        jaccard_scorer.initialize(&documents);
        
        // Test with empty query
        let empty_query = "";
        let empty_query_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = jaccard_scorer.score(empty_query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        println!("Empty query scores: {:?}", empty_query_scores);
        
        // Test with exact match query
        let exact_query = "test query";
        let exact_query_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = jaccard_scorer.score(exact_query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        println!("Exact match query scores: {:?}", exact_query_scores);
        
        // Verify empty query returns 0 for all documents
        for (_, score) in &empty_query_scores {
            assert_eq!(*score, 0.0, "Empty query should return 0 score");
        }
        
        // Verify exact match returns 1.0 for identical document
        let identical_score = exact_query_scores.iter()
            .find(|(id, _)| id == "identical")
            .unwrap().1;
        assert!(identical_score > 0.9, "Identical document should have score close to 1.0");
        
        // Verify no overlap has low score (not necessarily 0 due to how Jaccard works with term sets)
        let no_overlap_score = exact_query_scores.iter()
            .find(|(id, _)| id == "no_overlap")
            .unwrap().1;
        assert_eq!(no_overlap_score, 0.0, "Document with no overlapping terms should have a score of 0");
        
        // Debug the intersection calculation
        let query_terms: Vec<String> = exact_query.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        let no_overlap_doc = documents.iter().find(|doc| doc.id == "no_overlap").unwrap();
        let doc_terms: Vec<String> = no_overlap_doc.body.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        println!("Query terms: {:?}", query_terms);
        println!("Document terms: {:?}", doc_terms);
        
        let query_set: std::collections::HashSet<String> = query_terms.into_iter().collect();
        let doc_set: std::collections::HashSet<String> = doc_terms.into_iter().collect();
        
        println!("Query set: {:?}", query_set);
        println!("Document set: {:?}", doc_set);
        
        let intersection: std::collections::HashSet<_> = query_set.intersection(&doc_set).cloned().collect();
        println!("Intersection: {:?}", intersection);
        
        assert_eq!(intersection.len(), 0, "Intersection should be 0 for document with no overlap");
    }

    #[test]
    fn test_visualize_jaccard_similarity() {
        let documents = get_test_documents();
        let mut jaccard_scorer = JaccardScorer::new();
        jaccard_scorer.initialize(&documents);
        
        let query = "programming language";
        
        // Score documents
        let scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                // Calculate term sets
                let query_terms: HashSet<String> = query.split_whitespace()
                    .map(|s| s.to_lowercase())
                    .collect();
                
                let doc_terms: HashSet<String> = doc.body.split_whitespace()
                    .map(|s| s.to_lowercase())
                    .collect();
                
                // Calculate intersection and union
                let intersection = query_terms.intersection(&doc_terms).count();
                let union = query_terms.len() + doc_terms.len() - intersection;
                
                // Calculate Jaccard score
                let score = if union > 0 {
                    intersection as f64 / union as f64
                } else {
                    0.0
                };
                
                println!("Document: {}", doc.id);
                println!("  Query terms: {:?}", query_terms);
                println!("  Doc terms: {:?}", doc_terms);
                println!("  Intersection: {}", intersection);
                println!("  Union: {}", union);
                println!("  Jaccard score: {:.4}", score);
                println!();
                
                // Compare with the scorer's result
                let scorer_result = jaccard_scorer.score(query, doc);
                println!("  Scorer result: {:.4}", scorer_result);
                
                // They should be close (allowing for minor differences in implementation)
                assert!((score - scorer_result).abs() < 0.1, 
                    "Manual calculation ({}) should match scorer result ({})", 
                    score, scorer_result);
                
                (doc.id.clone(), score)
            })
            .collect();
        
        // Sort by score
        let mut ranked_scores = scores.clone();
        ranked_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        println!("Final ranking: {:?}", ranked_scores);
    }
} 