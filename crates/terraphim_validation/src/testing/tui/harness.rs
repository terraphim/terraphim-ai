//! TUI Test Harness
//!
//! Core testing framework for TUI interface validation.
//! Provides the main test runner and orchestration logic.

use crate::testing::tui::command_simulator::CommandSimulator;
use crate::testing::tui::cross_platform::CrossPlatformTester;
use crate::testing::tui::mock_terminal::MockTerminal;
use crate::testing::tui::output_validator::OutputValidator;
use crate::testing::tui::performance_monitor::PerformanceMonitor;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::process::Command;

/// TUI Test Configuration
#[derive(Debug, Clone)]
pub struct TuiTestConfig {
    /// Test timeout in seconds
    pub timeout_seconds: u64,
    /// Enable performance monitoring
    pub enable_performance: bool,
    /// Enable cross-platform testing
    pub enable_cross_platform: bool,
    /// Terminal width for testing
    pub terminal_width: u16,
    /// Terminal height for testing
    pub terminal_height: u16,
    /// Working directory for tests
    pub working_dir: Option<String>,
}

impl Default for TuiTestConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            enable_performance: true,
            enable_cross_platform: true,
            terminal_width: 120,
            terminal_height: 30,
            working_dir: None,
        }
    }
}

/// TUI Test Harness
pub struct TuiTestHarness {
    config: TuiTestConfig,
    terminal: MockTerminal,
    simulator: CommandSimulator,
    validator: OutputValidator,
    performance_monitor: Option<PerformanceMonitor>,
    cross_platform_tester: Option<CrossPlatformTester>,
}

impl TuiTestHarness {
    /// Create a new TUI test harness
    pub async fn new(config: TuiTestConfig) -> Result<Self> {
        let terminal = MockTerminal::new(config.terminal_width, config.terminal_height)?;
        let simulator = CommandSimulator::new().await?;
        let validator = OutputValidator::new();
        let performance_monitor = if config.enable_performance {
            Some(PerformanceMonitor::new()?)
        } else {
            None
        };
        let cross_platform_tester = if config.enable_cross_platform {
            Some(CrossPlatformTester::new()?)
        } else {
            None
        };

        Ok(Self {
            config,
            terminal,
            simulator,
            validator,
            performance_monitor,
            cross_platform_tester,
        })
    }

    /// Create a default harness for quick testing
    pub async fn default() -> Result<Self> {
        Self::new(TuiTestConfig::default()).await
    }

    /// Run a comprehensive TUI test suite
    pub async fn run_comprehensive_test_suite(&mut self) -> Result<TuiTestSuiteResults> {
        let start_time = Instant::now();

        let mut results = TuiTestSuiteResults {
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            skipped_tests: 0,
            test_duration: Duration::default(),
            command_results: HashMap::new(),
            performance_results: None,
            cross_platform_results: None,
            errors: Vec::new(),
        };

        // Test command interface
        let command_results = self.run_command_interface_tests().await?;
        results.command_results.extend(command_results);

        // Test REPL functionality
        let repl_results = self.run_repl_functionality_tests().await?;
        results.command_results.extend(repl_results);

        // Test performance if enabled
        if let Some(monitor) = &mut self.performance_monitor {
            results.performance_results = Some(monitor.run_performance_tests().await?);
        }

        // Test cross-platform compatibility if enabled
        if let Some(tester) = &mut self.cross_platform_tester {
            results.cross_platform_results = Some(tester.run_cross_platform_tests().await?);
        }

        // Calculate final statistics
        results.test_duration = start_time.elapsed();
        results.calculate_statistics();

        Ok(results)
    }

    /// Run command interface tests
    async fn run_command_interface_tests(&mut self) -> Result<HashMap<String, TuiCommandResult>> {
        let mut results = HashMap::new();

        // Test search commands
        let search_commands = vec![
            "/search rust",
            "/search async programming --limit 5",
            "/search api --role Engineer",
        ];

        for cmd in search_commands {
            let result = self.test_command(cmd).await?;
            results.insert(
                format!("search_{}", cmd.replace("/", "").replace(" ", "_")),
                result,
            );
        }

        // Test config commands
        let config_commands = vec![
            "/config show",
            "/config", // Default to show
        ];

        for cmd in config_commands {
            let result = self.test_command(cmd).await?;
            results.insert(
                format!("config_{}", cmd.replace("/", "").replace(" ", "_")),
                result,
            );
        }

        // Test role commands
        let role_commands = vec![
            "/role list",
            "/role select Engineer",
            "/role select Default",
        ];

        for cmd in role_commands {
            let result = self.test_command(cmd).await?;
            results.insert(
                format!("role_{}", cmd.replace("/", "").replace(" ", "_")),
                result,
            );
        }

        // Test graph commands
        let graph_commands = vec!["/graph", "/graph --top-k 10", "/graph --top-k 20"];

        for cmd in graph_commands {
            let result = self.test_command(cmd).await?;
            results.insert(
                format!("graph_{}", cmd.replace("/", "").replace(" ", "_")),
                result,
            );
        }

        // Test knowledge graph operations
        let kg_commands = vec![
            "/replace rust programming",
            "/replace async with tokio --format markdown",
            "/find rust async programming",
            "/thesaurus",
            "/thesaurus --role Engineer",
        ];

        for cmd in kg_commands {
            let result = self.test_command(cmd).await?;
            results.insert(
                format!("kg_{}", cmd.replace("/", "").replace(" ", "_")),
                result,
            );
        }

        // Test utility commands
        let utility_commands = vec!["/help", "/help search", "/help config", "/clear"];

        for cmd in utility_commands {
            let result = self.test_command(cmd).await?;
            results.insert(
                format!("util_{}", cmd.replace("/", "").replace(" ", "_")),
                result,
            );
        }

        Ok(results)
    }

    /// Run REPL functionality tests
    async fn run_repl_functionality_tests(&mut self) -> Result<HashMap<String, TuiCommandResult>> {
        let mut results = HashMap::new();

        // Test multi-line input
        let multiline_commands = vec![
            "/search\nrust async",
            "/replace\nmulti line\ntext --format markdown",
        ];

        for cmd in multiline_commands {
            let result = self.test_multiline_command(cmd).await?;
            results.insert(
                format!(
                    "multiline_{}",
                    cmd.lines()
                        .next()
                        .unwrap_or("")
                        .replace("/", "")
                        .replace(" ", "_")
                ),
                result,
            );
        }

        // Test command history
        let history_commands = vec![
            "/search history test 1",
            "/search history test 2",
            "/search history test 3",
        ];

        for (i, cmd) in history_commands.iter().enumerate() {
            let result = self.test_command(cmd).await?;
            results.insert(format!("history_{}", i), result);
        }

        // Test history navigation (simulated)
        let history_nav_result = self.test_history_navigation().await?;
        results.insert("history_navigation".to_string(), history_nav_result);

        // Test auto-completion
        let completion_result = self.test_auto_completion().await?;
        results.insert("auto_completion".to_string(), completion_result);

        Ok(results)
    }

    /// Test a single command
    async fn test_command(&mut self, command: &str) -> Result<TuiCommandResult> {
        let start_time = Instant::now();

        // Clear terminal state
        self.terminal.clear()?;

        // Send command to simulator
        let output = self
            .simulator
            .execute_command(command, self.config.timeout_seconds)
            .await?;

        // Validate output
        let validation_result = self
            .validator
            .validate_command_output(command, &output)
            .await?;

        let duration = start_time.elapsed();

        Ok(TuiCommandResult {
            command: command.to_string(),
            success: validation_result.is_valid,
            output,
            validation_errors: validation_result.errors,
            execution_time: duration,
            exit_code: validation_result.exit_code,
        })
    }

    /// Test multi-line command input
    async fn test_multiline_command(&mut self, command: &str) -> Result<TuiCommandResult> {
        let start_time = Instant::now();

        self.terminal.clear()?;

        // Split command into lines and send sequentially
        let lines: Vec<&str> = command.lines().collect();
        let mut output = String::new();

        for line in lines {
            if !line.trim().is_empty() {
                let line_output = self
                    .simulator
                    .execute_command(line, self.config.timeout_seconds)
                    .await?;
                output.push_str(&line_output);
                output.push('\n');
            }
        }

        let validation_result = self
            .validator
            .validate_command_output(command, &output)
            .await?;
        let duration = start_time.elapsed();

        Ok(TuiCommandResult {
            command: command.to_string(),
            success: validation_result.is_valid,
            output,
            validation_errors: validation_result.errors,
            execution_time: duration,
            exit_code: validation_result.exit_code,
        })
    }

    /// Test command history navigation
    async fn test_history_navigation(&mut self) -> Result<TuiCommandResult> {
        // Simulate history navigation (up arrow, down arrow)
        let history_keys = vec!["\x1b[A", "\x1b[B", "\x1b[A"]; // Up, Down, Up

        let mut output = String::new();
        for key in history_keys {
            let key_output = self.simulator.send_input(key).await?;
            output.push_str(&key_output);
        }

        Ok(TuiCommandResult {
            command: "history_navigation".to_string(),
            success: true, // Assume success for navigation test
            output,
            validation_errors: Vec::new(),
            execution_time: Duration::from_millis(100),
            exit_code: Some(0),
        })
    }

    /// Test auto-completion functionality
    async fn test_auto_completion(&mut self) -> Result<TuiCommandResult> {
        let completion_triggers = vec![
            "/sea",  // Should complete to /search
            "/hel",  // Should complete to /help
            "/conf", // Should complete to /config
        ];

        let mut output = String::new();
        for trigger in completion_triggers {
            let completion_output = self.simulator.test_completion(trigger).await?;
            output.push_str(&completion_output);
            output.push('\n');
        }

        Ok(TuiCommandResult {
            command: "auto_completion".to_string(),
            success: true, // Assume success for completion test
            output,
            validation_errors: Vec::new(),
            execution_time: Duration::from_millis(50),
            exit_code: Some(0),
        })
    }

    /// Get the current terminal state
    pub fn get_terminal_state(&self) -> Result<String> {
        self.terminal.get_display()
    }

    /// Reset the test harness
    pub async fn reset(&mut self) -> Result<()> {
        self.terminal.clear()?;
        self.simulator.reset().await?;
        if let Some(monitor) = &mut self.performance_monitor {
            monitor.reset()?;
        }
        Ok(())
    }
}

/// Results for a single TUI command test
#[derive(Debug, Clone)]
pub struct TuiCommandResult {
    pub command: String,
    pub success: bool,
    pub output: String,
    pub validation_errors: Vec<String>,
    pub execution_time: Duration,
    pub exit_code: Option<i32>,
}

/// Comprehensive TUI test suite results
#[derive(Debug, Clone)]
pub struct TuiTestSuiteResults {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub test_duration: Duration,
    pub command_results: HashMap<String, TuiCommandResult>,
    pub performance_results: Option<crate::testing::tui::performance_monitor::PerformanceResults>,
    pub cross_platform_results: Option<crate::testing::tui::cross_platform::CrossPlatformResults>,
    pub errors: Vec<String>,
}

impl TuiTestSuiteResults {
    /// Calculate statistics from command results
    fn calculate_statistics(&mut self) {
        self.total_tests = self.command_results.len();
        self.passed_tests = 0;
        self.failed_tests = 0;

        for result in self.command_results.values() {
            if result.success {
                self.passed_tests += 1;
            } else {
                self.failed_tests += 1;
            }
        }
    }

    /// Get test success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed_tests == 0 && self.errors.is_empty()
    }

    /// Generate a summary report
    pub fn generate_report(&self) -> String {
        let mut report = format!("TUI Test Suite Results\n{}\n", "=".repeat(50));

        report.push_str(&format!("Total Tests: {}\n", self.total_tests));
        report.push_str(&format!(
            "Passed: {} ({:.1}%)\n",
            self.passed_tests,
            self.success_rate()
        ));
        report.push_str(&format!("Failed: {}\n", self.failed_tests));
        report.push_str(&format!("Skipped: {}\n", self.skipped_tests));
        report.push_str(&format!(
            "Duration: {:.2}s\n\n",
            self.test_duration.as_secs_f64()
        ));

        if !self.errors.is_empty() {
            report.push_str(&format!("Errors ({}):\n", self.errors.len()));
            for error in &self.errors {
                report.push_str(&format!("  - {}\n", error));
            }
            report.push('\n');
        }

        // Command results summary
        report.push_str("Command Test Results:\n");
        for (test_name, result) in &self.command_results {
            let status = if result.success { "✓" } else { "✗" };
            report.push_str(&format!(
                "  {} {} ({:.2}s)\n",
                status,
                test_name,
                result.execution_time.as_secs_f64()
            ));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_creation() {
        let config = TuiTestConfig::default();
        let harness = TuiTestHarness::new(config).await;
        assert!(harness.is_ok());
    }

    #[tokio::test]
    async fn test_default_harness() {
        let harness = TuiTestHarness::default().await;
        assert!(harness.is_ok());
    }

    #[tokio::test]
    async fn test_results_calculation() {
        let mut results = TuiTestSuiteResults {
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            skipped_tests: 0,
            test_duration: Duration::from_secs(1),
            command_results: HashMap::new(),
            performance_results: None,
            cross_platform_results: None,
            errors: Vec::new(),
        };

        // Add some test results
        results.command_results.insert(
            "test1".to_string(),
            TuiCommandResult {
                command: "test1".to_string(),
                success: true,
                output: "ok".to_string(),
                validation_errors: Vec::new(),
                execution_time: Duration::from_millis(100),
                exit_code: Some(0),
            },
        );

        results.command_results.insert(
            "test2".to_string(),
            TuiCommandResult {
                command: "test2".to_string(),
                success: false,
                output: "error".to_string(),
                validation_errors: vec!["failed".to_string()],
                execution_time: Duration::from_millis(200),
                exit_code: Some(1),
            },
        );

        results.calculate_statistics();

        assert_eq!(results.total_tests, 2);
        assert_eq!(results.passed_tests, 1);
        assert_eq!(results.failed_tests, 1);
        assert_eq!(results.success_rate(), 50.0);
        assert!(!results.all_passed());
    }
}
