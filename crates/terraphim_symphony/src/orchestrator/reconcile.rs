//! Reconciliation logic for the orchestrator.
//!
//! Handles stall detection and tracker state refresh for running issues.

use crate::orchestrator::state::OrchestratorRuntimeState;
use chrono::Utc;
use tracing::{debug, info, warn};

/// Result of reconciling a single running entry.
#[derive(Debug)]
pub enum ReconcileAction {
    /// Issue is still active; optionally update the cached state.
    KeepRunning { new_state: Option<String> },
    /// Issue is in a terminal state; stop and clean workspace.
    TerminateAndCleanup,
    /// Issue is no longer active (but not terminal); stop without cleanup.
    TerminateNoCleanup,
    /// Session has stalled; kill and schedule retry.
    StallDetected,
}

/// Check a single running entry for stall.
///
/// Returns `Some(StallDetected)` if the session has stalled, `None` otherwise.
pub fn check_stall(
    last_event_timestamp: Option<chrono::DateTime<Utc>>,
    started_at: chrono::DateTime<Utc>,
    stall_timeout_ms: i64,
) -> Option<ReconcileAction> {
    if stall_timeout_ms <= 0 {
        return None; // Stall detection disabled
    }

    let reference_time = last_event_timestamp.unwrap_or(started_at);
    let elapsed_ms = (Utc::now() - reference_time).num_milliseconds();

    if elapsed_ms > stall_timeout_ms {
        debug!(
            elapsed_ms,
            stall_timeout_ms,
            "session stall detected"
        );
        Some(ReconcileAction::StallDetected)
    } else {
        None
    }
}

/// Determine the reconciliation action for a running issue based on
/// its refreshed tracker state.
pub fn determine_action(
    current_state: &str,
    active_states: &[String],
    terminal_states: &[String],
) -> ReconcileAction {
    let is_terminal = terminal_states
        .iter()
        .any(|s| s.eq_ignore_ascii_case(current_state));
    if is_terminal {
        info!(state = current_state, "issue reached terminal state");
        return ReconcileAction::TerminateAndCleanup;
    }

    let is_active = active_states
        .iter()
        .any(|s| s.eq_ignore_ascii_case(current_state));
    if is_active {
        return ReconcileAction::KeepRunning {
            new_state: Some(current_state.to_string()),
        };
    }

    // Neither active nor terminal
    warn!(
        state = current_state,
        "issue in unexpected state, terminating without cleanup"
    );
    ReconcileAction::TerminateNoCleanup
}

/// Collect stalled issue IDs from the runtime state.
pub fn find_stalled_issues(
    state: &OrchestratorRuntimeState,
    stall_timeout_ms: i64,
) -> Vec<String> {
    if stall_timeout_ms <= 0 {
        return vec![];
    }

    state
        .running
        .iter()
        .filter_map(|(issue_id, entry)| {
            if check_stall(
                entry.session.last_timestamp,
                entry.started_at,
                stall_timeout_ms,
            )
            .is_some()
            {
                Some(issue_id.clone())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_state_triggers_cleanup() {
        let action = determine_action(
            "Done",
            &["Todo".into(), "In Progress".into()],
            &["Done".into(), "Closed".into()],
        );
        assert!(matches!(action, ReconcileAction::TerminateAndCleanup));
    }

    #[test]
    fn active_state_keeps_running() {
        let action = determine_action(
            "In Progress",
            &["Todo".into(), "In Progress".into()],
            &["Done".into(), "Closed".into()],
        );
        assert!(matches!(action, ReconcileAction::KeepRunning { .. }));
    }

    #[test]
    fn unknown_state_terminates_without_cleanup() {
        let action = determine_action(
            "Review",
            &["Todo".into(), "In Progress".into()],
            &["Done".into(), "Closed".into()],
        );
        assert!(matches!(action, ReconcileAction::TerminateNoCleanup));
    }

    #[test]
    fn stall_detection_disabled_when_zero() {
        let result = check_stall(None, Utc::now(), 0);
        assert!(result.is_none());
    }

    #[test]
    fn stall_detection_disabled_when_negative() {
        let result = check_stall(None, Utc::now(), -1);
        assert!(result.is_none());
    }

    #[test]
    fn no_stall_when_recent_event() {
        let result = check_stall(Some(Utc::now()), Utc::now(), 300_000);
        assert!(result.is_none());
    }

    #[test]
    fn stall_detected_when_old_event() {
        let old = Utc::now() - chrono::Duration::seconds(600);
        let result = check_stall(Some(old), Utc::now(), 300_000);
        assert!(matches!(result, Some(ReconcileAction::StallDetected)));
    }

    #[test]
    fn stall_uses_started_at_when_no_events() {
        let old_start = Utc::now() - chrono::Duration::seconds(600);
        let result = check_stall(None, old_start, 300_000);
        assert!(matches!(result, Some(ReconcileAction::StallDetected)));
    }
}
