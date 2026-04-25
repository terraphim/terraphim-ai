//! Pure helpers for wiring `DispatchTask::ReviewPr` into the spawn pipeline.
//!
//! The orchestrator owns the heavyweight `spawn_agent` flow (budget gates,
//! pre-check, persona composition, worktrees). Everything in this module is
//! deliberately side-effect-free so it can be unit-tested without spinning up
//! an `AgentOrchestrator` or touching the filesystem. The companion method
//! `AgentOrchestrator::handle_review_pr` stitches the helpers together with
//! the routing engine, allow-list gate, and spawner.
//!
//! See `cto-executive-system/plans/adf-rate-of-change-design.md` §Step 4 and
//! Gitea issue `terraphim/adf-fleet#32` for the acceptance criteria.

use std::collections::HashMap;

use terraphim_spawner::SpawnContext;

use crate::config::{AgentDefinition, OrchestratorConfig};

/// Per-dispatch metadata for a PR-review task, mirroring
/// [`crate::dispatcher::DispatchTask::ReviewPr`] so the helpers below don't
/// have to know about the dispatcher enum variant shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewPrRequest {
    pub pr_number: u64,
    pub project: String,
    pub head_sha: String,
    pub author_login: String,
    pub title: String,
    pub diff_loc: u32,
}

/// Locate the `pr-reviewer` [`AgentDefinition`] for a given project.
///
/// Returns `None` when no matching agent is configured. Step E lands the
/// canonical `pr-reviewer.toml`; until then a missing definition must degrade
/// gracefully (log-and-skip) rather than blow up the dispatcher.
pub fn find_pr_reviewer<'a>(
    config: &'a OrchestratorConfig,
    project: &str,
) -> Option<&'a AgentDefinition> {
    config
        .agents
        .iter()
        .find(|a| a.name == "pr-reviewer" && a.project.as_deref() == Some(project))
}

/// Build the task prompt fed into `RoutingDecisionEngine::decide_route` and
/// ultimately to the spawned pr-reviewer process.
///
/// The prompt embeds "review" keywords (so KG and keyword routers can match it)
/// plus the full PR metadata so downstream skills can reference the PR without
/// reading environment variables.
pub fn build_review_task(req: &ReviewPrRequest) -> String {
    format!(
        "Structural review of PR #{}: {} (project={}, size={} LOC, author={}, head={})",
        req.pr_number, req.title, req.project, req.diff_loc, req.author_login, req.head_sha,
    )
}

/// Environment variables the pr-reviewer process receives for a given PR.
///
/// Keys use the `ADF_PR_*` prefix so skills running inside the agent can key
/// off them without needing to parse the task string.
pub fn pr_env_overrides(req: &ReviewPrRequest) -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("ADF_PR_NUMBER".to_string(), req.pr_number.to_string());
    env.insert("ADF_PR_HEAD_SHA".to_string(), req.head_sha.clone());
    env.insert("ADF_PR_PROJECT".to_string(), req.project.clone());
    env.insert("ADF_PR_AUTHOR".to_string(), req.author_login.clone());
    env.insert("ADF_PR_DIFF_LOC".to_string(), req.diff_loc.to_string());
    env.insert("ADF_PR_TITLE".to_string(), req.title.clone());
    env
}

/// Layer the per-PR `ADF_PR_*` env overrides on top of a base
/// [`SpawnContext`] without clobbering existing keys the orchestrator already
/// set (e.g. `ADF_PROJECT_ID`, `GITEA_OWNER`, `GITEA_REPO`).
pub fn layer_pr_env(mut base: SpawnContext, req: &ReviewPrRequest) -> SpawnContext {
    for (k, v) in pr_env_overrides(req) {
        base = base.with_env(k, v);
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> ReviewPrRequest {
        ReviewPrRequest {
            pr_number: 641,
            project: "terraphim".to_string(),
            head_sha: "deadbeef1234".to_string(),
            author_login: "claude-code".to_string(),
            title: "fix(kg): short synonyms".to_string(),
            diff_loc: 42,
        }
    }

    fn multi_project_config() -> OrchestratorConfig {
        let toml_str = r#"
working_dir = "/tmp/pr-dispatch-tests"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[[projects]]
id = "beta"
working_dir = "/tmp/beta"

[[agents]]
name = "pr-reviewer"
layer = "Safety"
cli_tool = "echo"
task = "review"
project = "alpha"

[[agents]]
name = "pr-reviewer"
layer = "Safety"
cli_tool = "echo"
task = "review"
project = "beta"

[[agents]]
name = "other"
layer = "Safety"
cli_tool = "echo"
task = "other"
project = "alpha"
"#;
        OrchestratorConfig::from_toml(toml_str).unwrap()
    }

    #[test]
    fn build_review_task_embeds_routing_keywords_and_metadata() {
        let t = build_review_task(&sample_request());
        assert!(
            t.contains("review"),
            "task must contain routing keyword 'review'"
        );
        assert!(t.contains("PR #641"));
        assert!(t.contains("project=terraphim"));
        assert!(t.contains("42 LOC"));
        assert!(t.contains("deadbeef1234"));
    }

    #[test]
    fn pr_env_overrides_populates_all_adf_pr_keys() {
        let e = pr_env_overrides(&sample_request());
        assert_eq!(e.get("ADF_PR_NUMBER"), Some(&"641".to_string()));
        assert_eq!(e.get("ADF_PR_HEAD_SHA"), Some(&"deadbeef1234".to_string()));
        assert_eq!(e.get("ADF_PR_PROJECT"), Some(&"terraphim".to_string()));
        assert_eq!(e.get("ADF_PR_AUTHOR"), Some(&"claude-code".to_string()));
        assert_eq!(e.get("ADF_PR_DIFF_LOC"), Some(&"42".to_string()));
        assert_eq!(
            e.get("ADF_PR_TITLE"),
            Some(&"fix(kg): short synonyms".to_string())
        );
    }

    #[test]
    fn layer_pr_env_preserves_base_env_and_adds_pr_keys() {
        let base = SpawnContext::default()
            .with_env("ADF_PROJECT_ID", "terraphim")
            .with_env("GITEA_OWNER", "terraphim");
        let ctx = layer_pr_env(base, &sample_request());
        assert_eq!(
            ctx.env_overrides.get("ADF_PROJECT_ID"),
            Some(&"terraphim".to_string())
        );
        assert_eq!(
            ctx.env_overrides.get("GITEA_OWNER"),
            Some(&"terraphim".to_string())
        );
        assert_eq!(
            ctx.env_overrides.get("ADF_PR_NUMBER"),
            Some(&"641".to_string())
        );
    }

    #[test]
    fn find_pr_reviewer_matches_on_name_and_project() {
        let config = multi_project_config();

        assert_eq!(
            find_pr_reviewer(&config, "alpha")
                .unwrap()
                .project
                .as_deref(),
            Some("alpha")
        );
        assert_eq!(
            find_pr_reviewer(&config, "beta")
                .unwrap()
                .project
                .as_deref(),
            Some("beta")
        );
        assert!(find_pr_reviewer(&config, "gamma").is_none());
    }
}
