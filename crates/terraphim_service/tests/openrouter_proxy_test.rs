//! Tests for OpenRouter service z.ai proxy integration
//!
//! These tests verify that the OpenRouter service correctly uses the z.ai proxy
//! when ANTHROPIC_BASE_URL and ANTHROPIC_AUTH_TOKEN environment variables are set.

#[cfg(feature = "openrouter")]
use std::env;
#[cfg(feature = "openrouter")]
use terraphim_service::openrouter::OpenRouterService;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_anthropic_model_with_z_ai_proxy() {
        let mut test_env = TestEnv::new();

        // Set up z.ai proxy environment variables
        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("ANTHROPIC_AUTH_TOKEN", "z-ai-auth-token-123");

        // Test Anthropic model detection
        let service = OpenRouterService::new("test-key", "anthropic/claude-3-sonnet-20240229");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Verify the service uses the z.ai proxy URL
        assert_eq!(service.base_url(), "https://api.z.ai/api/anthropic");
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_claude_model_with_z_ai_proxy() {
        let mut test_env = TestEnv::new();

        // Set up z.ai proxy environment variables
        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("ANTHROPIC_AUTH_TOKEN", "z-ai-auth-token-456");

        // Test Claude model detection (different naming)
        let service = OpenRouterService::new("test-key", "claude-3-haiku-20240307");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Verify the service uses the z.ai proxy URL
        assert_eq!(service.base_url(), "https://api.z.ai/api/anthropic");
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_non_anthropic_model_ignores_z_ai_proxy() {
        let mut test_env = TestEnv::new();

        // Set up z.ai proxy environment variables
        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("ANTHROPIC_AUTH_TOKEN", "z-ai-auth-token-789");

        // Test non-Anthropic model (should use OpenRouter)
        let service = OpenRouterService::new("test-key", "openai/gpt-4");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Should use default OpenRouter URL, not z.ai proxy
        assert_eq!(service.base_url(), "https://openrouter.ai/api/v1");
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_openrouter_base_url_override() {
        let mut test_env = TestEnv::new();

        // Set OpenRouter base URL override
        test_env.set_var(
            "OPENROUTER_BASE_URL",
            "https://custom-openrouter.example.com/api/v1",
        );

        let service = OpenRouterService::new("test-key", "openai/gpt-3.5-turbo");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Should use the custom OpenRouter URL
        assert_eq!(
            service.base_url(),
            "https://custom-openrouter.example.com/api/v1"
        );
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_z_ai_proxy_priority_over_openrouter() {
        let mut test_env = TestEnv::new();

        // Set both environment variables
        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("OPENROUTER_BASE_URL", "https://openrouter.ai/api/v1");

        // Test Anthropic model - should prefer z.ai proxy
        let service = OpenRouterService::new("test-key", "anthropic/claude-3-sonnet-20240229");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Should use z.ai proxy for Anthropic models
        assert_eq!(service.base_url(), "https://api.z.ai/api/anthropic");

        // Test non-Anthropic model - should use OpenRouter
        let service2 = OpenRouterService::new("test-key", "openai/gpt-4");
        assert!(service2.is_ok());

        let service2 = service2.unwrap();

        // Should use OpenRouter for non-Anthropic models
        assert_eq!(service2.base_url(), "https://openrouter.ai/api/v1");
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_api_key_selection_with_z_ai_proxy() {
        let mut test_env = TestEnv::new();

        // Set up z.ai proxy environment variables
        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("ANTHROPIC_AUTH_TOKEN", "z-ai-special-token");

        let service = OpenRouterService::new("fallback-key", "anthropic/claude-3-sonnet-20240229");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Test that the service would use ANTHROPIC_AUTH_TOKEN for z.ai proxy
        // We can't directly test the private method, but we can verify the URL detection
        assert!(service.base_url().contains("z.ai"));
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_no_environment_variables() {
        let mut test_env = TestEnv::new();

        // Ensure no proxy environment variables are set
        test_env.remove_var("ANTHROPIC_BASE_URL");
        test_env.remove_var("ANTHROPIC_AUTH_TOKEN");
        test_env.remove_var("OPENROUTER_BASE_URL");

        // Test default behavior
        let service = OpenRouterService::new("test-key", "anthropic/claude-3-sonnet-20240229");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Should use default OpenRouter URL when no proxy is configured
        assert_eq!(service.base_url(), "https://openrouter.ai/api/v1");
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_environment_variable_validation() {
        let mut test_env = TestEnv::new();

        // Test empty ANTHROPIC_BASE_URL
        test_env.set_var("ANTHROPIC_BASE_URL", "");
        test_env.remove_var("ANTHROPIC_AUTH_TOKEN");

        let service = OpenRouterService::new("test-key", "anthropic/claude-3-sonnet-20240229");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Should fall back to default when ANTHROPIC_BASE_URL is empty
        assert_eq!(service.base_url(), "https://openrouter.ai/api/v1");
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_model_name_patterns() {
        let mut test_env = TestEnv::new();

        // Set up z.ai proxy
        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token");

        // Test various Anthropic model patterns
        let test_cases = vec![
            "anthropic/claude-3-sonnet-20240229",
            "anthropic/claude-3-haiku-20240307",
            "anthropic/claude-3-opus-20240229",
            "claude-3-sonnet-20240229",
            "claude-3-haiku-20240307",
            "claude-instant-1.2",
        ];

        for model in test_cases {
            let service = OpenRouterService::new("test-key", model);
            assert!(
                service.is_ok(),
                "Failed to create service for model: {}",
                model
            );

            let service = service.unwrap();
            assert_eq!(
                service.base_url(),
                "https://api.z.ai/api/anthropic",
                "Model {} should use z.ai proxy",
                model
            );
        }
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_non_anthropic_model_patterns() {
        let mut test_env = TestEnv::new();

        // Set up z.ai proxy
        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token");

        // Test various non-Anthropic model patterns
        let test_cases = vec![
            "openai/gpt-4",
            "openai/gpt-3.5-turbo",
            "google/gemini-pro",
            "meta-llama/llama-3-70b-instruct",
            "mistralai/mixtral-8x7b-instruct",
        ];

        for model in test_cases {
            let service = OpenRouterService::new("test-key", model);
            assert!(
                service.is_ok(),
                "Failed to create service for model: {}",
                model
            );

            let service = service.unwrap();
            assert_eq!(
                service.base_url(),
                "https://openrouter.ai/api/v1",
                "Model {} should use OpenRouter, not z.ai proxy",
                model
            );
        }
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_service_configuration_methods() {
        let mut test_env = TestEnv::new();

        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token");

        let service = OpenRouterService::new("test-key", "anthropic/claude-3-sonnet-20240229");
        assert!(service.is_ok());

        let service = service.unwrap();

        // Test configuration methods
        assert!(service.is_configured());
        assert_eq!(service.get_model(), "anthropic/claude-3-sonnet-20240229");
    }

    #[cfg(feature = "openrouter")]
    #[test]
    fn test_error_cases() {
        // Test empty API key
        let service = OpenRouterService::new("", "anthropic/claude-3-sonnet-20240229");
        assert!(service.is_err());

        // Test empty model name
        let service = OpenRouterService::new("test-key", "");
        assert!(service.is_err());
    }

    #[cfg(feature = "openrouter")]
    #[tokio::test]
    async fn test_z_ai_proxy_url_detection() {
        let mut test_env = TestEnv::new();

        test_env.set_var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic");
        test_env.set_var("ANTHROPIC_AUTH_TOKEN", "test-token");

        let service =
            OpenRouterService::new("test-key", "anthropic/claude-3-sonnet-20240229").unwrap();

        // We can't directly test private methods, but we can verify the URL
        assert!(service.base_url().contains("z.ai"));

        // Verify the URL format is correct for z.ai proxy
        assert!(!service.base_url().ends_with("/"));
        assert!(service.base_url().starts_with("https://"));
    }
}
