//! Integration tests for config wiring to tools.
//!
//! Tests that configuration values from files are properly passed to tools.

mod common;

use terraphim_tinyclaw::config::{Config, ToolsConfig, WebToolsConfig};
use terraphim_tinyclaw::tools::create_default_registry;

/// Test that web tools configuration is wired through to the registry.
#[tokio::test]
async fn test_web_tools_config_wired_to_registry() {
    common::scrub_env();

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
    let registry = create_default_registry(None, web_tools_config, None).await;

    // Verify web_search tool is present
    let web_search = registry.get("web_search");
    assert!(web_search.is_some(), "web_search tool should be registered");

    // Verify web_fetch tool is present
    let web_fetch = registry.get("web_fetch");
    assert!(web_fetch.is_some(), "web_fetch tool should be registered");
}

/// Test that registry works with no web tools config.
#[tokio::test]
async fn test_registry_without_web_tools_config() {
    common::scrub_env();

    // Create registry without web tools config
    let registry = create_default_registry(None, None, None).await;

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
#[tokio::test]
async fn test_all_expected_tools_registered() {
    common::scrub_env();

    let registry = create_default_registry(None, None, None).await;

    let expected_tools = [
        "filesystem",
        "edit",
        "shell",
        "web_search",
        "web_fetch",
        "voice_transcribe",
        "todo",
        "patch_parse",
        "clarify",
        "process",
    ];

    for tool_name in &expected_tools {
        assert!(
            registry.get(tool_name).is_some(),
            "Tool '{}' should be registered",
            tool_name
        );
    }
}
