use crate::UsageRegistry;
use crate::formatter::{format_usage_json, format_usage_text};
use clap::{Parser, Subcommand};
use jiff::Zoned;
use std::collections::BTreeMap;

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
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long, help = "Shorthand period, e.g. '7d', '30d'")]
        last: Option<String>,
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long)]
        model: Option<String>,
        #[arg(long, help = "Group results by model")]
        by_model: bool,
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
        #[arg(short, long)]
        budget: Option<f64>,
        #[arg(short, long, default_value = "80")]
        threshold: u8,
    },
    Budgets {
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

fn resolve_since(last: &Option<String>, since: &Option<String>) -> String {
    if let Some(s) = since {
        return s.clone();
    }
    let days = match last {
        Some(period) => parse_period(period).unwrap_or(7),
        None => 7,
    };
    days_ago_date(days)
}

fn days_ago_date(days: i64) -> String {
    jiff::Span::new()
        .try_days(-days)
        .ok()
        .and_then(|span| Zoned::now().checked_add(span).ok())
        .map(|dt| dt.strftime("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "2020-01-01".to_string())
}

fn month_start_date() -> String {
    let now = Zoned::now();
    format!("{}-{:02}-01", now.year(), now.month())
}

fn today_date() -> String {
    Zoned::now().strftime("%Y-%m-%d").to_string()
}

fn now_timestamp() -> String {
    Zoned::now().strftime("%Y-%m-%d %H:%M UTC").to_string()
}

fn parse_period(period: &str) -> Option<i64> {
    let period = period.to_lowercase();
    if let Some(num) = period.strip_suffix('d') {
        return num.parse().ok();
    }
    if let Some(num) = period.strip_suffix('w') {
        return num.parse::<i64>().ok().map(|n| n * 7);
    }
    if let Some(num) = period.strip_suffix('m') {
        return num.parse::<i64>().ok().map(|n| n * 30);
    }
    period.parse().ok()
}

struct ModelAggregation {
    total_tokens: i64,
    total_cost: f64,
    count: usize,
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
            last,
            provider,
            model,
            by_model,
            format,
        } => execute_history(since, until, last, provider, model, by_model, format).await,
        UsageAction::Export { format, output: _ } => execute_export(format).await,
        UsageAction::Alert {
            provider,
            budget,
            threshold,
        } => execute_alert(provider, budget, threshold).await,
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

    let mut output = String::new();

    match format.as_str() {
        "json" => {
            let json_results: Vec<_> = results
                .iter()
                .map(format_usage_json)
                .collect::<Result<_, _>>()?;
            output.push_str(&format!("[{}]", json_results.join(",\n")));
        }
        _ => {
            output.push_str(&format!("AI Usage Summary - {}\n\n", now_timestamp()));
            for usage in &results {
                output.push_str(&format_usage_text(usage));
                output.push('\n');
            }
        }
    }

    #[cfg(feature = "persistence")]
    {
        let spend = aggregate_spend().await?;
        if !spend.is_empty() {
            output.push_str("Spend from Execution Records:\n");
            for (period, cost, tokens) in &spend {
                output.push_str(&format!("  {}: ${:.2} ({} tokens)\n", period, cost, tokens));
            }
        }
    }

    Ok(output)
}

#[cfg(feature = "persistence")]
async fn aggregate_spend() -> Result<Vec<(String, f64, i64)>, Box<dyn std::error::Error>> {
    use crate::store::UsageStore;

    let store = UsageStore::new();
    let mut result = Vec::new();

    let today_str = today_date();
    let week_str = days_ago_date(7);
    let month_str = month_start_date();

    for (label, since) in [
        ("Today", today_str),
        ("This week", week_str),
        ("This month", month_str),
    ] {
        let execs = store.query_executions(&since, None, None).await?;
        let cost: f64 = execs.iter().map(|e| e.cost_usd()).sum();
        let tokens: i64 = execs.iter().map(|e| e.total_tokens).sum();
        if cost > 0.0 || tokens > 0 {
            result.push((label.to_string(), cost, tokens));
        }
    }

    Ok(result)
}

#[cfg(not(feature = "persistence"))]
async fn aggregate_spend() -> Result<Vec<(String, f64, i64)>, Box<dyn std::error::Error>> {
    Ok(Vec::new())
}

#[cfg(feature = "persistence")]
async fn execute_history(
    since: Option<String>,
    until: Option<String>,
    last: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    by_model: bool,
    format: String,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::store::UsageStore;

    let since_str = resolve_since(&last, &since);
    let store = UsageStore::new();
    let mut executions = store
        .query_executions(&since_str, until.as_deref(), None)
        .await?;

    if let Some(ref model_filter) = model {
        let filter_lower = model_filter.to_lowercase();
        executions.retain(|e| {
            e.model
                .as_deref()
                .map(|m| m.to_lowercase().contains(&filter_lower))
                .unwrap_or(false)
        });
    }

    if let Some(ref provider_filter) = provider {
        let filter_lower = provider_filter.to_lowercase();
        executions.retain(|e| {
            e.provider
                .as_deref()
                .map(|p| p.to_lowercase().contains(&filter_lower))
                .unwrap_or(false)
        });
    }

    if executions.is_empty() {
        return Ok(format!("No execution history found from {}.", since_str));
    }

    if by_model {
        return format_by_model(&executions, &since_str);
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
                since_str,
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

#[cfg(feature = "persistence")]
fn format_by_model(
    executions: &[crate::store::ExecutionRecord],
    since_str: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut grouped: BTreeMap<String, ModelAggregation> = BTreeMap::new();

    for exec in executions {
        let key = exec.model.as_deref().unwrap_or("unknown").to_string();
        let entry = grouped.entry(key).or_insert_with(|| ModelAggregation {
            total_tokens: 0,
            total_cost: 0.0,
            count: 0,
        });
        entry.total_tokens += exec.total_tokens;
        entry.total_cost += exec.cost_usd();
        entry.count += 1;
    }

    let mut output = String::new();
    output.push_str(&format!("Cost by Model (from {})\n\n", since_str));
    output.push_str(&format!(
        "{:<40} {:>12} {:>10} {:>6}\n",
        "Model", "Tokens", "Cost", "Calls"
    ));
    output.push_str(&"-".repeat(70));
    output.push('\n');

    let mut grand_tokens: i64 = 0;
    let mut grand_cost: f64 = 0.0;
    let mut grand_calls: usize = 0;

    for (model, agg) in &grouped {
        output.push_str(&format!(
            "{:<40} {:>12} ${:>9.2} {:>6}\n",
            truncate(model, 40),
            agg.total_tokens,
            agg.total_cost,
            agg.count
        ));
        grand_tokens += agg.total_tokens;
        grand_cost += agg.total_cost;
        grand_calls += agg.count;
    }

    output.push_str(&"-".repeat(70));
    output.push('\n');
    output.push_str(&format!(
        "{:<40} {:>12} ${:>9.2} {:>6}\n",
        "TOTAL", grand_tokens, grand_cost, grand_calls
    ));

    Ok(output)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(not(feature = "persistence"))]
async fn execute_history(
    since: Option<String>,
    until: Option<String>,
    _last: Option<String>,
    _provider: Option<String>,
    _model: Option<String>,
    _by_model: bool,
    _format: String,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "History from {} to {} requires persistence feature",
        since.unwrap_or_default(),
        until.unwrap_or_else(|| "now".to_string())
    ))
}

#[cfg(feature = "persistence")]
async fn execute_export(format: String) -> Result<String, Box<dyn std::error::Error>> {
    use crate::store::UsageStore;

    let store = UsageStore::new();
    let since = days_ago_date(30);

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
    budget: Option<f64>,
    threshold: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::store::UsageStore;

    let store = UsageStore::new();

    if let Some(budget_usd) = budget {
        let month_start = month_start_date();

        let executions = store.query_executions(&month_start, None, None).await?;
        let spent: f64 = executions.iter().map(|e| e.cost_usd()).sum();
        let pct = if budget_usd > 0.0 {
            (spent / budget_usd) * 100.0
        } else {
            0.0
        };

        let status = if pct >= threshold as f64 {
            "ALERT"
        } else if pct >= (threshold as f64) * 0.9 {
            "WARNING"
        } else {
            "OK"
        };

        return Ok(format!(
            "{}: Monthly spend at ${:.2} ({:.1}% of ${:.2} budget, threshold: {}%)\nExecutions: {}, Tokens: {}",
            status,
            spent,
            pct,
            budget_usd,
            threshold,
            executions.len(),
            executions.iter().map(|e| e.total_tokens).sum::<i64>()
        ));
    }

    let agent_name = provider.unwrap_or_else(|| "*".to_string());

    if agent_name != "*" {
        if let Ok(Some(metrics)) = store.get_agent_metrics(&agent_name).await {
            let percentage = metrics.budget_percentage_used();
            let alert_config = crate::store::AlertConfig::default_for_agent(&agent_name);
            if let Some(triggered) = alert_config.should_alert(percentage) {
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
            Ok(format!(
                "Alert configured for {} at {}% threshold",
                agent_name, threshold
            ))
        }
    } else {
        Ok(format!(
            "Alert configured for all agents at {}% threshold",
            threshold
        ))
    }
}

#[cfg(not(feature = "persistence"))]
async fn execute_alert(
    provider: Option<String>,
    _budget: Option<f64>,
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
            output.push_str(&format!("Budget Status - {}\n\n", now_timestamp()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_period_days() {
        assert_eq!(parse_period("7d"), Some(7));
        assert_eq!(parse_period("30d"), Some(30));
        assert_eq!(parse_period("1d"), Some(1));
    }

    #[test]
    fn test_parse_period_weeks() {
        assert_eq!(parse_period("1w"), Some(7));
        assert_eq!(parse_period("2w"), Some(14));
    }

    #[test]
    fn test_parse_period_months() {
        assert_eq!(parse_period("1m"), Some(30));
        assert_eq!(parse_period("3m"), Some(90));
    }

    #[test]
    fn test_parse_period_bare_number() {
        assert_eq!(parse_period("7"), Some(7));
    }

    #[test]
    fn test_parse_period_invalid() {
        assert_eq!(parse_period("abc"), None);
    }

    #[test]
    fn test_resolve_since_explicit() {
        let result = resolve_since(&None, &Some("2026-01-01".to_string()));
        assert_eq!(result, "2026-01-01");
    }

    #[test]
    fn test_resolve_since_last() {
        let result = resolve_since(&Some("7d".to_string()), &None);
        let expected = days_ago_date(7);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_resolve_since_default() {
        let result = resolve_since(&None, &None);
        let expected = days_ago_date(7);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_month_start_date() {
        let result = month_start_date();
        let now = Zoned::now();
        let expected = format!("{}-{:02}-01", now.year(), now.month());
        assert_eq!(result, expected);
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(
            truncate("a_very_long_model_name_that_exceeds_limit", 20),
            "a_very_long_model..."
        );
    }
}
