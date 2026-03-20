use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::{info, warn};

/// A report indicating detected drift from strategic goals.
#[derive(Debug, Clone)]
pub struct DriftReport {
    pub agent: String,
    pub drift_score: f64,
    pub explanation: String,
}

/// Monitors agent outputs against strategic goals to detect drift.
pub struct DriftDetector {
    /// How many ticks between drift checks.
    pub check_interval_ticks: u32,
    /// Threshold above which a warning is logged (0.0 - 1.0).
    pub drift_threshold: f64,
    /// Path to the plans directory containing strategic goals.
    pub plans_dir: PathBuf,
    /// Tick counter (incremented on each check call).
    tick_counter: u32,
    /// Cached strategic goals loaded from plans directory.
    strategic_goals: Vec<String>,
    /// History of agent outputs for comparison.
    agent_output_history: HashMap<String, Vec<String>>,
}

impl DriftDetector {
    /// Create a new drift detector with the given configuration.
    pub fn new(
        check_interval_ticks: u32,
        drift_threshold: f64,
        plans_dir: impl AsRef<Path>,
    ) -> Self {
        let plans_path = plans_dir.as_ref().to_path_buf();
        let strategic_goals = Self::load_strategic_goals(&plans_path);

        info!(
            check_interval = check_interval_ticks,
            threshold = drift_threshold,
            plans_dir = %plans_path.display(),
            goals_loaded = strategic_goals.len(),
            "drift detector initialized"
        );

        Self {
            check_interval_ticks,
            drift_threshold,
            plans_dir: plans_path,
            tick_counter: 0,
            strategic_goals,
            agent_output_history: HashMap::new(),
        }
    }

    /// Load strategic goals from the plans directory.
    fn load_strategic_goals(plans_dir: &Path) -> Vec<String> {
        let mut goals = Vec::new();

        if !plans_dir.exists() {
            warn!(plans_dir = %plans_dir.display(), "plans directory does not exist");
            return goals;
        }

        // Read all .md files from the plans directory
        if let Ok(entries) = std::fs::read_dir(plans_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        info!(file = %path.display(), "loaded strategic goal");
                        goals.push(content);
                    }
                }
            }
        }

        goals
    }

    /// Record an agent output for later drift analysis.
    pub fn record_agent_output(&mut self, agent_name: &str, output: String) {
        self.agent_output_history
            .entry(agent_name.to_string())
            .or_default()
            .push(output);

        // Keep only the last 10 outputs per agent to limit memory usage
        if let Some(outputs) = self.agent_output_history.get_mut(agent_name) {
            if outputs.len() > 10 {
                outputs.remove(0);
            }
        }
    }

    /// Check for drift on every Nth tick. Returns drift reports if any detected.
    pub fn check_drift(&mut self, agent_name: &str, current_output: &str) -> Option<DriftReport> {
        self.tick_counter += 1;

        // Only check on every Nth tick
        if self.tick_counter % self.check_interval_ticks != 0 {
            return None;
        }

        // Record this output
        self.record_agent_output(agent_name, current_output.to_string());

        // Calculate drift score by comparing against strategic goals
        let drift_score = self.calculate_drift_score(current_output);

        if drift_score > self.drift_threshold {
            let report = DriftReport {
                agent: agent_name.to_string(),
                drift_score,
                explanation: format!(
                    "Agent output deviates {:.1}% from strategic goals",
                    drift_score * 100.0
                ),
            };

            warn!(
                agent = %agent_name,
                drift_score = %drift_score,
                threshold = %self.drift_threshold,
                "STRATEGIC DRIFT DETECTED"
            );

            return Some(report);
        }

        None
    }

    /// Calculate drift score by comparing output against strategic goals.
    /// Returns a score between 0.0 (no drift) and 1.0 (complete drift).
    fn calculate_drift_score(&self, output: &str) -> f64 {
        if self.strategic_goals.is_empty() {
            // No goals to compare against, assume no drift
            return 0.0;
        }

        // Simple keyword-based drift detection
        // Count how many goal keywords appear in the output
        let output_lower = output.to_lowercase();
        let mut total_keywords = 0;
        let mut matched_keywords = 0;

        for goal in &self.strategic_goals {
            // Extract keywords from goal (simple approach: words longer than 5 chars)
            let goal_lower = goal.to_lowercase();
            let keywords: Vec<&str> = goal_lower
                .split_whitespace()
                .filter(|w| w.len() > 5 && w.chars().all(|c| c.is_alphanumeric()))
                .collect();

            for keyword in keywords {
                total_keywords += 1;
                if output_lower.contains(keyword) {
                    matched_keywords += 1;
                }
            }
        }

        if total_keywords == 0 {
            return 0.0;
        }

        // Drift is inverse of keyword match ratio
        let match_ratio = matched_keywords as f64 / total_keywords as f64;
        1.0 - match_ratio
    }

    /// Get the current tick counter value.
    pub fn tick_counter(&self) -> u32 {
        self.tick_counter
    }

    /// Get the number of strategic goals loaded.
    pub fn strategic_goals_count(&self) -> usize {
        self.strategic_goals.len()
    }

    /// Manually reload strategic goals from the plans directory.
    pub fn reload_goals(&mut self) {
        self.strategic_goals = Self::load_strategic_goals(&self.plans_dir);
        info!(
            goals_count = self.strategic_goals.len(),
            "strategic goals reloaded"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_plans_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        // Create a mock strategic goal file
        let goal_path = dir.path().join("strategy.md");
        let mut file = std::fs::File::create(&goal_path).unwrap();
        writeln!(
            file,
            "Our strategic goal is to implement high quality code with comprehensive testing"
        )
        .unwrap();
        writeln!(
            file,
            "and security best practices throughout the entire codebase"
        )
        .unwrap();

        // Create another goal file
        let goal_path2 = dir.path().join("vision.md");
        let mut file2 = std::fs::File::create(&goal_path2).unwrap();
        writeln!(
            file2,
            "We prioritize performance optimization and scalable architecture"
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_drift_detector_creation() {
        let dir = create_test_plans_dir();
        let detector = DriftDetector::new(5, 0.5, dir.path());

        assert_eq!(detector.check_interval_ticks, 5);
        assert_eq!(detector.drift_threshold, 0.5);
        assert_eq!(detector.tick_counter(), 0);
        assert_eq!(detector.strategic_goals_count(), 2);
    }

    #[test]
    fn test_drift_detector_no_goals() {
        let dir = tempfile::tempdir().unwrap();
        let detector = DriftDetector::new(5, 0.5, dir.path());

        assert_eq!(detector.strategic_goals_count(), 0);
    }

    #[test]
    fn test_drift_check_interval() {
        let dir = create_test_plans_dir();
        let mut detector = DriftDetector::new(3, 0.5, dir.path());

        // First 2 checks should return None (not on 3rd tick yet)
        assert!(detector.check_drift("agent1", "some output").is_none());
        assert_eq!(detector.tick_counter(), 1);

        assert!(detector.check_drift("agent1", "some output").is_none());
        assert_eq!(detector.tick_counter(), 2);

        // 3rd check should evaluate (but may or may not detect drift)
        let _ = detector.check_drift("agent1", "some output");
        assert_eq!(detector.tick_counter(), 3);
    }

    #[test]
    fn test_drift_score_calculation() {
        let dir = create_test_plans_dir();
        let detector = DriftDetector::new(1, 0.3, dir.path());

        // Output that contains many goal keywords should have low drift
        let aligned_output = "We are implementing comprehensive testing and security best practices for high quality code";
        let aligned_score = detector.calculate_drift_score(aligned_output);
        assert!(
            aligned_score < 0.8,
            "aligned output should have low drift score, got {}",
            aligned_score
        );

        // Output that doesn't match goals should have high drift
        let divergent_output =
            "We are building a pizza delivery app with lots of cheese and toppings";
        let divergent_score = detector.calculate_drift_score(divergent_output);
        assert!(
            divergent_score > 0.3,
            "divergent output should have high drift score, got {}",
            divergent_score
        );
    }

    #[test]
    fn test_drift_report_generation() {
        let dir = create_test_plans_dir();
        let mut detector = DriftDetector::new(1, 0.3, dir.path());

        // Output that deviates from goals should trigger a report
        let divergent_output =
            "Building a game with graphics and sound effects for entertainment purposes";
        let report = detector.check_drift("test-agent", divergent_output);

        assert!(report.is_some(), "should generate drift report");
        let report = report.unwrap();
        assert_eq!(report.agent, "test-agent");
        assert!(report.drift_score > 0.3);
        assert!(!report.explanation.is_empty());
    }

    #[test]
    fn test_no_drift_below_threshold() {
        let dir = create_test_plans_dir();
        let mut detector = DriftDetector::new(1, 0.9, dir.path()); // High threshold

        // Output that somewhat aligns should not trigger report
        let output = "We focus on quality code implementation";
        let report = detector.check_drift("test-agent", output);

        assert!(
            report.is_none(),
            "should not generate report below threshold"
        );
    }

    #[test]
    fn test_output_history() {
        let dir = create_test_plans_dir();
        let mut detector = DriftDetector::new(1, 0.5, dir.path());

        // Record multiple outputs
        for i in 0..12 {
            detector.record_agent_output("agent1", format!("output {}", i));
        }

        let history = detector.agent_output_history.get("agent1").unwrap();
        assert_eq!(history.len(), 10, "should keep only last 10 outputs");
        assert_eq!(history[0], "output 2"); // Oldest should be output 2
        assert_eq!(history[9], "output 11"); // Newest should be output 11
    }

    #[test]
    fn test_reload_goals() {
        let dir = create_test_plans_dir();
        let mut detector = DriftDetector::new(5, 0.5, dir.path());

        assert_eq!(detector.strategic_goals_count(), 2);

        // Create a new goal file
        let new_goal_path = dir.path().join("new_goal.md");
        let mut file = std::fs::File::create(&new_goal_path).unwrap();
        writeln!(file, "New goal: focus on user experience").unwrap();

        // Reload goals
        detector.reload_goals();
        assert_eq!(detector.strategic_goals_count(), 3);
    }
}
