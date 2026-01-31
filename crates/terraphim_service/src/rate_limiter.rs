use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::summarization_queue::{RateLimitConfig, RateLimiterStatus};

/// Token bucket rate limiter implementation
#[derive(Debug)]
pub struct TokenBucketLimiter {
    /// Maximum number of tokens in bucket
    max_tokens: f64,
    /// Current number of tokens
    tokens: Arc<Mutex<f64>>,
    /// Rate at which tokens are refilled (tokens per second)
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: Arc<Mutex<Instant>>,
    /// Request count in current minute
    request_count: Arc<Mutex<u32>>,
    /// Window start time for request counting
    window_start: Arc<Mutex<Instant>>,
    /// Maximum requests per minute
    max_requests_per_minute: u32,
}

impl TokenBucketLimiter {
    /// Create a new token bucket limiter
    pub fn new(config: &RateLimitConfig) -> Self {
        let now = Instant::now();

        Self {
            max_tokens: config.burst_size as f64,
            tokens: Arc::new(Mutex::new(config.burst_size as f64)),
            refill_rate: config.max_tokens_per_minute as f64 / 60.0, // tokens per second
            last_refill: Arc::new(Mutex::new(now)),
            request_count: Arc::new(Mutex::new(0)),
            window_start: Arc::new(Mutex::new(now)),
            max_requests_per_minute: config.max_requests_per_minute,
        }
    }

    /// Try to acquire a token, returns true if successful
    pub async fn try_acquire(&self, tokens_needed: f64) -> bool {
        self.refill_tokens().await;
        self.reset_window_if_needed().await;

        let mut tokens = self.tokens.lock().await;
        let mut request_count = self.request_count.lock().await;

        // Check request rate limit
        if *request_count >= self.max_requests_per_minute {
            log::debug!(
                "Rate limit exceeded: {} requests in current minute",
                *request_count
            );
            return false;
        }

        // Check token availability
        if *tokens >= tokens_needed {
            *tokens -= tokens_needed;
            *request_count += 1;
            log::debug!(
                "Token acquired. Remaining tokens: {:.2}, requests in window: {}",
                *tokens,
                *request_count
            );
            true
        } else {
            log::debug!(
                "Insufficient tokens. Available: {:.2}, needed: {:.2}",
                *tokens,
                tokens_needed
            );
            false
        }
    }

    /// Wait until a token can be acquired
    pub async fn acquire(&self, tokens_needed: f64) -> Result<(), crate::ServiceError> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 60; // Maximum 1 minute wait

        while attempts < MAX_ATTEMPTS {
            if self.try_acquire(tokens_needed).await {
                return Ok(());
            }

            attempts += 1;
            sleep(Duration::from_secs(1)).await;
        }

        Err(crate::ServiceError::Config(
            "Rate limit timeout: unable to acquire token within time limit".to_string(),
        ))
    }

    /// Get current limiter status
    pub async fn get_status(&self) -> RateLimiterStatus {
        self.refill_tokens().await;
        self.reset_window_if_needed().await;

        let tokens = self.tokens.lock().await;
        let request_count = self.request_count.lock().await;
        let window_start = self.window_start.lock().await;

        let elapsed = window_start.elapsed();
        let reset_in_seconds = if elapsed >= Duration::from_secs(60) {
            0
        } else {
            60 - elapsed.as_secs() as u32
        };

        RateLimiterStatus {
            current_tokens: *tokens,
            max_tokens: self.max_tokens,
            requests_in_window: *request_count,
            reset_in_seconds,
        }
    }

    /// Refill tokens based on elapsed time
    async fn refill_tokens(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.lock().await;
        let mut tokens = self.tokens.lock().await;

        let elapsed = now.duration_since(*last_refill);
        let tokens_to_add = elapsed.as_secs_f64() * self.refill_rate;

        if tokens_to_add > 0.0 {
            *tokens = (*tokens + tokens_to_add).min(self.max_tokens);
            *last_refill = now;
        }
    }

    /// Reset the request counting window if needed
    async fn reset_window_if_needed(&self) {
        let now = Instant::now();
        let mut window_start = self.window_start.lock().await;
        let mut request_count = self.request_count.lock().await;

        if now.duration_since(*window_start) >= Duration::from_secs(60) {
            *window_start = now;
            *request_count = 0;
        }
    }
}

/// Multi-provider rate limiter manager
#[derive(Debug)]
pub struct RateLimiterManager {
    limiters: HashMap<String, TokenBucketLimiter>,
}

impl RateLimiterManager {
    /// Create a new rate limiter manager
    pub fn new(configs: HashMap<String, RateLimitConfig>) -> Self {
        let mut limiters = HashMap::new();

        for (provider, config) in configs {
            limiters.insert(provider, TokenBucketLimiter::new(&config));
        }

        Self { limiters }
    }

    /// Try to acquire a token for a specific provider
    pub async fn try_acquire(&self, provider: &str, tokens_needed: f64) -> bool {
        if let Some(limiter) = self.limiters.get(provider) {
            limiter.try_acquire(tokens_needed).await
        } else {
            log::warn!("No rate limiter configured for provider: {}", provider);
            true // Allow if no limiter configured
        }
    }

    /// Wait until a token can be acquired for a specific provider
    pub async fn acquire(
        &self,
        provider: &str,
        tokens_needed: f64,
    ) -> Result<(), crate::ServiceError> {
        if let Some(limiter) = self.limiters.get(provider) {
            limiter.acquire(tokens_needed).await
        } else {
            log::warn!("No rate limiter configured for provider: {}", provider);
            Ok(()) // Allow if no limiter configured
        }
    }

    /// Get status for all rate limiters
    pub async fn get_all_status(&self) -> HashMap<String, RateLimiterStatus> {
        let mut status_map = HashMap::new();

        for (provider, limiter) in &self.limiters {
            status_map.insert(provider.clone(), limiter.get_status().await);
        }

        status_map
    }

    /// Get status for a specific provider
    pub async fn get_status(&self, provider: &str) -> Option<RateLimiterStatus> {
        if let Some(limiter) = self.limiters.get(provider) {
            Some(limiter.get_status().await)
        } else {
            None
        }
    }

    /// Add or update a rate limiter for a provider
    pub fn add_provider(&mut self, provider: String, config: RateLimitConfig) {
        self.limiters
            .insert(provider, TokenBucketLimiter::new(&config));
    }

    /// Remove a rate limiter for a provider
    pub fn remove_provider(&mut self, provider: &str) {
        self.limiters.remove(provider);
    }
}

/// Utility function to estimate token usage for text
pub fn estimate_tokens(text: &str) -> f64 {
    // Rough estimation: ~4 characters per token for English text
    // Add some buffer for prompt overhead
    let base_tokens = text.len() as f64 / 4.0;
    let prompt_overhead = 100.0; // Estimated tokens for system prompt
    (base_tokens + prompt_overhead).ceil()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    fn create_test_config() -> RateLimitConfig {
        RateLimitConfig {
            max_requests_per_minute: 10,
            max_tokens_per_minute: 1000,
            burst_size: 5,
        }
    }

    #[tokio::test]
    async fn test_token_bucket_basic() {
        let config = create_test_config();
        let limiter = TokenBucketLimiter::new(&config);

        // Should be able to acquire tokens up to burst size
        assert!(limiter.try_acquire(1.0).await);
        assert!(limiter.try_acquire(2.0).await);
        assert!(limiter.try_acquire(2.0).await);

        // Should fail when burst is exhausted
        assert!(!limiter.try_acquire(1.0).await);
    }

    #[tokio::test]
    async fn test_token_refill() {
        let mut config = create_test_config();
        config.max_tokens_per_minute = 60; // 1 token per second
        let limiter = TokenBucketLimiter::new(&config);

        // Exhaust the bucket
        assert!(limiter.try_acquire(5.0).await);
        assert!(!limiter.try_acquire(1.0).await);

        // Wait for refill
        sleep(Duration::from_millis(2100)).await; // Wait 2.1 seconds

        // Should have ~2 tokens refilled
        assert!(limiter.try_acquire(1.0).await);
        assert!(limiter.try_acquire(1.0).await);
        assert!(!limiter.try_acquire(1.0).await); // Should fail
    }

    #[tokio::test]
    async fn test_request_rate_limit() {
        let mut config = create_test_config();
        config.max_requests_per_minute = 2;
        config.burst_size = 10; // Plenty of tokens
        let limiter = TokenBucketLimiter::new(&config);

        // Should be able to make 2 requests
        assert!(limiter.try_acquire(1.0).await);
        assert!(limiter.try_acquire(1.0).await);

        // Third request should fail due to rate limit
        assert!(!limiter.try_acquire(1.0).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_manager() {
        let mut configs = HashMap::new();
        configs.insert("provider1".to_string(), create_test_config());

        let manager = RateLimiterManager::new(configs);

        // Should work for configured provider
        assert!(manager.try_acquire("provider1", 1.0).await);

        // Should allow for unconfigured provider
        assert!(manager.try_acquire("provider2", 1.0).await);
    }

    #[tokio::test]
    async fn test_status_reporting() {
        let config = create_test_config();
        let limiter = TokenBucketLimiter::new(&config);

        let status = limiter.get_status().await;
        assert_eq!(status.max_tokens, config.burst_size as f64);
        assert!(status.current_tokens <= config.burst_size as f64);
        assert_eq!(status.requests_in_window, 0);
    }

    #[test]
    fn test_token_estimation() {
        let text = "This is a test text with approximately twenty words to test token estimation";
        let tokens = estimate_tokens(text);

        // Should estimate around 120 tokens (80 chars / 4 + 100 overhead)
        assert!(tokens > 100.0);
        assert!(tokens < 200.0);
    }

    #[tokio::test]
    async fn test_acquire_blocking() {
        let mut config = create_test_config();
        config.max_tokens_per_minute = 60; // 1 token per second
        let limiter = TokenBucketLimiter::new(&config);

        // Exhaust the bucket
        assert!(limiter.try_acquire(5.0).await);

        // This should block and then succeed
        let start = Instant::now();
        limiter.acquire(1.0).await.unwrap();
        let elapsed = start.elapsed();

        // Should have waited at least 1 second
        assert!(elapsed >= Duration::from_secs(1));
    }
}
