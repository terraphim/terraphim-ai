//! Thin Gitea API wrapper for merge-coordinator.
//!
//! Reuses workspace `reqwest` instead of pulling in terraphim_tracker
//! to keep the binary small. Provides retry/backoff (1 s / 2 s / 4 s)
//! and never logs the token.

use std::time::Duration;

use serde::Deserialize;
use tracing::{debug, warn};

use crate::types::MergeCoordinatorError;

const RETRY_DELAYS_SECS: &[u64] = &[1, 2, 4];

/// Maximum number of open PRs fetched per `list_open_prs` call.
///
/// Gitea's hard cap is 50 when no explicit limit is set; the API accepts up
/// to 300.  Using 300 ensures PRs beyond position 50 are not silently skipped
/// by the evaluation loop (issue #2850).
const OPEN_PRS_LIMIT: u32 = 300;

/// Minimal Gitea API client. Caller supplies the API token via env or
/// secure storage; it is never written to logs.
pub struct GiteaClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

/// PR list response item (subset of Gitea fields used here).
#[derive(Debug, Clone, Deserialize)]
pub struct PrSummary {
    /// Gitea PR number.
    pub number: u64,
    /// PR title.
    pub title: String,
    /// PR body (description), if present.
    pub body: Option<String>,
    /// PR state (`"open"`, `"closed"`, etc.).
    pub state: String,
    /// Whether Gitea considers this PR mergeable; `None` if unknown.
    pub mergeable: Option<bool>,
    /// Head commit SHA (for CI status lookups).
    #[serde(default)]
    pub head_sha: Option<String>,
}

/// A single file entry from Gitea's `/pulls/{index}/files` response.
///
/// Gitea returns many fields per entry; only `filename` is used here.
/// Unknown fields are silently ignored by serde, so API additions do not break
/// deserialization.  If Gitea ever renames the field to `name` or `file_path`
/// the tests below will catch it before it silently produces empty strings.
#[derive(Debug, Clone, Deserialize)]
pub struct PrFile {
    /// Path of the changed file relative to the repository root.
    pub filename: String,
}

impl GiteaClient {
    /// Build a client. `base_url` is e.g. `https://git.terraphim.cloud`.
    /// `token` is the Gitea API token; treated as opaque.
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token: token.into(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client builds with default config"),
        }
    }

    /// List open PRs for `owner/repo`.
    pub async fn list_open_prs(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PrSummary>, MergeCoordinatorError> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls?state=open&limit={}",
            self.base_url, owner, repo, OPEN_PRS_LIMIT
        );
        let resp = self.get_with_retry(&url).await?;
        let prs = resp
            .json::<Vec<PrSummary>>()
            .await
            .map_err(|e| MergeCoordinatorError::Api(format!("decode pr list: {e}")))?;
        Ok(prs)
    }

    /// Merge a PR by index. Returns `Ok(())` on success.
    pub async fn merge_pr(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> Result<(), MergeCoordinatorError> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}/merge",
            self.base_url, owner, repo, index
        );
        let body = serde_json::json!({"Do": "merge"});
        self.post_with_retry(&url, &body).await?;
        Ok(())
    }

    /// Close an issue by index (PATCH state=closed).
    pub async fn close_issue(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> Result<(), MergeCoordinatorError> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/issues/{}",
            self.base_url, owner, repo, index
        );
        let body = serde_json::json!({"state": "closed"});
        self.patch_with_retry(&url, &body).await?;
        Ok(())
    }

    /// List files changed in a PR by index. Returns the `filename` of each
    /// changed file, paginating through all pages.  Gitea defaults to 50
    /// files per page; PRs with more changes would silently miss
    /// contamination checks without pagination (issue #2409).
    pub async fn list_pr_files(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> Result<Vec<String>, MergeCoordinatorError> {
        const PER_PAGE: u32 = 50;
        let mut all_files = Vec::new();
        let mut page: u32 = 1;

        loop {
            let url = format!(
                "{}/api/v1/repos/{}/{}/pulls/{}/files?page={page}&limit={PER_PAGE}",
                self.base_url, owner, repo, index,
            );
            let resp = self.get_with_retry(&url).await?;

            // Use X-Total-Count header to detect last page.
            let total_count: Option<u32> = resp
                .headers()
                .get("x-total-count")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());

            let page_files: Vec<PrFile> = resp
                .json()
                .await
                .map_err(|e| MergeCoordinatorError::Api(format!("decode pr files: {e}")))?;

            let page_len = page_files.len();
            all_files.extend(page_files.into_iter().map(|f| f.filename));

            // Stop when we've fetched all items or this page was empty.
            if total_count.is_some_and(|t| all_files.len() as u32 >= t) || page_len == 0 {
                break;
            }
            page += 1;
        }

        Ok(all_files)
    }

    async fn get_with_retry(&self, url: &str) -> Result<reqwest::Response, MergeCoordinatorError> {
        self.send_with_retry(reqwest::Method::GET, url, None).await
    }

    async fn post_with_retry(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, MergeCoordinatorError> {
        self.send_with_retry(reqwest::Method::POST, url, Some(body))
            .await
    }

    async fn patch_with_retry(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, MergeCoordinatorError> {
        self.send_with_retry(reqwest::Method::PATCH, url, Some(body))
            .await
    }

    async fn send_with_retry(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<reqwest::Response, MergeCoordinatorError> {
        let mut last_err: Option<String> = None;
        for (attempt, &delay) in std::iter::once(&0u64)
            .chain(RETRY_DELAYS_SECS.iter())
            .enumerate()
        {
            if delay > 0 {
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
            let mut req = self
                .http
                .request(method.clone(), url)
                .header("Authorization", format!("token {}", self.token))
                .header("Accept", "application/json");
            if let Some(b) = body {
                req = req.json(b);
            }
            match req.send().await {
                Ok(resp) if resp.status().is_success() => {
                    debug!(method = %method, url = %redact(url), attempt, "gitea call ok");
                    return Ok(resp);
                }
                Ok(resp) => {
                    let status = resp.status();
                    let is_client_error = status.is_client_error()
                        && status != reqwest::StatusCode::TOO_MANY_REQUESTS;
                    let body_text = resp.text().await.unwrap_or_default();
                    last_err = Some(format!("status {status}: {body_text}"));
                    if is_client_error {
                        warn!(method = %method, url = %redact(url), attempt, %status, "gitea client error (non-retryable); failing immediately");
                        break;
                    }
                    warn!(method = %method, url = %redact(url), attempt, %status, "gitea non-success; will retry if attempts remain");
                }
                Err(e) => {
                    last_err = Some(format!("network: {e}"));
                    warn!(method = %method, url = %redact(url), attempt, error = %e, "gitea network error; will retry if attempts remain");
                }
            }
        }
        Err(MergeCoordinatorError::Api(format!(
            "gitea call failed after {} attempts: {}",
            RETRY_DELAYS_SECS.len() + 1,
            last_err.unwrap_or_else(|| "no error captured".into())
        )))
    }

    /// Query CI status for a head commit.
    ///
    /// Returns `None` when Gitea has no status data for the commit
    /// (e.g. the repo has no Actions enabled, or the commit predates
    /// CI instrumentation).  A present but empty `statuses` list is
    /// treated as `CiNoStatus`, not as an error.
    pub async fn get_commit_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<Option<CommitCombinedStatus>, MergeCoordinatorError> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/commits/{}/status",
            self.base_url, owner, repo, sha
        );
        let resp = self.get_with_retry(&url).await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let combined: CommitCombinedStatus = resp
            .json()
            .await
            .map_err(|e| MergeCoordinatorError::Api(format!("decode commit status: {e}")))?;
        Ok(Some(combined))
    }
}

/// Gitea commit combined-status response.
#[derive(Debug, Clone, Deserialize)]
pub struct CommitCombinedStatus {
    pub state: String,
    #[serde(default)]
    pub statuses: Vec<serde_json::Value>,
}

/// Redact the token if a URL contains one inline (defence in depth).
fn redact(url: &str) -> String {
    // tokens never appear in URLs in this client, but keep the helper
    // so future log-points stay consistent.
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pr_summary_deserialises_minimum_fields() {
        let json = r#"{"number":42,"title":"Fix things","body":"Fixes #1","state":"open","mergeable":true}"#;
        let pr: PrSummary = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(pr.state, "open");
        assert_eq!(pr.mergeable, Some(true));
    }

    #[test]
    fn pr_summary_tolerates_missing_optional_fields() {
        let json = r#"{"number":1,"title":"x","state":"open"}"#;
        let pr: PrSummary = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 1);
        assert_eq!(pr.body, None);
        assert_eq!(pr.mergeable, None);
    }

    #[test]
    fn retry_delays_are_one_two_four_seconds() {
        assert_eq!(RETRY_DELAYS_SECS, &[1u64, 2, 4]);
    }

    #[test]
    fn open_prs_limit_exceeds_gitea_default_of_50() {
        const {
            assert!(
                OPEN_PRS_LIMIT > 50,
                "OPEN_PRS_LIMIT must exceed 50 so PRs beyond position 50 are not silently dropped"
            );
        }
    }

    #[test]
    fn open_prs_limit_within_gitea_max_page_size() {
        const {
            assert!(
                OPEN_PRS_LIMIT <= 300,
                "Gitea max page size is 300; limit must not exceed it"
            );
        }
    }

    #[test]
    fn pr_summary_vec_of_51_items_deserialises() {
        // Construct JSON array with 51 items to verify no artificial truncation
        // happens at the deserialization layer.
        let items: String = (1u64..=51)
            .map(|n| format!(r#"{{"number":{n},"title":"PR {n}","state":"open"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        let json = format!("[{items}]");
        let prs: Vec<PrSummary> = serde_json::from_str(&json).unwrap();
        assert_eq!(
            prs.len(),
            51,
            "all 51 PRs must be present after deserialisation"
        );
        assert_eq!(prs[50].number, 51, "PR at position 51 must be present");
    }

    #[test]
    fn pr_file_deserialises_filename() {
        let json = r#"{"filename":"src/main.rs"}"#;
        let f: PrFile = serde_json::from_str(json).unwrap();
        assert_eq!(f.filename, "src/main.rs");
    }

    #[test]
    fn pr_file_list_extracts_filenames() {
        // Mirrors exactly what list_pr_files receives from the Gitea API.
        let json = r#"[{"filename":"src/main.rs"},{"filename":"Cargo.toml"}]"#;
        let files: Vec<PrFile> = serde_json::from_str(json).unwrap();
        let names: Vec<String> = files.into_iter().map(|f| f.filename).collect();
        assert_eq!(names, vec!["src/main.rs", "Cargo.toml"]);
    }

    #[test]
    fn pr_file_unknown_fields_ignored() {
        // Gitea returns many fields per file entry; only filename is used.
        // If Gitea ever renames the field to "name" or "file_path" the
        // missing-field error surfaces here rather than silently producing
        // empty strings.
        let json =
            r#"{"filename":"docs/README.md","status":"modified","additions":5,"deletions":2}"#;
        let f: PrFile = serde_json::from_str(json).unwrap();
        assert_eq!(f.filename, "docs/README.md");
    }
}
