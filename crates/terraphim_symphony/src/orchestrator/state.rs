//! Orchestrator runtime state.
//!
//! Single authoritative in-memory state owned by the orchestrator.
//! All scheduling decisions (dispatch, retry, reconciliation) are
//! serialised through this state.

use crate::runner::TokenTotals;
use crate::runner::protocol::TokenCounts;
use crate::tracker::Issue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The orchestrator's runtime state.
pub struct OrchestratorRuntimeState {
    /// Current effective poll interval.
    pub poll_interval_ms: u64,
    /// Current effective global concurrency limit.
    pub max_concurrent_agents: usize,
    /// Currently running issues.
    pub running: HashMap<String, RunningEntry>,
    /// Issue IDs that are claimed (running or retry-queued).
    pub claimed: HashSet<String>,
    /// Retry queue.
    pub retry_attempts: HashMap<String, RetryEntry>,
    /// Completed issue IDs (bookkeeping only).
    pub completed: HashSet<String>,
    /// Aggregate token totals.
    pub codex_totals: TokenTotals,
    /// Latest rate-limit snapshot from agent events.
    pub codex_rate_limits: Option<serde_json::Value>,
}

impl OrchestratorRuntimeState {
    /// Create a new empty state with the given configuration.
    pub fn new(poll_interval_ms: u64, max_concurrent_agents: usize) -> Self {
        Self {
            poll_interval_ms,
            max_concurrent_agents,
            running: HashMap::new(),
            claimed: HashSet::new(),
            retry_attempts: HashMap::new(),
            completed: HashSet::new(),
            codex_totals: TokenTotals::default(),
            codex_rate_limits: None,
        }
    }

    /// Number of available global dispatch slots.
    pub fn available_slots(&self) -> usize {
        self.max_concurrent_agents
            .saturating_sub(self.running.len())
    }

    /// Count running issues in a given state (case-insensitive).
    pub fn running_count_by_state(&self, state: &str) -> usize {
        self.running
            .values()
            .filter(|entry| entry.issue.state.eq_ignore_ascii_case(state))
            .count()
    }

    /// Check if an issue ID is claimed (running or retry-queued).
    pub fn is_claimed(&self, issue_id: &str) -> bool {
        self.claimed.contains(issue_id)
    }

    /// Add accumulated runtime seconds from a completed session.
    pub fn add_runtime_seconds(&mut self, seconds: f64) {
        self.codex_totals.seconds_running += seconds;
    }

    /// Add token deltas from a completed session.
    pub fn add_token_totals(&mut self, tokens: &TokenCounts) {
        self.codex_totals.input_tokens += tokens.input_tokens;
        self.codex_totals.output_tokens += tokens.output_tokens;
        self.codex_totals.total_tokens += tokens.total_tokens;
    }
}

/// State tracked for a running issue.
pub struct RunningEntry {
    /// The normalised issue being worked on.
    pub issue: Issue,
    /// Live session metadata.
    pub session: LiveSession,
    /// When this run started.
    pub started_at: DateTime<Utc>,
    /// Retry attempt number (`None` for first dispatch).
    pub retry_attempt: Option<u32>,
    /// Handle to the spawned worker task.
    pub worker_handle: tokio::task::JoinHandle<crate::runner::WorkerOutcome>,
}

/// Metadata for a live coding-agent session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveSession {
    /// Session ID (`<thread_id>-<turn_id>`).
    pub session_id: String,
    /// Thread ID from the coding agent.
    pub thread_id: String,
    /// Current turn ID.
    pub turn_id: String,
    /// PID of the app-server process.
    pub pid: Option<u32>,
    /// Last event type seen.
    pub last_event: Option<String>,
    /// Timestamp of last event.
    pub last_timestamp: Option<DateTime<Utc>>,
    /// Summary of last event message.
    pub last_message: String,
    /// Token counts for this session.
    pub tokens: TokenCounts,
    /// Number of turns started.
    pub turn_count: u32,
}

/// A scheduled retry.
pub struct RetryEntry {
    /// Issue ID.
    pub issue_id: String,
    /// Human-readable identifier (for logs/status).
    pub identifier: String,
    /// Retry attempt number (1-based).
    pub attempt: u32,
    /// When the retry timer should fire (monotonic).
    pub due_at: tokio::time::Instant,
    /// Timer task handle.
    pub timer_handle: tokio::task::JoinHandle<()>,
    /// Error that caused the retry.
    pub error: Option<String>,
}

/// Compute the retry delay for a given attempt.
///
/// - Normal continuation retries (attempt 1 after clean exit): 1000ms.
/// - Failure-driven retries: `min(10000 * 2^(attempt-1), max_backoff_ms)`.
pub fn retry_delay_ms(attempt: u32, max_backoff_ms: u64, is_continuation: bool) -> u64 {
    if is_continuation && attempt <= 1 {
        return 1_000;
    }
    let base: u64 = 10_000;
    let exp = attempt.saturating_sub(1).min(20); // cap exponent to avoid overflow
    let delay = base.saturating_mul(1u64 << exp);
    delay.min(max_backoff_ms)
}

/// Snapshot of the orchestrator state for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub generated_at: DateTime<Utc>,
    pub counts: SnapshotCounts,
    pub running: Vec<RunningSnapshot>,
    pub retrying: Vec<RetrySnapshot>,
    pub codex_totals: TokenTotals,
    pub rate_limits: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotCounts {
    pub running: usize,
    pub retrying: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningSnapshot {
    pub issue_id: String,
    pub issue_identifier: String,
    pub state: String,
    pub session_id: String,
    pub turn_count: u32,
    pub last_event: Option<String>,
    pub last_message: String,
    pub started_at: DateTime<Utc>,
    pub last_event_at: Option<DateTime<Utc>>,
    pub tokens: TokenCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySnapshot {
    pub issue_id: String,
    pub issue_identifier: String,
    pub attempt: u32,
    pub due_at: String,
    pub error: Option<String>,
}

impl OrchestratorRuntimeState {
    /// Create a snapshot of the current state for observability.
    /// The `now` parameter enables deterministic testing of elapsed time calculations.
    pub fn snapshot(&self, now: DateTime<Utc>) -> StateSnapshot {
        let running: Vec<RunningSnapshot> = self
            .running
            .iter()
            .map(|(id, entry)| RunningSnapshot {
                issue_id: id.clone(),
                issue_identifier: entry.issue.identifier.clone(),
                state: entry.issue.state.clone(),
                session_id: entry.session.session_id.clone(),
                turn_count: entry.session.turn_count,
                last_event: entry.session.last_event.clone(),
                last_message: entry.session.last_message.clone(),
                started_at: entry.started_at,
                last_event_at: entry.session.last_timestamp,
                tokens: entry.session.tokens.clone(),
            })
            .collect();

        let retrying: Vec<RetrySnapshot> = self
            .retry_attempts
            .values()
            .map(|entry| RetrySnapshot {
                issue_id: entry.issue_id.clone(),
                issue_identifier: entry.identifier.clone(),
                attempt: entry.attempt,
                due_at: format!("{:?}", entry.due_at),
                error: entry.error.clone(),
            })
            .collect();

        // Calculate live runtime including active sessions
        let mut totals = self.codex_totals.clone();
        for entry in self.running.values() {
            let elapsed = (now - entry.started_at).num_milliseconds().max(0) as f64 / 1000.0;
            totals.seconds_running += elapsed;
        }

        StateSnapshot {
            generated_at: now,
            counts: SnapshotCounts {
                running: self.running.len(),
                retrying: self.retry_attempts.len(),
            },
            running,
            retrying,
            codex_totals: totals,
            rate_limits: self.codex_rate_limits.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_retry_delay() {
        assert_eq!(retry_delay_ms(1, 300_000, true), 1_000);
    }

    #[test]
    fn failure_retry_backoff() {
        // attempt 1: 10_000
        assert_eq!(retry_delay_ms(1, 300_000, false), 10_000);
        // attempt 2: 20_000
        assert_eq!(retry_delay_ms(2, 300_000, false), 20_000);
        // attempt 3: 40_000
        assert_eq!(retry_delay_ms(3, 300_000, false), 40_000);
        // attempt 4: 80_000
        assert_eq!(retry_delay_ms(4, 300_000, false), 80_000);
    }

    #[test]
    fn retry_backoff_capped() {
        // Large attempt should be capped
        assert_eq!(retry_delay_ms(20, 300_000, false), 300_000);
    }

    #[test]
    fn available_slots() {
        let state = OrchestratorRuntimeState::new(30_000, 5);
        assert_eq!(state.available_slots(), 5);
    }

    #[test]
    fn empty_state_snapshot() {
        let state = OrchestratorRuntimeState::new(30_000, 10);
        let snap = state.snapshot(Utc::now());
        assert_eq!(snap.counts.running, 0);
        assert_eq!(snap.counts.retrying, 0);
        assert!(snap.rate_limits.is_none());
    }
}
