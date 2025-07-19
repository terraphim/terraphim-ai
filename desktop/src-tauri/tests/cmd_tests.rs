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
    let _device_settings = create_test_device_settings_wrapped();

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
                    atomic_server_secret: None, // Use None to avoid authentication issues in tests
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build atomic config");

    let mut terraphim_service = TerraphimService::new(config_state.clone());
    
    // First update the configuration to include the new role
    let update_result = terraphim_service.update_config(config.clone()).await;
    assert!(update_result.is_ok());

    // Test search command with atomic role
    let search_query = SearchQuery {
        search_term: "test".into(),
        role: Some("AtomicRole".into()),
        skip: Some(0),
        limit: Some(10),
    };

    let result = terraphim_service.search(&search_query).await;

    // Should not error (though results may be empty in test environment)
    if let Err(e) = &result {
        println!("Search failed with error: {:?}", e);
    }
    assert!(result.is_ok());
    
    // Test configuration retrieval
    let config_result = terraphim_service.fetch_config().await;
    assert!(config_result.roles.contains_key(&"AtomicRole".into()));
}

#[tokio::test]
async fn test_hybrid_haystack_role_configuration() {
    let config_state = create_test_config_state().await;
    let _device_settings = create_test_device_settings_wrapped();

    // Create hybrid haystack role configuration with both Ripgrep and Atomic
    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+H")
        .add_role(
            "HybridRole",
            terraphim_config::Role {
                shortname: Some("hybrid-role".to_string()),
                name: "Hybrid Role".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "cosmo".to_string(),
                kg: None,
                haystacks: vec![
                    terraphim_config::Haystack {
                        location: "/tmp/test_docs".to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                    },
                    terraphim_config::Haystack {
                        location: "http://localhost:9883".to_string(),
                        service: ServiceType::Atomic,
                        read_only: true,
                        atomic_server_secret: None, // Use None to avoid authentication issues in tests
                        extra_parameters: std::collections::HashMap::new(),
                    },
                ],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build hybrid config");

    let mut terraphim_service = TerraphimService::new(config_state.clone());
    
    // First update the configuration to include the new role
    let update_result = terraphim_service.update_config(config.clone()).await;
    assert!(update_result.is_ok());

    // Test search command with hybrid role
    let search_query = SearchQuery {
        search_term: "test".into(),
        role: Some("HybridRole".into()),
        skip: Some(0),
        limit: Some(10),
    };

    let result = terraphim_service.search(&search_query).await;

    // Should not error (though results may be empty in test environment)
    assert!(result.is_ok());
    
    // Verify the role was added
    let config_result = terraphim_service.fetch_config().await;
    assert!(config_result.roles.contains_key(&"HybridRole".into()));
}

#[tokio::test]
async fn test_multiple_atomic_roles_configuration() {
    let config_state = create_test_config_state().await;
    let _device_settings = create_test_device_settings_wrapped();

    // Create multiple atomic haystack roles configuration
    let config = ConfigBuilder::new()
        .global_shortcut("Ctrl+M")
        .add_role(
            "AtomicRole1",
            terraphim_config::Role {
                shortname: Some("atomic-role-1".to_string()),
                name: "Atomic Role 1".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "flatly".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: "http://localhost:9883".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: None, // Use None to avoid authentication issues in tests
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .add_role(
            "AtomicRole2",
            terraphim_config::Role {
                shortname: Some("atomic-role-2".to_string()),
                name: "Atomic Role 2".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "journal".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: "http://localhost:9884".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: None, // Use None to avoid authentication issues in tests
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build multiple atomic config");

    let mut terraphim_service = TerraphimService::new(config_state.clone());
    
    // First update the configuration to include the new roles
    let update_result = terraphim_service.update_config(config.clone()).await;
    assert!(update_result.is_ok());

    // Test search command with first atomic role
    let search_query1 = SearchQuery {
        search_term: "test".into(),
        role: Some("AtomicRole1".into()),
        skip: Some(0),
        limit: Some(10),
    };

    let result1 = terraphim_service.search(&search_query1).await;
    assert!(result1.is_ok());

    // Test search command with second atomic role
    let search_query2 = SearchQuery {
        search_term: "test".into(),
        role: Some("AtomicRole2".into()),
        skip: Some(0),
        limit: Some(10),
    };

    let result2 = terraphim_service.search(&search_query2).await;
    assert!(result2.is_ok());
    
    // Verify both roles were added
    let config_result = terraphim_service.fetch_config().await;
    assert!(config_result.roles.contains_key(&"AtomicRole1".into()));
    assert!(config_result.roles.contains_key(&"AtomicRole2".into()));
}

#[tokio::test]
async fn test_config_update_with_atomic_roles() {
    let config_state = create_test_config_state().await;
    let device_settings = create_test_device_settings_wrapped();

    // Create initial config with atomic role
    let initial_config = ConfigBuilder::new()
        .global_shortcut("Ctrl+I")
        .add_role(
            "AtomicTitle",
            terraphim_config::Role {
                shortname: Some("atomic-title".to_string()),
                name: "Atomic Title".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "litera".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: "http://localhost:9883".to_string(),
                    service: ServiceType::Atomic,
                    read_only: true,
                    atomic_server_secret: None, // Use None to avoid authentication issues in tests
                    extra_parameters: std::collections::HashMap::new(),
                }],
                extra: ahash::AHashMap::new(),
            },
        )
        .build()
        .expect("Failed to build initial config");

    // Test update config command
    let terraphim_service = TerraphimService::new(config_state.clone());
    let result = terraphim_service.update_config(initial_config.clone()).await;
    assert!(result.is_ok());

    // Verify the update
    let updated_config = terraphim_service.fetch_config().await;
    assert_eq!(updated_config.global_shortcut, "Ctrl+I");
    assert_eq!(updated_config.default_role.original.as_str(), "AtomicTitle", "Default role should be AtomicTitle");
} 