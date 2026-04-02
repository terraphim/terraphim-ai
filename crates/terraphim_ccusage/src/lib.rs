//! ccusage integration for Terraphim AI.
//!
//! This crate provides integration with the ccusage tool for tracking
//! daily usage reports across AI providers.
//!
//! # Architecture
//!
//! - [`CcusageClient`] struct: Client for interacting with ccusage
//! - [`DailyUsageReport`] model: Aggregated daily usage data

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use terraphim_usage::{ProviderId, ProviderUsage};

/// Client for interacting with ccusage.
#[derive(Debug, Clone)]
pub struct CcusageClient {
    /// The provider identifier for this ccusage instance
    pub provider: ProviderId,
    /// Path to ccusage home directory (contains usage data)
    pub home_path: PathBuf,
}

impl CcusageClient {
    /// Creates a new CcusageClient instance.
    pub fn new(provider: impl Into<ProviderId>, home_path: impl Into<PathBuf>) -> Self {
        Self {
            provider: provider.into(),
            home_path: home_path.into(),
        }
    }
}

/// Daily aggregated usage report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyUsageReport {
    /// The date for this report
    pub date: NaiveDate,
    /// Provider usage for this day
    pub usages: Vec<ProviderUsage>,
    /// Total lines across all providers
    pub total_lines: u64,
    /// When this report was generated
    pub generated_at: DateTime<Utc>,
}

impl DailyUsageReport {
    /// Creates a new empty DailyUsageReport for the given date.
    pub fn new(date: NaiveDate) -> Self {
        Self {
            date,
            usages: Vec::new(),
            total_lines: 0,
            generated_at: Utc::now(),
        }
    }

    /// Adds a provider usage entry to the report.
    pub fn add_usage(&mut self, usage: ProviderUsage) {
        self.total_lines += usage.lines;
        self.usages.push(usage);
    }
}

/// Errors specific to ccusage operations.
#[derive(Debug, thiserror::Error)]
pub enum CcusageError {
    /// Home path does not exist or is not accessible.
    #[error("home path error: {0}")]
    HomePath(String),

    /// Failed to read usage data.
    #[error("read error: {0}")]
    Read(String),

    /// Data format is invalid.
    #[error("format error: {0}")]
    Format(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn ccusage_client_new() {
        let client = CcusageClient::new("test-provider", "/home/user/.ccusage");

        assert_eq!(client.provider.0, "test-provider");
        assert_eq!(client.home_path, PathBuf::from("/home/user/.ccusage"));
    }

    #[test]
    fn daily_usage_report_new() {
        let today = Local::now().date_naive();
        let report = DailyUsageReport::new(today);

        assert_eq!(report.date, today);
        assert!(report.usages.is_empty());
        assert_eq!(report.total_lines, 0);
    }

    #[test]
    fn daily_usage_report_add_usage() {
        let today = Local::now().date_naive();
        let mut report = DailyUsageReport::new(today);

        let usage1 = ProviderUsage::new("provider-1", "Provider One", 100);
        let usage2 = ProviderUsage::new("provider-2", "Provider Two", 200);

        report.add_usage(usage1);
        report.add_usage(usage2);

        assert_eq!(report.usages.len(), 2);
        assert_eq!(report.total_lines, 300);
    }
}
