use std::collections::HashMap;

use tracing::{info, warn};

/// A signal indicating that an agent's outputs have converged.
#[derive(Debug, Clone)]
pub struct ConvergenceSignal {
    pub agent: String,
    pub similarity: f64,
    pub consecutive_count: u32,
}

/// Tracks output history and detects convergence for agents.
pub struct ConvergenceDetector {
    /// Similarity threshold (0.0 - 1.0) above which outputs are considered converged.
    pub convergence_threshold: f64,
    /// Number of consecutive similar outputs required to trigger convergence.
    pub consecutive_threshold: u32,
    /// History of recent outputs per agent.
    output_history: HashMap<String, Vec<String>>,
    /// Consecutive similar output count per agent.
    consecutive_counts: HashMap<String, u32>,
    /// Whether convergence has been signaled per agent.
    convergence_signaled: HashMap<String, bool>,
}

impl ConvergenceDetector {
    /// Create a new convergence detector.
    pub fn new(convergence_threshold: f64, consecutive_threshold: u32) -> Self {
        info!(
            threshold = convergence_threshold,
            consecutive = consecutive_threshold,
            "convergence detector initialized"
        );

        Self {
            convergence_threshold,
            consecutive_threshold,
            output_history: HashMap::new(),
            consecutive_counts: HashMap::new(),
            convergence_signaled: HashMap::new(),
        }
    }

    /// Record an agent output and check for convergence.
    /// Returns Some(ConvergenceSignal) if convergence is detected.
    pub fn record_output(&mut self, agent_name: &str, output: String) -> Option<ConvergenceSignal> {
        // Get previous output for comparison
        let similarity = if let Some(history) = self.output_history.get(agent_name) {
            if let Some(last_output) = history.last() {
                self.calculate_similarity(last_output, &output)
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Store the output
        self.output_history
            .entry(agent_name.to_string())
            .or_default()
            .push(output);

        // Keep only last 5 outputs per agent
        if let Some(history) = self.output_history.get_mut(agent_name) {
            if history.len() > 5 {
                history.remove(0);
            }
        }

        // Check if outputs are similar enough
        if similarity >= self.convergence_threshold {
            // Increment consecutive count
            let count = self
                .consecutive_counts
                .entry(agent_name.to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);

            info!(
                agent = %agent_name,
                similarity = %similarity,
                consecutive = *count,
                "similar output detected"
            );

            // Check if we've reached the threshold
            if *count >= self.consecutive_threshold {
                // Check if we haven't already signaled convergence
                let already_signaled = self
                    .convergence_signaled
                    .get(agent_name)
                    .copied()
                    .unwrap_or(false);

                if !already_signaled {
                    warn!(
                        agent = %agent_name,
                        similarity = %similarity,
                        consecutive = *count,
                        "CONVERGENCE DETECTED"
                    );

                    self.convergence_signaled
                        .insert(agent_name.to_string(), true);

                    return Some(ConvergenceSignal {
                        agent: agent_name.to_string(),
                        similarity,
                        consecutive_count: *count,
                    });
                }
            }
        } else {
            // Reset consecutive count and convergence signal on divergence
            if self.consecutive_counts.remove(agent_name).is_some() {
                info!(
                    agent = %agent_name,
                    similarity = %similarity,
                    "outputs diverged, resetting convergence counter"
                );
            }
            self.convergence_signaled.remove(agent_name);
        }

        None
    }

    /// Calculate similarity between two strings using a simple approach.
    /// Returns a value between 0.0 (completely different) and 1.0 (identical).
    fn calculate_similarity(&self, a: &str, b: &str) -> f64 {
        // Simple character-based similarity
        // For production, consider using more sophisticated algorithms like:
        // - Levenshtein distance
        // - Cosine similarity on word vectors
        // - Jaccard similarity on token sets

        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        // If strings are identical, return 1.0
        if a_lower == b_lower {
            return 1.0;
        }

        // Split into words and calculate overlap
        let a_words: std::collections::HashSet<&str> = a_lower.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b_lower.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        // Calculate Jaccard similarity: |A ∩ B| / |A ∪ B|
        let intersection: std::collections::HashSet<&str> =
            a_words.intersection(&b_words).copied().collect();
        let union: std::collections::HashSet<&str> = a_words.union(&b_words).copied().collect();

        intersection.len() as f64 / union.len() as f64
    }

    /// Check if an agent has converged.
    pub fn has_converged(&self, agent_name: &str) -> bool {
        self.convergence_signaled
            .get(agent_name)
            .copied()
            .unwrap_or(false)
    }

    /// Get the consecutive count for an agent.
    pub fn consecutive_count(&self, agent_name: &str) -> u32 {
        self.consecutive_counts
            .get(agent_name)
            .copied()
            .unwrap_or(0)
    }

    /// Reset convergence state for an agent.
    pub fn reset(&mut self, agent_name: &str) {
        info!(agent = %agent_name, "resetting convergence state");
        self.consecutive_counts.remove(agent_name);
        self.convergence_signaled.remove(agent_name);
        self.output_history.remove(agent_name);
    }

    /// Get the number of tracked agents.
    pub fn tracked_agent_count(&self) -> usize {
        self.output_history.len()
    }

    /// Clear all convergence state.
    pub fn clear_all(&mut self) {
        info!("clearing all convergence state");
        self.consecutive_counts.clear();
        self.convergence_signaled.clear();
        self.output_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convergence_detector_creation() {
        let detector = ConvergenceDetector::new(0.95, 3);
        assert_eq!(detector.convergence_threshold, 0.95);
        assert_eq!(detector.consecutive_threshold, 3);
        assert_eq!(detector.tracked_agent_count(), 0);
    }

    #[test]
    fn test_similar_identical_strings() {
        let mut detector = ConvergenceDetector::new(0.95, 3);

        // First output - no convergence
        let result = detector.record_output("agent1", "hello world".to_string());
        assert!(result.is_none());

        // Same output - count = 1
        let result = detector.record_output("agent1", "hello world".to_string());
        assert!(result.is_none());
        assert_eq!(detector.consecutive_count("agent1"), 1);

        // Same output - count = 2
        let result = detector.record_output("agent1", "hello world".to_string());
        assert!(result.is_none());
        assert_eq!(detector.consecutive_count("agent1"), 2);

        // Same output - count = 3, convergence!
        let result = detector.record_output("agent1", "hello world".to_string());
        assert!(result.is_some());
        let signal = result.unwrap();
        assert_eq!(signal.agent, "agent1");
        assert_eq!(signal.consecutive_count, 3);
        assert!(signal.similarity >= 0.95);

        // After convergence signaled, subsequent similar outputs don't signal again
        let result = detector.record_output("agent1", "hello world".to_string());
        assert!(result.is_none());
    }

    #[test]
    fn test_divergence_resets_counter() {
        let mut detector = ConvergenceDetector::new(0.95, 3);

        // Build up consecutive similar outputs
        detector.record_output("agent1", "hello world".to_string());
        detector.record_output("agent1", "hello world".to_string());
        assert_eq!(detector.consecutive_count("agent1"), 1);

        // Divergent output resets counter
        let result = detector.record_output("agent1", "completely different text".to_string());
        assert!(result.is_none());
        assert_eq!(detector.consecutive_count("agent1"), 0);

        // Need to build up again
        detector.record_output("agent1", "new consistent text".to_string());
        assert_eq!(detector.consecutive_count("agent1"), 0);

        detector.record_output("agent1", "new consistent text".to_string());
        assert_eq!(detector.consecutive_count("agent1"), 1);
    }

    #[test]
    fn test_partial_similarity() {
        let mut detector = ConvergenceDetector::new(0.5, 2); // Lower threshold

        // Partially similar outputs
        detector.record_output("agent1", "hello world today".to_string());
        let result = detector.record_output("agent1", "hello world tomorrow".to_string());

        // Should detect some similarity
        assert!(detector.consecutive_count("agent1") > 0 || result.is_some());
    }

    #[test]
    fn test_has_converged() {
        let mut detector = ConvergenceDetector::new(0.95, 2);

        assert!(!detector.has_converged("agent1"));

        detector.record_output("agent1", "test".to_string());
        detector.record_output("agent1", "test".to_string());
        detector.record_output("agent1", "test".to_string());

        assert!(detector.has_converged("agent1"));
    }

    #[test]
    fn test_reset() {
        let mut detector = ConvergenceDetector::new(0.95, 2);

        detector.record_output("agent1", "test".to_string());
        detector.record_output("agent1", "test".to_string());
        detector.record_output("agent1", "test".to_string());

        assert!(detector.has_converged("agent1"));

        detector.reset("agent1");

        assert!(!detector.has_converged("agent1"));
        assert_eq!(detector.consecutive_count("agent1"), 0);
    }

    #[test]
    fn test_multiple_agents() {
        let mut detector = ConvergenceDetector::new(0.95, 2);

        // Agent 1 converges
        detector.record_output("agent1", "output".to_string());
        detector.record_output("agent1", "output".to_string());
        detector.record_output("agent1", "output".to_string());

        assert!(detector.has_converged("agent1"));
        assert!(!detector.has_converged("agent2"));

        // Agent 2 doesn't converge
        detector.record_output("agent2", "output1".to_string());
        detector.record_output("agent2", "output2".to_string());
        detector.record_output("agent2", "output3".to_string());

        assert!(!detector.has_converged("agent2"));
    }

    #[test]
    fn test_history_limit() {
        let mut detector = ConvergenceDetector::new(0.95, 2);

        // Add 6 different outputs
        for i in 0..6 {
            detector.record_output("agent1", format!("output{}", i));
        }

        // Should only keep last 5
        assert_eq!(detector.tracked_agent_count(), 1);
    }

    #[test]
    fn test_clear_all() {
        let mut detector = ConvergenceDetector::new(0.95, 2);

        detector.record_output("agent1", "test".to_string());
        detector.record_output("agent2", "test".to_string());

        detector.clear_all();

        assert_eq!(detector.tracked_agent_count(), 0);
        assert!(!detector.has_converged("agent1"));
        assert!(!detector.has_converged("agent2"));
    }
}
