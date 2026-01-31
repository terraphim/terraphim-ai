//! Tests for TuiService methods
//!
//! These tests verify that the core TuiService methods work correctly.

use anyhow::Result;
use serial_test::serial;
use std::ffi::{OsStr, OsString};
use terraphim_agent::service::TuiService;
use terraphim_settings::DeviceSettings;
use terraphim_types::RoleName;

#[cfg(rust_has_unsafe_env_setters)]
fn set_env_var(key: &'static str, value: impl AsRef<OsStr>) {
    unsafe {
        std::env::set_var(key, value);
    }
}

#[cfg(not(rust_has_unsafe_env_setters))]
fn set_env_var(key: &'static str, value: impl AsRef<OsStr>) {
    std::env::set_var(key, value);
}

#[cfg(rust_has_unsafe_env_setters)]
fn remove_env_var(key: &'static str) {
    unsafe {
        std::env::remove_var(key);
    }
}

#[cfg(not(rust_has_unsafe_env_setters))]
fn remove_env_var(key: &'static str) {
    std::env::remove_var(key);
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        let original = std::env::var_os(key);
        set_env_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            set_env_var(self.key, value);
        } else {
            remove_env_var(self.key);
        }
    }
}

/// Test that TuiService can be created and basic methods work
#[tokio::test]
async fn test_tui_service_creation() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;

    // Get the current config
    let config = service.get_config().await;
    let roles_count = config.roles.len();
    println!("Configuration has {} roles", roles_count);

    // Get the selected role
    let selected_role = service.get_selected_role().await;
    assert!(
        !selected_role.to_string().is_empty(),
        "Should have a selected role"
    );

    Ok(())
}

/// Ensure the real constructor loads device settings and config paths
#[tokio::test]
#[serial]
async fn test_tui_service_new_uses_host_settings_path() -> Result<()> {
    let temp_home = tempfile::tempdir()?;
    let _home_guard = EnvVarGuard::set("HOME", temp_home.path());

    let xdg_config_home = temp_home.path().join(".config");
    let _xdg_guard = EnvVarGuard::set("XDG_CONFIG_HOME", &xdg_config_home);

    let data_path = temp_home.path().join(".terraphim");
    let _data_guard = EnvVarGuard::set("TERRAPHIM_DATA_PATH", &data_path);

    let service = TuiService::new().await?;

    let config_dir = DeviceSettings::default_config_path();
    let settings_file = config_dir.join("settings.toml");
    assert!(
        settings_file.exists(),
        "TuiService::new should bootstrap host settings at {:?}",
        settings_file
    );

    assert!(
        !service.get_config().await.roles.is_empty(),
        "Service initialized via TuiService::new should still load embedded roles"
    );

    Ok(())
}

/// Test the search method with default role
#[tokio::test]
async fn test_tui_service_search() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;

    // Search with the default search method (uses selected role)
    let results = service.search("test", Some(5)).await;

    // Search may return empty or results depending on data, but should not panic
    match results {
        Ok(docs) => {
            println!("Search returned {} documents", docs.len());
        }
        Err(e) => {
            // Expected if no haystack data is available
            println!("Search returned error (expected if no data): {}", e);
        }
    }

    Ok(())
}

/// Test autocomplete method
#[tokio::test]
async fn test_tui_service_autocomplete() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let role_name = service.get_selected_role().await;

    // Autocomplete may fail if no thesaurus is loaded, which is expected
    let results = service.autocomplete(&role_name, "test", Some(5)).await;

    match results {
        Ok(suggestions) => {
            println!("Autocomplete returned {} suggestions", suggestions.len());
            for suggestion in &suggestions {
                println!("  - {} (score: {})", suggestion.term, suggestion.score);
            }
        }
        Err(e) => {
            // Expected if no thesaurus data is available
            println!("Autocomplete returned error (expected if no data): {}", e);
        }
    }

    Ok(())
}

/// Test replace_matches method
#[tokio::test]
async fn test_tui_service_replace_matches() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let role_name = service.get_selected_role().await;

    let text = "This is a test with some terms to replace.";
    let link_type = terraphim_automata::LinkType::HTMLLinks;

    // Replace matches may fail if no thesaurus is loaded
    let result = service.replace_matches(&role_name, text, link_type).await;

    match result {
        Ok(replaced_text) => {
            println!("Replace matches result: {}", replaced_text);
        }
        Err(e) => {
            // Expected if no thesaurus data is available
            println!(
                "Replace matches returned error (expected if no data): {}",
                e
            );
        }
    }

    Ok(())
}

/// Test summarize method
#[tokio::test]
async fn test_tui_service_summarize() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let role_name = service.get_selected_role().await;

    let content = "This is a test paragraph that needs to be summarized. It contains multiple sentences with various topics and information that should be condensed.";

    // Summarize will fail if no LLM is configured, which is expected in tests
    let result = service.summarize(&role_name, content).await;

    match result {
        Ok(summary) => {
            println!("Summary: {}", summary);
        }
        Err(e) => {
            // Expected if no LLM is configured
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("No LLM configured") || error_msg.contains("LLM"),
                "Should indicate LLM not configured: {}",
                error_msg
            );
            println!("Summarize returned expected error (no LLM): {}", e);
        }
    }

    Ok(())
}

/// Test list roles with info
#[tokio::test]
async fn test_tui_service_list_roles_with_info() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;

    let roles = service.list_roles_with_info().await;

    // Should return role names with optional shortnames
    for (name, shortname) in &roles {
        println!("Role: {} (shortname: {:?})", name, shortname);
    }

    Ok(())
}

/// Test find_matches method
#[tokio::test]
async fn test_tui_service_find_matches() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let role_name = service.get_selected_role().await;

    let text = "This is a test paragraph with some terms to match.";

    let result = service.find_matches(&role_name, text).await;

    match result {
        Ok(matches) => {
            println!("Found {} matches", matches.len());
            for m in &matches {
                println!("  - {} at position {:?}", m.term, m.pos);
            }
        }
        Err(e) => {
            // Expected if no thesaurus data is available
            println!("Find matches returned error (expected if no data): {}", e);
        }
    }

    Ok(())
}

/// Test that role discovery works with shortnames and case-insensitive lookups
#[tokio::test]
async fn test_tui_service_find_role_by_shortname() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let roles = service.list_roles_with_info().await;

    let (role_name, shortname) = roles
        .into_iter()
        .find(|(_, short)| short.is_some())
        .expect("Embedded config should include at least one shortname");

    let shortname = shortname.expect("shortname already verified as Some");

    let found = service
        .find_role_by_name_or_shortname(&shortname)
        .await
        .expect("Should find role by its shortname");
    assert_eq!(found, RoleName::new(&role_name));

    // Ensure lookup is case-insensitive
    let found_upper = service
        .find_role_by_name_or_shortname(&shortname.to_uppercase())
        .await
        .expect("Should find role ignoring case");
    assert_eq!(found_upper, RoleName::new(&role_name));

    Ok(())
}

/// Test that updating the selected role persists across service queries
#[tokio::test]
async fn test_tui_service_update_selected_role() -> Result<()> {
    let service = TuiService::new_with_embedded_defaults().await?;
    let current_role = service.get_selected_role().await;

    let new_role = service
        .list_roles_with_info()
        .await
        .into_iter()
        .map(|(name, _)| RoleName::new(&name))
        .find(|role| role != &current_role)
        .expect("Embedded config should contain multiple roles");

    let updated_config = service
        .update_selected_role(new_role.clone())
        .await
        .expect("Should update selected role");
    assert_eq!(updated_config.selected_role, new_role);

    let persisted_role = service.get_selected_role().await;
    assert_eq!(persisted_role, new_role);

    Ok(())
}
