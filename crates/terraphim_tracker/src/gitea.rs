//! Gitea REST issue tracker client.

use crate::{Issue, IssueTracker, Result, TrackerError};
use async_trait::async_trait;
use jiff::Zoned;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Result of a claim operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimResult {
    /// Successfully claimed the issue.
    Success,
    /// Issue already assigned to this agent (idempotent success).
    AlreadyAssigned,
    /// Issue assigned to another agent (claim failed).
    AssignedToOther { assignee: String },
    /// Issue not found.
    NotFound,
    /// Permission denied (agent cannot assign to themselves).
    PermissionDenied { reason: String },
    /// Transient failure, may retry.
    TransientFailure { reason: String },
}

/// Strategy for claiming issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStrategy {
    /// Prefer gitea-robot CLI, fallback to REST API.
    #[default]
    PreferRobot,
    /// Use REST API only.
    ApiOnly,
    /// Use gitea-robot CLI only (fail if unavailable).
    RobotOnly,
}

/// Configuration for Gitea tracker.
#[derive(Debug, Clone)]
pub struct GiteaConfig {
    pub base_url: String,
    pub token: String,
    pub owner: String,
    pub repo: String,
    pub active_states: Vec<String>,
    pub terminal_states: Vec<String>,
    pub use_robot_api: bool,
    /// Path to gitea-robot binary.
    pub robot_path: PathBuf,
    /// Claim strategy.
    pub claim_strategy: ClaimStrategy,
}

impl Default for GiteaConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            token: String::new(),
            owner: String::new(),
            repo: String::new(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
            robot_path: PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: ClaimStrategy::default(),
        }
    }
}

impl GiteaConfig {
    /// Create a new config with default robot path and claim strategy.
    pub fn new(base_url: String, token: String, owner: String, repo: String) -> Self {
        Self {
            base_url,
            token,
            owner,
            repo,
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
            robot_path: PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: ClaimStrategy::default(),
        }
    }
}

/// Gitea REST API client.
pub struct GiteaTracker {
    client: Client,
    pub(crate) config: GiteaConfig,
}

/// Gitea API issue response.
#[derive(Debug, Deserialize)]
pub struct GiteaIssue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub labels: Option<Vec<GiteaLabel>>,
}

/// Gitea label.
#[derive(Debug, Deserialize)]
pub struct GiteaLabel {
    pub name: String,
}

/// Gitea issue comment from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: u64,
    /// Issue number (extracted from issue_url for repo-wide API, filled by caller for per-issue API).
    #[serde(default)]
    pub issue_number: u64,
    pub body: String,
    pub user: CommentUser,
    pub created_at: String,
    pub updated_at: String,
}

/// User who authored a comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentUser {
    pub login: String,
}

/// Backward-compatible alias.
pub type GiteaComment = IssueComment;
impl GiteaTracker {
    /// Create a new Gitea tracker from configuration.
    pub fn new(config: GiteaConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| TrackerError::Api {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self { client, config })
    }

    /// Build request with authentication.
    pub(crate) fn build_request(
        &self,
        method: reqwest::Method,
        url: &str,
    ) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .header("Authorization", format!("token {}", self.config.token))
            .header("Accept", "application/json")
    }

    /// Convert Gitea issue to normalised Issue.
    fn normalise_issue(&self, gi: GiteaIssue) -> Issue {
        let identifier = format!("{}/{}/{}", self.config.owner, self.config.repo, gi.number);
        let labels: Vec<String> = gi
            .labels
            .unwrap_or_default()
            .into_iter()
            .map(|l| l.name.to_lowercase())
            .collect();
        let state = gi.state.to_lowercase();

        Issue {
            id: gi.id.to_string(),
            identifier,
            title: gi.title,
            description: gi.body,
            priority: None,
            state,
            branch_name: None,
            url: gi.html_url,
            labels,
            blocked_by: Vec::new(),
            pagerank_score: None,
            created_at: gi.created_at.and_then(|s| parse_datetime(&s)),
            updated_at: gi.updated_at.and_then(|s| parse_datetime(&s)),
        }
    }

    /// Fetch a single issue by number.
    pub async fn fetch_issue(&self, issue_number: u64) -> Result<GiteaIssue> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );
        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!(
                    "Gitea fetch_issue error {} on issue {}: {}",
                    status, issue_number, text
                ),
            });
        }
        response.json().await.map_err(TrackerError::Http)
    }

    /// Fetch all issues in the repository for a given Gitea API state.
    async fn fetch_issues_for_gitea_state(&self, gitea_state: &str) -> Result<Vec<Issue>> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues",
            self.config.base_url, self.config.owner, self.config.repo
        );
        let mut all_issues = Vec::new();
        let mut page = 1u32;

        loop {
            let response = self
                .build_request(reqwest::Method::GET, &url)
                .query(&[("state", gitea_state), ("type", "issues"), ("limit", "50")])
                .query(&[("page", page)])
                .send()
                .await?;
            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(TrackerError::Api {
                    message: format!(
                        "Gitea fetch issues error {} for state {}: {}",
                        status, gitea_state, text
                    ),
                });
            }
            let issues: Vec<GiteaIssue> = response.json().await.map_err(TrackerError::Http)?;
            let issue_count = issues.len();
            all_issues.extend(issues.into_iter().map(|gi| self.normalise_issue(gi)));

            if issue_count < 50 {
                break;
            }
            page += 1;
        }

        Ok(all_issues)
    }

    /// Fetch all open issues in the repository.
    pub async fn fetch_open_issues(&self) -> Result<Vec<Issue>> {
        self.fetch_issues_for_gitea_state("open").await
    }

    /// Post a comment on a Gitea issue.
    pub async fn post_comment(&self, issue_number: u64, body: &str) -> Result<IssueComment> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}/comments",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );
        let response = self
            .build_request(reqwest::Method::POST, &url)
            .json(&serde_json::json!({"body": body}))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!(
                    "Gitea post_comment error {} on issue {}: {}",
                    status, issue_number, text
                ),
            });
        }
        response.json().await.map_err(TrackerError::Http)
    }

    /// Create a new issue with optional labels.
    pub async fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<GiteaIssue> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues",
            self.config.base_url, self.config.owner, self.config.repo
        );
        let response = self
            .build_request(reqwest::Method::POST, &url)
            .json(&serde_json::json!({
                "title": title,
                "body": body,
                "labels": labels,
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea create_issue error {}: {}", status, text),
            });
        }
        response.json().await.map_err(TrackerError::Http)
    }

    /// Assign a Gitea issue to one or more users.
    ///
    /// Uses the authenticated user's token, so when called with a per-agent
    /// tracker the issue is assigned to that agent's Gitea user.
    pub async fn assign_issue(&self, issue_number: u64, assignees: &[&str]) -> Result<()> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );
        let response = self
            .build_request(reqwest::Method::PATCH, &url)
            .json(&serde_json::json!({"assignees": assignees}))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!(
                    "Gitea assign_issue error {} on issue {}: {}",
                    status, issue_number, text
                ),
            });
        }
        Ok(())
    }

    /// Fetch the list of assignee logins for a Gitea issue.
    ///
    /// Returns an empty vec if the issue has no assignees or on error.
    pub async fn fetch_issue_assignees(&self, issue_number: u64) -> Result<Vec<String>> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );
        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!(
                    "Gitea fetch_issue_assignees error {} on issue {}: {}",
                    status, issue_number, text
                ),
            });
        }
        // Parse just the assignees array from the issue JSON
        let body: serde_json::Value = response.json().await.map_err(TrackerError::Http)?;
        let assignees = body
            .get("assignees")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|u| u.get("login").and_then(|l| l.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(assignees)
    }

    /// Search open issues by keyword in title.
    /// Returns issue numbers whose titles contain the given keyword.
    pub async fn search_issues_by_title(&self, keyword: &str) -> Result<Vec<u64>> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues?state=open&q={}&type=issues",
            self.config.base_url,
            self.config.owner,
            self.config.repo,
            urlencoding::encode(keyword)
        );
        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea search issues error {}: {}", status, text),
            });
        }
        let issues: Vec<GiteaIssue> = response.json().await.map_err(TrackerError::Http)?;
        Ok(issues.into_iter().map(|i| i.number).collect())
    }

    /// Fetch comments on a Gitea issue, optionally filtering by `since` timestamp.
    pub async fn fetch_comments(
        &self,
        issue_number: u64,
        since: Option<&str>,
    ) -> Result<Vec<IssueComment>> {
        let mut url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}/comments",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );
        if let Some(since_ts) = since {
            url.push_str(&format!("?since={}", since_ts));
        }
        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!(
                    "Gitea comments GET error {} on issue {}: {}",
                    status, issue_number, text
                ),
            });
        }
        response.json().await.map_err(TrackerError::Http)
    }

    /// Fetch comments across all issues in the repo, optionally filtering by `since` timestamp.
    ///
    /// This is the repo-wide endpoint that returns comments from all issues,
    /// which is more efficient than polling each issue individually.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v1/repos/{owner}/{repo}/issues/comments?since={since}&limit={limit}`
    ///
    /// # Response
    ///
    /// Each comment includes an `issue_url` field from which the issue number
    /// can be extracted.
    pub async fn fetch_repo_comments(
        &self,
        since: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<IssueComment>> {
        self.fetch_repo_comments_page(since, limit, None).await
    }

    /// Fetch comments across all issues in the repo with optional paging.
    pub async fn fetch_repo_comments_page(
        &self,
        since: Option<&str>,
        limit: Option<u32>,
        page: Option<u32>,
    ) -> Result<Vec<IssueComment>> {
        let mut url = format!(
            "{}/api/v1/repos/{}/{}/issues/comments",
            self.config.base_url, self.config.owner, self.config.repo
        );
        let mut params = Vec::new();
        if let Some(since_ts) = since {
            params.push(format!("since={}", since_ts));
        }
        if let Some(limit_val) = limit {
            params.push(format!("limit={}", limit_val));
        }
        if let Some(page_val) = page {
            params.push(format!("page={}", page_val));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TrackerError::Api {
                message: format!("Gitea repo comments GET error {}: {}", status, text),
            });
        }

        // Parse response with diagnostic logging on failure
        let text = response.text().await.map_err(TrackerError::Http)?;
        let raw_comments: Vec<RepoComment> = match serde_json::from_str(&text) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    body_preview = &text[..text.len().min(200)],
                    "failed to deserialise repo comments"
                );
                return Err(TrackerError::Api {
                    message: format!("repo comments deserialisation failed: {e}"),
                });
            }
        };
        Ok(raw_comments.into_iter().map(|rc| rc.into()).collect())
    }

    /// Claim an issue for an agent using the configured strategy.
    ///
    /// Attempts to assign the issue to the specified agent, with verification.
    /// Uses gitea-robot CLI if available (and configured), otherwise falls back
    /// to direct REST API call.
    ///
    /// # Arguments
    /// * `agent_name` - The Gitea username of the agent claiming the issue
    /// * `issue_number` - The issue number to claim
    /// * `strategy` - Which claim strategy to use
    ///
    /// # Returns
    /// * `Ok(ClaimResult)` - The outcome of the claim attempt
    /// * `Err(TrackerError)` - Unexpected error (network, auth, etc.)
    pub async fn claim_issue(
        &self,
        agent_name: &str,
        issue_number: u64,
        strategy: ClaimStrategy,
    ) -> Result<ClaimResult> {
        // 1. Pre-check: Fetch current assignees
        let current_assignees = match self.fetch_issue_assignees(issue_number).await {
            Ok(assignees) => assignees,
            Err(e) => {
                // Fail open on assignee check error - will attempt assignment anyway
                tracing::warn!(
                    agent = %agent_name,
                    issue = issue_number,
                    error = %e,
                    "failed to fetch assignees, attempting claim anyway"
                );
                Vec::new()
            }
        };

        // 2. Idempotency check: already assigned to this agent
        if current_assignees.iter().any(|a| a == agent_name) {
            return Ok(ClaimResult::AlreadyAssigned);
        }

        // 3. Conflict check: assigned to another agent
        if !current_assignees.is_empty() {
            return Ok(ClaimResult::AssignedToOther {
                assignee: current_assignees.join(", "),
            });
        }

        // 4. Attempt claim based on strategy
        let result = match strategy {
            ClaimStrategy::PreferRobot => {
                match self.claim_via_robot(agent_name, issue_number).await {
                    Ok(result) => Ok(result),
                    Err(e) if Self::is_robot_unavailable_error(&e) => {
                        tracing::info!(
                            agent = %agent_name,
                            issue = issue_number,
                            "gitea-robot unavailable, falling back to API"
                        );
                        self.claim_via_api(agent_name, issue_number).await
                    }
                    Err(e) => Err(e),
                }
            }
            ClaimStrategy::RobotOnly => self.claim_via_robot(agent_name, issue_number).await,
            ClaimStrategy::ApiOnly => self.claim_via_api(agent_name, issue_number).await,
        };

        let result = result?;

        // 5. Verify assignment succeeded (for Success case only)
        if matches!(result, ClaimResult::Success) {
            match self
                .verify_assignment(agent_name, issue_number, Some(3), Some(500))
                .await
            {
                Ok(true) => {}
                Ok(false) => {
                    tracing::warn!(
                        agent = %agent_name,
                        issue = issue_number,
                        "Assignment verification failed after claim"
                    );
                    return Ok(ClaimResult::AssignedToOther {
                        assignee: "unknown (race condition)".to_string(),
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        agent = %agent_name,
                        issue = issue_number,
                        error = %e,
                        "Failed to verify assignment after claim"
                    );
                    return Ok(ClaimResult::TransientFailure {
                        reason: format!("failed to verify assignment after claim: {e}"),
                    });
                }
            }
        }

        Ok(result)
    }

    /// Internal: Attempt claim via gitea-robot CLI.
    async fn claim_via_robot(&self, agent_name: &str, issue_number: u64) -> Result<ClaimResult> {
        let output = Command::new(&self.config.robot_path)
            .env("GITEA_URL", &self.config.base_url)
            .env("GITEA_TOKEN", &self.config.token)
            .args([
                "assign",
                "--owner",
                &self.config.owner,
                "--repo",
                &self.config.repo,
                "--issue",
                &issue_number.to_string(),
                "--to",
                agent_name,
            ])
            .output()
            .map_err(|e| TrackerError::Api {
                message: format!("Failed to execute gitea-robot: {}", e),
            })?;

        if output.status.success() {
            return Ok(ClaimResult::Success);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{} {}", stderr, stdout);

        // Parse error types from stderr/stdout
        if combined.contains("not found") || combined.contains("404") {
            return Ok(ClaimResult::NotFound);
        }
        if combined.contains("already assigned")
            || combined.contains("conflict")
            || combined.contains("409")
        {
            return Ok(ClaimResult::AssignedToOther {
                assignee: "unknown".to_string(),
            });
        }
        if combined.contains("permission") || combined.contains("403") {
            return Ok(ClaimResult::PermissionDenied {
                reason: stderr.to_string(),
            });
        }

        // Transient errors
        if combined.contains("timeout")
            || combined.contains("connection")
            || combined.contains("temporarily")
        {
            return Ok(ClaimResult::TransientFailure {
                reason: stderr.to_string(),
            });
        }

        Err(TrackerError::Api {
            message: format!("gitea-robot assign failed: {} (stdout: {})", stderr, stdout),
        })
    }

    /// Internal: Attempt claim via REST API.
    async fn claim_via_api(&self, agent_name: &str, issue_number: u64) -> Result<ClaimResult> {
        // First, fetch current state to detect races
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}",
            self.config.base_url, self.config.owner, self.config.repo, issue_number
        );

        let response = self
            .build_request(reqwest::Method::GET, &url)
            .send()
            .await?;

        if response.status() == 404 {
            return Ok(ClaimResult::NotFound);
        }

        if !response.status().is_success() {
            return Err(TrackerError::Api {
                message: format!("Failed to fetch issue state: {}", response.status()),
            });
        }

        // Check assignees before attempting assignment
        let body: serde_json::Value = response.json().await?;
        let assignees: Vec<String> = body
            .get("assignees")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|u| u.get("login").and_then(|l| l.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if assignees.iter().any(|a| a == agent_name) {
            return Ok(ClaimResult::AlreadyAssigned);
        }
        if !assignees.is_empty() {
            return Ok(ClaimResult::AssignedToOther {
                assignee: assignees.join(", "),
            });
        }

        // Attempt assignment
        let patch_response = self
            .build_request(reqwest::Method::PATCH, &url)
            .json(&serde_json::json!({"assignees": [agent_name]}))
            .send()
            .await?;

        match patch_response.status().as_u16() {
            200 => Ok(ClaimResult::Success),
            403 => Ok(ClaimResult::PermissionDenied {
                reason: "Insufficient permissions to assign issue".to_string(),
            }),
            404 => Ok(ClaimResult::NotFound),
            409 => Ok(ClaimResult::AssignedToOther {
                assignee: "unknown (conflict)".to_string(),
            }),
            500..=599 => Ok(ClaimResult::TransientFailure {
                reason: format!("Server error: {}", patch_response.status()),
            }),
            _ => Err(TrackerError::Api {
                message: format!("Assignment failed: {}", patch_response.status()),
            }),
        }
    }

    /// Verify that an issue is actually assigned to the expected agent.
    ///
    /// Handles race conditions where assignment may have succeeded but
    /// not yet visible, or was changed by another concurrent process.
    ///
    /// # Arguments
    /// * `agent_name` - The expected assignee
    /// * `issue_number` - The issue to check
    /// * `max_retries` - Number of verification attempts (default 3)
    /// * `retry_delay_ms` - Delay between retries in milliseconds (default 500)
    ///
    /// # Returns
    /// * `Ok(true)` - Verified assignment matches expected
    /// * `Ok(false)` - Assignment does not match (may need re-claim)
    /// * `Err(TrackerError)` - Could not verify (network error, etc.)
    pub async fn verify_assignment(
        &self,
        agent_name: &str,
        issue_number: u64,
        max_retries: Option<u32>,
        retry_delay_ms: Option<u64>,
    ) -> Result<bool> {
        let max_retries = max_retries.unwrap_or(3);
        let retry_delay_ms = retry_delay_ms.unwrap_or(500);

        for attempt in 0..max_retries {
            match self.fetch_issue_assignees(issue_number).await {
                Ok(assignees) => {
                    if assignees.iter().any(|a| a == agent_name) {
                        return Ok(true);
                    }
                    // Not assigned yet - retry if not last attempt
                    if attempt < max_retries - 1 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms))
                            .await;
                    }
                }
                Err(e) => {
                    if attempt < max_retries - 1 {
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_retries = max_retries,
                            error = %e,
                            "verify_assignment failed, retrying"
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms))
                            .await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check if an error indicates gitea-robot is unavailable.
    fn is_robot_unavailable_error(error: &TrackerError) -> bool {
        let err_str = error.to_string().to_lowercase();
        err_str.contains("no such file or directory")
            || err_str.contains("not found")
            || err_str.contains("permission denied")
            || err_str.contains("cannot find")
    }
}

#[async_trait]
impl IssueTracker for GiteaTracker {
    async fn fetch_candidate_issues(&self) -> Result<Vec<Issue>> {
        let active_states = self.config.active_states.clone();
        self.fetch_issues_by_states(&active_states).await
    }

    async fn fetch_issue_states_by_ids(&self, ids: &[String]) -> Result<Vec<Issue>> {
        let mut issues = Vec::with_capacity(ids.len());

        for id in ids {
            let issue_number = match id.parse::<u64>() {
                Ok(issue_number) => issue_number,
                Err(_) => {
                    return Err(TrackerError::Api {
                        message: format!("invalid Gitea issue id '{id}'"),
                    });
                }
            };

            let issue = self.fetch_issue(issue_number).await?;
            issues.push(self.normalise_issue(issue));
        }

        Ok(issues)
    }

    async fn fetch_issues_by_states(&self, states: &[String]) -> Result<Vec<Issue>> {
        if states.is_empty() {
            return Ok(vec![]);
        }

        let need_open = states.iter().any(|state| {
            state.eq_ignore_ascii_case("open")
                || self
                    .config
                    .active_states
                    .iter()
                    .any(|active| active.eq_ignore_ascii_case(state))
        });
        let need_closed = states.iter().any(|state| {
            state.eq_ignore_ascii_case("closed")
                || self
                    .config
                    .terminal_states
                    .iter()
                    .any(|terminal| terminal.eq_ignore_ascii_case(state))
        });

        let mut issues = Vec::new();
        if need_open {
            issues.extend(self.fetch_issues_for_gitea_state("open").await?);
        }
        if need_closed {
            issues.extend(self.fetch_issues_for_gitea_state("closed").await?);
        }

        Ok(issues
            .into_iter()
            .filter(|issue| {
                states
                    .iter()
                    .any(|state| state.eq_ignore_ascii_case(&issue.state))
            })
            .collect())
    }
}

/// Raw comment from repo-wide API (includes issue_url instead of issue number).
///
/// Fields use `Option<String>` because Gitea may return `null` for system
/// comments (missing `issue_url`), deleted comments (null `body`), etc.
#[derive(Debug, Clone, serde::Deserialize)]
struct RepoComment {
    id: u64,
    #[serde(default)]
    issue_url: Option<String>,
    user: CommentUser,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

impl From<RepoComment> for IssueComment {
    fn from(rc: RepoComment) -> Self {
        // Extract issue number from issue_url like "/api/v1/repos/owner/repo/issues/123"
        let issue_number = rc
            .issue_url
            .as_deref()
            .unwrap_or("")
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        IssueComment {
            id: rc.id,
            issue_number,
            body: rc.body.unwrap_or_default(),
            user: rc.user,
            created_at: rc.created_at.unwrap_or_default(),
            updated_at: rc.updated_at.unwrap_or_default(),
        }
    }
}

/// Parse ISO 8601 datetime string.
fn parse_datetime(s: &str) -> Option<Zoned> {
    s.parse::<jiff::Timestamp>()
        .ok()
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_tracker(base_url: &str) -> GiteaTracker {
        let config = GiteaConfig {
            base_url: base_url.to_string(),
            token: "test-token".to_string(),
            owner: "testowner".to_string(),
            repo: "testrepo".to_string(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
            robot_path: PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: ClaimStrategy::PreferRobot,
        };
        GiteaTracker::new(config).unwrap()
    }

    fn test_config() -> GiteaConfig {
        GiteaConfig {
            base_url: "https://git.example.com".into(),
            token: "test-token".into(),
            owner: "testowner".into(),
            repo: "testrepo".into(),
            active_states: vec!["open".into(), "Todo".into()],
            terminal_states: vec!["closed".into(), "Done".into()],
            use_robot_api: false,
            robot_path: PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: ClaimStrategy::PreferRobot,
        }
    }

    #[test]
    fn normalise_issue_converts_fields() {
        let config = test_config();
        let tracker = GiteaTracker::new(config).unwrap();

        let gi = GiteaIssue {
            id: 42,
            number: 123,
            title: "Test Issue".into(),
            body: Some("Description".into()),
            state: "open".into(),
            html_url: Some("https://example.com/issue/123".into()),
            created_at: Some("2024-01-15T10:30:00Z".into()),
            updated_at: Some("2024-01-15T11:00:00Z".into()),
            labels: Some(vec![
                GiteaLabel { name: "bug".into() },
                GiteaLabel {
                    name: "Priority:High".into(),
                },
            ]),
        };

        let issue = tracker.normalise_issue(gi);

        assert_eq!(issue.id, "42");
        assert_eq!(issue.identifier, "testowner/testrepo/123");
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.description, Some("Description".into()));
        assert_eq!(issue.state, "open");
        assert!(issue.labels.contains(&"bug".to_string()));
        assert!(issue.labels.contains(&"priority:high".to_string()));
    }

    #[test]
    fn normalise_issue_lowercases_labels() {
        let config = test_config();
        let tracker = GiteaTracker::new(config).unwrap();

        let gi = GiteaIssue {
            id: 1,
            number: 1,
            title: "Test".into(),
            body: None,
            state: "open".into(),
            html_url: None,
            created_at: None,
            updated_at: None,
            labels: Some(vec![
                GiteaLabel { name: "BUG".into() },
                GiteaLabel {
                    name: "FEATURE".into(),
                },
            ]),
        };

        let issue = tracker.normalise_issue(gi);
        assert!(
            issue
                .labels
                .iter()
                .all(|l| l.chars().all(|c| !c.is_uppercase()))
        );
    }

    #[tokio::test]
    async fn test_fetch_open_issues_paginates() {
        let mock_server = MockServer::start().await;
        let page_one: Vec<_> = (1..=50)
            .map(|number| {
                serde_json::json!({
                    "id": number,
                    "number": number,
                    "title": format!("Issue {number}"),
                    "state": "open"
                })
            })
            .collect();
        let page_two = serde_json::json!([
            {
                "id": 51,
                "number": 51,
                "title": "Issue 51",
                "state": "open"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues"))
            .and(query_param("state", "open"))
            .and(query_param("type", "issues"))
            .and(query_param("limit", "50"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page_one))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues"))
            .and(query_param("state", "open"))
            .and(query_param("type", "issues"))
            .and(query_param("limit", "50"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page_two))
            .expect(1)
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let issues = tracker.fetch_open_issues().await.unwrap();
        assert_eq!(issues.len(), 51);
        assert_eq!(issues.last().unwrap().identifier, "testowner/testrepo/51");
    }

    #[tokio::test]
    async fn test_fetch_issues_by_states_fetches_closed_issues() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues"))
            .and(query_param("state", "closed"))
            .and(query_param("type", "issues"))
            .and(query_param("limit", "50"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 200,
                    "number": 12,
                    "title": "Done issue",
                    "state": "closed"
                }
            ])))
            .expect(1)
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let issues = tracker
            .fetch_issues_by_states(&["closed".to_string()])
            .await
            .unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].state, "closed");
        assert_eq!(issues[0].identifier, "testowner/testrepo/12");
    }

    #[tokio::test]
    async fn test_post_comment_success() {
        let mock_server = MockServer::start().await;
        let comment_json = serde_json::json!({
            "id": 42,
            "body": "Test comment",
            "user": {"login": "testbot"},
            "created_at": "2026-03-31T12:00:00Z",
            "updated_at": "2026-03-31T12:00:00Z"
        });
        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/1/comments"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&comment_json))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.post_comment(1, "Test comment").await;
        assert!(result.is_ok());
        let comment = result.unwrap();
        assert_eq!(comment.id, 42);
        assert_eq!(comment.body, "Test comment");
        assert_eq!(comment.user.login, "testbot");
    }

    #[tokio::test]
    async fn test_post_comment_error_returns_api_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/999/comments"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.post_comment(999, "body").await;
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("403"),
            "Expected 403 in error: {}",
            err_str
        );
    }

    #[tokio::test]
    async fn test_fetch_comments_without_since() {
        let mock_server = MockServer::start().await;
        let comments_json = serde_json::json!([
            {
                "id": 1,
                "body": "First",
                "user": {"login": "alice"},
                "created_at": "2026-03-31T10:00:00Z",
                "updated_at": "2026-03-31T10:00:00Z"
            },
            {
                "id": 2,
                "body": "Second",
                "user": {"login": "bob"},
                "created_at": "2026-03-31T11:00:00Z",
                "updated_at": "2026-03-31T11:00:00Z"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/5/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&comments_json))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.fetch_comments(5, None).await;
        assert!(result.is_ok());
        let comments = result.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].body, "First");
        assert_eq!(comments[1].user.login, "bob");
    }

    #[tokio::test]
    async fn test_fetch_comments_with_since() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/5/comments"))
            .and(query_param("since", "2026-03-31T00:00:00Z"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .fetch_comments(5, Some("2026-03-31T00:00:00Z"))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_comments_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/404/comments"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.fetch_comments(404, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_repo_comments() {
        let mock_server = MockServer::start().await;
        let comments_json = serde_json::json!([
            {
                "id": 100,
                "issue_url": "https://example.com/api/v1/repos/testowner/testrepo/issues/5",
                "body": "Hello @adf:security-sentinel",
                "user": {"login": "root"},
                "created_at": "2026-04-04T10:00:00Z",
                "updated_at": "2026-04-04T10:00:00Z"
            },
            {
                "id": 101,
                "issue_url": "https://example.com/api/v1/repos/testowner/testrepo/issues/7",
                "body": null,
                "user": {"login": "system"},
                "created_at": "2026-04-04T11:00:00Z",
                "updated_at": "2026-04-04T11:00:00Z"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/comments"))
            .and(query_param("since", "2026-04-04T00:00:00Z"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&comments_json))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .fetch_repo_comments(Some("2026-04-04T00:00:00Z"), Some(50))
            .await;
        assert!(
            result.is_ok(),
            "fetch_repo_comments failed: {:?}",
            result.err()
        );
        let comments = result.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].issue_number, 5);
        assert_eq!(comments[1].issue_number, 7);
        assert!(comments[0].body.contains("@adf:security-sentinel"));
        assert_eq!(comments[1].body, "") // null body defaults to empty
    }

    #[tokio::test]
    async fn test_fetch_repo_comments_missing_fields() {
        let mock_server = MockServer::start().await;
        // Simulate comments with missing optional fields
        let comments_json = serde_json::json!([
            {
                "id": 200,
                "user": {"login": "bot"},
                "created_at": "2026-04-04T12:00:00Z",
                "updated_at": "2026-04-04T12:00:00Z"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&comments_json))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.fetch_repo_comments(None, None).await;
        assert!(
            result.is_ok(),
            "should handle missing issue_url and body: {:?}",
            result.err()
        );
        let comments = result.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].issue_number, 0); // no issue_url -> default 0
        assert_eq!(comments[0].body, "") // missing body -> default empty
    }

    #[tokio::test]
    async fn test_issue_comment_deserialisation() {
        let json = r#"{
            "id": 100,
            "body": "Hello @adf:security-sentinel",
            "user": {"login": "root"},
            "created_at": "2026-03-31T14:00:00+02:00",
            "updated_at": "2026-03-31T14:00:00+02:00"
        }"#;
        let comment: IssueComment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.id, 100);
        assert!(comment.body.contains("@adf:security-sentinel"));
        assert_eq!(comment.user.login, "root");
    }

    #[tokio::test]
    async fn test_assign_issue_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test",
                "state": "open"
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.assign_issue(42, &["quality-coordinator"]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_assign_issue_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/99"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.assign_issue(99, &["unknown-agent"]).await;
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("403"),
            "Expected 403 in error: {}",
            err_str
        );
    }

    #[tokio::test]
    async fn test_fetch_issue_assignees_returns_logins() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [
                    {"id": 1, "login": "security-sentinel"},
                    {"id": 2, "login": "test-guardian"}
                ]
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let assignees = tracker.fetch_issue_assignees(42).await.unwrap();
        assert_eq!(assignees, vec!["security-sentinel", "test-guardian"]);
    }

    #[tokio::test]
    async fn test_fetch_issue_assignees_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/99"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 200,
                "number": 99,
                "title": "Unassigned issue",
                "state": "open",
                "assignees": []
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let assignees = tracker.fetch_issue_assignees(99).await.unwrap();
        assert!(assignees.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_issue_assignees_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/404"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker.fetch_issue_assignees(404).await;
        assert!(result.is_err());
    }

    // Claim Abstraction Tests

    #[tokio::test]
    async fn test_claim_issue_already_assigned() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [
                    {"id": 1, "login": "quality-coordinator"}
                ]
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .claim_issue("quality-coordinator", 42, ClaimStrategy::ApiOnly)
            .await;
        assert_eq!(result.unwrap(), ClaimResult::AlreadyAssigned);
    }

    #[tokio::test]
    async fn test_claim_issue_assigned_to_other() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [
                    {"id": 1, "login": "other-agent"}
                ]
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .claim_issue("quality-coordinator", 42, ClaimStrategy::ApiOnly)
            .await;
        assert_eq!(
            result.unwrap(),
            ClaimResult::AssignedToOther {
                assignee: "other-agent".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_claim_issue_success_api() {
        let mock_server = MockServer::start().await;

        // The first two GETs are the pre-check and the API claim path.
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": []
            })))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        // Verification sees the assignment after the PATCH succeeds.
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [{"id": 1, "login": "quality-coordinator"}]
            })))
            .mount(&mock_server)
            .await;

        // Assignment patch
        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [{"id": 1, "login": "quality-coordinator"}]
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .claim_issue("quality-coordinator", 42, ClaimStrategy::ApiOnly)
            .await;
        assert_eq!(result.unwrap(), ClaimResult::Success);
    }

    #[tokio::test]
    async fn test_claim_issue_not_found() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .claim_issue("quality-coordinator", 999, ClaimStrategy::ApiOnly)
            .await;
        assert_eq!(result.unwrap(), ClaimResult::NotFound);
    }

    #[tokio::test]
    async fn test_claim_issue_permission_denied() {
        let mock_server = MockServer::start().await;

        // Initial fetch - no assignees
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": []
            })))
            .mount(&mock_server)
            .await;

        // Assignment forbidden
        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .claim_issue("quality-coordinator", 42, ClaimStrategy::ApiOnly)
            .await;
        assert!(matches!(
            result.unwrap(),
            ClaimResult::PermissionDenied { .. }
        ));
    }

    #[tokio::test]
    async fn test_verify_assignment_with_retry() {
        let mock_server = MockServer::start().await;

        // First two calls return empty, third returns the agent
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [{"id": 1, "login": "quality-coordinator"}]
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let verified = tracker
            .verify_assignment("quality-coordinator", 42, Some(3), Some(100))
            .await;
        assert!(verified.unwrap());
    }

    #[tokio::test]
    async fn test_verify_assignment_fails_after_retries() {
        let mock_server = MockServer::start().await;

        // Always returns different assignee
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [{"id": 1, "login": "other-agent"}]
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let verified = tracker
            .verify_assignment("quality-coordinator", 42, Some(2), Some(100))
            .await;
        assert!(!verified.unwrap());
    }

    #[tokio::test]
    async fn test_claim_strategy_api_only_uses_no_robot() {
        // This test verifies ApiOnly strategy doesn't try robot
        // Since we can't easily mock process::Command, we verify it works
        // when API is available
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": []
            })))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [{"id": 1, "login": "test-agent"}]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [{"id": 1, "login": "test-agent"}]
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        // Use a non-existent robot path to ensure it would fail if tried
        let result = tracker
            .claim_issue("test-agent", 42, ClaimStrategy::ApiOnly)
            .await;
        assert!(matches!(result.unwrap(), ClaimResult::Success));
    }

    #[tokio::test]
    async fn test_claim_issue_returns_assigned_to_other_when_verification_fails() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": []
            })))
            .up_to_n_times(2)
            .expect(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [{"id": 1, "login": "other-agent"}]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100,
                "number": 42,
                "title": "Test issue",
                "state": "open",
                "assignees": [{"id": 1, "login": "quality-coordinator"}]
            })))
            .mount(&mock_server)
            .await;

        let tracker = make_tracker(&mock_server.uri());
        let result = tracker
            .claim_issue("quality-coordinator", 42, ClaimStrategy::ApiOnly)
            .await
            .unwrap();

        assert_eq!(
            result,
            ClaimResult::AssignedToOther {
                assignee: "unknown (race condition)".to_string()
            }
        );
    }
}
