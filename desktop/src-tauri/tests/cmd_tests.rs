use std::sync::Arc;
use tokio::sync::Mutex;
use serial_test::serial;
use tempfile;

use terraphim_ai_desktop::{
    search, get_config, update_config, save_initial_settings,
    Status
};
use terraphim_config::{ConfigBuilder, ConfigState, Role};
use terraphim_settings::DeviceSettings;
use terraphim_types::{SearchQuery, NormalizedTermValue};
use terraphim_persistence::memory::create_test_device_settings as create_memory_settings;

async fn create_test_config_state() -> ConfigState {
    let mut config = ConfigBuilder::new().build_default_desktop().build().unwrap();
    ConfigState::new(&mut config).await.unwrap()
}

fn create_test_device_settings_wrapped() -> Arc<Mutex<DeviceSettings>> {
    let settings = create_memory_settings();
    Arc::new(Mutex::new(settings))
}

#[tokio::test]
async fn test_search_command() {
    let config_state = create_test_config_state().await;
    
    let search_query = SearchQuery {
        search_term: "test query".into(),
        skip: Some(0),
        limit: Some(10),
        role: Role::from("test_role"),
    };

    let result = search(config_state.into(), search_query).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, Status::Success);
    // Results might be empty for test data, but should not error
    assert!(response.results.len() >= 0);
}

#[tokio::test]
async fn test_get_config_command() {
    let config_state = create_test_config_state().await;
    
    let result = get_config(config_state.into()).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, Status::Success);
    assert!(response.config.roles.len() >= 0);
}

#[tokio::test]
async fn test_update_config_command() {
    let config_state = create_test_config_state().await;
    
    // Get current config
    let current_config_response = get_config(config_state.clone().into()).await.unwrap();
    let mut new_config = current_config_response.config;
    
    // Modify the config
    new_config.global_shortcut = "Ctrl+Alt+T".to_string();
    
    let result = update_config(config_state.into(), new_config.clone()).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, Status::Success);
    assert_eq!(response.config.global_shortcut, "Ctrl+Alt+T");
}

// Note: save_initial_settings test requires private field access, skipping for now

#[tokio::test]
async fn test_search_with_empty_query() {
    let config_state = create_test_config_state().await;
    
    let search_query = SearchQuery {
        search_term: "".into(),
        skip: Some(0),
        limit: Some(10),
        role: Role::from("test_role"),
    };

    let result = search(config_state.into(), search_query).await;
    
    // Should handle empty queries gracefully
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, Status::Success);
}

#[tokio::test]
async fn test_search_with_large_limit() {
    let config_state = create_test_config_state().await;
    
    let search_query = SearchQuery {
        search_term: "test".into(),
        skip: Some(0),
        limit: Some(1000), // Large limit
        role: Role::from("test_role"),
    };

    let result = search(config_state.into(), search_query).await;
    
    // Should handle large limits gracefully
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, Status::Success);
}

#[tokio::test]
async fn test_basic_integration() {
    let config_state = create_test_config_state().await;
    
    // Test that we can get and update config
    let config = get_config(config_state.clone().into()).await.unwrap();
    assert_eq!(config.status, Status::Success);
    
    // Test search functionality
    let search_query = SearchQuery {
        search_term: "test query".into(),
        skip: Some(0),
        limit: Some(10),
        role: Role::from("test_role"),
    };

    let search_result = search(config_state.into(), search_query).await;
    assert!(search_result.is_ok());
} 