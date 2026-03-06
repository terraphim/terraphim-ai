use std::time::Instant;

use crate::config::CompoundReviewConfig;
use crate::error::OrchestratorError;

/// Result of a compound review cycle.
#[derive(Debug, Clone)]
pub struct CompoundReviewResult {
    /// What was found during review.
    pub findings: Vec<String>,
    /// Highest-priority improvement identified.
    pub top_improvement: Option<String>,
    /// Whether a PR was created.
    pub pr_created: bool,
    /// PR URL if created.
    pub pr_url: Option<String>,
    /// Duration of the review.
    pub duration: std::time::Duration,
}

/// Nightly compound review workflow.
///
/// Scans git log, identifies improvement opportunities,
/// and optionally creates PRs with fixes.
#[derive(Debug)]
pub struct CompoundReviewWorkflow {
    config: CompoundReviewConfig,
}

impl CompoundReviewWorkflow {
    pub fn new(config: CompoundReviewConfig) -> Self {
        Self { config }
    }

    /// Run a full compound review cycle.
    ///
    /// 1. Scan git log for last 24h of changes
    /// 2. Identify top improvement opportunity
    /// 3. Optionally create PR with results
    pub async fn run(&self) -> Result<CompoundReviewResult, OrchestratorError> {
        let start = Instant::now();

        let findings = self.scan_git_log().await?;

        let top_improvement = findings.first().cloned();

        let (pr_created, pr_url) = if self.config.create_prs && top_improvement.is_some() {
            // In Phase 1, PR creation is placeholder -- will wire to agent in Step 6
            (false, None)
        } else {
            (false, None)
        };

        Ok(CompoundReviewResult {
            findings,
            top_improvement,
            pr_created,
            pr_url,
            duration: start.elapsed(),
        })
    }

    /// Scan git log for recent changes and extract improvement findings.
    async fn scan_git_log(&self) -> Result<Vec<String>, OrchestratorError> {
        let repo_path = &self.config.repo_path;

        let output = tokio::process::Command::new("git")
            .args(["log", "--oneline", "--since=24 hours ago"])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| {
                OrchestratorError::CompoundReviewFailed(format!(
                    "git log failed in {:?}: {}",
                    repo_path, e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OrchestratorError::CompoundReviewFailed(format!(
                "git log returned non-zero: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let findings: Vec<String> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect();

        Ok(findings)
    }

    /// Check if the compound review is in dry-run mode.
    pub fn is_dry_run(&self) -> bool {
        !self.config.create_prs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_compound_review_dry_run() {
        // Use the current repo as the test repo
        let config = CompoundReviewConfig {
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
        };

        let workflow = CompoundReviewWorkflow::new(config);
        assert!(workflow.is_dry_run());

        let result = workflow.run().await.unwrap();
        assert!(!result.pr_created);
        assert!(result.pr_url.is_none());
        // The current repo should have some recent commits
        // (but we don't assert exact count since it depends on CI timing)
    }

    #[tokio::test]
    async fn test_compound_review_nonexistent_repo() {
        let config = CompoundReviewConfig {
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from("/nonexistent/path"),
            create_prs: false,
        };

        let workflow = CompoundReviewWorkflow::new(config);
        let result = workflow.run().await;
        assert!(result.is_err());
    }
}
