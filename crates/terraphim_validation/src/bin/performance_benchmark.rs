//! Command-line tool for running performance benchmarks
//!
//! This binary provides a CLI interface for running comprehensive performance
//! benchmarks on terraphim-ai components, with CI/CD integration.

#![allow(unused)]
#![allow(dead_code)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use terraphim_validation::performance::benchmarking::{
    BenchmarkConfig, BenchmarkReport, PerformanceBenchmarker,
};
use terraphim_validation::performance::ci_integration::{
    CIPerformanceRunner, CLIInterface, PerformanceGateConfig,
};

/// Terraphim AI Performance Benchmarking Tool
#[derive(Parser)]
#[command(name = "terraphim-bench")]
#[command(about = "Performance benchmarking suite for Terraphim AI")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all performance benchmarks
    Run {
        /// Output directory for reports
        #[arg(short, long, default_value = "benchmark-results")]
        output_dir: PathBuf,

        /// Baseline file to compare against
        #[arg(short, long)]
        baseline: Option<PathBuf>,

        /// Number of benchmark iterations
        #[arg(short, long, default_value = "1000")]
        iterations: u32,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run CI-integrated performance benchmarks with gates
    Ci {
        /// Performance gates configuration file
        #[arg(short, long)]
        config: PathBuf,

        /// Baseline file path
        #[arg(short, long, default_value = "baseline.json")]
        baseline: PathBuf,

        /// Reports output directory
        #[arg(short, long, default_value = "reports")]
        reports_dir: PathBuf,

        /// Update baseline on successful run
        #[arg(long)]
        update_baseline: bool,
    },

    /// Compare benchmark results against baseline
    Compare {
        /// Current benchmark results file
        current: PathBuf,

        /// Baseline benchmark results file
        baseline: PathBuf,

        /// Output format (json, markdown, console)
        #[arg(short, long, default_value = "console")]
        format: String,
    },

    /// Generate performance report from results
    Report {
        /// Benchmark results file
        input: PathBuf,

        /// Output format (html, json, markdown)
        #[arg(short, long, default_value = "html")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate performance against SLO requirements
    Validate {
        /// Benchmark results file
        input: PathBuf,

        /// SLO configuration file
        #[arg(short, long)]
        slo_config: Option<PathBuf>,

        /// Exit with error code on SLO violations
        #[arg(long)]
        strict: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    env_logger::init();

    match args.command {
        Commands::Run {
            output_dir,
            baseline,
            iterations,
            verbose,
        } => run_benchmarks(output_dir, baseline, iterations, verbose).await,
        Commands::Ci {
            config,
            baseline,
            reports_dir,
            update_baseline,
        } => run_ci_benchmarks(config, baseline, reports_dir, update_baseline).await,
        Commands::Compare {
            current,
            baseline,
            format,
        } => compare_results(current, baseline, format).await,
        Commands::Report {
            input,
            format,
            output,
        } => generate_report(input, format, output).await,
        Commands::Validate {
            input,
            slo_config,
            strict,
        } => validate_performance(input, slo_config, strict).await,
    }
}

/// Run standalone performance benchmarks
async fn run_benchmarks(
    output_dir: PathBuf,
    baseline: Option<PathBuf>,
    iterations: u32,
    verbose: bool,
) -> Result<()> {
    println!("🚀 Starting Terraphim AI Performance Benchmarks");
    println!("📊 Iterations: {}", iterations);
    println!("📁 Output directory: {}", output_dir.display());

    // Create output directory
    tokio::fs::create_dir_all(&output_dir).await?;

    // Load baseline if provided
    let mut benchmark_config = BenchmarkConfig {
        iterations,
        ..BenchmarkConfig::default()
    };

    let mut benchmarker = PerformanceBenchmarker::new(benchmark_config);

    if let Some(baseline_path) = baseline {
        if baseline_path.exists() {
            println!("📈 Loading baseline from: {}", baseline_path.display());
            match load_optional_baseline_report(&baseline_path).await? {
                Some(baseline_report) => benchmarker.load_baseline(baseline_report),
                None => {
                    println!(
                        "⚠️  Ignoring malformed baseline file: {}",
                        baseline_path.display()
                    );
                }
            }
        } else {
            println!("⚠️  Baseline file not found: {}", baseline_path.display());
        }
    }

    // Run benchmarks
    println!("⏱️  Running benchmarks...");
    let report = benchmarker.run_all_benchmarks().await?;

    // Save results
    let json_path = output_dir.join("benchmark_results.json");
    let json = benchmarker.export_json(&report)?;
    tokio::fs::write(&json_path, json).await?;

    let html_path = output_dir.join("benchmark_report.html");
    let html = benchmarker.export_html(&report)?;
    tokio::fs::write(&html_path, html).await?;

    // Print summary
    println!("✅ Benchmarks completed!");
    println!(
        "📈 SLO Compliance: {:.1}%",
        report.slo_compliance.overall_compliance
    );
    println!("📄 Results saved to:");
    println!("   JSON: {}", json_path.display());
    println!("   HTML: {}", html_path.display());

    if verbose {
        println!("\n📊 Benchmark Results Summary:");
        for (operation, result) in &report.results {
            println!(
                "  {}: {:.1}ms avg, {:.1} ops/sec, {:.1}% success",
                operation,
                result.avg_time.as_millis(),
                result.ops_per_second,
                result.success_rate * 100.0
            );
        }
    }

    Ok(())
}

async fn load_optional_baseline_report(path: &Path) -> Result<Option<BenchmarkReport>> {
    let baseline_content = tokio::fs::read_to_string(path).await?;
    parse_optional_baseline_report(&baseline_content)
}

fn parse_optional_baseline_report(content: &str) -> Result<Option<BenchmarkReport>> {
    match serde_json::from_str(content) {
        Ok(report) => Ok(Some(report)),
        Err(error) => {
            log::warn!("Ignoring malformed benchmark baseline: {}", error);
            Ok(None)
        }
    }
}

/// Run CI-integrated benchmarks with performance gates
async fn run_ci_benchmarks(
    config_path: PathBuf,
    baseline: PathBuf,
    reports_dir: PathBuf,
    update_baseline: bool,
) -> Result<()> {
    println!("🔧 Running CI Performance Benchmarks");
    println!("⚙️  Config: {}", config_path.display());
    println!("📈 Baseline: {}", baseline.display());
    println!("📁 Reports: {}", reports_dir.display());

    // Load configuration
    let config_content = tokio::fs::read_to_string(&config_path).await?;
    let mut gate_config: PerformanceGateConfig = serde_json::from_str(&config_content)?;

    // Override update baseline setting
    gate_config.update_baseline_on_success = update_baseline;

    // Create CI runner
    let runner = CIPerformanceRunner::new(
        gate_config,
        baseline.to_string_lossy().to_string(),
        reports_dir.to_string_lossy().to_string(),
    );
    let cli = CLIInterface::new(runner);

    // Run benchmarks
    let exit_code = cli.run().await?;

    std::process::exit(exit_code);
}

/// Compare benchmark results against baseline
async fn compare_results(current: PathBuf, baseline: PathBuf, format: String) -> Result<()> {
    println!("🔍 Comparing benchmark results");
    println!("📊 Current: {}", current.display());
    println!("📈 Baseline: {}", baseline.display());

    // Load results
    let current_content = tokio::fs::read_to_string(&current).await?;
    let current_report: terraphim_validation::performance::benchmarking::BenchmarkReport =
        serde_json::from_str(&current_content)?;

    let baseline_content = tokio::fs::read_to_string(&baseline).await?;
    let baseline_report: terraphim_validation::performance::benchmarking::BenchmarkReport =
        serde_json::from_str(&baseline_content)?;

    // Compare results
    println!("\n📊 Performance Comparison:");

    for (operation, current_result) in &current_report.results {
        if let Some(baseline_result) = baseline_report.results.get(operation) {
            let current_avg = current_result.avg_time.as_secs_f64();
            let baseline_avg = baseline_result.avg_time.as_secs_f64();

            if current_avg > 0.0 && baseline_avg > 0.0 {
                let change_percent = ((baseline_avg - current_avg) / baseline_avg) * 100.0;
                let change_symbol = if change_percent > 0.0 { "📈" } else { "📉" };

                println!(
                    "  {}: {:.1}ms → {:.1}ms ({}{:.1}%)",
                    operation,
                    baseline_avg * 1000.0,
                    current_avg * 1000.0,
                    change_symbol,
                    change_percent.abs()
                );
            }
        } else {
            println!("  {}: 🆕 New operation", operation);
        }
    }

    Ok(())
}

/// Generate performance report from results
async fn generate_report(input: PathBuf, format: String, output: Option<PathBuf>) -> Result<()> {
    println!("📄 Generating performance report");
    println!("📊 Input: {}", input.display());
    println!("📋 Format: {}", format);

    // Load results
    let content = tokio::fs::read_to_string(&input).await?;
    let report: terraphim_validation::performance::benchmarking::BenchmarkReport =
        serde_json::from_str(&content)?;

    let benchmarker = PerformanceBenchmarker::new(BenchmarkConfig::default());

    // Generate report
    let report_content = match format.as_str() {
        "html" => benchmarker.export_html(&report)?,
        "json" => benchmarker.export_json(&report)?,
        "markdown" => generate_markdown_report(&report)?,
        _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
    };

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let extension = match format.as_str() {
            "html" => "html",
            "json" => "json",
            "markdown" => "md",
            _ => "txt",
        };
        input.with_extension(extension)
    });

    // Write report
    tokio::fs::write(&output_path, report_content).await?;
    println!("✅ Report saved to: {}", output_path.display());

    Ok(())
}

/// Generate markdown report (simplified version)
fn generate_markdown_report(
    report: &terraphim_validation::performance::benchmarking::BenchmarkReport,
) -> Result<String> {
    let mut content = format!(
        "# Performance Benchmark Report\n\n**Generated:** {}\n\n",
        report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );

    content.push_str(&format!(
        "## SLO Compliance: {:.1}%\n\n",
        report.slo_compliance.overall_compliance
    ));

    content.push_str("## Benchmark Results\n\n");
    content.push_str("| Operation | Avg Time | Ops/sec | Success Rate |\n");
    content.push_str("|-----------|----------|---------|--------------|\n");

    for (operation, result) in &report.results {
        content.push_str(&format!(
            "| {} | {:.1}ms | {:.1} | {:.1}% |\n",
            operation,
            result.avg_time.as_millis(),
            result.ops_per_second,
            result.success_rate * 100.0
        ));
    }

    Ok(content)
}

/// Validate performance against SLO requirements
async fn validate_performance(
    input: PathBuf,
    slo_config: Option<PathBuf>,
    strict: bool,
) -> Result<()> {
    println!("✅ Validating performance against SLOs");
    println!("📊 Input: {}", input.display());

    // Load results
    let content = tokio::fs::read_to_string(&input).await?;
    let report: terraphim_validation::performance::benchmarking::BenchmarkReport =
        serde_json::from_str(&content)?;

    // Load SLO config if provided
    let slo_threshold = if let Some(config_path) = slo_config {
        let config_content = tokio::fs::read_to_string(&config_path).await?;
        let config: serde_json::Value = serde_json::from_str(&config_content)?;
        config["overall_compliance_threshold"]
            .as_f64()
            .unwrap_or(95.0)
    } else {
        95.0
    };

    println!("🎯 SLO Threshold: {:.1}%", slo_threshold);
    println!(
        "📊 Actual Compliance: {:.1}%",
        report.slo_compliance.overall_compliance
    );

    // Check compliance
    if report.slo_compliance.overall_compliance >= slo_threshold {
        println!("✅ Performance requirements met!");
        Ok(())
    } else {
        println!("❌ Performance requirements not met!");

        // Print violations
        for violation in &report.slo_compliance.violations {
            println!(
                "⚠️  {}: {} (threshold: {})",
                violation.metric, violation.actual_value, violation.threshold_value
            );
        }

        for violation in &report.slo_compliance.critical_violations {
            println!(
                "🚨 {}: {} (threshold: {})",
                violation.metric, violation.actual_value, violation.threshold_value
            );
        }

        if strict {
            std::process::exit(1);
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_validation::performance::benchmarking::{SLOCompliance, SystemInfo};

    fn empty_report() -> BenchmarkReport {
        BenchmarkReport {
            timestamp: chrono::Utc::now(),
            config: BenchmarkConfig::default(),
            results: std::collections::HashMap::new(),
            slo_compliance: SLOCompliance {
                overall_compliance: 100.0,
                violations: vec![],
                critical_violations: vec![],
            },
            system_info: SystemInfo {
                os: "unknown".to_string(),
                os_version: "unknown".to_string(),
                cpu_model: "unknown".to_string(),
                cpu_cores: 0,
                total_memory_mb: 0,
                available_memory_mb: 0,
                rust_version: "unknown".to_string(),
                terraphim_version: "unknown".to_string(),
            },
            trends: None,
        }
    }

    #[test]
    fn parse_optional_baseline_report_accepts_valid_report() {
        let json = serde_json::to_string(&empty_report()).unwrap();

        let parsed = parse_optional_baseline_report(&json).unwrap();

        assert!(parsed.is_some());
        assert_eq!(
            parsed.unwrap().config.iterations,
            BenchmarkConfig::default().iterations
        );
    }

    #[test]
    fn parse_optional_baseline_report_ignores_legacy_placeholder() {
        let parsed =
            parse_optional_baseline_report(r#"{"timestamp":"2024-01-01T00:00:00Z","results":{}}"#)
                .unwrap();

        assert!(parsed.is_none());
    }
}
