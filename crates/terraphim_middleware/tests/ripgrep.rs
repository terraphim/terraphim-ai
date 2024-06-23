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
    use terraphim_types::{NormalizedTermValue, SearchQuery};
    use terraphim_types::{IndexedDocument, KnowledgeGraphInputType, RelevanceFunction};

    use terraphim_middleware::Result;

    #[tokio::test]
    async fn test_roundtrip() -> Result<()> {
        let role = Role {
            shortname: Some("operator".to_string()),
            name: "System Operator".to_string(),
            relevance_function: RelevanceFunction::TitleScorer,
            theme: "superhero".to_string(),
            kg: Some(KnowledgeGraph {
                automata_path: Some(AutomataPath::local_example()),
                knowledge_graph_local: Some(KnowledgeGraphLocal {
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from("/tmp/system_operator/pages/"),
                    public: true,
                    publish: true, 
                }),
            }),
            haystacks: vec![Haystack {
                path: PathBuf::from("/tmp/system_operator/pages/"),
                service: ServiceType::Ripgrep,
            }],
            extra: AHashMap::new(),
        };
        let mut config = ConfigBuilder::new()
            .add_role("System Operator", role.clone())
            .add_role(
                "Default",
                Role {
                    shortname: Some("Default".to_string()),
                    name: "Default".to_string(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "spacelab".to_string(),
                    kg: None,
                    haystacks: vec![Haystack {
                        path: PathBuf::from("/tmp/system_operator/pages/"),
                        service: ServiceType::Ripgrep,
                    }],
                    extra: AHashMap::new(),
                },
            )
            .default_role("Default")?
            .build()?;

        let config_state = ConfigState::new(&mut config).await?;

        let role_name = "System Operator".to_string();
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new("life cycle framework".to_string()),
            role: Some(role_name.clone()),
            skip: Some(0),
            limit: Some(10),
        };
        println!("Searching documents with query: {search_query:?} {role_name}");

        let index = search_haystacks(config_state.clone(), search_query.clone()).await?;
        let indexed_docs: Vec<IndexedDocument> = config_state
            .search_indexed_documents(&search_query, &role)
            .await;
        let documents = index.get_documents(indexed_docs);
        log::debug!("Final documents: {documents:?}");

        Ok(())
    }
}
