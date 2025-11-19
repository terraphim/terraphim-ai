use clap::{ArgAction, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser, Debug)]
#[command(
    name = "terraphim-kg-lint",
    version,
    about = "Lint markdown-based Terraphim KG schemas (commands, types, permissions)"
)]
struct Cli {
    /// Path to a markdown file or directory (recursively scanned)
    #[arg(short, long, default_value = "docs/src/kg")]
    path: PathBuf,

    /// Output format
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Fail with non-zero exit code if any issues are found
    #[arg(long, action = ArgAction::SetTrue)]
    strict: bool,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();
    let path = cli.path;

    match terraphim_kg_linter::lint_path(&path).await {
        Ok(report) => match cli.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
                if cli.strict && !report.issues.is_empty() {
                    std::process::exit(2);
                }
            }
            OutputFormat::Text => {
                println!(
                    "Scanned {} files | commands: {} types: {} roles: {} thesaurus terms: {}",
                    report.scanned_files,
                    report.stats.command_count,
                    report.stats.type_count,
                    report.stats.role_count,
                    report.stats.thesaurus_terms
                );
                if report.issues.is_empty() {
                    println!("No issues found ✅");
                } else {
                    println!("Found {} issue(s):", report.issues.len());
                    for issue in report.issues {
                        println!(
                            "- [{:?}] {}: {} ({})",
                            issue.severity,
                            issue.code,
                            issue.message,
                            issue.path.display()
                        );
                    }
                    if cli.strict {
                        std::process::exit(2);
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
