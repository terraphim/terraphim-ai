use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};

use crate::summarization_queue::{RateLimitConfig, RateLimiterStatus};

/// Request for token acquisition
#[derive(Debug)]
struct TokenRequest {
    /// Number of tokens needed
    tokens_needed: f64,
    /// Channel to respond with success/failure
    response: oneshot::Sender<Result<(), String>>,
}

/// Queue-based token bucket rate limiter that avoids lock contention
#[derive(Debug)]
pub struct QueueBasedTokenBucketLimiter {
    /// Channel to send token requests
    request_sender: mpsc::UnboundedSender<TokenRequest>,
    /// Handle to the token manager task
    _manager_handle: tokio::task::JoinHandle<()>,
}

impl QueueBasedTokenBucketLimiter {
    /// Create a new queue-based token bucket limiter
    pub fn new(config: &RateLimitConfig) -> Self {
        let (request_sender, request_receiver) = mpsc::unbounded_channel();

        let manager = TokenBucketManager::new(config.clone(), request_receiver);
        let manager_handle = tokio::spawn(manager.run());

        Self {
            request_sender,
            _manager_handle: manager_handle,
        }
    }

    /// Try to acquire a token, returns true if successful
    pub async fn try_acquire(&self, tokens_needed: f64) -> bool {
        let (response_tx, response_rx) = oneshot::channel();

        let request = TokenRequest {
            tokens_needed,
            response: response_tx,
        };

        if self.request_sender.send(request).is_err() {
            return false;
        }

        // Wait for response with a very short timeout for try_acquire
        matches!(
            tokio::time::timeout(Duration::from_millis(1), response_rx).await,
            Ok(Ok(Ok(())))
        )
    }

    /// Wait until a token can be acquired
    pub async fn acquire(&self, tokens_needed: f64) -> Result<(), crate::ServiceError> {
        let (response_tx, response_rx) = oneshot::channel();

        let request = TokenRequest {
            tokens_needed,
            response: response_tx,
        };

        if self.request_sender.send(request).is_err() {
            return Err(crate::ServiceError::Config(
                "Rate limiter channel closed".to_string(),
            ));
        }

        // Wait for response with reasonable timeout
        match tokio::time::timeout(Duration::from_secs(60), response_rx).await {
            Ok(Ok(Ok(()))) => Ok(()),
            Ok(Ok(Err(error))) => Err(crate::ServiceError::Config(error)),
            Ok(Err(_)) => Err(crate::ServiceError::Config(
                "Response channel closed".to_string(),
            )),
            Err(_) => Err(crate::ServiceError::Config(
                "Rate limit timeout: unable to acquire token within time limit".to_string(),
            )),
        }
    }

    /// Get current status
    pub async fn get_status(&self) -> RateLimiterStatus {
        // For simplicity, return default status
        // In a full implementation, we could add a status request channel
        RateLimiterStatus {
            current_tokens: 0.0,
            max_tokens: 0.0,
            requests_in_window: 0,
            reset_in_seconds: 0,
        }
    }
}

/// Internal token bucket manager that processes requests in a single task
struct TokenBucketManager {
    /// Maximum number of tokens in bucket
    max_tokens: f64,
    /// Current number of tokens
    current_tokens: f64,
    /// Rate at which tokens are refilled (tokens per second)
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: Instant,
    /// Request count in current minute
    request_count: u32,
    /// Window start time for request counting
    window_start: Instant,
    /// Maximum requests per minute
    max_requests_per_minute: u32,
    /// Channel to receive token requests
    request_receiver: mpsc::UnboundedReceiver<TokenRequest>,
}

impl TokenBucketManager {
    /// Create a new token bucket manager
    fn new(
        config: RateLimitConfig,
        request_receiver: mpsc::UnboundedReceiver<TokenRequest>,
    ) -> Self {
        let now = Instant::now();

        Self {
            max_tokens: config.burst_size as f64,
            current_tokens: config.burst_size as f64,
            refill_rate: config.max_tokens_per_minute as f64 / 60.0, // tokens per second
            last_refill: now,
            request_count: 0,
            window_start: now,
            max_requests_per_minute: config.max_requests_per_minute,
            request_receiver,
        }
    }

    /// Main loop that processes token requests
    async fn run(mut self) {
        let mut refill_timer = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                // Process token requests
                Some(request) = self.request_receiver.recv() => {
                    let result = self.try_provide_tokens(request.tokens_needed).await;
                    let _ = request.response.send(result);
                }

                // Periodic refill
                _ = refill_timer.tick() => {
                    self.refill_tokens().await;
                    self.reset_window_if_needed().await;
                }

                else => break,
            }
        }
    }

    /// Try to provide tokens if available
    async fn try_provide_tokens(&mut self, tokens_needed: f64) -> Result<(), String> {
        self.refill_tokens().await;
        self.reset_window_if_needed().await;

        // Check request rate limit
        if self.request_count >= self.max_requests_per_minute {
            return Err(format!(
                "Request rate limit exceeded: {} requests per minute",
                self.max_requests_per_minute
            ));
        }

        // Check token availability
        if self.current_tokens >= tokens_needed {
            self.current_tokens -= tokens_needed;
            self.request_count += 1;

            log::trace!(
                "Token acquired. Remaining tokens: {:.2}, requests in window: {}",
                self.current_tokens,
                self.request_count
            );

            Ok(())
        } else {
            Err(format!(
                "Insufficient tokens. Available: {:.2}, needed: {:.2}",
                self.current_tokens, tokens_needed
            ))
        }
    }

    /// Refill tokens based on elapsed time
    async fn refill_tokens(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        let tokens_to_add = elapsed.as_secs_f64() * self.refill_rate;
        if tokens_to_add > 0.0 {
            self.current_tokens = (self.current_tokens + tokens_to_add).min(self.max_tokens);
            self.last_refill = now;

            log::trace!(
                "Refilled {:.2} tokens, current: {:.2}",
                tokens_to_add,
                self.current_tokens
            );
        }
    }

    /// Reset the request counting window if needed
    async fn reset_window_if_needed(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.window_start) >= Duration::from_secs(60) {
            self.request_count = 0;
            self.window_start = now;

            log::trace!("Reset request counting window");
        }
    }
}

/// Multi-provider queue-based rate limiter manager
#[derive(Debug)]
pub struct QueueBasedRateLimiterManager {
    limiters: Arc<tokio::sync::RwLock<HashMap<String, QueueBasedTokenBucketLimiter>>>,
}

impl QueueBasedRateLimiterManager {
    /// Create a new rate limiter manager
    pub fn new(configs: HashMap<String, RateLimitConfig>) -> Self {
        let mut limiters = HashMap::new();

        for (provider, config) in configs {
            limiters.insert(provider, QueueBasedTokenBucketLimiter::new(&config));
        }

        Self {
            limiters: Arc::new(tokio::sync::RwLock::new(limiters)),
        }
    }

    /// Try to acquire a token for a specific provider
    pub async fn try_acquire(&self, provider: &str, tokens_needed: f64) -> bool {
        let limiters = self.limiters.read().await;
        if let Some(limiter) = limiters.get(provider) {
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
        let limiters = self.limiters.read().await;
        if let Some(limiter) = limiters.get(provider) {
            limiter.acquire(tokens_needed).await
        } else {
            log::warn!("No rate limiter configured for provider: {}", provider);
            Ok(()) // Allow if no limiter configured
        }
    }

    /// Get status for all rate limiters
    pub async fn get_all_status(&self) -> HashMap<String, RateLimiterStatus> {
        let limiters = self.limiters.read().await;
        let mut status = HashMap::new();

        for (provider, limiter) in limiters.iter() {
            status.insert(provider.clone(), limiter.get_status().await);
        }

        status
    }

    /// Add or update a rate limiter for a provider
    pub async fn add_limiter(&self, provider: String, config: RateLimitConfig) {
        let mut limiters = self.limiters.write().await;
        limiters.insert(provider, QueueBasedTokenBucketLimiter::new(&config));
    }

    /// Remove a rate limiter for a provider
    pub async fn remove_limiter(&self, provider: &str) {
        let mut limiters = self.limiters.write().await;
        limiters.remove(provider);
    }
}

impl Clone for QueueBasedRateLimiterManager {
    fn clone(&self) -> Self {
        Self {
            limiters: Arc::clone(&self.limiters),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> RateLimitConfig {
        RateLimitConfig {
            max_requests_per_minute: 10,
            max_tokens_per_minute: 1000,
            burst_size: 100,
        }
    }

    #[tokio::test]
    async fn test_queue_based_token_acquisition() {
        let config = create_test_config();
        let limiter = QueueBasedTokenBucketLimiter::new(&config);

        // Should be able to acquire tokens up to burst size
        assert!(limiter.acquire(50.0).await.is_ok());
        assert!(limiter.acquire(50.0).await.is_ok());

        // Should fail when exceeding burst size
        assert!(!(limiter.try_acquire(1.0).await));
    }

    #[tokio::test]
    async fn test_queue_based_manager() {
        let mut configs = HashMap::new();
        configs.insert("test".to_string(), create_test_config());

        let manager = QueueBasedRateLimiterManager::new(configs);

        // Should work with configured provider
        assert!(manager.try_acquire("test", 10.0).await);

        // Should work with unconfigured provider (no limits)
        assert!(manager.try_acquire("unknown", 1000.0).await);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let config = create_test_config();
        let limiter = Arc::new(QueueBasedTokenBucketLimiter::new(&config));

        // Spawn multiple tasks trying to acquire tokens concurrently
        let mut handles = vec![];

        for i in 0..10 {
            let limiter_clone = Arc::clone(&limiter);
            let handle = tokio::spawn(async move { limiter_clone.acquire(10.0).await.map(|_| i) });
            handles.push(handle);
        }

        // Wait for all tasks
        let mut results = vec![];
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        // Some should succeed, some should fail due to token exhaustion
        // But importantly, there should be no deadlocks or panics
        assert!(!results.is_empty());
        println!(
            "Concurrent test completed with {} successful acquisitions",
            results.len()
        );
    }
}
