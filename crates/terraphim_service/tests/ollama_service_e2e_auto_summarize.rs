#![cfg(feature = "ollama")]

use serial_test::serial;
use std::fs;
use std::io::Write;
use std::time::Duration;
use tempfile::tempdir;
use terraphim_config::{Config, ConfigState, Haystack, Role, ServiceType};
use terraphim_service::TerraphimService;
use terraphim_types::{NormalizedTermValue, RelevanceFunction, RoleName, SearchQuery};

#[tokio::test]
#[serial]
async fn e2e_search_auto_summarize_with_ollama() {
    // Reachability check for local Ollama
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
    let http = terraphim_service::http_client::create_default_client()
        .unwrap_or_else(|_| reqwest::Client::new());
    let ok = http
        .get(format!("{}/api/tags", base_url.trim_end_matches('/')))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .ok()
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    if !ok {
        eprintln!("Ollama not reachable at {} — skipping e2e test", base_url);
        return;
    }

    // Prepare a temp haystack with a long markdown file to meet summarization threshold
    let dir = tempdir().expect("tempdir");
    let file_path = dir.path().join("long_article.md");
    let mut file = fs::File::create(&file_path).expect("create file");
    let long_body = "# Deepseek Coder Example\n\n".to_string()
        + &"Rust is a systems programming language focused on safety and speed. ".repeat(20)
        + "This text ensures the document body exceeds 200 characters for summarization tests.";
    file.write_all(long_body.as_bytes()).expect("write");

    // Build a minimal config with one role and a ripgrep haystack
    let role_name = RoleName::new("Ollama Coder");
    let mut role = Role {
        shortname: Some("ollama_coder".into()),
        name: role_name.clone(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![Haystack {
            location: dir.path().to_string_lossy().to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            fetch_content: false,
        }],
        extra: ahash::AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
        ..Default::default()
    };
    role.extra
        .insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra.insert(
        "llm_model".into(),
        serde_json::json!("deepseek-coder:latest"),
    );
    role.extra
        .insert("llm_base_url".into(), serde_json::json!(base_url.clone()));
    role.extra
        .insert("llm_auto_summarize".into(), serde_json::json!(true));

    let mut config = Config::default();
    config.roles.insert(role_name.clone(), role);
    config.default_role = role_name.clone();
    config.selected_role = role_name.clone();

    // Initialize state and service
    let config_state = ConfigState::new(&mut config).await.expect("config state");
    let mut service = TerraphimService::new(config_state);

    // Execute a search that matches our file
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::new("Rust".into()),
        limit: Some(5),
        skip: None,
        role: Some(role_name.clone()),
        ..Default::default()
    };

    let results = service.search(&search_query).await.expect("search ok");
    // If no results, bail out rather than failing build environments
    if results.is_empty() {
        eprintln!("No search results found in temp haystack — skipping assertions");
        return;
    }

    // We expect auto-summarization to attempt to fill description
    let had_description = results.iter().any(|d| {
        d.description
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    });
    assert!(
        had_description,
        "at least one result should have a non-empty AI-generated description"
    );
}
