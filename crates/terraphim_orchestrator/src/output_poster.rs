//! Posts agent output to Gitea issues after agent exit.
//!
//! Supports multi-project configurations where each project owns its own
//! `owner`/`repo`/`token` triple. The legacy single-project mode is retained
//! by keying the default tracker on [`crate::dispatcher::LEGACY_PROJECT_ID`].

use std::collections::HashMap;
use std::path::PathBuf;

use terraphim_tracker::GiteaTracker;
use terraphim_tracker::gitea::{ClaimStrategy, GiteaConfig};

use crate::config::{GiteaOutputConfig, OrchestratorConfig};
use crate::dispatcher::LEGACY_PROJECT_ID;

const ROBOT_PATH: &str = "/home/alex/go/bin/gitea-robot";

/// Trackers for a single project (root token + any per-agent token overrides).
struct ProjectTrackers {
    /// Default tracker using the project-level token from config.
    default_tracker: GiteaTracker,
    /// Per-agent trackers keyed by agent name (scoped to this project).
    agent_trackers: HashMap<String, GiteaTracker>,
    /// Per-agent raw token strings keyed by agent name (parallel to
    /// agent_trackers). Exposed via [`OutputPoster::agent_token`] so the
    /// spawn path can inject the token as a `GITEA_TOKEN` env override,
    /// giving each agent's own `gtr` calls their own identity.
    agent_tokens: HashMap<String, String>,
}

/// Posts collected agent output to a Gitea issue comment.
///
/// Supports per-project Gitea targets (owner/repo) and per-agent tokens so
/// each agent posts under its own user within its owning project. Falls back
/// to the legacy (top-level) Gitea config when no project routing is defined.
pub struct OutputPoster {
    /// Trackers keyed by project id.
    projects: HashMap<String, ProjectTrackers>,
    /// Fallback project id used when the caller doesn't know which project to
    /// post against (e.g. compound review posts on legacy configs). In
    /// multi-project mode this stays `None` and callers must pass a project id.
    fallback_project: Option<String>,
    /// Base URL retained for diagnostics.
    #[allow(dead_code)]
    base_url: String,
}

impl OutputPoster {
    /// Build a single-project poster from a bare Gitea output config.
    ///
    /// Used by tests and by the legacy (single-project) code path — the
    /// resulting poster stores everything under
    /// [`crate::dispatcher::LEGACY_PROJECT_ID`] and uses that as the fallback
    /// project id.
    pub fn new(config: &GiteaOutputConfig) -> Self {
        let trackers = build_project_trackers(config);

        let mut projects = HashMap::new();
        projects.insert(LEGACY_PROJECT_ID.to_string(), trackers);

        Self {
            projects,
            fallback_project: Some(LEGACY_PROJECT_ID.to_string()),
            base_url: config.base_url.clone(),
        }
    }

    /// Build a poster from a full orchestrator config, wiring one entry per
    /// project as well as a legacy fallback from the top-level `config.gitea`.
    ///
    /// When `config.projects` is empty this behaves exactly like
    /// [`OutputPoster::new`] against the top-level gitea config.
    pub fn from_orchestrator_config(config: &OrchestratorConfig) -> Option<Self> {
        let mut projects: HashMap<String, ProjectTrackers> = HashMap::new();
        let mut base_url = String::new();

        // Per-project trackers.
        for project in &config.projects {
            let Some(gitea_cfg) = project.gitea.as_ref().or(config.gitea.as_ref()) else {
                continue;
            };
            base_url = gitea_cfg.base_url.clone();
            projects.insert(project.id.clone(), build_project_trackers(gitea_cfg));
        }

        // Legacy fallback from top-level gitea block.
        let fallback_project = if let Some(ref top) = config.gitea {
            base_url = top.base_url.clone();
            projects.insert(LEGACY_PROJECT_ID.to_string(), build_project_trackers(top));
            Some(LEGACY_PROJECT_ID.to_string())
        } else {
            // No top-level config: pick an arbitrary project id as fallback so
            // callers that don't know their project (e.g. compound review on a
            // multi-project fleet) still have somewhere to post.
            projects.keys().next().cloned()
        };

        if projects.is_empty() {
            return None;
        }

        Some(Self {
            projects,
            fallback_project,
            base_url,
        })
    }

    /// Return the tracker to use for `(project, agent)`.
    ///
    /// Resolution order:
    /// 1. Per-agent tracker within the requested project.
    /// 2. Project default tracker.
    /// 3. Fallback project's default tracker (legacy behaviour).
    pub fn tracker_for(&self, project: &str, agent_name: &str) -> Option<&GiteaTracker> {
        if let Some(p) = self.projects.get(project) {
            if let Some(agent_tracker) = p.agent_trackers.get(agent_name) {
                return Some(agent_tracker);
            }
            return Some(&p.default_tracker);
        }
        let fallback = self.fallback_project.as_deref()?;
        let p = self.projects.get(fallback)?;
        Some(
            p.agent_trackers
                .get(agent_name)
                .unwrap_or(&p.default_tracker),
        )
    }

    /// Return the default (root-token) tracker for the requested project,
    /// falling back to the fallback project.
    fn default_tracker_for(&self, project: &str) -> Option<&GiteaTracker> {
        if let Some(p) = self.projects.get(project) {
            return Some(&p.default_tracker);
        }
        let fallback = self.fallback_project.as_deref()?;
        self.projects.get(fallback).map(|p| &p.default_tracker)
    }

    /// Whether the tracker for `(project, agent)` is using an agent-specific
    /// token (rather than the project default).
    fn has_own_token(&self, project: &str, agent_name: &str) -> bool {
        self.projects
            .get(project)
            .is_some_and(|p| p.agent_trackers.contains_key(agent_name))
    }

    /// Return the agent's own Gitea token for this project, if one is
    /// configured in `agent_tokens.json`. Used by the spawn path to inject
    /// `GITEA_TOKEN` into the child process so the agent's own `gtr` /
    /// API calls post under its own Gitea user identity (matching what
    /// [`OutputPoster`] does for the wrapped completion comment).
    pub fn agent_token(&self, project: &str, agent_name: &str) -> Option<&str> {
        self.projects
            .get(project)
            .and_then(|p| p.agent_tokens.get(agent_name))
            .map(|s| s.as_str())
    }

    /// Post agent output as a comment on a Gitea issue in the given project.
    ///
    /// Uses the agent's own Gitea token if configured, otherwise falls back
    /// to the project default. Truncates output to 60000 bytes to stay within
    /// Gitea's comment size limit.
    pub async fn post_agent_output_for_project(
        &self,
        project: &str,
        agent_name: &str,
        issue_number: u64,
        output_lines: &[String],
        exit_code: Option<i32>,
    ) -> Result<(), String> {
        if output_lines.is_empty() {
            tracing::debug!(
                project = project,
                agent = %agent_name,
                issue = issue_number,
                "no output to post"
            );
            return Ok(());
        }

        let exit_str = match exit_code {
            Some(code) => format!("exit code {}", code),
            None => "unknown exit".to_string(),
        };

        let mut body = format!(
            "**Agent `{}`** completed ({}).\n\n<details>\n<summary>Output ({} lines)</summary>\n\n```\n",
            agent_name,
            exit_str,
            output_lines.len()
        );

        let joined = output_lines.join("\n");
        let max_output = 60000;
        if joined.len() > max_output {
            body.push_str(&joined[..max_output]);
            body.push_str("\n... (truncated)\n");
        } else {
            body.push_str(&joined);
        }
        body.push_str("\n```\n\n</details>");

        let Some(tracker) = self.tracker_for(project, agent_name) else {
            let msg = format!(
                "no Gitea tracker configured for project {} and no fallback available",
                project
            );
            tracing::error!("{}", msg);
            return Err(msg);
        };

        match tracker.post_comment(issue_number, &body).await {
            Ok(comment) => {
                tracing::info!(
                    project = project,
                    agent = %agent_name,
                    issue = issue_number,
                    comment_id = comment.id,
                    own_token = self.has_own_token(project, agent_name),
                    "posted agent output to Gitea"
                );
                Ok(())
            }
            Err(e) => {
                let msg = format!(
                    "failed to post output for {} in project {}: {}",
                    agent_name, project, e
                );
                tracing::error!("{}", msg);
                Err(msg)
            }
        }
    }

    /// Legacy-compatible post using the fallback project.
    pub async fn post_agent_output(
        &self,
        agent_name: &str,
        issue_number: u64,
        output_lines: &[String],
        exit_code: Option<i32>,
    ) -> Result<(), String> {
        let project = self.fallback_project.clone().unwrap_or_else(|| {
            tracing::warn!(
                agent = %agent_name,
                "no fallback project for legacy post_agent_output; defaulting to {}",
                LEGACY_PROJECT_ID
            );
            LEGACY_PROJECT_ID.to_string()
        });
        self.post_agent_output_for_project(
            &project,
            agent_name,
            issue_number,
            output_lines,
            exit_code,
        )
        .await
    }

    /// Post raw markdown as a comment on a Gitea issue using the project
    /// default (root-token) tracker.
    pub async fn post_raw_for_project(
        &self,
        project: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<(), String> {
        let Some(tracker) = self.default_tracker_for(project) else {
            let msg = format!("no Gitea tracker configured for project {}", project);
            tracing::error!("{}", msg);
            return Err(msg);
        };
        match tracker.post_comment(issue_number, body).await {
            Ok(comment) => {
                tracing::info!(
                    project = project,
                    issue = issue_number,
                    comment_id = comment.id,
                    "posted raw comment to Gitea"
                );
                Ok(())
            }
            Err(e) => {
                let msg = format!(
                    "failed to post comment to issue {} in project {}: {}",
                    issue_number, project, e
                );
                tracing::error!("{}", msg);
                Err(msg)
            }
        }
    }

    /// Legacy-compatible raw post using the fallback project.
    pub async fn post_raw(&self, issue_number: u64, body: &str) -> Result<(), String> {
        let project = self
            .fallback_project
            .clone()
            .unwrap_or_else(|| LEGACY_PROJECT_ID.to_string());
        self.post_raw_for_project(&project, issue_number, body)
            .await
    }

    /// Post raw markdown as a specific agent within a project.
    pub async fn post_raw_as_agent_for_project(
        &self,
        project: &str,
        agent_name: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<(), String> {
        let Some(tracker) = self.tracker_for(project, agent_name) else {
            let msg = format!(
                "no Gitea tracker configured for project {} and no fallback available",
                project
            );
            tracing::error!("{}", msg);
            return Err(msg);
        };
        match tracker.post_comment(issue_number, body).await {
            Ok(comment) => {
                tracing::info!(
                    project = project,
                    agent = %agent_name,
                    issue = issue_number,
                    comment_id = comment.id,
                    own_token = self.has_own_token(project, agent_name),
                    "posted raw comment as agent"
                );
                Ok(())
            }
            Err(e) => {
                let msg = format!(
                    "failed to post comment as {} to issue {} in project {}: {}",
                    agent_name, issue_number, project, e
                );
                tracing::error!("{}", msg);
                Err(msg)
            }
        }
    }

    /// Legacy-compatible agent-scoped raw post using the fallback project.
    pub async fn post_raw_as_agent(
        &self,
        agent_name: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<(), String> {
        let project = self
            .fallback_project
            .clone()
            .unwrap_or_else(|| LEGACY_PROJECT_ID.to_string());
        self.post_raw_as_agent_for_project(&project, agent_name, issue_number, body)
            .await
    }

    /// Reference to the underlying default tracker for the fallback project.
    ///
    /// Retained for backwards compatibility with code that reaches into the
    /// poster to drive Gitea directly. In multi-project mode, prefer
    /// [`OutputPoster::default_tracker_for_project`].
    pub fn tracker(&self) -> &GiteaTracker {
        let fallback = self
            .fallback_project
            .as_deref()
            .expect("OutputPoster constructed without a fallback project");
        &self
            .projects
            .get(fallback)
            .expect("fallback project always populated")
            .default_tracker
    }

    /// Reference to the default tracker for a specific project.
    pub fn default_tracker_for_project(&self, project: &str) -> Option<&GiteaTracker> {
        self.default_tracker_for(project)
    }
}

/// Build a `ProjectTrackers` bundle for one Gitea output config, including
/// agent-specific tokens loaded from `config.agent_tokens_path` if set.
fn build_project_trackers(config: &GiteaOutputConfig) -> ProjectTrackers {
    let default_gitea_config = GiteaConfig {
        base_url: config.base_url.clone(),
        token: config.token.clone(),
        owner: config.owner.clone(),
        repo: config.repo.clone(),
        active_states: vec!["open".to_string()],
        terminal_states: vec!["closed".to_string()],
        use_robot_api: false,
        robot_path: PathBuf::from(ROBOT_PATH),
        claim_strategy: ClaimStrategy::PreferRobot,
    };
    let default_tracker =
        GiteaTracker::new(default_gitea_config).expect("Failed to create default GiteaTracker");

    let (agent_trackers, agent_tokens): (HashMap<String, GiteaTracker>, HashMap<String, String>) =
        match &config.agent_tokens_path {
            Some(path) => match std::fs::read_to_string(path) {
                Ok(contents) => match serde_json::from_str::<HashMap<String, String>>(&contents) {
                    Ok(tokens) => {
                        tracing::info!(
                            count = tokens.len(),
                            path = %path.display(),
                            owner = %config.owner,
                            repo = %config.repo,
                            "loaded per-agent Gitea tokens"
                        );
                        let mut trackers = HashMap::with_capacity(tokens.len());
                        let mut raw = HashMap::with_capacity(tokens.len());
                        for (agent_name, token) in tokens {
                            let agent_config = GiteaConfig {
                                base_url: config.base_url.clone(),
                                token: token.clone(),
                                owner: config.owner.clone(),
                                repo: config.repo.clone(),
                                active_states: vec!["open".to_string()],
                                terminal_states: vec!["closed".to_string()],
                                use_robot_api: false,
                                robot_path: PathBuf::from(ROBOT_PATH),
                                claim_strategy: ClaimStrategy::PreferRobot,
                            };
                            match GiteaTracker::new(agent_config) {
                                Ok(tracker) => {
                                    trackers.insert(agent_name.clone(), tracker);
                                    raw.insert(agent_name, token);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        agent = %agent_name,
                                        error = %e,
                                        "failed to create agent tracker, will use project default"
                                    );
                                }
                            }
                        }
                        (trackers, raw)
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "failed to parse agent tokens JSON, all agents will use project default token"
                        );
                        (HashMap::new(), HashMap::new())
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "failed to read agent tokens file, all agents will use project default token"
                    );
                    (HashMap::new(), HashMap::new())
                }
            },
            None => (HashMap::new(), HashMap::new()),
        };

    ProjectTrackers {
        default_tracker,
        agent_trackers,
        agent_tokens,
    }
}

#[cfg(test)]
mod agent_token_tests {
    use super::*;
    use crate::config::GiteaOutputConfig;

    #[test]
    fn agent_token_returns_configured_value_when_tokens_file_loaded() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let tokens_path = dir.path().join("agent_tokens.json");
        let mut f = std::fs::File::create(&tokens_path).unwrap();
        writeln!(f, r#"{{"alpha":"tok-alpha","beta":"tok-beta"}}"#).unwrap();
        drop(f);

        let config = GiteaOutputConfig {
            base_url: "https://example.com".to_string(),
            token: "root-token".to_string(),
            owner: "terraphim".to_string(),
            repo: "acme".to_string(),
            agent_tokens_path: Some(tokens_path),
        };
        let poster = OutputPoster::new(&config);

        assert_eq!(
            poster.agent_token(crate::dispatcher::LEGACY_PROJECT_ID, "alpha"),
            Some("tok-alpha")
        );
        assert_eq!(
            poster.agent_token(crate::dispatcher::LEGACY_PROJECT_ID, "beta"),
            Some("tok-beta")
        );
        assert_eq!(
            poster.agent_token(crate::dispatcher::LEGACY_PROJECT_ID, "gamma"),
            None
        );
        assert_eq!(poster.agent_token("nonexistent", "alpha"), None);
    }

    #[test]
    fn agent_token_returns_none_when_no_tokens_file() {
        let config = GiteaOutputConfig {
            base_url: "https://example.com".to_string(),
            token: "root-token".to_string(),
            owner: "terraphim".to_string(),
            repo: "acme".to_string(),
            agent_tokens_path: None,
        };
        let poster = OutputPoster::new(&config);
        assert!(
            poster
                .agent_token(crate::dispatcher::LEGACY_PROJECT_ID, "anything")
                .is_none()
        );
    }
}
