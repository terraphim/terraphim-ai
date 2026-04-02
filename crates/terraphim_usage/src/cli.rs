use crate::UsageRegistry;
use crate::formatter::{format_usage_csv, format_usage_json, format_usage_text};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "terraphim", about = "Terraphim AI CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show AI usage across all providers
    Usage {
        #[command(subcommand)]
        action: UsageAction,
    },
}

#[derive(Subcommand)]
pub enum UsageAction {
    /// Show current usage for all providers
    Show {
        /// Show specific provider (all if not specified)
        #[arg(short, long)]
        provider: Option<String>,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Show usage history for a time period
    History {
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        since: String,

        /// End date (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,

        /// Provider filter
        #[arg(short, long)]
        provider: Option<String>,

        /// Output format (text, json, csv)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Export usage data
    Export {
        /// Output format (json, csv, markdown)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Configure budget alerts
    Alert {
        /// Provider name
        #[arg(short, long)]
        provider: Option<String>,

        /// Alert threshold percentage
        #[arg(short, long, default_value = "80")]
        threshold: u8,
    },
    /// Show budget status for all agents
    Budgets {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

pub async fn execute_usage_action(
    action: UsageAction,
    registry: &UsageRegistry,
) -> Result<String, Box<dyn std::error::Error>> {
    match action {
        UsageAction::Show { provider, format } => {
            let mut results = Vec::new();

            let provider_ids: Vec<&str> = if let Some(ref p) = provider {
                vec![p.as_str()]
            } else {
                registry.ids()
            };

            for id in provider_ids {
                if let Some(provider) = registry.get(id) {
                    match provider.fetch_usage().await {
                        Ok(usage) => results.push(usage),
                        Err(e) => eprintln!("Warning: Failed to fetch {} usage: {}", id, e),
                    }
                }
            }

            match format.as_str() {
                "json" => {
                    let json_results: Vec<_> = results
                        .iter()
                        .map(format_usage_json)
                        .collect::<Result<_, _>>()?;
                    Ok(format!("[{}]", json_results.join(",\n")))
                }
                _ => {
                    let mut output = String::new();
                    output.push_str(&format!(
                        "AI Usage Summary - {}\n\n",
                        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
                    ));
                    for usage in &results {
                        output.push_str(&format_usage_text(usage));
                        output.push('\n');
                    }
                    Ok(output)
                }
            }
        }
        UsageAction::History {
            since,
            until,
            provider: _,
            format: _,
        } => {
            // TODO: Query UsageStore for historical data
            Ok(format!(
                "History from {} to {} (not yet implemented)",
                since,
                until.unwrap_or_else(|| "now".to_string())
            ))
        }
        UsageAction::Export { format, output } => {
            // TODO: Export all usage data
            Ok(format!("Export as {} (not yet implemented)", format))
        }
        UsageAction::Alert {
            provider,
            threshold,
        } => {
            // TODO: Configure alert thresholds
            Ok(format!(
                "Alert for {:?} at {}% (not yet implemented)",
                provider, threshold
            ))
        }
        UsageAction::Budgets { format: _ } => {
            // TODO: Show budget status from CostTracker
            Ok("Budget status (not yet implemented)".to_string())
        }
    }
}
