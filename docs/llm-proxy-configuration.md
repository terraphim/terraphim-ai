# LLM Proxy Configuration Guide

## Overview

Terraphim AI supports using proxy services for LLM providers, including the z.ai proxy for Anthropic models. This guide explains how to configure and use LLM proxies in your Terraphim deployment.

## Supported Proxy Providers

### z.ai Proxy for Anthropic Models

The z.ai proxy provides an alternative endpoint for Anthropic Claude models with enhanced performance and reliability.

**Environment Variables:**
```bash
# Base URL for the z.ai proxy
ANTHROPIC_BASE_URL=https://api.z.ai/api/anthropic

# Authentication token for z.ai proxy
ANTHROPIC_AUTH_TOKEN=your-z-ai-auth-token

# Optional: Fallback direct Anthropic API key
ANTHROPIC_API_KEY=your-anthropic-api-key
```

### OpenRouter Proxy

You can configure a custom proxy for OpenRouter requests.

**Environment Variables:**
```bash
# Custom OpenRouter proxy endpoint
OPENROUTER_BASE_URL=https://your-proxy.example.com/api/v1

# OpenRouter API key
OPENROUTER_API_KEY=your-openrouter-api-key
```

### Ollama Custom Endpoint

Configure Ollama to use a custom endpoint instead of the default local instance.

**Environment Variables:**
```bash
# Custom Ollama endpoint
OLLAMA_BASE_URL=http://your-ollama-server:11434

# Optional: Custom model name
OLLAMA_MODEL_NAME=llama3.1
```

## Configuration Methods

### Method 1: Environment Variables (Recommended)

Set the environment variables directly in your shell or configuration:

```bash
export ANTHROPIC_BASE_URL=https://api.z.ai/api/anthropic
export ANTHROPIC_AUTH_TOKEN=your-z-ai-auth-token
export OPENROUTER_API_KEY=your-openrouter-api-key
```

### Method 2: 1Password Integration

Use Terraphim's 1Password integration for secure credential management:

```bash
# Generate configuration from template
op inject -i templates/env.terraphim.template -o .env.terraphim

# Source the environment
source .env.terraphim

# Run Terraphim
cargo run
```

### Method 3: Configuration File

Create a role configuration file with proxy settings:

```json
{
  "name": "AI Engineer with Proxy",
  "relevance_function": "TerraphimGraph",
  "extra": {
    "llm_provider": "openrouter",
    "llm_auto_summarize": true,
    "anthropic_base_url": "https://api.z.ai/api/anthropic",
    "anthropic_auth_token": "your-z-ai-auth-token"
  },
  "haystacks": [
    {
      "name": "Local Code",
      "service": "Ripgrep",
      "extra_parameters": {
        "path": "./src"
      }
    }
  ]
}
```

## Usage Examples

### Basic Proxy Usage

```rust
use terraphim_service::llm_proxy::{LlmProxyClient, ProxyConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create proxy client with auto-configuration
    let client = LlmProxyClient::new("anthropic".to_string())?;

    // Log current configuration
    client.log_configuration();

    // Test connectivity
    let results = client.test_all_connectivity().await;
    for (provider, result) in results {
        match result {
            Ok(success) => println!("{}: {}", provider, success),
            Err(e) => println!("{}: Error - {}", provider, e),
        }
    }

    Ok(())
}
```

### Custom Proxy Configuration

```rust
use terraphim_service::llm_proxy::{LlmProxyClient, ProxyConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = LlmProxyClient::new("anthropic".to_string())?;

    // Create custom proxy configuration
    let config = ProxyConfig::new(
        "anthropic".to_string(),
        "claude-3-sonnet-20240229".to_string(),
    )
    .with_base_url("https://api.z.ai/api/anthropic".to_string())
    .with_api_key("your-auth-token".to_string())
    .with_timeout(Duration::from_secs(60))
    .with_fallback(true);

    // Apply configuration
    client.configure(config);

    // Use the proxy
    if client.is_using_proxy("anthropic") {
        println!("Using proxy for Anthropic requests");
        let effective_url = client.get_effective_url("anthropic")
            .expect("Failed to get effective URL");
        println!("Effective URL: {}", effective_url);
    }

    Ok(())
}
```

### OpenRouter Service with z.ai Proxy

```rust
use terraphim_service::openrouter::OpenRouterService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The service automatically detects z.ai proxy configuration
    let service = OpenRouterService::new(
        "your-api-key",
        "anthropic/claude-3-sonnet-20240229"
    )?;

    // Generate summary using z.ai proxy
    let summary = service.generate_summary(
        "Your content to summarize...",
        200
    ).await?;

    println!("Summary: {}", summary);

    Ok(())
}
```

## Troubleshooting

### Common Issues

#### 1. Proxy Not Detected

**Problem:** The service is not using the configured proxy.

**Solution:**
- Verify environment variables are set correctly
- Check that the model name matches the expected pattern
- Ensure the provider name is correct

```bash
# Check environment variables
env | grep ANTHROPIC

# Test proxy detection
echo "ANTHROPIC_BASE_URL: $ANTHROPIC_BASE_URL"
echo "ANTHROPIC_AUTH_TOKEN: $ANTHROPIC_AUTH_TOKEN"
```

#### 2. Authentication Failures

**Problem:** Requests are failing with authentication errors.

**Solution:**
- Verify the auth token is correct and active
- Check that `ANTHROPIC_AUTH_TOKEN` is set when using z.ai proxy
- Ensure the token has the required permissions

```bash
# Test authentication with curl
curl -H "x-api-key: $ANTHROPIC_AUTH_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"model":"claude-3-sonnet-20240229","messages":[{"role":"user","content":"test"}],"max_tokens":10}' \
     "$ANTHROPIC_BASE_URL/v1/messages"
```

#### 3. Connection Timeouts

**Problem:** Requests to the proxy are timing out.

**Solution:**
- Check network connectivity to the proxy endpoint
- Verify firewall rules allow connections to the proxy
- Increase timeout values in configuration

```rust
let config = ProxyConfig::new("anthropic".to_string(), "claude-3-sonnet".to_string())
    .with_timeout(Duration::from_secs(120)); // Increase timeout
```

#### 4. Fallback Not Working

**Problem:** Fallback to direct endpoints is not working when proxy fails.

**Solution:**
- Ensure fallback is enabled in configuration
- Verify direct endpoint credentials are available
- Check logs for fallback attempts

```rust
let config = ProxyConfig::new("anthropic".to_string(), "claude-3-sonnet".to_string())
    .with_fallback(true); // Enable fallback
```

### Debug Mode

Enable debug logging to troubleshoot proxy issues:

```bash
# Set log level
export LOG_LEVEL=debug

# Run with logging
RUST_LOG=debug cargo run
```

### Testing Proxy Configuration

Use the built-in testing utilities:

```bash
# Test proxy connectivity
cargo test test_llm_proxy_auto_configuration -- --nocapture

# Test OpenRouter proxy integration
cargo test test_anthropic_model_with_z_ai_proxy -- --nocapture

# Test all proxy functionality
cargo test --test llm_proxy_integration_test -- --nocapture
```

## Performance Considerations

### Proxy vs Direct Access

- **z.ai Proxy:** Generally offers better performance and reliability for Anthropic models
- **Direct Access:** Use as fallback when proxy is unavailable
- **Latency:** Proxy may add small latency overhead but provides better throughput

### Timeout Configuration

Configure appropriate timeouts based on your use case:

```rust
// Fast responses (chat completion)
let fast_config = ProxyConfig::new("anthropic".to_string(), "claude-3-haiku".to_string())
    .with_timeout(Duration::from_secs(30));

// Longer processing (document summarization)
let slow_config = ProxyConfig::new("anthropic".to_string(), "claude-3-sonnet".to_string())
    .with_timeout(Duration::from_secs(120));
```

### Connection Pooling

The proxy client automatically handles connection pooling and reuse for optimal performance.

## Security Considerations

### Credential Management

- Use 1Password integration for secure credential storage
- Never hardcode API keys in source code
- Rotate auth tokens regularly
- Use environment-specific configurations

### Network Security

- Ensure TLS/SSL is used for all proxy connections
- Verify proxy endpoint certificates
- Consider using VPN for additional security

### Access Control

- Limit proxy access to authorized applications
- Use service accounts with minimal required permissions
- Monitor proxy usage and access logs

## Advanced Configuration

### Multiple Proxy Endpoints

Configure different proxies for different providers:

```rust
let mut client = LlmProxyClient::new("anthropic".to_string())?;

// z.ai proxy for Anthropic
let anthropic_config = ProxyConfig::new("anthropic".to_string(), "claude-3-sonnet".to_string())
    .with_base_url("https://api.z.ai/api/anthropic".to_string())
    .with_api_key("z-ai-token".to_string());

// Custom proxy for OpenRouter
let openrouter_config = ProxyConfig::new("openrouter".to_string(), "anthropic/claude-3-sonnet".to_string())
    .with_base_url("https://proxy.example.com/openrouter".to_string())
    .with_api_key("openrouter-token".to_string());

client.configure(anthropic_config);
client.configure(openrouter_config);
```

### Load Balancing

For high-availability deployments, configure multiple proxy endpoints:

```rust
// Implement load balancing logic
let proxy_urls = vec![
    "https://proxy1.example.com/api/anthropic",
    "https://proxy2.example.com/api/anthropic",
    "https://proxy3.example.com/api/anthropic",
];

let selected_url = proxy_urls[hash(&request_id) % proxy_urls.len()];
```

### Monitoring and Metrics

Track proxy performance and reliability:

```rust
// Log proxy usage
log::info!("Proxy request: {} -> {}", provider, effective_url);

// Monitor response times
let start = std::time::Instant::now();
let response = make_request().await?;
let duration = start.elapsed();

log::debug!("Proxy response time: {}ms", duration.as_millis());
```

## Migration Guide

### From Direct API Access

1. **Backup Configuration:** Save your current configuration
2. **Get Proxy Credentials:** Obtain z.ai proxy credentials
3. **Update Environment Variables:** Set `ANTHROPIC_BASE_URL` and `ANTHROPIC_AUTH_TOKEN`
4. **Test Configuration:** Run connectivity tests
5. **Update Deployments:** Apply changes to production

### From Other Proxy Services

1. **Export Current Configuration:** Note current proxy settings
2. **Update URLs:** Change base URLs to z.ai endpoints
3. **Update Authentication:** Switch to z.ai auth tokens
4. **Validate Functionality:** Test all LLM operations
5. **Monitor Performance:** Compare with previous proxy service

## Support

For issues with LLM proxy configuration:

1. Check this documentation for common solutions
2. Review the troubleshooting section
3. Test with the provided test utilities
4. Check the Terraphim AI documentation
5. Contact support with specific error messages and configuration details

## Additional Resources

- [Terraphim AI Documentation](https://docs.terraphim.ai)
- [1Password Integration Guide](../README_1PASSWORD_INTEGRATION.md)
- [OpenRouter API Documentation](https://openrouter.ai/docs)
- [z.ai Proxy Documentation](https://docs.z.ai)
