use std::path::PathBuf;

use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_middleware::indexer::search_haystacks;
use terraphim_types::SearchQuery;

/// Integration test for Atlassian haystack located at ../atlassian_haystack
/// This test will be skipped if the directory does not exist.
#[tokio::test]
async fn atlassian_ripgrep_haystack_smoke() {
    // Resolve ../atlassian_haystack relative to repo root (current dir during tests)
    let path = PathBuf::from("..").join("atlassian_haystack");
    if !path.exists() {
        eprintln!(
            "Skipping Atlassian haystack test: directory not found at {}",
            path.display()
        );
        return;
    }

    // Create a role with a ripgrep haystack pointing to the Atlassian directory
    let mut role = Role::new("Atlassian");
    role.shortname = Some("Atlassian".to_string());
    role.theme = "lumen".to_string();
    role.haystacks = vec![Haystack::new(
        path.to_string_lossy().to_string(),
        ServiceType::Ripgrep,
        true,
    )];

    let mut config = ConfigBuilder::new()
        .add_role("Atlassian", role)
        .default_role("Atlassian")
        .unwrap()
        .build()
        .unwrap();

    let config_state = terraphim_config::ConfigState::new(&mut config)
        .await
        .expect("config state");

    // Perform a simple search; do not assert on content as it's environment-specific
    let query = SearchQuery {
        search_term: "work".into(),
        skip: Some(0),
        limit: Some(10),
        role: Some("Atlassian".into()),
        operator: Some(terraphim_types::LogicalOperator::And),
        search_terms: None,
    };
    let result = search_haystacks(config_state, query).await;
    assert!(
        result.is_ok(),
        "search should succeed against Atlassian haystack"
    );

    let index = result.unwrap();
    eprintln!(
        "Atlassian haystack search returned {} documents",
        index.len()
    );
}
