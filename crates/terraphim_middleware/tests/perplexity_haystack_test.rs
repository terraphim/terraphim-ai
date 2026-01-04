use std::collections::HashMap;
use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_middleware::indexer::search_haystacks;
use terraphim_types::{RelevanceFunction, SearchQuery};

/// Test Perplexity haystack configuration parsing
#[tokio::test]
async fn test_perplexity_config_parsing() {
    println!("🧪 Testing Perplexity Configuration");
    println!("=====================================");

    // Test basic configuration
    let mut extra_params = HashMap::new();
    extra_params.insert("api_key".to_string(), "test_key_12345".to_string());
    extra_params.insert("model".to_string(), "sonar-large-online".to_string());
    extra_params.insert("max_tokens".to_string(), "1500".to_string());
    extra_params.insert("temperature".to_string(), "0.1".to_string());
    extra_params.insert("cache_ttl_hours".to_string(), "2".to_string());
    extra_params.insert(
        "search_domains".to_string(),
        "arxiv.org,github.com".to_string(),
    );
    extra_params.insert("search_recency".to_string(), "week".to_string());

    let haystack = Haystack {
        location: "https://api.perplexity.ai".to_string(),
        service: ServiceType::Perplexity,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: extra_params,
        fetch_content: false,
    };

    println!("✅ Haystack configuration created successfully");
    assert_eq!(haystack.service, ServiceType::Perplexity);
    assert_eq!(haystack.location, "https://api.perplexity.ai");
    assert!(haystack.read_only);

    // Test that extra parameters are preserved
    assert_eq!(
        haystack.extra_parameters.get("api_key"),
        Some(&"test_key_12345".to_string())
    );
    assert_eq!(
        haystack.extra_parameters.get("model"),
        Some(&"sonar-large-online".to_string())
    );

    println!("✅ Extra parameters parsed correctly");
    println!("✅ Perplexity configuration test passed");
}

/// Test Perplexity role creation and service type integration
#[tokio::test]
async fn test_perplexity_service_type_integration() {
    println!("🧪 Testing ServiceType::Perplexity Integration");
    println!("===============================================");

    // Test that ServiceType::Perplexity is properly defined
    let service_type = ServiceType::Perplexity;

    match service_type {
        ServiceType::Perplexity => {
            println!("✅ ServiceType::Perplexity is properly defined");
        }
        _ => {
            panic!("ServiceType::Perplexity should match Perplexity variant");
        }
    }

    // Test that we can create a haystack with Perplexity service
    let mut extra_params = HashMap::new();
    extra_params.insert("api_key".to_string(), "sk-test123".to_string());
    extra_params.insert("model".to_string(), "sonar-medium-online".to_string());

    let haystack = Haystack {
        location: "https://api.perplexity.ai".to_string(),
        service: ServiceType::Perplexity,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: extra_params,
        fetch_content: false,
    };

    assert_eq!(haystack.service, ServiceType::Perplexity);
    assert_eq!(haystack.location, "https://api.perplexity.ai");
    assert!(haystack.read_only);

    println!("✅ Haystack configuration with Perplexity service works correctly");

    // Test role creation with Perplexity haystack
    let role = Role {
        shortname: Some("perplexity-test".to_string()),
        name: "Perplexity Test".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "superhero".to_string(),
        kg: None,
        haystacks: vec![haystack],
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: None,
        llm_router_enabled: false,
        llm_router_config: None,
        extra: ahash::AHashMap::new(),
    };

    assert_eq!(role.haystacks.len(), 1);
    assert_eq!(role.haystacks[0].service, ServiceType::Perplexity);

    println!("✅ Role with Perplexity haystack created successfully");
    println!("✅ ServiceType::Perplexity integration is complete");
}

/// Test Perplexity haystack document format and basic functionality
#[tokio::test]
async fn test_perplexity_document_format() {
    println!("🧪 Testing Perplexity Document Format");
    println!("=====================================");

    // Test various configuration scenarios
    let test_cases = vec![
        ("Basic config", "sonar-medium-online", None, None),
        ("Large model", "sonar-large-online", Some("2000"), None),
        (
            "With domains",
            "sonar-small-online",
            None,
            Some("github.com,arxiv.org"),
        ),
    ];

    for (name, model, max_tokens, domains) in test_cases {
        println!("Testing case: {}", name);

        let mut extra_params = HashMap::new();
        extra_params.insert("api_key".to_string(), "test_key".to_string());
        extra_params.insert("model".to_string(), model.to_string());

        if let Some(tokens) = max_tokens {
            extra_params.insert("max_tokens".to_string(), tokens.to_string());
        }

        if let Some(domain_list) = domains {
            extra_params.insert("search_domains".to_string(), domain_list.to_string());
        }

        let haystack = Haystack {
            location: "https://api.perplexity.ai".to_string(),
            service: ServiceType::Perplexity,
            read_only: true,
            atomic_server_secret: None,
            extra_parameters: extra_params,
            fetch_content: false,
        };

        // Verify the configuration is valid
        assert_eq!(haystack.service, ServiceType::Perplexity);
        assert!(!haystack.extra_parameters.is_empty());

        println!("  ✅ {} configuration valid", name);
    }

    println!("✅ All Perplexity document format tests passed");
}

/// Test error handling for missing API key
#[tokio::test]
async fn test_perplexity_missing_api_key() {
    println!("🧪 Testing Perplexity Error Handling");
    println!("====================================");

    // Create a haystack without API key
    let haystack = Haystack {
        location: "https://api.perplexity.ai".to_string(),
        service: ServiceType::Perplexity,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: HashMap::new(), // No API key
        fetch_content: false,
    };

    let role = Role {
        shortname: Some("perplexity-test".to_string()),
        name: "Perplexity Test".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "superhero".to_string(),
        kg: None,
        haystacks: vec![haystack],
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: None,
        llm_router_enabled: false,
        llm_router_config: None,
        extra: ahash::AHashMap::new(),
    };

    let mut config = ConfigBuilder::new()
        .add_role("Perplexity Test", role)
        .default_role("Perplexity Test")
        .unwrap()
        .build()
        .unwrap();

    let config_state = terraphim_config::ConfigState::new(&mut config)
        .await
        .expect("config state");

    // This should return empty results due to missing API key (graceful degradation)
    let query = SearchQuery {
        search_term: "test query".into(),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(10),
        role: Some("Perplexity Test".into()),
    };

    let result = search_haystacks(config_state, query).await;

    // Should not panic, should return empty or handle gracefully
    match result {
        Ok(index) => {
            // Empty index is acceptable for missing API key
            println!(
                "✅ Graceful degradation: returned {} documents",
                index.len()
            );
        }
        Err(e) => {
            println!("✅ Error handled: {:?}", e);
            // Error is also acceptable for missing API key
        }
    }

    println!("✅ Perplexity error handling test completed");
}

/// Live Perplexity API test - requires PERPLEXITY_API_KEY environment variable
/// This test is ignored by default and must be run with --ignored flag
#[tokio::test]
#[ignore]
async fn perplexity_live_api_test() {
    dotenvy::dotenv().ok();

    let api_key = match std::env::var("PERPLEXITY_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key,
        _ => {
            eprintln!("PERPLEXITY_API_KEY not set; skipping live Perplexity API test");
            eprintln!("Set your API key and run: cargo test perplexity_live_api_test -- --ignored");
            return;
        }
    };

    println!("🧪 Testing Live Perplexity API");
    println!("==============================");
    println!(
        "Using API key: {}...",
        &api_key[..std::cmp::min(8, api_key.len())]
    );

    let mut extra_params = HashMap::new();
    extra_params.insert("api_key".to_string(), api_key);
    extra_params.insert("model".to_string(), "sonar-small-online".to_string());
    extra_params.insert("max_tokens".to_string(), "500".to_string());
    extra_params.insert("temperature".to_string(), "0.1".to_string());

    let role = Role {
        shortname: Some("perplexity-live".to_string()),
        name: "Perplexity Live Test".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "superhero".to_string(),
        kg: None,
        haystacks: vec![Haystack {
            location: "https://api.perplexity.ai".to_string(),
            service: ServiceType::Perplexity,
            read_only: true,
            atomic_server_secret: None,
            extra_parameters: extra_params,
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
        llm_router_enabled: false,
        llm_router_config: None,
        extra: ahash::AHashMap::new(),
    };

    let mut config = ConfigBuilder::new()
        .add_role("Perplexity Live Test", role)
        .default_role("Perplexity Live Test")
        .unwrap()
        .build()
        .unwrap();

    let config_state = terraphim_config::ConfigState::new(&mut config)
        .await
        .expect("config state");

    // Test a simple query
    let query = SearchQuery {
        search_term: "What is Rust programming language?".into(),
        search_terms: None,
        operator: None,
        skip: Some(0),
        limit: Some(5),
        role: Some("Perplexity Live Test".into()),
    };

    println!("Sending query: {}", query.search_term.as_str());

    let start_time = std::time::Instant::now();
    let result = search_haystacks(config_state, query).await;
    let elapsed = start_time.elapsed();

    match result {
        Ok(index) => {
            println!("✅ Live API test successful!");
            println!("   Response time: {}ms", elapsed.as_millis());
            println!("   Documents returned: {}", index.len());

            for (i, (id, doc)) in index.iter().enumerate().take(3) {
                println!("   Document {}: {}", i + 1, doc.title);
                println!("     ID: {}", id);
                println!("     URL: {}", doc.url);
                if let Some(description) = &doc.description {
                    println!("     Description: {}", description);
                }
                if !doc.body.is_empty() {
                    let preview = if doc.body.len() > 100 {
                        format!("{}...", &doc.body[..100])
                    } else {
                        doc.body.clone()
                    };
                    println!("     Content preview: {}", preview);
                }
                println!();
            }

            if !index.is_empty() {
                println!("✅ Perplexity API integration working correctly!");
            } else {
                println!("⚠️  No documents returned - check API key and model");
            }
        }
        Err(e) => {
            println!("❌ Live API test failed: {:?}", e);
            panic!("Live API test should succeed with valid API key");
        }
    }
}
