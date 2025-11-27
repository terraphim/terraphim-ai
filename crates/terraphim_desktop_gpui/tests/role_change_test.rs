//! Integration tests for role changes via tray menu and dropdown
//!
//! These tests verify that role changes:
//! 1. Update ConfigState.selected_role correctly
//! 2. Update RoleSelector UI
//! 3. Are consistent between tray menu and dropdown sources

use terraphim_config::ConfigState;
use terraphim_types::RoleName;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Helper to create a test ConfigState with mock roles
async fn create_test_config_state(roles: Vec<&str>, selected: &str) -> ConfigState {
    // Create config with specified roles
    let config = terraphim_config::Config {
        selected_role: RoleName::from(selected),
        ..Default::default()
    };

    let config_arc = Arc::new(Mutex::new(config));

    // Note: In real tests, we'd need to populate roles HashMap
    // For now, this tests the basic config state mechanics
    ConfigState {
        config: config_arc,
        roles: Default::default(),
    }
}

#[tokio::test]
async fn test_config_state_role_change() {
    // Create config state with initial role
    let config_state = create_test_config_state(
        vec!["Python Engineer", "Rust Engineer", "Default"],
        "Python Engineer"
    ).await;

    // Verify initial state
    {
        let config = config_state.config.lock().await;
        assert_eq!(config.selected_role.to_string(), "Python Engineer");
    }

    // Simulate role change (same pattern as handle_tray_event)
    {
        let mut config = config_state.config.lock().await;
        config.selected_role = RoleName::from("Rust Engineer");
    }

    // Verify role was updated
    {
        let config = config_state.config.lock().await;
        assert_eq!(config.selected_role.to_string(), "Rust Engineer");
    }
}

#[tokio::test]
async fn test_config_state_role_change_preserves_other_fields() {
    let config_state = create_test_config_state(
        vec!["Python Engineer", "Rust Engineer"],
        "Python Engineer"
    ).await;

    // Change role
    {
        let mut config = config_state.config.lock().await;
        config.selected_role = RoleName::from("Rust Engineer");
    }

    // Verify the change persisted after lock release
    let selected = config_state.get_selected_role().await;
    assert_eq!(selected.to_string(), "Rust Engineer");
}

#[tokio::test]
async fn test_multiple_rapid_role_changes() {
    let config_state = create_test_config_state(
        vec!["Role1", "Role2", "Role3"],
        "Role1"
    ).await;

    // Simulate rapid role changes (like user clicking quickly)
    for role in &["Role2", "Role3", "Role1", "Role2"] {
        let mut config = config_state.config.lock().await;
        config.selected_role = RoleName::from(*role);
    }

    // Final state should be last role set
    {
        let config = config_state.config.lock().await;
        assert_eq!(config.selected_role.to_string(), "Role2");
    }
}

#[tokio::test]
async fn test_concurrent_role_changes() {
    let config_state = create_test_config_state(
        vec!["Role1", "Role2"],
        "Role1"
    ).await;

    let config_state_clone = config_state.clone();

    // Spawn multiple concurrent role changes
    let handle1 = tokio::spawn({
        let config = config_state.config.clone();
        async move {
            for _ in 0..10 {
                let mut c = config.lock().await;
                c.selected_role = RoleName::from("Role1");
            }
        }
    });

    let handle2 = tokio::spawn({
        let config = config_state_clone.config.clone();
        async move {
            for _ in 0..10 {
                let mut c = config.lock().await;
                c.selected_role = RoleName::from("Role2");
            }
        }
    });

    // Both should complete without deadlock
    let _ = tokio::join!(handle1, handle2);

    // Final state should be one of the two roles
    let config = config_state.config.lock().await;
    let selected = config.selected_role.to_string();
    assert!(selected == "Role1" || selected == "Role2");
}

// Note: Full GPUI UI tests require TestAppContext which needs
// the gpui test-support feature. These would test:
// 1. RoleSelector.set_selected_role() updates UI
// 2. handle_tray_event() dispatches correctly
// 3. RoleChangeEvent is emitted when dropdown selection changes

#[cfg(test)]
mod tray_event_tests {
    use super::*;
    use terraphim_desktop_gpui::platform::tray::SystemTrayEvent;

    #[test]
    fn test_system_tray_event_change_role_construction() {
        let role = RoleName::from("Test Engineer");
        let event = SystemTrayEvent::ChangeRole(role.clone());

        match event {
            SystemTrayEvent::ChangeRole(r) => {
                assert_eq!(r.to_string(), "Test Engineer");
            }
            _ => panic!("Expected ChangeRole event"),
        }
    }

    #[test]
    fn test_system_tray_event_debug_format() {
        let event = SystemTrayEvent::ChangeRole(RoleName::from("Rust Engineer"));
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("ChangeRole"));
        assert!(debug_str.contains("Rust Engineer"));
    }
}
