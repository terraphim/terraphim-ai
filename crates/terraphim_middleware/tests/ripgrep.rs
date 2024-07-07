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
    use terraphim_types::{IndexedDocument, KnowledgeGraphInputType, RelevanceFunction};
    use terraphim_types::{NormalizedTermValue, SearchQuery};

    use terraphim_middleware::Result;

    #[tokio::test]
    async fn test_terraphim_engineer_roundtrip() -> Result<()> {
        // Create the path to the default haystack directory
        // by concating the current directory with the default haystack path
        const DEFAULT_HAYSTACK_PATH: &str = "docs/src/";
        let mut docs_path = std::env::current_dir().unwrap();
        docs_path.pop();
        docs_path.pop();
        docs_path = docs_path.join(DEFAULT_HAYSTACK_PATH);
        println!("Docs path: {:?}", docs_path);
        let role_name = "Terraphim Engineer".to_string();
        let role = Role {
            shortname: Some("tfengineer".into()),
            name: role_name.clone().into(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "lumen".to_string(),
            kg: Some(KnowledgeGraph {
                automata_path: Some(AutomataPath::from_local(
                    docs_path.join("Terraphim Engineer_thesaurus.json".to_string()),
                )),
                public: true,
                publish: true,
                knowledge_graph_local: Some(KnowledgeGraphLocal {
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: docs_path.join("kg"),

                }),
            }),
            haystacks: vec![Haystack {
                path: docs_path.clone(),
                service: ServiceType::Ripgrep,
            }],
            extra: AHashMap::new(),
        };
        let mut config = ConfigBuilder::new()
            .add_role(&role_name, role.clone())
            .default_role(&role_name)?
            .build()?;

        let config_state = ConfigState::new(&mut config).await?;

        let role_name = "Terraphim Engineer".to_string();
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new("terraphim-graph".to_string()),
            role: Some(role_name.clone().into()),
            skip: Some(0),
            limit: Some(10),
        };
        println!("Searching documents with query: {search_query:?} {role_name}");

        let index = search_haystacks(config_state.clone(), search_query.clone()).await?;
        let indexed_docs: Vec<IndexedDocument> = config_state
            .search_indexed_documents(&search_query, &role)
            .await;
        println!("Indexed docs: {:?}", indexed_docs);
        let documents = index.get_documents(indexed_docs);
        println!("Documents: {:#?}", documents);
        log::debug!("Final documents: {documents:?}");

        Ok(())
    }

    #[tokio::test]
    async fn test_roundtrip() -> Result<()> {
        let role = Role {
            shortname: Some("operator".to_string()),
            name: "System Operator".into(),
            relevance_function: RelevanceFunction::TitleScorer,
            theme: "superhero".to_string(),
            kg: Some(KnowledgeGraph {
                automata_path: Some(AutomataPath::local_example()),
                knowledge_graph_local: Some(KnowledgeGraphLocal {
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from("/tmp/system_operator/pages/"), 
                }),
                public: true,
                publish: true,
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
                    shortname: Some("Default".into()),
                    name: "Default".into(),
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
            role: Some(role_name.clone().into()),
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
