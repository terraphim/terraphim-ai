#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ahash::AHashMap;
    use terraphim_automata::AutomataPath;
    use terraphim_config::{
        ConfigBuilder, ConfigState, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role,
        ServiceType,
    };
    use terraphim_middleware::search_haystacks;
    use terraphim_service::score;
    use terraphim_types::{
        Document, IndexedDocument, KnowledgeGraphInputType, NormalizedTermValue, RelevanceFunction,
        SearchQuery,
    };

    use terraphim_middleware::Result;

    // Helper function to create a test configuration with a specific relevance function
    async fn setup_test_config(relevance_function: RelevanceFunction) -> (ConfigState, Role) {
        // Create the path to the test haystack directory
        let docs_path = PathBuf::from("/Users/alex/projects/terraphim/terraphim-ai/crates/terraphim_rolegraph/data/system_operator_cc");
        println!("Docs path: {:?}", docs_path);

        let role_name = "Test Role".to_string();
        let role = Role {
            shortname: Some("test".into()),
            name: role_name.clone().into(),
            relevance_function,
            theme: "lumen".to_string(),
            kg: None, // No knowledge graph needed for these tests
            haystacks: vec![Haystack {
                path: docs_path.clone(),
                service: ServiceType::Ripgrep,
            }],
            extra: AHashMap::new(),
        };

        let mut config = ConfigBuilder::new()
            .add_role(&role_name, role.clone())
            .default_role(&role_name)
            .unwrap()
            .build()
            .unwrap();

        let config_state = ConfigState::new(&mut config).await.unwrap();

        (config_state, role)
    }

    // Helper function to perform a search with a specific query and role
    async fn perform_search(
        config_state: &ConfigState,
        role: &Role,
        search_term: &str,
    ) -> Vec<Document> {
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(search_term.to_string()),
            role: Some(role.name.clone()),
            skip: Some(0),
            limit: Some(10),
        };

        println!("Searching documents with query: {search_query:?}");

        let index = search_haystacks(config_state.clone(), search_query.clone())
            .await
            .unwrap();
        
        let indexed_docs: Vec<IndexedDocument> = config_state
            .search_indexed_documents(&search_query, role)
            .await;
        
        println!("Found {} indexed documents", indexed_docs.len());
        
        let documents = index.get_documents(indexed_docs);
        println!("Retrieved {} documents", documents.len());
        
        documents
    }

    // Helper function to rescore documents with a specific relevance function
    fn rescore_documents(
        documents: Vec<Document>,
        search_term: &str,
        relevance_function: RelevanceFunction,
    ) -> Vec<Document> {
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(search_term.to_string()),
            role: None,
            skip: None,
            limit: None,
        };

        score::rescore_documents(&search_query, documents, relevance_function)
    }

    #[tokio::test]
    async fn test_all_scorers_ripgrep() -> Result<()> {
        // Create the path to the test haystack directory
        let docs_path = PathBuf::from("/Users/alex/projects/terraphim/terraphim-ai/crates/terraphim_rolegraph/data/system_operator_cc");
        println!("Docs path: {:?}", docs_path);

        let role_name = "Test Role".to_string();
        let role = Role {
            shortname: Some("test".into()),
            name: role_name.clone().into(),
            relevance_function: RelevanceFunction::TitleScorer,
            theme: "lumen".to_string(),
            kg: None, // No knowledge graph needed for these tests
            haystacks: vec![Haystack {
                path: docs_path.clone(),
                service: ServiceType::Ripgrep,
            }],
            extra: AHashMap::new(),
        };

        let mut config = ConfigBuilder::new()
            .add_role(&role_name, role.clone())
            .default_role(&role_name)
            .unwrap()
            .build()
            .unwrap();

        let config_state = ConfigState::new(&mut config).await.unwrap();

        // Search for a term that should be in the test documents
        let search_term = "operation";
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(search_term.to_string()),
            role: Some(role.name.clone()),
            skip: Some(0),
            limit: Some(10),
        };

        println!("Searching documents with query: {search_query:?}");

        let index = search_haystacks(config_state.clone(), search_query.clone())
            .await
            .unwrap();
        
        let indexed_docs: Vec<IndexedDocument> = config_state
            .search_indexed_documents(&search_query, &role)
            .await;
        
        println!("Found {} indexed documents", indexed_docs.len());
        
        let documents = index.get_documents(indexed_docs);
        println!("Retrieved {} documents", documents.len());
        
        // Verify we got some results
        assert!(!documents.is_empty(), "Should have found documents");
        
        // If we have documents, test rescoring with different relevance functions
        if !documents.is_empty() {
            // Rescore with each relevance function
            let title_scored = rescore_documents(
                documents.clone(),
                search_term,
                RelevanceFunction::TitleScorer,
            );
            
            let bm25f_scored = rescore_documents(
                documents.clone(),
                search_term,
                RelevanceFunction::BM25F,
            );
            
            let bm25plus_scored = rescore_documents(
                documents.clone(),
                search_term,
                RelevanceFunction::BM25Plus,
            );
            
            // Print the top document for each scorer
            println!("TitleScorer top document: {}", title_scored[0].title);
            println!("BM25F top document: {}", bm25f_scored[0].title);
            println!("BM25Plus top document: {}", bm25plus_scored[0].title);
            
            // Compare the number of documents
            assert_eq!(
                title_scored.len(),
                bm25f_scored.len(),
                "All scorers should return the same number of documents"
            );
            
            assert_eq!(
                bm25f_scored.len(),
                bm25plus_scored.len(),
                "All scorers should return the same number of documents"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_terraphim_graph_ripgrep() -> Result<()> {
        let (config_state, role) = setup_test_config(RelevanceFunction::TerraphimGraph).await;
        
        // Search for a term that should be in the test documents
        let documents = perform_search(&config_state, &role, "maintenance").await;
        
        // Verify we got some results
        assert!(!documents.is_empty(), "Should have found documents with TerraphimGraph");

        Ok(())
    }

    #[tokio::test]
    async fn test_bm25f_ripgrep() -> Result<()> {
        let (config_state, role) = setup_test_config(RelevanceFunction::BM25F).await;
        
        // Search for a term that should be in the test documents
        let documents = perform_search(&config_state, &role, "operation").await;
        
        // Verify we got some results
        assert!(!documents.is_empty(), "Should have found documents with BM25F");
        
        // Rescore the documents with BM25F
        let rescored_documents = rescore_documents(
            documents.clone(),
            "operation",
            RelevanceFunction::BM25F,
        );
        
        // Verify we still have the same number of documents
        assert_eq!(
            documents.len(),
            rescored_documents.len(),
            "Rescoring should not change the number of documents"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_bm25plus_ripgrep() -> Result<()> {
        let (config_state, role) = setup_test_config(RelevanceFunction::BM25Plus).await;
        
        // Search for a term that should be in the test documents
        let documents = perform_search(&config_state, &role, "operation").await;
        
        // Verify we got some results
        assert!(!documents.is_empty(), "Should have found documents with BM25Plus");
        
        // Rescore the documents with BM25Plus
        let rescored_documents = rescore_documents(
            documents.clone(),
            "operation",
            RelevanceFunction::BM25Plus,
        );
        
        // Verify we still have the same number of documents
        assert_eq!(
            documents.len(),
            rescored_documents.len(),
            "Rescoring should not change the number of documents"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_compare_all_scorers_ripgrep() -> Result<()> {
        // Setup with TitleScorer (doesn't matter which one we use for initial setup)
        let (config_state, role) = setup_test_config(RelevanceFunction::TitleScorer).await;
        
        // Search for a term that should be in the test documents
        let search_term = "operation";
        let documents = perform_search(&config_state, &role, search_term).await;
        
        // Verify we got some results
        assert!(!documents.is_empty(), "Should have found documents");
        
        // Rescore with each relevance function
        let title_scored = rescore_documents(
            documents.clone(),
            search_term,
            RelevanceFunction::TitleScorer,
        );
        
        let bm25f_scored = rescore_documents(
            documents.clone(),
            search_term,
            RelevanceFunction::BM25F,
        );
        
        let bm25plus_scored = rescore_documents(
            documents.clone(),
            search_term,
            RelevanceFunction::BM25Plus,
        );
        
        // Print the top document for each scorer
        if !title_scored.is_empty() {
            println!("TitleScorer top document: {}", title_scored[0].title);
        }
        
        if !bm25f_scored.is_empty() {
            println!("BM25F top document: {}", bm25f_scored[0].title);
        }
        
        if !bm25plus_scored.is_empty() {
            println!("BM25Plus top document: {}", bm25plus_scored[0].title);
        }
        
        // Compare the number of documents
        assert_eq!(
            title_scored.len(),
            bm25f_scored.len(),
            "All scorers should return the same number of documents"
        );
        
        assert_eq!(
            bm25f_scored.len(),
            bm25plus_scored.len(),
            "All scorers should return the same number of documents"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_haystacks_with_all_scorers() -> Result<()> {
        // Setup two different haystacks
        let haystack1_path = PathBuf::from("/Users/alex/projects/terraphim/terraphim-ai/crates/terraphim_rolegraph/data/system_operator_cc");
        let haystack2_path = PathBuf::from("/Users/alex/projects/terraphim/terraphim-ai/docs/en");
        
        println!("Haystack 1 path: {:?}", haystack1_path);
        println!("Haystack 2 path: {:?}", haystack2_path);
        
        let role_name = "Multi Haystack Role".to_string();
        let role = Role {
            shortname: Some("multi".into()),
            name: role_name.clone().into(),
            relevance_function: RelevanceFunction::BM25F, // Use BM25F for this test
            theme: "lumen".to_string(),
            kg: None, // No knowledge graph needed for these tests
            haystacks: vec![
                Haystack {
                    path: haystack1_path.clone(),
                    service: ServiceType::Ripgrep,
                },
                Haystack {
                    path: haystack2_path.clone(),
                    service: ServiceType::Ripgrep,
                },
            ],
            extra: AHashMap::new(),
        };
        
        let mut config = ConfigBuilder::new()
            .add_role(&role_name, role.clone())
            .default_role(&role_name)
            .unwrap()
            .build()
            .unwrap();
            
        let config_state = ConfigState::new(&mut config).await.unwrap();
        
        // Search for a term that should be in both haystacks
        let search_term = "operation";
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(search_term.to_string()),
            role: Some(role.name.clone()),
            skip: Some(0),
            limit: Some(20),
        };
        
        let index = search_haystacks(config_state.clone(), search_query.clone())
            .await
            .unwrap();
            
        let indexed_docs: Vec<IndexedDocument> = config_state
            .search_indexed_documents(&search_query, &role)
            .await;
            
        println!("Found {} indexed documents from multiple haystacks", indexed_docs.len());
        
        let documents = index.get_documents(indexed_docs);
        println!("Retrieved {} documents from multiple haystacks", documents.len());
        
        // Verify we got some results
        assert!(!documents.is_empty(), "Should have found documents from multiple haystacks");
        
        // Rescore with each relevance function
        let title_scored = rescore_documents(
            documents.clone(),
            search_term,
            RelevanceFunction::TitleScorer,
        );
        
        let bm25f_scored = rescore_documents(
            documents.clone(),
            search_term,
            RelevanceFunction::BM25F,
        );
        
        let bm25plus_scored = rescore_documents(
            documents.clone(),
            search_term,
            RelevanceFunction::BM25Plus,
        );
        
        // Print the top document for each scorer
        if !title_scored.is_empty() {
            println!("TitleScorer top document from multiple haystacks: {}", title_scored[0].title);
        }
        
        if !bm25f_scored.is_empty() {
            println!("BM25F top document from multiple haystacks: {}", bm25f_scored[0].title);
        }
        
        if !bm25plus_scored.is_empty() {
            println!("BM25Plus top document from multiple haystacks: {}", bm25plus_scored[0].title);
        }
        
        // Compare the number of documents
        assert_eq!(
            title_scored.len(),
            bm25f_scored.len(),
            "All scorers should return the same number of documents"
        );
        
        assert_eq!(
            bm25f_scored.len(),
            bm25plus_scored.len(),
            "All scorers should return the same number of documents"
        );

        Ok(())
    }
} 