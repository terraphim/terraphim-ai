//! Cross-project meta-coordinator for issue-driven agent dispatch.
//!
//! Polls all configured projects for ready issues (via PageRank API),
//! computes a global priority score across projects, selects the
//! appropriate agent based on issue labels/title/body, claims the issue,
//! and dispatches into the unified [`Dispatcher`] queue.
//!
//! Replaces the standalone bash meta-coordinator with a type-safe,
//! async-native implementation integrated into the orchestrator's
//! reconcile loop.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use terraphim_tracker::{Issue, PagerankClient};

use crate::config::Project;
use crate::dispatcher::{DispatchTask, Dispatcher};
use crate::error::OrchestratorError;

/// Configuration for agent selection rules.
#[derive(Debug, Clone)]
pub struct AgentSelectionRule {
    /// Lowercase keywords to match against title, body, and labels.
    pub keywords: Vec<String>,
    /// Agent name to dispatch when matched.
    pub agent: String,
    /// Project id this rule applies to (None = all projects).
    pub project: Option<String>,
}

/// Per-project dispatch coordinates and state.
pub struct ProjectDispatchState {
    /// Project id.
    pub id: String,
    /// Gitea owner.
    pub owner: String,
    /// Gitea repo.
    pub repo: String,
    /// Base URL for Gitea API.
    pub base_url: String,
    /// API token.
    pub token: String,
    /// Working directory for agents dispatched to this project.
    pub working_dir: std::path::PathBuf,
    /// PageRank client (lazy-initialised on first poll).
    pub pagerank: Option<PagerankClient>,
}

impl std::fmt::Debug for ProjectDispatchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectDispatchState")
            .field("id", &self.id)
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("base_url", &self.base_url)
            .field("working_dir", &self.working_dir)
            .field("has_pagerank", &self.pagerank.is_some())
            .finish()
    }
}

/// Record of a dispatched issue to prevent duplicate dispatches.
#[derive(Debug, Clone)]
struct DispatchRecord {
    dispatched_at: Instant,
    #[allow(dead_code)]
    agent: String,
}

/// Cross-project meta-coordinator.
///
/// Owns the global priority queue logic, agent selection rules, and
/// deduplication state. Designed to be called once per reconcile tick.
pub struct MetaCoordinator {
    /// Per-project dispatch state.
    projects: Vec<ProjectDispatchState>,
    /// Agent selection rules, evaluated in order.
    rules: Vec<AgentSelectionRule>,
    /// Default agent per project when no rule matches.
    default_agents: HashMap<String, String>,
    /// Dispatched issue tracking: (project_id, issue_id) -> record.
    dispatched: Arc<Mutex<HashMap<(String, String), DispatchRecord>>>,
    /// TTL for dispatched records (default 4 hours).
    dispatch_ttl: Duration,
    /// Last cleanup timestamp.
    last_cleanup: Instant,
}

/// A candidate issue from a specific project, ready for scoring.
#[derive(Debug, Clone)]
pub struct CandidateIssue {
    /// Project id this issue belongs to.
    pub project_id: String,
    /// The issue itself.
    pub issue: Issue,
    /// PageRank score from dependency graph (higher = more impact).
    pub pagerank: f64,
    /// Priority level (lower = more urgent).
    pub priority: i32,
}

/// Result of a single dispatch attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchResult {
    /// Successfully dispatched.
    Dispatched {
        /// Name of the agent that was assigned the issue.
        agent: String,
        /// Gitea issue id that was dispatched.
        issue_id: String,
    },
    /// No ready issues found.
    NoIssues,
    /// Issue was already dispatched recently.
    AlreadyDispatched,
    /// Failed to claim the issue.
    ClaimFailed {
        /// Human-readable description of why the claim failed.
        reason: String,
    },
    /// No matching agent for the issue.
    NoMatchingAgent,
}

impl MetaCoordinator {
    /// Create a new meta-coordinator from project configurations.
    ///
    /// Extracts Gitea coordinates from each project's `gitea` config block.
    /// Projects without Gitea config are ignored for issue dispatch.
    pub fn from_projects(
        projects: &[Project],
        gitea_url: &str,
        gitea_token: &str,
    ) -> Result<Self, OrchestratorError> {
        let mut dispatch_states = Vec::new();
        let mut default_agents = HashMap::new();

        for project in projects {
            let Some(gitea) = &project.gitea else {
                debug!(project = %project.id, "skipping project without Gitea config");
                continue;
            };

            dispatch_states.push(ProjectDispatchState {
                id: project.id.clone(),
                owner: gitea.owner.clone(),
                repo: gitea.repo.clone(),
                base_url: gitea_url.to_string(),
                token: gitea_token.to_string(),
                working_dir: project.working_dir.clone(),
                pagerank: Some(PagerankClient::new(gitea_url, gitea_token)),
            });

            // Default agent per project
            default_agents.insert(project.id.clone(), format!("{}-developer", project.id));
        }

        if dispatch_states.is_empty() {
            return Err(OrchestratorError::Config(
                "meta-coordinator requires at least one project with Gitea config".to_string(),
            ));
        }

        let rules = build_default_rules();

        Ok(Self {
            projects: dispatch_states,
            rules,
            default_agents,
            dispatched: Arc::new(Mutex::new(HashMap::new())),
            dispatch_ttl: Duration::from_secs(14400), // 4 hours
            last_cleanup: Instant::now(),
        })
    }

    /// Poll all projects for ready issues and return aggregated candidates.
    pub async fn poll_all_projects(&self) -> Vec<CandidateIssue> {
        let mut all_candidates = Vec::new();

        for project in &self.projects {
            let Some(ref pagerank) = project.pagerank else {
                continue;
            };

            match pagerank.fetch_ready(&project.owner, &project.repo).await {
                Ok(ready) => {
                    debug!(
                        project = %project.id,
                        owner = %project.owner,
                        repo = %project.repo,
                        ready_count = ready.ready_issues.len(),
                        "fetched ready issues"
                    );

                    for score in ready.ready_issues {
                        if score.is_blocked || score.blocker_count > 0 {
                            continue;
                        }

                        let issue = Issue {
                            id: score.id.to_string(),
                            identifier: format!(
                                "{}/{}#{}",
                                project.owner, project.repo, score.index
                            ),
                            title: score.title.clone(),
                            description: None,
                            priority: Some(score.priority),
                            state: "open".to_string(),
                            branch_name: None,
                            url: Some(format!(
                                "{}/{}/{}/issues/{}",
                                project.base_url, project.owner, project.repo, score.index
                            )),
                            labels: vec![],
                            blocked_by: vec![],
                            pagerank_score: Some(score.page_rank),
                            created_at: None,
                            updated_at: None,
                        };

                        all_candidates.push(CandidateIssue {
                            project_id: project.id.clone(),
                            issue,
                            pagerank: score.page_rank,
                            priority: score.priority,
                        });
                    }
                }
                Err(e) => {
                    warn!(
                        project = %project.id,
                        error = %e,
                        "failed to fetch ready issues"
                    );
                }
            }
        }

        all_candidates
    }

    /// Compute global priority score (lower = higher priority).
    ///
    /// Formula: negative PageRank * 100 + priority * 10 + age_hours * 0.5
    /// Matches the bash script's scoring logic.
    pub fn compute_score(candidate: &CandidateIssue) -> f64 {
        let pagerank_term = -candidate.pagerank * 100.0;
        let priority_term = candidate.priority as f64 * 10.0;
        let age_term = candidate
            .issue
            .created_at
            .as_ref()
            .map(|created| {
                let now = jiff::Zoned::now();
                let age = now.duration_since(created);
                age.as_hours() as f64 * 0.5
            })
            .unwrap_or(0.0);

        pagerank_term + priority_term + age_term
    }

    /// Select the best agent for an issue based on rules.
    pub fn select_agent(&self, candidate: &CandidateIssue) -> Option<String> {
        let title_lower = candidate.issue.title.to_lowercase();
        let labels_lower: Vec<String> = candidate
            .issue
            .labels
            .iter()
            .map(|l| l.to_lowercase())
            .collect();
        let text = format!("{} {}", title_lower, labels_lower.join(" "));

        for rule in &self.rules {
            // Skip if rule is project-specific and doesn't match
            if let Some(ref rule_project) = rule.project {
                if rule_project != &candidate.project_id {
                    continue;
                }
            }

            for keyword in &rule.keywords {
                if text.contains(keyword) {
                    return Some(rule.agent.clone());
                }
            }
        }

        // Fall back to default agent for project
        self.default_agents.get(&candidate.project_id).cloned()
    }

    /// Check if an issue was already dispatched within the TTL.
    async fn is_dispatched(&self, project_id: &str, issue_id: &str) -> bool {
        let key = (project_id.to_string(), issue_id.to_string());
        let dispatched = self.dispatched.lock().await;
        dispatched
            .get(&key)
            .is_some_and(|record| record.dispatched_at.elapsed() < self.dispatch_ttl)
    }

    /// Mark an issue as dispatched.
    async fn mark_dispatched(&self, project_id: &str, issue_id: &str, agent: &str) {
        let key = (project_id.to_string(), issue_id.to_string());
        let mut dispatched = self.dispatched.lock().await;
        dispatched.insert(
            key,
            DispatchRecord {
                dispatched_at: Instant::now(),
                agent: agent.to_string(),
            },
        );
    }

    /// Clean up expired dispatch records.
    async fn cleanup_expired(&self) {
        let mut dispatched = self.dispatched.lock().await;
        let before = dispatched.len();
        dispatched.retain(|_, record| record.dispatched_at.elapsed() < self.dispatch_ttl);
        let after = dispatched.len();
        if before != after {
            debug!(cleaned = before - after, "removed expired dispatch records");
        }
    }

    /// Run one dispatch cycle: poll, score, select, claim, dispatch.
    ///
    /// Returns the result of the top-priority dispatch attempt.
    pub async fn dispatch_cycle(&self, dispatcher: &mut Dispatcher) -> DispatchResult {
        // Periodic cleanup of expired records
        if self.last_cleanup.elapsed() > Duration::from_secs(3600) {
            self.cleanup_expired().await;
        }

        let mut candidates = self.poll_all_projects().await;
        if candidates.is_empty() {
            return DispatchResult::NoIssues;
        }

        // Sort by score (lower = better)
        candidates.sort_by(|a, b| {
            let a_score = Self::compute_score(a);
            let b_score = Self::compute_score(b);
            a_score
                .partial_cmp(&b_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Try to dispatch the highest-priority issue
        for candidate in candidates {
            let issue_id = candidate.issue.id.clone();

            // Skip if already dispatched
            if self.is_dispatched(&candidate.project_id, &issue_id).await {
                debug!(
                    project = %candidate.project_id,
                    issue = %issue_id,
                    "issue already dispatched recently, skipping"
                );
                continue;
            }

            // Select agent
            let Some(agent) = self.select_agent(&candidate) else {
                warn!(
                    project = %candidate.project_id,
                    issue = %issue_id,
                    title = %candidate.issue.title,
                    "no matching agent for issue"
                );
                continue;
            };

            // Build dispatch task
            let task = DispatchTask::IssueDriven {
                identifier: candidate.issue.identifier.clone(),
                title: candidate.issue.title.clone(),
                priority: candidate.issue.priority,
                pagerank_score: Some(candidate.pagerank),
                project: candidate.project_id.clone(),
            };

            dispatcher.enqueue(task);
            self.mark_dispatched(&candidate.project_id, &issue_id, &agent)
                .await;

            info!(
                project = %candidate.project_id,
                issue = %issue_id,
                agent = %agent,
                score = %Self::compute_score(&candidate),
                title = %candidate.issue.title,
                "dispatched issue"
            );

            return DispatchResult::Dispatched { agent, issue_id };
        }

        DispatchResult::AlreadyDispatched
    }

    /// Get dispatch statistics.
    pub async fn stats(&self) -> MetaCoordinatorStats {
        let dispatched = self.dispatched.lock().await;
        MetaCoordinatorStats {
            tracked_issues: dispatched.len(),
            projects_configured: self.projects.len(),
        }
    }
}

/// Statistics for the meta-coordinator.
#[derive(Debug, Clone, Default)]
pub struct MetaCoordinatorStats {
    /// Number of issues currently tracked (dispatched within TTL).
    pub tracked_issues: usize,
    /// Number of projects configured.
    pub projects_configured: usize,
}

/// Build default agent selection rules matching the bash script logic.
fn build_default_rules() -> Vec<AgentSelectionRule> {
    vec![
        // Security issues
        AgentSelectionRule {
            keywords: vec![
                "security".to_string(),
                "vulnerability".to_string(),
                "cve".to_string(),
                "unsafe".to_string(),
                "secret".to_string(),
                "credential".to_string(),
                "auth".to_string(),
            ],
            agent: "security-sentinel".to_string(),
            project: Some("terraphim-ai".to_string()),
        },
        AgentSelectionRule {
            keywords: vec![
                "security".to_string(),
                "vulnerability".to_string(),
                "cve".to_string(),
                "unsafe".to_string(),
                "secret".to_string(),
                "credential".to_string(),
                "auth".to_string(),
            ],
            agent: "reviewer".to_string(),
            project: Some("digital-twins".to_string()),
        },
        AgentSelectionRule {
            keywords: vec![
                "security".to_string(),
                "vulnerability".to_string(),
                "cve".to_string(),
                "unsafe".to_string(),
                "secret".to_string(),
                "credential".to_string(),
                "auth".to_string(),
            ],
            agent: "odilo-reviewer".to_string(),
            project: Some("odilo".to_string()),
        },
        // Test-related issues
        AgentSelectionRule {
            keywords: vec![
                "test".to_string(),
                "coverage".to_string(),
                "benchmark".to_string(),
                "performance".to_string(),
                "flake".to_string(),
            ],
            agent: "test-guardian".to_string(),
            project: Some("terraphim-ai".to_string()),
        },
        AgentSelectionRule {
            keywords: vec![
                "test".to_string(),
                "coverage".to_string(),
                "benchmark".to_string(),
                "performance".to_string(),
                "flake".to_string(),
            ],
            agent: "reviewer".to_string(),
            project: Some("digital-twins".to_string()),
        },
        AgentSelectionRule {
            keywords: vec![
                "test".to_string(),
                "coverage".to_string(),
                "benchmark".to_string(),
                "performance".to_string(),
                "flake".to_string(),
            ],
            agent: "odilo-reviewer".to_string(),
            project: Some("odilo".to_string()),
        },
        // Documentation issues
        AgentSelectionRule {
            keywords: vec![
                "doc".to_string(),
                "readme".to_string(),
                "changelog".to_string(),
                "comment".to_string(),
                "guide".to_string(),
            ],
            agent: "documentation-generator".to_string(),
            project: Some("terraphim-ai".to_string()),
        },
        AgentSelectionRule {
            keywords: vec![
                "doc".to_string(),
                "readme".to_string(),
                "changelog".to_string(),
                "comment".to_string(),
                "guide".to_string(),
            ],
            agent: "developer".to_string(),
            project: Some("digital-twins".to_string()),
        },
        AgentSelectionRule {
            keywords: vec![
                "doc".to_string(),
                "readme".to_string(),
                "changelog".to_string(),
                "comment".to_string(),
                "guide".to_string(),
            ],
            agent: "odilo-developer".to_string(),
            project: Some("odilo".to_string()),
        },
        // Spec/validation issues
        AgentSelectionRule {
            keywords: vec![
                "spec".to_string(),
                "validation".to_string(),
                "compliance".to_string(),
                "audit".to_string(),
                "adr".to_string(),
            ],
            agent: "spec-validator".to_string(),
            project: Some("terraphim-ai".to_string()),
        },
        AgentSelectionRule {
            keywords: vec![
                "spec".to_string(),
                "validation".to_string(),
                "compliance".to_string(),
                "audit".to_string(),
                "adr".to_string(),
            ],
            agent: "reviewer".to_string(),
            project: Some("digital-twins".to_string()),
        },
        AgentSelectionRule {
            keywords: vec![
                "spec".to_string(),
                "validation".to_string(),
                "compliance".to_string(),
                "audit".to_string(),
                "adr".to_string(),
            ],
            agent: "odilo-reviewer".to_string(),
            project: Some("odilo".to_string()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidate(title: &str, pagerank: f64, priority: i32) -> CandidateIssue {
        CandidateIssue {
            project_id: "terraphim-ai".to_string(),
            issue: Issue {
                id: "42".to_string(),
                identifier: "terraphim/terraphim-ai#42".to_string(),
                title: title.to_string(),
                description: None,
                priority: Some(priority),
                state: "open".to_string(),
                branch_name: None,
                url: None,
                labels: vec![],
                blocked_by: vec![],
                pagerank_score: Some(pagerank),
                created_at: None,
                updated_at: None,
            },
            pagerank,
            priority,
        }
    }

    #[test]
    fn test_compute_score_pagerank_dominates() {
        let low_pr = make_candidate("Low PR", 0.1, 1);
        let high_pr = make_candidate("High PR", 2.5, 1);

        // High PR should have lower (better) score
        assert!(MetaCoordinator::compute_score(&high_pr) < MetaCoordinator::compute_score(&low_pr));
    }

    #[test]
    fn test_compute_score_priority_penalty() {
        let urgent = make_candidate("Urgent", 1.0, 1);
        let low = make_candidate("Low", 1.0, 4);

        // Urgent should have lower (better) score
        assert!(MetaCoordinator::compute_score(&urgent) < MetaCoordinator::compute_score(&low));
    }

    #[test]
    fn test_select_agent_security() {
        let mc = MetaCoordinator {
            projects: vec![],
            rules: build_default_rules(),
            default_agents: HashMap::new(),
            dispatched: Arc::new(Mutex::new(HashMap::new())),
            dispatch_ttl: Duration::from_secs(14400),
            last_cleanup: Instant::now(),
        };

        let security = CandidateIssue {
            project_id: "terraphim-ai".to_string(),
            issue: Issue {
                id: "1".to_string(),
                identifier: "t/t#1".to_string(),
                title: "Fix security vulnerability in auth".to_string(),
                description: None,
                priority: Some(1),
                state: "open".to_string(),
                branch_name: None,
                url: None,
                labels: vec!["security".to_string()],
                blocked_by: vec![],
                pagerank_score: None,
                created_at: None,
                updated_at: None,
            },
            pagerank: 1.0,
            priority: 1,
        };

        assert_eq!(
            mc.select_agent(&security),
            Some("security-sentinel".to_string())
        );
    }

    #[test]
    fn test_select_agent_default() {
        let mut defaults = HashMap::new();
        defaults.insert(
            "terraphim-ai".to_string(),
            "implementation-swarm".to_string(),
        );

        let mc = MetaCoordinator {
            projects: vec![],
            rules: build_default_rules(),
            default_agents: defaults,
            dispatched: Arc::new(Mutex::new(HashMap::new())),
            dispatch_ttl: Duration::from_secs(14400),
            last_cleanup: Instant::now(),
        };

        let generic = make_candidate("Random feature", 1.0, 2);
        assert_eq!(
            mc.select_agent(&generic),
            Some("implementation-swarm".to_string())
        );
    }

    #[test]
    fn test_select_agent_no_match_no_default() {
        let mc = MetaCoordinator {
            projects: vec![],
            rules: build_default_rules(),
            default_agents: HashMap::new(),
            dispatched: Arc::new(Mutex::new(HashMap::new())),
            dispatch_ttl: Duration::from_secs(14400),
            last_cleanup: Instant::now(),
        };

        let generic = make_candidate("Random feature", 1.0, 2);
        assert_eq!(mc.select_agent(&generic), None);
    }

    #[tokio::test]
    async fn test_dispatch_dedup() {
        let mut defaults = HashMap::new();
        defaults.insert(
            "terraphim-ai".to_string(),
            "implementation-swarm".to_string(),
        );

        let mc = MetaCoordinator {
            projects: vec![],
            rules: build_default_rules(),
            default_agents: defaults,
            dispatched: Arc::new(Mutex::new(HashMap::new())),
            dispatch_ttl: Duration::from_secs(14400),
            last_cleanup: Instant::now(),
        };

        assert!(!mc.is_dispatched("terraphim-ai", "42").await);

        mc.mark_dispatched("terraphim-ai", "42", "implementation-swarm")
            .await;

        assert!(mc.is_dispatched("terraphim-ai", "42").await);
        assert!(!mc.is_dispatched("terraphim-ai", "43").await);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let mut defaults = HashMap::new();
        defaults.insert(
            "terraphim-ai".to_string(),
            "implementation-swarm".to_string(),
        );

        let mc = MetaCoordinator {
            projects: vec![],
            rules: build_default_rules(),
            default_agents: defaults,
            dispatched: Arc::new(Mutex::new(HashMap::new())),
            dispatch_ttl: Duration::from_secs(1),
            last_cleanup: Instant::now(),
        };

        mc.mark_dispatched("terraphim-ai", "42", "agent").await;
        assert!(mc.is_dispatched("terraphim-ai", "42").await);

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;
        mc.cleanup_expired().await;

        assert!(!mc.is_dispatched("terraphim-ai", "42").await);
    }
}
