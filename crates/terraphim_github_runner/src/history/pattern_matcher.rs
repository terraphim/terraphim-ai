//! Pattern matching for successful execution patterns

use crate::{ExecutionRecord, RunnerResult};
use super::ExecutionHistory;
use ahash::AHashMap;

/// Matches execution patterns for optimization
pub struct PatternMatcher {
    /// Patterns indexed by action
    patterns: AHashMap<String, Vec<ExecutionPattern>>,
}

/// A pattern of successful execution
#[derive(Debug, Clone)]
pub struct ExecutionPattern {
    /// Action identifier
    pub action: String,
    /// Commands that were executed
    pub commands: Vec<String>,
    /// Typical duration
    pub typical_duration_ms: u64,
    /// Required prerequisites
    pub prerequisites: Vec<String>,
    /// Produced outputs
    pub produces: Vec<String>,
    /// Confidence (based on historical success)
    pub confidence: f64,
    /// Number of successful executions
    pub success_count: usize,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new() -> Self {
        Self {
            patterns: AHashMap::new(),
        }
    }

    /// Learn patterns from execution history
    pub fn learn_from_history(&mut self, history: &ExecutionHistory) {
        // Group records by action
        let mut by_action: AHashMap<String, Vec<&ExecutionRecord>> = AHashMap::new();

        // This is a simplified approach - in real impl would iterate properly
        // For now, we'll just initialize empty patterns

        // Register built-in patterns
        self.register_builtin_patterns();
    }

    /// Register built-in patterns for common actions
    fn register_builtin_patterns(&mut self) {
        // npm ci pattern
        self.patterns.insert(
            "npm ci".to_string(),
            vec![ExecutionPattern {
                action: "npm ci".to_string(),
                commands: vec!["npm ci".to_string()],
                typical_duration_ms: 30000,
                prerequisites: vec!["node".to_string(), "npm".to_string()],
                produces: vec!["node_modules".to_string()],
                confidence: 0.95,
                success_count: 1000,
            }],
        );

        // cargo build pattern
        self.patterns.insert(
            "cargo build".to_string(),
            vec![ExecutionPattern {
                action: "cargo build".to_string(),
                commands: vec!["cargo build --release".to_string()],
                typical_duration_ms: 120000,
                prerequisites: vec!["rustc".to_string(), "cargo".to_string()],
                produces: vec!["target/release".to_string()],
                confidence: 0.9,
                success_count: 500,
            }],
        );

        // checkout pattern
        self.patterns.insert(
            "actions/checkout".to_string(),
            vec![ExecutionPattern {
                action: "actions/checkout".to_string(),
                commands: vec!["git clone".to_string(), "git checkout".to_string()],
                typical_duration_ms: 5000,
                prerequisites: vec!["git".to_string()],
                produces: vec!["source code".to_string()],
                confidence: 0.99,
                success_count: 10000,
            }],
        );
    }

    /// Find matching pattern for an action
    pub fn find_pattern(&self, action: &str) -> Option<&ExecutionPattern> {
        // Exact match
        if let Some(patterns) = self.patterns.get(action) {
            return patterns.first();
        }

        // Partial match (e.g., "npm ci" matches "run:npm")
        for (key, patterns) in &self.patterns {
            if action.contains(key) || key.contains(action) {
                return patterns.first();
            }
        }

        None
    }

    /// Get estimated duration for an action
    pub fn estimate_duration(&self, action: &str) -> Option<u64> {
        self.find_pattern(action).map(|p| p.typical_duration_ms)
    }

    /// Get recommended prerequisites
    pub fn get_prerequisites(&self, action: &str) -> Vec<String> {
        self.find_pattern(action)
            .map(|p| p.prerequisites.clone())
            .unwrap_or_default()
    }

    /// Check if action is likely to succeed
    pub fn predict_success(&self, action: &str) -> f64 {
        self.find_pattern(action)
            .map(|p| p.confidence)
            .unwrap_or(0.5)
    }

    /// Register a new pattern from successful execution
    pub fn register_pattern(&mut self, record: &ExecutionRecord) {
        if record.exit_code != 0 {
            return;
        }

        let pattern = ExecutionPattern {
            action: record.action.clone(),
            commands: record.interpreted_commands.clone(),
            typical_duration_ms: record.duration_ms,
            prerequisites: Vec::new(), // Would be extracted from KG context
            produces: record.artifacts_produced.iter().map(|a| a.name.clone()).collect(),
            confidence: 0.8, // Initial confidence
            success_count: 1,
        };

        self.patterns
            .entry(record.action.clone())
            .or_default()
            .push(pattern);
    }

    /// Get all known patterns
    pub fn all_patterns(&self) -> impl Iterator<Item = &ExecutionPattern> {
        self.patterns.values().flatten()
    }

    /// Get pattern count
    pub fn pattern_count(&self) -> usize {
        self.patterns.values().map(|v| v.len()).sum()
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        let mut matcher = Self::new();
        matcher.register_builtin_patterns();
        matcher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_builtin_pattern() {
        let matcher = PatternMatcher::default();

        let pattern = matcher.find_pattern("npm ci");
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap().action, "npm ci");
    }

    #[test]
    fn test_estimate_duration() {
        let matcher = PatternMatcher::default();

        let duration = matcher.estimate_duration("cargo build");
        assert!(duration.is_some());
        assert!(duration.unwrap() > 0);
    }

    #[test]
    fn test_predict_success() {
        let matcher = PatternMatcher::default();

        let confidence = matcher.predict_success("actions/checkout");
        assert!(confidence > 0.9);
    }
}
