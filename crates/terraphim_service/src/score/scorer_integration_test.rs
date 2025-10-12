#[cfg(test)]
mod tests {
    use super::super::*;
    use terraphim_types::Document;

    fn create_test_documents() -> Vec<Document> {
        vec![
            Document {
                id: "doc1".to_string(),
                title: "Rust Programming".to_string(),
                body:
                    "Rust is a systems programming language that focuses on safety and performance"
                        .to_string(),
                url: "http://example.com/doc1".to_string(),
                description: Some("About Rust programming".to_string()),
                summarization: None,
                stub: None,
                tags: Some(vec!["programming".to_string(), "rust".to_string()]),
                rank: None,
                source_haystack: None,
            },
            Document {
                id: "doc2".to_string(),
                title: "Python Development".to_string(),
                body: "Python is a high-level programming language with dynamic typing".to_string(),
                url: "http://example.com/doc2".to_string(),
                description: Some("About Python development".to_string()),
                summarization: None,
                stub: None,
                tags: Some(vec!["programming".to_string(), "python".to_string()]),
                rank: None,
                source_haystack: None,
            },
            Document {
                id: "doc3".to_string(),
                title: "Machine Learning".to_string(),
                body: "Machine learning involves algorithms that improve through experience"
                    .to_string(),
                url: "http://example.com/doc3".to_string(),
                description: Some("About machine learning".to_string()),
                summarization: None,
                stub: None,
                tags: Some(vec!["ai".to_string(), "ml".to_string()]),
                rank: None,
                source_haystack: None,
            },
        ]
    }

    #[test]
    fn test_okapi_bm25_scorer_integration() {
        let documents = create_test_documents();
        let mut scorer = bm25_additional::OkapiBM25Scorer::new();
        scorer.initialize(&documents);

        // Test scoring
        let score1 = scorer.score("programming", &documents[0]);
        let score2 = scorer.score("programming", &documents[1]);
        let score3 = scorer.score("programming", &documents[2]);

        // Documents 1 and 2 should have higher scores than document 3 for "programming" query
        assert!(score1 > 0.0);
        assert!(score2 > 0.0);
        assert!(score3 >= 0.0);
        assert!(score1 > score3);
        assert!(score2 > score3);
    }

    #[test]
    fn test_jaccard_scorer_integration() {
        let documents = create_test_documents();
        let mut scorer = bm25_additional::JaccardScorer::new();
        scorer.initialize(&documents);

        // Test scoring
        let score1 = scorer.score("programming language", &documents[0]);
        let score2 = scorer.score("programming language", &documents[1]);
        let score3 = scorer.score("programming language", &documents[2]);

        // Documents 1 and 2 should have higher scores than document 3
        assert!(score1 > 0.0);
        assert!(score2 > 0.0);
        assert!(score3 >= 0.0);
    }

    #[test]
    fn test_query_ratio_scorer_integration() {
        let documents = create_test_documents();
        let mut scorer = bm25_additional::QueryRatioScorer::new();
        scorer.initialize(&documents);

        // Test scoring
        let score1 = scorer.score("rust systems", &documents[0]);
        let score2 = scorer.score("rust systems", &documents[1]);
        let score3 = scorer.score("rust systems", &documents[2]);

        // Document 1 should have the highest score for "rust systems" query
        assert!(score1 > 0.0);
        assert!(score1 >= score2);
        assert!(score1 >= score3);
    }

    #[test]
    fn test_tfidf_scorer_integration() {
        let documents = create_test_documents();
        let mut scorer = bm25_additional::TFIDFScorer::new();
        scorer.initialize(&documents);

        // Test scoring
        let score1 = scorer.score("programming", &documents[0]);
        let score2 = scorer.score("programming", &documents[1]);
        let score3 = scorer.score("programming", &documents[2]);

        // Documents 1 and 2 should have higher scores than document 3
        assert!(score1 > 0.0);
        assert!(score2 > 0.0);
        assert!(score1 > score3);
        assert!(score2 > score3);
    }

    #[test]
    fn test_with_params_functionality() {
        use super::super::common::BM25Params;

        let params = BM25Params {
            k1: 2.0,
            b: 0.5,
            delta: 0.0,
        };

        let documents = create_test_documents();
        let mut scorer = bm25_additional::OkapiBM25Scorer::with_params(params);
        scorer.initialize(&documents);

        // Should work with custom parameters
        let score = scorer.score("programming", &documents[0]);
        assert!(score > 0.0);
    }

    #[test]
    fn test_sort_documents_with_different_scorers() {
        let documents = create_test_documents();

        // Test with BM25 scorer
        let query = Query {
            name: "programming".to_string(),
            name_scorer: QueryScorer::BM25,
            similarity: Similarity::default(),
            size: 30,
        };

        let sorted_docs = sort_documents(&query, documents.clone());
        assert_eq!(sorted_docs.len(), 3);

        // Test with Jaccard scorer
        let query = Query {
            name: "programming".to_string(),
            name_scorer: QueryScorer::Jaccard,
            similarity: Similarity::default(),
            size: 30,
        };

        let sorted_docs = sort_documents(&query, documents.clone());
        assert_eq!(sorted_docs.len(), 3);

        // Test with TFIDF scorer
        let query = Query {
            name: "programming".to_string(),
            name_scorer: QueryScorer::Tfidf,
            similarity: Similarity::default(),
            size: 30,
        };

        let sorted_docs = sort_documents(&query, documents);
        assert_eq!(sorted_docs.len(), 3);
    }
}
