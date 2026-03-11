//! Integration tests for config wiring to tools.
//!
//! Tests that configuration values from files are properly passed to tools.

use terraphim_tinyclaw::config::{Config, ToolsConfig, WebToolsConfig};
use terraphim_tinyclaw::tools::create_default_registry;

/// Test that web tools configuration is wired through to the registry.
#[test]
fn test_web_tools_config_wired_to_registry() {
    // Create a config with specific web tools settings
    let config = Config {
        tools: ToolsConfig {
            web: Some(WebToolsConfig {
                search_provider: Some("exa".to_string()),
                fetch_mode: Some("readability".to_string()),
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    // Create registry with the web tools config
    let web_tools_config = config.tools.web.as_ref();
    let registry = create_default_registry(None, web_tools_config);

    // Verify web_search tool is present
    let web_search = registry.get("web_search");
    assert!(web_search.is_some(), "web_search tool should be registered");

    // Verify web_fetch tool is present
    let web_fetch = registry.get("web_fetch");
    assert!(web_fetch.is_some(), "web_fetch tool should be registered");
}

/// Test that registry works with no web tools config.
#[test]
fn test_registry_without_web_tools_config() {
    // Create registry without web tools config
    let registry = create_default_registry(None, None);

    // Verify web_search tool is still present (with defaults)
    let web_search = registry.get("web_search");
    assert!(
        web_search.is_some(),
        "web_search tool should be registered even without config"
    );

    // Verify web_fetch tool is still present (with defaults)
    let web_fetch = registry.get("web_fetch");
    assert!(
        web_fetch.is_some(),
        "web_fetch tool should be registered even without config"
    );
}

/// Test that all expected tools are registered.
#[test]
fn test_all_expected_tools_registered() {
    let registry = create_default_registry(None, None);

    let expected_tools = [
        "filesystem",
        "edit",
        "shell",
        "web_search",
        "web_fetch",
        "voice_transcribe",
    ];

    for tool_name in &expected_tools {
        assert!(
            registry.get(tool_name).is_some(),
            "Tool '{}' should be registered",
            tool_name
        );
    }
}
