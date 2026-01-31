//! CI/CD Integration for Performance Benchmarking
//!
//! This module provides automated performance benchmarking integration with CI/CD pipelines,
//! including performance gates, regression detection, and automated reporting.

use anyhow::{Result, anyhow};
use chrono;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tokio::fs;

use crate::performance::benchmarking::{BenchmarkConfig, BenchmarkReport, PerformanceBenchmarker};

/// CI/CD performance gate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceGateConfig {
    /// Performance gates that must pass for CI to succeed
    pub gates: Vec<PerformanceGate>,
    /// Whether to fail CI on performance regressions
    pub fail_on_regression: bool,
    /// Regression threshold percentage (e.g., 5.0 = 5% degradation)
    pub regression_threshold_percent: f64,
    /// Whether to update baseline on successful runs
    pub update_baseline_on_success: bool,
    /// Report generation options
    pub reporting: ReportConfig,
}

/// Individual performance gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceGate {
    /// Gate name
    pub name: String,
    /// Metric to check (e.g., "search_api.avg_time")
    pub metric: String,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Threshold value
    pub threshold: f64,
    /// Gate severity (warning or blocking)
    pub severity: GateSeverity,
}

/// Comparison operators for gates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
}

/// Gate severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateSeverity {
    Warning,
    Blocking,
}

/// Report generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    /// Generate JSON report
    pub json: bool,
    /// Generate HTML report
    pub html: bool,
    /// Generate Markdown summary
    pub markdown: bool,
    /// Upload to external service (e.g., dashboard)
    pub upload_external: bool,
    /// External upload URL
    pub upload_url: Option<String>,
}

/// CI/CD performance benchmarking runner
pub struct CIPerformanceRunner {
    config: PerformanceGateConfig,
    baseline_path: String,
    reports_dir: String,
}

impl CIPerformanceRunner {
    /// Create a new CI performance runner
    pub fn new(config: PerformanceGateConfig, baseline_path: String, reports_dir: String) -> Self {
        Self {
            config,
            baseline_path,
            reports_dir,
        }
    }

    /// Run performance benchmarks in CI environment
    pub async fn run_ci_benchmarks(&self) -> Result<CIPerformanceResult> {
        log::info!("Starting CI performance benchmarking");

        // Load baseline if it exists
        let baseline = self.load_baseline().await.ok();

        // Create benchmarker
        let benchmark_config = BenchmarkConfig::default();
        let mut benchmarker = PerformanceBenchmarker::new(benchmark_config);

        // Load baseline for trend analysis
        if let Some(ref baseline_report) = baseline {
            benchmarker.load_baseline(baseline_report.clone());
        }

        // Run all benchmarks
        let report = benchmarker.run_all_benchmarks().await?;

        // Check performance gates
        let gate_results = self.check_performance_gates(&report)?;

        // Generate reports
        self.generate_reports(&report, &benchmarker).await?;

        // Determine overall result
        let passed = gate_results.blocking_failures.is_empty();

        let result = CIPerformanceResult {
            report,
            gate_results,
            passed,
            baseline_loaded: baseline.is_some(),
        };

        // Update baseline if successful and configured
        if passed && self.config.update_baseline_on_success {
            self.save_baseline(&result.report).await?;
        }

        Ok(result)
    }

    /// Load baseline report from file
    async fn load_baseline(&self) -> Result<BenchmarkReport> {
        let path = Path::new(&self.baseline_path);
        if !path.exists() {
            return Err(anyhow!(
                "Baseline file does not exist: {}",
                self.baseline_path
            ));
        }

        let content = fs::read_to_string(path).await?;
        let report: BenchmarkReport = serde_json::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse baseline: {}", e))?;

        log::info!("Loaded baseline from {}", self.baseline_path);
        Ok(report)
    }

    /// Save current report as new baseline
    async fn save_baseline(&self, report: &BenchmarkReport) -> Result<()> {
        let json = serde_json::to_string_pretty(report)?;
        fs::write(&self.baseline_path, json).await?;
        log::info!("Updated baseline at {}", self.baseline_path);
        Ok(())
    }

    /// Check performance gates against benchmark results
    fn check_performance_gates(&self, report: &BenchmarkReport) -> Result<GateResults> {
        let mut warnings = Vec::new();
        let mut blocking_failures = Vec::new();

        for gate in &self.config.gates {
            let gate_result = self.evaluate_gate(gate, report);

            match gate_result {
                Ok(passed) => {
                    if !passed {
                        let failure = GateFailure {
                            gate: gate.clone(),
                            message: format!(
                                "Gate '{}' failed: {} {} {}",
                                gate.name,
                                gate.metric,
                                self.operator_symbol(&gate.operator),
                                gate.threshold
                            ),
                        };

                        match gate.severity {
                            GateSeverity::Warning => warnings.push(failure),
                            GateSeverity::Blocking => blocking_failures.push(failure),
                        }
                    }
                }
                Err(e) => {
                    blocking_failures.push(GateFailure {
                        gate: gate.clone(),
                        message: format!("Gate '{}' evaluation error: {}", gate.name, e),
                    });
                }
            }
        }

        Ok(GateResults {
            warnings,
            blocking_failures,
        })
    }

    /// Evaluate a single performance gate
    fn evaluate_gate(&self, gate: &PerformanceGate, report: &BenchmarkReport) -> Result<bool> {
        // Parse metric path (e.g., "search_api.avg_time")
        let parts: Vec<&str> = gate.metric.split('.').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid metric format: {}", gate.metric));
        }

        let operation = parts[0];
        let metric_field = parts[1];

        let result = report
            .results
            .get(operation)
            .ok_or_else(|| anyhow!("Operation '{}' not found in results", operation))?;

        let actual_value = match metric_field {
            "avg_time_ms" => result.avg_time.as_millis() as f64,
            "max_time_ms" => result.max_time.as_millis() as f64,
            "min_time_ms" => result.min_time.as_millis() as f64,
            "ops_per_second" => result.ops_per_second,
            "success_rate" => result.success_rate * 100.0,
            "cpu_percent" => result.resource_usage.cpu_percent as f64,
            "memory_mb" => result.resource_usage.memory_bytes as f64 / (1024.0 * 1024.0),
            _ => return Err(anyhow!("Unknown metric field: {}", metric_field)),
        };

        let passed = match gate.operator {
            ComparisonOperator::LessThan => actual_value < gate.threshold,
            ComparisonOperator::LessThanOrEqual => actual_value <= gate.threshold,
            ComparisonOperator::GreaterThan => actual_value > gate.threshold,
            ComparisonOperator::GreaterThanOrEqual => actual_value >= gate.threshold,
            ComparisonOperator::Equal => (actual_value - gate.threshold).abs() < f64::EPSILON,
        };

        Ok(passed)
    }

    /// Get symbol for comparison operator
    fn operator_symbol(&self, op: &ComparisonOperator) -> &'static str {
        match op {
            ComparisonOperator::LessThan => "<",
            ComparisonOperator::LessThanOrEqual => "<=",
            ComparisonOperator::GreaterThan => ">",
            ComparisonOperator::GreaterThanOrEqual => ">=",
            ComparisonOperator::Equal => "==",
        }
    }

    /// Generate all configured reports
    async fn generate_reports(
        &self,
        report: &BenchmarkReport,
        benchmarker: &PerformanceBenchmarker,
    ) -> Result<()> {
        // Create reports directory if it doesn't exist
        fs::create_dir_all(&self.reports_dir).await?;

        if self.config.reporting.json {
            self.generate_json_report(report).await?;
        }

        if self.config.reporting.html {
            self.generate_html_report(report, benchmarker).await?;
        }

        if self.config.reporting.markdown {
            self.generate_markdown_report(report).await?;
        }

        if self.config.reporting.upload_external {
            if let Some(url) = &self.config.reporting.upload_url {
                self.upload_report(report, url).await?;
            }
        }

        Ok(())
    }

    /// Generate JSON report
    async fn generate_json_report(&self, report: &BenchmarkReport) -> Result<()> {
        let json_path = Path::new(&self.reports_dir).join("benchmark_report.json");
        let json = serde_json::to_string_pretty(report)
            .map_err(|e| anyhow::anyhow!("Failed to serialize report: {}", e))?;
        fs::write(json_path, json).await?;
        log::info!("Generated JSON report");
        Ok(())
    }

    /// Generate HTML report
    async fn generate_html_report(
        &self,
        report: &BenchmarkReport,
        benchmarker: &PerformanceBenchmarker,
    ) -> Result<()> {
        let html_path = Path::new(&self.reports_dir).join("benchmark_report.html");
        let html = benchmarker.export_html(report)?;
        fs::write(html_path, html).await?;
        log::info!("Generated HTML report");
        Ok(())
    }

    /// Generate Markdown summary report
    async fn generate_markdown_report(&self, report: &BenchmarkReport) -> Result<()> {
        let markdown_path = Path::new(&self.reports_dir).join("benchmark_summary.md");

        let mut content = format!(
            "# Performance Benchmark Report\n\n**Generated:** {}\n\n",
            report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        content.push_str(&format!(
            "## SLO Compliance: {:.1}%\n\n",
            report.slo_compliance.overall_compliance
        ));

        if report.slo_compliance.overall_compliance >= 95.0 {
            content.push_str("✅ **PASS**: Performance requirements met\n\n");
        } else {
            content.push_str("❌ **FAIL**: Performance requirements not met\n\n");
        }

        // System information
        content.push_str("## System Information\n\n");
        content.push_str(&format!(
            "- **OS:** {} {}\n",
            report.system_info.os, report.system_info.os_version
        ));
        content.push_str(&format!(
            "- **CPU:** {} ({} cores)\n",
            report.system_info.cpu_model, report.system_info.cpu_cores
        ));
        content.push_str(&format!(
            "- **Memory:** {} MB total\n",
            report.system_info.total_memory_mb
        ));
        content.push_str(&format!(
            "- **Terraphim Version:** {}\n\n",
            report.system_info.terraphim_version
        ));

        // Benchmark results
        content.push_str("## Benchmark Results\n\n");
        content.push_str("| Operation | Avg Time | Ops/sec | Success Rate | CPU % | Memory MB |\n");
        content
            .push_str("|-----------|----------|---------|--------------|--------|-----------|\n");

        for (operation, result) in &report.results {
            content.push_str(&format!(
                "| {} | {:.1}ms | {:.1} | {:.1}% | {:.1}% | {:.1} |\n",
                operation,
                result.avg_time.as_millis(),
                result.ops_per_second,
                result.success_rate * 100.0,
                result.resource_usage.cpu_percent,
                result.resource_usage.memory_bytes as f64 / (1024.0 * 1024.0)
            ));
        }

        content.push_str("\n");

        // SLO violations
        if !report.slo_compliance.violations.is_empty()
            || !report.slo_compliance.critical_violations.is_empty()
        {
            content.push_str("## SLO Violations\n\n");

            for violation in &report.slo_compliance.critical_violations {
                content.push_str(&format!(
                    "🚨 **CRITICAL:** {} - {} (threshold: {})\n",
                    violation.metric, violation.actual_value, violation.threshold_value
                ));
            }

            for violation in &report.slo_compliance.violations {
                content.push_str(&format!(
                    "⚠️ **WARNING:** {} - {} (threshold: {})\n",
                    violation.metric, violation.actual_value, violation.threshold_value
                ));
            }

            content.push_str("\n");
        }

        // Performance trends
        if let Some(trends) = &report.trends {
            content.push_str("## Performance Trends\n\n");

            if !trends.improvements.is_empty() {
                content.push_str("### Improvements\n\n");
                for (operation, change) in &trends.improvements {
                    content.push_str(&format!("✅ **{}:** {:.1}% faster\n", operation, change));
                }
                content.push_str("\n");
            }

            if !trends.regressions.is_empty() {
                content.push_str("### Regressions\n\n");
                for (operation, change) in &trends.regressions {
                    content.push_str(&format!(
                        "❌ **{}:** {:.1}% slower\n",
                        operation,
                        change.abs()
                    ));
                }
                content.push_str("\n");
            }

            if !trends.new_operations.is_empty() {
                content.push_str("### New Operations\n\n");
                for operation in &trends.new_operations {
                    content.push_str(&format!("🆕 **{}:** New benchmark added\n", operation));
                }
                content.push_str("\n");
            }
        }

        fs::write(markdown_path, content).await?;
        log::info!("Generated Markdown summary report");
        Ok(())
    }

    /// Upload report to external service
    async fn upload_report(&self, report: &BenchmarkReport, url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let json = serde_json::to_string(report)?;

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .body(json)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to upload report: HTTP {}",
                response.status()
            ));
        }

        log::info!("Uploaded report to external service");
        Ok(())
    }
}

/// Results from CI performance benchmarking
#[derive(Debug)]
pub struct CIPerformanceResult {
    /// Benchmark report
    pub report: BenchmarkReport,
    /// Performance gate results
    pub gate_results: GateResults,
    /// Whether all gates passed
    pub passed: bool,
    /// Whether baseline was loaded
    pub baseline_loaded: bool,
}

/// Performance gate evaluation results
#[derive(Debug)]
pub struct GateResults {
    /// Warning-level gate failures
    pub warnings: Vec<GateFailure>,
    /// Blocking gate failures
    pub blocking_failures: Vec<GateFailure>,
}

/// Individual gate failure
#[derive(Debug, Clone)]
pub struct GateFailure {
    /// The gate that failed
    pub gate: PerformanceGate,
    /// Failure message
    pub message: String,
}

/// Default CI performance gate configuration
impl Default for PerformanceGateConfig {
    fn default() -> Self {
        Self {
            gates: vec![
                PerformanceGate {
                    name: "API Response Time".to_string(),
                    metric: "search_api.avg_time_ms".to_string(),
                    operator: ComparisonOperator::LessThan,
                    threshold: 1000.0, // 1 second
                    severity: GateSeverity::Blocking,
                },
                PerformanceGate {
                    name: "CPU Usage Idle".to_string(),
                    metric: "resource_monitoring_idle.cpu_percent".to_string(),
                    operator: ComparisonOperator::LessThan,
                    threshold: 5.0,
                    severity: GateSeverity::Warning,
                },
                PerformanceGate {
                    name: "Memory Usage".to_string(),
                    metric: "resource_monitoring_load.memory_mb".to_string(),
                    operator: ComparisonOperator::LessThan,
                    threshold: 1024.0, // 1GB
                    severity: GateSeverity::Blocking,
                },
                PerformanceGate {
                    name: "Search Success Rate".to_string(),
                    metric: "search_api.success_rate".to_string(),
                    operator: ComparisonOperator::GreaterThanOrEqual,
                    threshold: 99.0,
                    severity: GateSeverity::Blocking,
                },
            ],
            fail_on_regression: true,
            regression_threshold_percent: 5.0,
            update_baseline_on_success: true,
            reporting: ReportConfig {
                json: true,
                html: true,
                markdown: true,
                upload_external: false,
                upload_url: None,
            },
        }
    }
}

/// Command-line interface for CI performance benchmarking
pub struct CLIInterface {
    runner: CIPerformanceRunner,
}

impl CLIInterface {
    /// Create CLI interface
    pub fn new(runner: CIPerformanceRunner) -> Self {
        Self { runner }
    }

    /// Run performance benchmarks from command line
    pub async fn run(&self) -> Result<i32> {
        match self.runner.run_ci_benchmarks().await {
            Ok(result) => {
                // Print summary to stdout
                println!("Performance benchmarking completed");
                println!(
                    "SLO Compliance: {:.1}%",
                    result.report.slo_compliance.overall_compliance
                );
                println!(
                    "Blocking failures: {}",
                    result.gate_results.blocking_failures.len()
                );
                println!("Warnings: {}", result.gate_results.warnings.len());

                if !result.passed {
                    println!("❌ Performance gates failed - CI should fail");

                    // Print blocking failures
                    for failure in &result.gate_results.blocking_failures {
                        println!("🚫 {}", failure.message);
                    }

                    return Ok(1); // Non-zero exit code for CI failure
                } else {
                    println!("✅ All performance gates passed");
                    return Ok(0); // Success exit code
                }
            }
            Err(e) => {
                eprintln!("Error running performance benchmarks: {}", e);
                Ok(1)
            }
        }
    }
}

/// GitHub Actions integration helper
pub struct GitHubActions;

impl GitHubActions {
    /// Set GitHub Actions output
    pub fn set_output(name: &str, value: &str) {
        println!("::set-output name={}::{}", name, value);
    }

    /// Log a warning message
    pub fn warning(message: &str) {
        println!("::warning ::{}", message);
    }

    /// Log an error message
    pub fn error(message: &str) {
        println!("::error ::{}", message);
    }

    /// Create a job summary
    pub async fn write_summary(content: &str) -> Result<()> {
        if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
            fs::write(path, content).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_evaluation() {
        let gate = PerformanceGate {
            name: "Test Gate".to_string(),
            metric: "test_operation.avg_time_ms".to_string(),
            operator: ComparisonOperator::LessThan,
            threshold: 100.0,
            severity: GateSeverity::Blocking,
        };

        // Create mock report
        let mut results = std::collections::HashMap::new();
        results.insert(
            "test_operation".to_string(),
            crate::performance::benchmarking::BenchmarkResult {
                operation: "test_operation".to_string(),
                total_time: std::time::Duration::from_millis(1000),
                avg_time: std::time::Duration::from_millis(50),
                min_time: std::time::Duration::from_millis(20),
                max_time: std::time::Duration::from_millis(100),
                ops_per_second: 20.0,
                success_rate: 1.0,
                error_count: 0,
                resource_usage: crate::performance::benchmarking::ResourceUsage {
                    cpu_percent: 10.0,
                    memory_bytes: 100 * 1024 * 1024,
                    virtual_memory_bytes: 200 * 1024 * 1024,
                    disk_read_bytes: 0,
                    disk_write_bytes: 0,
                    network_rx_bytes: 0,
                    network_tx_bytes: 0,
                    thread_count: 4,
                },
            },
        );

        let report = BenchmarkReport {
            timestamp: chrono::Utc::now(),
            config: BenchmarkConfig::default(),
            results,
            slo_compliance: crate::performance::benchmarking::SLOCompliance {
                overall_compliance: 100.0,
                violations: vec![],
                critical_violations: vec![],
            },
            system_info: crate::performance::benchmarking::SystemInfo {
                os: "Linux".to_string(),
                os_version: "5.4.0".to_string(),
                cpu_model: "Intel i7".to_string(),
                cpu_cores: 8,
                total_memory_mb: 16384,
                available_memory_mb: 8192,
                rust_version: "1.70.0".to_string(),
                terraphim_version: "1.0.0".to_string(),
            },
            trends: None,
        };

        let runner = CIPerformanceRunner::new(
            PerformanceGateConfig::default(),
            "baseline.json".to_string(),
            "reports".to_string(),
        );

        let result = runner.evaluate_gate(&gate, &report);
        assert!(result.unwrap()); // 50ms < 100ms, so should pass
    }
}
