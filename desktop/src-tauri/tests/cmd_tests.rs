use terraphim_config::{ConfigBuilder, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::SearchQuery;

async fn create_test_config_state() -> ConfigState {
    let mut config = ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .unwrap();
    ConfigState::new(&mut config).await.unwrap()
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
    // Initialize memory-only persistence to avoid filesystem/network dependencies
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .unwrap();

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
