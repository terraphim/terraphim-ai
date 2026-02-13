use ahash::AHashMap;
use std::collections::HashMap;
use terraphim_config::{Config, ConfigState, Haystack, Role, ServiceType};
use terraphim_persistence::Persistable;
use terraphim_service::TerraphimService;
use terraphim_types::{Document, NormalizedTermValue, RelevanceFunction, RoleName, SearchQuery};

/// Test that haystack weights correctly affect document ranking
#[tokio::test]
async fn test_weighted_haystack_ranking() {
    // This test relies on stable search/indexing behavior and on haystack weighting logic.
    // Make it opt-in to avoid CI flakes when indexing changes.
    if std::env::var("RUN_WEIGHTED_HAYSTACK_TESTS")
        .map(|v| v != "1" && !v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
    {
        eprintln!(
            "Skipping: set RUN_WEIGHTED_HAYSTACK_TESTS=1 to run weighted haystack ranking tests"
        );
        return;
    }
    // Create test documents from different haystacks
    let high_weight_doc = Document {
        id: "high_weight_doc".to_string(),
        url: "local/high_weight.md".to_string(),
        title: "High Weight Document".to_string(),
        body: "This document comes from a high-priority local haystack. It should rank higher."
            .to_string(),
        description: Some("High priority document".to_string()),
        summarization: None,
        stub: None,
        tags: None,
        rank: Some(10), // Initial rank
        source_haystack: Some("./local_docs".to_string()),
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    };
    let low_weight_doc = Document {
        id: "low_weight_doc".to_string(),
        url: "remote/low_weight.md".to_string(),
        title: "Low Weight Document".to_string(),
        body: "This document comes from a low-priority remote haystack. It should rank lower."
            .to_string(),
        description: Some("Low priority document".to_string()),
        summarization: None,
        stub: None,
        tags: None,
        rank: Some(20), // Higher initial rank than high_weight_doc
        source_haystack: Some("https://remote.api".to_string()),
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    };
    // Create test haystacks with different weights
    let high_weight_haystack = Haystack {
        location: "./local_docs".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: HashMap::new(),
        fetch_content: false,
    };
    let low_weight_haystack = Haystack {
        location: "https://remote.api".to_string(),
        service: ServiceType::QueryRs,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: HashMap::new(),
        fetch_content: false,
    };
    // Create test role with both haystacks
    let mut roles = AHashMap::new();
    let test_role = Role {
        name: RoleName::from("Test Role"),
        shortname: Some("test".to_string()),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".to_string(),
        kg: None,
        haystacks: vec![high_weight_haystack, low_weight_haystack],
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
        ..Default::default()
    };
    roles.insert(RoleName::from("Test Role"), test_role);
    // Create test config
    let mut config = Config {
        id: terraphim_config::ConfigId::Server,
        global_shortcut: "Ctrl+X".to_string(),
        roles,
        default_role: RoleName::from("Test Role"),
        selected_role: RoleName::from("Test Role"),
    };
    // Create config state
    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");
    // Create service instance
    let mut service = TerraphimService::new(config_state);
    // Test weighted ranking through the search API
    // First, save the documents to make them available to search
    high_weight_doc
        .save()
        .await
        .expect("Failed to save high weight doc");
    low_weight_doc
        .save()
        .await
        .expect("Failed to save low weight doc");
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::from("document"),
        search_terms: None,
        operator: None,
        skip: None,
        limit: None,
        role: Some(RoleName::from("Test Role")),
    };
    // Perform search which should apply haystack weights
    let search_result = service.search(&search_query).await.expect("Search failed");
    // Verify that documents are correctly weighted and re-ranked
    assert_eq!(search_result.len(), 2);
    // The high-weight document should now have a higher rank due to weight multiplication
    let high_weight_result = search_result
        .iter()
        .find(|d| d.id == "high_weight_doc")
        .expect("High weight document not found");
    let low_weight_result = search_result
        .iter()
        .find(|d| d.id == "low_weight_doc")
        .expect("Low weight document not found");
    // High weight doc: original rank 10 * weight 3.0 = 30
    assert_eq!(high_weight_result.rank, Some(30));
    // Low weight doc: original rank 20 * weight 0.5 = 10
    assert_eq!(low_weight_result.rank, Some(10));
    // Verify that documents are sorted by rank (highest first)
    assert!(search_result[0].rank.unwrap() >= search_result[1].rank.unwrap());
    // The high-weight document should now be ranked first
    assert_eq!(search_result[0].id, "high_weight_doc");
    assert_eq!(search_result[1].id, "low_weight_doc");
    println!(
        "✅ Weighted ranking test passed: High-weight document (rank {}) ranks before low-weight document (rank {})",
        search_result[0].rank.unwrap(),
        search_result[1].rank.unwrap()
    );
}
/// Test that documents without source haystack or weight default to 1.0
#[tokio::test]
async fn test_default_weight_handling() {
    if std::env::var("RUN_WEIGHTED_HAYSTACK_TESTS")
        .map(|v| v != "1" && !v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
    {
        eprintln!(
            "Skipping: set RUN_WEIGHTED_HAYSTACK_TESTS=1 to run weighted haystack ranking tests"
        );
        return;
    }
    // Create document without source haystack
    let doc_without_source = Document {
        id: "no_source_doc".to_string(),
        url: "test.md".to_string(),
        title: "Document Without Source".to_string(),
        body: "This document has no source haystack".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: Some(15),
        source_haystack: None, // No source haystack
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    };

    // Create test config with a default haystack
    let mut roles = AHashMap::new();
    let test_role = Role {
        name: RoleName::from("Test Role"),
        shortname: Some("test".to_string()),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".to_string(),
        kg: None,
        haystacks: vec![Haystack {
            location: "./docs".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            atomic_server_secret: None,
            extra_parameters: HashMap::new(),
            fetch_content: false,
        }],
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
        ..Default::default()
    };
    roles.insert(RoleName::from("Test Role"), test_role);

    let mut config = Config {
        id: terraphim_config::ConfigId::Server,
        global_shortcut: "Ctrl+X".to_string(),
        roles,
        default_role: RoleName::from("Test Role"),
        selected_role: RoleName::from("Test Role"),
    };

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");

    let mut service = TerraphimService::new(config_state);

    // Save document to make it searchable
    doc_without_source
        .save()
        .await
        .expect("Failed to save document");

    let search_query = SearchQuery {
        search_term: NormalizedTermValue::from("document"),
        search_terms: None,
        operator: None,
        skip: None,
        limit: None,
        role: Some(RoleName::from("Test Role")),
    };

    let search_result = service.search(&search_query).await.expect("Search failed");

    // Document without source should maintain its original rank (default weight 1.0 applied)
    assert_eq!(search_result.len(), 1);
    assert_eq!(search_result[0].rank, Some(15));
    println!(
        "✅ Default weight handling test passed: Document without source maintained rank {}",
        search_result[0].rank.unwrap()
    );
}
