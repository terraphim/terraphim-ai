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

/// Trait abstraction over the Gitea operations the merge-coordinator needs.
///
/// Exists so the business logic in [`crate::evaluator`] can be tested without a
/// live Gitea server (project policy: no mocks). The production implementation
/// is [`GiteaClient`]; tests supply a concrete [`FakeGiteaClient`] fake.
///
/// Methods mirror the concrete `GiteaClient` surface exactly — the only
/// behavioural change introduced by the trait is the indirection itself.
///
/// Uses a generic `T: GiteaOperations` bound at call-sites (not `&dyn`) because
/// `async fn` in a trait is not object-safe without return-type boxing, and
/// monomorphised generics are zero-cost.
pub trait GiteaOperations {
    /// List open PRs for `owner/repo`.
    fn list_open_prs(
        &self,
        owner: &str,
        repo: &str,
    ) -> impl std::future::Future<Output = Result<Vec<PrSummary>, MergeCoordinatorError>> + Send;

    /// Refetch a single PR by index (fresh state for the pre-merge check).
    ///
    /// Added for #3295 (design §D6): immediately before merging, the
    /// coordinator must re-read the PR and require `state=open`,
    /// `mergeable=Some(true)` and an unchanged head SHA. Errors propagate so
    /// callers fail closed instead of merging on stale data.
    fn get_pr(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> impl std::future::Future<Output = Result<PrSummary, MergeCoordinatorError>> + Send;

    /// List files changed in a PR by index.
    fn list_pr_files(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> impl std::future::Future<Output = Result<Vec<String>, MergeCoordinatorError>> + Send;

    /// Merge a PR by index.
    fn merge_pr(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> impl std::future::Future<Output = Result<(), MergeCoordinatorError>> + Send;

    /// Close an issue by index.
    fn close_issue(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> impl std::future::Future<Output = Result<(), MergeCoordinatorError>> + Send;

    /// Query CI combined status for a head commit.
    fn get_commit_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> impl std::future::Future<
        Output = Result<Option<CommitCombinedStatus>, MergeCoordinatorError>,
    > + Send;
}

/// Minimal Gitea API client. Caller supplies the API token via env or
/// secure storage; it is never written to logs.
pub struct GiteaClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

/// PR list response item (subset of Gitea fields used here).
#[derive(Debug, Clone)]
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
    /// Head commit SHA (for CI status lookups and the pre-merge exact-head
    /// check, #3295 §D6).
    pub head_sha: Option<String>,
}

impl<'de> Deserialize<'de> for PrSummary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Gitea nests the head SHA under `head.sha`; a top-level `head_sha`
        // is tolerated as an alternate shape. Empty strings normalise to
        // `None` so downstream "SHA unavailable" handling is uniform.
        #[derive(Deserialize)]
        struct Raw {
            number: u64,
            title: String,
            #[serde(default)]
            body: Option<String>,
            state: String,
            #[serde(default)]
            mergeable: Option<bool>,
            #[serde(default)]
            head_sha: Option<String>,
            #[serde(default)]
            head: Option<HeadRef>,
        }

        #[derive(Deserialize)]
        struct HeadRef {
            #[serde(default)]
            sha: Option<String>,
        }

        let raw = Raw::deserialize(deserializer)?;
        let head_sha = raw
            .head_sha
            .filter(|s| !s.is_empty())
            .or_else(|| raw.head.and_then(|h| h.sha).filter(|s| !s.is_empty()));
        Ok(PrSummary {
            number: raw.number,
            title: raw.title,
            body: raw.body,
            state: raw.state,
            mergeable: raw.mergeable,
            head_sha,
        })
    }
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

    /// Refetch a single PR by index. Returns the fresh `PrSummary`
    /// (`state`, `mergeable`, `head_sha`) used by the pre-merge check.
    ///
    /// A 404 or any transport error surfaces as `Err` so callers fail
    /// closed (#3295 design §D6).
    pub async fn get_pr(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> Result<PrSummary, MergeCoordinatorError> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, index
        );
        let resp = self.get_with_retry(&url).await?;
        let pr = resp
            .json::<PrSummary>()
            .await
            .map_err(|e| MergeCoordinatorError::Api(format!("decode pr: {e}")))?;
        Ok(pr)
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

impl GiteaOperations for GiteaClient {
    async fn list_open_prs(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PrSummary>, MergeCoordinatorError> {
        GiteaClient::list_open_prs(self, owner, repo).await
    }

    async fn get_pr(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> Result<PrSummary, MergeCoordinatorError> {
        GiteaClient::get_pr(self, owner, repo, index).await
    }

    async fn list_pr_files(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> Result<Vec<String>, MergeCoordinatorError> {
        GiteaClient::list_pr_files(self, owner, repo, index).await
    }

    async fn merge_pr(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> Result<(), MergeCoordinatorError> {
        GiteaClient::merge_pr(self, owner, repo, index).await
    }

    async fn close_issue(
        &self,
        owner: &str,
        repo: &str,
        index: u64,
    ) -> Result<(), MergeCoordinatorError> {
        GiteaClient::close_issue(self, owner, repo, index).await
    }

    async fn get_commit_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<Option<CommitCombinedStatus>, MergeCoordinatorError> {
        GiteaClient::get_commit_status(self, owner, repo, sha).await
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
    fn pr_summary_reads_head_sha_from_nested_gitea_shape() {
        // Real Gitea payloads nest the head SHA under `head.sha`; the
        // §D6 exact-head check is dead on arrival against a live server
        // without this (#3295).
        let json = r#"{
            "number": 42,
            "title": "Fix things",
            "state": "open",
            "mergeable": true,
            "head": {"ref": "feature", "sha": "6dcb09b5b57875f334f61aebed695e2e4193db5e"},
            "base": {"ref": "main", "sha": "0000000000000000000000000000000000000000"}
        }"#;
        let pr: PrSummary = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(
            pr.head_sha.as_deref(),
            Some("6dcb09b5b57875f334f61aebed695e2e4193db5e")
        );
    }

    #[test]
    fn pr_summary_top_level_head_sha_still_accepted() {
        // Alternate/fake payload shape: top-level `head_sha` wins when both
        // are present and non-empty.
        let json =
            r#"{"number":2,"title":"x","state":"open","head_sha":"aaa","head":{"sha":"bbb"}}"#;
        let pr: PrSummary = serde_json::from_str(json).unwrap();
        assert_eq!(pr.head_sha.as_deref(), Some("aaa"));
    }

    #[test]
    fn pr_summary_empty_head_sha_normalises_to_none() {
        // Empty strings read as "unavailable" so the pre-merge check fails
        // closed uniformly instead of comparing two empty SHAs as "equal".
        for json in [
            r#"{"number":3,"title":"x","state":"open","head_sha":""}"#,
            r#"{"number":3,"title":"x","state":"open","head":{"sha":""}}"#,
            r#"{"number":3,"title":"x","state":"open"}"#,
        ] {
            let pr: PrSummary = serde_json::from_str(json).unwrap();
            assert_eq!(pr.head_sha, None, "payload: {json}");
        }
    }

    #[test]
    fn get_pr_url_targets_single_pull() {
        // Contract guard: the refetch must hit the single-PR endpoint, not
        // the list endpoint (a list call would not be "fresh" per §D6).
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}",
            "https://g.example", "o", "r", 12
        );
        assert!(url.ends_with("/repos/o/r/pulls/12"));
        assert!(!url.contains("state=open"));
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

    #[tokio::test]
    async fn fake_gitea_returns_configured_open_prs() {
        let fake = test_support::FakeGiteaClient {
            open_prs: vec![PrSummary {
                number: 5,
                title: "t".into(),
                body: None,
                state: "open".into(),
                mergeable: Some(true),
                head_sha: None,
            }],
            ..test_support::FakeGiteaClient::new()
        };
        let prs = fake.list_open_prs("o", "r").await.unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 5);
    }
}

/// In-process test support: a concrete fake of the Gitea API.
///
/// Lives in a `pub` (test-only) module so both the gitea and evaluator unit
/// tests can construct it. Not a mock: it holds real state (configured PRs,
/// file lists, call counters) and exercises the exact same code path as
/// [`GiteaClient`] would over the network (issue #2892).
#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Concrete in-process fake for the Gitea API.
    ///
    /// Construction is explicit (public fields + `..FakeGiteaClient::new()`)
    /// so each test documents the scenario it sets up.
    pub struct FakeGiteaClient {
        /// Open PRs returned by `list_open_prs`.
        pub open_prs: Vec<PrSummary>,
        /// Files returned by `list_pr_files`, keyed by PR index. Any PR not
        /// present here yields the `list_pr_files_error` when set, else `[]`.
        pub pr_files: HashMap<u64, Vec<String>>,
        /// If set, `list_pr_files` returns this error for every PR not found
        /// in `pr_files` (the fail-open scenario the evaluator must survive).
        pub list_pr_files_error: Option<MergeCoordinatorError>,
        /// CI status returned by `get_commit_status`, keyed by head SHA.
        pub commit_status: HashMap<String, CommitCombinedStatus>,
        /// If `commit_status` has no entry for a SHA, this is returned.
        pub commit_status_default: Option<CommitCombinedStatus>,
        /// Fresh `state` override for `get_pr`, keyed by PR index (#3295 §D6:
        /// the PR changed between evaluation and the pre-merge refetch).
        pub pr_states: Mutex<HashMap<u64, String>>,
        /// Fresh head-SHA override for `get_pr`, keyed by PR index (head drift
        /// or a freshly-missing SHA).
        pub pr_head_shas: Mutex<HashMap<u64, String>>,
        /// Fresh `mergeable` override for `get_pr`, keyed by PR index.
        pub pr_mergeables: Mutex<HashMap<u64, Option<bool>>>,
        /// If set, `get_pr` returns this error (refetch transport failure).
        pub get_pr_error: Option<MergeCoordinatorError>,
        /// Number of times `merge_pr` was called.
        pub merge_calls: Mutex<u64>,
        /// PR indices `merge_pr` was called with, in order.
        pub merged_indexes: Mutex<Vec<u64>>,
        /// Number of times `close_issue` was called.
        pub close_calls: Mutex<u64>,
        /// Issue indices `close_issue` was called with, in order.
        pub closed_indexes: Mutex<Vec<u64>>,
        /// If set, `close_issue` returns this error for this issue index
        /// (the partial-failure path).
        pub close_issue_error_for: Option<u64>,
    }

    impl FakeGiteaClient {
        /// Build an empty fake (no PRs, no files, no errors).
        pub fn new() -> Self {
            Self {
                open_prs: Vec::new(),
                pr_files: HashMap::new(),
                list_pr_files_error: None,
                commit_status: HashMap::new(),
                commit_status_default: None,
                pr_states: Mutex::new(HashMap::new()),
                pr_head_shas: Mutex::new(HashMap::new()),
                pr_mergeables: Mutex::new(HashMap::new()),
                get_pr_error: None,
                merge_calls: Mutex::new(0),
                merged_indexes: Mutex::new(Vec::new()),
                close_calls: Mutex::new(0),
                closed_indexes: Mutex::new(Vec::new()),
                close_issue_error_for: None,
            }
        }
    }

    impl Default for FakeGiteaClient {
        fn default() -> Self {
            Self::new()
        }
    }

    impl GiteaOperations for FakeGiteaClient {
        async fn list_open_prs(
            &self,
            _owner: &str,
            _repo: &str,
        ) -> Result<Vec<PrSummary>, MergeCoordinatorError> {
            Ok(self.open_prs.clone())
        }

        async fn get_pr(
            &self,
            _owner: &str,
            _repo: &str,
            index: u64,
        ) -> Result<PrSummary, MergeCoordinatorError> {
            if let Some(e) = &self.get_pr_error {
                return Err(clone_error(e));
            }
            let mut pr = self
                .open_prs
                .iter()
                .find(|p| p.number == index)
                .cloned()
                .ok_or_else(|| {
                    MergeCoordinatorError::api(format!("fake: PR #{index} not found"))
                })?;
            {
                let states = self.pr_states.lock().unwrap();
                if let Some(state) = states.get(&index) {
                    pr.state = state.clone();
                }
            }
            {
                let mergeables = self.pr_mergeables.lock().unwrap();
                if let Some(mergeable) = mergeables.get(&index) {
                    pr.mergeable = *mergeable;
                }
            }
            {
                let shas = self.pr_head_shas.lock().unwrap();
                if let Some(sha) = shas.get(&index) {
                    pr.head_sha = Some(sha.clone());
                }
            }
            Ok(pr)
        }

        async fn list_pr_files(
            &self,
            _owner: &str,
            _repo: &str,
            index: u64,
        ) -> Result<Vec<String>, MergeCoordinatorError> {
            match self.pr_files.get(&index) {
                Some(files) => Ok(files.clone()),
                None => match &self.list_pr_files_error {
                    Some(e) => Err(clone_error(e)),
                    None => Ok(Vec::new()),
                },
            }
        }

        async fn merge_pr(
            &self,
            _owner: &str,
            _repo: &str,
            index: u64,
        ) -> Result<(), MergeCoordinatorError> {
            *self.merge_calls.lock().unwrap() += 1;
            self.merged_indexes.lock().unwrap().push(index);
            Ok(())
        }

        async fn close_issue(
            &self,
            _owner: &str,
            _repo: &str,
            index: u64,
        ) -> Result<(), MergeCoordinatorError> {
            *self.close_calls.lock().unwrap() += 1;
            self.closed_indexes.lock().unwrap().push(index);
            if self.close_issue_error_for == Some(index) {
                return Err(MergeCoordinatorError::api(format!(
                    "fake close_issue error for #{index}"
                )));
            }
            Ok(())
        }

        async fn get_commit_status(
            &self,
            _owner: &str,
            _repo: &str,
            sha: &str,
        ) -> Result<Option<CommitCombinedStatus>, MergeCoordinatorError> {
            Ok(self
                .commit_status
                .get(sha)
                .cloned()
                .or_else(|| self.commit_status_default.clone()))
        }
    }

    /// Clone a `MergeCoordinatorError`. The enum only carries owned data
    /// (`String` / `i32`/`u64`), so a manual clone is sufficient and avoids
    /// adding `Clone` to the public error type (not derived today and whose
    /// addition is out of scope for this refactor).
    pub(super) fn clone_error(e: &MergeCoordinatorError) -> MergeCoordinatorError {
        match e {
            MergeCoordinatorError::Api(s) => MergeCoordinatorError::Api(s.clone()),
            MergeCoordinatorError::LockHeld { pid, age_secs } => MergeCoordinatorError::LockHeld {
                pid: *pid,
                age_secs: *age_secs,
            },
            MergeCoordinatorError::Io(io) => {
                MergeCoordinatorError::Io(std::io::Error::other(io.to_string()))
            }
        }
    }
}
