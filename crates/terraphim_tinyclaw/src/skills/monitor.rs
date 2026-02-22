//! Skills monitoring and execution tracking.
//!
//! Provides progress reporting, execution logging, and detailed reports
//! for skill workflows.

use crate::skills::types::{Skill, SkillResult, SkillStatus, StepResult};
use std::fmt;
use std::time::{Duration, Instant};

/// Monitors skill execution progress and generates reports.
#[derive(Debug, Clone)]
pub struct SkillMonitor {
    /// Total number of steps in the skill
    total_steps: usize,
    /// Current step being executed
    current_step: usize,
    /// When execution started
    start_time: Option<Instant>,
    /// Step-by-step timing
    step_durations: Vec<Duration>,
    /// Whether execution was cancelled
    cancelled: bool,
    /// Error message if failed
    error: Option<String>,
}

impl SkillMonitor {
    /// Create a new monitor for a skill with the given number of steps.
    pub fn new(total_steps: usize) -> Self {
        Self {
            total_steps,
            current_step: 0,
            start_time: None,
            step_durations: Vec::with_capacity(total_steps),
            cancelled: false,
            error: None,
        }
    }

    /// Mark execution as started.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Mark the beginning of a step.
    pub fn begin_step(&mut self, step_number: usize) {
        self.current_step = step_number;
    }

    /// Mark the end of a step with its duration.
    pub fn end_step(&mut self, duration: Duration) {
        self.step_durations.push(duration);
    }

    /// Mark execution as cancelled.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Mark execution as failed with an error message.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    /// Get current progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        if self.total_steps == 0 {
            return 1.0;
        }
        self.current_step as f32 / self.total_steps as f32
    }

    /// Get progress as a percentage.
    pub fn progress_percent(&self) -> u8 {
        (self.progress() * 100.0) as u8
    }

    /// Get current step number (1-indexed for display).
    pub fn current_step_display(&self) -> usize {
        self.current_step + 1
    }

    /// Get progress message like "Step 2 of 5".
    pub fn progress_message(&self) -> String {
        format!(
            "Step {} of {}",
            self.current_step_display(),
            self.total_steps
        )
    }

    /// Get elapsed time since start.
    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    /// Get the total duration of completed steps.
    pub fn completed_steps_duration(&self) -> Duration {
        self.step_durations.iter().sum()
    }

    /// Get average step duration.
    pub fn average_step_duration(&self) -> Option<Duration> {
        if self.step_durations.is_empty() {
            return None;
        }
        Some(self.completed_steps_duration() / self.step_durations.len() as u32)
    }

    /// Estimate remaining time based on average step duration.
    pub fn estimated_remaining(&self) -> Option<Duration> {
        let avg = self.average_step_duration()?;
        let remaining_steps = self.total_steps.saturating_sub(self.current_step);
        Some(avg * remaining_steps as u32)
    }

    /// Check if execution is complete.
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.total_steps
    }

    /// Check if execution was successful.
    pub fn is_success(&self) -> bool {
        self.is_complete() && !self.cancelled && self.error.is_none()
    }

    /// Generate a progress bar string.
    pub fn progress_bar(&self, width: usize) -> String {
        let filled = (self.progress() * width as f32) as usize;
        let empty = width.saturating_sub(filled);
        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }

    /// Create a monitor from a skill result for reporting.
    pub fn from_result(result: &SkillResult, total_steps: usize) -> Self {
        let mut monitor = Self::new(total_steps);
        monitor.current_step = result.execution_log.len();
        monitor.step_durations = result
            .execution_log
            .iter()
            .map(|log| Duration::from_millis(log.duration_ms))
            .collect();

        match &result.status {
            SkillStatus::Success => {
                // Normal completion
            }
            SkillStatus::Failed { .. } => {
                monitor.error = Some(format!("{:?}", result.status));
            }
            SkillStatus::Cancelled => {
                monitor.cancelled = true;
            }
            SkillStatus::Timeout => {
                monitor.error = Some("Execution timed out".to_string());
            }
        }

        monitor
    }
}

impl fmt::Display for SkillMonitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} ({}%)",
            self.progress_bar(20),
            self.progress_message(),
            self.progress_percent()
        )
    }
}

/// Builder for creating execution reports.
#[derive(Debug)]
pub struct ExecutionReport {
    /// Skill name
    pub skill_name: String,
    /// Skill version
    pub skill_version: String,
    /// Final status
    pub status: SkillStatus,
    /// Total duration
    pub total_duration_ms: u64,
    /// Step-by-step breakdown
    pub step_reports: Vec<StepReport>,
    /// Summary statistics
    pub statistics: ExecutionStatistics,
}

/// Report for a single step.
#[derive(Debug, Clone)]
pub struct StepReport {
    /// Step number (1-indexed)
    pub step_number: usize,
    /// Step type
    pub step_type: String,
    /// Whether it succeeded
    pub success: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Output preview
    pub output_preview: String,
}

impl From<&StepResult> for StepReport {
    fn from(result: &StepResult) -> Self {
        Self {
            step_number: result.step_number + 1,
            step_type: result.step_type.clone(),
            success: result.success,
            duration_ms: result.duration_ms,
            output_preview: result.output.chars().take(100).collect::<String>(),
        }
    }
}

/// Execution statistics.
#[derive(Debug, Default)]
pub struct ExecutionStatistics {
    /// Total number of steps
    pub total_steps: usize,
    /// Number of successful steps
    pub successful_steps: usize,
    /// Number of failed steps
    pub failed_steps: usize,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Average step duration
    pub average_step_duration_ms: u64,
    /// Slowest step duration
    pub slowest_step_duration_ms: u64,
    /// Fastest step duration
    pub fastest_step_duration_ms: u64,
}

impl ExecutionReport {
    /// Create a report from a skill execution result.
    pub fn from_result(skill: &Skill, result: &SkillResult) -> Self {
        let step_reports: Vec<StepReport> =
            result.execution_log.iter().map(|log| log.into()).collect();

        let successful_steps = step_reports.iter().filter(|r| r.success).count();
        let failed_steps = step_reports.len() - successful_steps;

        let durations: Vec<u64> = step_reports.iter().map(|r| r.duration_ms).collect();
        let avg_duration = if !durations.is_empty() {
            durations.iter().sum::<u64>() / durations.len() as u64
        } else {
            0
        };

        Self {
            skill_name: skill.name.clone(),
            skill_version: skill.version.clone(),
            status: result.status.clone(),
            total_duration_ms: result.duration_ms,
            step_reports,
            statistics: ExecutionStatistics {
                total_steps: skill.steps.len(),
                successful_steps,
                failed_steps,
                total_duration_ms: result.duration_ms,
                average_step_duration_ms: avg_duration,
                slowest_step_duration_ms: durations.iter().copied().max().unwrap_or(0),
                fastest_step_duration_ms: durations.iter().copied().min().unwrap_or(0),
            },
        }
    }

    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        let status_emoji = match &self.status {
            SkillStatus::Success => "✓",
            SkillStatus::Failed { .. } => "✗",
            SkillStatus::Cancelled => "⚠",
            SkillStatus::Timeout => "⏱",
        };

        format!(
            "{} Skill '{}' (v{}) completed in {}ms\n  Steps: {}/{} successful",
            status_emoji,
            self.skill_name,
            self.skill_version,
            self.total_duration_ms,
            self.statistics.successful_steps,
            self.statistics.total_steps
        )
    }

    /// Generate detailed report.
    pub fn detailed(&self) -> String {
        let mut lines = vec![
            format!(
                "Execution Report: {} (v{})",
                self.skill_name, self.skill_version
            ),
            "=".repeat(50),
            format!("Status: {:?}", self.status),
            format!("Total Duration: {}ms", self.total_duration_ms),
            String::new(),
            "Statistics:".to_string(),
            format!("  Total Steps: {}", self.statistics.total_steps),
            format!("  Successful: {}", self.statistics.successful_steps),
            format!("  Failed: {}", self.statistics.failed_steps),
            format!(
                "  Average Step: {}ms",
                self.statistics.average_step_duration_ms
            ),
            format!(
                "  Slowest Step: {}ms",
                self.statistics.slowest_step_duration_ms
            ),
            format!(
                "  Fastest Step: {}ms",
                self.statistics.fastest_step_duration_ms
            ),
            String::new(),
            "Step Details:".to_string(),
        ];

        for report in &self.step_reports {
            let status = if report.success { "✓" } else { "✗" };
            lines.push(format!(
                "  {} Step {} ({}): {}ms",
                status, report.step_number, report.step_type, report.duration_ms
            ));
            if !report.output_preview.is_empty() {
                lines.push(format!("    Preview: {}", report.output_preview));
            }
        }

        lines.join("\n")
    }
}

/// Real-time progress tracker that can be shared across tasks.
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    /// Current progress (0.0 to 1.0)
    progress: std::sync::Arc<std::sync::atomic::AtomicU32>,
    /// Current message
    message: std::sync::Arc<parking_lot::RwLock<String>>,
    /// Whether tracking is active
    active: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new() -> Self {
        Self {
            progress: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            message: std::sync::Arc::new(parking_lot::RwLock::new(String::new())),
            active: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    /// Update progress (0.0 to 1.0).
    pub fn set_progress(&self, progress: f32) {
        let value = (progress.clamp(0.0, 1.0) * 10000.0) as u32;
        self.progress
            .store(value, std::sync::atomic::Ordering::SeqCst);
    }

    /// Get current progress (0.0 to 1.0).
    pub fn get_progress(&self) -> f32 {
        let value = self.progress.load(std::sync::atomic::Ordering::SeqCst);
        value as f32 / 10000.0
    }

    /// Set progress message.
    pub fn set_message(&self, message: impl Into<String>) {
        *self.message.write() = message.into();
    }

    /// Get current progress message.
    pub fn get_message(&self) -> String {
        self.message.read().clone()
    }

    /// Mark tracking as complete.
    pub fn finish(&self) {
        self.active
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.set_progress(1.0);
    }

    /// Check if tracking is still active.
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get a formatted progress string.
    pub fn format_progress(&self) -> String {
        let pct = (self.get_progress() * 100.0) as u8;
        let msg = self.get_message();
        format!("[{}%] {}", pct, msg)
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_new() {
        let monitor = SkillMonitor::new(5);
        assert_eq!(monitor.total_steps, 5);
        assert_eq!(monitor.current_step, 0);
        assert_eq!(monitor.progress(), 0.0);
    }

    #[test]
    fn test_monitor_progress() {
        let mut monitor = SkillMonitor::new(10);
        monitor.start();

        assert_eq!(monitor.progress(), 0.0);
        assert_eq!(monitor.progress_percent(), 0);

        monitor.begin_step(5);
        assert_eq!(monitor.progress(), 0.5);
        assert_eq!(monitor.progress_percent(), 50);
        assert_eq!(monitor.progress_message(), "Step 6 of 10");
    }

    #[test]
    fn test_monitor_complete() {
        let mut monitor = SkillMonitor::new(3);
        monitor.begin_step(3);
        assert!(monitor.is_complete());
        assert!(monitor.is_success());
    }

    #[test]
    fn test_monitor_cancelled() {
        let mut monitor = SkillMonitor::new(5);
        monitor.cancel();
        assert!(!monitor.is_success());
    }

    #[test]
    fn test_monitor_failed() {
        let mut monitor = SkillMonitor::new(5);
        monitor.fail("Something went wrong");
        assert!(!monitor.is_success());
        assert!(monitor.error.is_some());
    }

    #[test]
    fn test_monitor_progress_bar() {
        let mut monitor = SkillMonitor::new(10);
        monitor.begin_step(5);
        let bar = monitor.progress_bar(10);
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));
        assert!(bar.starts_with("["));
        assert!(bar.ends_with("]"));
        // Unicode block chars are 3 bytes each, so check char count not byte len
        assert_eq!(bar.chars().count(), 12); // [ + 10 chars + ]
    }

    #[test]
    fn test_monitor_display() {
        let mut monitor = SkillMonitor::new(10);
        monitor.begin_step(5);
        let display = format!("{}", monitor);
        assert!(display.contains("50%"));
        assert!(display.contains("Step 6 of 10"));
    }

    #[test]
    fn test_monitor_step_durations() {
        let mut monitor = SkillMonitor::new(3);
        monitor.end_step(Duration::from_millis(100));
        monitor.end_step(Duration::from_millis(200));

        assert_eq!(
            monitor.completed_steps_duration(),
            Duration::from_millis(300)
        );
        assert_eq!(
            monitor.average_step_duration(),
            Some(Duration::from_millis(150))
        );
    }

    #[test]
    fn test_monitor_estimated_remaining() {
        let mut monitor = SkillMonitor::new(4);
        monitor.begin_step(2);
        monitor.end_step(Duration::from_millis(100));
        monitor.end_step(Duration::from_millis(100));

        // 2 steps done, avg 100ms, 2 remaining = 200ms estimated
        assert_eq!(
            monitor.estimated_remaining(),
            Some(Duration::from_millis(200))
        );
    }

    #[test]
    fn test_execution_report_from_result() {
        let skill = Skill {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: None,
            steps: vec![
                crate::skills::types::SkillStep::Llm {
                    prompt: "Step 1".to_string(),
                    use_context: false,
                },
                crate::skills::types::SkillStep::Llm {
                    prompt: "Step 2".to_string(),
                    use_context: false,
                },
            ],
            inputs: vec![],
        };

        let result = SkillResult {
            status: SkillStatus::Success,
            output: "Test output".to_string(),
            execution_log: vec![
                StepResult {
                    step_number: 0,
                    step_type: "llm".to_string(),
                    success: true,
                    output: "Output 1".to_string(),
                    duration_ms: 100,
                },
                StepResult {
                    step_number: 1,
                    step_type: "llm".to_string(),
                    success: true,
                    output: "Output 2".to_string(),
                    duration_ms: 200,
                },
            ],
            duration_ms: 300,
        };

        let report = ExecutionReport::from_result(&skill, &result);
        assert_eq!(report.skill_name, "test-skill");
        assert_eq!(report.skill_version, "1.0.0");
        assert_eq!(report.statistics.total_steps, 2);
        assert_eq!(report.statistics.successful_steps, 2);
        assert_eq!(report.statistics.average_step_duration_ms, 150);
    }

    #[test]
    fn test_execution_report_summary() {
        let skill = Skill {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: None,
            steps: vec![],
            inputs: vec![],
        };

        let result = SkillResult {
            status: SkillStatus::Success,
            output: String::new(),
            execution_log: vec![],
            duration_ms: 1000,
        };

        let report = ExecutionReport::from_result(&skill, &result);
        let summary = report.summary();
        assert!(summary.contains("test"));
        assert!(summary.contains("1000ms"));
        assert!(summary.contains("✓"));
    }

    #[test]
    fn test_execution_report_failed() {
        let skill = Skill {
            name: "failing".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: None,
            steps: vec![],
            inputs: vec![],
        };

        let result = SkillResult {
            status: SkillStatus::Failed {
                step: 1,
                error: "Oops".to_string(),
            },
            output: String::new(),
            execution_log: vec![StepResult {
                step_number: 0,
                step_type: "llm".to_string(),
                success: true,
                output: "Good".to_string(),
                duration_ms: 100,
            }],
            duration_ms: 100,
        };

        let report = ExecutionReport::from_result(&skill, &result);
        assert_eq!(report.statistics.successful_steps, 1);
        assert_eq!(report.statistics.failed_steps, 0); // Failed means didn't complete, not step failure
    }

    #[test]
    fn test_progress_tracker() {
        let tracker = ProgressTracker::new();
        assert!(tracker.is_active());
        assert_eq!(tracker.get_progress(), 0.0);

        tracker.set_progress(0.5);
        assert_eq!(tracker.get_progress(), 0.5);

        tracker.set_message("Processing...");
        assert_eq!(tracker.get_message(), "Processing...");

        let formatted = tracker.format_progress();
        assert!(formatted.contains("50%"));
        assert!(formatted.contains("Processing..."));

        tracker.finish();
        assert!(!tracker.is_active());
        assert_eq!(tracker.get_progress(), 1.0);
    }

    #[test]
    fn test_progress_tracker_clamping() {
        let tracker = ProgressTracker::new();

        tracker.set_progress(-0.5);
        assert_eq!(tracker.get_progress(), 0.0);

        tracker.set_progress(1.5);
        assert_eq!(tracker.get_progress(), 1.0);
    }

    #[test]
    fn test_step_report_from_result() {
        let result = StepResult {
            step_number: 0,
            step_type: "tool".to_string(),
            success: true,
            output: "Some output here that is long".to_string(),
            duration_ms: 500,
        };

        let report = StepReport::from(&result);
        assert_eq!(report.step_number, 1); // Converted to 1-indexed
        assert_eq!(report.step_type, "tool");
        assert!(report.success);
        assert_eq!(report.output_preview, result.output); // Not truncated since < 100 chars

        // Test truncation with longer output
        let long_result = StepResult {
            step_number: 1,
            step_type: "llm".to_string(),
            success: true,
            output: "x".repeat(150),
            duration_ms: 100,
        };
        let long_report = StepReport::from(&long_result);
        assert_eq!(long_report.output_preview.len(), 100); // Truncated to 100
    }
}
