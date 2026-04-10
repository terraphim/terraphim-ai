//! Telemetry types for live model-selection control plane.
//!
//! Captures per-completion events from CLI tool output (opencode/claude JSON streams),
//! stores them durably via terraphim_persistence, and exposes snapshots for routing policy.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A single model completion event parsed from CLI tool output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletionEvent {
    /// Model identifier (e.g. "zai-coding-plan/glm-5.1").
    pub model: String,
    /// Session ID (opencode session or Gitea issue reference).
    pub session_id: String,
    /// Timestamp of completion.
    pub completed_at: DateTime<Utc>,
    /// Wall-clock latency in milliseconds.
    pub latency_ms: u64,
    /// Whether the completion succeeded.
    pub success: bool,
    /// Token breakdown.
    pub tokens: TokenBreakdown,
    /// Estimated cost in USD (0.0 if unavailable).
    pub cost_usd: f64,
    /// Error message if failed, e.g. "weekly session limit reached".
    pub error: Option<String>,
}

/// Token breakdown from a completion event.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TokenBreakdown {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// A snapshot of model performance for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformanceSnapshot {
    /// Model identifier.
    pub model: String,
    /// Successful completions in the rolling window.
    pub successful_completions: u64,
    /// Failed completions in the rolling window.
    pub failed_completions: u64,
    /// Window duration in seconds.
    pub window_secs: u64,
    /// Throughput: successful completions / window_secs.
    pub throughput: f64,
    /// Average latency in ms over successful completions.
    pub avg_latency_ms: f64,
    /// Success rate (0.0-1.0).
    pub success_rate: f64,
    /// Timestamp of the most recent event.
    pub last_event_at: Option<DateTime<Utc>>,
    /// Whether a subscription limit error was detected for this model.
    pub subscription_limit_reached: bool,
    /// Time when the subscription limit flag should expire.
    pub subscription_limit_expires_at: Option<DateTime<Utc>>,
}

impl ModelPerformanceSnapshot {
    pub fn empty(model: &str, window_secs: u64) -> Self {
        Self {
            model: model.to_string(),
            successful_completions: 0,
            failed_completions: 0,
            window_secs,
            throughput: 0.0,
            avg_latency_ms: 0.0,
            success_rate: 0.0,
            last_event_at: None,
            subscription_limit_reached: false,
            subscription_limit_expires_at: None,
        }
    }

    pub fn is_stale(&self, max_staleness_secs: u64) -> bool {
        match self.last_event_at {
            None => true,
            Some(ts) => {
                let now = Utc::now();
                (now - ts).num_seconds().unsigned_abs() > max_staleness_secs
            }
        }
    }

    pub fn is_subscription_limited(&self) -> bool {
        if !self.subscription_limit_reached {
            return false;
        }
        if let Some(expires) = self.subscription_limit_expires_at {
            Utc::now() < expires
        } else {
            true
        }
    }
}

/// Usage snapshot for a session or rolling window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSnapshot {
    /// Session or window identifier.
    pub id: String,
    /// Start of the window.
    pub window_start: DateTime<Utc>,
    /// End of the window.
    pub window_end: DateTime<Utc>,
    /// Per-model token usage.
    pub by_model: HashMap<String, ModelUsage>,
    /// Aggregate totals.
    pub totals: ModelUsage,
}

/// Per-model usage totals.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
    pub message_count: u64,
}

impl ModelUsage {
    pub fn merge(&mut self, other: &ModelUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
        self.cost_usd += other.cost_usd;
        self.message_count += other.message_count;
    }
}

/// Serializable snapshot of telemetry state for persistence across restarts.
///
/// Contains aggregated model performances (not individual events) to keep
/// the persisted payload small and relevant for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySummary {
    /// Unique identifier for this summary (always "telemetry_summary").
    pub id: String,
    /// Aggregated model performances at time of export.
    pub model_performances: Vec<ModelPerformanceSnapshot>,
    /// Rolling window duration in seconds.
    pub window_secs: u64,
    /// Timestamp when this summary was exported.
    pub exported_at: DateTime<Utc>,
}

/// In-memory telemetry store backed by terraphim_persistence.
///
/// Stores rolling completion events and exposes performance/usage snapshots
/// for routing policy consumption. Uses terraphim_persistence Persistable
/// for durable storage across restarts.
#[derive(Debug, Clone)]
pub struct TelemetryStore {
    inner: Arc<RwLock<TelemetryStoreInner>>,
}

#[derive(Debug)]
struct TelemetryStoreInner {
    /// Rolling window of completion events, keyed by model.
    events: HashMap<String, Vec<CompletionEvent>>,
    /// Subscription limit flags, keyed by model.
    subscription_limits: HashMap<String, DateTime<Utc>>,
    /// Rolling window duration in seconds.
    window_secs: u64,
    /// Subscription limit TTL in seconds (default: 1 hour).
    subscription_limit_ttl_secs: u64,
}

impl TelemetryStore {
    pub fn new(window_secs: u64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(TelemetryStoreInner {
                events: HashMap::new(),
                subscription_limits: HashMap::new(),
                window_secs,
                subscription_limit_ttl_secs: 3600,
            })),
        }
    }

    pub fn with_subscription_limit_ttl(self, ttl_secs: u64) -> Self {
        let window_secs = self.inner.blocking_read().window_secs;
        Self {
            inner: Arc::new(RwLock::new(TelemetryStoreInner {
                events: HashMap::new(),
                subscription_limits: HashMap::new(),
                window_secs,
                subscription_limit_ttl_secs: ttl_secs,
            })),
        }
    }

    /// Record a completion event from a parsed CLI output stream.
    pub async fn record(&self, event: CompletionEvent) {
        let mut inner = self.inner.write().await;
        let now = Utc::now();
        let cutoff = now - chrono::Duration::seconds(inner.window_secs as i64);

        if let Some(ref error) = event.error {
            if is_subscription_limit_error(error) {
                let expires =
                    now + chrono::Duration::seconds(inner.subscription_limit_ttl_secs as i64);
                inner
                    .subscription_limits
                    .insert(event.model.clone(), expires);
            }
        }

        let events = inner.events.entry(event.model.clone()).or_default();
        events.push(event);

        events.retain(|e| e.completed_at > cutoff);
    }

    /// Get performance snapshot for a specific model.
    pub async fn model_performance(&self, model: &str) -> ModelPerformanceSnapshot {
        let inner = self.inner.read().await;
        let events = inner.events.get(model);

        let (successful, failed, avg_latency) = match events {
            None => (0u64, 0u64, 0.0),
            Some(evts) => {
                let mut success_count = 0u64;
                let mut fail_count = 0u64;
                let mut latency_sum = 0.0f64;
                let mut latency_count = 0u64;

                for e in evts {
                    if e.success {
                        success_count += 1;
                        latency_sum += e.latency_ms as f64;
                        latency_count += 1;
                    } else {
                        fail_count += 1;
                    }
                }

                let avg = if latency_count > 0 {
                    latency_sum / latency_count as f64
                } else {
                    0.0
                };

                (success_count, fail_count, avg)
            }
        };

        let total = successful + failed;
        let success_rate = if total > 0 {
            successful as f64 / total as f64
        } else {
            0.0
        };
        let throughput = if inner.window_secs > 0 {
            successful as f64 / inner.window_secs as f64
        } else {
            0.0
        };

        let last_event_at = events.and_then(|evts| evts.last().map(|e| e.completed_at));

        let subscription_limit_reached = inner
            .subscription_limits
            .get(model)
            .map(|expires| Utc::now() < *expires)
            .unwrap_or(false);

        let subscription_limit_expires_at = inner.subscription_limits.get(model).copied();

        ModelPerformanceSnapshot {
            model: model.to_string(),
            successful_completions: successful,
            failed_completions: failed,
            window_secs: inner.window_secs,
            throughput,
            avg_latency_ms: avg_latency,
            success_rate,
            last_event_at,
            subscription_limit_reached,
            subscription_limit_expires_at,
        }
    }

    /// Get usage snapshot for a specific session.
    pub async fn session_usage(&self, session_id: &str) -> UsageSnapshot {
        let inner = self.inner.read().await;
        let now = Utc::now();
        let mut by_model: HashMap<String, ModelUsage> = HashMap::new();

        for events in inner.events.values() {
            for e in events {
                if e.session_id == session_id {
                    let usage = by_model.entry(e.model.clone()).or_default();
                    usage.input_tokens += e.tokens.input;
                    usage.output_tokens += e.tokens.output;
                    usage.total_tokens += e.tokens.total;
                    usage.cost_usd += e.cost_usd;
                    usage.message_count += 1;
                }
            }
        }

        let mut totals = ModelUsage::default();
        for usage in by_model.values() {
            totals.merge(usage);
        }

        UsageSnapshot {
            id: session_id.to_string(),
            window_start: now - chrono::Duration::seconds(inner.window_secs as i64),
            window_end: now,
            by_model,
            totals,
        }
    }

    /// Get all known model names.
    pub async fn known_models(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.events.keys().cloned().collect()
    }

    /// Get performance snapshots for all known models.
    pub async fn all_model_performances(&self) -> Vec<ModelPerformanceSnapshot> {
        let models = self.known_models().await;
        let mut snapshots = Vec::with_capacity(models.len());
        for model in &models {
            snapshots.push(self.model_performance(model).await);
        }
        snapshots
    }

    /// Export a serialisable summary of current telemetry state.
    ///
    /// Used for persistence across orchestrator restarts.
    pub async fn export_summary(&self) -> TelemetrySummary {
        let performances = self.all_model_performances().await;
        let window_secs = self.inner.read().await.window_secs;

        TelemetrySummary {
            id: "telemetry_summary".to_string(),
            model_performances: performances,
            window_secs,
            exported_at: Utc::now(),
        }
    }

    /// Import a previously exported summary, restoring model performance data.
    ///
    /// Restores subscription limit flags and synthesises representative completion
    /// events from the aggregated snapshot so that throughput/latency metrics
    /// survive orchestrator restarts.
    pub async fn import_summary(&self, summary: TelemetrySummary) {
        let mut inner = self.inner.write().await;
        let now = Utc::now();
        let window_secs = inner.window_secs;
        let cutoff = now - chrono::Duration::seconds(window_secs as i64);

        for perf in &summary.model_performances {
            if let Some(expires) = perf.subscription_limit_expires_at {
                if now < expires {
                    inner
                        .subscription_limits
                        .insert(perf.model.clone(), expires);
                }
            }

            if let Some(event_time) = perf.last_event_at {
                if event_time >= cutoff && perf.successful_completions > 0 {
                    let synthetic_count = perf.successful_completions.min(10);
                    let events = inner.events.entry(perf.model.clone()).or_default();
                    for i in 0..synthetic_count {
                        let spread = chrono::Duration::seconds(
                            (window_secs as i64 * i as i64) / synthetic_count.max(1) as i64,
                        );
                        events.push(CompletionEvent {
                            model: perf.model.clone(),
                            session_id: String::new(),
                            completed_at: event_time - spread,
                            latency_ms: perf.avg_latency_ms as u64,
                            success: true,
                            tokens: TokenBreakdown::default(),
                            cost_usd: 0.0,
                            error: None,
                        });
                    }
                }

                if event_time >= cutoff {
                    for _ in 0..perf.failed_completions.min(3) {
                        let events = inner.events.entry(perf.model.clone()).or_default();
                        events.push(CompletionEvent {
                            model: perf.model.clone(),
                            session_id: String::new(),
                            completed_at: event_time,
                            latency_ms: 0,
                            success: false,
                            tokens: TokenBreakdown::default(),
                            cost_usd: 0.0,
                            error: Some("recovered failure".to_string()),
                        });
                    }
                }
            }
        }

        tracing::info!(
            model_count = summary.model_performances.len(),
            exported_at = %summary.exported_at,
            "Imported telemetry summary"
        );
    }
}

/// Check if an error message indicates a subscription limit.
pub fn is_subscription_limit_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("weekly session limit")
        || lower.contains("monthly limit")
        || lower.contains("rate limit exceeded")
        || lower.contains("quota exceeded")
        || lower.contains("429")
        || lower.contains("too many requests")
        || lower.contains("capacity limit")
        || lower.contains("spending limit")
        || lower.contains("billing limit")
        || lower.contains("usage limit")
        || lower.contains("subscription limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_event(model: &str, success: bool, latency_ms: u64) -> CompletionEvent {
        CompletionEvent {
            model: model.to_string(),
            session_id: "test-session".to_string(),
            completed_at: Utc::now(),
            latency_ms,
            success,
            tokens: TokenBreakdown {
                total: 1000,
                input: 800,
                output: 200,
                ..Default::default()
            },
            cost_usd: 0.01,
            error: if success {
                None
            } else {
                Some("rate limit exceeded".to_string())
            },
        }
    }

    #[tokio::test]
    async fn test_record_and_query_performance() {
        let store = TelemetryStore::new(3600);
        store.record(test_event("model-a", true, 100)).await;
        store.record(test_event("model-a", true, 200)).await;
        store.record(test_event("model-a", false, 5000)).await;

        let perf = store.model_performance("model-a").await;
        assert_eq!(perf.successful_completions, 2);
        assert_eq!(perf.failed_completions, 1);
        assert!((perf.avg_latency_ms - 150.0).abs() < 1.0);
        assert!((perf.success_rate - 0.6667).abs() < 0.01);
        assert!(perf.subscription_limit_reached);
    }

    #[tokio::test]
    async fn test_session_usage() {
        let store = TelemetryStore::new(3600);
        store
            .record(CompletionEvent {
                model: "model-a".to_string(),
                session_id: "sess-1".to_string(),
                completed_at: Utc::now(),
                latency_ms: 100,
                success: true,
                tokens: TokenBreakdown {
                    total: 500,
                    input: 400,
                    output: 100,
                    ..Default::default()
                },
                cost_usd: 0.005,
                error: None,
            })
            .await;

        let usage = store.session_usage("sess-1").await;
        assert_eq!(usage.totals.total_tokens, 500);
        assert_eq!(usage.totals.message_count, 1);
        assert!(usage.by_model.contains_key("model-a"));
    }

    #[tokio::test]
    async fn test_unknown_model_returns_empty() {
        let store = TelemetryStore::new(3600);
        let perf = store.model_performance("unknown").await;
        assert_eq!(perf.successful_completions, 0);
        assert!(!perf.subscription_limit_reached);
    }

    #[tokio::test]
    async fn test_import_summary_restores_events() {
        let store = TelemetryStore::new(3600);
        let now = Utc::now();

        store
            .record(CompletionEvent {
                model: "model-a".to_string(),
                session_id: "sess-1".to_string(),
                completed_at: now,
                latency_ms: 200,
                success: true,
                tokens: TokenBreakdown {
                    total: 1000,
                    input: 800,
                    output: 200,
                    ..Default::default()
                },
                cost_usd: 0.01,
                error: None,
            })
            .await;

        let original_perf = store.model_performance("model-a").await;
        assert_eq!(original_perf.successful_completions, 1);

        let summary = store.export_summary().await;

        let restored = TelemetryStore::new(3600);
        restored.import_summary(summary).await;

        let restored_perf = restored.model_performance("model-a").await;
        assert!(restored_perf.successful_completions > 0);
        assert!(restored_perf.avg_latency_ms > 0.0);
    }

    #[tokio::test]
    async fn test_import_summary_restores_subscription_limits() {
        let store = TelemetryStore::new(3600);
        let now = Utc::now();

        store
            .record(CompletionEvent {
                model: "model-b".to_string(),
                session_id: "sess-2".to_string(),
                completed_at: now,
                latency_ms: 100,
                success: false,
                tokens: TokenBreakdown::default(),
                cost_usd: 0.0,
                error: Some("weekly session limit reached".to_string()),
            })
            .await;

        let summary = store.export_summary().await;

        let restored = TelemetryStore::new(3600);
        restored.import_summary(summary).await;

        let perf = restored.model_performance("model-b").await;
        assert!(perf.subscription_limit_reached);
    }

    #[tokio::test]
    async fn test_import_summary_prunes_stale_data() {
        let _store = TelemetryStore::new(3600);
        let old_time = Utc::now() - chrono::Duration::seconds(7200);

        let summary = TelemetrySummary {
            id: "telemetry_summary".to_string(),
            model_performances: vec![ModelPerformanceSnapshot {
                model: "stale-model".to_string(),
                successful_completions: 5,
                failed_completions: 0,
                window_secs: 3600,
                throughput: 0.001,
                avg_latency_ms: 500.0,
                success_rate: 1.0,
                last_event_at: Some(old_time),
                subscription_limit_reached: false,
                subscription_limit_expires_at: None,
            }],
            window_secs: 3600,
            exported_at: old_time,
        };

        let restored = TelemetryStore::new(3600);
        restored.import_summary(summary).await;

        let perf = restored.model_performance("stale-model").await;
        assert_eq!(perf.successful_completions, 0);
    }

    #[test]
    fn test_subscription_limit_detection() {
        assert!(is_subscription_limit_error("weekly session limit reached"));
        assert!(is_subscription_limit_error(
            "Rate Limit Exceeded - try again later"
        ));
        assert!(is_subscription_limit_error("Error 429: Too Many Requests"));
        assert!(is_subscription_limit_error(
            "Quota exceeded for this billing period"
        ));
        assert!(!is_subscription_limit_error("connection refused"));
        assert!(!is_subscription_limit_error("syntax error in response"));
    }
}
