//! Posts agent output to Gitea issues after agent exit.

use std::collections::HashMap;

use terraphim_tracker::gitea::GiteaConfig;
use terraphim_tracker::GiteaTracker;

use crate::config::GiteaOutputConfig;

/// Posts collected agent output to a Gitea issue comment.
///
/// Supports per-agent Gitea tokens so each agent posts under its own user.
/// Falls back to the default (root) token when no agent-specific token exists.
pub struct OutputPoster {
    /// Default tracker using the root token from config.
    default_tracker: GiteaTracker,
    /// Per-agent trackers keyed by agent name.
    agent_trackers: HashMap<String, GiteaTracker>,
    /// Base config retained for diagnostics.
    #[allow(dead_code)]
    base_url: String,
}

impl OutputPoster {
    /// Create a new OutputPoster from Gitea output configuration.
    ///
    /// If `agent_tokens_path` is set, loads the JSON file as a
    /// `HashMap<String, String>` mapping agent names to Gitea API tokens.
    pub fn new(config: &GiteaOutputConfig) -> Self {
        let default_gitea_config = GiteaConfig {
            base_url: config.base_url.clone(),
            token: config.token.clone(),
            owner: config.owner.clone(),
            repo: config.repo.clone(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
            robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
        };
        let default_tracker =
            GiteaTracker::new(default_gitea_config).expect("Failed to create default GiteaTracker");

        // Load per-agent tokens if path is configured
        let agent_trackers = match &config.agent_tokens_path {
            Some(path) => match std::fs::read_to_string(path) {
                Ok(contents) => match serde_json::from_str::<HashMap<String, String>>(&contents) {
                    Ok(tokens) => {
                        tracing::info!(
                            count = tokens.len(),
                            path = %path.display(),
                            "loaded per-agent Gitea tokens"
                        );
                        let mut trackers = HashMap::with_capacity(tokens.len());
                        for (agent_name, token) in tokens {
                            let agent_config = GiteaConfig {
                                base_url: config.base_url.clone(),
                                token,
                                owner: config.owner.clone(),
                                repo: config.repo.clone(),
                                active_states: vec!["open".to_string()],
                                terminal_states: vec!["closed".to_string()],
                                use_robot_api: false,
                                robot_path: std::path::PathBuf::from(
                                    "/home/alex/go/bin/gitea-robot",
                                ),
                                claim_strategy:
                                    terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
                            };
                            match GiteaTracker::new(agent_config) {
                                Ok(tracker) => {
                                    trackers.insert(agent_name, tracker);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        agent = %agent_name,
                                        error = %e,
                                        "failed to create agent tracker, will use default"
                                    );
                                }
                            }
                        }
                        trackers
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "failed to parse agent tokens JSON, all agents will use default token"
                        );
                        HashMap::new()
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "failed to read agent tokens file, all agents will use default token"
                    );
                    HashMap::new()
                }
            },
            None => HashMap::new(),
        };

        Self {
            default_tracker,
            agent_trackers,
            base_url: config.base_url.clone(),
        }
    }

    /// Get the tracker for a specific agent, falling back to the default.
    pub fn tracker_for(&self, agent_name: &str) -> &GiteaTracker {
        self.agent_trackers
            .get(agent_name)
            .unwrap_or(&self.default_tracker)
    }

    /// Post agent output as a comment on the given Gitea issue.
    ///
    /// Uses the agent's own Gitea token if configured, otherwise falls back
    /// to the default token. Truncates output to 60000 bytes to stay within
    /// Gitea's comment size limit.
    pub async fn post_agent_output(
        &self,
        agent_name: &str,
        issue_number: u64,
        output_lines: &[String],
        exit_code: Option<i32>,
    ) -> Result<(), String> {
        if output_lines.is_empty() {
            tracing::debug!(agent = %agent_name, issue = issue_number, "no output to post");
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
        // Truncate to stay within Gitea limits (~65535 bytes)
        let max_output = 60000;
        if joined.len() > max_output {
            body.push_str(&joined[..max_output]);
            body.push_str("\n... (truncated)\n");
        } else {
            body.push_str(&joined);
        }
        body.push_str("\n```\n\n</details>");

        let tracker = self.tracker_for(agent_name);
        match tracker.post_comment(issue_number, &body).await {
            Ok(comment) => {
                let has_own_token = self.agent_trackers.contains_key(agent_name);
                tracing::info!(
                    agent = %agent_name,
                    issue = issue_number,
                    comment_id = comment.id,
                    own_token = has_own_token,
                    "posted agent output to Gitea"
                );
                Ok(())
            }
            Err(e) => {
                let msg = format!("failed to post output for {}: {}", agent_name, e);
                tracing::error!("{}", msg);
                Err(msg)
            }
        }
    }

    /// Post raw markdown as a comment on the given Gitea issue.
    ///
    /// Uses the default (root) token. For agent-specific posting, use
    /// `post_agent_output` instead.
    pub async fn post_raw(&self, issue_number: u64, body: &str) -> Result<(), String> {
        match self.default_tracker.post_comment(issue_number, body).await {
            Ok(comment) => {
                tracing::info!(
                    issue = issue_number,
                    comment_id = comment.id,
                    "posted raw comment to Gitea"
                );
                Ok(())
            }
            Err(e) => {
                let msg = format!("failed to post comment to issue {}: {}", issue_number, e);
                tracing::error!("{}", msg);
                Err(msg)
            }
        }
    }

    /// Post raw markdown as a specific agent (using agent's own token if available).
    pub async fn post_raw_as_agent(
        &self,
        agent_name: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<(), String> {
        let tracker = self.tracker_for(agent_name);
        match tracker.post_comment(issue_number, body).await {
            Ok(comment) => {
                let has_own_token = self.agent_trackers.contains_key(agent_name);
                tracing::info!(
                    agent = %agent_name,
                    issue = issue_number,
                    comment_id = comment.id,
                    own_token = has_own_token,
                    "posted raw comment as agent"
                );
                Ok(())
            }
            Err(e) => {
                let msg = format!(
                    "failed to post comment as {} to issue {}: {}",
                    agent_name, issue_number, e
                );
                tracing::error!("{}", msg);
                Err(msg)
            }
        }
    }

    /// Get a reference to the underlying default GiteaTracker.
    pub fn tracker(&self) -> &terraphim_tracker::GiteaTracker {
        &self.default_tracker
    }
}
