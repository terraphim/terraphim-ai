use crate::{MetricLine, ProviderUsage};
use std::fmt::Write;

pub fn format_usage_text(usage: &ProviderUsage) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "{} ({})",
        usage.display_name,
        usage.plan.as_deref().unwrap_or("Unknown")
    )
    .unwrap();

    for line in &usage.lines {
        match line {
            MetricLine::Text { label, value, .. } => {
                writeln!(output, "  {}: {}", label, value).unwrap();
            }
            MetricLine::Progress {
                label,
                used,
                limit,
                format,
                resets_at,
                ..
            } => {
                let pct = if *limit > 0.0 {
                    (*used / *limit) * 100.0
                } else {
                    0.0
                };
                let bar = progress_bar(pct);
                let display = match format {
                    crate::ProgressFormat::Percent => format!("{:.0}%", pct),
                    crate::ProgressFormat::Dollars => format!("${:.2}", used / 100.0),
                    crate::ProgressFormat::Count { suffix } => format!("{:.0} {}", used, suffix),
                };
                let reset_info = resets_at
                    .as_ref()
                    .map(|r| format!(" (resets {})", r))
                    .unwrap_or_default();
                writeln!(output, "  {}: {} {}{}", label, bar, display, reset_info).unwrap();
            }
            MetricLine::Badge { label, text, .. } => {
                writeln!(output, "  {}: {}", label, text).unwrap();
            }
        }
    }
    output
}

fn progress_bar(pct: f64) -> String {
    let filled = (pct / 5.0).round() as usize;
    let filled = filled.min(20);
    let empty = 20 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

pub fn format_usage_json(usage: &ProviderUsage) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(usage)
}

pub fn format_usage_csv(usages: &[ProviderUsage]) -> String {
    let mut csv =
        String::from("provider,plan,line_type,label,value,used,limit,resets_at,fetched_at\n");
    for usage in usages {
        for line in &usage.lines {
            match line {
                MetricLine::Text { label, value, .. } => {
                    writeln!(
                        csv,
                        "{},{},text,\"{}\",\"{}\",,,{}",
                        usage.display_name,
                        usage.plan.as_deref().unwrap_or(""),
                        label,
                        value,
                        usage.fetched_at
                    )
                    .unwrap();
                }
                MetricLine::Progress {
                    label,
                    used,
                    limit,
                    resets_at,
                    ..
                } => {
                    writeln!(
                        csv,
                        "{},{},progress,\"{}\",{},{},{},{}",
                        usage.display_name,
                        usage.plan.as_deref().unwrap_or(""),
                        label,
                        used,
                        limit,
                        resets_at.as_deref().unwrap_or(""),
                        usage.fetched_at
                    )
                    .unwrap();
                }
                MetricLine::Badge { label, text, .. } => {
                    writeln!(
                        csv,
                        "{},{},badge,\"{}\",\"{}\",,,{}",
                        usage.display_name,
                        usage.plan.as_deref().unwrap_or(""),
                        label,
                        text,
                        usage.fetched_at
                    )
                    .unwrap();
                }
            }
        }
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProgressFormat;

    #[test]
    fn test_format_usage_text_with_progress() {
        let usage = ProviderUsage {
            provider_id: "test".to_string(),
            display_name: "Test Provider".to_string(),
            plan: Some("Pro".to_string()),
            lines: vec![MetricLine::Progress {
                label: "Session".to_string(),
                used: 42.0,
                limit: 100.0,
                format: ProgressFormat::Percent,
                resets_at: Some("2026-04-02T15:00:00Z".to_string()),
                period_duration_ms: None,
                color: None,
            }],
            fetched_at: "2026-04-02T10:00:00Z".to_string(),
        };
        let output = format_usage_text(&usage);
        assert!(output.contains("Test Provider (Pro)"));
        assert!(output.contains("Session"));
        assert!(output.contains("42%"));
    }

    #[test]
    fn test_format_usage_json() {
        let usage = ProviderUsage {
            provider_id: "test".to_string(),
            display_name: "Test".to_string(),
            plan: None,
            lines: vec![],
            fetched_at: "2026-04-02T10:00:00Z".to_string(),
        };
        let json = format_usage_json(&usage).unwrap();
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_format_usage_csv() {
        let usages = vec![ProviderUsage {
            provider_id: "test".to_string(),
            display_name: "Test".to_string(),
            plan: Some("Pro".to_string()),
            lines: vec![MetricLine::Text {
                label: "Account".to_string(),
                value: "user@test.com".to_string(),
                color: None,
                subtitle: None,
            }],
            fetched_at: "2026-04-02T10:00:00Z".to_string(),
        }];
        let csv = format_usage_csv(&usages);
        assert!(csv.contains("provider,plan,line_type"));
        assert!(csv.contains("Test"));
        assert!(csv.contains("Account"));
    }

    #[test]
    fn test_progress_bar_full() {
        assert_eq!(progress_bar(100.0), "████████████████████");
    }

    #[test]
    fn test_progress_bar_half() {
        assert_eq!(progress_bar(50.0), "██████████░░░░░░░░░░");
    }

    #[test]
    fn test_progress_bar_empty() {
        assert_eq!(progress_bar(0.0), "░░░░░░░░░░░░░░░░░░░░");
    }
}
