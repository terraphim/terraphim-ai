//! Posts agent output to Gitea issues after agent exit.

use terraphim_tracker::GiteaTracker;
use terraphim_tracker::gitea::GiteaConfig;

use crate::config::GiteaOutputConfig;

/// Posts collected agent output to a Gitea issue comment.
pub struct OutputPoster {
    tracker: GiteaTracker,
}

impl OutputPoster {
    /// Create a new OutputPoster from Gitea output configuration.
    pub fn new(config: &GiteaOutputConfig) -> Self {
        let gitea_config = GiteaConfig {
            base_url: config.base_url.clone(),
            token: config.token.clone(),
            owner: config.owner.clone(),
            repo: config.repo.clone(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
        };
        Self {
            tracker: GiteaTracker::new(gitea_config).expect("Failed to create GiteaTracker"),
        }
    }

    /// Post agent output as a comment on the given Gitea issue.
    ///
    /// Truncates output to 60000 bytes to stay within Gitea's comment size limit.
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

        match self.tracker.post_comment(issue_number, &body).await {
            Ok(comment) => {
                tracing::info!(
                    agent = %agent_name,
                    issue = issue_number,
                    comment_id = comment.id,
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
}


