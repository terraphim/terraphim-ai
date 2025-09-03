/// Centralized HTTP client creation and configuration for Terraphim services
///
/// This module provides shared HTTP client creation to avoid duplication across
/// the codebase and ensure consistent configuration for timeouts, headers, and
/// other HTTP client settings.
use reqwest::Client;
use std::time::Duration;

/// Default timeout for HTTP requests (30 seconds)
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default user agent for Terraphim HTTP clients
pub const DEFAULT_USER_AGENT: &str = concat!(
    "Terraphim/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/terraphim/terraphim-ai)"
);

/// Create a default HTTP client with standard configuration
///
/// This client includes:
/// - 30-second timeout for requests
/// - Terraphim user agent header
/// - Standard SSL/TLS configuration
/// - Connection pooling and keep-alive
///
/// Use this for most HTTP operations where no special configuration is needed.
pub fn create_default_client() -> reqwest::Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .user_agent(DEFAULT_USER_AGENT)
        .build()
}

/// Create an HTTP client with custom timeout
///
/// Use this when you need a different timeout than the default 30 seconds.
/// For example, use shorter timeouts for health checks or longer timeouts
/// for large file downloads.
pub fn create_client_with_timeout(timeout_secs: u64) -> reqwest::Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(DEFAULT_USER_AGENT)
        .build()
}

/// Create an HTTP client with custom configuration
///
/// This provides maximum flexibility for specialized use cases like:
/// - Custom headers (API keys, authentication)
/// - Proxy configuration
/// - Custom SSL/TLS settings
/// - Different timeouts and retry policies
///
/// # Example
///
/// ```rust
/// use terraphim_service::http_client::create_custom_client;
/// use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
/// use std::time::Duration;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token"));
///
/// let client = create_custom_client(
///     Some(Duration::from_secs(60)),
///     Some(headers),
///     None
/// )?;
/// ```
pub fn create_custom_client(
    timeout: Option<Duration>,
    default_headers: Option<reqwest::header::HeaderMap>,
    proxy: Option<reqwest::Proxy>,
) -> reqwest::Result<Client> {
    let mut builder = Client::builder().user_agent(DEFAULT_USER_AGENT);

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

/// Create an HTTP client optimized for API calls
///
/// This client is configured for typical REST API usage:
/// - Shorter timeout (10 seconds) for responsive APIs
/// - JSON content type header
/// - Accept JSON responses
pub fn create_api_client() -> reqwest::Result<Client> {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(DEFAULT_USER_AGENT)
        .default_headers(headers)
        .build()
}

/// Create an HTTP client optimized for web scraping
///
/// This client is configured for scraping web pages:
/// - Longer timeout (60 seconds) for slow websites
/// - Browser-like headers to avoid blocking
/// - HTML content acceptance
pub fn create_scraping_client() -> reqwest::Result<Client> {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};

    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );

    Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("Mozilla/5.0 (compatible; Terraphim/1.0; +https://terraphim.ai)")
        .default_headers(headers)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_client() {
        let client = create_default_client();
        assert!(client.is_ok(), "Default client creation should succeed");
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
    fn test_create_api_client() {
        let client = create_api_client();
        assert!(client.is_ok(), "API client creation should succeed");
    }

    #[test]
    fn test_create_scraping_client() {
        let client = create_scraping_client();
        assert!(client.is_ok(), "Scraping client creation should succeed");
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
}
