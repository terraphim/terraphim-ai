//! Judge Evaluator CLI
//!
//! Command-line interface for the terraphim judge evaluator.
//! Provides commands for single file evaluation, batch evaluation,
//! and calibration of judge tiers.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

use terraphim_judge_evaluator::{BatchEvaluator, BatchSummary, JudgeAgent};

/// CLI arguments for the judge-evaluator binary
#[derive(Parser)]
#[command(name = "judge-evaluator")]
#[command(about = "Terraphim Judge Evaluator CLI")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available CLI commands
#[derive(Subcommand)]
enum Commands {
    /// Evaluate a single file
    Evaluate {
        /// Path to the file to evaluate
        #[arg(short, long)]
        file: PathBuf,
        /// Evaluation profile to use
        #[arg(short, long)]
        profile: String,
        /// Judge tier to use (optional, uses profile default if not specified)
        #[arg(short, long)]
        tier: Option<String>,
    },
    /// Evaluate a batch of files in a directory
    Batch {
        /// Directory containing files to evaluate
        #[arg(short, long)]
        dir: PathBuf,
        /// Evaluation profile to use
        #[arg(short, long)]
        profile: String,
        /// Maximum number of concurrent evaluations
        #[arg(short, long, default_value = "4")]
        max_concurrency: usize,
        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        output: OutputFormat,
    },
    /// Calibrate judge tier thresholds
    Calibrate {
        /// Judge tier to calibrate
        #[arg(short, long)]
        tier: String,
        /// Number of sample evaluations to run
        #[arg(short, long, default_value = "100")]
        samples: usize,
    },
}

/// Output format options
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output for automation
    Json,
}

/// CLI error type
#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Evaluation error: {0}")]
    Evaluation(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Evaluate {
            file,
            profile,
            tier: _,
        } => {
            evaluate_single(file, &profile).await?;
        }
        Commands::Batch {
            dir,
            profile,
            max_concurrency,
            output,
        } => {
            evaluate_batch(dir, &profile, max_concurrency, output).await?;
        }
        Commands::Calibrate { tier, samples: _ } => {
            calibrate_tier(&tier).await?;
        }
    }

    Ok(())
}

/// Evaluate a single file
async fn evaluate_single(file: PathBuf, profile: &str) -> Result<(), CliError> {
    let judge = JudgeAgent::new();

    match judge.evaluate(&file, profile).await {
        Ok(verdict) => {
            println!("File: {}", file.display());
            println!("Verdict: {}", verdict.verdict);
            println!("Tier: {}", verdict.judge_tier);
            println!("Latency: {}ms", verdict.latency_ms);

            if !verdict.scores.is_empty() {
                println!("Scores:");
                for (category, score) in &verdict.scores {
                    println!("  {}: {:.2}", category, score);
                }
            }

            if verdict.is_pass() {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error evaluating file: {}", e);
            Err(CliError::Evaluation(e.to_string()))
        }
    }
}

/// Evaluate a batch of files
async fn evaluate_batch(
    dir: PathBuf,
    profile: &str,
    max_concurrency: usize,
    output: OutputFormat,
) -> Result<(), CliError> {
    // Collect all files from directory
    let files = collect_files(&dir)?;

    if files.is_empty() {
        eprintln!("No files found in directory: {}", dir.display());
        return Ok(());
    }

    let judge = JudgeAgent::new();
    let evaluator = BatchEvaluator::new(judge, max_concurrency);

    let (results, summary) = evaluator.evaluate_batch_with_summary(files, profile).await;

    match output {
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "results": results,
                "summary": summary,
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Text => {
            print_text_summary(&results, &summary, &dir);
        }
    }

    // Exit with error code if any evaluations failed
    if summary.errors > 0 || summary.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Calibrate a judge tier
async fn calibrate_tier(tier: &str) -> Result<(), CliError> {
    println!("Calibrating judge tier: {}", tier);
    println!("This feature is not yet fully implemented.");
    println!("Tier {} would be calibrated with default samples.", tier);
    Ok(())
}

/// Collect all files from a directory recursively
fn collect_files(dir: &PathBuf) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();

    if dir.is_file() {
        files.push(dir.clone());
        return Ok(files);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            files.extend(collect_files(&path)?);
        }
    }

    Ok(files)
}

/// Print results in text format
fn print_text_summary(
    results: &[terraphim_judge_evaluator::BatchResult],
    summary: &BatchSummary,
    dir: &Path,
) {
    println!("Batch Evaluation Results");
    println!("========================");
    println!("Directory: {}", dir.display());
    println!();

    // Print individual results
    for result in results {
        let status = if result.is_error() {
            "ERROR"
        } else if result
            .verdict
            .as_ref()
            .map(|v| v.is_pass())
            .unwrap_or(false)
        {
            "PASS"
        } else if result
            .verdict
            .as_ref()
            .map(|v| v.is_fail())
            .unwrap_or(false)
        {
            "FAIL"
        } else {
            "UNKNOWN"
        };

        println!(
            "{}: {} ({}ms)",
            status,
            result.file.display(),
            result.duration_ms
        );

        if let Some(error) = &result.error {
            println!("  Error: {}", error);
        }
    }

    println!();
    println!("Summary");
    println!("-------");
    println!("Total files: {}", summary.total);
    println!("Passed: {}", summary.passed);
    println!("Failed: {}", summary.failed);
    println!("Errors: {}", summary.errors);
    println!("Average latency: {}ms", summary.avg_latency_ms);
    println!("Total duration: {}ms", summary.total_duration_ms);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cli_parse_evaluate() {
        let args = vec![
            "judge-evaluator",
            "evaluate",
            "--file",
            "test.rs",
            "--profile",
            "default",
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Evaluate {
                file,
                profile,
                tier,
            } => {
                assert_eq!(file, PathBuf::from("test.rs"));
                assert_eq!(profile, "default");
                assert!(tier.is_none());
            }
            _ => panic!("Expected Evaluate command"),
        }
    }

    #[test]
    fn test_cli_parse_evaluate_with_tier() {
        let args = vec![
            "judge-evaluator",
            "evaluate",
            "--file",
            "test.rs",
            "--profile",
            "default",
            "--tier",
            "quick",
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Evaluate {
                file,
                profile,
                tier,
            } => {
                assert_eq!(file, PathBuf::from("test.rs"));
                assert_eq!(profile, "default");
                assert_eq!(tier, Some("quick".to_string()));
            }
            _ => panic!("Expected Evaluate command"),
        }
    }

    #[test]
    fn test_cli_parse_batch() {
        let args = vec![
            "judge-evaluator",
            "batch",
            "--dir",
            "./src",
            "--profile",
            "thorough",
            "--max-concurrency",
            "8",
            "--output",
            "json",
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Batch {
                dir,
                profile,
                max_concurrency,
                output,
            } => {
                assert_eq!(dir, PathBuf::from("./src"));
                assert_eq!(profile, "thorough");
                assert_eq!(max_concurrency, 8);
                assert!(matches!(output, OutputFormat::Json));
            }
            _ => panic!("Expected Batch command"),
        }
    }

    #[test]
    fn test_cli_parse_batch_defaults() {
        let args = vec![
            "judge-evaluator",
            "batch",
            "--dir",
            "./src",
            "--profile",
            "default",
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Batch {
                dir,
                profile,
                max_concurrency,
                output,
            } => {
                assert_eq!(dir, PathBuf::from("./src"));
                assert_eq!(profile, "default");
                assert_eq!(max_concurrency, 4); // default value
                assert!(matches!(output, OutputFormat::Text)); // default value
            }
            _ => panic!("Expected Batch command"),
        }
    }

    #[test]
    fn test_cli_parse_calibrate() {
        let args = vec!["judge-evaluator", "calibrate", "--tier", "quick"];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Calibrate { tier, samples } => {
                assert_eq!(tier, "quick");
                assert_eq!(samples, 100); // default value
            }
            _ => panic!("Expected Calibrate command"),
        }
    }

    #[test]
    fn test_cli_parse_calibrate_with_samples() {
        let args = vec![
            "judge-evaluator",
            "calibrate",
            "--tier",
            "deep",
            "--samples",
            "50",
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Calibrate { tier, samples } => {
                assert_eq!(tier, "deep");
                assert_eq!(samples, 50);
            }
            _ => panic!("Expected Calibrate command"),
        }
    }

    #[test]
    fn test_help_text_generation() {
        // Test that help text is generated without panicking
        let mut app = <Cli as clap::CommandFactory>::command();
        let mut buf = Vec::new();
        app.write_help(&mut buf).expect("Failed to write help");
        let help_text = String::from_utf8(buf).expect("Invalid UTF-8 in help text");

        assert!(help_text.contains("judge-evaluator"));
        assert!(help_text.contains("evaluate"));
        assert!(help_text.contains("batch"));
        assert!(help_text.contains("calibrate"));
    }

    #[test]
    fn test_collect_files_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let files = collect_files(&file_path).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], file_path);
    }

    #[test]
    fn test_collect_files_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let file1 = temp_dir.path().join("file1.rs");
        let file2 = temp_dir.path().join("file2.rs");
        std::fs::write(&file1, "fn main() {}").unwrap();
        std::fs::write(&file2, "fn test() {}").unwrap();

        let files = collect_files(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&file1));
        assert!(files.contains(&file2));
    }

    #[test]
    fn test_output_format_variants() {
        // Test that OutputFormat can be parsed from strings
        let text = OutputFormat::from_str("text", true);
        assert!(text.is_ok());
        assert!(matches!(text.unwrap(), OutputFormat::Text));

        let json = OutputFormat::from_str("json", true);
        assert!(json.is_ok());
        assert!(matches!(json.unwrap(), OutputFormat::Json));
    }
}
