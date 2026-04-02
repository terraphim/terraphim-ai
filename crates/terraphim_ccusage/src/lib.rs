use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CcusageError {
    #[error("No package runner found (bun, pnpm, yarn, npm, npx)")]
    NoRunner,

    #[error("Runner execution failed: {0}")]
    RunnerFailed(String),

    #[error("Failed to parse ccusage output: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Timeout after {0:?}")]
    Timeout(Duration),
}

pub type Result<T> = std::result::Result<T, CcusageError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String,
    #[serde(rename = "inputTokens", alias = "input_tokens")]
    pub input_tokens: Option<u64>,
    #[serde(rename = "outputTokens", alias = "output_tokens")]
    pub output_tokens: Option<u64>,
    #[serde(rename = "cacheCreationTokens", alias = "cache_creation_tokens")]
    pub cache_creation_tokens: Option<u64>,
    #[serde(rename = "cacheReadTokens", alias = "cache_read_tokens")]
    pub cache_read_tokens: Option<u64>,
    #[serde(rename = "cachedInputTokens", alias = "cached_input_tokens")]
    pub cached_input_tokens: Option<u64>,
    #[serde(rename = "totalTokens", alias = "total_tokens")]
    pub total_tokens: Option<u64>,
    #[serde(rename = "totalCost", alias = "total_cost")]
    pub total_cost: Option<f64>,
    #[serde(rename = "costUSD", alias = "cost_usd")]
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsageReport {
    pub daily: Vec<DailyUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CcusageProvider {
    Claude,
    Codex,
}

impl std::fmt::Display for CcusageProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CcusageProvider::Claude => write!(f, "claude"),
            CcusageProvider::Codex => write!(f, "codex"),
        }
    }
}

pub struct CcusageClient {
    provider: CcusageProvider,
    home_path: Option<PathBuf>,
    cache: HashMap<String, (DailyUsageReport, Instant)>,
    cache_ttl: Duration,
}

impl CcusageClient {
    pub fn new(provider: CcusageProvider) -> Self {
        Self {
            provider,
            home_path: None,
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(300),
        }
    }

    pub fn with_home_path(mut self, path: PathBuf) -> Self {
        self.home_path = Some(path);
        self
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    pub fn query(&mut self, since: &str, until: Option<&str>) -> Result<DailyUsageReport> {
        let cache_key = format!("{}:{}:{}", self.provider, since, until.unwrap_or("now"));

        if let Some((report, fetched_at)) = self.cache.get(&cache_key) {
            if fetched_at.elapsed() < self.cache_ttl {
                tracing::debug!("ccusage cache hit for {}", cache_key);
                return Ok(report.clone());
            }
        }

        let runner = find_runner()?;

        let package = match self.provider {
            CcusageProvider::Claude => "ccusage@18.0.10",
            CcusageProvider::Codex => "@ccusage/codex@18.0.10",
        };

        let mut args = vec![
            "dlx".to_string(),
            package.to_string(),
            "--".to_string(),
            "--since".to_string(),
            since.to_string(),
        ];

        if let Some(u) = until {
            args.push("--until".to_string());
            args.push(u.to_string());
        }

        if let Some(ref home) = self.home_path {
            args.push("--home".to_string());
            args.push(home.to_string_lossy().to_string());
        }

        tracing::info!("Running ccusage: {} {}", runner, args.join(" "));

        let output = std::process::Command::new(&runner)
            .args(&args)
            .output()
            .map_err(CcusageError::IoError)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CcusageError::RunnerFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let report = parse_ccusage_output(&stdout)?;

        self.cache
            .insert(cache_key, (report.clone(), Instant::now()));

        Ok(report)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

fn find_runner() -> Result<String> {
    let runners = ["bun", "pnpm", "yarn", "npm", "npx"];
    for runner in &runners {
        if which::which(runner).is_ok() {
            return Ok(runner.to_string());
        }
    }
    Err(CcusageError::NoRunner)
}

fn parse_ccusage_output(output: &str) -> Result<DailyUsageReport> {
    let report: DailyUsageReport = serde_json::from_str(output).map_err(|e| {
        CcusageError::ParseError(format!("Failed to parse JSON: {}\nOutput: {}", e, output))
    })?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ccusage_output_camel_case() {
        let json = r#"{
            "daily": [
                {
                    "date": "2026-04-01",
                    "inputTokens": 50000,
                    "outputTokens": 25000,
                    "cacheCreationTokens": 10000,
                    "cacheReadTokens": 5000,
                    "totalTokens": 90000,
                    "totalCost": 2.50
                }
            ]
        }"#;
        let report = parse_ccusage_output(json).unwrap();
        assert_eq!(report.daily.len(), 1);
        assert_eq!(report.daily[0].date, "2026-04-01");
        assert_eq!(report.daily[0].total_tokens, Some(90000));
        assert_eq!(report.daily[0].total_cost, Some(2.50));
    }

    #[test]
    fn test_parse_ccusage_output_snake_case() {
        let json = r#"{
            "daily": [
                {
                    "date": "2026-04-01",
                    "input_tokens": 50000,
                    "output_tokens": 25000,
                    "total_tokens": 75000,
                    "cost_usd": 1.25
                }
            ]
        }"#;
        let report = parse_ccusage_output(json).unwrap();
        assert_eq!(report.daily.len(), 1);
        assert_eq!(report.daily[0].input_tokens, Some(50000));
        assert_eq!(report.daily[0].cost_usd, Some(1.25));
    }

    #[test]
    fn test_parse_empty_output() {
        let json = r#"{"daily": []}"#;
        let report = parse_ccusage_output(json).unwrap();
        assert_eq!(report.daily.len(), 0);
    }

    #[test]
    fn test_ccusage_provider_display() {
        assert_eq!(CcusageProvider::Claude.to_string(), "claude");
        assert_eq!(CcusageProvider::Codex.to_string(), "codex");
    }

    #[test]
    fn test_cache_ttl_setting() {
        let client =
            CcusageClient::new(CcusageProvider::Claude).with_cache_ttl(Duration::from_millis(1));
        assert_eq!(client.cache_ttl, Duration::from_millis(1));
    }

    #[test]
    fn test_home_path_setting() {
        let client =
            CcusageClient::new(CcusageProvider::Claude).with_home_path(PathBuf::from("/tmp/test"));
        assert_eq!(client.home_path, Some(PathBuf::from("/tmp/test")));
    }
}
