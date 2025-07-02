use std::sync::Arc;
use tokio::sync::Mutex;

use terraphim_ai_desktop::{
    search, get_config, update_config,
    Status
};
use terraphim_config::{ConfigBuilder, ConfigState};
use terraphim_settings::DeviceSettings;
use terraphim_types::{SearchQuery, RoleName};
use terraphim_persistence::memory::create_test_device_settings as create_memory_settings;

async fn create_test_config_state() -> ConfigState {
    let mut config = ConfigBuilder::new().build_default_desktop().build().unwrap();
    ConfigState::new(&mut config).await.unwrap()
}

fn create_test_device_settings_wrapped() -> Arc<Mutex<DeviceSettings>> {
    let settings = create_memory_settings().unwrap();
    Arc::new(Mutex::new(settings))
}

// Note: These tests need to be run in a Tauri app context to work properly
// The State<'_, ConfigState> type requires the Tauri runtime
// For now, we'll skip these tests until we can set up proper Tauri test context

#[tokio::test]
#[ignore]
async fn test_search_command() {
    // This test requires Tauri State context
    // TODO: Set up proper Tauri test environment
}

#[tokio::test]
#[ignore]
async fn test_get_config_command() {
    // This test requires Tauri State context
    // TODO: Set up proper Tauri test environment
}

#[tokio::test]
#[ignore]
async fn test_update_config_command() {
    // This test requires Tauri State context
    // TODO: Set up proper Tauri test environment
}

#[tokio::test]
#[ignore]
async fn test_search_with_empty_query() {
    // This test requires Tauri State context
    // TODO: Set up proper Tauri test environment
}

#[tokio::test]
#[ignore]
async fn test_search_with_large_limit() {
    // This test requires Tauri State context
    // TODO: Set up proper Tauri test environment
}

#[tokio::test]
#[ignore]
async fn test_basic_integration() {
    // This test requires Tauri State context
    // TODO: Set up proper Tauri test environment
} 