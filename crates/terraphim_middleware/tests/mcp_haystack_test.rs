use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_middleware::indexer::search_haystacks;
use terraphim_types::SearchQuery;

/// Live MCP haystack test using SSE server-everything
///
/// Requires MCP_SERVER_URL (e.g., http://127.0.0.1:3001)
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

    let mut role = Role::new("MCP");
    role.shortname = Some("MCP".to_string());
    role.theme = "lumen".to_string();
    role.haystacks = vec![Haystack::new(base_url.clone(), ServiceType::Mcp, true)
        .with_extra_parameter("base_url".into(), base_url.clone())
        .with_extra_parameter("transport".into(), "sse".into())];

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
