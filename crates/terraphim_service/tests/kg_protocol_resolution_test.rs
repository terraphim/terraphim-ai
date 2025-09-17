#[cfg(test)]
mod kg_protocol_resolution_test {
    use ahash::AHashMap;
    use std::sync::Arc;
    use terraphim_config::{
        Config, ConfigId, ConfigState, KnowledgeGraph, KnowledgeGraphLocal, Role,
    };
    use terraphim_service::TerraphimService;
    use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction, RoleName};
    use tokio::sync::Mutex;

    /// Test that KG protocol links (kg:term) resolve to the correct definition documents
    ///
    /// This test verifies that when a KG link like [graph embeddings](kg:terraphim-graph)
    /// is clicked, the system retrieves ./docs/src/kg/terraphim-graph.md
    #[tokio::test]
    async fn test_kg_protocol_resolves_to_definition_documents() {
        println!("🧪 Testing KG protocol resolution to definition documents...");

        // Create a test role with KG configuration
        let test_role = Role {
            shortname: Some("test-engineer".to_string()),
            name: "Test Engineer".to_string().into(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            terraphim_it: true, // Enable KG preprocessing
            theme: "superhero".to_string(),
            kg: Some(KnowledgeGraph {
                automata_path: None, // Will be built from local KG files
                knowledge_graph_local: Some(KnowledgeGraphLocal {
                    input_type: KnowledgeGraphInputType::Markdown,
                    // Use absolute path from current working directory
                    path: std::env::current_dir().unwrap().join("docs/src/kg"),
                }),
                public: false,
                publish: false,
            }),
            haystacks: vec![],
            #[cfg(feature = "openrouter")]
            openrouter_enabled: false,
            #[cfg(feature = "openrouter")]
            openrouter_api_key: None,
            #[cfg(feature = "openrouter")]
            openrouter_model: None,
            #[cfg(feature = "openrouter")]
            openrouter_auto_summarize: false,
            #[cfg(feature = "openrouter")]
            openrouter_chat_enabled: false,
            #[cfg(feature = "openrouter")]
            openrouter_chat_system_prompt: None,
            #[cfg(feature = "openrouter")]
            openrouter_chat_model: None,
            llm_system_prompt: None,
            extra: AHashMap::new(),
        };

        let role_name = RoleName::new("Test Engineer");
        let mut roles = AHashMap::new();
        roles.insert(role_name.clone(), test_role);

        let config = Config {
            id: ConfigId::Server,
            global_shortcut: "Cmd+Space".to_string(),
            roles,
            default_role: role_name.clone(),
            selected_role: role_name.clone(),
            default_chat_model: None,
            default_model_provider: None,
            default_summarization_model: None,
        };

        let config_state = ConfigState {
            config: Arc::new(Mutex::new(config)),
            roles: AHashMap::new(),
        };

        let mut terraphim_service = TerraphimService::new(config_state);

        // Test cases: KG terms that should resolve to definition documents
        let test_cases = vec![
            ("terraphim-graph", "./docs/src/kg/terraphim-graph.md"),
            ("graph", "./docs/src/kg/terraphim-graph.md"), // synonym should resolve to same doc
            ("graph embeddings", "./docs/src/kg/terraphim-graph.md"), // synonym should resolve to same doc
            (
                "knowledge graph based embeddings",
                "./docs/src/kg/terraphim-graph.md",
            ), // synonym
        ];

        println!("📋 Testing {} KG term resolution cases", test_cases.len());

        for (kg_term, expected_doc_path) in test_cases {
            println!(
                "  🔍 Testing KG term: '{}' → should resolve to '{}'",
                kg_term, expected_doc_path
            );

            // Call find_documents_for_kg_term - this is what gets called when kg: links are clicked
            let documents = terraphim_service
                .find_documents_for_kg_term(&role_name, kg_term)
                .await
                .unwrap_or_else(|_| panic!("Failed to find documents for KG term '{}'", kg_term));

            println!(
                "    📄 Found {} documents for term '{}'",
                documents.len(),
                kg_term
            );

            // Verify we found at least one document
            assert!(
                !documents.is_empty(),
                "No documents found for KG term '{}' - should find definition document at '{}'",
                kg_term,
                expected_doc_path
            );

            // Check if any of the returned documents is the expected definition document
            let found_definition_doc = documents.iter().find(|doc| {
                doc.url.contains("terraphim-graph.md")
                    || doc.url.ends_with("terraphim-graph.md")
                    || doc.id.contains("terraphim-graph")
            });

            assert!(
                found_definition_doc.is_some(),
                "KG term '{}' should resolve to definition document '{}', but found documents: {:?}",
                kg_term,
                expected_doc_path,
                documents.iter().map(|d| &d.url).collect::<Vec<_>>()
            );

            let definition_doc = found_definition_doc.unwrap();
            println!(
                "    ✅ Found definition document: '{}' (url: '{}')",
                definition_doc.title, definition_doc.url
            );

            // Verify the document contains expected content
            assert!(
                !definition_doc.body.is_empty(),
                "Definition document body should not be empty for term '{}'",
                kg_term
            );

            // Verify it contains the synonyms declaration (this is what makes it a KG definition doc)
            assert!(
                definition_doc.body.contains("synonyms::")
                    || definition_doc.body.contains("graph embeddings")
                    || definition_doc.body.contains("knowledge graph"),
                "Definition document for '{}' should contain KG content with synonyms, found: '{}'",
                kg_term,
                definition_doc.body.chars().take(200).collect::<String>()
            );

            println!("    ✅ Definition document contains expected KG content");
        }

        println!("🎉 All KG protocol resolution tests passed!");
        println!("   ✓ KG terms resolve to correct definition documents");
        println!("   ✓ Synonyms resolve to the same definition document");
        println!("   ✓ Definition documents contain expected KG content");
    }

    /// Test that KG protocol resolution works for multiple synonyms
    #[tokio::test]
    async fn test_kg_synonyms_resolve_to_same_definition() {
        println!("🧪 Testing that KG synonyms resolve to the same definition document...");

        // Create a test role with KG configuration
        let test_role = Role {
            shortname: Some("synonym-test".to_string()),
            name: "Synonym Test".to_string().into(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            terraphim_it: true,
            theme: "lumen".to_string(),
            kg: Some(KnowledgeGraph {
                automata_path: None,
                knowledge_graph_local: Some(KnowledgeGraphLocal {
                    input_type: KnowledgeGraphInputType::Markdown,
                    // Use absolute path from current working directory
                    path: std::env::current_dir().unwrap().join("docs/src/kg"),
                }),
                public: false,
                publish: false,
            }),
            haystacks: vec![],
            #[cfg(feature = "openrouter")]
            openrouter_enabled: false,
            #[cfg(feature = "openrouter")]
            openrouter_api_key: None,
            #[cfg(feature = "openrouter")]
            openrouter_model: None,
            #[cfg(feature = "openrouter")]
            openrouter_auto_summarize: false,
            #[cfg(feature = "openrouter")]
            openrouter_chat_enabled: false,
            #[cfg(feature = "openrouter")]
            openrouter_chat_system_prompt: None,
            #[cfg(feature = "openrouter")]
            openrouter_chat_model: None,
            llm_system_prompt: None,
            extra: AHashMap::new(),
        };

        let role_name = RoleName::new("Synonym Test");
        let mut roles = AHashMap::new();
        roles.insert(role_name.clone(), test_role);

        let config = Config {
            id: ConfigId::Server,
            global_shortcut: "Cmd+Space".to_string(),
            roles,
            default_role: role_name.clone(),
            selected_role: role_name.clone(),
            default_chat_model: None,
            default_model_provider: None,
            default_summarization_model: None,
        };

        let config_state = ConfigState {
            config: Arc::new(Mutex::new(config)),
            roles: AHashMap::new(),
        };

        let mut terraphim_service = TerraphimService::new(config_state);

        // Test synonyms from the terraphim-graph.md file:
        // synonyms:: graph embeddings, graph, knowledge graph based embeddings
        let synonyms = vec![
            "graph embeddings",
            "graph",
            "knowledge graph based embeddings",
        ];

        println!(
            "📋 Testing {} synonyms resolve to same definition",
            synonyms.len()
        );

        let mut resolved_docs = Vec::new();

        for synonym in &synonyms {
            println!("  🔍 Resolving synonym: '{}'", synonym);

            let documents = terraphim_service
                .find_documents_for_kg_term(&role_name, synonym)
                .await
                .unwrap_or_else(|_| panic!("Failed to find documents for synonym '{}'", synonym));

            // Find the definition document (should contain terraphim-graph)
            let definition_doc = documents.iter().find(|doc| {
                doc.url.contains("terraphim-graph") || doc.id.contains("terraphim-graph")
            });

            assert!(
                definition_doc.is_some(),
                "Synonym '{}' should resolve to terraphim-graph definition document",
                synonym
            );

            let doc = definition_doc.unwrap();
            resolved_docs.push((synonym, doc.url.clone(), doc.id.clone()));

            println!("    ✅ Resolved to: '{}' (id: '{}')", doc.url, doc.id);
        }

        // Verify all synonyms resolved to the same definition document
        let first_url = &resolved_docs[0].1;
        let first_id = &resolved_docs[0].2;

        for (synonym, url, id) in &resolved_docs {
            assert_eq!(
                url, first_url,
                "All synonyms should resolve to the same URL. '{}' resolved to '{}' but expected '{}'",
                synonym, url, first_url
            );

            // Note: IDs might be different due to URL normalization, so we check the core concept
            assert!(
                id.contains("terraphim-graph") && first_id.contains("terraphim-graph"),
                "All synonyms should resolve to documents with 'terraphim-graph' in ID. '{}' resolved to '{}' but expected similar to '{}'",
                synonym, id, first_id
            );
        }

        println!("🎉 Synonym resolution test passed!");
        println!(
            "   ✓ All {} synonyms resolve to the same definition document",
            synonyms.len()
        );
        println!("   ✓ Definition document: '{}'", first_url);
    }
}
