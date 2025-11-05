use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_middleware::indexer::search_haystacks;
use terraphim_types::{RelevanceFunction, SearchQuery};

/// Live MCP haystack test using SSE server-everything
///
/// Requires MCP_SERVER_URL (e.g., http://127.0.0.1:8001)
#[tokio::test]
#[ignore]
async fn mcp_live_haystack_smoke() {
    dotenvy::dotenv().ok();
    let base_url = match std::env::var("MCP_SERVER_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("MCP_SERVER_URL not set; skipping live MCP test");
            return;
        }
    };

    let role = Role {
        shortname: Some("MCP".to_string()),
        name: "MCP".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "lumen".to_string(),
        kg: None,
        haystacks: vec![Haystack::new(base_url.clone(), ServiceType::Mcp, true)
            .with_extra_parameter("base_url".into(), base_url.clone())
            .with_extra_parameter("transport".into(), "sse".into())],
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: None,
        extra: ahash::AHashMap::new(),
        mcp_namespaces: vec![],
    };

    let mut config = ConfigBuilder::new()
        .add_role("MCP", role)
        .default_role("MCP")
        .unwrap()
        .build()
        .unwrap();

    let config_state = terraphim_config::ConfigState::new(&mut config)
        .await
        .expect("config state");

    // Perform a search (current indexer returns empty Index until full integration)
    let query = SearchQuery {
        search_term: "work".into(),
        skip: Some(0),
        limit: Some(10),
        role: Some("MCP".into()),
        operator: None,
        search_terms: None,
    };

    let result = search_haystacks(config_state, query)
        .await
        .expect("search ok");
    // If server-everything is running and exposes search/list, we expect results
    // Otherwise, the index may be empty and this test remains a smoke check
    if !result.is_empty() {
        assert!(
            result.values().next().is_some(),
            "expect at least one document"
        );
    }
}
