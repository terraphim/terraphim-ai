#[cfg(test)]
mod roundtrip {
    use std::path::PathBuf;

    use ahash::AHashMap;
    use terraphim_config::{Config, ConfigState, Haystack, KnowledgeGraph, Role, ServiceType};
    use terraphim_middleware::search_haystacks;
    use terraphim_types::{merge_and_serialize, IndexedDocument, RelevanceFunction, SearchQuery};

    use terraphim_middleware::Result;
    use ulid::Ulid;

    const SYSTEM_OPERATOR_ROLE_NAME: &str = "System Operator";

    // Helper function to create a new TerraphimConfig for testing logseq.
    // Eventually, we should make the config initialization more flexible.
    pub fn create_logseq_config() -> Config {
        let kg = KnowledgeGraph {
            automata_url: "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                .to_string(),
            kg_type: KnowledgeGraphType::Markdown,
            kg_path: "pkm".to_string(),
            public: true,
            publish: true,
        };
        let haystack = Haystack {
            path: PathBuf::from("fixtures/logseq_haystack".to_string()),
            service: ServiceType::Logseq,
        };
        let default_role = Role {
            shortname: Some(SYSTEM_OPERATOR_ROLE_NAME.to_string()),
            name: SYSTEM_OPERATOR_ROLE_NAME.to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "spacelab".to_string(),
            server_url: "http://localhost:8000/articles/search".to_string(),
            kg,
            haystacks: vec![haystack],
            extra: AHashMap::new(),
        };

        Config {
            id: Ulid::new().to_string(),
            // global shortcut for terraphim desktop
            global_shortcut: "Ctrl+X".to_string(),
            roles: AHashMap::from_iter(vec![(
                SYSTEM_OPERATOR_ROLE_NAME.to_lowercase().to_string(),
                default_role,
            )]),
            default_role: SYSTEM_OPERATOR_ROLE_NAME.to_string(),
        }
    }

    #[tokio::test]
    async fn test_roundtrip() -> Result<()> {
        let mut config = create_logseq_config();
        dbg!(&config);
        let config_state = ConfigState::new(&mut config).await?;
        let role = SYSTEM_OPERATOR_ROLE_NAME.to_string();
        // In this case, this is the synonym
        let needle = "life cycle framework".to_string();

        println!("Searching articles with query: {needle} {role}");
        let search_query = SearchQuery {
            search_term: needle.clone(),
            role: Some(role),
            skip: Some(0),
            limit: Some(10),
        };

        let articles = search_haystacks(config_state.clone(), search_query.clone()).await?;

        let docs: Vec<IndexedDocument> = config_state.search_articles(search_query).await;
        let articles = merge_and_serialize(articles, docs);
        println!("Articles: {articles:?}");

        Ok(())
    }
}
