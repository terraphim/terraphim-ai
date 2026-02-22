//! Production Metrics and Structured Logging
//!
//! This module provides structured logging fields and metrics collection
//! for production monitoring and observability.

use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn, Span};
use uuid::Uuid;

/// Request context for structured logging and tracing
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub start_time: Instant,
    pub client_id: Option<String>,
    pub user_agent: Option<String>,
    pub content_length: Option<usize>,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
            client_id: None,
            user_agent: None,
            content_length: None,
        }
    }
}

impl RequestContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_client_id(mut self, client_id: String) -> Self {
        self.client_id = Some(client_id);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_content_length(mut self, content_length: usize) -> Self {
        self.content_length = Some(content_length);
        self
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Provider metrics for structured logging
#[derive(Debug, Clone)]
pub struct ProviderMetrics {
    pub provider: String,
    pub model: String,
    pub endpoint: String,
    pub request_id: String,
    pub token_count: Option<usize>,
    pub response_time_ms: Option<u64>,
    pub status: RequestStatus,
    pub error_type: Option<String>,
    pub retry_count: u32,
}

impl ProviderMetrics {
    pub fn new(provider: String, model: String, endpoint: String, request_id: String) -> Self {
        Self {
            provider,
            model,
            endpoint,
            request_id,
            token_count: None,
            response_time_ms: None,
            status: RequestStatus::Pending,
            error_type: None,
            retry_count: 0,
        }
    }

    pub fn success(mut self) -> Self {
        self.status = RequestStatus::Success;
        self
    }

    pub fn error(mut self, error_type: String) -> Self {
        self.status = RequestStatus::Error;
        self.error_type = Some(error_type);
        self
    }

    pub fn timeout(mut self) -> Self {
        self.status = RequestStatus::Timeout;
        self
    }

    pub fn with_tokens(mut self, token_count: usize) -> Self {
        self.token_count = Some(token_count);
        self
    }

    pub fn with_response_time(mut self, duration: Duration) -> Self {
        self.response_time_ms = Some(duration.as_millis() as u64);
        self
    }

    pub fn with_retry(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }
}

/// Request status for metrics
#[derive(Debug, Clone, PartialEq)]
pub enum RequestStatus {
    Pending,
    Success,
    Error,
    Timeout,
}

impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestStatus::Pending => write!(f, "pending"),
            RequestStatus::Success => write!(f, "success"),
            RequestStatus::Error => write!(f, "error"),
            RequestStatus::Timeout => write!(f, "timeout"),
        }
    }
}

/// Routing decision metrics
#[derive(Debug, Clone)]
pub struct RoutingMetrics {
    pub request_id: String,
    pub scenario: String,
    pub provider: String,
    pub model: String,
    pub hints_used: Vec<String>,
    pub decision_time_ms: u64,
    pub fallback_used: bool,
}

impl RoutingMetrics {
    pub fn new(request_id: String, scenario: String, provider: String, model: String) -> Self {
        Self {
            request_id,
            scenario,
            provider,
            model,
            hints_used: Vec::new(),
            decision_time_ms: 0,
            fallback_used: false,
        }
    }

    pub fn with_hints(mut self, hints: Vec<String>) -> Self {
        self.hints_used = hints;
        self
    }

    pub fn with_decision_time(mut self, duration: Duration) -> Self {
        self.decision_time_ms = duration.as_millis() as u64;
        self
    }

    pub fn with_fallback(mut self) -> Self {
        self.fallback_used = true;
        self
    }
}

/// Log structured request metrics
#[instrument(skip_all, fields(
    request_id = %context.request_id,
    client_id = ?context.client_id,
    user_agent = ?context.user_agent,
    content_length = ?context.content_length
))]
pub fn log_request_metrics(context: &RequestContext, endpoint: &str, method: &str) {
    info!(
        request_id = %context.request_id,
        client_id = ?context.client_id,
        user_agent = ?context.user_agent,
        content_length = ?context.content_length,
        endpoint = %endpoint,
        method = %method,
        "HTTP request received"
    );
}

/// Log structured response metrics
#[instrument(skip_all, fields(
    request_id = %context.request_id,
    elapsed_ms = context.elapsed().as_millis()
))]
pub fn log_response_metrics(
    context: &RequestContext,
    status_code: u16,
    content_length: Option<usize>,
) {
    info!(
        request_id = %context.request_id,
        elapsed_ms = context.elapsed().as_millis(),
        status_code = status_code,
        content_length = ?content_length,
        "HTTP response sent"
    );
}

/// Log structured provider request metrics
#[instrument(skip_all, fields(
    request_id = %metrics.request_id,
    provider = %metrics.provider,
    model = %metrics.model,
    endpoint = %metrics.endpoint,
    status = %metrics.status,
    retry_count = metrics.retry_count,
    response_time_ms = ?metrics.response_time_ms,
    token_count = ?metrics.token_count,
    error_type = ?metrics.error_type
))]
pub fn log_provider_metrics(metrics: &ProviderMetrics) {
    match metrics.status {
        RequestStatus::Success => {
            info!(
                request_id = %metrics.request_id,
                provider = %metrics.provider,
                model = %metrics.model,
                endpoint = %metrics.endpoint,
                status = %metrics.status,
                retry_count = metrics.retry_count,
                response_time_ms = ?metrics.response_time_ms,
                token_count = ?metrics.token_count,
                "Provider request completed successfully"
            );
        }
        RequestStatus::Error => {
            error!(
                request_id = %metrics.request_id,
                provider = %metrics.provider,
                model = %metrics.model,
                endpoint = %metrics.endpoint,
                status = %metrics.status,
                retry_count = metrics.retry_count,
                response_time_ms = ?metrics.response_time_ms,
                error_type = ?metrics.error_type,
                token_count = ?metrics.token_count,
                "Provider request failed"
            );
        }
        RequestStatus::Timeout => {
            warn!(
                request_id = %metrics.request_id,
                provider = %metrics.provider,
                model = %metrics.model,
                endpoint = %metrics.endpoint,
                status = %metrics.status,
                retry_count = metrics.retry_count,
                response_time_ms = ?metrics.response_time_ms,
                "Provider request timed out"
            );
        }
        RequestStatus::Pending => {
            debug!(
                request_id = %metrics.request_id,
                provider = %metrics.provider,
                model = %metrics.model,
                endpoint = %metrics.endpoint,
                status = %metrics.status,
                retry_count = metrics.retry_count,
                "Provider request in progress"
            );
        }
    }
}

/// Log structured routing metrics
#[instrument(skip_all, fields(
    request_id = %metrics.request_id,
    scenario = %metrics.scenario,
    provider = %metrics.provider,
    model = %metrics.model,
    hints_used = ?metrics.hints_used,
    decision_time_ms = metrics.decision_time_ms,
    fallback_used = metrics.fallback_used
))]
pub fn log_routing_metrics(metrics: &RoutingMetrics) {
    info!(
        request_id = %metrics.request_id,
        scenario = %metrics.scenario,
        provider = %metrics.provider,
        model = %metrics.model,
        hints_used = ?metrics.hints_used,
        decision_time_ms = metrics.decision_time_ms,
        fallback_used = metrics.fallback_used,
        "Routing decision made"
    );
}

/// Log streaming event metrics
#[instrument(skip_all, fields(
    request_id = %request_id,
    provider = %provider,
    event_type = %event_type,
    sequence_id = ?sequence_id
))]
pub fn log_streaming_event(
    request_id: &str,
    provider: &str,
    event_type: &str,
    sequence_id: Option<u32>,
    content_length: Option<usize>,
) {
    debug!(
        request_id = %request_id,
        provider = %provider,
        event_type = %event_type,
        sequence_id = ?sequence_id,
        content_length = ?content_length,
        "Streaming event"
    );
}

/// Log error with structured context
#[instrument(skip_all, fields(
    request_id = ?request_id,
    provider = ?provider,
    error_type = %error_type,
    context = ?context
))]
pub fn log_structured_error(
    error: &str,
    error_type: &str,
    request_id: Option<&str>,
    provider: Option<&str>,
    context: Option<&str>,
) {
    error!(
        error = %error,
        error_type = %error_type,
        request_id = ?request_id,
        provider = ?provider,
        context = ?context,
        "Structured error occurred"
    );
}

/// Create child span for provider operations
pub fn create_provider_span(
    provider: &str,
    model: &str,
    operation: &str,
    request_id: &str,
) -> Span {
    tracing::info_span!(
        "provider_operation",
        provider = %provider,
        model = %model,
        operation = %operation,
        request_id = %request_id
    )
}

/// Create child span for streaming operations
pub fn create_streaming_span(provider: &str, model: &str, request_id: &str) -> Span {
    tracing::info_span!(
        "streaming_operation",
        provider = %provider,
        model = %model,
        request_id = %request_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_request_context_creation() {
        let context = RequestContext::new();
        assert!(!context.request_id.is_empty());
        assert_eq!(context.client_id, None);
        assert_eq!(context.user_agent, None);
        assert_eq!(context.content_length, None);
    }

    #[test]
    fn test_request_context_with_fields() {
        let context = RequestContext::new()
            .with_client_id("test_client".to_string())
            .with_user_agent("test_agent".to_string())
            .with_content_length(100);

        assert_eq!(context.client_id, Some("test_client".to_string()));
        assert_eq!(context.user_agent, Some("test_agent".to_string()));
        assert_eq!(context.content_length, Some(100));
    }

    #[test]
    fn test_request_context_elapsed() {
        let context = RequestContext::new();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = context.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_provider_metrics_creation() {
        let metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.example.com".to_string(),
            "req_123".to_string(),
        );

        assert_eq!(metrics.provider, "test_provider");
        assert_eq!(metrics.model, "test_model");
        assert_eq!(metrics.endpoint, "https://api.example.com");
        assert_eq!(metrics.request_id, "req_123");
        assert_eq!(metrics.status, RequestStatus::Pending);
        assert_eq!(metrics.token_count, None);
        assert_eq!(metrics.response_time_ms, None);
        assert_eq!(metrics.retry_count, 0);
    }

    #[test]
    fn test_provider_metrics_success() {
        let metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.example.com".to_string(),
            "req_123".to_string(),
        )
        .with_tokens(100)
        .with_response_time(Duration::from_millis(500))
        .success();

        assert_eq!(metrics.status, RequestStatus::Success);
        assert_eq!(metrics.token_count, Some(100));
        assert_eq!(metrics.response_time_ms, Some(500));
    }

    #[test]
    fn test_provider_metrics_error() {
        let metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.example.com".to_string(),
            "req_123".to_string(),
        )
        .with_response_time(Duration::from_millis(200))
        .error("timeout".to_string());

        assert_eq!(metrics.status, RequestStatus::Error);
        assert_eq!(metrics.error_type, Some("timeout".to_string()));
        assert_eq!(metrics.response_time_ms, Some(200));
    }

    #[test]
    fn test_provider_metrics_timeout() {
        let metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.example.com".to_string(),
            "req_123".to_string(),
        )
        .with_response_time(Duration::from_millis(5000))
        .timeout();

        assert_eq!(metrics.status, RequestStatus::Timeout);
        assert_eq!(metrics.response_time_ms, Some(5000));
    }

    #[test]
    fn test_provider_metrics_retry() {
        let metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.example.com".to_string(),
            "req_123".to_string(),
        )
        .with_retry(3);

        assert_eq!(metrics.retry_count, 3);
    }

    #[test]
    fn test_request_status_display() {
        assert_eq!(RequestStatus::Pending.to_string(), "pending");
        assert_eq!(RequestStatus::Success.to_string(), "success");
        assert_eq!(RequestStatus::Error.to_string(), "error");
        assert_eq!(RequestStatus::Timeout.to_string(), "timeout");
    }

    #[test]
    fn test_routing_metrics_creation() {
        let metrics = RoutingMetrics::new(
            "req_456".to_string(),
            "Default".to_string(),
            "openrouter".to_string(),
            "anthropic/claude-3.5-sonnet".to_string(),
        );

        assert_eq!(metrics.request_id, "req_456");
        assert_eq!(metrics.scenario, "Default");
        assert_eq!(metrics.provider, "openrouter");
        assert_eq!(metrics.model, "anthropic/claude-3.5-sonnet");
        assert!(metrics.hints_used.is_empty());
        assert_eq!(metrics.decision_time_ms, 0);
        assert!(!metrics.fallback_used);
    }

    #[test]
    fn test_routing_metrics_with_hints() {
        let hints = vec![
            "is_background: false".to_string(),
            "has_thinking: true".to_string(),
            "token_count: 50".to_string(),
        ];

        let metrics = RoutingMetrics::new(
            "req_789".to_string(),
            "Thinking".to_string(),
            "deepseek".to_string(),
            "deepseek-chat".to_string(),
        )
        .with_hints(hints.clone())
        .with_decision_time(Duration::from_millis(100))
        .with_fallback();

        assert_eq!(metrics.hints_used, hints);
        assert_eq!(metrics.decision_time_ms, 100);
        assert!(metrics.fallback_used);
    }

    #[test]
    fn test_provider_metrics_clone() {
        let original = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.example.com".to_string(),
            "req_123".to_string(),
        )
        .with_tokens(50)
        .success();

        let cloned = original.clone();
        assert_eq!(cloned.provider, original.provider);
        assert_eq!(cloned.model, original.model);
        assert_eq!(cloned.status, original.status);
        assert_eq!(cloned.token_count, original.token_count);
    }

    #[test]
    fn test_routing_metrics_clone() {
        let original = RoutingMetrics::new(
            "req_456".to_string(),
            "Default".to_string(),
            "openrouter".to_string(),
            "anthropic/claude-3.5-sonnet".to_string(),
        )
        .with_decision_time(Duration::from_millis(75))
        .with_fallback();

        let cloned = original.clone();
        assert_eq!(cloned.request_id, original.request_id);
        assert_eq!(cloned.scenario, original.scenario);
        assert_eq!(cloned.decision_time_ms, original.decision_time_ms);
        assert_eq!(cloned.fallback_used, original.fallback_used);
    }

    #[test]
    fn test_provider_metrics_with_retry_and_success() {
        let metrics = ProviderMetrics::new(
            "groq".to_string(),
            "llama-3.1-8b-instant".to_string(),
            "https://api.groq.com".to_string(),
            "req_retry".to_string(),
        )
        .with_retry(2)
        .with_response_time(Duration::from_millis(1000))
        .with_tokens(75)
        .success();

        assert_eq!(metrics.retry_count, 2);
        assert_eq!(metrics.status, RequestStatus::Success);
        assert_eq!(metrics.response_time_ms, Some(1000));
        assert_eq!(metrics.token_count, Some(75));
    }

    #[test]
    fn test_request_context_uuid_generation() {
        let context1 = RequestContext::new();
        let context2 = RequestContext::new();

        // Should generate different UUIDs
        assert_ne!(context1.request_id, context2.request_id);

        // Should be valid UUID format (version 4)
        assert!(context1.request_id.len() == 36);
        assert!(context1.request_id.contains('-'));
    }

    #[test]
    fn test_provider_status_equality() {
        assert_eq!(RequestStatus::Success, RequestStatus::Success);
        assert_ne!(RequestStatus::Success, RequestStatus::Error);
        assert_ne!(RequestStatus::Error, RequestStatus::Timeout);
        assert_ne!(RequestStatus::Pending, RequestStatus::Success);
    }

    #[test]
    fn test_metrics_with_zero_values() {
        let provider_metrics = ProviderMetrics::new(
            "test".to_string(),
            "test_model".to_string(),
            "https://api.test.com".to_string(),
            "req_zero".to_string(),
        )
        .with_response_time(Duration::from_millis(0))
        .with_tokens(0);

        assert_eq!(provider_metrics.response_time_ms, Some(0));
        assert_eq!(provider_metrics.token_count, Some(0));

        let routing_metrics = RoutingMetrics::new(
            "req_zero_routing".to_string(),
            "Test".to_string(),
            "test_provider".to_string(),
            "test_model".to_string(),
        )
        .with_decision_time(Duration::from_millis(0));

        assert_eq!(routing_metrics.decision_time_ms, 0);
    }

    #[test]
    fn test_provider_metrics_error_with_type() {
        let error_types = vec![
            "network_timeout".to_string(),
            "rate_limit".to_string(),
            "invalid_request".to_string(),
            "provider_error".to_string(),
        ];

        for error_type in error_types {
            let metrics = ProviderMetrics::new(
                "test_provider".to_string(),
                "test_model".to_string(),
                "https://api.test.com".to_string(),
                "req_error".to_string(),
            )
            .error(error_type.clone());

            assert_eq!(metrics.status, RequestStatus::Error);
            assert_eq!(metrics.error_type, Some(error_type));
        }
    }
}
