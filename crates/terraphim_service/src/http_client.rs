/// Centralized HTTP client creation and configuration for Terraphim services
///
/// This module provides shared HTTP client instances with connection pooling
/// to avoid creating new connections for each request. This significantly
/// improves performance for repeated API calls by reusing TCP connections.
use std::time::Duration;

use once_cell::sync::Lazy;
use reqwest::Client;

/// Default timeout for HTTP requests (30 seconds)
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default user agent for Terraphim HTTP clients
pub const DEFAULT_USER_AGENT: &str = concat!(
    "Terraphim/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/terraphim/terraphim-ai)"
);

/// Default connection pool settings
///
/// These values are tuned for typical API usage patterns:
/// - 10 idle connections per host allows for concurrent requests
/// - 90 second idle timeout balances connection reuse vs resource cleanup
const POOL_MAX_IDLE_PER_HOST: usize = 10;
const POOL_IDLE_TIMEOUT_SECS: u64 = 90;

/// Global default HTTP client with connection pooling
///
/// This client is lazily initialized on first use and reused for all
/// default HTTP operations. It includes:
/// - 30-second timeout for requests
/// - Terraphim user agent header
/// - Connection pooling (10 idle per host, 90s timeout)
static DEFAULT_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .user_agent(DEFAULT_USER_AGENT)
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT_SECS))
        .build()
        .expect("Failed to build default HTTP client")
});

/// Global API HTTP client with connection pooling and JSON headers
///
/// Optimized for REST API calls with:
/// - 10-second timeout for responsive APIs
/// - JSON content type and accept headers
/// - Connection pooling for repeated API calls
static API_CLIENT: Lazy<Client> = Lazy::new(|| {
    use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(DEFAULT_USER_AGENT)
        .default_headers(headers)
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT_SECS))
        .build()
        .expect("Failed to build API HTTP client")
});

/// Global web scraping HTTP client with connection pooling
///
/// Optimized for web scraping with:
/// - 60-second timeout for slow websites
/// - Browser-like user agent
/// - HTML content acceptance
static SCRAPING_CLIENT: Lazy<Client> = Lazy::new(|| {
    use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};

    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );

    Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("Mozilla/5.0 (compatible; Terraphim/1.0; +https://terraphim.ai)")
        .default_headers(headers)
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT_SECS))
        .build()
        .expect("Failed to build scraping HTTP client")
});

/// Get the global default HTTP client with connection pooling
///
/// This client includes:
/// - 30-second timeout for requests
/// - Terraphim user agent header
/// - Connection pooling and keep-alive
///
/// Use this for most HTTP operations where no special configuration is needed.
pub fn get_default_client() -> &'static Client {
    &DEFAULT_CLIENT
}

/// Get an HTTP client with custom timeout
///
/// Note: This creates a new client instance. For better performance,
/// prefer `get_default_client()` when possible.
pub fn create_client_with_timeout(timeout_secs: u64) -> reqwest::Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(DEFAULT_USER_AGENT)
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT_SECS))
        .build()
}

/// Get the global API HTTP client with JSON headers and connection pooling
///
/// This client is configured for typical REST API usage:
/// - Shorter timeout (10 seconds) for responsive APIs
/// - JSON content type header
/// - Accept JSON responses
/// - Connection pooling for repeated API calls
///
/// Use this for LLM API calls and other JSON-based APIs.
pub fn get_api_client() -> &'static Client {
    &API_CLIENT
}

/// Create a custom HTTP client with specific configuration
///
/// Note: This creates a new client instance. For better performance,
/// prefer the global clients when possible.
///
/// Use this for specialized use cases like:
/// - Custom headers (API keys, authentication)
/// - Proxy configuration
/// - Custom SSL/TLS settings
pub fn create_custom_client(
    timeout: Option<Duration>,
    default_headers: Option<reqwest::header::HeaderMap>,
    proxy: Option<reqwest::Proxy>,
) -> reqwest::Result<Client> {
    let mut builder = Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT_SECS));

    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    } else {
        builder = builder.timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    }

    if let Some(headers) = default_headers {
        builder = builder.default_headers(headers);
    }

    if let Some(proxy) = proxy {
        builder = builder.proxy(proxy);
    }

    builder.build()
}

/// Get the global web scraping HTTP client with connection pooling
///
/// This client is configured for scraping web pages:
/// - Longer timeout (60 seconds) for slow websites
/// - Browser-like headers to avoid blocking
/// - HTML content acceptance
/// - Connection pooling for repeated requests
///
/// Use this for web scraping operations.
pub fn get_scraping_client() -> &'static Client {
    &SCRAPING_CLIENT
}

// Backwards compatibility aliases - these return Result for compatibility
// with existing code that uses `?` or `unwrap()`

/// Backwards compatibility: returns a clone of the default client
///
/// This function returns `Ok(Client)` for full backwards compatibility.
/// The client is cheap to clone (internally Arc-based).
pub fn create_default_client() -> reqwest::Result<Client> {
    Ok(get_default_client().clone())
}

/// Backwards compatibility: returns a clone of the API client
///
/// This function returns `Ok(Client)` for full backwards compatibility.
/// The client is cheap to clone (internally Arc-based).
pub fn create_api_client() -> reqwest::Result<Client> {
    Ok(get_api_client().clone())
}

/// Backwards compatibility: returns a clone of the scraping client
///
/// This function returns `Ok(Client)` for full backwards compatibility.
/// The client is cheap to clone (internally Arc-based).
pub fn create_scraping_client() -> reqwest::Result<Client> {
    Ok(get_scraping_client().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_client() {
        let client = get_default_client();
        // Verify we get the same instance (singleton)
        let client2 = get_default_client();
        assert!(
            std::ptr::eq(client, client2),
            "Should return same client instance"
        );
    }

    #[test]
    fn test_get_api_client() {
        let client = get_api_client();
        let client2 = get_api_client();
        assert!(
            std::ptr::eq(client, client2),
            "Should return same API client instance"
        );
    }

    #[test]
    fn test_get_scraping_client() {
        let client = get_scraping_client();
        let client2 = get_scraping_client();
        assert!(
            std::ptr::eq(client, client2),
            "Should return same scraping client instance"
        );
    }

    #[test]
    fn test_create_client_with_timeout() {
        let client = create_client_with_timeout(5);
        assert!(
            client.is_ok(),
            "Client with custom timeout should be created"
        );
    }

    #[test]
    fn test_create_custom_client_minimal() {
        let client = create_custom_client(None, None, None);
        assert!(client.is_ok(), "Custom client with no options should work");
    }

    #[test]
    fn test_user_agent_contains_version() {
        assert!(DEFAULT_USER_AGENT.contains("Terraphim/"));
        assert!(DEFAULT_USER_AGENT.contains("https://github.com/terraphim/terraphim-ai"));
    }

    #[test]
    fn test_backwards_compatibility() {
        // Ensure old API still works (returns Result with owned Client)
        let _client = create_default_client().unwrap();
        let _api_client = create_api_client().unwrap();
        let _scraping_client = create_scraping_client().unwrap();

        // All should be valid clients (clone of global instances)
        // Note: Client uses Arc internally, so clones share the same connection pool
        // Just verify they were created successfully
    }
}
