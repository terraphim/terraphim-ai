//! PageRank client for Gitea Robot API.

use serde::Deserialize;

/// A PageRank score for an issue.
#[derive(Debug, Clone, Deserialize)]
pub struct PagerankScore {
    /// Issue ID.
    pub id: i64,
    /// Issue number/index.
    pub index: i64,
    /// Issue title.
    pub title: String,
    /// PageRank score (higher = more downstream impact).
    pub page_rank: f64,
    /// Priority level.
    pub priority: i32,
    /// Whether the issue is blocked.
    pub is_blocked: bool,
    /// Number of blockers.
    pub blocker_count: i32,
}

/// Response from Gitea Robot ready endpoint.
#[derive(Debug, Deserialize)]
pub struct ReadyResponse {
    /// Repository ID.
    pub repo_id: i64,
    /// Repository name.
    pub repo_name: String,
    /// Total issue count.
    pub total_count: i32,
    /// Ready issues with PageRank scores.
    pub ready_issues: Vec<PagerankScore>,
}

/// Client for Gitea Robot PageRank API.
pub struct PagerankClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl PagerankClient {
    /// Create a new PageRank client.
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            token: token.into(),
        }
    }

    /// Fetch ready issues with PageRank scores.
    pub async fn fetch_ready(&self, owner: &str, repo: &str) -> crate::Result<ReadyResponse> {
        let url = format!(
            "{}/api/v1/robot/ready?owner={}&repo={}",
            self.base_url, owner, repo
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(crate::TrackerError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(crate::TrackerError::Api {
                message: format!("Robot API error {}: {}", status, text),
            });
        }

        let ready: ReadyResponse = response.json().await.map_err(crate::TrackerError::Http)?;

        tracing::info!(
            total = ready.total_count,
            ready = ready.ready_issues.len(),
            "fetched PageRank scores from Robot API"
        );

        Ok(ready)
    }

    /// Merge PageRank scores into issues.
    pub fn merge_scores(issues: &mut [crate::Issue], scores: &[PagerankScore]) {
        for issue in issues.iter_mut() {
            if let Ok(id) = issue.id.parse::<i64>() {
                if let Some(score) = scores.iter().find(|s| s.id == id) {
                    issue.pagerank_score = Some(score.page_rank);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Issue;

    #[test]
    fn merge_scores_updates_issues() {
        let scores = vec![
            PagerankScore {
                id: 42,
                index: 1,
                title: "Issue 1".into(),
                page_rank: 2.5,
                priority: 1,
                is_blocked: false,
                blocker_count: 0,
            },
            PagerankScore {
                id: 99,
                index: 2,
                title: "Issue 2".into(),
                page_rank: 1.0,
                priority: 2,
                is_blocked: false,
                blocker_count: 0,
            },
        ];

        let mut issues = vec![
            Issue {
                id: "42".into(),
                identifier: "TEST-1".into(),
                title: "Issue 1".into(),
                description: None,
                priority: None,
                state: "open".into(),
                branch_name: None,
                url: None,
                labels: vec![],
                blocked_by: vec![],
                pagerank_score: None,
                created_at: None,
                updated_at: None,
            },
            Issue {
                id: "99".into(),
                identifier: "TEST-2".into(),
                title: "Issue 2".into(),
                description: None,
                priority: None,
                state: "open".into(),
                branch_name: None,
                url: None,
                labels: vec![],
                blocked_by: vec![],
                pagerank_score: None,
                created_at: None,
                updated_at: None,
            },
            Issue {
                id: "55".into(),
                identifier: "TEST-3".into(),
                title: "Issue 3".into(),
                description: None,
                priority: None,
                state: "open".into(),
                branch_name: None,
                url: None,
                labels: vec![],
                blocked_by: vec![],
                pagerank_score: None,
                created_at: None,
                updated_at: None,
            },
        ];

        PagerankClient::merge_scores(&mut issues, &scores);

        assert!((issues[0].pagerank_score.unwrap() - 2.5).abs() < 0.001);
        assert!((issues[1].pagerank_score.unwrap() - 1.0).abs() < 0.001);
        assert!(issues[2].pagerank_score.is_none());
    }
}
