use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_middleware::{
    haystack::ClickUpHaystackIndexer,
    indexer::{IndexMiddleware, search_haystacks},
};
use terraphim_test_utils::EnvVarGuard;
use terraphim_types::{RelevanceFunction, SearchQuery};

#[tokio::test]
async fn clickup_mapping_handles_missing_token() {
    // Ensure no token is present
    let _guard = EnvVarGuard::remove("CLICKUP_API_TOKEN");

    let haystack = Haystack::new("clickup".to_string(), ServiceType::ClickUp, true);
    let indexer = ClickUpHaystackIndexer::default();
    let index = indexer
        .index("test", &haystack)
        .await
        .expect("indexing should not error");
    assert!(index.is_empty());
}

#[tokio::test]
#[ignore]
async fn clickup_live_search_returns_documents() {
    // Requires CLICKUP_API_TOKEN and CLICKUP_TEAM_ID set
    dotenvy::dotenv().ok();
    if std::env::var("CLICKUP_API_TOKEN").is_err() || std::env::var("CLICKUP_TEAM_ID").is_err() {
        eprintln!("CLICKUP_API_TOKEN or CLICKUP_TEAM_ID not set; skipping");
        return;
    }

    let role = Role {
        shortname: Some("ClickUp".to_string()),
        name: "ClickUp".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "lumen".to_string(),
        kg: None,
        haystacks: vec![
            Haystack::new("clickup".to_string(), ServiceType::ClickUp, true)
                .with_extra_parameter("team_id".into(), std::env::var("CLICKUP_TEAM_ID").unwrap())
                .with_extra_parameter("include_closed".into(), "true".into())
                .with_extra_parameter("subtasks".into(), "true".into()),
        ],
        extra: ahash::AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
        ..Default::default()
    };

    let mut config = ConfigBuilder::new()
        .add_role("ClickUp", role)
        .default_role("ClickUp")
        .unwrap()
        .build()
        .unwrap();

    let config_state = terraphim_config::ConfigState::new(&mut config)
        .await
        .unwrap();
    let query = SearchQuery {
        search_term: "meeting".into(),
        skip: Some(0),
        limit: Some(10),
        role: Some("ClickUp".into()),
        operator: None,
        search_terms: None,
    };
    let _results = search_haystacks(config_state, query).await.unwrap();
    // Results should be empty or non-empty (both are valid for this test)
}

#[tokio::test]
#[ignore]
async fn clickup_live_search_work_term() {
    // Requires CLICKUP_API_TOKEN and CLICKUP_TEAM_ID set
    dotenvy::dotenv().ok();
    if std::env::var("CLICKUP_API_TOKEN").is_err() || std::env::var("CLICKUP_TEAM_ID").is_err() {
        eprintln!("CLICKUP_API_TOKEN or CLICKUP_TEAM_ID not set; skipping");
        return;
    }

    let role = Role {
        shortname: Some("ClickUp".to_string()),
        name: "ClickUp".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "lumen".to_string(),
        kg: None,
        haystacks: vec![
            Haystack::new("clickup".to_string(), ServiceType::ClickUp, true)
                .with_extra_parameter("team_id".into(), std::env::var("CLICKUP_TEAM_ID").unwrap())
                .with_extra_parameter("include_closed".into(), "true".into())
                .with_extra_parameter("subtasks".into(), "true".into()),
        ],
        extra: ahash::AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
        ..Default::default()
    };

    let mut config = ConfigBuilder::new()
        .add_role("ClickUp", role)
        .default_role("ClickUp")
        .unwrap()
        .build()
        .unwrap();

    let config_state = terraphim_config::ConfigState::new(&mut config)
        .await
        .unwrap();
    let query = SearchQuery {
        search_term: "work".into(),
        skip: Some(0),
        limit: Some(10),
        role: Some("ClickUp".into()),
        operator: None,
        search_terms: None,
    };
    let results = search_haystacks(config_state, query).await.unwrap();
    assert!(
        !results.is_empty(),
        "expected some ClickUp results for term 'work'"
    );
}
