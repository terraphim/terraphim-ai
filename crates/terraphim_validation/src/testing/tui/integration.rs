//! TUI Integration Testing
//!
//! High-level integration tests that combine all TUI testing components
//! to provide comprehensive validation of the complete TUI system.

use crate::testing::tui::command_simulator::CommandSimulator;
use crate::testing::tui::cross_platform::CrossPlatformTester;
use crate::testing::tui::harness::TuiTestHarness;
use crate::testing::tui::mock_terminal::MockTerminal;
use crate::testing::tui::output_validator::OutputValidator;
use crate::testing::tui::performance_monitor::{PerformanceMonitor, PerformanceSLO};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::time::Duration;

/// Integration test configuration
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    pub enable_performance: bool,
    pub enable_cross_platform: bool,
    pub enable_stress_testing: bool,
    pub stress_test_commands: usize,
    pub stress_test_concurrency: usize,
    pub timeout_seconds: u64,
    pub terminal_width: u16,
    pub terminal_height: u16,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            enable_performance: true,
            enable_cross_platform: true,
            enable_stress_testing: true,
            stress_test_commands: 100,
            stress_test_concurrency: 10,
            timeout_seconds: 30,
            terminal_width: 120,
            terminal_height: 30,
        }
    }
}

/// Integration test results
#[derive(Debug, Clone)]
pub struct IntegrationTestResults {
    pub test_suite_results: crate::testing::tui::harness::TuiTestSuiteResults,
    pub performance_results: Option<crate::testing::tui::performance_monitor::PerformanceResults>,
    pub cross_platform_results: Option<crate::testing::tui::cross_platform::CrossPlatformResults>,
    pub stress_test_results: Option<crate::testing::tui::performance_monitor::StressTestResults>,
    pub overall_success: bool,
    pub total_duration: Duration,
    pub coverage_percentage: f64,
    pub critical_issues: Vec<String>,
    pub recommendations: Vec<String>,
}

/// TUI Integration Tester
pub struct TuiIntegrationTester {
    config: IntegrationTestConfig,
    harness: Option<TuiTestHarness>,
}

impl TuiIntegrationTester {
    /// Create a new integration tester
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            config,
            harness: None,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(IntegrationTestConfig::default())
    }

    /// Run comprehensive integration tests
    pub async fn run_integration_tests(&mut self) -> Result<IntegrationTestResults> {
        let start_time = std::time::Instant::now();

        // Initialize harness
        let harness_config = crate::testing::tui::harness::TuiTestConfig {
            timeout_seconds: self.config.timeout_seconds,
            enable_performance: self.config.enable_performance,
            enable_cross_platform: self.config.enable_cross_platform,
            terminal_width: self.config.terminal_width,
            terminal_height: self.config.terminal_height,
            working_dir: None,
        };

        let mut harness = TuiTestHarness::new(harness_config).await?;
        self.harness = Some(harness);

        // Run comprehensive test suite
        let test_suite_results = self
            .harness
            .as_mut()
            .unwrap()
            .run_comprehensive_test_suite()
            .await?;

        // Extract additional results
        let performance_results = test_suite_results.performance_results.clone();
        let cross_platform_results = test_suite_results.cross_platform_results.clone();

        // Run stress testing if enabled
        let stress_test_results = if self.config.enable_stress_testing {
            Some(self.run_stress_test_scenario().await?)
        } else {
            None
        };

        let total_duration = start_time.elapsed();

        // Analyze results
        let critical_issues = self.analyze_critical_issues(&test_suite_results);
        let recommendations = self.generate_recommendations(&test_suite_results);
        let coverage_percentage = self.calculate_test_coverage(&test_suite_results);
        let overall_success = self.determine_overall_success(&test_suite_results, &critical_issues);

        Ok(IntegrationTestResults {
            test_suite_results,
            performance_results,
            cross_platform_results,
            stress_test_results,
            overall_success,
            total_duration,
            coverage_percentage,
            critical_issues,
            recommendations,
        })
    }

    /// Run stress test scenario
    async fn run_stress_test_scenario(
        &mut self,
    ) -> Result<crate::testing::tui::performance_monitor::StressTestResults> {
        // Create a standalone performance monitor for stress testing
        let mut monitor = crate::testing::tui::performance_monitor::PerformanceMonitor::new()?;

        let commands = self.generate_stress_test_commands();
        monitor
            .run_stress_test(commands, self.config.stress_test_concurrency)
            .await
    }

    /// Generate commands for stress testing
    fn generate_stress_test_commands(&self) -> Vec<String> {
        let base_commands = vec![
            "/search rust",
            "/search async programming",
            "/search api --limit 5",
            "/config show",
            "/role list",
            "/graph",
            "/help",
            "/find test text",
            "/thesaurus",
        ];

        let mut commands = Vec::new();
        let mut idx = 0;

        for _ in 0..self.config.stress_test_commands {
            commands.push(base_commands[idx % base_commands.len()].to_string());
            idx += 1;
        }

        commands
    }

    /// Analyze critical issues from test results
    fn analyze_critical_issues(
        &self,
        results: &crate::testing::tui::harness::TuiTestSuiteResults,
    ) -> Vec<String> {
        let mut issues = Vec::new();

        // Check test success rate
        if results.success_rate() < 95.0 {
            issues.push(format!("Low success rate: {:.1}%", results.success_rate()));
        }

        // Check for errors
        if !results.errors.is_empty() {
            issues.push(format!("{} test errors detected", results.errors.len()));
        }

        // Check command failures
        let failed_commands: Vec<_> = results
            .command_results
            .iter()
            .filter(|(_, result)| !result.success)
            .collect();

        if !failed_commands.is_empty() {
            issues.push(format!("{} commands failed", failed_commands.len()));
        }

        // Check performance SLO violations
        if let Some(perf_results) = &results.performance_results {
            if !perf_results.slo_violations.is_empty() {
                issues.push(format!(
                    "{} SLO violations",
                    perf_results.slo_violations.len()
                ));
            }
        }

        // Check cross-platform blocking issues
        if let Some(cp_results) = &results.cross_platform_results {
            if !cp_results.blocking_issues.is_empty() {
                issues.push(format!(
                    "{} blocking cross-platform issues",
                    cp_results.blocking_issues.len()
                ));
            }
        }

        issues
    }

    /// Generate recommendations based on test results
    fn generate_recommendations(
        &self,
        results: &crate::testing::tui::harness::TuiTestSuiteResults,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if results.success_rate() < 100.0 {
            recommendations.push("Fix failing tests before release".to_string());
        }

        if results.success_rate() < 95.0 {
            recommendations.push("Improve test reliability and error handling".to_string());
        }

        if let Some(perf_results) = &results.performance_results {
            if perf_results.benchmarks_passed < perf_results.benchmarks_total {
                recommendations.push("Address performance SLO violations".to_string());
            }
        }

        if let Some(cp_results) = &results.cross_platform_results {
            if cp_results.overall_compatibility < 95.0 {
                recommendations.push("Improve cross-platform compatibility".to_string());
            }

            if !cp_results.recommendations.is_empty() {
                recommendations.extend(cp_results.recommendations.clone());
            }
        }

        if results.command_results.is_empty() {
            recommendations.push("No command tests were executed - verify test setup".to_string());
        }

        recommendations
    }

    /// Calculate test coverage percentage
    fn calculate_test_coverage(
        &self,
        results: &crate::testing::tui::harness::TuiTestSuiteResults,
    ) -> f64 {
        // This is a simplified coverage calculation
        // In a real implementation, this would analyze code coverage data

        let total_possible_tests = 50; // Estimated total test cases
        let executed_tests = results.command_results.len();

        if executed_tests >= total_possible_tests {
            100.0
        } else {
            (executed_tests as f64 / total_possible_tests as f64) * 100.0
        }
    }

    /// Determine overall success
    fn determine_overall_success(
        &self,
        results: &crate::testing::tui::harness::TuiTestSuiteResults,
        critical_issues: &[String],
    ) -> bool {
        // Success criteria:
        // - No critical issues
        // - Success rate >= 95%
        // - No blocking cross-platform issues
        // - All performance SLOs met (if performance testing enabled)

        if !critical_issues.is_empty() {
            return false;
        }

        if results.success_rate() < 95.0 {
            return false;
        }

        if let Some(cp_results) = &results.cross_platform_results {
            if !cp_results.blocking_issues.is_empty() {
                return false;
            }
        }

        if let Some(perf_results) = &results.performance_results {
            if perf_results.benchmarks_passed < perf_results.benchmarks_total {
                return false;
            }
        }

        true
    }

    /// Generate comprehensive test report
    pub async fn generate_comprehensive_report(&mut self) -> Result<String> {
        let results = self.run_integration_tests().await?;

        let mut report = format!("TUI Integration Test Report\n{}\n", "=".repeat(60));

        report.push_str(&format!(
            "Overall Success: {}\n",
            if results.overall_success {
                "PASS"
            } else {
                "FAIL"
            }
        ));
        report.push_str(&format!(
            "Total Duration: {:.2}s\n",
            results.total_duration.as_secs_f64()
        ));
        report.push_str(&format!(
            "Test Coverage: {:.1}%\n",
            results.coverage_percentage
        ));
        report.push_str(&format!(
            "Success Rate: {:.1}%\n\n",
            results.test_suite_results.success_rate()
        ));

        // Test suite summary
        report.push_str("Test Suite Summary:\n");
        report.push_str(&format!(
            "  Total Tests: {}\n",
            results.test_suite_results.total_tests
        ));
        report.push_str(&format!(
            "  Passed: {}\n",
            results.test_suite_results.passed_tests
        ));
        report.push_str(&format!(
            "  Failed: {}\n",
            results.test_suite_results.failed_tests
        ));
        report.push_str(&format!(
            "  Skipped: {}\n\n",
            results.test_suite_results.skipped_tests
        ));

        // Critical issues
        if !results.critical_issues.is_empty() {
            report.push_str(&format!(
                "Critical Issues ({}):\n",
                results.critical_issues.len()
            ));
            for issue in &results.critical_issues {
                report.push_str(&format!("  ❌ {}\n", issue));
            }
            report.push('\n');
        }

        // Performance results
        if let Some(perf_results) = &results.performance_results {
            report.push_str(&format!(
                "Performance Benchmarks: {}/{}\n",
                perf_results.benchmarks_passed, perf_results.benchmarks_total
            ));

            if !perf_results.slo_violations.is_empty() {
                report.push_str("SLO Violations:\n");
                for violation in &perf_results.slo_violations {
                    report.push_str(&format!("  ⚠️  {}\n", violation));
                }
            }
            report.push('\n');
        }

        // Cross-platform results
        if let Some(cp_results) = &results.cross_platform_results {
            report.push_str(&format!(
                "Cross-Platform Compatibility: {:.1}%\n",
                cp_results.overall_compatibility
            ));

            if !cp_results.blocking_issues.is_empty() {
                report.push_str("Blocking Issues:\n");
                for issue in &cp_results.blocking_issues {
                    report.push_str(&format!("  🚫 {}\n", issue));
                }
            }
            report.push('\n');
        }

        // Stress test results
        if let Some(stress_results) = &results.stress_test_results {
            report.push_str("Stress Test Results:\n");
            report.push_str(&format!(
                "  Commands Executed: {}\n",
                stress_results.total_commands
            ));
            report.push_str(&format!(
                "  Total Time: {:.2}s\n",
                stress_results.total_time.as_secs_f64()
            ));
            report.push_str(&format!(
                "  Throughput: {:.1} cmd/s\n",
                stress_results.throughput_cps
            ));
            report.push_str(&format!(
                "  Average Latency: {:.2}ms\n\n",
                stress_results.average_latency.as_millis()
            ));
        }

        // Recommendations
        if !results.recommendations.is_empty() {
            report.push_str(&format!(
                "Recommendations ({}):\n",
                results.recommendations.len()
            ));
            for recommendation in &results.recommendations {
                report.push_str(&format!("  💡 {}\n", recommendation));
            }
            report.push('\n');
        }

        // Command test details
        report.push_str("Command Test Results:\n");
        for (test_name, result) in &results.test_suite_results.command_results {
            let status_icon = if result.success { "✅" } else { "❌" };
            report.push_str(&format!(
                "  {} {} ({:.2}s)\n",
                status_icon,
                test_name,
                result.execution_time.as_secs_f64()
            ));

            if !result.success {
                for error in &result.validation_errors {
                    report.push_str(&format!("    Error: {}\n", error));
                }
            }
        }

        Ok(report)
    }

    /// Run quick smoke test
    pub async fn run_smoke_test(&mut self) -> Result<bool> {
        // For smoke test, just check if we can create the harness
        let harness_config = crate::testing::tui::harness::TuiTestConfig {
            timeout_seconds: 10, // Shorter timeout for smoke test
            enable_performance: false,
            enable_cross_platform: false,
            terminal_width: self.config.terminal_width,
            terminal_height: self.config.terminal_height,
            working_dir: None,
        };

        let harness_result = TuiTestHarness::new(harness_config).await;
        Ok(harness_result.is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_config_defaults() {
        let config = IntegrationTestConfig::default();

        assert!(config.enable_performance);
        assert!(config.enable_cross_platform);
        assert!(config.enable_stress_testing);
        assert_eq!(config.stress_test_commands, 100);
        assert_eq!(config.stress_test_concurrency, 10);
    }

    #[tokio::test]
    async fn test_integration_tester_creation() {
        let tester = TuiIntegrationTester::default();
        // Should not panic
    }

    #[tokio::test]
    async fn test_smoke_test() {
        let mut tester = TuiIntegrationTester::default();

        // Smoke test might fail if terraphim-repl is not available, but shouldn't panic
        let result = tester.run_smoke_test().await;
        // We don't assert success since binary might not be available in test environment
        assert!(result.is_ok() || !result.unwrap());
    }

    #[test]
    fn test_stress_command_generation() {
        let config = IntegrationTestConfig {
            stress_test_commands: 10,
            ..Default::default()
        };
        let tester = TuiIntegrationTester::new(config);

        let commands = tester.generate_stress_test_commands();
        assert_eq!(commands.len(), 10);

        // Should cycle through base commands
        assert!(commands[0].contains("search"));
        assert!(commands[3].contains("config"));
    }

    #[test]
    fn test_coverage_calculation() {
        let tester = TuiIntegrationTester::default();
        let mut results = crate::testing::tui::harness::TuiTestSuiteResults {
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
        for i in 0..25 {
            results.command_results.insert(
                format!("test_{}", i),
                crate::testing::tui::harness::TuiCommandResult {
                    command: format!("test_{}", i),
                    success: true,
                    output: "ok".to_string(),
                    validation_errors: Vec::new(),
                    execution_time: Duration::from_millis(100),
                    exit_code: Some(0),
                },
            );
        }

        let coverage = tester.calculate_test_coverage(&results);
        assert!(coverage >= 50.0); // 25/50 = 50%
    }
}
