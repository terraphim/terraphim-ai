use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use terraphim_types::shared_learning::SuggestionStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionMetricsEntry {
    pub id: String,
    pub status: SuggestionStatus,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub title: String,
}

#[derive(Debug, Clone, Default)]
pub struct SuggestionMetricsSummary {
    pub total: usize,
    pub approved: usize,
    pub rejected: usize,
    pub pending: usize,
    pub approval_rate: f64,
}

pub struct SuggestionMetrics {
    pub metrics_path: PathBuf,
}

impl SuggestionMetrics {
    pub fn new(metrics_path: PathBuf) -> Self {
        Self { metrics_path }
    }

    pub fn default_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".terraphim")
            .join("suggestion-metrics.jsonl")
    }

    pub fn append(&self, entry: SuggestionMetricsEntry) -> std::io::Result<()> {
        if let Some(parent) = self.metrics_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.metrics_path)?;
        let mut line = serde_json::to_string(&entry)?;
        line.push('\n');
        file.write_all(line.as_bytes())
    }

    pub fn read_recent(&self, limit: usize) -> std::io::Result<Vec<SuggestionMetricsEntry>> {
        if !self.metrics_path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.metrics_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if let Ok(entry) = serde_json::from_str::<SuggestionMetricsEntry>(&line) {
                entries.push(entry);
            }
        }

        let skip = entries.len().saturating_sub(limit);
        Ok(entries.into_iter().skip(skip).collect())
    }

    pub fn summary(&self) -> std::io::Result<SuggestionMetricsSummary> {
        let all = self.read_recent(usize::MAX)?;
        let mut s = SuggestionMetricsSummary {
            total: all.len(),
            ..Default::default()
        };
        for entry in &all {
            match entry.status {
                SuggestionStatus::Approved => s.approved += 1,
                SuggestionStatus::Rejected => s.rejected += 1,
                SuggestionStatus::Pending => s.pending += 1,
            }
        }
        let decided = s.approved + s.rejected;
        s.approval_rate = if decided > 0 {
            s.approved as f64 / decided as f64
        } else {
            0.0
        };
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_entry(status: SuggestionStatus, title: &str) -> SuggestionMetricsEntry {
        SuggestionMetricsEntry {
            id: format!("id-{}", title),
            status,
            confidence: 0.5,
            timestamp: Utc::now(),
            title: title.to_string(),
        }
    }

    #[test]
    fn test_metrics_append_and_read() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("metrics.jsonl");
        let metrics = SuggestionMetrics::new(path.clone());

        metrics
            .append(make_entry(SuggestionStatus::Approved, "first"))
            .unwrap();
        metrics
            .append(make_entry(SuggestionStatus::Rejected, "second"))
            .unwrap();

        let entries = metrics.read_recent(10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].status, SuggestionStatus::Approved);
        assert_eq!(entries[1].status, SuggestionStatus::Rejected);
    }

    #[test]
    fn test_metrics_summary() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("metrics.jsonl");
        let metrics = SuggestionMetrics::new(path);

        metrics
            .append(make_entry(SuggestionStatus::Approved, "a"))
            .unwrap();
        metrics
            .append(make_entry(SuggestionStatus::Approved, "b"))
            .unwrap();
        metrics
            .append(make_entry(SuggestionStatus::Rejected, "c"))
            .unwrap();
        metrics
            .append(make_entry(SuggestionStatus::Pending, "d"))
            .unwrap();

        let summary = metrics.summary().unwrap();
        assert_eq!(summary.total, 4);
        assert_eq!(summary.approved, 2);
        assert_eq!(summary.rejected, 1);
        assert_eq!(summary.pending, 1);
        assert!((summary.approval_rate - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_metrics_read_recent_limit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("metrics.jsonl");
        let metrics = SuggestionMetrics::new(path);

        for i in 0..5 {
            metrics
                .append(make_entry(SuggestionStatus::Pending, &format!("e{}", i)))
                .unwrap();
        }

        let entries = metrics.read_recent(3).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].title, "e2");
    }

    #[test]
    fn test_metrics_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("metrics.jsonl");
        let metrics = SuggestionMetrics::new(path);

        let entries = metrics.read_recent(10).unwrap();
        assert!(entries.is_empty());

        let summary = metrics.summary().unwrap();
        assert_eq!(summary.total, 0);
    }
}
