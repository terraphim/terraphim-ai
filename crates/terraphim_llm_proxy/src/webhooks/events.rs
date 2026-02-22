//! Webhook event types.
//!
//! Defines events that can trigger webhook notifications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Webhook event payload.
///
/// Contains all information about a webhook event including
/// a unique ID, timestamp, event type, and associated data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Unique identifier for this event.
    pub id: Uuid,

    /// Type of event that occurred.
    #[serde(flatten)]
    pub event_type: WebhookEventType,

    /// When the event occurred.
    pub timestamp: DateTime<Utc>,

    /// Additional event data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl WebhookEvent {
    /// Create a new webhook event.
    pub fn new(event_type: WebhookEventType) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            timestamp: Utc::now(),
            data: None,
        }
    }

    /// Create a new webhook event with additional data.
    pub fn with_data(event_type: WebhookEventType, data: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            timestamp: Utc::now(),
            data: Some(data),
        }
    }

    /// Get the event category for filtering.
    pub fn category(&self) -> &str {
        self.event_type.category()
    }
}

/// Types of webhook events.
///
/// Each variant represents a significant state change that
/// external systems may want to be notified about.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebhookEventType {
    /// OAuth token was refreshed for a provider.
    OAuthTokenRefreshed {
        /// Provider name (e.g., "claude", "gemini", "copilot")
        provider: String,
        /// Account or user identifier
        account_id: String,
    },

    /// OAuth token refresh failed.
    OAuthTokenRefreshFailed {
        /// Provider name
        provider: String,
        /// Account or user identifier
        account_id: String,
        /// Error message
        error: String,
    },

    /// Provider circuit breaker opened due to failures.
    ProviderCircuitOpen {
        /// Provider name
        provider: String,
        /// Reason for opening (e.g., "consecutive failures: 5")
        reason: String,
        /// Consecutive failure count
        failure_count: u32,
    },

    /// Provider circuit breaker closed (recovered).
    ProviderCircuitClosed {
        /// Provider name
        provider: String,
        /// How long the circuit was open
        duration_seconds: u64,
    },

    /// Provider circuit breaker entered half-open state.
    ProviderCircuitHalfOpen {
        /// Provider name
        provider: String,
    },

    /// Quota exceeded for a provider/model.
    QuotaExceeded {
        /// Provider name
        provider: String,
        /// Model name
        model: String,
        /// Current usage
        current_usage: f64,
        /// Quota limit
        limit: f64,
    },

    /// Configuration was updated.
    ConfigUpdated {
        /// Sections that changed
        changed_sections: Vec<String>,
        /// Source of update (e.g., "api", "file_reload")
        source: String,
    },

    /// API key was created.
    ApiKeyCreated {
        /// Key ID (not the actual key)
        key_id: String,
    },

    /// API key was revoked.
    ApiKeyRevoked {
        /// Key ID
        key_id: String,
        /// Reason for revocation
        reason: Option<String>,
    },

    /// Provider health check failed.
    HealthCheckFailed {
        /// Provider name
        provider: String,
        /// Error message
        error: String,
    },

    /// Request rate limit hit.
    RateLimitHit {
        /// Provider name
        provider: String,
        /// Client identifier (hashed)
        client_id_hash: String,
        /// Current request count
        request_count: u32,
        /// Time window in seconds
        window_seconds: u32,
    },
}

impl WebhookEventType {
    /// Get the category for this event type.
    ///
    /// Categories are used for event filtering in configuration.
    pub fn category(&self) -> &str {
        match self {
            WebhookEventType::OAuthTokenRefreshed { .. }
            | WebhookEventType::OAuthTokenRefreshFailed { .. } => "oauth_refresh",
            WebhookEventType::ProviderCircuitOpen { .. }
            | WebhookEventType::ProviderCircuitClosed { .. }
            | WebhookEventType::ProviderCircuitHalfOpen { .. } => "circuit_breaker",
            WebhookEventType::QuotaExceeded { .. } => "quota_exceeded",
            WebhookEventType::ConfigUpdated { .. } => "config_updated",
            WebhookEventType::ApiKeyCreated { .. } | WebhookEventType::ApiKeyRevoked { .. } => {
                "api_key"
            }
            WebhookEventType::HealthCheckFailed { .. } => "health_check",
            WebhookEventType::RateLimitHit { .. } => "rate_limit",
        }
    }

    /// Get a human-readable description of the event.
    pub fn description(&self) -> String {
        match self {
            WebhookEventType::OAuthTokenRefreshed { provider, .. } => {
                format!("OAuth token refreshed for {}", provider)
            }
            WebhookEventType::OAuthTokenRefreshFailed {
                provider, error, ..
            } => {
                format!("OAuth token refresh failed for {}: {}", provider, error)
            }
            WebhookEventType::ProviderCircuitOpen {
                provider, reason, ..
            } => format!("Circuit breaker opened for {}: {}", provider, reason),
            WebhookEventType::ProviderCircuitClosed { provider, .. } => {
                format!("Circuit breaker closed for {}", provider)
            }
            WebhookEventType::ProviderCircuitHalfOpen { provider } => {
                format!("Circuit breaker half-open for {}", provider)
            }
            WebhookEventType::QuotaExceeded {
                provider, model, ..
            } => format!("Quota exceeded for {}:{}", provider, model),
            WebhookEventType::ConfigUpdated {
                changed_sections, ..
            } => format!("Configuration updated: {:?}", changed_sections),
            WebhookEventType::ApiKeyCreated { key_id } => format!("API key created: {}", key_id),
            WebhookEventType::ApiKeyRevoked { key_id, .. } => {
                format!("API key revoked: {}", key_id)
            }
            WebhookEventType::HealthCheckFailed { provider, error } => {
                format!("Health check failed for {}: {}", provider, error)
            }
            WebhookEventType::RateLimitHit { provider, .. } => {
                format!("Rate limit hit for {}", provider)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_new() {
        let event = WebhookEvent::new(WebhookEventType::ProviderCircuitOpen {
            provider: "openrouter".to_string(),
            reason: "consecutive failures".to_string(),
            failure_count: 5,
        });

        assert!(!event.id.is_nil());
        assert!(event.data.is_none());
        assert_eq!(event.category(), "circuit_breaker");
    }

    #[test]
    fn test_webhook_event_with_data() {
        let data = serde_json::json!({
            "extra_field": "extra_value"
        });

        let event = WebhookEvent::with_data(
            WebhookEventType::ConfigUpdated {
                changed_sections: vec!["router".to_string()],
                source: "api".to_string(),
            },
            data.clone(),
        );

        assert!(event.data.is_some());
        assert_eq!(event.data.unwrap(), data);
    }

    #[test]
    fn test_event_type_categories() {
        assert_eq!(
            WebhookEventType::OAuthTokenRefreshed {
                provider: "claude".to_string(),
                account_id: "123".to_string()
            }
            .category(),
            "oauth_refresh"
        );

        assert_eq!(
            WebhookEventType::ProviderCircuitOpen {
                provider: "test".to_string(),
                reason: "test".to_string(),
                failure_count: 5
            }
            .category(),
            "circuit_breaker"
        );

        assert_eq!(
            WebhookEventType::QuotaExceeded {
                provider: "test".to_string(),
                model: "gpt-4".to_string(),
                current_usage: 100.0,
                limit: 90.0
            }
            .category(),
            "quota_exceeded"
        );

        assert_eq!(
            WebhookEventType::ApiKeyRevoked {
                key_id: "key-1".to_string(),
                reason: None
            }
            .category(),
            "api_key"
        );
    }

    #[test]
    fn test_webhook_event_serialization() {
        let event = WebhookEvent::new(WebhookEventType::OAuthTokenRefreshed {
            provider: "claude".to_string(),
            account_id: "user-123".to_string(),
        });

        let json = serde_json::to_string(&event).unwrap();
        // Event type is flattened, so it appears at the top level
        assert!(json.contains("\"provider\":\"claude\""));
        assert!(json.contains("\"account_id\":\"user-123\""));
        assert!(json.contains("\"id\":"));
        assert!(json.contains("\"timestamp\":"));

        // Deserialize and verify
        let deserialized: WebhookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, event.id);
    }

    #[test]
    fn test_event_type_description() {
        let event_type = WebhookEventType::ProviderCircuitOpen {
            provider: "openrouter".to_string(),
            reason: "consecutive failures".to_string(),
            failure_count: 5,
        };

        assert!(event_type.description().contains("openrouter"));
        assert!(event_type.description().contains("consecutive failures"));
    }

    #[test]
    fn test_circuit_breaker_events() {
        // Test all circuit breaker states
        let open = WebhookEventType::ProviderCircuitOpen {
            provider: "test".to_string(),
            reason: "failures".to_string(),
            failure_count: 5,
        };
        assert_eq!(open.category(), "circuit_breaker");

        let half_open = WebhookEventType::ProviderCircuitHalfOpen {
            provider: "test".to_string(),
        };
        assert_eq!(half_open.category(), "circuit_breaker");

        let closed = WebhookEventType::ProviderCircuitClosed {
            provider: "test".to_string(),
            duration_seconds: 30,
        };
        assert_eq!(closed.category(), "circuit_breaker");
    }

    #[test]
    fn test_rate_limit_event() {
        let event = WebhookEventType::RateLimitHit {
            provider: "openai".to_string(),
            client_id_hash: "abc123".to_string(),
            request_count: 100,
            window_seconds: 60,
        };

        assert_eq!(event.category(), "rate_limit");
        assert!(event.description().contains("openai"));
    }
}
