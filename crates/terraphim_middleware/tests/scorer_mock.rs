#[cfg(test)]
mod tests {
    use terraphim_service::score;
    use terraphim_types::{
        Document, NormalizedTermValue, RelevanceFunction, SearchQuery,
    };

    #[test]
    fn test_all_scorers_with_mock_documents() {
        // Create mock documents
        let documents = vec![
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
                body: "JavaScript is a programming language used for web development.".to_string(),
                description: Some("Learn JavaScript for web development".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string(), "web".to_string()]),
                rank: None,
            },
        ];

        // Create search query
        let search_term = "programming";
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(search_term.to_string()),
            role: None,
            skip: None,
            limit: None,
        };

        // Test TitleScorer
        let title_scored = score::rescore_documents(&search_query, documents.clone(), RelevanceFunction::TitleScorer);
        
        // Verify we got the same number of documents
        assert_eq!(title_scored.len(), documents.len(), "TitleScorer should return the same number of documents");
        
        // The first document should have "programming" in the title
        assert!(
            title_scored[0].title.to_lowercase().contains("programming"),
            "First document should have 'programming' in the title with TitleScorer"
        );

        // Test BM25F
        let bm25f_scored = score::rescore_documents(&search_query, documents.clone(), RelevanceFunction::BM25F);
        
        // Verify we got the same number of documents
        assert_eq!(bm25f_scored.len(), documents.len(), "BM25F should return the same number of documents");
        
        // Print the top document for BM25F
        println!("BM25F top document: {}", bm25f_scored[0].title);

        // Test BM25Plus
        let bm25plus_scored = score::rescore_documents(&search_query, documents.clone(), RelevanceFunction::BM25Plus);
        
        // Verify we got the same number of documents
        assert_eq!(bm25plus_scored.len(), documents.len(), "BM25Plus should return the same number of documents");
        
        // Print the top document for BM25Plus
        println!("BM25Plus top document: {}", bm25plus_scored[0].title);

        // Compare rankings between different scorers
        println!("TitleScorer ranking:");
        for (i, doc) in title_scored.iter().enumerate() {
            println!("  {}. {}", i + 1, doc.title);
        }

        println!("BM25F ranking:");
        for (i, doc) in bm25f_scored.iter().enumerate() {
            println!("  {}. {}", i + 1, doc.title);
        }

        println!("BM25Plus ranking:");
        for (i, doc) in bm25plus_scored.iter().enumerate() {
            println!("  {}. {}", i + 1, doc.title);
        }
    }

    #[test]
    fn test_bm25f_field_weighting() {
        // Create mock documents with different field emphasis
        let documents = vec![
            Document {
                id: "1".to_string(),
                url: "http://example.com/1".to_string(),
                title: "Document with Programming in Title".to_string(),
                body: "This document has some content but not much about the search term.".to_string(),
                description: Some("General description".to_string()),
                stub: None,
                tags: Some(vec!["general".to_string()]),
                rank: None,
            },
            Document {
                id: "2".to_string(),
                url: "http://example.com/2".to_string(),
                title: "General Document".to_string(),
                body: "This document talks a lot about programming, programming concepts, and programming languages.".to_string(),
                description: Some("General description".to_string()),
                stub: None,
                tags: Some(vec!["general".to_string()]),
                rank: None,
            },
            Document {
                id: "3".to_string(),
                url: "http://example.com/3".to_string(),
                title: "General Document".to_string(),
                body: "General content".to_string(),
                description: Some("Description about programming".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string()]),
                rank: None,
            },
        ];

        // Create search query
        let search_term = "programming";
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(search_term.to_string()),
            role: None,
            skip: None,
            limit: None,
        };

        // Test BM25F which should weight title and tags higher
        let bm25f_scored = score::rescore_documents(&search_query, documents.clone(), RelevanceFunction::BM25F);
        
        // Print the ranking for BM25F
        println!("BM25F ranking for field weighting test:");
        for (i, doc) in bm25f_scored.iter().enumerate() {
            println!("  {}. {}", i + 1, doc.title);
        }

        // Document 1 (title match) or Document 3 (tag match) should be ranked higher than Document 2 (body match)
        // due to field weighting in BM25F
        assert!(
            bm25f_scored[0].id == "1" || bm25f_scored[0].id == "3",
            "BM25F should prioritize title and tags over body content"
        );
    }

    #[test]
    fn test_bm25plus_rare_terms() {
        // Create mock documents with rare terms
        let documents = vec![
            Document {
                id: "1".to_string(),
                url: "http://example.com/1".to_string(),
                title: "Document One".to_string(),
                body: "This document contains the rare term xylophone exactly once.".to_string(),
                description: None,
                stub: None,
                tags: None,
                rank: None,
            },
            Document {
                id: "2".to_string(),
                url: "http://example.com/2".to_string(),
                title: "Document Two".to_string(),
                body: "This is a very long document with lots of content. It talks about many different topics and subjects. It has many paragraphs and sentences. The content goes on and on. Eventually, it mentions the term xylophone once, but it's buried in all this other content.".to_string(),
                description: None,
                stub: None,
                tags: None,
                rank: None,
            },
        ];

        // Create search query for a rare term
        let search_term = "xylophone";
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(search_term.to_string()),
            role: None,
            skip: None,
            limit: None,
        };

        // Test BM25Plus which should handle rare terms better
        let bm25plus_scored = score::rescore_documents(&search_query, documents.clone(), RelevanceFunction::BM25Plus);
        
        // Print the ranking for BM25Plus
        println!("BM25Plus ranking for rare term test:");
        for (i, doc) in bm25plus_scored.iter().enumerate() {
            println!("  {}. {}", i + 1, doc.title);
        }

        // Document 1 should be ranked higher than Document 2 because it's shorter
        // but both contain the rare term once
        assert_eq!(
            bm25plus_scored[0].id, "1",
            "BM25Plus should rank the shorter document higher for rare terms"
        );
    }
} 