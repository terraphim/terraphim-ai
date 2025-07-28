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