//! Candidate issue selection and dispatch logic.
//!
//! Implements the dispatch eligibility rules and sorting from
//! Symphony spec Section 8.2.

use crate::orchestrator::state::OrchestratorRuntimeState;
use crate::tracker::Issue;
use std::collections::HashMap;

/// Check if an issue is eligible for dispatch.
pub fn is_dispatch_eligible(
    issue: &Issue,
    state: &OrchestratorRuntimeState,
    active_states: &[String],
    terminal_states: &[String],
    per_state_limits: &HashMap<String, usize>,
) -> bool {
    // Must have required fields
    if !issue.is_dispatchable() {
        return false;
    }

    // State must be active and not terminal
    let is_active = active_states
        .iter()
        .any(|s| s.eq_ignore_ascii_case(&issue.state));
    if !is_active {
        return false;
    }

    let is_terminal = terminal_states
        .iter()
        .any(|s| s.eq_ignore_ascii_case(&issue.state));
    if is_terminal {
        return false;
    }

    // Not already running or claimed
    if state.running.contains_key(&issue.id) || state.is_claimed(&issue.id) {
        return false;
    }

    // Global concurrency check
    if state.available_slots() == 0 {
        return false;
    }

    // Per-state concurrency check
    let state_lower = issue.state.to_lowercase();
    if let Some(&limit) = per_state_limits.get(&state_lower) {
        let current = state.running_count_by_state(&issue.state);
        if current >= limit {
            return false;
        }
    }

    // Todo blocker rule: if state is "todo", all blockers must be terminal
    if issue.state.eq_ignore_ascii_case("todo")
        && !issue.blocked_by.is_empty()
        && !issue.all_blockers_terminal(terminal_states)
    {
        return false;
    }

    true
}

/// Sort issues for dispatch priority.
///
/// Sort order (stable):
/// 1. PageRank descending (higher score = more downstream impact; None sorts last)
/// 2. Priority ascending (lower = higher priority; None sorts last)
/// 3. Created at oldest first
/// 4. Identifier lexicographic tiebreaker
pub fn sort_for_dispatch(issues: &mut [Issue]) {
    #[allow(clippy::unnecessary_sort_by)]
    issues.sort_by(|a, b| {
        // PageRank: higher score first (more downstream impact)
        let pra = a.pagerank_score.unwrap_or(0.0);
        let prb = b.pagerank_score.unwrap_or(0.0);
        prb.partial_cmp(&pra)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                // Priority: Some(n) before None, lower n first
                let pa = a.priority.unwrap_or(i32::MAX);
                let pb = b.priority.unwrap_or(i32::MAX);
                pa.cmp(&pb)
            })
            .then_with(|| {
                // Created at: oldest first
                let ca = a.created_at.unwrap_or_default();
                let cb = b.created_at.unwrap_or_default();
                ca.cmp(&cb)
            })
            .then_with(|| {
                // Identifier: lexicographic
                a.identifier.cmp(&b.identifier)
            })
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::BlockerRef;
    use chrono::Utc;

    fn make_issue(id: &str, identifier: &str, state: &str, priority: Option<i32>) -> Issue {
        Issue {
            id: id.into(),
            identifier: identifier.into(),
            title: format!("Issue {identifier}"),
            description: None,
            priority,
            state: state.into(),
            branch_name: None,
            url: None,
            labels: vec![],
            blocked_by: vec![],
            pagerank_score: None,
            created_at: Some(Utc::now()),
            updated_at: None,
        }
    }

    fn default_active() -> Vec<String> {
        vec!["Todo".into(), "In Progress".into()]
    }

    fn default_terminal() -> Vec<String> {
        vec!["Done".into(), "Closed".into(), "Cancelled".into()]
    }

    #[test]
    fn eligible_basic() {
        let state = OrchestratorRuntimeState::new(30_000, 10);
        let issue = make_issue("1", "MT-1", "Todo", Some(1));
        assert!(is_dispatch_eligible(
            &issue,
            &state,
            &default_active(),
            &default_terminal(),
            &HashMap::new(),
        ));
    }

    #[test]
    fn ineligible_terminal_state() {
        let state = OrchestratorRuntimeState::new(30_000, 10);
        let issue = make_issue("1", "MT-1", "Done", Some(1));
        assert!(!is_dispatch_eligible(
            &issue,
            &state,
            &default_active(),
            &default_terminal(),
            &HashMap::new(),
        ));
    }

    #[test]
    fn ineligible_non_active_state() {
        let state = OrchestratorRuntimeState::new(30_000, 10);
        let issue = make_issue("1", "MT-1", "Review", Some(1));
        assert!(!is_dispatch_eligible(
            &issue,
            &state,
            &default_active(),
            &default_terminal(),
            &HashMap::new(),
        ));
    }

    #[test]
    fn ineligible_no_slots() {
        let state = OrchestratorRuntimeState::new(30_000, 0);
        let issue = make_issue("1", "MT-1", "Todo", Some(1));
        assert!(!is_dispatch_eligible(
            &issue,
            &state,
            &default_active(),
            &default_terminal(),
            &HashMap::new(),
        ));
    }

    #[test]
    fn ineligible_already_claimed() {
        let mut state = OrchestratorRuntimeState::new(30_000, 10);
        state.claimed.insert("1".into());
        let issue = make_issue("1", "MT-1", "Todo", Some(1));
        assert!(!is_dispatch_eligible(
            &issue,
            &state,
            &default_active(),
            &default_terminal(),
            &HashMap::new(),
        ));
    }

    #[test]
    fn todo_with_non_terminal_blocker_ineligible() {
        let state = OrchestratorRuntimeState::new(30_000, 10);
        let mut issue = make_issue("1", "MT-1", "Todo", Some(1));
        issue.blocked_by = vec![BlockerRef {
            id: Some("2".into()),
            identifier: Some("MT-2".into()),
            state: Some("In Progress".into()),
        }];
        assert!(!is_dispatch_eligible(
            &issue,
            &state,
            &default_active(),
            &default_terminal(),
            &HashMap::new(),
        ));
    }

    #[test]
    fn todo_with_terminal_blockers_eligible() {
        let state = OrchestratorRuntimeState::new(30_000, 10);
        let mut issue = make_issue("1", "MT-1", "Todo", Some(1));
        issue.blocked_by = vec![BlockerRef {
            id: Some("2".into()),
            identifier: Some("MT-2".into()),
            state: Some("Done".into()),
        }];
        assert!(is_dispatch_eligible(
            &issue,
            &state,
            &default_active(),
            &default_terminal(),
            &HashMap::new(),
        ));
    }

    #[test]
    fn per_state_limit_enforced() {
        let state = OrchestratorRuntimeState::new(30_000, 10);
        let mut limits = HashMap::new();
        limits.insert("todo".into(), 0usize);
        let issue = make_issue("1", "MT-1", "Todo", Some(1));
        assert!(!is_dispatch_eligible(
            &issue,
            &state,
            &default_active(),
            &default_terminal(),
            &limits,
        ));
    }

    #[test]
    fn sort_by_priority() {
        let mut issues = vec![
            make_issue("3", "MT-3", "Todo", Some(3)),
            make_issue("1", "MT-1", "Todo", Some(1)),
            make_issue("2", "MT-2", "Todo", None),
        ];
        sort_for_dispatch(&mut issues);
        assert_eq!(issues[0].identifier, "MT-1");
        assert_eq!(issues[1].identifier, "MT-3");
        assert_eq!(issues[2].identifier, "MT-2"); // None sorts last
    }

    #[test]
    fn sort_by_created_at_tiebreak() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::hours(1);

        let mut issues = vec![
            {
                let mut i = make_issue("1", "MT-1", "Todo", Some(1));
                i.created_at = Some(now);
                i
            },
            {
                let mut i = make_issue("2", "MT-2", "Todo", Some(1));
                i.created_at = Some(earlier);
                i
            },
        ];
        sort_for_dispatch(&mut issues);
        assert_eq!(issues[0].identifier, "MT-2"); // older first
        assert_eq!(issues[1].identifier, "MT-1");
    }

    #[test]
    fn sort_identifier_tiebreak() {
        let now = Utc::now();
        let mut issues = vec![
            {
                let mut i = make_issue("2", "MT-B", "Todo", Some(1));
                i.created_at = Some(now);
                i
            },
            {
                let mut i = make_issue("1", "MT-A", "Todo", Some(1));
                i.created_at = Some(now);
                i
            },
        ];
        sort_for_dispatch(&mut issues);
        assert_eq!(issues[0].identifier, "MT-A");
        assert_eq!(issues[1].identifier, "MT-B");
    }

    #[test]
    fn sort_by_pagerank() {
        let mut issues = vec![
            {
                let mut i = make_issue("1", "MT-1", "Todo", Some(1));
                i.pagerank_score = Some(0.5);
                i
            },
            {
                let mut i = make_issue("2", "MT-2", "Todo", Some(1));
                i.pagerank_score = Some(2.847);
                i
            },
            {
                let mut i = make_issue("3", "MT-3", "Todo", Some(1));
                i.pagerank_score = None; // No PageRank sorts last
                i
            },
        ];
        sort_for_dispatch(&mut issues);
        assert_eq!(issues[0].identifier, "MT-2"); // highest PageRank
        assert_eq!(issues[1].identifier, "MT-1"); // second highest
        assert_eq!(issues[2].identifier, "MT-3"); // None (0.0)
    }

    #[test]
    fn sort_pagerank_tiebreak_priority() {
        let mut issues = vec![
            {
                let mut i = make_issue("1", "MT-1", "Todo", Some(3));
                i.pagerank_score = Some(1.0);
                i
            },
            {
                let mut i = make_issue("2", "MT-2", "Todo", Some(1));
                i.pagerank_score = Some(1.0);
                i
            },
        ];
        sort_for_dispatch(&mut issues);
        // Same PageRank, so priority wins (lower = higher priority)
        assert_eq!(issues[0].identifier, "MT-2"); // priority 1
        assert_eq!(issues[1].identifier, "MT-1"); // priority 3
    }
}
