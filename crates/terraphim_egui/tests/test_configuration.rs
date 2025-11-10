//! Configuration tests
//!
//! These tests verify configuration loading, role switching,
//! and configuration validation.

use std::sync::{Arc, Mutex};
use terraphim_config::Role;
use terraphim_egui::state::AppState;
use terraphim_types::RoleName;

/// Test config file loading
#[tokio::test]
async fn test_config_file_loading() {
    let state = AppState::new();

    // Verify initial state has a default role
    let current_role = state.get_current_role();
    assert!(
        !current_role.name.to_string().is_empty(),
        "Should have a default role name"
    );
}

/// Test role switching
#[tokio::test]
async fn test_role_switching() {
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_arc);

    // Get initial role
    let initial_role_name = {
        let state = state_ref.lock().unwrap();
        let name = state.get_current_role().name.clone();
        name
    };
    let initial_role = initial_role_name;

    // Switch to Rust Engineer role
    let rust_role = Role::new("Rust Engineer");
    state_ref.lock().unwrap().set_current_role(rust_role);

    // Verify role changed
    let new_role_name = {
        let state = state_ref.lock().unwrap();
        let name = state.get_current_role().name.clone();
        name
    };
    assert_eq!(new_role_name, RoleName::new("Rust Engineer"));
    assert_ne!(new_role_name, initial_role);

    // Switch to another role
    let dev_role = Role::new("Developer");
    state_ref.lock().unwrap().set_current_role(dev_role);

    // Verify second switch worked
    let current_role_name = {
        let state = state_ref.lock().unwrap();
        let name = state.get_current_role().name.clone();
        name
    };
    assert_eq!(current_role_name, RoleName::new("Developer"));
}

/// Test role name validation
#[tokio::test]
async fn test_role_name_validation() {
    // Test valid role names
    let role1 = Role::new("Rust Engineer");
    assert_eq!(role1.name, RoleName::new("Rust Engineer"));

    let role2 = Role::new("System Administrator");
    assert_eq!(role2.name, RoleName::new("System Administrator"));

    // Test with different casing
    let role3 = Role::new("TERRAPHIM ENGINEER");
    assert_eq!(role3.name, RoleName::new("TERRAPHIM ENGINEER"));

    let role4 = Role::new("terraphim engineer");
    assert_eq!(role4.name, RoleName::new("terraphim engineer"));
}

/// Test multiple role switches
#[tokio::test]
async fn test_multiple_role_switches() {
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_arc);

    let roles = vec![
        "Rust Engineer",
        "System Operator",
        "Data Scientist",
        "DevOps Engineer",
        "Product Manager",
    ];

    // Switch through multiple roles
    for role_name in roles {
        let role = Role::new(role_name);
        state_ref.lock().unwrap().set_current_role(role);
    }

    // Verify final role
    let final_role_name = {
        let state = state_ref.lock().unwrap();
        let name = state.get_current_role().name.clone();
        name
    };
    assert_eq!(final_role_name, RoleName::new("Product Manager"));
}
