use crate::UsageRegistry;
use crate::formatter::{format_usage_json, format_usage_text};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "terraphim", about = "Terraphim AI CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Usage {
        #[command(subcommand)]
        action: UsageAction,
    },
}

#[derive(Subcommand)]
pub enum UsageAction {
    Show {
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    History {
        #[arg(long)]
        since: String,
        #[arg(long)]
        until: Option<String>,
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    Export {
        #[arg(short, long, default_value = "json")]
        format: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    Alert {
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long, default_value = "80")]
        threshold: u8,
    },
    Budgets {
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

pub async fn execute_usage_action(
    action: UsageAction,
    registry: &UsageRegistry,
) -> Result<String, Box<dyn std::error::Error>> {
    match action {
        UsageAction::Show { provider, format } => execute_show(provider, format, registry).await,
        UsageAction::History {
            since,
            until,
            provider,
            format,
        } => execute_history(since, until, provider, format).await,
        UsageAction::Export { format, output: _ } => execute_export(format).await,
        UsageAction::Alert {
            provider,
            threshold,
        } => execute_alert(provider, threshold).await,
        UsageAction::Budgets { format } => execute_budgets(format).await,
    }
}

async fn execute_show(
    provider: Option<String>,
    format: String,
    registry: &UsageRegistry,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    let provider_ids: Vec<&str> = if let Some(ref p) = provider {
        vec![p.as_str()]
    } else {
        registry.ids()
    };

    for id in provider_ids {
        if let Some(p) = registry.get(id) {
            match p.fetch_usage().await {
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

#[cfg(feature = "persistence")]
async fn execute_history(
    since: String,
    until: Option<String>,
    provider: Option<String>,
    format: String,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::store::UsageStore;

    let store = UsageStore::new();
    let executions = store
        .query_executions(&since, until.as_deref(), provider.as_deref())
        .await?;

    if executions.is_empty() {
        return Ok(format!("No execution history found from {}.", since));
    }

    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&executions)?;
            Ok(json)
        }
        "csv" => {
            let mut csv = String::from(
                "agent,input_tokens,output_tokens,total_tokens,cost_usd,model,provider,success,started_at\n",
            );
            for exec in &executions {
                csv.push_str(&format!(
                    "{},{},{},{},{:.4},{},{},{},{}\n",
                    exec.agent_name,
                    exec.input_tokens,
                    exec.output_tokens,
                    exec.total_tokens,
                    exec.cost_usd(),
                    exec.model.as_deref().unwrap_or(""),
                    exec.provider.as_deref().unwrap_or(""),
                    exec.success,
                    exec.started_at,
                ));
            }
            Ok(csv)
        }
        _ => {
            let mut output = String::new();
            output.push_str(&format!(
                "Execution History ({} to {})\n\n",
                since,
                until.as_deref().unwrap_or("now")
            ));
            for exec in &executions {
                output.push_str(&format!(
                    "  [{}] {} - {} tokens, ${:.4} ({})\n",
                    if exec.success { "OK" } else { "FAIL" },
                    exec.agent_name,
                    exec.total_tokens,
                    exec.cost_usd(),
                    exec.started_at,
                ));
            }
            output.push_str(&format!("\nTotal: {} executions\n", executions.len()));
            Ok(output)
        }
    }
}

#[cfg(not(feature = "persistence"))]
async fn execute_history(
    since: String,
    until: Option<String>,
    _provider: Option<String>,
    _format: String,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "History from {} to {} requires persistence feature",
        since,
        until.unwrap_or_else(|| "now".to_string())
    ))
}

#[cfg(feature = "persistence")]
async fn execute_export(format: String) -> Result<String, Box<dyn std::error::Error>> {
    use crate::store::UsageStore;

    let store = UsageStore::new();
    let since = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(30))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "2020-01-01".to_string());

    let export = store.export_usage_data(&since, None).await?;

    match format.as_str() {
        "csv" => {
            let mut csv = String::from("agent,budget_cents,spent_usd,total_tokens,executions\n");
            for m in &export.agent_metrics {
                csv.push_str(&format!(
                    "{},{},{:.2},{},{}\n",
                    m.agent_name,
                    m.budget_monthly_cents,
                    m.total_cost_usd(),
                    m.total_tokens(),
                    m.total_executions,
                ));
            }
            Ok(csv)
        }
        _ => {
            let json = serde_json::to_string_pretty(&export)?;
            Ok(json)
        }
    }
}

#[cfg(not(feature = "persistence"))]
async fn execute_export(format: String) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("Export as {} requires persistence feature", format))
}

#[cfg(feature = "persistence")]
async fn execute_alert(
    provider: Option<String>,
    threshold: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::store::{AlertConfig, UsageStore};
    use terraphim_persistence::Persistable;

    let agent_name = provider.unwrap_or_else(|| "*".to_string());
    let mut alert_config = AlertConfig::default_for_agent(&agent_name);

    let store = UsageStore::new();

    if agent_name != "*" {
        if let Ok(Some(metrics)) = store.get_agent_metrics(&agent_name).await {
            let percentage = metrics.budget_percentage_used();
            if let Some(triggered) = alert_config.should_alert(percentage) {
                alert_config.mark_alerted(triggered);
                alert_config.save().await?;
                Ok(format!(
                    "ALERT: {} has used {:.1}% of budget (threshold: {}%)",
                    agent_name, percentage, triggered
                ))
            } else {
                Ok(format!(
                    "OK: {} has used {:.1}% of budget (no threshold exceeded)",
                    agent_name, percentage
                ))
            }
        } else {
            alert_config.thresholds = vec![threshold];
            alert_config.save().await?;
            Ok(format!(
                "Alert configured for {} at {}% threshold",
                agent_name, threshold
            ))
        }
    } else {
        alert_config.thresholds = vec![threshold];
        alert_config.save().await?;
        Ok(format!(
            "Alert configured for all agents at {}% threshold",
            threshold
        ))
    }
}

#[cfg(not(feature = "persistence"))]
async fn execute_alert(
    provider: Option<String>,
    threshold: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "Alert for {:?} at {}% requires persistence feature",
        provider, threshold
    ))
}

#[cfg(feature = "persistence")]
async fn execute_budgets(format: String) -> Result<String, Box<dyn std::error::Error>> {
    use crate::store::{BudgetSnapshotRecord, UsageStore};

    let store = UsageStore::new();
    let metrics = store.list_agent_metrics().await?;

    if metrics.is_empty() {
        return Ok("No agent budget data found.".to_string());
    }

    let snapshots: Vec<_> = metrics
        .iter()
        .map(BudgetSnapshotRecord::from_agent_metrics)
        .collect();

    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&snapshots)?;
            Ok(json)
        }
        _ => {
            let mut output = String::new();
            output.push_str(&format!(
                "Budget Status - {}\n\n",
                chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
            ));
            for snapshot in &snapshots {
                let status = match snapshot.verdict {
                    crate::store::BudgetVerdict::WithinBudget => "OK",
                    crate::store::BudgetVerdict::ApproachingLimit => "WARN",
                    crate::store::BudgetVerdict::Exceeded => "OVER",
                };
                output.push_str(&format!(
                    "  [{:>4}] {} - ${:.2}/${:.2} ({:.1}%)\n",
                    status,
                    snapshot.agent_name,
                    snapshot.spent_usd(),
                    snapshot.budget_usd(),
                    snapshot.percentage_used,
                ));
            }
            Ok(output)
        }
    }
}

#[cfg(not(feature = "persistence"))]
async fn execute_budgets(_format: String) -> Result<String, Box<dyn std::error::Error>> {
    Ok("Budget status requires persistence feature".to_string())
}
