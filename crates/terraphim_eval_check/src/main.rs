use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use terraphim_eval_check::{Metrics, evaluate};

#[derive(Parser)]
#[command(name = "eval-check")]
#[command(about = "Terraphim codebase evaluation: compare baseline vs candidate metrics")]
struct Cli {
    /// Baseline metrics JSON file (produced by evaluate-agent.sh --mode baseline).
    #[arg(short, long)]
    baseline: PathBuf,

    /// Candidate metrics JSON file (produced by evaluate-agent.sh --mode candidate).
    #[arg(short, long)]
    candidate: PathBuf,

    /// Output format.
    #[arg(short, long, value_enum, default_value = "json")]
    format: OutputFormat,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
}

fn main() {
    let cli = Cli::parse();

    let baseline: Metrics = {
        let raw = std::fs::read_to_string(&cli.baseline)
            .unwrap_or_else(|e| panic!("cannot read baseline {}: {e}", cli.baseline.display()));
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid baseline JSON: {e}"))
    };

    let candidate: Metrics = {
        let raw = std::fs::read_to_string(&cli.candidate)
            .unwrap_or_else(|e| panic!("cannot read candidate {}: {e}", cli.candidate.display()));
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid candidate JSON: {e}"))
    };

    let report = evaluate(&baseline, &candidate);

    match cli.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("serialise")
            );
        }
        OutputFormat::Text => {
            println!("Verdict: {}", report.verdict);
            println!("Rationale: {}", report.rationale);
            println!("Delta — test_failures: {:+}", report.delta.test_failures);
            println!(
                "Delta — clippy_warnings: {:+}",
                report.delta.clippy_warnings
            );
            println!("Delta — clippy_errors: {:+}", report.delta.clippy_errors);
            println!("Delta — test_count: {:+}", report.delta.test_count);
        }
    }

    // Exit non-zero on Degraded so CI gates can act on it.
    if report.verdict == terraphim_eval_check::Verdict::Degraded {
        std::process::exit(1);
    }
}
