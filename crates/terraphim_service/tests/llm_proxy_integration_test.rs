//! Integration tests for LLM proxy functionality
//!
//! These tests verify that the z.ai proxy integration works correctly
//! with environment variables and fallback mechanisms.

use std::env;
use std::time::Duration;
use terraphim_service::llm_proxy::{LlmProxyClient, ProxyConfig};

/// Test environment variable cleanup utility
struct TestEnv {
    original_vars: Vec<(String, Option<String>)>,
}

impl TestEnv {
    fn new() -> Self {
        Self {
            original_vars: Vec::new(),
        }
    }

    fn set_var(&mut self, key: &str, value: &str) {
        // Store original value if it exists
        self.original_vars
            .push((key.to_string(), env::var(key).ok()));
        env::set_var(key, value);
    }

    fn remove_var(&mut self, key: &str) {
        self.original_vars
            .push((key.to_string(), env::var(key).ok()));
        env::remove_var(key);
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Restore original environment variables
        for (key, original_value) in self.original_vars.drain(..) {
            match original_value {
                Some(value) => env::set_var(&key, value),
                None => env::remove_var(&key),
            }
        }
    }
}

#[tokio::test]
async fn test_llm_proxy_auto_configuration() {
    let mut test_env = TestEnv::new();

    // Set up z.ai proxy environment variables
    test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
    test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token-123");

    // Create proxy client
    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();

    // Verify auto-configuration
    assert!(client
        .configured_providers()
        .contains(&"anthropic".to_string()));

    let config = client.get_config("anthropic").unwrap();
    assert_eq!(config.provider, "anthropic");
    assert_eq!(
        config.base_url.as_ref().unwrap(),
        "https://api.z.ai/api/anthropic"
    );
    assert_eq!(config.api_key.as_ref().unwrap(), "test-token-123");

    // Test effective URL resolution
    let effective_url = client.get_effective_url("anthropic").unwrap();
    assert_eq!(effective_url, "https://api.z.ai/api/anthropic");

    // Verify proxy detection
    assert!(client.is_using_proxy("anthropic"));
}

#[tokio::test]
async fn test_llm_proxy_fallback_mechanism() {
    // Test without proxy configuration (fallback to direct)
    let mut test_env = TestEnv::new();
    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();

    // Should fall back to direct endpoint
    let effective_url = client.get_effective_url("anthropic").unwrap();
    assert_eq!(effective_url, "https://api.anthropic.com");

    // Should not be using proxy
    assert!(!client.is_using_proxy("anthropic"));

    // Test environment variable removal functionality
    test_env.set_var("TEST_VAR", "test_value");
    assert_eq!(std::env::var("TEST_VAR").unwrap(), "test_value");

    // Remove the variable and verify it's gone
    test_env.remove_var("TEST_VAR");
    assert!(std::env::var("TEST_VAR").is_err());
}

#[tokio::test]
async fn test_multiple_provider_configuration() {
    let mut test_env = TestEnv::new();

    // Configure multiple providers
    test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
    test_env.set_var("ANTHROPIC_AUTH_TOKEN", "anthropic-token");
    test_env.set_var("OPENROUTER_BASE_URL", "https://proxy.openrouter.ai/api/v1");
    test_env.set_var("OPENROUTER_API_KEY", "openrouter-token");
    test_env.set_var("OLLAMA_BASE_URL", "http://custom-ollama:11434");

    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();

    // Verify all providers are configured
    let providers = client.configured_providers();
    assert!(providers.contains(&"anthropic".to_string()));
    assert!(providers.contains(&"openrouter".to_string()));
    assert!(providers.contains(&"ollama".to_string()));

    // Test effective URLs for each provider
    assert_eq!(
        client.get_effective_url("anthropic").unwrap(),
        "https://api.z.ai/api/anthropic"
    );
    assert_eq!(
        client.get_effective_url("openrouter").unwrap(),
        "https://proxy.openrouter.ai/api/v1"
    );
    assert_eq!(
        client.get_effective_url("ollama").unwrap(),
        "http://custom-ollama:11434"
    );
}

#[tokio::test]
async fn test_custom_proxy_configuration() {
    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();

    // Create custom proxy configuration
    let config = ProxyConfig::new(
        "anthropic".to_string(),
        "claude-3-sonnet-20240229".to_string(),
    )
    .with_base_url("https://custom-proxy.example.com/anthropic".to_string())
    .with_api_key("custom-api-key".to_string())
    .with_timeout(Duration::from_secs(60))
    .with_fallback(true);

    // Apply configuration
    let mut client = client;
    client.configure(config);

    // Verify configuration
    let stored_config = client.get_config("anthropic").unwrap();
    assert_eq!(stored_config.model, "claude-3-sonnet-20240229");
    assert_eq!(
        stored_config.base_url.as_ref().unwrap(),
        "https://custom-proxy.example.com/anthropic"
    );
    assert_eq!(stored_config.api_key.as_ref().unwrap(), "custom-api-key");
    assert_eq!(stored_config.timeout, Duration::from_secs(60));
    assert!(stored_config.enable_fallback);
}

#[tokio::test]
async fn test_connectivity_testing() {
    let mut test_env = TestEnv::new();

    // Set up test proxy configuration
    test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
    test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token");

    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();

    // Test connectivity (this will likely fail with test token, but should not panic)
    let result = client.test_connectivity("anthropic").await;

    // The result should be either Ok(false) (connection worked but auth failed)
    // or Err (network error), but never panic
    match result {
        Ok(success) => {
            println!("Connectivity test result: {}", success);
        }
        Err(e) => {
            println!("Connectivity test error: {}", e);
        }
    }

    // Test connectivity for all providers
    let results = client.test_all_connectivity().await;
    assert!(results.contains_key("anthropic"));
}

#[tokio::test]
async fn test_proxy_url_detection() {
    let mut test_env = TestEnv::new();

    // Test z.ai URL detection
    test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
    test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token");

    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();
    let config = client.get_config("anthropic").unwrap();

    assert!(config.base_url.clone().unwrap().contains("z.ai"));

    // Test non-z.ai URL
    test_env.set_var(
        "ANTHROPIC_BASE_URL",
        "https://api.other-provider.com/anthropic",
    );

    let client2 = LlmProxyClient::new("anthropic".to_string()).unwrap();
    let config2 = client2.get_config("anthropic").unwrap();

    assert!(!config2.base_url.clone().unwrap().contains("z.ai"));
}

#[tokio::test]
async fn test_configuration_logging() {
    let mut test_env = TestEnv::new();

    // Set up multiple providers
    test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
    test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token");
    test_env.set_var("OPENROUTER_API_KEY", "openrouter-token");

    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();

    // This should not panic and should log configuration
    client.log_configuration();

    // Verify default provider is set
    assert_eq!(client.default_provider, "anthropic");
}

#[test]
fn test_proxy_config_builder() {
    // Test configuration builder pattern
    let config = ProxyConfig::new("test-provider".to_string(), "test-model".to_string())
        .with_base_url("https://test-proxy.com".to_string())
        .with_api_key("test-key".to_string())
        .with_timeout(Duration::from_secs(45))
        .with_fallback(false);

    assert_eq!(config.provider, "test-provider");
    assert_eq!(config.model, "test-model");
    assert_eq!(config.base_url.unwrap(), "https://test-proxy.com");
    assert_eq!(config.api_key.unwrap(), "test-key");
    assert_eq!(config.timeout, Duration::from_secs(45));
    assert!(!config.enable_fallback);
}

#[test]
fn test_environment_variable_precedence() {
    let mut test_env = TestEnv::new();

    // Test that ANTHROPIC_AUTH_TOKEN takes precedence over ANTHROPIC_API_KEY
    test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
    test_env.set_var("ANTHROPIC_AUTH_TOKEN", "auth-token-value");
    test_env.set_var("ANTHROPIC_API_KEY", "api-key-value");

    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();
    let config = client.get_config("anthropic").unwrap();

    // Should prefer ANTHROPIC_AUTH_TOKEN
    assert_eq!(config.api_key.clone().unwrap(), "auth-token-value");
}

#[test]
fn test_error_handling() {
    // Test client creation with invalid provider
    let client = LlmProxyClient::new("invalid-provider".to_string());
    assert!(client.is_ok());

    let client = client.unwrap();

    // Test get_config for non-existent provider
    assert!(client.get_config("non-existent").is_none());

    // Test get_effective_url for unsupported provider
    assert!(client.get_effective_url("unsupported-provider").is_none());

    // Test is_using_proxy for non-existent provider
    assert!(!client.is_using_proxy("non-existent"));
}

#[tokio::test]
async fn test_concurrent_connectivity_tests() {
    let mut test_env = TestEnv::new();

    // Set up multiple providers
    test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
    test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token");
    test_env.set_var("OPENROUTER_API_KEY", "test-token");

    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();

    // Test concurrent connectivity checks
    let results = client.test_all_connectivity().await;

    // Should have results for all configured providers
    assert!(!results.is_empty());
    for provider in client.configured_providers() {
        assert!(results.contains_key(&provider));
    }
}

#[tokio::test]
async fn test_timeout_configuration() {
    let client = LlmProxyClient::new("anthropic".to_string()).unwrap();

    // Create configuration with short timeout for testing
    let config = ProxyConfig::new(
        "anthropic".to_string(),
        "claude-3-sonnet-20240229".to_string(),
    )
    .with_base_url("https://httpbin.org/delay/10".to_string()) // Will timeout
    .with_timeout(Duration::from_millis(100))
    .with_fallback(false);

    let mut client = client;
    client.configure(config);

    // Test connectivity with short timeout
    let start = std::time::Instant::now();
    let result = client.test_connectivity("anthropic").await;
    let duration = start.elapsed();

    // Should timeout quickly (under 5 seconds)
    assert!(duration < Duration::from_secs(5));

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected timeout/error: {}", e),
    }
}
