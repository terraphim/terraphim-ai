use std::time::Duration;
use terraphim_mcp_server::auth::{ApiKey, AuthManager};

#[tokio::test]
async fn test_mcp_authentication_system() {
    // Create auth manager
    let auth_manager = AuthManager::new();

    // Create a test API key
    let api_key = ApiKey::new("test_key_123", "Test API Key")
        .with_permissions(vec!["tool:list".to_string(), "tool:call".to_string()])
        .with_rate_limit(100, Duration::from_secs(3600));

    // Add the API key to the manager
    auth_manager.add_api_key(api_key).await;

    // Test API key validation
    let result = auth_manager.validate_api_key("test_key_123").await;
    assert!(result.is_ok());

    let validated_key = result.unwrap();
    assert_eq!(validated_key.key, "test_key_123");
    assert_eq!(validated_key.name, "Test API Key");

    // Test permission check
    let has_list_perm = auth_manager
        .check_permission("test_key_123", "tool:list")
        .await;
    assert!(has_list_perm.is_ok());
    assert!(has_list_perm.unwrap());

    let has_admin_perm = auth_manager
        .check_permission("test_key_123", "tool:admin")
        .await;
    assert!(has_admin_perm.is_ok());
    assert!(!has_admin_perm.unwrap());

    // Test rate limiting
    let within_limit = auth_manager.check_rate_limit("test_key_123").await;
    assert!(within_limit.is_ok());
    assert!(within_limit.unwrap());

    // Test invalid API key
    let invalid_result = auth_manager.validate_api_key("invalid_key").await;
    assert!(invalid_result.is_err());

    println!("✅ MCP authentication system tests passed!");
}

#[tokio::test]
async fn test_mcp_rate_limiting() {
    let auth_manager = AuthManager::new();

    // Create API key with very low rate limit for testing
    let api_key =
        ApiKey::new("limited_key", "Limited Key").with_rate_limit(2, Duration::from_secs(60));

    auth_manager.add_api_key(api_key).await;

    // First two requests should pass
    assert!(auth_manager.check_rate_limit("limited_key").await.unwrap());
    assert!(auth_manager.check_rate_limit("limited_key").await.unwrap());

    // Third request should be rate limited
    assert!(!auth_manager.check_rate_limit("limited_key").await.unwrap());

    println!("✅ MCP rate limiting tests passed!");
}

#[tokio::test]
async fn test_mcp_api_key_generation() {
    let auth_manager = AuthManager::new();

    // Generate a new API key
    let generated_key = auth_manager.generate_api_key("Generated Key").await;

    // Verify the generated key exists and is valid
    let result = auth_manager.validate_api_key(&generated_key).await;
    assert!(result.is_ok());

    let api_key = result.unwrap();
    assert_eq!(api_key.name, "Generated Key");
    assert_eq!(api_key.key, generated_key);

    println!("✅ MCP API key generation tests passed!");
}
