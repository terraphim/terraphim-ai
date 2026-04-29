use crate::{MetricLine, ProgressFormat, ProviderUsage, Result, UsageError, UsageProvider};
use std::path::PathBuf;

pub struct OpenCodeGoProvider {
    db_path: PathBuf,
}

impl OpenCodeGoProvider {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        Self {
            db_path: PathBuf::from(format!("{}/.local/share/opencode/opencode.db", home)),
        }
    }

    pub fn with_db_path(path: PathBuf) -> Self {
        Self { db_path: path }
    }
}

impl Default for OpenCodeGoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenCodeGoProvider {
    fn query_sqlite(&self, query: &str) -> std::result::Result<String, std::io::Error> {
        let output = std::process::Command::new("sqlite3")
            .arg("-json")
            .arg(&self.db_path)
            .arg(query)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(std::io::Error::other(format!("sqlite3 failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl UsageProvider for OpenCodeGoProvider {
    fn id(&self) -> &str {
        "opencode-go"
    }

    fn display_name(&self) -> &str {
        "OpenCode Go"
    }

    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>
    {
        Box::pin(async move {
            if !self.db_path.exists() {
                return Err(UsageError::ProviderNotFound(
                    "opencode-go database not found".to_string(),
                ));
            }

            if which::which("sqlite3").is_err() {
                return Err(UsageError::FetchFailed {
                    provider: "opencode-go".to_string(),
                    source: "sqlite3 CLI not found. Install sqlite3 to query OpenCode Go usage."
                        .into(),
                });
            }

            let query = "SELECT role, COUNT(*) as count, SUM(CASE WHEN cost IS NOT NULL THEN cost ELSE 0 END) as total_cost FROM message GROUP BY role";

            let json_str = self
                .query_sqlite(query)
                .map_err(|e| UsageError::FetchFailed {
                    provider: "opencode-go".to_string(),
                    source: e.into(),
                })?;

            let rows: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap_or_default();

            let mut total_cost: f64 = 0.0;
            let mut total_messages: i64 = 0;
            let mut assistant_count: i64 = 0;

            for row in &rows {
                if let Some(cost) = row.get("total_cost").and_then(|v| v.as_f64()) {
                    total_cost += cost;
                }
                if let Some(count) = row.get("count").and_then(|v| v.as_i64()) {
                    total_messages += count;
                }
                if row
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|r| r == "assistant")
                    .unwrap_or(false)
                {
                    assistant_count = row.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                }
            }

            let mut lines = Vec::new();

            lines.push(MetricLine::Progress {
                label: "Total spend".to_string(),
                used: total_cost,
                limit: 50.0,
                format: ProgressFormat::Dollars,
                resets_at: None,
                period_duration_ms: None,
                color: None,
            });

            lines.push(MetricLine::Text {
                label: "Messages".to_string(),
                value: format!(
                    "{} total ({} assistant responses)",
                    total_messages, assistant_count
                ),
                color: None,
                subtitle: None,
            });

            Ok(ProviderUsage {
                provider_id: "opencode-go".to_string(),
                display_name: "OpenCode Go".to_string(),
                plan: None,
                lines,
                fetched_at: chrono::Utc::now().to_rfc3339(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencode_go_provider_id() {
        let provider = OpenCodeGoProvider::new();
        assert_eq!(provider.id(), "opencode-go");
    }

    #[test]
    fn test_opencode_go_provider_display_name() {
        let provider = OpenCodeGoProvider::new();
        assert_eq!(provider.display_name(), "OpenCode Go");
    }

    #[test]
    fn test_opencode_go_missing_db() {
        let provider = OpenCodeGoProvider::with_db_path(PathBuf::from("/nonexistent/opencode.db"));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.fetch_usage());
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            UsageError::ProviderNotFound(msg) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected ProviderNotFound, got {:?}", err),
        }
    }
}
