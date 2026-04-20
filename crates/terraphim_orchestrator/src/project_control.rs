//! Project-level operational controls: pause flags and circuit breakers.
//!
//! A pause flag is an empty sentinel file at `<pause_dir>/<project_id>` that
//! causes the orchestrator to skip all dispatches for that project until the
//! file is removed. The default directory is `/opt/ai-dark-factory/data/pause`
//! and is configurable via [`crate::config::OrchestratorConfig::pause_dir`].
//!
//! The project circuit breaker tracks consecutive `project-meta` failures per
//! project. When the threshold is reached, the orchestrator touches the pause
//! flag and opens an `[ADF]` Gitea issue to notify operators.
//!
//! The counter is intentionally narrow: it only advances on `project-meta`
//! exits so that a flaky developer agent or reviewer cannot trip a
//! project-wide pause.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::config::AgentDefinition;

/// Default directory containing per-project pause flag files.
pub const DEFAULT_PAUSE_DIR: &str = "/opt/ai-dark-factory/data/pause";

/// Default number of consecutive `project-meta` failures that trips the
/// project-level pause.
pub const DEFAULT_PROJECT_CIRCUIT_BREAKER_THRESHOLD: u32 = 3;

/// Return `true` when a pause flag file exists for the given project id.
///
/// Passing `None` always returns `false` (legacy/global agents are never
/// paused by this mechanism).
pub fn is_project_paused(pause_dir: &Path, project_id: Option<&str>) -> bool {
    let Some(pid) = project_id else {
        return false;
    };
    pause_dir.join(pid).exists()
}

/// Create the pause flag file for a project. Returns the final path on
/// success. Errors include missing parent dir permissions.
pub fn touch_pause_flag(pause_dir: &Path, project_id: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(pause_dir)?;
    let path = pause_dir.join(project_id);
    // Create-or-touch. We do not care about content; existence is the signal.
    fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)?;
    Ok(path)
}

/// Outcome returned by [`ProjectFailureCounter::record_project_meta_result`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldPause {
    /// Keep running; the counter has not reached the threshold.
    No,
    /// The threshold was just reached. The caller must touch the pause flag
    /// and open the escalation issue. Subsequent failures will not re-trip
    /// until the counter is reset by a success.
    Yes,
}

/// Heuristic check: does this agent definition correspond to a per-project
/// `project-meta` agent?
///
/// Exact name `project-meta` or any `project-meta-<suffix>` form qualifies.
/// The suffix form supports multi-instance deployments (e.g.
/// `project-meta-odilo`, `project-meta-digital-twins`).
pub fn is_project_meta_agent(def: &AgentDefinition) -> bool {
    def.name == "project-meta" || def.name.starts_with("project-meta-")
}

/// Consecutive-failure counter scoped to a single project's `project-meta`
/// agent.
#[derive(Debug, Clone)]
pub struct ProjectFailureCounter {
    threshold: u32,
    counts: HashMap<String, u32>,
    tripped: HashMap<String, bool>,
}

impl ProjectFailureCounter {
    /// Create a new counter with the given trip threshold. Zero is treated as
    /// one (any failure trips immediately), which exists mainly for tests.
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold: threshold.max(1),
            counts: HashMap::new(),
            tripped: HashMap::new(),
        }
    }

    /// Record the outcome of a single `project-meta` run. Success resets the
    /// counter (and the tripped flag). Failure increments and returns
    /// [`ShouldPause::Yes`] exactly once when the threshold is crossed.
    pub fn record_project_meta_result(&mut self, project_id: &str, success: bool) -> ShouldPause {
        if success {
            self.counts.remove(project_id);
            self.tripped.remove(project_id);
            return ShouldPause::No;
        }
        let entry = self.counts.entry(project_id.to_string()).or_insert(0);
        *entry += 1;
        let already_tripped = self.tripped.get(project_id).copied().unwrap_or(false);
        if *entry >= self.threshold && !already_tripped {
            self.tripped.insert(project_id.to_string(), true);
            ShouldPause::Yes
        } else {
            ShouldPause::No
        }
    }

    /// Current failure count for a project (for diagnostics / tests).
    pub fn count(&self, project_id: &str) -> u32 {
        self.counts.get(project_id).copied().unwrap_or(0)
    }

    /// Trip threshold.
    pub fn threshold(&self) -> u32 {
        self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentDefinition, AgentLayer};
    use tempfile::tempdir;

    fn def_with_name(name: &str) -> AgentDefinition {
        AgentDefinition {
            name: name.to_string(),
            layer: AgentLayer::Core,
            cli_tool: "/bin/true".to_string(),
            task: String::new(),
            schedule: None,
            model: None,
            capabilities: Vec::new(),
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: None,
            terraphim_role: None,
            skill_chain: Vec::new(),
            sfia_skills: Vec::new(),
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            pre_check: None,
            gitea_issue: None,
            project: Some("odilo".to_string()),
        }
    }

    #[test]
    fn pause_flag_absent_means_not_paused() {
        let dir = tempdir().unwrap();
        assert!(!is_project_paused(dir.path(), Some("odilo")));
    }

    #[test]
    fn pause_flag_present_means_paused() {
        let dir = tempdir().unwrap();
        touch_pause_flag(dir.path(), "odilo").unwrap();
        assert!(is_project_paused(dir.path(), Some("odilo")));
    }

    #[test]
    fn pause_flag_not_shared_across_projects() {
        let dir = tempdir().unwrap();
        touch_pause_flag(dir.path(), "odilo").unwrap();
        assert!(is_project_paused(dir.path(), Some("odilo")));
        assert!(!is_project_paused(dir.path(), Some("digital-twins")));
    }

    #[test]
    fn pause_flag_ignored_for_legacy_global_agents() {
        let dir = tempdir().unwrap();
        touch_pause_flag(dir.path(), "__global__").unwrap();
        assert!(!is_project_paused(dir.path(), None));
    }

    #[test]
    fn touching_pause_flag_creates_parent_dir() {
        let root = tempdir().unwrap();
        let nested = root.path().join("deep/dir/pause");
        let flag = touch_pause_flag(&nested, "odilo").unwrap();
        assert!(flag.exists());
    }

    #[test]
    fn identifies_project_meta_agent_by_name() {
        assert!(is_project_meta_agent(&def_with_name("project-meta")));
        assert!(is_project_meta_agent(&def_with_name("project-meta-odilo")));
        assert!(!is_project_meta_agent(&def_with_name("fleet-meta")));
        assert!(!is_project_meta_agent(&def_with_name("meta-coordinator")));
        assert!(!is_project_meta_agent(&def_with_name("developer")));
    }

    #[test]
    fn counter_resets_on_success() {
        let mut c = ProjectFailureCounter::new(3);
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::No
        );
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::No
        );
        assert_eq!(c.count("odilo"), 2);
        assert_eq!(c.record_project_meta_result("odilo", true), ShouldPause::No);
        assert_eq!(c.count("odilo"), 0);
    }

    #[test]
    fn counter_trips_at_threshold() {
        let mut c = ProjectFailureCounter::new(3);
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::No
        );
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::No
        );
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::Yes
        );
    }

    #[test]
    fn counter_does_not_double_trip_without_success() {
        let mut c = ProjectFailureCounter::new(2);
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::No
        );
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::Yes
        );
        // Further failures past the trip do not re-fire.
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::No
        );
    }

    #[test]
    fn counter_is_per_project() {
        let mut c = ProjectFailureCounter::new(2);
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::No
        );
        assert_eq!(
            c.record_project_meta_result("digital-twins", false),
            ShouldPause::No
        );
        assert_eq!(c.count("odilo"), 1);
        assert_eq!(c.count("digital-twins"), 1);
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::Yes
        );
        assert_eq!(
            c.record_project_meta_result("digital-twins", false),
            ShouldPause::Yes
        );
    }

    #[test]
    fn zero_threshold_trips_on_first_failure() {
        let mut c = ProjectFailureCounter::new(0);
        assert_eq!(c.threshold(), 1);
        assert_eq!(
            c.record_project_meta_result("odilo", false),
            ShouldPause::Yes
        );
    }
}
