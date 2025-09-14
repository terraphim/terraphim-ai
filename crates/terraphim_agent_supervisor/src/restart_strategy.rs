//! Restart strategies for supervision trees
//!
//! Implements Erlang/OTP-style restart strategies for handling agent failures.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Restart strategies for handling agent failures
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartStrategy {
    /// Restart only the failed agent
    OneForOne,
    /// Restart all agents if one fails
    OneForAll,
    /// Restart the failed agent and all agents started after it
    RestForOne,
}

impl Default for RestartStrategy {
    fn default() -> Self {
        RestartStrategy::OneForOne
    }
}

/// Restart intensity configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestartIntensity {
    /// Maximum number of restarts allowed
    pub max_restarts: u32,
    /// Time window for restart counting
    pub time_window: Duration,
}

impl Default for RestartIntensity {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            time_window: Duration::from_secs(60),
        }
    }
}

impl RestartIntensity {
    /// Create a new restart intensity configuration
    pub fn new(max_restarts: u32, time_window: Duration) -> Self {
        Self {
            max_restarts,
            time_window,
        }
    }

    /// Create a lenient restart policy (more restarts allowed)
    pub fn lenient() -> Self {
        Self {
            max_restarts: 10,
            time_window: Duration::from_secs(120),
        }
    }

    /// Create a strict restart policy (fewer restarts allowed)
    pub fn strict() -> Self {
        Self {
            max_restarts: 3,
            time_window: Duration::from_secs(30),
        }
    }

    /// Create a policy that never restarts
    pub fn never() -> Self {
        Self {
            max_restarts: 0,
            time_window: Duration::from_secs(1),
        }
    }

    /// Check if restart is allowed given the current restart history
    pub fn is_restart_allowed(
        &self,
        restart_count: u32,
        time_since_first_restart: Duration,
    ) -> bool {
        // If no restarts yet, allow the first one
        if restart_count == 0 {
            return true;
        }

        // If time window has passed since first restart, reset the counter
        if time_since_first_restart > self.time_window {
            return true;
        }

        // Check if we're within the restart limit
        restart_count < self.max_restarts
    }
}

/// Restart policy combining strategy and intensity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestartPolicy {
    pub strategy: RestartStrategy,
    pub intensity: RestartIntensity,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            strategy: RestartStrategy::OneForOne,
            intensity: RestartIntensity::default(),
        }
    }
}

impl RestartPolicy {
    /// Create a new restart policy
    pub fn new(strategy: RestartStrategy, intensity: RestartIntensity) -> Self {
        Self {
            strategy,
            intensity,
        }
    }

    /// Create a lenient one-for-one policy
    pub fn lenient_one_for_one() -> Self {
        Self {
            strategy: RestartStrategy::OneForOne,
            intensity: RestartIntensity::lenient(),
        }
    }

    /// Create a strict one-for-all policy
    pub fn strict_one_for_all() -> Self {
        Self {
            strategy: RestartStrategy::OneForAll,
            intensity: RestartIntensity::strict(),
        }
    }

    /// Create a policy that never restarts
    pub fn never_restart() -> Self {
        Self {
            strategy: RestartStrategy::OneForOne,
            intensity: RestartIntensity::never(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_restart_intensity_default() {
        let intensity = RestartIntensity::default();
        assert_eq!(intensity.max_restarts, 5);
        assert_eq!(intensity.time_window, Duration::from_secs(60));
    }

    #[test]
    fn test_restart_intensity_is_allowed() {
        let intensity = RestartIntensity::new(3, Duration::from_secs(60));

        // First restart should be allowed
        assert!(intensity.is_restart_allowed(0, Duration::from_secs(0)));

        // Within limits should be allowed
        assert!(intensity.is_restart_allowed(2, Duration::from_secs(30)));

        // At limit should not be allowed
        assert!(!intensity.is_restart_allowed(3, Duration::from_secs(30)));

        // After time window should be allowed again
        assert!(intensity.is_restart_allowed(3, Duration::from_secs(120)));
    }

    #[test]
    fn test_restart_policy_presets() {
        let lenient = RestartPolicy::lenient_one_for_one();
        assert_eq!(lenient.strategy, RestartStrategy::OneForOne);
        assert_eq!(lenient.intensity.max_restarts, 10);

        let strict = RestartPolicy::strict_one_for_all();
        assert_eq!(strict.strategy, RestartStrategy::OneForAll);
        assert_eq!(strict.intensity.max_restarts, 3);

        let never = RestartPolicy::never_restart();
        assert_eq!(never.intensity.max_restarts, 0);
    }

    #[test]
    fn test_restart_strategy_serialization() {
        let strategy = RestartStrategy::RestForOne;
        let serialized = serde_json::to_string(&strategy).unwrap();
        let deserialized: RestartStrategy = serde_json::from_str(&serialized).unwrap();
        assert_eq!(strategy, deserialized);
    }
}
