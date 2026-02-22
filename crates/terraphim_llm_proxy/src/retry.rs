//! Retry logic with exponential backoff
//!
//! This module provides robust retry mechanisms for failed provider requests,
//! implementing exponential backoff with jitter and integration with the circuit
//! breaker system for optimal resilience.

use crate::error::ProxyError;
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Jitter factor to add randomness (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Whether to retry on specific HTTP status codes
    pub retryable_status_codes: Vec<u16>,
    /// Whether to retry on network errors
    pub retry_on_network_errors: bool,
    /// Whether to retry on timeout errors
    pub retry_on_timeout_errors: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            retryable_status_codes: vec![429, 500, 502, 503, 504], // Rate limiting and server errors
            retry_on_network_errors: true,
            retry_on_timeout_errors: true,
        }
    }
}

impl RetryConfig {
    /// Create a retry config optimized for fast responses
    pub fn fast() -> Self {
        Self {
            max_attempts: 2,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_millis(500),
            backoff_multiplier: 1.5,
            jitter_factor: 0.05,
            ..Default::default()
        }
    }

    /// Create a retry config optimized for resilience
    pub fn resilient() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.5,
            jitter_factor: 0.2,
            ..Default::default()
        }
    }

    /// Create a retry config with no retries
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            ..Default::default()
        }
    }
}

/// Result of a retry attempt
#[derive(Debug)]
pub struct RetryResult<T> {
    /// Whether the operation eventually succeeded
    pub success: bool,
    /// The successful result value (present when success is true)
    pub value: Option<T>,
    /// Number of attempts made
    pub attempts: u32,
    /// Total time spent retrying
    pub total_duration: Duration,
    /// Final error if unsuccessful
    pub final_error: Option<ProxyError>,
}

/// Information about a retry attempt
#[derive(Debug, Clone)]
pub struct RetryAttempt {
    /// Attempt number (1-based)
    pub attempt: u32,
    /// Delay before this attempt
    pub delay: Duration,
    /// Error from this attempt (if failed)
    pub error: Option<ProxyError>,
}

/// Strategy for calculating retry delays
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Fixed delay between attempts
    Fixed(Duration),
    /// Linear increase: delay = initial_delay * attempt
    Linear { initial_delay: Duration },
    /// Exponential backoff: delay = initial_delay * multiplier^(attempt-1)
    Exponential {
        initial_delay: Duration,
        multiplier: f64,
        max_delay: Duration,
    },
}

impl BackoffStrategy {
    /// Calculate delay for a given attempt number
    pub fn calculate_delay(&self, attempt: u32, jitter_factor: f64) -> Duration {
        let base_delay = match self {
            BackoffStrategy::Fixed(delay) => *delay,
            BackoffStrategy::Linear { initial_delay } => *initial_delay * attempt,
            BackoffStrategy::Exponential {
                initial_delay,
                multiplier,
                max_delay,
            } => {
                let delay_ms =
                    initial_delay.as_millis() as f64 * multiplier.powi(attempt as i32 - 1);
                let delay = Duration::from_millis(delay_ms as u64);
                std::cmp::min(delay, *max_delay)
            }
        };

        // Add jitter to prevent thundering herd
        if jitter_factor > 0.0 {
            let jitter_ms = (base_delay.as_millis() as f64 * jitter_factor) as u64;
            let jitter = rand::thread_rng().gen_range(0..=jitter_ms);
            base_delay + Duration::from_millis(jitter)
        } else {
            base_delay
        }
    }
}

/// Retry executor with configurable backoff strategies
pub struct RetryExecutor {
    config: RetryConfig,
    backoff_strategy: BackoffStrategy,
}

impl RetryExecutor {
    /// Create a new retry executor with the given configuration
    pub fn new(config: RetryConfig) -> Self {
        let backoff_strategy = BackoffStrategy::Exponential {
            initial_delay: config.initial_delay,
            multiplier: config.backoff_multiplier,
            max_delay: config.max_delay,
        };

        Self {
            config,
            backoff_strategy,
        }
    }

    /// Create a retry executor with a custom backoff strategy
    pub fn with_strategy(config: RetryConfig, strategy: BackoffStrategy) -> Self {
        Self {
            config,
            backoff_strategy: strategy,
        }
    }

    /// Execute an operation with retry logic
    pub async fn execute<F, T, Fut>(&self, operation: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, ProxyError>>,
    {
        let start_time = std::time::Instant::now();
        let mut attempts = Vec::new();
        let mut last_error: Option<ProxyError> = None;

        for attempt in 1..=self.config.max_attempts {
            let delay_before = if attempt == 1 {
                Duration::ZERO
            } else {
                self.backoff_strategy
                    .calculate_delay(attempt, self.config.jitter_factor)
            };

            attempts.push(RetryAttempt {
                attempt,
                delay: delay_before,
                error: None,
            });

            // Wait before retry (except for first attempt)
            if attempt > 1 {
                debug!(
                    attempt = attempt,
                    delay_ms = delay_before.as_millis(),
                    "Retrying operation after delay"
                );
                sleep(delay_before).await;
            }

            // Execute the operation
            match operation().await {
                Ok(result) => {
                    let total_duration = start_time.elapsed();

                    if attempt > 1 {
                        debug!(
                            attempt = attempt,
                            total_duration_ms = total_duration.as_millis(),
                            "Operation succeeded after retry"
                        );
                    }

                    return RetryResult {
                        success: true,
                        value: Some(result),
                        attempts: attempt,
                        total_duration,
                        final_error: None,
                    };
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    attempts.last_mut().unwrap().error = Some(e.clone());

                    // Check if we should retry this error
                    if !self.should_retry(&e, attempt) {
                        warn!(
                            attempt = attempt,
                            error = %e,
                            "Error is not retryable, stopping retries"
                        );
                        break;
                    }

                    warn!(
                        attempt = attempt,
                        error = %e,
                        max_attempts = self.config.max_attempts,
                        "Operation failed, will retry"
                    );
                }
            }
        }

        let total_duration = start_time.elapsed();
        error!(
            attempts = attempts.len(),
            total_duration_ms = total_duration.as_millis(),
            final_error = ?last_error,
            "Operation failed after all retry attempts"
        );

        RetryResult {
            success: false,
            value: None,
            attempts: attempts.len() as u32,
            total_duration,
            final_error: last_error,
        }
    }

    /// Determine whether an error should be retried
    fn should_retry(&self, error: &ProxyError, attempt: u32) -> bool {
        // Don't retry if we've reached max attempts
        if attempt >= self.config.max_attempts {
            return false;
        }

        match error {
            ProxyError::NetworkError(_) => self.config.retry_on_network_errors,
            ProxyError::ProviderError {
                provider: _,
                message,
            } => {
                // Check if it's a rate limit error
                if message.to_lowercase().contains("rate limit")
                    || message.to_lowercase().contains("too many requests")
                {
                    return true;
                }
                false
            }
            ProxyError::ProviderTimeout {
                provider: _,
                elapsed: _,
            } => self.config.retry_on_timeout_errors,
            ProxyError::ProviderUnavailable { provider: _ } => true, // Retry if provider is unavailable
            ProxyError::RateLimitExceeded {
                limit_type: _,
                retry_after: _,
            } => true, // Always retry rate limits
            ProxyError::TooManyConcurrentRequests { max: _ } => true, // Retry on concurrent request limits
            ProxyError::InvalidProviderResponse(_) => true,           // Retry on invalid responses
            ProxyError::DnsResolutionFailed(_) => self.config.retry_on_network_errors,
            // Don't retry authentication errors
            ProxyError::MissingApiKey | ProxyError::InvalidApiKey | ProxyError::ApiKeyExpired => {
                false
            }
            ProxyError::InsufficientPermissions(_) => false,
            ProxyError::SessionHijackingAttempt => false,
            ProxyError::TooManyFailedAttempts { retry_after: _ } => false, // Don't retry if explicitly told to wait
            ProxyError::ResponseTooLarge { size: _, max: _ } => false, // Don't retry if response is too large
            // Don't retry validation errors
            ProxyError::InvalidRequest(_) => false,
            ProxyError::InvalidModel(_) => false,
            ProxyError::InvalidContent(_) => false,
            ProxyError::InvalidMaxTokens(_) => false,
            ProxyError::RequestTooLarge { size: _, max: _ } => false,
            ProxyError::TooManyMessages { count: _, max: _ } => false,
            // Don't retry token counting errors
            ProxyError::TokenCountOverflow => false,
            ProxyError::TokenCountTooLarge(_) => false,
            ProxyError::TokenCountingError(_) => false,
            // Don't retry routing errors
            ProxyError::NoProviderFound => false,
            ProxyError::RoutingPolicyViolation(_) => false,
            ProxyError::InvalidRoutingDecision(_) => false,
            // Don't retry security errors
            ProxyError::SsrfAttempt(_) => false,
            ProxyError::DnsRebindingAttack => false,
            ProxyError::InvalidProviderUrl(_) => false,
            // Don't retry config errors
            ProxyError::ConfigError(_) => false,
            ProxyError::InvalidConfig(_) => false,
            ProxyError::InsecureConfigPermissions => false,
            ProxyError::ConfigurationError(_) => false,
            ProxyError::TestError(_) => false,
            // Don't retry session errors
            ProxyError::InvalidSession => false,
            ProxyError::SessionExpired => false,
            // Don't retry transformer errors
            ProxyError::TransformerError {
                transformer: _,
                message: _,
            } => false,
            ProxyError::TransformerChainError(_) => false,
            // Don't retry I/O errors that are likely persistent
            ProxyError::Io(_) => false,
            // Don't retry serialization errors
            ProxyError::JsonSerialization(_) => false,
            ProxyError::TomlParsing(_) => false,
            // Don't retry internal errors
            ProxyError::Internal(_) => false,
            ProxyError::NotImplemented(_) => false,
            // Don't retry not found errors
            ProxyError::NotFound(_) => false,
        }
    }

    /// Get the current retry configuration
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }
}

/// Determine whether a final error is eligible for trying a fallback provider.
///
/// This is intentionally stricter than retry behavior for request-shape/auth errors,
/// and broader for upstream availability failures.
pub fn is_fallback_eligible(error: &ProxyError) -> bool {
    match error {
        ProxyError::NetworkError(_) => true,
        ProxyError::ProviderTimeout { .. } => true,
        ProxyError::ProviderUnavailable { .. } => true,
        ProxyError::RateLimitExceeded { .. } => true,
        ProxyError::TooManyConcurrentRequests { .. } => true,
        ProxyError::InvalidProviderResponse(_) => true,
        ProxyError::DnsResolutionFailed(_) => true,
        ProxyError::ProviderError { message, .. } => {
            let lower = message.to_lowercase();
            if lower.contains(" 400")
                || lower.contains(" 401")
                || lower.contains(" 403")
                || lower.contains("bad request")
                || lower.contains("invalid request")
                || lower.contains("not supported")
            {
                return false;
            }

            lower.contains("rate limit")
                || lower.contains("too many requests")
                || lower.contains(" status 5")
                || lower.contains("http 5") // catches "HTTP 500", "HTTP 502", etc.
                || lower.contains(" 500")
                || lower.contains(" 502")
                || lower.contains(" 503")
                || lower.contains(" 504")
                || lower.contains("temporarily unavailable")
                || lower.contains("upstream")
                || lower.contains("connection")
                || lower.contains("timeout")
                || lower.contains("http request failed")
                || lower.contains("error sending request")
                || lower.contains("provider '")
        }
        ProxyError::MissingApiKey
        | ProxyError::InvalidApiKey
        | ProxyError::ApiKeyExpired
        | ProxyError::InsufficientPermissions(_)
        | ProxyError::InvalidRequest(_)
        | ProxyError::InvalidModel(_)
        | ProxyError::InvalidContent(_)
        | ProxyError::InvalidMaxTokens(_)
        | ProxyError::RequestTooLarge { .. }
        | ProxyError::TooManyMessages { .. }
        | ProxyError::RoutingPolicyViolation(_)
        | ProxyError::InvalidRoutingDecision(_)
        | ProxyError::ConfigError(_)
        | ProxyError::InvalidConfig(_)
        | ProxyError::ConfigurationError(_)
        | ProxyError::TransformerError { .. }
        | ProxyError::TransformerChainError(_) => false,
        _ => false,
    }
}

/// Convenience function to execute an operation with default retry configuration
pub async fn retry_with_default<F, T, Fut>(operation: F) -> RetryResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ProxyError>>,
{
    let executor = RetryExecutor::new(RetryConfig::default());
    executor.execute(operation).await
}

/// Convenience function to execute an operation with fast retry configuration
pub async fn retry_with_fast<F, T, Fut>(operation: F) -> RetryResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ProxyError>>,
{
    let executor = RetryExecutor::new(RetryConfig::fast());
    executor.execute(operation).await
}

/// Convenience function to execute an operation with resilient retry configuration
pub async fn retry_with_resilient<F, T, Fut>(operation: F) -> RetryResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ProxyError>>,
{
    let executor = RetryExecutor::new(RetryConfig::resilient());
    executor.execute(operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(10));
        assert_eq!(config.backoff_multiplier, 2.0);
        assert_eq!(config.jitter_factor, 0.1);
    }

    #[test]
    fn test_retry_config_fast() {
        let config = RetryConfig::fast();
        assert_eq!(config.max_attempts, 2);
        assert_eq!(config.initial_delay, Duration::from_millis(50));
        assert_eq!(config.max_delay, Duration::from_millis(500));
    }

    #[test]
    fn test_retry_config_resilient() {
        let config = RetryConfig::resilient();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(200));
        assert_eq!(config.max_delay, Duration::from_secs(30));
    }

    #[test]
    fn test_backoff_strategy_fixed() {
        let strategy = BackoffStrategy::Fixed(Duration::from_millis(100));
        let delay = strategy.calculate_delay(1, 0.0);
        assert_eq!(delay, Duration::from_millis(100));

        let delay = strategy.calculate_delay(5, 0.0);
        assert_eq!(delay, Duration::from_millis(100));
    }

    #[test]
    fn test_backoff_strategy_linear() {
        let strategy = BackoffStrategy::Linear {
            initial_delay: Duration::from_millis(100),
        };

        let delay1 = strategy.calculate_delay(1, 0.0);
        assert_eq!(delay1, Duration::from_millis(100));

        let delay3 = strategy.calculate_delay(3, 0.0);
        assert_eq!(delay3, Duration::from_millis(300));
    }

    #[test]
    fn test_backoff_strategy_exponential() {
        let strategy = BackoffStrategy::Exponential {
            initial_delay: Duration::from_millis(100),
            multiplier: 2.0,
            max_delay: Duration::from_millis(1000),
        };

        let delay1 = strategy.calculate_delay(1, 0.0);
        assert_eq!(delay1, Duration::from_millis(100));

        let delay2 = strategy.calculate_delay(2, 0.0);
        assert_eq!(delay2, Duration::from_millis(200));

        let delay3 = strategy.calculate_delay(3, 0.0);
        assert_eq!(delay3, Duration::from_millis(400));

        // Test max delay cap
        let delay10 = strategy.calculate_delay(10, 0.0);
        assert_eq!(delay10, Duration::from_millis(1000));
    }

    #[test]
    fn test_jitter_calculation() {
        let strategy = BackoffStrategy::Fixed(Duration::from_millis(100));

        // With jitter, delay should be between 100ms and 110ms
        let delay = strategy.calculate_delay(1, 0.1);
        assert!(delay >= Duration::from_millis(100));
        assert!(delay <= Duration::from_millis(110));
    }

    #[tokio::test]
    async fn test_retry_executor_success_immediately() {
        let executor = RetryExecutor::new(RetryConfig::default());

        let result = executor
            .execute(|| async { Ok::<(), ProxyError>(()) })
            .await;

        assert!(result.success);
        assert!(result.value.is_some());
        assert_eq!(result.attempts, 1);
        assert!(result.final_error.is_none());
    }

    #[tokio::test]
    async fn test_retry_executor_success_after_retries() {
        let executor = RetryExecutor::new(RetryConfig::default());
        let attempt_count = Arc::new(AtomicU32::new(0));

        let attempt_count_clone = Arc::clone(&attempt_count);
        let result = executor
            .execute(move || {
                let count = Arc::clone(&attempt_count_clone);
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current < 2 {
                        Err(ProxyError::NetworkError("Temporary failure".to_string()))
                    } else {
                        Ok::<(), ProxyError>(())
                    }
                }
            })
            .await;

        assert!(result.success);
        assert!(result.value.is_some());
        assert_eq!(result.attempts, 3);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
        assert!(result.final_error.is_none());
    }

    #[tokio::test]
    async fn test_retry_executor_failure_all_attempts() {
        let executor = RetryExecutor::new(RetryConfig::default());

        let result = executor
            .execute(|| async {
                Err::<(), ProxyError>(ProxyError::NetworkError("Persistent failure".to_string()))
            })
            .await;

        assert!(!result.success);
        assert!(result.value.is_none());
        assert_eq!(result.attempts, 3);
        assert!(result.final_error.is_some());
    }

    #[tokio::test]
    async fn test_retry_executor_no_retry_auth_error() {
        let executor = RetryExecutor::new(RetryConfig::default());
        let attempt_count = Arc::new(AtomicU32::new(0));

        let attempt_count_clone = Arc::clone(&attempt_count);
        let result = executor
            .execute(move || {
                let count = Arc::clone(&attempt_count_clone);
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<(), ProxyError>(ProxyError::InvalidApiKey)
                }
            })
            .await;

        assert!(!result.success);
        assert!(result.value.is_none());
        assert_eq!(result.attempts, 1); // Should not retry auth errors
        assert!(result.final_error.is_some());
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let result = retry_with_default(|| async { Ok::<(), ProxyError>(()) }).await;
        assert!(result.success);

        let result = retry_with_fast(|| async { Ok::<(), ProxyError>(()) }).await;
        assert!(result.success);

        let result = retry_with_resilient(|| async { Ok::<(), ProxyError>(()) }).await;
        assert!(result.success);
    }

    #[test]
    fn test_fallback_eligible_network_errors() {
        assert!(is_fallback_eligible(&ProxyError::NetworkError(
            "connection reset".to_string()
        )));
        assert!(is_fallback_eligible(&ProxyError::ProviderTimeout {
            provider: "test".to_string(),
            elapsed: Duration::from_secs(30),
        }));
        assert!(is_fallback_eligible(&ProxyError::ProviderUnavailable {
            provider: "test".to_string(),
        }));
        assert!(is_fallback_eligible(&ProxyError::RateLimitExceeded {
            limit_type: "requests".to_string(),
            retry_after: Duration::from_secs(60),
        }));
        assert!(is_fallback_eligible(&ProxyError::InvalidProviderResponse(
            "bad json".to_string()
        )));
        assert!(is_fallback_eligible(&ProxyError::DnsResolutionFailed(
            "nxdomain".to_string()
        )));
    }

    #[test]
    fn test_fallback_eligible_provider_5xx_errors() {
        // HTTP 500 from ZAI format: "HTTP 500 - Internal service error"
        assert!(is_fallback_eligible(&ProxyError::ProviderError {
            provider: "zai".to_string(),
            message: "HTTP 500 - Internal service error".to_string(),
        }));

        // HTTP 502
        assert!(is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "HTTP 502 - Bad Gateway".to_string(),
        }));

        // HTTP 503
        assert!(is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "HTTP 503 - Service Unavailable".to_string(),
        }));

        // HTTP 504
        assert!(is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "HTTP 504 - Gateway Timeout".to_string(),
        }));

        // Status 5xx format
        assert!(is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "Request failed with status 500".to_string(),
        }));

        // Rate limit messages
        assert!(is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "Rate limit exceeded, try again later".to_string(),
        }));

        // Connection errors
        assert!(is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "HTTP request failed: connection refused".to_string(),
        }));
    }

    #[test]
    fn test_fallback_not_eligible_client_errors() {
        // 400 Bad Request - not eligible
        assert!(!is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "HTTP 400 - Bad request body".to_string(),
        }));

        // 401 Unauthorized - not eligible
        assert!(!is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "HTTP 401 - Unauthorized".to_string(),
        }));

        // 403 Forbidden - not eligible
        assert!(!is_fallback_eligible(&ProxyError::ProviderError {
            provider: "test".to_string(),
            message: "HTTP 403 - Forbidden".to_string(),
        }));

        // Auth errors - not eligible
        assert!(!is_fallback_eligible(&ProxyError::InvalidApiKey));
        assert!(!is_fallback_eligible(&ProxyError::MissingApiKey));
        assert!(!is_fallback_eligible(&ProxyError::InvalidRequest(
            "bad".to_string()
        )));
    }
}
