use ahash::AHashMap;
use std::collections::HashMap;
use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_middleware::command::ripgrep::RipgrepCommand;
use terraphim_types::RelevanceFunction;

/// Test that demonstrates the security improvement: atomic_server_secret is not serialized for Ripgrep haystacks
#[tokio::test]
async fn test_ripgrep_haystack_security_no_atomic_secret_exposed() {
    let ripgrep_haystack = Haystack {
        location: "fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: Some("secret-that-should-not-be-serialized".to_string()),
        extra_parameters: HashMap::new(),
    };

    // Serialize the haystack
    let serialized = serde_json::to_string(&ripgrep_haystack).unwrap();

    // The atomic_server_secret should NOT be present in the serialized JSON for Ripgrep services
    assert!(!serialized.contains("atomic_server_secret"));
    assert!(!serialized.contains("secret-that-should-not-be-serialized"));

    // But other fields should be present
    assert!(serialized.contains("fixtures/haystack"));
    assert!(serialized.contains("Ripgrep"));
    assert!(serialized.contains("read_only"));

    println!(
        "✅ Ripgrep haystack serialized without atomic secret: {}",
        serialized
    );
}

/// Test that demonstrates atomic haystacks still include the secret when present
#[tokio::test]
async fn test_atomic_haystack_includes_secret_when_present() {
    let atomic_haystack = Haystack {
        location: "http://localhost:9883".to_string(),
        service: ServiceType::Atomic,
        read_only: true,
        atomic_server_secret: Some("valid-atomic-secret".to_string()),
        extra_parameters: HashMap::new(),
    };

    // Serialize the haystack
    let serialized = serde_json::to_string(&atomic_haystack).unwrap();

    // The atomic_server_secret SHOULD be present for Atomic services
    assert!(serialized.contains("atomic_server_secret"));
    assert!(serialized.contains("valid-atomic-secret"));

    println!("✅ Atomic haystack serialized with secret: {}", serialized);
}

/// Test that demonstrates atomic haystacks exclude the secret when it's None
#[tokio::test]
async fn test_atomic_haystack_excludes_none_secret() {
    let atomic_haystack = Haystack {
        location: "http://localhost:9883".to_string(),
        service: ServiceType::Atomic,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: HashMap::new(),
    };

    // Serialize the haystack
    let serialized = serde_json::to_string(&atomic_haystack).unwrap();

    // The atomic_server_secret should NOT be present when None
    assert!(!serialized.contains("atomic_server_secret"));

    println!(
        "✅ Atomic haystack serialized without None secret: {}",
        serialized
    );
}

/// Test extra parameters functionality for ripgrep tag filtering
#[tokio::test]
async fn test_ripgrep_extra_parameters_tag_filtering() {
    let mut extra_params = HashMap::new();
    extra_params.insert("tag".to_string(), "#rust".to_string());
    extra_params.insert("max_count".to_string(), "5".to_string());
    extra_params.insert("context".to_string(), "2".to_string());

    let haystack = Haystack {
        location: "fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    // Test parameter parsing
    let command = RipgrepCommand::default();
    let parsed_args = command.parse_extra_parameters(haystack.get_extra_parameters());

    // Should contain the tag filter with --all-match and -e pattern
    assert!(parsed_args.contains(&"--all-match".to_string()));
    assert!(parsed_args.contains(&"-e".to_string()));
    assert!(parsed_args.contains(&"#rust".to_string()));

    // Should contain max count
    assert!(parsed_args.contains(&"--max-count".to_string()));
    assert!(parsed_args.contains(&"5".to_string()));

    // Should contain context override
    assert!(parsed_args.contains(&"-C".to_string()));
    assert!(parsed_args.contains(&"2".to_string()));

    println!("✅ Parsed ripgrep args: {:?}", parsed_args);
}

/// Test extra parameters functionality with different parameter types
#[tokio::test]
async fn test_ripgrep_extra_parameters_various_types() {
    let mut extra_params = HashMap::new();
    extra_params.insert("type".to_string(), "rs".to_string());
    extra_params.insert("glob".to_string(), "*.md".to_string());
    extra_params.insert("case_sensitive".to_string(), "true".to_string());

    let command = RipgrepCommand::default();
    let parsed_args = command.parse_extra_parameters(&extra_params);

    // Should contain type filter
    assert!(parsed_args.contains(&"-t".to_string()));
    assert!(parsed_args.contains(&"rs".to_string()));

    // Should contain glob pattern
    assert!(parsed_args.contains(&"--glob".to_string()));
    assert!(parsed_args.contains(&"*.md".to_string()));

    // Should contain case sensitive flag
    assert!(parsed_args.contains(&"--case-sensitive".to_string()));

    println!(
        "✅ Parsed ripgrep args for various types: {:?}",
        parsed_args
    );
}

/// Test that extra parameters are included in serialization when not empty
#[tokio::test]
async fn test_extra_parameters_serialization() {
    let mut extra_params = HashMap::new();
    extra_params.insert("tag".to_string(), "#rust".to_string());
    extra_params.insert("max_count".to_string(), "10".to_string());

    let haystack = Haystack {
        location: "fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let serialized = serde_json::to_string(&haystack).unwrap();

    // Should contain extra_parameters
    assert!(serialized.contains("extra_parameters"));
    assert!(serialized.contains("#rust"));
    assert!(serialized.contains("max_count"));
    assert!(serialized.contains("10"));

    println!("✅ Haystack with extra parameters: {}", serialized);
}

/// Test that empty extra parameters are excluded from serialization
#[tokio::test]
async fn test_empty_extra_parameters_excluded() {
    let haystack = Haystack {
        location: "fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: HashMap::new(),
    };

    let serialized = serde_json::to_string(&haystack).unwrap();

    // Should NOT contain extra_parameters when empty
    assert!(!serialized.contains("extra_parameters"));

    println!("✅ Haystack without extra parameters: {}", serialized);
}

/// Test haystack builder methods for easier configuration
#[tokio::test]
async fn test_haystack_builder_methods() {
    let haystack = Haystack::new("fixtures/haystack".to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameter("tag".to_string(), "#rust".to_string())
        .with_extra_parameter("max_count".to_string(), "5".to_string());

    assert_eq!(haystack.location, "fixtures/haystack");
    assert_eq!(haystack.service, ServiceType::Ripgrep);
    assert!(haystack.read_only);
    assert_eq!(haystack.atomic_server_secret, None);
    assert_eq!(
        haystack.extra_parameters.get("tag"),
        Some(&"#rust".to_string())
    );
    assert_eq!(
        haystack.extra_parameters.get("max_count"),
        Some(&"5".to_string())
    );

    println!("✅ Haystack builder methods work correctly");
}

/// Test that atomic secrets are only set for Atomic service haystacks
#[tokio::test]
async fn test_atomic_secret_only_for_atomic_service() {
    let ripgrep_haystack =
        Haystack::new("fixtures/haystack".to_string(), ServiceType::Ripgrep, true)
            .with_atomic_secret(Some("secret".to_string()));

    // Secret should not be set for Ripgrep service
    assert_eq!(ripgrep_haystack.atomic_server_secret, None);

    let atomic_haystack = Haystack::new(
        "http://localhost:9883".to_string(),
        ServiceType::Atomic,
        true,
    )
    .with_atomic_secret(Some("secret".to_string()));

    // Secret should be set for Atomic service
    assert_eq!(
        atomic_haystack.atomic_server_secret,
        Some("secret".to_string())
    );

    println!("✅ Atomic secrets only set for Atomic service");
}

/// Integration test demonstrating complete workflow with extra parameters
#[tokio::test]
async fn test_complete_ripgrep_workflow_with_extra_parameters() {
    let mut extra_params = HashMap::new();
    extra_params.insert("tag".to_string(), "#rust".to_string());

    let role = Role {
        shortname: Some("RustDeveloper".to_string()),
        name: "Rust Developer".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "rust".to_string(),
        kg: None,
        haystacks: vec![Haystack {
            location: "fixtures/haystack".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            atomic_server_secret: None,
            extra_parameters: extra_params,
        }],
        #[cfg(feature = "openrouter")]
        llm_enabled: false,
        #[cfg(feature = "openrouter")]
        llm_api_key: None,
        #[cfg(feature = "openrouter")]
        llm_model: None,
        #[cfg(feature = "openrouter")]
        llm_auto_summarize: false,
        #[cfg(feature = "openrouter")]
        llm_chat_enabled: false,
        #[cfg(feature = "openrouter")]
        llm_chat_system_prompt: None,
        #[cfg(feature = "openrouter")]
        llm_chat_model: None,
        #[cfg(feature = "openrouter")]
        llm_context_window: None,
        extra: AHashMap::new(),
    };

    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+R")
        .add_role("RustDeveloper", role)
        .build()
        .unwrap();

    // Serialize the config to ensure no secrets are exposed
    let serialized_config = serde_json::to_string(&config).unwrap();
    assert!(!serialized_config.contains("atomic_server_secret"));
    assert!(serialized_config.contains("extra_parameters"));
    assert!(serialized_config.contains("#rust"));

    println!("✅ Complete workflow test passed");
    println!(
        "Config preview: {}",
        &serialized_config[..std::cmp::min(200, serialized_config.len())]
    );
}
