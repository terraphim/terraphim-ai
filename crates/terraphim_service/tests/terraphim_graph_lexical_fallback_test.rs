use ahash::AHashMap;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;
use terraphim_config::{
    Config, ConfigId, ConfigState, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role, ServiceType,
};
use terraphim_persistence::DeviceStorage;
use terraphim_service::TerraphimService;
use terraphim_types::{
    KnowledgeGraphInputType, NormalizedTermValue, RelevanceFunction, RoleName, SearchQuery,
};

#[tokio::test]
#[serial]
async fn terraphim_graph_falls_back_to_lexical_when_graph_returns_empty() {
    let _ = DeviceStorage::init_memory_only().await;

    let kg_dir = tempdir().expect("kg tempdir");
    let haystack_dir = tempdir().expect("haystack tempdir");

    fs::write(
        kg_dir.path().join("concept.md"),
        "# Concept\n\nsynonyms:: graph-concept, concept-node\n",
    )
    .expect("write kg file");

    let unique_query_term = "lexicalfallbackneedle2026";
    fs::write(
        haystack_dir.path().join("target.md"),
        format!(
            "# Target\n\nThis content contains {} only in haystack body.\n",
            unique_query_term
        ),
    )
    .expect("write haystack file");

    let role_name = RoleName::new("Graph Fallback");
    let role = Role {
        shortname: Some("graph_fallback".to_string()),
        name: role_name.clone(),
        relevance_function: RelevanceFunction::TerraphimGraph,
        terraphim_it: false,
        theme: "default".to_string(),
        kg: Some(KnowledgeGraph {
            automata_path: None,
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: kg_dir.path().to_path_buf(),
            }),
            public: false,
            publish: false,
        }),
        haystacks: vec![Haystack {
            location: haystack_dir.path().to_string_lossy().to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: HashMap::new(),
            fetch_content: false,
        }],
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: None,
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };

    let mut config = Config {
        id: ConfigId::Embedded,
        default_role: role_name.clone(),
        selected_role: role_name.clone(),
        ..Config::default()
    };
    config.roles.insert(role_name.clone(), role);

    let config_state = ConfigState::new(&mut config).await.expect("config state");
    let mut service = TerraphimService::new(config_state);

    let query = SearchQuery {
        search_term: NormalizedTermValue::from(unique_query_term),
        search_terms: None,
        operator: None,
        role: Some(role_name),
        skip: None,
        limit: None,
        layer: Default::default(),
        include_pinned: false,
    };

    let results = service.search(&query).await.expect("search succeeds");

    assert!(
        !results.is_empty(),
        "Expected lexical fallback results when TerraphimGraph query has no matching thesaurus term"
    );
    assert!(
        results.iter().any(|doc| doc.url.contains("target.md")),
        "Expected fallback results to include target.md, got: {:?}",
        results.iter().map(|d| d.url.clone()).collect::<Vec<_>>()
    );
}
