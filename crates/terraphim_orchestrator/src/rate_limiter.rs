//! Rate limiter with exponential backoff and circuit breaker integration.
//!
//! Provides per-provider rate limiting that respects 429 responses and
//! backs off exponentially to avoid burning API budget.
//!
//! # Feature flag
//!
//! Rate limit backoff is controlled by the `RATE_LIMIT_BACKOFF_ENABLED`
//! environment variable (default: `false` for backward compatibility).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::{debug, info};

/// Whether rate limit backoff is enabled via environment variable.
pub fn is_rate_limit_backoff_enabled() -> bool {
    std::env::var("RATE_LIMIT_BACKOFF_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

/// Rate limit state for a single provider.
#[derive(Debug, Clone)]
struct ProviderRateLimit {
    /// When the rate limit was first detected.
    #[allow(dead_code)]
    limited_at: Instant,
    /// Number of consecutive rate limit events.
    consecutive_count: u32,
    /// Current backoff duration.
    backoff_duration: Duration,
    /// When the backoff expires.
    backoff_until: Option<Instant>,
}

impl ProviderRateLimit {
    fn new() -> Self {
        Self {
            limited_at: Instant::now(),
            consecutive_count: 0,
            backoff_duration: Duration::from_secs(60),
            backoff_until: Some(Instant::now() + Duration::from_secs(60)),
        }
    }

    /// Record another rate limit event and increase backoff.
    fn record_rate_limit(&mut self) {
        self.consecutive_count += 1;
        // Exponential backoff: 60s, 120s, 240s, 480s, 600s (max 10 minutes)
        let multiplier = 2u64.pow(self.consecutive_count.min(5) - 1);
        self.backoff_duration = Duration::from_secs(60 * multiplier).min(Duration::from_secs(600));
        self.backoff_until = Some(Instant::now() + self.backoff_duration);
        info!(
            consecutive_count = self.consecutive_count,
            backoff_secs = self.backoff_duration.as_secs(),
            "rate limit backoff increased"
        );
    }

    /// Check if the provider is currently in backoff.
    fn is_in_backoff(&self) -> bool {
        self.backoff_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    /// Get the remaining backoff duration.
    fn remaining_backoff(&self) -> Option<Duration> {
        self.backoff_until.map(|until| {
            let now = Instant::now();
            if now < until {
                until.duration_since(now)
            } else {
                Duration::ZERO
            }
        })
    }

    /// Clear the rate limit state (call when provider recovers).
    fn clear(&mut self) {
        if self.consecutive_count > 0 {
            info!(
                consecutive_count = self.consecutive_count,
                "rate limit cleared for provider"
            );
        }
        self.consecutive_count = 0;
        self.backoff_duration = Duration::from_secs(60);
        self.backoff_until = None;
    }
}

/// Per-provider rate limit tracker with exponential backoff.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Per-provider rate limit state.
    providers: Arc<Mutex<HashMap<String, ProviderRateLimit>>>,
    /// Whether rate limit backoff is enabled.
    enabled: bool,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new() -> Self {
        Self {
            providers: Arc::new(Mutex::new(HashMap::new())),
            enabled: is_rate_limit_backoff_enabled(),
        }
    }

    /// Create a new rate limiter with explicit enable flag.
    pub fn with_enabled(enabled: bool) -> Self {
        Self {
            providers: Arc::new(Mutex::new(HashMap::new())),
            enabled,
        }
    }

    /// Check if rate limit backoff is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a rate limit event for a provider.
    pub fn record_rate_limit(&self, provider: &str) {
        if !self.enabled {
            return;
        }

        let mut providers = self.providers.lock().unwrap();
        let entry = providers
            .entry(provider.to_string())
            .or_insert_with(ProviderRateLimit::new);
        entry.record_rate_limit();
    }

    /// Check if a provider is currently rate-limited (in backoff period).
    pub fn is_rate_limited(&self, provider: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let providers = self.providers.lock().unwrap();
        match providers.get(provider) {
            Some(entry) => {
                let limited = entry.is_in_backoff();
                if limited {
                    if let Some(remaining) = entry.remaining_backoff() {
                        debug!(
                            provider = %provider,
                            remaining_secs = remaining.as_secs(),
                            "provider is rate-limited"
                        );
                    }
                }
                limited
            }
            None => false,
        }
    }

    /// Clear rate limit state for a provider (call when it recovers).
    pub fn clear_rate_limit(&self, provider: &str) {
        let mut providers = self.providers.lock().unwrap();
        if let Some(entry) = providers.get_mut(provider) {
            entry.clear();
        }
    }

    /// Get the current backoff duration for a provider.
    pub fn backoff_duration(&self, provider: &str) -> Option<Duration> {
        let providers = self.providers.lock().unwrap();
        providers.get(provider).and_then(|e| e.remaining_backoff())
    }

    /// Get all currently rate-limited providers.
    pub fn rate_limited_providers(&self) -> Vec<(String, Duration)> {
        if !self.enabled {
            return Vec::new();
        }

        let providers = self.providers.lock().unwrap();
        providers
            .iter()
            .filter_map(|(provider, entry)| {
                entry.remaining_backoff().map(|dur| (provider.clone(), dur))
            })
            .collect()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_disabled_by_default() {
        std::env::remove_var("RATE_LIMIT_BACKOFF_ENABLED");
        let limiter = RateLimiter::new();
        assert!(!limiter.is_enabled());
        assert!(!limiter.is_rate_limited("test-provider"));
    }

    #[test]
    fn test_rate_limiter_enabled() {
        let limiter = RateLimiter::with_enabled(true);
        assert!(limiter.is_enabled());
        assert!(!limiter.is_rate_limited("test-provider"));

        limiter.record_rate_limit("test-provider");
        assert!(limiter.is_rate_limited("test-provider"));

        // After clearing, should not be limited
        limiter.clear_rate_limit("test-provider");
        assert!(!limiter.is_rate_limited("test-provider"));
    }

    #[test]
    fn test_exponential_backoff() {
        let limiter = RateLimiter::with_enabled(true);
        let provider = "test-provider";

        // First rate limit: 60s backoff
        limiter.record_rate_limit(provider);
        let dur1 = limiter.backoff_duration(provider).unwrap();
        assert!(
            dur1.as_secs() >= 50 && dur1.as_secs() <= 65,
            "first backoff: {}s",
            dur1.as_secs()
        );

        // Second rate limit: 120s backoff
        limiter.record_rate_limit(provider);
        let dur2 = limiter.backoff_duration(provider).unwrap();
        assert!(
            dur2.as_secs() >= 110 && dur2.as_secs() <= 125,
            "second backoff: {}s",
            dur2.as_secs()
        );

        // Third rate limit: 240s backoff
        limiter.record_rate_limit(provider);
        let dur3 = limiter.backoff_duration(provider).unwrap();
        assert!(
            dur3.as_secs() >= 230 && dur3.as_secs() <= 245,
            "third backoff: {}s",
            dur3.as_secs()
        );
    }
}
