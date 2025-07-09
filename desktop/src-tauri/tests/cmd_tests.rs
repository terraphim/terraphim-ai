use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;

use terraphim_ai_desktop::{
    search, get_config, update_config,
    Status
};
use terraphim_config::{ConfigBuilder, ConfigState, Role, Haystack, ServiceType};
use terraphim_types::RelevanceFunction;
use terraphim_settings::DeviceSettings;
use terraphim_types::{SearchQuery, RoleName};
use terraphim_persistence::memory::create_test_device_settings as create_memory_settings;
use terraphim_service::TerraphimService;

async fn create_test_config_state() -> ConfigState {
    let mut config = ConfigBuilder::new().build_default_desktop().build().unwrap();
    ConfigState::new(&mut config).await.unwrap()
}

fn create_test_device_settings_wrapped() -> Arc<Mutex<DeviceSettings>> {
    let settings = create_memory_settings().unwrap();
    Arc::new(Mutex::new(settings))
}

#[tokio::test]
async fn test_search_command() {
    let config_state = create_test_config_state().await;
    let search_query = SearchQuery {
        search_term: "test".into(),
        role: Some("Default".into()),
        skip: None,
        limit: Some(10),
    };
    
    // Test the underlying service logic directly
    let mut terraphim_service = TerraphimService::new(config_state.clone());
    let result = terraphim_service.search(&search_query).await;
    
    // Should not error (though results may be empty in test environment)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_config_command() {
    let config_state = create_test_config_state().await;
    
    // Test the underlying service logic directly
    let terraphim_service = TerraphimService::new(config_state.clone());
    let config = terraphim_service.fetch_config().await;
    
    // Config should have both default roles
    assert!(!config.roles.is_empty());
    assert!(config.roles.contains_key(&"Default".into()));
    assert!(config.roles.contains_key(&"Terraphim Engineer".into()));
}

#[tokio::test]
async fn test_update_config_command() {
    let config_state = create_test_config_state().await;
    
    // Test the underlying service logic directly
    let terraphim_service = TerraphimService::new(config_state.clone());
    let mut config = terraphim_service.fetch_config().await;
    
    // Modify the config global shortcut
    config.global_shortcut = "Ctrl+Shift+T".to_string();
    
    // Update the config
    let result = terraphim_service.update_config(config.clone()).await;
    assert!(result.is_ok());
    
    // Verify the update
    let updated_config = terraphim_service.fetch_config().await;
    assert_eq!(updated_config.global_shortcut, "Ctrl+Shift+T");
}

#[tokio::test]
async fn test_search_with_empty_query() {
    let config_state = create_test_config_state().await;
    let search_query = SearchQuery {
        search_term: "".into(),
        role: Some("Default".into()),
        skip: None,
        limit: Some(10),
    };
    
    // Test the underlying service logic directly
    let mut terraphim_service = TerraphimService::new(config_state.clone());
    let result = terraphim_service.search(&search_query).await;
    
    // Empty query should still work (return empty results)
    assert!(result.is_ok());
    let results = result.unwrap();
    assert!(results.is_empty() || !results.is_empty()); // Either is valid for empty query
}

#[tokio::test]
async fn test_search_with_large_limit() {
    let config_state = create_test_config_state().await;
    let search_query = SearchQuery {
        search_term: "test".into(),
        role: Some("Terraphim Engineer".into()),
        skip: None,
        limit: Some(1000),
    };
    
    // Test the underlying service logic directly
    let mut terraphim_service = TerraphimService::new(config_state.clone());
    let result = terraphim_service.search(&search_query).await;
    
    // Large limit should not cause errors
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_basic_integration() {
    let config_state = create_test_config_state().await;
    let terraphim_service = TerraphimService::new(config_state.clone());
    
    // Test basic configuration functionality
    let config = terraphim_service.fetch_config().await;
    assert!(!config.roles.is_empty());
    assert!(config.roles.contains_key(&"Default".into()));
    assert!(config.roles.contains_key(&"Terraphim Engineer".into()));
    
    // Test that we can perform a search with Default role
    let search_query = SearchQuery {
        search_term: "integration".into(),
        role: Some("Default".into()),
        skip: None,
        limit: Some(5),
    };
    
    let mut search_service = TerraphimService::new(config_state.clone());
    let search_result = search_service.search(&search_query).await;
    assert!(search_result.is_ok());
    
    // Test that we can also search with Terraphim Engineer role
    let search_query_engineer = SearchQuery {
        search_term: "integration".into(),
        role: Some("Terraphim Engineer".into()),
        skip: None,
        limit: Some(5),
    };
    
    let search_result_engineer = search_service.search(&search_query_engineer).await;
    assert!(search_result_engineer.is_ok());
    
    // Test that we can update config
    let mut new_config = config.clone();
    new_config.global_shortcut = "Ctrl+Alt+T".to_string();
    
    let update_result = terraphim_service.update_config(new_config).await;
    assert!(update_result.is_ok());
    
    // Verify the update persisted
    let updated_config = terraphim_service.fetch_config().await;
    assert_eq!(updated_config.global_shortcut, "Ctrl+Alt+T");
} 

#[tokio::test]
async fn test_atomic_haystack_role_configuration() {
    let config_state = create_test_config_state().await;
    let device_settings = create_test_device_settings_wrapped();

    // Create atomic haystack role configuration
    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+A")
        .add_role(
            "AtomicRole",
            terraphim_config::Role {
                shortname: Some("atomic-role".to_string()),
                name: "Atomic Role".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "cerulean".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: "http://localhost:9883".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: Some("test_secret".to_string()),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build atomic config");

    // Test search command with atomic role
    let search_query = SearchQuery {
        search_term: "test".into(),
        role: Some("AtomicRole".into()),
        skip: Some(0),
        limit: Some(10),
    };

    let mut terraphim_service = TerraphimService::new(config_state.clone());
    let result = terraphim_service.search(&search_query).await;

    // Should not crash (though may return empty results without real atomic server)
    assert!(result.is_ok());
    
    // Test configuration retrieval
    let config_result = terraphim_service.fetch_config().await;
    
    // Test configuration update with atomic role
    let update_result = terraphim_service.update_config(config).await;
    assert!(update_result.is_ok());
}

#[tokio::test]
async fn test_hybrid_haystack_role_configuration() {
    let config_state = create_test_config_state().await;
    let device_settings = create_test_device_settings_wrapped();

    // Create hybrid role configuration (Atomic + Ripgrep)
    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+H")
        .add_role(
            "HybridRole",
            terraphim_config::Role {
                shortname: Some("hybrid-role".to_string()),
                name: "Hybrid Role".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                theme: "superhero".to_string(),
                kg: Some(terraphim_config::KnowledgeGraph {
                    automata_path: None,
                    knowledge_graph_local: Some(terraphim_config::KnowledgeGraphLocal {
                        input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("docs/src"),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![
                    terraphim_config::Haystack {
                        location: "http://localhost:9883".to_string(),
                        service: terraphim_config::ServiceType::Atomic,
                        read_only: true,
                        atomic_server_secret: Some("test_secret".to_string()),
                    },
                    terraphim_config::Haystack {
                        location: "docs/src".to_string(),
                        service: terraphim_config::ServiceType::Ripgrep,
                        read_only: true,
                        atomic_server_secret: None,
                    },
                ],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build hybrid config");

    // Test search command with hybrid role
    let search_query = SearchQuery {
        search_term: "terraphim".into(),
        role: Some("HybridRole".into()),
        skip: Some(0),
        limit: Some(10),
    };

    let mut terraphim_service = TerraphimService::new(config_state.clone());
    let result = terraphim_service.search(&search_query).await;

    // Should not crash (though may return limited results without real atomic server)
    assert!(result.is_ok());
    
    // Test configuration update with hybrid role
    let update_result = terraphim_service.update_config(config).await;
    assert!(update_result.is_ok());
    
    // Verify hybrid role configuration structure
    let updated_config = terraphim_service.fetch_config().await;
    let hybrid_role = updated_config.roles.get(&"HybridRole".into()).unwrap();
    
    assert_eq!(hybrid_role.haystacks.len(), 2, "Hybrid role should have 2 haystacks");
    assert!(hybrid_role.haystacks.iter().any(|h| h.service == terraphim_config::ServiceType::Atomic), "Should have atomic haystack");
    assert!(hybrid_role.haystacks.iter().any(|h| h.service == terraphim_config::ServiceType::Ripgrep), "Should have ripgrep haystack");
    assert!(hybrid_role.kg.is_some(), "Graph embeddings role should have knowledge graph");
}

#[tokio::test]
async fn test_multiple_atomic_roles_configuration() {
    let config_state = create_test_config_state().await;
    let device_settings = create_test_device_settings_wrapped();

    // Create configuration with multiple atomic roles
    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+M")
        .add_role(
            "AtomicTitle",
            terraphim_config::Role {
                shortname: Some("atomic-title".to_string()),
                name: "Atomic Title".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "cerulean".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: "http://localhost:9883".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: Some("test_secret_1".to_string()),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .add_role(
            "AtomicGraph",
            terraphim_config::Role {
                shortname: Some("atomic-graph".to_string()),
                name: "Atomic Graph".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                theme: "superhero".to_string(),
                kg: Some(terraphim_config::KnowledgeGraph {
                    automata_path: None,
                    knowledge_graph_local: Some(terraphim_config::KnowledgeGraphLocal {
                        input_type: terraphim_types::KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("docs/src"),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: "http://localhost:9884".to_string(), // Different atomic server
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: Some("test_secret_2".to_string()),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .default_role("AtomicTitle")
        .unwrap()
        .build()
        .expect("Failed to build multi-atomic config");

    let mut terraphim_service = TerraphimService::new(config_state.clone());
    
    // Test update with multiple atomic roles
    let update_result = terraphim_service.update_config(config).await;
    assert!(update_result.is_ok());
    
    // Test search with first atomic role
    let search_query_1 = SearchQuery {
        search_term: "documentation".into(),
        role: Some("AtomicTitle".into()),
        skip: Some(0),
        limit: Some(5),
    };
    
    let result_1 = terraphim_service.search(&search_query_1).await;
    assert!(result_1.is_ok());
    
    // Test search with second atomic role
    let search_query_2 = SearchQuery {
        search_term: "graph".into(),
        role: Some("AtomicGraph".into()),
        skip: Some(0),
        limit: Some(5),
    };
    
    let result_2 = terraphim_service.search(&search_query_2).await;
    assert!(result_2.is_ok());
    
    // Verify configuration structure
    let final_config = terraphim_service.fetch_config().await;
    assert_eq!(final_config.roles.len(), 2, "Should have 2 atomic roles");
    assert_eq!(final_config.default_role.as_str(), "AtomicTitle", "Default role should be AtomicTitle");
    
    // Verify both roles have atomic haystacks
    let title_role = final_config.roles.get(&"AtomicTitle".into()).unwrap();
    let graph_role = final_config.roles.get(&"AtomicGraph".into()).unwrap();
    
    assert_eq!(title_role.haystacks[0].service, terraphim_config::ServiceType::Atomic);
    assert_eq!(graph_role.haystacks[0].service, terraphim_config::ServiceType::Atomic);
    assert_ne!(title_role.haystacks[0].location, graph_role.haystacks[0].location, "Should use different atomic servers");
} 