//! Dispatch outcome classification and durable persistence.
//!
//! After each agent dispatch completes, the orchestrator classifies the
//! `WorkerOutcome` into one of five canonical `DispatchOutcomeKind` values,
//! writes the entry to a JSONL file for durability, and keeps the last N
//! entries in an in-memory ring buffer for the `/meta/dispatch-stats` API.

use crate::runner::session::WorkerOutcome;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Five canonical dispatch outcome classifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchOutcomeKind {
    /// Agent ran and produced non-empty output (total_tokens > 0).
    Success,
    /// Agent exited cleanly but produced no token output.
    EmptySuccess,
    /// Agent failed for a general reason.
    Error,
    /// Agent was rate-limited by the upstream LLM provider.
    RateLimit,
    /// Agent exceeded the configured wall-time timeout.
    WallTimeExceeded,
}

impl fmt::Display for DispatchOutcomeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::EmptySuccess => write!(f, "empty_success"),
            Self::Error => write!(f, "error"),
            Self::RateLimit => write!(f, "rate_limit"),
            Self::WallTimeExceeded => write!(f, "wall_time_exceeded"),
        }
    }
}

/// A single dispatch outcome record persisted to JSONL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchOutcomeEntry {
    pub ts: DateTime<Utc>,
    pub issue_id: String,
    pub identifier: String,
    pub outcome: DispatchOutcomeKind,
    pub elapsed_ms: u64,
    pub turn_count: u32,
    pub total_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Classify a `WorkerOutcome` into a `DispatchOutcomeEntry`.
pub fn classify(
    outcome: &WorkerOutcome,
    elapsed_ms: u64,
    issue_id: &str,
    identifier: &str,
    ts: DateTime<Utc>,
) -> DispatchOutcomeEntry {
    match outcome {
        WorkerOutcome::Normal { turn_count, tokens } => {
            let kind = if tokens.total_tokens == 0 {
                DispatchOutcomeKind::EmptySuccess
            } else {
                DispatchOutcomeKind::Success
            };
            DispatchOutcomeEntry {
                ts,
                issue_id: issue_id.to_string(),
                identifier: identifier.to_string(),
                outcome: kind,
                elapsed_ms,
                turn_count: *turn_count,
                total_tokens: tokens.total_tokens,
                reason: None,
            }
        }
        WorkerOutcome::Failed {
            reason,
            turn_count,
            tokens,
        } => {
            let lower = reason.to_lowercase();
            let kind = if lower.contains("rate limit")
                || lower.contains("rate_limit")
                || lower.contains("too many requests")
                || lower.contains("429")
            {
                DispatchOutcomeKind::RateLimit
            } else if lower.contains("timeout")
                || lower.contains("timed out")
                || lower.contains("wall time")
            {
                DispatchOutcomeKind::WallTimeExceeded
            } else {
                DispatchOutcomeKind::Error
            };
            DispatchOutcomeEntry {
                ts,
                issue_id: issue_id.to_string(),
                identifier: identifier.to_string(),
                outcome: kind,
                elapsed_ms,
                turn_count: *turn_count,
                total_tokens: tokens.total_tokens,
                reason: Some(reason.clone()),
            }
        }
    }
}

/// Ring-buffer of recent dispatch outcomes with optional JSONL persistence.
pub struct OutcomeStore {
    entries: RwLock<VecDeque<DispatchOutcomeEntry>>,
    capacity: usize,
    jsonl_path: Option<PathBuf>,
}

impl OutcomeStore {
    /// Create an outcome store with the given ring-buffer capacity.
    ///
    /// If `jsonl_path` is `Some`, each entry is also appended to that file in
    /// JSONL format as a durable fallback (best-effort; write errors are ignored).
    pub fn new(capacity: usize, jsonl_path: Option<PathBuf>) -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
            jsonl_path,
        }
    }

    /// Default path: `~/.terraphim/meta_coordinator_outcomes.jsonl`.
    pub fn default_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".terraphim").join("meta_coordinator_outcomes.jsonl"))
    }

    /// Push a new entry into the store and append it to the JSONL file.
    pub async fn push(&self, entry: DispatchOutcomeEntry) {
        // Best-effort JSONL append
        if let Some(path) = &self.jsonl_path {
            if let Ok(line) = serde_json::to_string(&entry) {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    let _ = writeln!(f, "{line}");
                }
            }
        }

        let mut entries = self.entries.write().await;
        if entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// Return the most recent `limit` entries (newest first).
    pub async fn recent(&self, limit: usize) -> Vec<DispatchOutcomeEntry> {
        let entries = self.entries.read().await;
        entries.iter().rev().take(limit).cloned().collect()
    }

    /// Return all entries (oldest first).
    pub async fn all(&self) -> Vec<DispatchOutcomeEntry> {
        let entries = self.entries.read().await;
        entries.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::protocol::TokenCounts;

    fn make_normal(total_tokens: u64) -> WorkerOutcome {
        WorkerOutcome::Normal {
            turn_count: 1,
            tokens: TokenCounts {
                input_tokens: total_tokens / 2,
                output_tokens: total_tokens / 2,
                total_tokens,
            },
        }
    }

    fn make_failed(reason: &str) -> WorkerOutcome {
        WorkerOutcome::Failed {
            reason: reason.to_string(),
            turn_count: 1,
            tokens: TokenCounts::default(),
        }
    }

    #[test]
    fn normal_with_tokens_is_success() {
        let outcome = classify(&make_normal(100), 500, "id1", "MT-1", Utc::now());
        assert_eq!(outcome.outcome, DispatchOutcomeKind::Success);
        assert_eq!(outcome.total_tokens, 100);
    }

    #[test]
    fn normal_with_zero_tokens_is_empty_success() {
        let outcome = classify(&make_normal(0), 200, "id1", "MT-1", Utc::now());
        assert_eq!(outcome.outcome, DispatchOutcomeKind::EmptySuccess);
    }

    #[test]
    fn failed_timeout_is_wall_time_exceeded() {
        let outcome = classify(
            &make_failed("session timeout after 300s"),
            300_000,
            "id1",
            "MT-1",
            Utc::now(),
        );
        assert_eq!(outcome.outcome, DispatchOutcomeKind::WallTimeExceeded);
    }

    #[test]
    fn failed_rate_limit_classified() {
        for msg in &["rate limit exceeded", "too many requests", "HTTP 429"] {
            let outcome = classify(&make_failed(msg), 1000, "id1", "MT-1", Utc::now());
            assert_eq!(
                outcome.outcome,
                DispatchOutcomeKind::RateLimit,
                "expected RateLimit for '{msg}'"
            );
        }
    }

    #[test]
    fn failed_general_is_error() {
        let outcome = classify(
            &make_failed("claude exited with code 1"),
            1000,
            "id1",
            "MT-1",
            Utc::now(),
        );
        assert_eq!(outcome.outcome, DispatchOutcomeKind::Error);
        assert_eq!(outcome.reason.as_deref(), Some("claude exited with code 1"));
    }

    #[tokio::test]
    async fn ring_buffer_caps_at_capacity() {
        let store = OutcomeStore::new(3, None);
        let ts = Utc::now();

        for i in 0u32..5 {
            store
                .push(DispatchOutcomeEntry {
                    ts,
                    issue_id: format!("id{i}"),
                    identifier: format!("MT-{i}"),
                    outcome: DispatchOutcomeKind::Success,
                    elapsed_ms: 100,
                    turn_count: 1,
                    total_tokens: 50,
                    reason: None,
                })
                .await;
        }

        let all = store.all().await;
        assert_eq!(all.len(), 3, "ring buffer should cap at capacity");
        // Oldest (id0, id1) should be evicted; id2/id3/id4 remain
        assert_eq!(all[0].issue_id, "id2");
        assert_eq!(all[2].issue_id, "id4");
    }

    #[tokio::test]
    async fn recent_returns_newest_first() {
        let store = OutcomeStore::new(10, None);
        let ts = Utc::now();

        for i in 0u32..3 {
            store
                .push(DispatchOutcomeEntry {
                    ts,
                    issue_id: format!("id{i}"),
                    identifier: format!("MT-{i}"),
                    outcome: DispatchOutcomeKind::Success,
                    elapsed_ms: 100,
                    turn_count: 1,
                    total_tokens: 50,
                    reason: None,
                })
                .await;
        }

        let recent = store.recent(2).await;
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].issue_id, "id2"); // newest first
        assert_eq!(recent[1].issue_id, "id1");
    }

    #[tokio::test]
    async fn jsonl_written_to_temp_file() {
        use std::io::BufRead;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("outcomes.jsonl");
        let store = OutcomeStore::new(100, Some(path.clone()));
        let ts = Utc::now();

        store
            .push(DispatchOutcomeEntry {
                ts,
                issue_id: "id1".into(),
                identifier: "MT-1".into(),
                outcome: DispatchOutcomeKind::Success,
                elapsed_ms: 500,
                turn_count: 2,
                total_tokens: 100,
                reason: None,
            })
            .await;

        store
            .push(DispatchOutcomeEntry {
                ts,
                issue_id: "id2".into(),
                identifier: "MT-2".into(),
                outcome: DispatchOutcomeKind::EmptySuccess,
                elapsed_ms: 200,
                turn_count: 1,
                total_tokens: 0,
                reason: None,
            })
            .await;

        let file = std::fs::File::open(&path).unwrap();
        let lines: Vec<String> = std::io::BufReader::new(file)
            .lines()
            .map(|l| l.unwrap())
            .collect();

        assert_eq!(lines.len(), 2, "two JSONL lines should be written");

        let entry0: DispatchOutcomeEntry = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(entry0.outcome, DispatchOutcomeKind::Success);

        let entry1: DispatchOutcomeEntry = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(entry1.outcome, DispatchOutcomeKind::EmptySuccess);
    }
}
