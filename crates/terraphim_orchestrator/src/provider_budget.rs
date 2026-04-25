//! Provider-level spend tracking with hour and day windows.
//!
//! Complements [`crate::cost_tracker::CostTracker`] (which tracks a monthly
//! budget per agent) by tracking accumulated spend per external LLM
//! provider (`opencode-go`, `kimi-for-coding`, ...) in tumbling UTC
//! hour and day windows. Re-uses [`BudgetVerdict`] so the routing
//! engine treats both signals uniformly: `Exhausted` providers are
//! dropped from the candidate set and `NearExhaustion` providers are
//! deprioritised via `BudgetPressure`.
//!
//! Buckets tumble at UTC hour / day boundaries. Missing `max_hour_cents`
//! or `max_day_cents` disables that window (verdict `Uncapped`).
//!
//! State can be snapshotted to a JSON file so a restart does not reset
//! the current-window counters.

use crate::cost_tracker::BudgetVerdict;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

const SUB_CENTS_PER_USD: u64 = 10_000;
const WARNING_THRESHOLD: f64 = 0.80;

/// Per-provider budget caps. Missing fields disable the corresponding window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderBudgetConfig {
    /// Provider id (e.g. `opencode-go`, `kimi-for-coding`).
    pub id: String,
    /// Max spend in cents per UTC hour. `None` = uncapped.
    #[serde(default)]
    pub max_hour_cents: Option<u64>,
    /// Max spend in cents per UTC day. `None` = uncapped.
    #[serde(default)]
    pub max_day_cents: Option<u64>,
    /// Optional regex patterns for classifying this provider's stderr.
    /// Consumed by [`crate::error_signatures`] at config load time.
    #[serde(default)]
    pub error_signatures: Option<crate::error_signatures::ProviderErrorSignatures>,
}

/// Serialisable window state.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowState {
    /// UTC tumbling bucket id: `YYYYMMDDHH` for hour, `YYYYMMDD` for day.
    pub window_id: u64,
    /// Accumulated spend in hundredths-of-a-cent.
    pub sub_cents: u64,
}

/// Serialisable snapshot of a single provider's two windows.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderSnapshotEntry {
    pub hour: WindowState,
    pub day: WindowState,
}

/// Serialisable snapshot of the whole tracker.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderBudgetSnapshot {
    pub providers: HashMap<String, ProviderSnapshotEntry>,
}

/// Hour + day windows held behind a single mutex so record and check
/// observe a consistent snapshot across both windows. A prior split
/// (one mutex per window) allowed a concurrent recorder to interleave
/// updates such that an observer between the two locks saw the hour
/// bucket advanced but not the day bucket.
#[derive(Debug, Default)]
struct ProviderWindows {
    hour: WindowState,
    day: WindowState,
}

#[derive(Debug, Default)]
struct ProviderState {
    windows: Mutex<ProviderWindows>,
}

/// Tracks provider spend across hour and day windows.
#[derive(Debug)]
pub struct ProviderBudgetTracker {
    configs: HashMap<String, ProviderBudgetConfig>,
    state: HashMap<String, ProviderState>,
    state_file: Option<PathBuf>,
}

impl ProviderBudgetTracker {
    /// Build a tracker from the given config list. No persistence.
    pub fn new(configs: Vec<ProviderBudgetConfig>) -> Self {
        let mut config_map = HashMap::new();
        let mut state_map = HashMap::new();
        for cfg in configs {
            let id = cfg.id.clone();
            state_map.insert(id.clone(), ProviderState::default());
            config_map.insert(id, cfg);
        }
        Self {
            configs: config_map,
            state: state_map,
            state_file: None,
        }
    }

    /// Build a tracker and load any prior snapshot from `state_file`.
    /// A missing file is not an error -- the tracker starts empty.
    pub fn with_persistence(
        configs: Vec<ProviderBudgetConfig>,
        state_file: PathBuf,
    ) -> io::Result<Self> {
        let mut tracker = Self::new(configs);
        tracker.state_file = Some(state_file.clone());
        if state_file.exists() {
            let raw = fs::read_to_string(&state_file)?;
            if raw.trim().is_empty() {
                return Ok(tracker);
            }
            let snap: ProviderBudgetSnapshot = serde_json::from_str(&raw)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            tracker.apply_snapshot(snap);
        }
        Ok(tracker)
    }

    /// Overlay a snapshot onto an existing tracker. Only providers that
    /// are still configured keep their state; the rest are discarded so
    /// stale entries from removed providers do not linger.
    fn apply_snapshot(&mut self, snap: ProviderBudgetSnapshot) {
        for (provider, entry) in snap.providers {
            if let Some(state) = self.state.get_mut(&provider) {
                let mut w = state.windows.lock().expect("windows lock poisoned");
                w.hour = entry.hour;
                w.day = entry.day;
            }
        }
    }

    /// Return a serialisable view of the current state.
    pub fn snapshot(&self) -> ProviderBudgetSnapshot {
        let mut providers = HashMap::with_capacity(self.state.len());
        for (id, state) in &self.state {
            let w = state.windows.lock().expect("windows lock poisoned");
            providers.insert(
                id.clone(),
                ProviderSnapshotEntry {
                    hour: w.hour,
                    day: w.day,
                },
            );
        }
        ProviderBudgetSnapshot { providers }
    }

    /// Persist the current snapshot to `state_file` (if configured).
    /// Writes are atomic via a sibling `.tmp` file + rename.
    pub fn persist(&self) -> io::Result<()> {
        let Some(path) = self.state_file.as_ref() else {
            return Ok(());
        };
        let snap = self.snapshot();
        let json = serde_json::to_string_pretty(&snap)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = path.with_extension("tmp");
        if let Some(parent) = tmp.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&tmp, json)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Record a USD spend against the provider and return the merged
    /// hour+day verdict. Unknown providers are silently ignored (return
    /// `Uncapped`) -- we only track providers with an explicit config.
    pub fn record_cost(&self, provider: &str, cost_usd: f64) -> BudgetVerdict {
        self.record_cost_at(provider, cost_usd, Utc::now())
    }

    /// Test hook: record against a caller-supplied timestamp so tests
    /// can cross window boundaries deterministically.
    pub fn record_cost_at(
        &self,
        provider: &str,
        cost_usd: f64,
        now: DateTime<Utc>,
    ) -> BudgetVerdict {
        let Some(cfg) = self.configs.get(provider) else {
            return BudgetVerdict::Uncapped;
        };
        let Some(state) = self.state.get(provider) else {
            return BudgetVerdict::Uncapped;
        };
        let delta = (cost_usd * SUB_CENTS_PER_USD as f64).round().max(0.0) as u64;

        // Single lock across both windows: record and prune atomically
        // so a concurrent `check` cannot observe a half-updated state.
        let mut w = state.windows.lock().expect("windows lock poisoned");
        let hour_verdict =
            update_window_in_place(&mut w.hour, hour_window_id(now), cfg.max_hour_cents, delta);
        let day_verdict =
            update_window_in_place(&mut w.day, day_window_id(now), cfg.max_day_cents, delta);

        combine_verdicts(hour_verdict, day_verdict)
    }

    /// Non-mutating check. Does NOT record spend; returns the verdict
    /// that would apply if a zero-cost call were made right now.
    pub fn check(&self, provider: &str) -> BudgetVerdict {
        self.check_at(provider, Utc::now())
    }

    /// Test hook for `check` with caller-supplied timestamp.
    pub fn check_at(&self, provider: &str, now: DateTime<Utc>) -> BudgetVerdict {
        let Some(cfg) = self.configs.get(provider) else {
            return BudgetVerdict::Uncapped;
        };
        let Some(state) = self.state.get(provider) else {
            return BudgetVerdict::Uncapped;
        };
        // Single lock: hour and day observed atomically.
        let w = state.windows.lock().expect("windows lock poisoned");
        let hour_verdict = check_window_state(&w.hour, hour_window_id(now), cfg.max_hour_cents);
        let day_verdict = check_window_state(&w.day, day_window_id(now), cfg.max_day_cents);
        combine_verdicts(hour_verdict, day_verdict)
    }

    /// Force both windows for `provider` past their caps so the next
    /// [`Self::check`] returns [`BudgetVerdict::Exhausted`]. Used by the
    /// error-signature classifier when a spawn stderr signals that the
    /// provider has hit its external quota even though our spend counter
    /// has not yet registered the charge (providers bill asynchronously).
    ///
    /// Only providers with at least one configured cap are affected --
    /// uncapped providers remain `Uncapped` because there is nothing to
    /// exhaust. Unknown providers are silently ignored.
    pub fn force_exhaust(&self, provider: &str) {
        let Some(cfg) = self.configs.get(provider) else {
            return;
        };
        let Some(state) = self.state.get(provider) else {
            return;
        };
        let now = Utc::now();
        let mut w = state.windows.lock().expect("windows lock poisoned");
        if let Some(cap) = cfg.max_hour_cents {
            w.hour.window_id = hour_window_id(now);
            w.hour.sub_cents = cap.saturating_mul(100).saturating_add(100);
        }
        if let Some(cap) = cfg.max_day_cents {
            w.day.window_id = day_window_id(now);
            w.day.sub_cents = cap.saturating_mul(100).saturating_add(100);
        }
    }

    /// Iterate over all provider ids known to the tracker.
    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.configs.keys().map(|s| s.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }
}

fn hour_window_id(ts: DateTime<Utc>) -> u64 {
    (ts.year() as u64) * 1_000_000
        + (ts.month() as u64) * 10_000
        + (ts.day() as u64) * 100
        + (ts.hour() as u64)
}

fn day_window_id(ts: DateTime<Utc>) -> u64 {
    (ts.year() as u64) * 10_000 + (ts.month() as u64) * 100 + (ts.day() as u64)
}

/// Apply `delta` sub-cents to the window, resetting first if the bucket
/// has rolled over. Returns the verdict that applies post-record.
/// Operates on an already-locked `WindowState` so the caller holds the
/// combined hour+day mutex.
fn update_window_in_place(
    ws: &mut WindowState,
    current_id: u64,
    max_cents: Option<u64>,
    delta: u64,
) -> BudgetVerdict {
    if ws.window_id != current_id {
        ws.window_id = current_id;
        ws.sub_cents = 0;
    }
    ws.sub_cents = ws.sub_cents.saturating_add(delta);
    verdict_for(ws.sub_cents, max_cents)
}

fn check_window_state(ws: &WindowState, current_id: u64, max_cents: Option<u64>) -> BudgetVerdict {
    if ws.window_id != current_id {
        // Fresh bucket -- no spend yet.
        return verdict_for(0, max_cents);
    }
    verdict_for(ws.sub_cents, max_cents)
}

fn verdict_for(sub_cents: u64, max_cents: Option<u64>) -> BudgetVerdict {
    let Some(cap) = max_cents else {
        return BudgetVerdict::Uncapped;
    };
    if cap == 0 {
        // Zero cap = always exhausted; a misconfiguration, but handled.
        return BudgetVerdict::Exhausted {
            spent_cents: sub_cents / 100,
            budget_cents: 0,
        };
    }
    // sub_cents is hundredths-of-a-cent. Normalise to cents for the verdict.
    let spent_cents = sub_cents / 100;
    let cap_sub_cents = cap.saturating_mul(100);

    if sub_cents >= cap_sub_cents {
        BudgetVerdict::Exhausted {
            spent_cents,
            budget_cents: cap,
        }
    } else if (sub_cents as f64) >= (cap_sub_cents as f64) * WARNING_THRESHOLD {
        BudgetVerdict::NearExhaustion {
            spent_cents,
            budget_cents: cap,
        }
    } else {
        BudgetVerdict::WithinBudget
    }
}

/// Merge two verdicts -- more urgent wins. Exhausted > NearExhaustion >
/// WithinBudget > Uncapped.
fn combine_verdicts(a: BudgetVerdict, b: BudgetVerdict) -> BudgetVerdict {
    fn rank(v: &BudgetVerdict) -> u8 {
        match v {
            BudgetVerdict::Exhausted { .. } => 3,
            BudgetVerdict::NearExhaustion { .. } => 2,
            BudgetVerdict::WithinBudget => 1,
            BudgetVerdict::Uncapped => 0,
        }
    }
    if rank(&a) >= rank(&b) { a } else { b }
}

/// Filter helper for the routing engine: returns `true` if the
/// provider is safe to consider, `false` if its verdict is `Exhausted`.
pub fn provider_has_budget(tracker: &ProviderBudgetTracker, provider: &str) -> bool {
    !matches!(tracker.check(provider), BudgetVerdict::Exhausted { .. })
}

/// Extract the provider-budget key for a `provider/model` string
/// (e.g. `opencode-go/minimax-m2.5` -> `opencode-go`). Bare model
/// names fall back to the Anthropic subscription id (`claude-code`)
/// so any caller that knows only the model can still look up quota.
///
/// Returns `None` for model strings that cannot be classified.
pub fn provider_key_for_model(provider_or_model: &str) -> Option<&str> {
    if let Some((prefix, _)) = provider_or_model.split_once('/') {
        return Some(prefix);
    }
    if crate::config::CLAUDE_CLI_BARE_MODELS.contains(&provider_or_model) {
        return Some("claude-code");
    }
    if crate::config::ANTHROPIC_BARE_PROVIDERS.contains(&provider_or_model) {
        return Some("claude-code");
    }
    // Unknown bare identifier -- treat as its own provider.
    Some(provider_or_model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn cfg(id: &str, hour: Option<u64>, day: Option<u64>) -> ProviderBudgetConfig {
        ProviderBudgetConfig {
            id: id.to_string(),
            max_hour_cents: hour,
            max_day_cents: day,
            error_signatures: None,
        }
    }

    #[test]
    fn test_uncapped_for_unknown_provider() {
        let t = ProviderBudgetTracker::new(vec![]);
        assert_eq!(t.check("missing"), BudgetVerdict::Uncapped);
        // Recording against an unknown provider is a no-op.
        assert_eq!(t.record_cost("missing", 10.0), BudgetVerdict::Uncapped);
    }

    #[test]
    fn test_hour_window_exhausts() {
        // 100-cent = $1/hour cap.
        let t = ProviderBudgetTracker::new(vec![cfg("opencode-go", Some(100), None)]);
        let t0 = Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap();

        assert_eq!(
            t.record_cost_at("opencode-go", 0.50, t0),
            BudgetVerdict::WithinBudget
        );
        // Now at 80 cents -> near exhaustion.
        assert!(matches!(
            t.record_cost_at("opencode-go", 0.30, t0),
            BudgetVerdict::NearExhaustion { .. }
        ));
        // Push over the cap.
        assert!(matches!(
            t.record_cost_at("opencode-go", 0.30, t0),
            BudgetVerdict::Exhausted { .. }
        ));
    }

    #[test]
    fn test_hour_window_resets_on_next_hour() {
        let t = ProviderBudgetTracker::new(vec![cfg("opencode-go", Some(100), None)]);
        let t0 = Utc.with_ymd_and_hms(2026, 4, 19, 10, 30, 0).unwrap();
        let t1 = Utc.with_ymd_and_hms(2026, 4, 19, 11, 5, 0).unwrap();

        // Exhaust the 10:00 bucket.
        let _ = t.record_cost_at("opencode-go", 1.50, t0);
        assert!(matches!(
            t.check_at("opencode-go", t0),
            BudgetVerdict::Exhausted { .. }
        ));

        // 11:00 bucket is fresh.
        assert_eq!(t.check_at("opencode-go", t1), BudgetVerdict::WithinBudget);
    }

    #[test]
    fn test_day_cap_independent_of_hour_cap() {
        // 100 cents/hour, 150 cents/day -- hitting the daily cap across
        // two hours while each hour stays under its per-hour limit.
        let t = ProviderBudgetTracker::new(vec![cfg("opencode-go", Some(100), Some(150))]);
        let t0 = Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap();
        let t1 = Utc.with_ymd_and_hms(2026, 4, 19, 11, 0, 0).unwrap();

        let _ = t.record_cost_at("opencode-go", 0.90, t0);
        // Hour bucket at 90c (near), day at 90c (within 150). Merge: NearExhaustion.
        assert!(matches!(
            t.check_at("opencode-go", t0),
            BudgetVerdict::NearExhaustion { .. }
        ));

        // Cross into hour 11; daily counter continues to accumulate.
        let _ = t.record_cost_at("opencode-go", 0.70, t1);
        let verdict = t.check_at("opencode-go", t1);
        assert!(
            matches!(verdict, BudgetVerdict::Exhausted { .. }),
            "day cap should trip even though hour bucket rolled: {:?}",
            verdict
        );
    }

    #[test]
    fn test_force_exhaust_trips_both_windows() {
        // Every window that has a cap must end up Exhausted after
        // force_exhaust so the routing gate drops the provider until
        // the next UTC window rolls -- even though we never recorded
        // any cost.
        let t = ProviderBudgetTracker::new(vec![cfg("claude-code", Some(500), Some(2000))]);
        assert_eq!(t.check("claude-code"), BudgetVerdict::WithinBudget);

        t.force_exhaust("claude-code");
        assert!(
            matches!(t.check("claude-code"), BudgetVerdict::Exhausted { .. }),
            "force_exhaust must trip the combined verdict"
        );
        // And the filter helper that routing uses must reject it.
        assert!(!provider_has_budget(&t, "claude-code"));
    }

    #[test]
    fn test_force_exhaust_is_noop_for_uncapped_provider() {
        // No cap means there's nothing to exhaust -- verdict stays
        // Uncapped so we don't accidentally block a provider that
        // the operator has intentionally left unbounded.
        let t = ProviderBudgetTracker::new(vec![cfg("claude-code", None, None)]);
        t.force_exhaust("claude-code");
        assert_eq!(t.check("claude-code"), BudgetVerdict::Uncapped);
    }

    #[test]
    fn test_force_exhaust_ignores_unknown_provider() {
        let t = ProviderBudgetTracker::new(vec![cfg("claude-code", Some(100), None)]);
        t.force_exhaust("not-a-provider"); // must not panic or poison state
        assert_eq!(t.check("claude-code"), BudgetVerdict::WithinBudget);
    }

    #[test]
    fn test_day_window_resets_next_day() {
        let t = ProviderBudgetTracker::new(vec![cfg("opencode-go", None, Some(100))]);
        let t0 = Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap();
        let t1 = Utc.with_ymd_and_hms(2026, 4, 20, 0, 1, 0).unwrap();

        let _ = t.record_cost_at("opencode-go", 1.50, t0);
        assert!(matches!(
            t.check_at("opencode-go", t0),
            BudgetVerdict::Exhausted { .. }
        ));
        // Fresh day.
        assert_eq!(t.check_at("opencode-go", t1), BudgetVerdict::WithinBudget);
    }

    #[test]
    fn test_snapshot_round_trip_via_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        // Remove the blank file so `with_persistence` treats it as missing.
        drop(tmp);

        let configs = vec![cfg("opencode-go", Some(500), Some(2000))];
        let t = ProviderBudgetTracker::with_persistence(configs.clone(), path.clone()).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap();
        let _ = t.record_cost_at("opencode-go", 1.23, now);
        t.persist().unwrap();

        // New tracker loads the snapshot.
        let t2 = ProviderBudgetTracker::with_persistence(configs, path.clone()).unwrap();
        let snap = t2.snapshot();
        let entry = snap
            .providers
            .get("opencode-go")
            .expect("provider state persisted");
        assert_eq!(entry.hour.sub_cents, 12_300);
        assert_eq!(entry.day.sub_cents, 12_300);

        // Cleanup
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_combine_verdicts_picks_worst() {
        let hour = BudgetVerdict::NearExhaustion {
            spent_cents: 80,
            budget_cents: 100,
        };
        let day = BudgetVerdict::Exhausted {
            spent_cents: 1000,
            budget_cents: 1000,
        };
        assert!(matches!(
            combine_verdicts(hour, day),
            BudgetVerdict::Exhausted { .. }
        ));
    }

    #[test]
    fn test_provider_has_budget_helper() {
        // Use `record_cost` + `check` at real-now so both land in the
        // same hour bucket and the helper's verdict reflects the spend.
        let t = ProviderBudgetTracker::new(vec![cfg("p", Some(100), None)]);
        assert!(provider_has_budget(&t, "p"));
        let _ = t.record_cost("p", 2.00);
        assert!(!provider_has_budget(&t, "p"));
    }

    #[test]
    fn test_stale_snapshot_entry_for_removed_provider_discarded() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        drop(tmp);

        // Persist state for "old-provider".
        let t = ProviderBudgetTracker::with_persistence(
            vec![cfg("old-provider", Some(100), None)],
            path.clone(),
        )
        .unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap();
        let _ = t.record_cost_at("old-provider", 0.50, now);
        t.persist().unwrap();

        // Reload with a new config that drops "old-provider".
        let t2 = ProviderBudgetTracker::with_persistence(
            vec![cfg("new-provider", Some(100), None)],
            path.clone(),
        )
        .unwrap();
        let snap = t2.snapshot();
        assert!(!snap.providers.contains_key("old-provider"));

        let _ = fs::remove_file(&path);
    }
}
