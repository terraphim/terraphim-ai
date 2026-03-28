mod ground_truth;
mod llm_client;
mod models;
mod runner;
mod scoring;
mod types;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
#[command(
    name = "dumb-critic-experiment",
    about = "Validate the dumb critic hypothesis: smaller models as better plan reviewers",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Results directory
    #[arg(short, long, default_value = "experiment_results")]
    results_dir: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate ground truth manifest from plans directory
    GenerateGroundTruth {
        /// Plans directory
        #[arg(short, long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Output file path
        #[arg(short, long, default_value = "experiment_results/ground_truth.json")]
        output: PathBuf,
    },

    /// Run the experiment (review all plans with all models)
    Run {
        /// Ground truth file
        #[arg(short, long, default_value = "experiment_results/ground_truth.json")]
        ground_truth: PathBuf,
        /// Plans directory
        #[arg(short, long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Specific models to test (comma-separated: nano,small,medium,large,oracle)
        #[arg(short, long)]
        models: Option<String>,
        /// Resume from previous run
        #[arg(short, long)]
        resume: bool,
    },

    /// Score experiment results
    Score {
        /// Ground truth file
        #[arg(short, long, default_value = "experiment_results/ground_truth.json")]
        ground_truth: PathBuf,
    },

    /// Generate report from scored results
    Report {
        /// Results directory
        #[arg(short, long, default_value = "experiment_results")]
        results_dir: PathBuf,
        /// Output report file
        #[arg(short, long, default_value = "experiment_results/REPORT.md")]
        output: PathBuf,
    },

    /// Full pipeline: generate, run, score, report
    Full {
        /// Plans directory
        #[arg(short, long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Specific models to test (comma-separated)
        #[arg(short, long)]
        models: Option<String>,
    },

    /// Health check: verify API connectivity
    HealthCheck,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateGroundTruth { plans_dir, output } => {
            info!("Generating ground truth from {:?}", plans_dir);

            let llm_client = llm_client::LlmClient::from_env()?;
            let runner = runner::ExperimentRunner::new(llm_client, cli.results_dir);

            let manifest = runner.generate_ground_truth(&plans_dir, &output)?;

            println!("Ground truth generated:");
            println!("  Plans: {}", manifest.plans.len());
            println!("  Total defects: {}", manifest.total_defects());
            println!("  Seeded defects: {}", manifest.total_seeded_defects());
            println!("  Organic defects: {}", manifest.total_organic_defects());
            println!("  Output: {:?}", output);
        }

        Commands::Run {
            ground_truth,
            plans_dir,
            models,
            resume,
        } => {
            info!("Running experiment");

            let llm_client = llm_client::LlmClient::from_env()?;
            let runner = runner::ExperimentRunner::new(llm_client, cli.results_dir.clone());

            let manifest = runner.load_ground_truth(&ground_truth)?;

            let model_tiers = models.map(|m| parse_models(&m)).transpose()?;

            let reviews = if resume {
                runner
                    .resume_experiment(&manifest, &plans_dir, model_tiers)
                    .await?
            } else {
                runner
                    .run_experiment(&manifest, &plans_dir, model_tiers)
                    .await?
            };

            println!("Experiment complete:");
            println!("  Reviews collected: {}", reviews.len());
            println!("  Results saved to: {}/reviews/", cli.results_dir);
        }

        Commands::Score { ground_truth } => {
            info!("Scoring experiment results");

            let llm_client = llm_client::LlmClient::from_env()?;
            let runner = runner::ExperimentRunner::new(llm_client, cli.results_dir.clone());

            let manifest = runner.load_ground_truth(&ground_truth)?;
            let reviews = load_all_reviews(&PathBuf::from(&cli.results_dir).join("reviews"))?;

            let results = runner.score_experiment(&manifest, &reviews)?;

            println!("Scoring complete:");
            println!("  Plan scores: {}", results.plan_scores.len());

            match &results.conclusion {
                scoring::ExperimentConclusion::Confirmed {
                    winning_tier,
                    evidence,
                } => {
                    println!("  Conclusion: CONFIRMED");
                    println!("  Winner: {}", winning_tier);
                    println!("  Evidence: {}", evidence);
                }
                scoring::ExperimentConclusion::Refuted { evidence } => {
                    println!("  Conclusion: REFUTED");
                    println!("  Evidence: {}", evidence);
                }
                scoring::ExperimentConclusion::Nuanced { summary, .. } => {
                    println!("  Conclusion: NUANCED");
                    println!("  Summary: {}", summary);
                }
            }
        }

        Commands::Report {
            results_dir,
            output,
        } => {
            info!("Generating report");

            let llm_client = llm_client::LlmClient::from_env()?;
            let runner = runner::ExperimentRunner::new(
                llm_client,
                results_dir.to_string_lossy().to_string(),
            );

            let results_path = results_dir.join("experiment_results.json");
            let results_json = std::fs::read_to_string(&results_path)?;
            let results: scoring::ExperimentResults = serde_json::from_str(&results_json)?;

            runner.generate_report(&results, &output)?;

            println!("Report generated: {:?}", output);
        }

        Commands::Full { plans_dir, models } => {
            info!("Running full pipeline");

            let llm_client = llm_client::LlmClient::from_env()?;
            let runner = runner::ExperimentRunner::new(llm_client, cli.results_dir.clone());

            // Step 1: Generate ground truth
            let gt_path = PathBuf::from(&cli.results_dir).join("ground_truth.json");
            let manifest = runner.generate_ground_truth(&plans_dir, &gt_path)?;

            // Step 2: Run experiment
            let model_tiers = models.map(|m| parse_models(&m)).transpose()?;
            let reviews = runner
                .run_experiment(&manifest, &plans_dir, model_tiers)
                .await?;

            // Step 3: Score
            let results = runner.score_experiment(&manifest, &reviews)?;

            // Step 4: Report
            let report_path = PathBuf::from(&cli.results_dir).join("REPORT.md");
            runner.generate_report(&results, &report_path)?;

            println!("\n=== Full Pipeline Complete ===");
            println!("Ground truth: {:?}", gt_path);
            println!("Reviews collected: {}", reviews.len());
            println!("Report: {:?}", report_path);

            match &results.conclusion {
                scoring::ExperimentConclusion::Confirmed { winning_tier, .. } => {
                    println!(
                        "\n✓ Hypothesis CONFIRMED: {} is the optimal plan reviewer",
                        winning_tier
                    );
                }
                scoring::ExperimentConclusion::Refuted { .. } => {
                    println!("\n✗ Hypothesis REFUTED: Larger models still outperform");
                }
                scoring::ExperimentConclusion::Nuanced { .. } => {
                    println!("\n○ Hypothesis NUANCED: Results are mixed");
                }
            }
        }

        Commands::HealthCheck => {
            info!("Checking API connectivity");

            let llm_client = llm_client::LlmClient::from_env()?;

            match llm_client.health_check().await {
                Ok(true) => {
                    println!("✓ OpenRouter API is accessible");
                }
                Ok(false) => {
                    println!("✗ OpenRouter API returned error");
                }
                Err(e) => {
                    println!("✗ Failed to connect to OpenRouter API: {}", e);
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

fn parse_models(s: &str) -> Result<Vec<models::ModelTier>> {
    s.split(',')
        .map(|m| match m.trim().to_ascii_lowercase().as_str() {
            "nano" => Ok(models::ModelTier::Nano),
            "small" => Ok(models::ModelTier::Small),
            "medium" => Ok(models::ModelTier::Medium),
            "large" => Ok(models::ModelTier::Large),
            "oracle" => Ok(models::ModelTier::Oracle),
            other => Err(anyhow!("Unknown model tier: {}", other)),
        })
        .collect()
}

fn load_all_reviews(reviews_dir: &PathBuf) -> Result<Vec<models::ModelReview>> {
    let mut reviews = Vec::new();

    if !reviews_dir.exists() {
        return Ok(reviews);
    }

    for entry in std::fs::read_dir(reviews_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)?;
            let review: models::ModelReview = serde_json::from_str(&content)?;
            reviews.push(review);
        }
    }

    Ok(reviews)
}
