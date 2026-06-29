//! Pure-function module for reconciling PR gate state against branch protection requirements.
//!
//! This module has zero dependency on orchestrator runtime state and performs no I/O.
//! It reads a [`PrGateSnapshot`] capturing the current state of a PR head and produces
//! a deterministic [`PrGateDecision`] indicating what action the reconciler should take.
//!
//! See `.docs/design-adf-pr-merge-progress-2026-05-01.md` for the full design.

/// Terminal state of a single commit status context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommitStatusState {
    Pending,
    Success,
    Failure,
    Error,
}

impl CommitStatusState {
    /// Parse from the Gitea API string representation.
    pub fn from_api_str(s: &str) -> Self {
        match s {
            "success" => Self::Success,
            "failure" => Self::Failure,
            "error" => Self::Error,
            _ => Self::Pending,
        }
    }

    /// True when the context has reached a terminal (non-pending) state.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Pending)
    }
}

/// One commit status entry posted against a SHA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitStatusSummary {
    pub context: String,
    pub state: CommitStatusState,
    /// Unix timestamp (seconds) when the status was created, if available.
    pub created_at_unix: Option<i64>,
}

/// Default stale pending timeout in seconds (60 minutes).
pub const STALE_PENDING_TIMEOUT_SECS: i64 = 3600;

/// Snapshot of everything the reconciler needs to classify a PR head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrGateSnapshot {
    pub pr_number: u64,
    pub head_sha: String,
    pub base_branch: String,
    /// Context names required by branch protection (e.g. `["adf/build", "adf/pr-reviewer"]`).
    pub required_contexts: Vec<String>,
    /// Commit statuses actually posted on the head SHA.
    pub head_statuses: Vec<CommitStatusSummary>,
    /// Current time as Unix timestamp (seconds). Used for stale pending detection.
    pub now_unix: i64,
}

/// Deterministic classification of a PR head's gate state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrGateDecision {
    /// All required contexts green; proceed to auto-merge policy evaluation.
    ReadyForPolicy,
    /// Required contexts not yet posted; enqueue the responsible agents.
    EnqueueMissingChecks { missing: Vec<String> },
    /// Required contexts posted but still pending; wait for next reconcile tick.
    AwaitingChecks { pending: Vec<String> },
    /// At least one required context failed; open remediation issue.
    BlockedByFailedChecks { failed: Vec<(String, String)> },
    /// Status API or branch protection API failure; service fault.
    FactoryFault { error: String },
}

/// Reconcile the PR gate state from a snapshot. Pure function.
///
/// Decision logic:
/// 1. If required_contexts is empty -> `ReadyForPolicy` (no gate configured).
/// 2. Find missing contexts (required but not posted) -> `EnqueueMissingChecks`.
/// 3. Find pending contexts (posted but not terminal) -> `AwaitingChecks`.
/// 4. Find failed contexts (terminal but not success) -> `BlockedByFailedChecks`.
/// 5. All required contexts are terminal success -> `ReadyForPolicy`.
pub fn reconcile_pr_gate(snapshot: &PrGateSnapshot) -> PrGateDecision {
    if snapshot.required_contexts.is_empty() {
        return PrGateDecision::ReadyForPolicy;
    }

    let missing = missing_required_contexts(&snapshot.required_contexts, &snapshot.head_statuses);
    if !missing.is_empty() {
        return PrGateDecision::EnqueueMissingChecks { missing };
    }

    let stale = stale_pending_contexts(
        &snapshot.required_contexts,
        &snapshot.head_statuses,
        snapshot.now_unix,
        STALE_PENDING_TIMEOUT_SECS,
    );
    if !stale.is_empty() {
        return PrGateDecision::FactoryFault {
            error: format!(
                "stale pending contexts (>{:>0}min): {}",
                STALE_PENDING_TIMEOUT_SECS / 60,
                stale.join(", ")
            ),
        };
    }

    let pending = pending_required_contexts(&snapshot.required_contexts, &snapshot.head_statuses);
    if !pending.is_empty() {
        return PrGateDecision::AwaitingChecks { pending };
    }

    let failed = failed_required_contexts(&snapshot.required_contexts, &snapshot.head_statuses);
    if !failed.is_empty() {
        return PrGateDecision::BlockedByFailedChecks { failed };
    }

    PrGateDecision::ReadyForPolicy
}

/// Compute which required contexts have no status posted at all on the head SHA.
pub fn missing_required_contexts(
    required: &[String],
    statuses: &[CommitStatusSummary],
) -> Vec<String> {
    let posted: std::collections::HashSet<&str> =
        statuses.iter().map(|s| s.context.as_str()).collect();
    required
        .iter()
        .filter(|ctx| !posted.contains(ctx.as_str()))
        .cloned()
        .collect()
}

/// Groups statuses by context, keeping the latest one for each context.
///
/// Prefers entries with a higher `created_at_unix`. If `created_at_unix` is None
/// for some entries, prefers the ones that have a timestamp. If multiple entries
/// have None for `created_at_unix`, keeps the last one in the slice.
fn latest_status_per_context<'a>(
    statuses: &'a [CommitStatusSummary],
) -> std::collections::HashMap<&'a str, &'a CommitStatusSummary> {
    let mut map: std::collections::HashMap<&'a str, &'a CommitStatusSummary> =
        std::collections::HashMap::new();
    for status in statuses {
        let should_replace = match map.get(status.context.as_str()) {
            None => true,
            Some(entry) => match (entry.created_at_unix, status.created_at_unix) {
                // Existing has timestamp, new doesn't -> keep existing
                (Some(_), None) => false,
                // Existing doesn't have timestamp, new does -> replace
                (None, Some(_)) => true,
                // Both have timestamps -> replace if new is higher
                (Some(existing), Some(new)) => new > existing,
                // Neither has timestamp -> keep last one in slice (replace)
                (None, None) => true,
            },
        };
        if should_replace {
            map.insert(status.context.as_str(), status);
        }
    }
    map
}

/// Compute which required contexts are posted but still pending.
pub fn pending_required_contexts(
    required: &[String],
    statuses: &[CommitStatusSummary],
) -> Vec<String> {
    let status_map = latest_status_per_context(statuses);
    required
        .iter()
        .filter(|ctx| {
            status_map
                .get(ctx.as_str())
                .is_some_and(|status| !status.state.is_terminal())
        })
        .cloned()
        .collect()
}

/// Compute which required contexts have been pending longer than the timeout.
/// Uses `created_at_unix` from the status entry and `now_unix` from the snapshot.
pub fn stale_pending_contexts(
    required: &[String],
    statuses: &[CommitStatusSummary],
    now_unix: i64,
    timeout_secs: i64,
) -> Vec<String> {
    let status_map = latest_status_per_context(statuses);
    required
        .iter()
        .filter(|ctx| {
            let Some(status) = status_map.get(ctx.as_str()) else {
                return false;
            };
            if status.state.is_terminal() {
                return false;
            }
            let Some(created_ts) = status.created_at_unix else {
                return false;
            };
            now_unix - created_ts > timeout_secs
        })
        .cloned()
        .collect()
}

/// Compute which required contexts have reached a failed/error terminal state.
/// Returns `(context_name, state_label)` pairs.
pub fn failed_required_contexts(
    required: &[String],
    statuses: &[CommitStatusSummary],
) -> Vec<(String, String)> {
    let status_map = latest_status_per_context(statuses);
    required
        .iter()
        .filter(|ctx| {
            status_map.get(ctx.as_str()).is_some_and(|status| {
                matches!(
                    status.state,
                    CommitStatusState::Failure | CommitStatusState::Error
                )
            })
        })
        .map(|ctx| {
            let status = status_map[ctx.as_str()];
            let label = match status.state {
                CommitStatusState::Failure => "failure",
                CommitStatusState::Error => "error",
                _ => unreachable!(),
            };
            (ctx.clone(), label.to_string())
        })
        .collect()
}

/// Deterministic dedup key for remediation issues.
///
/// Format: `"pr-gate:{pr_number}:{head_sha}:{context}"` for failed checks,
/// or `"pr-gate:{pr_number}:{head_sha}:factory-fault:{error}"` for factory faults.
pub fn remediation_key(
    project: &str,
    pr_number: u64,
    head_sha: &str,
    decision: &PrGateDecision,
) -> String {
    match decision {
        PrGateDecision::BlockedByFailedChecks { failed } => {
            let contexts: Vec<&str> = failed.iter().map(|(ctx, _)| ctx.as_str()).collect();
            format!(
                "pr-gate:{}:{}:{}:{}",
                project,
                pr_number,
                &head_sha[..head_sha.len().min(12)],
                contexts.join(",")
            )
        }
        PrGateDecision::FactoryFault { error } => {
            format!(
                "pr-gate:{}:{}:{}:factory-fault:{}",
                project,
                pr_number,
                &head_sha[..head_sha.len().min(12)],
                error.chars().take(40).collect::<String>()
            )
        }
        other => format!(
            "pr-gate:{}:{}:{}:{:?}",
            project,
            pr_number,
            &head_sha[..head_sha.len().min(12)],
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(s: &str) -> String {
        s.to_string()
    }

    fn required(contexts: &[&str]) -> Vec<String> {
        contexts.iter().map(|s| s.to_string()).collect()
    }

    fn statuses(items: &[(&str, CommitStatusState)]) -> Vec<CommitStatusSummary> {
        statuses_with_times(
            &items
                .iter()
                .map(|(ctx, state)| (*ctx, *state, None))
                .collect::<Vec<_>>(),
        )
    }

    fn statuses_with_times(
        items: &[(&str, CommitStatusState, Option<i64>)],
    ) -> Vec<CommitStatusSummary> {
        items
            .iter()
            .map(|(ctx, state, ts)| CommitStatusSummary {
                context: ctx.to_string(),
                state: *state,
                created_at_unix: *ts,
            })
            .collect()
    }

    fn snapshot_ctx(pr: u64, req: &[&str], sts: &[(&str, CommitStatusState)]) -> PrGateSnapshot {
        PrGateSnapshot {
            pr_number: pr,
            head_sha: sha("abc123def456"),
            base_branch: "main".into(),
            required_contexts: required(req),
            head_statuses: statuses(sts),
            now_unix: 10_000,
        }
    }

    fn snapshot_ctx_with_times(
        pr: u64,
        req: &[&str],
        sts: &[(&str, CommitStatusState, Option<i64>)],
        now_unix: i64,
    ) -> PrGateSnapshot {
        PrGateSnapshot {
            pr_number: pr,
            head_sha: sha("abc123def456"),
            base_branch: "main".into(),
            required_contexts: required(req),
            head_statuses: statuses_with_times(sts),
            now_unix,
        }
    }

    #[test]
    fn no_required_contexts_is_ready() {
        let snap = PrGateSnapshot {
            pr_number: 1,
            head_sha: sha("aaa"),
            base_branch: "main".into(),
            required_contexts: vec![],
            head_statuses: vec![],
            now_unix: 10_000,
        };
        assert_eq!(reconcile_pr_gate(&snap), PrGateDecision::ReadyForPolicy);
    }

    #[test]
    fn all_green_is_ready() {
        let snap = snapshot_ctx(
            42,
            &["adf/build", "adf/pr-reviewer"],
            &[
                ("adf/build", CommitStatusState::Success),
                ("adf/pr-reviewer", CommitStatusState::Success),
            ],
        );
        assert_eq!(reconcile_pr_gate(&snap), PrGateDecision::ReadyForPolicy);
    }

    #[test]
    fn missing_both_contexts_enqueues() {
        let snap = snapshot_ctx(1099, &["adf/build", "adf/pr-reviewer"], &[]);
        let decision = reconcile_pr_gate(&snap);
        assert_eq!(
            decision,
            PrGateDecision::EnqueueMissingChecks {
                missing: vec!["adf/build".into(), "adf/pr-reviewer".into()]
            }
        );
    }

    #[test]
    fn missing_one_context_enqueues_only_that() {
        let snap = snapshot_ctx(
            42,
            &["adf/build", "adf/pr-reviewer"],
            &[("adf/build", CommitStatusState::Success)],
        );
        let decision = reconcile_pr_gate(&snap);
        assert_eq!(
            decision,
            PrGateDecision::EnqueueMissingChecks {
                missing: vec!["adf/pr-reviewer".into()]
            }
        );
    }

    #[test]
    fn pending_contexts_waits() {
        let snap = snapshot_ctx(
            42,
            &["adf/build", "adf/pr-reviewer"],
            &[
                ("adf/build", CommitStatusState::Success),
                ("adf/pr-reviewer", CommitStatusState::Pending),
            ],
        );
        let decision = reconcile_pr_gate(&snap);
        assert_eq!(
            decision,
            PrGateDecision::AwaitingChecks {
                pending: vec!["adf/pr-reviewer".into()]
            }
        );
    }

    #[test]
    fn failed_context_blocks() {
        let snap = snapshot_ctx(
            42,
            &["adf/build", "adf/pr-reviewer"],
            &[
                ("adf/build", CommitStatusState::Failure),
                ("adf/pr-reviewer", CommitStatusState::Success),
            ],
        );
        let decision = reconcile_pr_gate(&snap);
        assert_eq!(
            decision,
            PrGateDecision::BlockedByFailedChecks {
                failed: vec![("adf/build".into(), "failure".into())]
            }
        );
    }

    #[test]
    fn error_context_blocks() {
        let snap = snapshot_ctx(
            42,
            &["adf/build"],
            &[("adf/build", CommitStatusState::Error)],
        );
        let decision = reconcile_pr_gate(&snap);
        assert_eq!(
            decision,
            PrGateDecision::BlockedByFailedChecks {
                failed: vec![("adf/build".into(), "error".into())]
            }
        );
    }

    #[test]
    fn all_pending_waits() {
        let snap = snapshot_ctx(
            42,
            &["adf/build", "adf/pr-reviewer"],
            &[
                ("adf/build", CommitStatusState::Pending),
                ("adf/pr-reviewer", CommitStatusState::Pending),
            ],
        );
        let decision = reconcile_pr_gate(&snap);
        assert_eq!(
            decision,
            PrGateDecision::AwaitingChecks {
                pending: vec!["adf/build".into(), "adf/pr-reviewer".into()]
            }
        );
    }

    #[test]
    fn extra_statuses_ignored() {
        let snap = snapshot_ctx(
            42,
            &["adf/build"],
            &[
                ("adf/build", CommitStatusState::Success),
                ("unrelated/context", CommitStatusState::Failure),
            ],
        );
        assert_eq!(reconcile_pr_gate(&snap), PrGateDecision::ReadyForPolicy);
    }

    #[test]
    fn pr_1099_fixture_both_missing() {
        let snap = snapshot_ctx(1099, &["adf/build", "adf/pr-reviewer"], &[]);
        let decision = reconcile_pr_gate(&snap);
        match &decision {
            PrGateDecision::EnqueueMissingChecks { missing } => {
                assert_eq!(missing.len(), 2);
                assert!(missing.contains(&"adf/build".to_string()));
                assert!(missing.contains(&"adf/pr-reviewer".to_string()));
            }
            other => panic!("expected EnqueueMissingChecks, got {:?}", other),
        }
    }

    #[test]
    fn commit_status_state_parsing() {
        assert_eq!(
            CommitStatusState::from_api_str("success"),
            CommitStatusState::Success
        );
        assert_eq!(
            CommitStatusState::from_api_str("failure"),
            CommitStatusState::Failure
        );
        assert_eq!(
            CommitStatusState::from_api_str("error"),
            CommitStatusState::Error
        );
        assert_eq!(
            CommitStatusState::from_api_str("pending"),
            CommitStatusState::Pending
        );
        assert_eq!(
            CommitStatusState::from_api_str("unknown"),
            CommitStatusState::Pending
        );
    }

    #[test]
    fn commit_status_terminal_check() {
        assert!(!CommitStatusState::Pending.is_terminal());
        assert!(CommitStatusState::Success.is_terminal());
        assert!(CommitStatusState::Failure.is_terminal());
        assert!(CommitStatusState::Error.is_terminal());
    }

    #[test]
    fn remediation_key_dedup_format() {
        let snap = snapshot_ctx(
            42,
            &["adf/build"],
            &[("adf/build", CommitStatusState::Failure)],
        );
        let decision = reconcile_pr_gate(&snap);
        let key = remediation_key("test-project", 42, "abc123def456", &decision);
        assert!(key.starts_with("pr-gate:test-project:42:abc123def456:"));
        assert!(key.contains("adf/build"));
    }

    #[test]
    fn remediation_key_factory_fault() {
        let decision = PrGateDecision::FactoryFault {
            error: "connection refused".into(),
        };
        let key = remediation_key("test-project", 42, "abc123def456", &decision);
        assert!(key.contains("factory-fault"));
        assert!(key.contains("connection refused"));
    }

    #[test]
    fn remediation_keys_different_for_different_shas() {
        let d1 = PrGateDecision::BlockedByFailedChecks {
            failed: vec![("adf/build".into(), "failure".into())],
        };
        let key1 = remediation_key("p", 42, "aaa111", &d1);
        let key2 = remediation_key("p", 42, "bbb222", &d1);
        assert_ne!(key1, key2);
    }

    #[test]
    fn stale_pending_triggers_factory_fault() {
        let snap = snapshot_ctx_with_times(
            42,
            &["adf/build", "adf/pr-reviewer"],
            &[
                ("adf/build", CommitStatusState::Pending, Some(5_000)),
                ("adf/pr-reviewer", CommitStatusState::Success, Some(5_000)),
            ],
            10_000,
        );
        let decision = reconcile_pr_gate(&snap);
        match &decision {
            PrGateDecision::FactoryFault { error } => {
                assert!(error.contains("adf/build"));
                assert!(error.contains("stale pending"));
            }
            other => panic!("expected FactoryFault, got {:?}", other),
        }
    }

    #[test]
    fn recent_pending_does_not_trigger_stale() {
        let snap = snapshot_ctx_with_times(
            42,
            &["adf/build", "adf/pr-reviewer"],
            &[
                ("adf/build", CommitStatusState::Pending, Some(9_500)),
                ("adf/pr-reviewer", CommitStatusState::Success, Some(5_000)),
            ],
            10_000,
        );
        let decision = reconcile_pr_gate(&snap);
        assert_eq!(
            decision,
            PrGateDecision::AwaitingChecks {
                pending: vec!["adf/build".into()]
            }
        );
    }

    #[test]
    fn stale_without_created_at_does_not_trigger() {
        let snap = snapshot_ctx_with_times(
            42,
            &["adf/build"],
            &[("adf/build", CommitStatusState::Pending, None)],
            10_000,
        );
        let decision = reconcile_pr_gate(&snap);
        assert_eq!(
            decision,
            PrGateDecision::AwaitingChecks {
                pending: vec!["adf/build".into()]
            }
        );
    }

    // ------------------------------------------------------------------
    // latest_status_per_context helper tests
    // ------------------------------------------------------------------

    #[test]
    fn latest_status_per_context_prefers_higher_timestamp() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Failure, Some(1_000)),
            ("adf/build", CommitStatusState::Success, Some(2_000)),
        ]);
        let map = latest_status_per_context(&statuses);
        assert_eq!(map.len(), 1);
        assert_eq!(map["adf/build"].state, CommitStatusState::Success);
        assert_eq!(map["adf/build"].created_at_unix, Some(2_000));
    }

    #[test]
    fn latest_status_per_context_prefers_timestamp_over_none() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Success, None),
            ("adf/build", CommitStatusState::Failure, Some(1_000)),
        ]);
        let map = latest_status_per_context(&statuses);
        assert_eq!(map.len(), 1);
        assert_eq!(map["adf/build"].state, CommitStatusState::Failure);
        assert_eq!(map["adf/build"].created_at_unix, Some(1_000));
    }

    #[test]
    fn latest_status_per_context_keeps_existing_timestamp_over_none() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Failure, Some(1_000)),
            ("adf/build", CommitStatusState::Success, None),
        ]);
        let map = latest_status_per_context(&statuses);
        assert_eq!(map.len(), 1);
        assert_eq!(map["adf/build"].state, CommitStatusState::Failure);
        assert_eq!(map["adf/build"].created_at_unix, Some(1_000));
    }

    #[test]
    fn latest_status_per_context_last_wins_when_both_no_timestamp() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Failure, None),
            ("adf/build", CommitStatusState::Success, None),
        ]);
        let map = latest_status_per_context(&statuses);
        assert_eq!(map.len(), 1);
        assert_eq!(map["adf/build"].state, CommitStatusState::Success);
        assert_eq!(map["adf/build"].created_at_unix, None);
    }

    #[test]
    fn latest_status_per_context_different_contexts_independent() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Failure, Some(1_000)),
            ("adf/build", CommitStatusState::Success, Some(2_000)),
            ("adf/pr-reviewer", CommitStatusState::Pending, Some(500)),
            ("adf/pr-reviewer", CommitStatusState::Success, Some(1_500)),
        ]);
        let map = latest_status_per_context(&statuses);
        assert_eq!(map.len(), 2);
        assert_eq!(map["adf/build"].state, CommitStatusState::Success);
        assert_eq!(map["adf/pr-reviewer"].state, CommitStatusState::Success);
    }

    // ------------------------------------------------------------------
    // Integration: duplicate contexts in gate functions
    // ------------------------------------------------------------------

    #[test]
    fn pending_uses_latest_status_for_duplicate_contexts() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Pending, Some(1_000)),
            ("adf/build", CommitStatusState::Success, Some(2_000)),
        ]);
        let pending = pending_required_contexts(&required(&["adf/build"]), &statuses);
        assert!(pending.is_empty());
    }

    #[test]
    fn pending_uses_latest_status_when_still_pending() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Success, Some(1_000)),
            ("adf/build", CommitStatusState::Pending, Some(2_000)),
        ]);
        let pending = pending_required_contexts(&required(&["adf/build"]), &statuses);
        assert_eq!(pending, vec!["adf/build"]);
    }

    #[test]
    fn stale_uses_latest_status_for_duplicate_contexts() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Pending, Some(1_000)),
            ("adf/build", CommitStatusState::Pending, Some(9_500)),
        ]);
        let stale = stale_pending_contexts(
            &required(&["adf/build"]),
            &statuses,
            10_000,
            STALE_PENDING_TIMEOUT_SECS,
        );
        assert!(stale.is_empty());
    }

    #[test]
    fn stale_uses_latest_status_when_still_stale() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Pending, Some(9_500)),
            ("adf/build", CommitStatusState::Pending, Some(1_000)),
        ]);
        let stale = stale_pending_contexts(
            &required(&["adf/build"]),
            &statuses,
            10_000,
            STALE_PENDING_TIMEOUT_SECS,
        );
        assert!(stale.is_empty());
    }

    #[test]
    fn failed_uses_latest_status_for_duplicate_contexts() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Failure, Some(1_000)),
            ("adf/build", CommitStatusState::Success, Some(2_000)),
        ]);
        let failed = failed_required_contexts(&required(&["adf/build"]), &statuses);
        assert!(failed.is_empty());
    }

    #[test]
    fn failed_uses_latest_status_when_still_failed() {
        let statuses = statuses_with_times(&[
            ("adf/build", CommitStatusState::Success, Some(1_000)),
            ("adf/build", CommitStatusState::Failure, Some(2_000)),
        ]);
        let failed = failed_required_contexts(&required(&["adf/build"]), &statuses);
        assert_eq!(failed, vec![("adf/build".into(), "failure".into())]);
    }

    #[test]
    fn reconcile_uses_latest_status_across_duplicate_contexts() {
        let snap = snapshot_ctx_with_times(
            42,
            &["adf/build", "adf/pr-reviewer"],
            &[
                ("adf/build", CommitStatusState::Failure, Some(1_000)),
                ("adf/build", CommitStatusState::Success, Some(2_000)),
                ("adf/pr-reviewer", CommitStatusState::Pending, Some(1_000)),
                ("adf/pr-reviewer", CommitStatusState::Success, Some(2_000)),
            ],
            10_000,
        );
        assert_eq!(reconcile_pr_gate(&snap), PrGateDecision::ReadyForPolicy);
    }
}
