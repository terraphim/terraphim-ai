//! Pre-check capability for `AgentOrchestrator`: evaluating an agent's
//! pre-check strategy (git-diff, shell, gitea-issue) before spawning. Split
//! from lib.rs as part of the Gitea #1910 god-file decomposition; behaviour
//! unchanged.
#![allow(clippy::too_many_lines)]

use std::time::Duration;

use tracing::{info, warn};

use crate::config::{AgentDefinition, PreCheckStrategy};
use crate::{has_matching_changes, AgentOrchestrator, PreCheckResult};

impl AgentOrchestrator {
    /// Evaluate the pre-check strategy for an agent.
    pub(crate) async fn run_pre_check(&mut self, def: &AgentDefinition) -> PreCheckResult {
        match &def.pre_check {
            None | Some(PreCheckStrategy::Always) => PreCheckResult::Findings(String::new()),
            Some(PreCheckStrategy::GitDiff { watch_paths }) => {
                self.git_diff_pre_check(&def.name, watch_paths).await
            }
            Some(PreCheckStrategy::GiteaIssue { issue_number }) => {
                self.gitea_issue_pre_check(*issue_number).await
            }
            Some(PreCheckStrategy::Shell {
                script,
                timeout_secs,
            }) => self.shell_pre_check(script, *timeout_secs).await,
        }
    }

    /// Git diff pre-check: compare last_run_commit to HEAD.
    async fn git_diff_pre_check(&self, agent_name: &str, watch_paths: &[String]) -> PreCheckResult {
        let last_commit = match self.last_run_commits.get(agent_name) {
            Some(c) => c.clone(),
            None => {
                info!(agent = %agent_name, "no last_run_commit recorded, spawning (first run)");
                return PreCheckResult::Findings(String::new());
            }
        };

        // Get current HEAD
        let head = match self.get_current_head().await {
            Ok(h) => h,
            Err(e) => {
                warn!(agent = %agent_name, error = %e, "failed to get HEAD, spawning (fail-open)");
                return PreCheckResult::Failed(format!("git rev-parse failed: {}", e));
            }
        };

        if head == last_commit {
            info!(agent = %agent_name, commit = %head, "HEAD unchanged since last run, skipping");
            return PreCheckResult::NoFindings;
        }

        // Get changed files
        let diff_range = format!("{}..{}", last_commit, head);
        let output = match tokio::time::timeout(
            Duration::from_secs(30),
            tokio::process::Command::new("git")
                .args(["diff", "--name-only", &diff_range])
                .current_dir(&self.config.working_dir)
                .output(),
        )
        .await
        {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                warn!(agent = %agent_name, error = %e, "git diff failed, spawning (fail-open)");
                return PreCheckResult::Failed(format!("git diff failed: {}", e));
            }
            Err(_) => {
                warn!(agent = %agent_name, "git diff timed out after 30s, spawning (fail-open)");
                return PreCheckResult::Failed("git diff timed out after 30s".into());
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(agent = %agent_name, stderr = %stderr, "git diff non-zero exit, spawning (fail-open)");
            return PreCheckResult::Failed(format!("git diff exit {}: {}", output.status, stderr));
        }

        let changed_files: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        if changed_files.is_empty() {
            info!(agent = %agent_name, "no files changed, skipping");
            return PreCheckResult::NoFindings;
        }

        if has_matching_changes(&changed_files, watch_paths) {
            let summary = format!("{} files changed matching watch_paths", changed_files.len());
            info!(agent = %agent_name, files = changed_files.len(), "matching changes found");
            PreCheckResult::Findings(summary)
        } else {
            info!(agent = %agent_name, files = changed_files.len(), "changes found but none match watch_paths, skipping");
            PreCheckResult::NoFindings
        }
    }

    /// Shell pre-check: run script via sh -c.
    async fn shell_pre_check(&self, script: &str, timeout_secs: u64) -> PreCheckResult {
        let result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(script)
                .current_dir(&self.config.working_dir)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if stdout.is_empty() {
                        PreCheckResult::NoFindings
                    } else {
                        PreCheckResult::Findings(stdout)
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    PreCheckResult::Failed(format!("script exit {}: {}", output.status, stderr))
                }
            }
            Ok(Err(e)) => PreCheckResult::Failed(format!("script I/O error: {}", e)),
            Err(_) => PreCheckResult::Failed(format!("script timed out after {}s", timeout_secs)),
        }
    }

    /// Get or lazily construct the GiteaTracker for pre-check.
    pub(crate) fn get_or_init_pre_check_tracker(
        &mut self,
    ) -> Option<&terraphim_tracker::GiteaTracker> {
        if self.pre_check_tracker.is_some() {
            return self.pre_check_tracker.as_ref();
        }
        let workflow = self.config.workflow.as_ref()?;
        let tc = &workflow.tracker;
        let config = terraphim_tracker::GiteaConfig {
            base_url: tc.endpoint.clone(),
            token: tc.api_key.clone(),
            owner: tc.owner.clone(),
            repo: tc.repo.clone(),
            active_states: tc.states.active.clone(),
            terminal_states: tc.states.terminal.clone(),
            use_robot_api: tc.use_robot_api,
            robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
        };
        match terraphim_tracker::GiteaTracker::new(config) {
            Ok(tracker) => {
                self.pre_check_tracker = Some(tracker);
                self.pre_check_tracker.as_ref()
            }
            Err(e) => {
                warn!(error = %e, "failed to construct GiteaTracker for pre-check");
                None
            }
        }
    }

    /// Evaluate the gitea-issue pre-check strategy.
    async fn gitea_issue_pre_check(&mut self, issue_number: u64) -> PreCheckResult {
        let tracker = match self.get_or_init_pre_check_tracker() {
            Some(t) => t,
            None => {
                return PreCheckResult::Failed(
                    "no workflow config for gitea-issue pre-check".into(),
                );
            }
        };

        // Fetch comments with 15s timeout
        let comments = match tokio::time::timeout(
            Duration::from_secs(15),
            tracker.fetch_comments(issue_number, None),
        )
        .await
        {
            Ok(Ok(comments)) => comments,
            Ok(Err(e)) => {
                warn!(
                    issue = issue_number,
                    error = %e,
                    "gitea comment fetch failed, spawning (fail-open)"
                );
                return PreCheckResult::Failed(format!("comment fetch failed: {}", e));
            }
            Err(_) => {
                warn!(
                    issue = issue_number,
                    "gitea comment fetch timed out, spawning (fail-open)"
                );
                return PreCheckResult::Failed("comment fetch timed out after 15s".into());
            }
        };

        if comments.is_empty() {
            info!(issue = issue_number, "no comments on issue, spawning");
            return PreCheckResult::Findings(String::new());
        }

        // Check the most recent comment for PASS verdict
        let latest = comments.last().expect("checked non-empty above");
        let body_lower = latest.body.to_lowercase();

        if body_lower.contains("verdict: pass") {
            // Check if there are new commits since this comment
            let comment_time = &latest.created_at;

            // Use git log to check for commits after the comment time
            let output = match tokio::time::timeout(
                Duration::from_secs(30),
                tokio::process::Command::new("git")
                    .args(["log", "--oneline", &format!("--since={}", comment_time)])
                    .current_dir(&self.config.working_dir)
                    .output(),
            )
            .await
            {
                Ok(Ok(o)) => o,
                Ok(Err(e)) => {
                    warn!(error = %e, "git log failed, spawning (fail-open)");
                    return PreCheckResult::Failed(format!("git log failed: {}", e));
                }
                Err(_) => {
                    warn!("git log --since timed out after 30s, spawning (fail-open)");
                    return PreCheckResult::Failed("git log timed out after 30s".into());
                }
            };

            let log_output = String::from_utf8_lossy(&output.stdout);
            if log_output.trim().is_empty() {
                info!(
                    issue = issue_number,
                    "PASS verdict and no new commits, skipping"
                );
                return PreCheckResult::NoFindings;
            } else {
                let commit_count = log_output.lines().count();
                info!(
                    issue = issue_number,
                    new_commits = commit_count,
                    "PASS verdict but new commits found, spawning"
                );
                return PreCheckResult::Findings(format!(
                    "{} new commits since last PASS verdict",
                    commit_count
                ));
            }
        }

        info!(
            issue = issue_number,
            "no PASS verdict in latest comment, spawning"
        );
        PreCheckResult::Findings(String::new())
    }
}
