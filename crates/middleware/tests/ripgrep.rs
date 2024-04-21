#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ahash::AHashMap;
    use terraphim_automata::AutomataPath;
    use terraphim_config::{
        ConfigBuilder, ConfigState, Haystack, KnowledgeGraph, Role, ServiceType,
    };
    use terraphim_middleware::search_haystacks;
    use terraphim_types::{merge_and_serialize, SearchQuery};
    use terraphim_types::{IndexedDocument, KnowledgeGraphInputType, RelevanceFunction};

    use terraphim_middleware::Result;
    use url::Url;

    #[tokio::test]
    async fn test_roundtrip() -> Result<()> {
        let mut config = ConfigBuilder::new()
            .add_role(
                "System Operator",
                Role {
                    shortname: Some("operator".to_string()),
                    name: "System Operator".to_string(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "superhero".to_string(),
                    server_url: Url::parse("http://localhost:8000/articles/search").unwrap(),
                    kg: KnowledgeGraph {
                        automata_path: AutomataPath::remote_example(),
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("~/pkm"),
                        public: true,
                        publish: true,
                    },
                    haystacks: vec![Haystack {
                        path: PathBuf::from("/tmp/system_operator/pages/"),
                        service: ServiceType::Ripgrep,
                    }],
                    extra: AHashMap::new(),
                },
            )
            .default_role("Default")
            .build()?;

        let config_state = ConfigState::new(&mut config).await?;

        let role_name = "System Operator".to_string();
        let search_query = SearchQuery {
            search_term: "life cycle framework".to_string(),
            role: Some(role_name.clone()),
            skip: Some(0),
            limit: Some(10),
        };
        println!("Searching documents with query: {search_query:?} {role_name}");

        let haystack_documents =
            search_haystacks(config_state.clone(), search_query.clone()).await?;
        let docs: Vec<IndexedDocument> = config_state.search_documents(&search_query).await;
        let documents = merge_and_serialize(haystack_documents, docs);
        log::debug!("Final documents: {documents:?}");

        Ok(())
    }
}
