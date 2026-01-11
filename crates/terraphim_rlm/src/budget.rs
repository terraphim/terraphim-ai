//! Budget tracking for RLM execution.
//!
//! The BudgetTracker enforces:
//! - Token budget: Maximum LLM tokens consumed per session
//! - Time budget: Maximum wall-clock time per session
//! - Recursion depth: Maximum nested LLM calls

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::config::RlmConfig;
use crate::error::{RlmError, RlmResult};
use crate::types::BudgetStatus;

/// Tracks resource consumption for a session.
///
/// Thread-safe budget tracker using atomic operations.
pub struct BudgetTracker {
    /// Token budget limit.
    token_budget: u64,
    /// Tokens consumed.
    tokens_used: AtomicU64,

    /// Time budget in milliseconds.
    time_budget_ms: u64,
    /// Start time for time tracking.
    start_time: Instant,

    /// Maximum recursion depth.
    max_recursion_depth: u32,
    /// Current recursion depth (not atomic - managed by SessionManager).
    current_depth: std::sync::atomic::AtomicU32,
}

impl BudgetTracker {
    /// Create a new budget tracker with default limits.
    pub fn new(config: &RlmConfig) -> Self {
        Self {
            token_budget: config.token_budget,
            tokens_used: AtomicU64::new(0),
            time_budget_ms: config.time_budget_ms,
            start_time: Instant::now(),
            max_recursion_depth: config.max_recursion_depth,
            current_depth: std::sync::atomic::AtomicU32::new(0),
        }
    }

    /// Create a child budget tracker for recursive calls.
    ///
    /// The child inherits remaining budget from parent.
    pub fn child(&self, remaining_tokens: u64, remaining_time_ms: u64) -> Self {
        Self {
            token_budget: remaining_tokens,
            tokens_used: AtomicU64::new(0),
            time_budget_ms: remaining_time_ms,
            start_time: Instant::now(),
            max_recursion_depth: self.max_recursion_depth,
            current_depth: std::sync::atomic::AtomicU32::new(
                self.current_depth.load(Ordering::Relaxed) + 1,
            ),
        }
    }

    /// Add tokens to the consumption count.
    pub fn add_tokens(&self, tokens: u64) -> RlmResult<()> {
        let new_total = self.tokens_used.fetch_add(tokens, Ordering::Relaxed) + tokens;

        if new_total > self.token_budget {
            return Err(RlmError::TokenBudgetExceeded {
                used: new_total,
                budget: self.token_budget,
            });
        }

        Ok(())
    }

    /// Check if token budget is exhausted.
    pub fn check_token_budget(&self) -> RlmResult<()> {
        let used = self.tokens_used.load(Ordering::Relaxed);
        if used >= self.token_budget {
            return Err(RlmError::TokenBudgetExceeded {
                used,
                budget: self.token_budget,
            });
        }
        Ok(())
    }

    /// Check if time budget is exhausted.
    pub fn check_time_budget(&self) -> RlmResult<()> {
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        if elapsed_ms >= self.time_budget_ms {
            return Err(RlmError::TimeBudgetExceeded {
                used_ms: elapsed_ms,
                budget_ms: self.time_budget_ms,
            });
        }
        Ok(())
    }

    /// Check if recursion depth is exhausted.
    pub fn check_recursion_depth(&self) -> RlmResult<()> {
        let depth = self.current_depth.load(Ordering::Relaxed);
        if depth > self.max_recursion_depth {
            return Err(RlmError::RecursionDepthExceeded {
                depth,
                max_depth: self.max_recursion_depth,
            });
        }
        Ok(())
    }

    /// Check all budgets at once.
    pub fn check_all(&self) -> RlmResult<()> {
        self.check_token_budget()?;
        self.check_time_budget()?;
        self.check_recursion_depth()?;
        Ok(())
    }

    /// Increment recursion depth.
    pub fn push_recursion(&self) -> RlmResult<u32> {
        let depth = self.current_depth.fetch_add(1, Ordering::Relaxed) + 1;
        if depth > self.max_recursion_depth {
            // Rollback
            self.current_depth.fetch_sub(1, Ordering::Relaxed);
            return Err(RlmError::RecursionDepthExceeded {
                depth,
                max_depth: self.max_recursion_depth,
            });
        }
        Ok(depth)
    }

    /// Decrement recursion depth.
    pub fn pop_recursion(&self) -> u32 {
        let depth = self.current_depth.fetch_sub(1, Ordering::Relaxed);
        depth.saturating_sub(1)
    }

    /// Get tokens used.
    pub fn tokens_used(&self) -> u64 {
        self.tokens_used.load(Ordering::Relaxed)
    }

    /// Get remaining tokens.
    pub fn tokens_remaining(&self) -> u64 {
        self.token_budget
            .saturating_sub(self.tokens_used.load(Ordering::Relaxed))
    }

    /// Get elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Get remaining time in milliseconds.
    pub fn time_remaining_ms(&self) -> u64 {
        self.time_budget_ms.saturating_sub(self.elapsed_ms())
    }

    /// Get current recursion depth.
    pub fn current_depth(&self) -> u32 {
        self.current_depth.load(Ordering::Relaxed)
    }

    /// Get remaining recursion depth.
    pub fn depth_remaining(&self) -> u32 {
        self.max_recursion_depth
            .saturating_sub(self.current_depth.load(Ordering::Relaxed))
    }

    /// Get current budget status.
    pub fn status(&self) -> BudgetStatus {
        BudgetStatus {
            token_budget: self.token_budget,
            tokens_used: self.tokens_used.load(Ordering::Relaxed),
            time_budget_ms: self.time_budget_ms,
            time_used_ms: self.elapsed_ms(),
            max_recursion_depth: self.max_recursion_depth,
            current_recursion_depth: self.current_depth.load(Ordering::Relaxed),
        }
    }

    /// Check if any budget is close to exhaustion (>80% used).
    pub fn is_near_exhaustion(&self) -> bool {
        let token_ratio =
            self.tokens_used.load(Ordering::Relaxed) as f64 / self.token_budget as f64;
        let time_ratio = self.elapsed_ms() as f64 / self.time_budget_ms as f64;
        let depth_ratio =
            self.current_depth.load(Ordering::Relaxed) as f64 / self.max_recursion_depth as f64;

        token_ratio > 0.8 || time_ratio > 0.8 || depth_ratio > 0.8
    }

    /// Reset the tracker (for testing).
    #[cfg(test)]
    pub fn reset(&self) {
        self.tokens_used.store(0, Ordering::Relaxed);
        self.current_depth.store(0, Ordering::Relaxed);
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new(&RlmConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RlmConfig {
        RlmConfig {
            token_budget: 1000,
            time_budget_ms: 60_000, // 1 minute
            max_recursion_depth: 5,
            ..Default::default()
        }
    }

    #[test]
    fn test_token_tracking() {
        let tracker = BudgetTracker::new(&test_config());

        // Add tokens within budget
        assert!(tracker.add_tokens(500).is_ok());
        assert_eq!(tracker.tokens_used(), 500);
        assert_eq!(tracker.tokens_remaining(), 500);

        // Add more tokens within budget
        assert!(tracker.add_tokens(400).is_ok());
        assert_eq!(tracker.tokens_used(), 900);

        // Exceed budget
        let result = tracker.add_tokens(200);
        assert!(matches!(result, Err(RlmError::TokenBudgetExceeded { .. })));
    }

    #[test]
    fn test_recursion_tracking() {
        let tracker = BudgetTracker::new(&test_config());

        // Push within limits
        assert_eq!(tracker.push_recursion().unwrap(), 1);
        assert_eq!(tracker.push_recursion().unwrap(), 2);
        assert_eq!(tracker.current_depth(), 2);

        // Pop
        assert_eq!(tracker.pop_recursion(), 1);
        assert_eq!(tracker.current_depth(), 1);

        // Push to limit
        tracker.push_recursion().unwrap();
        tracker.push_recursion().unwrap();
        tracker.push_recursion().unwrap();
        tracker.push_recursion().unwrap();

        // Should fail at max depth
        let result = tracker.push_recursion();
        assert!(matches!(result, Err(RlmError::RecursionDepthExceeded { .. })));
    }

    #[test]
    fn test_budget_status() {
        let tracker = BudgetTracker::new(&test_config());

        tracker.add_tokens(250).unwrap();
        tracker.push_recursion().unwrap();

        let status = tracker.status();
        assert_eq!(status.tokens_used, 250);
        assert_eq!(status.token_budget, 1000);
        assert_eq!(status.current_recursion_depth, 1);
        assert_eq!(status.max_recursion_depth, 5);
    }

    #[test]
    fn test_child_budget() {
        let parent = BudgetTracker::new(&test_config());
        parent.add_tokens(400).unwrap();
        parent.push_recursion().unwrap();

        let child = parent.child(parent.tokens_remaining(), parent.time_remaining_ms());

        // Child starts with remaining budget
        assert_eq!(child.token_budget, 600);
        assert_eq!(child.current_depth(), 2);

        // Child can use its own budget
        assert!(child.add_tokens(300).is_ok());
        assert_eq!(child.tokens_remaining(), 300);
    }

    #[test]
    fn test_near_exhaustion() {
        let config = RlmConfig {
            token_budget: 100,
            time_budget_ms: 60_000,
            max_recursion_depth: 5,
            ..Default::default()
        };
        let tracker = BudgetTracker::new(&config);

        assert!(!tracker.is_near_exhaustion());

        tracker.add_tokens(85).unwrap();
        assert!(tracker.is_near_exhaustion());
    }

    #[test]
    fn test_check_all() {
        let tracker = BudgetTracker::new(&test_config());

        // All should pass initially
        assert!(tracker.check_all().is_ok());

        // Exhaust tokens
        tracker.add_tokens(1001).ok();
        assert!(tracker.check_all().is_err());
    }
}
