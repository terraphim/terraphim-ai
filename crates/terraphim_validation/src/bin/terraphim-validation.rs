//! Command line interface for Terraphim validation system

#![allow(unused)]
#![allow(dead_code)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use terraphim_validation::orchestrator::{ValidationConfig, ValidationOrchestrator};
use terraphim_validation::reporting::{ReportFormat, ReportGenerator, ValidationReport};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "terraphim-validation",
    about = "Terraphim AI Release Validation System",
    long_about = "Comprehensive validation system for Terraphim AI releases, including download testing, installation validation, functional verification, and security scanning across multiple platforms and package formats."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<String>,

    /// Output directory for reports
    #[arg(short, long, default_value = "target/validation-reports")]
    pub output_dir: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Validate a complete release
    Validate {
        /// Release version to validate (e.g., "1.0.0", "v1.0.0")
        version: String,

        /// Validation categories to run (default: all enabled)
        #[arg(short, long)]
        categories: Option<Vec<String>>,

        /// Report formats to generate
        #[arg(short, long, value_delimiter = ',', default_values = ["json", "markdown"])]
        formats: Vec<ReportFormat>,
    },

    /// List active validations
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Get validation status by ID
    Status {
        /// Validation ID
        id: Uuid,
    },

    /// Generate configuration file
    InitConfig {
        /// Output path for configuration file
        #[arg(short, long, default_value = "validation-config.toml")]
        path: String,
    },
}

/// Main CLI entry point
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    setup_logging(cli.verbose);

    match cli.command {
        Commands::Validate {
            version,
            categories,
            formats,
        } => {
            validate_release(version, categories, formats, cli.output_dir).await?;
        }
        Commands::List { detailed } => {
            list_validations(detailed).await?;
        }
        Commands::Status { id } => {
            show_validation_status(id).await?;
        }
        Commands::InitConfig { path } => {
            init_config(path)?;
        }
    }

    Ok(())
}

/// Validate a release
async fn validate_release(
    version: String,
    categories: Option<Vec<String>>,
    formats: Vec<ReportFormat>,
    output_dir: String,
) -> Result<()> {
    info!("Starting validation for release version: {}", version);

    // Create orchestrator
    let mut orchestrator = ValidationOrchestrator::new()?;

    // Run validation
    let report = if let Some(cats) = categories {
        orchestrator.validate_categories(&version, cats).await?
    } else {
        orchestrator.validate_release(&version).await?
    };

    // Generate reports
    let generator = ReportGenerator::new(output_dir);
    let output_files = generator.generate_all_formats(&report, &formats).await?;

    // Print summary
    print_validation_summary(&report);

    // Output generated files
    info!("Generated reports:");
    for file in output_files {
        info!("  - {}", file);
    }

    // Send webhook if configured
    let config = orchestrator.get_config();
    if let Some(webhook_url) = &config.notification_webhook {
        generator.send_webhook(&report, webhook_url).await?;
    }

    // Exit with appropriate code
    if report.is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Validation failed with issues"))
    }
}

/// List active validations
async fn list_validations(detailed: bool) -> Result<()> {
    let orchestrator = ValidationOrchestrator::new()?;
    let validations = orchestrator.list_validations().await;

    if validations.is_empty() {
        println!("No active validations found.");
        return Ok(());
    }

    println!("Active Validations:");
    println!(
        "{:<36} {:<10} {:<12} {:<10}",
        "ID", "Version", "Status", "Duration"
    );
    println!("{}", "-".repeat(72));

    for (id, summary) in validations {
        let status = format!("{:?}", summary.overall_status);
        let duration = format!("{}ms", summary.total_duration_ms);

        println!(
            "{:<36} {:<10} {:<12} {:<10}",
            id, summary.version, status, duration
        );

        if detailed {
            let stats = summary.get_statistics();
            println!(
                "  Total: {}, Passed: {}, Failed: {}, Issues: {}",
                stats.total_validations,
                stats.passed_validations,
                stats.failed_validations,
                stats.total_issues
            );
        }
    }

    Ok(())
}

/// Show validation status
async fn show_validation_status(id: Uuid) -> Result<()> {
    let orchestrator = ValidationOrchestrator::new()?;

    if let Some(validation) = orchestrator.get_validation(&id).await {
        let report = ValidationReport::from_summary(validation);
        print_validation_summary(&report);

        // Print detailed results
        println!("\nDetailed Results:");
        for (result_id, result) in &report.summary.results {
            println!("  {}:", result.name);
            println!("    Status: {:?}", result.status);
            println!("    Duration: {}ms", result.duration_ms);
            println!("    Issues: {}", result.issues.len());

            for issue in &result.issues {
                println!(
                    "      {:?}: {} - {}",
                    issue.severity, issue.title, issue.description
                );
            }
        }
    } else {
        println!("Validation with ID {} not found.", id);
    }

    Ok(())
}

/// Initialize configuration file
fn init_config(path: String) -> Result<()> {
    let config = ValidationConfig::default();
    let config_str = toml::to_string_pretty(&config)?;

    std::fs::write(&path, config_str)?;

    println!("Configuration file initialized at: {}", path);
    println!("Please review and customize the configuration before running validations.");

    Ok(())
}

/// Setup logging based on verbosity
fn setup_logging(verbose: bool) {
    let level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::from_default_env()
        .filter_level(level)
        .format_timestamp_secs()
        .init();
}

/// Print validation summary to console
fn print_validation_summary(report: &ValidationReport) {
    let stats = report.get_statistics();

    println!("\n🎯 Validation Summary");
    println!("==================");
    println!("Version: {}", report.version);
    println!("Overall Status: {:?}", report.summary.overall_status);
    println!("Total Validations: {}", stats.total_validations);
    println!(
        "Passed: {} ({:.1}%)",
        stats.passed_validations,
        stats.success_rate() * 100.0
    );
    println!(
        "Failed: {} ({:.1}%)",
        stats.failed_validations,
        stats.failure_rate() * 100.0
    );
    println!("Total Issues: {}", stats.total_issues);
    println!("Critical Issues: {}", stats.critical_issues);

    if report.is_success() {
        println!("\n✅ Validation PASSED");
    } else {
        println!("\n❌ Validation FAILED");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(&[
            "terraphim-validation",
            "validate",
            "1.0.0",
            "--formats",
            "json,markdown",
        ])
        .unwrap();

        match cli.command {
            Commands::Validate {
                version,
                categories,
                formats,
            } => {
                assert_eq!(version, "1.0.0");
                assert_eq!(categories, None);
                assert_eq!(formats.len(), 2);
                assert!(formats.contains(&ReportFormat::Json));
                assert!(formats.contains(&ReportFormat::Markdown));
            }
            _ => panic!("Expected Validate command"),
        }
    }
}
