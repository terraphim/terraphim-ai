//! Desktop UI Test Orchestrator
//!
//! High-level orchestrator for running comprehensive desktop UI tests
//! including all test categories and result aggregation.

use crate::ValidationStatus;
use crate::testing::desktop_ui::cross_platform::{TestValidationResult, TestValidationStatus};
use crate::testing::desktop_ui::*;
use crate::testing::{Result, ValidationResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// Helper to convert ValidationResult to UITestResult
fn validation_to_ui_result(vr: ValidationResult) -> UITestResult {
    UITestResult {
        name: vr.name,
        status: match vr.status {
            ValidationStatus::Passed => UITestStatus::Pass,
            ValidationStatus::Failed => UITestStatus::Fail,
            ValidationStatus::Skipped => UITestStatus::Skip,
            _ => UITestStatus::Error,
        },
        message: None,
        details: None,
        duration_ms: Some(vr.duration_ms),
    }
}

/// Helper to convert TestValidationResult to UITestResult
fn test_validation_to_ui_result(tvr: TestValidationResult) -> UITestResult {
    UITestResult {
        name: tvr.name,
        status: match tvr.status {
            TestValidationStatus::Pass => UITestStatus::Pass,
            TestValidationStatus::Fail => UITestStatus::Fail,
            TestValidationStatus::Skip => UITestStatus::Skip,
            TestValidationStatus::Error => UITestStatus::Error,
        },
        message: tvr.message,
        details: tvr.details,
        duration_ms: None,
    }
}

/// Master configuration for desktop UI testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopUITestSuiteConfig {
    pub harness: DesktopUITestConfig,
    pub components: ComponentTestConfig,
    pub auto_updater: AutoUpdaterTestConfig,
    pub cross_platform: CrossPlatformTestConfig,
    pub performance: PerformanceTestConfig,
    pub accessibility: AccessibilityTestConfig,
    pub integration: IntegrationTestConfig,
    pub output: TestOutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestOutputConfig {
    pub results_dir: PathBuf,
    pub screenshots_dir: PathBuf,
    pub reports_dir: PathBuf,
    pub save_screenshots: bool,
    pub generate_html_report: bool,
}

/// Desktop UI Test Orchestrator
pub struct DesktopUITestOrchestrator {
    config: DesktopUITestSuiteConfig,
    harness: Option<DesktopUITestHarness>,
}

impl DesktopUITestOrchestrator {
    pub fn new(config: DesktopUITestSuiteConfig) -> Self {
        Self {
            config,
            harness: None,
        }
    }

    /// Run complete desktop UI test suite
    pub async fn run_full_test_suite(&mut self) -> anyhow::Result<TestSuiteResults> {
        let start_time = Instant::now();
        let mut all_results = Vec::new();

        // Initialize test harness
        self.harness = Some(DesktopUITestHarness::new(self.config.harness.clone()));

        if let Some(harness) = &mut self.harness {
            harness.start().await?;
        }

        // Run all test categories
        println!("Starting Desktop UI Test Suite...");

        // 1. Component Tests
        println!("Running component tests...");
        let component_results = self.run_component_tests().await?;
        all_results.extend(component_results);

        // 2. Cross-platform Tests
        println!("Running cross-platform tests...");
        let cross_platform_results = self.run_cross_platform_tests().await?;
        all_results.extend(cross_platform_results);

        // 3. Auto-updater Tests
        println!("Running auto-updater tests...");
        let updater_results = self.run_auto_updater_tests().await?;
        all_results.extend(updater_results);

        // 4. Performance Tests
        println!("Running performance tests...");
        let performance_results = self.run_performance_tests().await?;
        all_results.extend(performance_results);

        // 5. Accessibility Tests
        println!("Running accessibility tests...");
        let accessibility_results = self.run_accessibility_tests().await?;
        all_results.extend(accessibility_results);

        // 6. Integration Tests
        println!("Running integration tests...");
        let integration_results = self.run_integration_tests().await?;
        all_results.extend(integration_results);

        // Cleanup
        if let Some(harness) = &mut self.harness {
            harness.stop().await?;
        }

        let duration = start_time.elapsed();

        // Generate final results
        let aggregated = utils::ResultUtils::aggregate_ui_results(all_results.clone());

        let results = TestSuiteResults {
            test_results: all_results,
            aggregated,
            duration,
            timestamp: chrono::Utc::now(),
        };

        // Save results if configured
        self.save_results(&results).await?;

        Ok(results)
    }

    /// Run component tests
    async fn run_component_tests(&self) -> Result<Vec<UITestResult>> {
        let tester = UIComponentTester::new(self.config.components.clone());

        let mut results = Vec::new();

        for vr in tester.test_system_tray().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_main_window().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_search_interface().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_configuration_panel().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_knowledge_graph().await? {
            results.push(validation_to_ui_result(vr));
        }

        Ok(results)
    }

    /// Run cross-platform tests
    async fn run_cross_platform_tests(&self) -> Result<Vec<UITestResult>> {
        let tester = CrossPlatformUITester::new(self.config.cross_platform.clone());

        let mut results = Vec::new();

        for tvr in tester.test_macos_ui().await? {
            results.push(test_validation_to_ui_result(tvr));
        }
        for tvr in tester.test_windows_ui().await? {
            results.push(test_validation_to_ui_result(tvr));
        }
        for tvr in tester.test_linux_ui().await? {
            results.push(test_validation_to_ui_result(tvr));
        }

        Ok(results)
    }

    /// Run auto-updater tests
    async fn run_auto_updater_tests(&self) -> Result<Vec<UITestResult>> {
        let tester = AutoUpdaterTester::new(self.config.auto_updater.clone());

        let mut results = Vec::new();

        for vr in tester.test_update_detection().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_download_process().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_installation_process().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_rollback_scenarios().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_post_update_verification().await? {
            results.push(validation_to_ui_result(vr));
        }

        Ok(results)
    }

    /// Run performance tests
    async fn run_performance_tests(&self) -> Result<Vec<UITestResult>> {
        let tester = PerformanceTester::new(self.config.performance.clone());

        let mut results = Vec::new();

        // Test startup performance
        let perf_results = tester.test_startup_performance().await?;
        results.push(UITestResult {
            name: "Startup Performance".to_string(),
            status: if perf_results.startup_time <= self.config.performance.startup.max_startup_time
            {
                UITestStatus::Pass
            } else {
                UITestStatus::Fail
            },
            message: Some(format!(
                "Startup time: {}ms",
                perf_results.startup_time.as_millis()
            )),
            details: None,
            duration_ms: Some(perf_results.startup_time.as_millis() as u64),
        });

        // Test memory usage
        for vr in tester.test_memory_usage().await? {
            results.push(validation_to_ui_result(vr));
        }

        // Test UI responsiveness
        for vr in tester.test_ui_responsiveness().await? {
            results.push(validation_to_ui_result(vr));
        }

        Ok(results)
    }

    /// Run accessibility tests
    async fn run_accessibility_tests(&self) -> Result<Vec<UITestResult>> {
        let tester = AccessibilityTester::new(self.config.accessibility.clone());

        let mut results = Vec::new();

        for vr in tester.test_keyboard_navigation().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_screen_reader_compatibility().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_color_contrast().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_wcag_compliance().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_semantic_markup().await? {
            results.push(validation_to_ui_result(vr));
        }

        Ok(results)
    }

    /// Run integration tests
    async fn run_integration_tests(&self) -> Result<Vec<UITestResult>> {
        let tester = IntegrationTester::new(self.config.integration.clone());

        let mut results = Vec::new();

        for vr in tester.test_server_communication().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_file_operations().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_external_links().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_keyboard_shortcuts().await? {
            results.push(validation_to_ui_result(vr));
        }
        for vr in tester.test_network_scenarios().await? {
            results.push(validation_to_ui_result(vr));
        }

        Ok(results)
    }

    /// Save test results to files
    async fn save_results(&self, results: &TestSuiteResults) -> anyhow::Result<()> {
        use utils::TestDataUtils;

        // Save JSON results
        let results_path = self
            .config
            .output
            .results_dir
            .join("desktop_ui_test_results.json");
        TestDataUtils::save_test_data(&results_path, results)?;

        // Generate summary report
        let summary_path = self
            .config
            .output
            .reports_dir
            .join("desktop_ui_test_summary.txt");
        let summary = utils::ResultUtils::generate_ui_summary(&results.test_results);
        std::fs::write(summary_path, summary)?;

        // Generate HTML report if configured
        if self.config.output.generate_html_report {
            self.generate_html_report(results).await?;
        }

        Ok(())
    }

    /// Generate HTML test report
    async fn generate_html_report(&self, results: &TestSuiteResults) -> Result<()> {
        let html_path = self
            .config
            .output
            .reports_dir
            .join("desktop_ui_test_report.html");

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Terraphim AI Desktop UI Test Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .summary {{ background: #f0f0f0; padding: 20px; border-radius: 5px; margin-bottom: 20px; }}
        .passed {{ color: #28a745; }}
        .failed {{ color: #dc3545; }}
        .skipped {{ color: #ffc107; }}
        .test-result {{ margin: 10px 0; padding: 10px; border-left: 4px solid; }}
        .test-result.pass {{ border-left-color: #28a745; background: #f8fff8; }}
        .test-result.fail {{ border-left-color: #dc3545; background: #fff8f8; }}
        .test-result.skip {{ border-left-color: #ffc107; background: #fffef8; }}
    </style>
</head>
<body>
    <h1>Terraphim AI Desktop UI Test Report</h1>
    <div class="summary">
        <h2>Test Summary</h2>
        <p><strong>Total Tests:</strong> {}</p>
        <p><strong>Passed:</strong> <span class="passed">{}</span></p>
        <p><strong>Failed:</strong> <span class="failed">{}</span></p>
        <p><strong>Skipped:</strong> <span class="skipped">{}</span></p>
        <p><strong>Success Rate:</strong> {:.1}%</p>
        <p><strong>Duration:</strong> {:.2}s</p>
        <p><strong>Timestamp:</strong> {}</p>
    </div>

    <h2>Test Results</h2>
    {}
</body>
</html>"#,
            results.aggregated.total,
            results.aggregated.passed,
            results.aggregated.failed,
            results.aggregated.skipped,
            results.aggregated.success_rate,
            results.duration.as_secs_f64(),
            results.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.generate_html_test_results(&results.test_results)
        );

        std::fs::write(html_path, html)?;
        Ok(())
    }

    fn generate_html_test_results(&self, test_results: &[UITestResult]) -> String {
        test_results
            .iter()
            .map(|result| {
                let css_class = match result.status {
                    UITestStatus::Pass => "pass",
                    UITestStatus::Fail => "fail",
                    UITestStatus::Skip => "skip",
                    UITestStatus::Error => "fail",
                };

                format!(
                    r#"<div class="test-result {}">
                        <h3>{}</h3>
                        <p><strong>Status:</strong> {}</p>
                        {}
                        {}
                    </div>"#,
                    css_class,
                    result.name,
                    format!("{:?}", result.status),
                    result
                        .message
                        .as_ref()
                        .map(|msg| format!("<p><strong>Message:</strong> {}</p>", msg))
                        .unwrap_or_default(),
                    result
                        .details
                        .as_ref()
                        .map(|details| format!("<p><strong>Details:</strong> {}</p>", details))
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Test suite results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteResults {
    pub test_results: Vec<UITestResult>,
    pub aggregated: utils::AggregatedResults,
    pub duration: std::time::Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for DesktopUITestSuiteConfig {
    fn default() -> Self {
        Self {
            harness: DesktopUITestConfig::default(),
            components: ComponentTestConfig::default(),
            auto_updater: AutoUpdaterTestConfig::default(),
            cross_platform: CrossPlatformTestConfig::default(),
            performance: PerformanceTestConfig::default(),
            accessibility: AccessibilityTestConfig::default(),
            integration: IntegrationTestConfig::default(),
            output: TestOutputConfig {
                results_dir: PathBuf::from("./test-results/desktop-ui"),
                screenshots_dir: PathBuf::from("./test-results/screenshots"),
                reports_dir: PathBuf::from("./test-results/reports"),
                save_screenshots: true,
                generate_html_report: true,
            },
        }
    }
}
