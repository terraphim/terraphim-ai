//! Webhook event system for external notifications.
//!
//! Provides a webhook dispatcher for sending notifications about significant
//! state changes with HMAC-SHA256 signing and automatic retries.
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_llm_proxy::webhooks::{WebhookDispatcher, WebhookSettings, WebhookEvent, WebhookEventType};
//!
//! let settings = WebhookSettings {
//!     enabled: true,
//!     url: "https://example.com/webhook".to_string(),
//!     secret: "my-secret".to_string(),
//!     events: vec!["oauth_refresh".to_string(), "circuit_breaker".to_string()],
//!     retry_count: 3,
//!     timeout_seconds: 5,
//! };
//!
//! let dispatcher = WebhookDispatcher::new(settings);
//!
//! let event = WebhookEvent::new(WebhookEventType::ProviderCircuitOpen {
//!     provider: "openrouter".to_string(),
//!     reason: "consecutive failures".to_string(),
//!     failure_count: 5,
//! });
//!
//! dispatcher.dispatch(event).await;
//! ```

pub mod events;
pub mod signing;

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub use events::{WebhookEvent, WebhookEventType};
pub use signing::{
    format_signature_header, parse_signature_header, sign_payload, verify_signature,
};

/// Webhook configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSettings {
    /// Whether webhooks are enabled.
    #[serde(default)]
    pub enabled: bool,

    /// URL to send webhook events to.
    #[serde(default)]
    pub url: String,

    /// Shared secret for HMAC signing.
    #[serde(default)]
    pub secret: String,

    /// Event categories to send (e.g., "oauth_refresh", "circuit_breaker").
    #[serde(default)]
    pub events: Vec<String>,

    /// Number of retry attempts on failure.
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,

    /// Timeout for webhook requests in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_retry_count() -> u32 {
    3
}

fn default_timeout() -> u64 {
    5
}

impl Default for WebhookSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            secret: String::new(),
            events: vec![],
            retry_count: default_retry_count(),
            timeout_seconds: default_timeout(),
        }
    }
}

/// Webhook delivery result.
#[derive(Debug, Clone)]
pub struct DeliveryResult {
    /// Whether delivery was successful.
    pub success: bool,

    /// HTTP status code (if request was made).
    pub status_code: Option<u16>,

    /// Error message (if failed).
    pub error: Option<String>,

    /// Number of attempts made.
    pub attempts: u32,
}

impl DeliveryResult {
    fn success(status_code: u16, attempts: u32) -> Self {
        Self {
            success: true,
            status_code: Some(status_code),
            error: None,
            attempts,
        }
    }

    fn failure(error: String, status_code: Option<u16>, attempts: u32) -> Self {
        Self {
            success: false,
            status_code,
            error: Some(error),
            attempts,
        }
    }
}

/// Statistics for webhook delivery.
#[derive(Debug, Clone, Default, Serialize)]
pub struct WebhookStats {
    /// Total events dispatched.
    pub total_dispatched: u64,

    /// Successful deliveries.
    pub successful: u64,

    /// Failed deliveries.
    pub failed: u64,

    /// Events filtered (not sent due to config).
    pub filtered: u64,

    /// Total retry attempts.
    pub retry_attempts: u64,
}

/// Webhook dispatcher for sending event notifications.
#[derive(Debug)]
pub struct WebhookDispatcher {
    settings: WebhookSettings,
    http_client: Client,
    stats: Arc<RwLock<WebhookStats>>,
}

impl WebhookDispatcher {
    /// Create a new webhook dispatcher.
    pub fn new(settings: WebhookSettings) -> Self {
        let timeout = Duration::from_secs(settings.timeout_seconds);

        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            settings,
            http_client,
            stats: Arc::new(RwLock::new(WebhookStats::default())),
        }
    }

    /// Check if webhooks are enabled.
    pub fn is_enabled(&self) -> bool {
        self.settings.enabled && !self.settings.url.is_empty()
    }

    /// Check if a specific event type is enabled.
    pub fn is_event_enabled(&self, event: &WebhookEvent) -> bool {
        if !self.is_enabled() {
            return false;
        }

        // If no events specified, all are enabled
        if self.settings.events.is_empty() {
            return true;
        }

        self.settings.events.contains(&event.category().to_string())
    }

    /// Dispatch a webhook event.
    ///
    /// Sends the event asynchronously with automatic retries.
    /// If the event type is not enabled, it will be silently ignored.
    pub async fn dispatch(&self, event: WebhookEvent) -> Option<DeliveryResult> {
        if !self.is_event_enabled(&event) {
            debug!(
                event_type = event.category(),
                "Webhook event filtered (not enabled)"
            );
            {
                let mut stats = self.stats.write().await;
                stats.filtered += 1;
            }
            return None;
        }

        info!(
            event_id = %event.id,
            event_type = event.category(),
            "Dispatching webhook event"
        );

        let result = self.deliver_with_retry(&event).await;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_dispatched += 1;
            if result.success {
                stats.successful += 1;
            } else {
                stats.failed += 1;
            }
            if result.attempts > 1 {
                stats.retry_attempts += (result.attempts - 1) as u64;
            }
        }

        Some(result)
    }

    /// Deliver event with retry logic.
    async fn deliver_with_retry(&self, event: &WebhookEvent) -> DeliveryResult {
        let payload = match serde_json::to_vec(event) {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "Failed to serialize webhook event");
                return DeliveryResult::failure(format!("Serialization error: {}", e), None, 0);
            }
        };

        let signature = sign_payload(&payload, &self.settings.secret);
        let signature_header = format_signature_header(&signature);

        let mut last_error = String::new();
        let mut last_status = None;

        for attempt in 1..=self.settings.retry_count {
            debug!(
                attempt = attempt,
                max_attempts = self.settings.retry_count,
                url = %self.settings.url,
                "Attempting webhook delivery"
            );

            match self
                .http_client
                .post(&self.settings.url)
                .header("Content-Type", "application/json")
                .header("X-Webhook-Signature", &signature_header)
                .header("X-Webhook-ID", event.id.to_string())
                .body(payload.clone())
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status().as_u16();

                    if response.status().is_success() {
                        info!(
                            event_id = %event.id,
                            status = status,
                            attempts = attempt,
                            "Webhook delivered successfully"
                        );
                        return DeliveryResult::success(status, attempt);
                    }

                    // Non-success status
                    last_status = Some(status);
                    last_error = format!("HTTP {}", status);

                    // Don't retry on client errors (4xx)
                    if response.status().is_client_error() {
                        warn!(
                            event_id = %event.id,
                            status = status,
                            "Webhook delivery failed with client error, not retrying"
                        );
                        return DeliveryResult::failure(last_error, last_status, attempt);
                    }

                    warn!(
                        event_id = %event.id,
                        status = status,
                        attempt = attempt,
                        "Webhook delivery failed, will retry"
                    );
                }
                Err(e) => {
                    last_error = e.to_string();
                    warn!(
                        event_id = %event.id,
                        error = %e,
                        attempt = attempt,
                        "Webhook request failed"
                    );
                }
            }

            // Exponential backoff before retry
            if attempt < self.settings.retry_count {
                let delay = Duration::from_millis(100 * 2u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }
        }

        error!(
            event_id = %event.id,
            error = %last_error,
            attempts = self.settings.retry_count,
            "Webhook delivery failed after all retries"
        );

        DeliveryResult::failure(last_error, last_status, self.settings.retry_count)
    }

    /// Get delivery statistics.
    pub async fn stats(&self) -> WebhookStats {
        self.stats.read().await.clone()
    }

    /// Reset delivery statistics.
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = WebhookStats::default();
    }

    /// Get the configured URL.
    pub fn url(&self) -> &str {
        &self.settings.url
    }

    /// Get enabled event categories.
    pub fn enabled_events(&self) -> &[String] {
        &self.settings.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_settings() -> WebhookSettings {
        WebhookSettings {
            enabled: true,
            url: "https://example.com/webhook".to_string(),
            secret: "test-secret".to_string(),
            events: vec!["oauth_refresh".to_string(), "circuit_breaker".to_string()],
            retry_count: 3,
            timeout_seconds: 5,
        }
    }

    #[test]
    fn test_webhook_settings_default() {
        let settings = WebhookSettings::default();

        assert!(!settings.enabled);
        assert!(settings.url.is_empty());
        assert!(settings.events.is_empty());
        assert_eq!(settings.retry_count, 3);
        assert_eq!(settings.timeout_seconds, 5);
    }

    #[test]
    fn test_webhook_dispatcher_is_enabled() {
        let mut settings = create_test_settings();
        let dispatcher = WebhookDispatcher::new(settings.clone());
        assert!(dispatcher.is_enabled());

        // Disable webhooks
        settings.enabled = false;
        let dispatcher = WebhookDispatcher::new(settings.clone());
        assert!(!dispatcher.is_enabled());

        // Empty URL
        settings.enabled = true;
        settings.url = String::new();
        let dispatcher = WebhookDispatcher::new(settings);
        assert!(!dispatcher.is_enabled());
    }

    #[test]
    fn test_webhook_event_filter() {
        let settings = create_test_settings();
        let dispatcher = WebhookDispatcher::new(settings);

        // OAuth event should be enabled
        let oauth_event = WebhookEvent::new(WebhookEventType::OAuthTokenRefreshed {
            provider: "claude".to_string(),
            account_id: "123".to_string(),
        });
        assert!(dispatcher.is_event_enabled(&oauth_event));

        // Circuit breaker event should be enabled
        let circuit_event = WebhookEvent::new(WebhookEventType::ProviderCircuitOpen {
            provider: "openrouter".to_string(),
            reason: "failures".to_string(),
            failure_count: 5,
        });
        assert!(dispatcher.is_event_enabled(&circuit_event));

        // Quota event should NOT be enabled
        let quota_event = WebhookEvent::new(WebhookEventType::QuotaExceeded {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            current_usage: 100.0,
            limit: 90.0,
        });
        assert!(!dispatcher.is_event_enabled(&quota_event));
    }

    #[test]
    fn test_webhook_all_events_enabled() {
        let mut settings = create_test_settings();
        settings.events = vec![]; // Empty means all enabled
        let dispatcher = WebhookDispatcher::new(settings);

        let event = WebhookEvent::new(WebhookEventType::QuotaExceeded {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            current_usage: 100.0,
            limit: 90.0,
        });
        assert!(dispatcher.is_event_enabled(&event));
    }

    #[test]
    fn test_webhook_serialization() {
        let event = WebhookEvent::new(WebhookEventType::ConfigUpdated {
            changed_sections: vec!["router".to_string(), "providers".to_string()],
            source: "api".to_string(),
        });

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"config_updated\""));
        assert!(json.contains("\"changed_sections\":[\"router\",\"providers\"]"));
    }

    #[test]
    fn test_delivery_result() {
        let success = DeliveryResult::success(200, 1);
        assert!(success.success);
        assert_eq!(success.status_code, Some(200));
        assert!(success.error.is_none());

        let failure = DeliveryResult::failure("timeout".to_string(), None, 3);
        assert!(!failure.success);
        assert!(failure.status_code.is_none());
        assert_eq!(failure.error, Some("timeout".to_string()));
    }

    #[tokio::test]
    async fn test_webhook_dispatch_disabled() {
        let mut settings = create_test_settings();
        settings.enabled = false;
        let dispatcher = WebhookDispatcher::new(settings);

        let event = WebhookEvent::new(WebhookEventType::OAuthTokenRefreshed {
            provider: "claude".to_string(),
            account_id: "123".to_string(),
        });

        let result = dispatcher.dispatch(event).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_webhook_stats() {
        let mut settings = create_test_settings();
        settings.enabled = false; // Disable to prevent actual HTTP calls
        let dispatcher = WebhookDispatcher::new(settings);

        let event = WebhookEvent::new(WebhookEventType::OAuthTokenRefreshed {
            provider: "claude".to_string(),
            account_id: "123".to_string(),
        });

        dispatcher.dispatch(event).await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.filtered, 1);
        assert_eq!(stats.total_dispatched, 0);
    }

    #[tokio::test]
    async fn test_webhook_reset_stats() {
        let settings = create_test_settings();
        let dispatcher = WebhookDispatcher::new(settings);

        // Get initial stats
        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_dispatched, 0);

        // Reset
        dispatcher.reset_stats().await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_dispatched, 0);
        assert_eq!(stats.filtered, 0);
    }

    #[test]
    fn test_webhook_settings_serialization() {
        let settings = create_test_settings();

        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"url\":\"https://example.com/webhook\""));

        let deserialized: WebhookSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, settings.url);
        assert_eq!(deserialized.enabled, settings.enabled);
    }

    #[test]
    fn test_dispatcher_getters() {
        let settings = create_test_settings();
        let dispatcher = WebhookDispatcher::new(settings.clone());

        assert_eq!(dispatcher.url(), "https://example.com/webhook");
        assert_eq!(dispatcher.enabled_events().len(), 2);
    }
}
