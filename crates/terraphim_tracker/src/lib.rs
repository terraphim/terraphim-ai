//! Issue tracker integration for Terraphim
//!
//! Provides a unified interface for interacting with issue trackers:
//! - Gitea (via REST API v1)
//! - Extensible trait for other trackers
//!
//! Features:
//! - List, get, create, update, and close issues
//! - PageRank integration via gitea-robot API
//! - Async trait-based interface

use std::collections::HashMap;

pub mod gitea;

pub use gitea::GiteaTracker;

/// Errors that can occur during tracker operations
#[derive(thiserror::Error, Debug)]
pub enum TrackerError {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Issue not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for tracker operations
pub type Result<T> = std::result::Result<T, TrackerError>;

/// Issue state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    #[default]
    Open,
    Closed,
}

impl std::fmt::Display for IssueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueState::Open => write!(f, "open"),
            IssueState::Closed => write!(f, "closed"),
        }
    }
}

/// Represents an issue tracked by an issue tracker
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrackedIssue {
    /// Issue ID/number
    pub id: u64,
    /// Issue title
    pub title: String,
    /// Issue state
    pub state: IssueState,
    /// Labels attached to the issue
    #[serde(default)]
    pub labels: Vec<String>,
    /// Assignees (usernames)
    #[serde(default)]
    pub assignees: Vec<String>,
    /// Priority level (if available)
    pub priority: Option<String>,
    /// PageRank score from gitea-robot (if available)
    pub page_rank_score: Option<f64>,
    /// Issue body/description
    pub body: Option<String>,
    /// Created timestamp
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Updated timestamp
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Closed timestamp
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// URL to the issue
    pub url: Option<String>,
    /// Additional metadata
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl TrackedIssue {
    /// Create a new tracked issue
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            state: IssueState::Open,
            labels: Vec::new(),
            assignees: Vec::new(),
            priority: None,
            page_rank_score: None,
            body: None,
            created_at: None,
            updated_at: None,
            closed_at: None,
            url: None,
            extra: HashMap::new(),
        }
    }

    /// Set the issue state
    pub fn with_state(mut self, state: IssueState) -> Self {
        self.state = state;
        self
    }

    /// Add a label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Set assignees
    pub fn with_assignees(mut self, assignees: Vec<String>) -> Self {
        self.assignees = assignees;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: impl Into<String>) -> Self {
        self.priority = Some(priority.into());
        self
    }

    /// Set PageRank score
    pub fn with_page_rank_score(mut self, score: f64) -> Self {
        self.page_rank_score = Some(score);
        self
    }

    /// Set body
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set URL
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Check if issue is open
    pub fn is_open(&self) -> bool {
        self.state == IssueState::Open
    }

    /// Check if issue is closed
    pub fn is_closed(&self) -> bool {
        self.state == IssueState::Closed
    }

    /// Check if issue has a specific label
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l.eq_ignore_ascii_case(label))
    }

    /// Check if issue is assigned to a specific user
    pub fn is_assigned_to(&self, username: &str) -> bool {
        self.assignees.iter().any(|a| a.eq_ignore_ascii_case(username))
    }
}

/// Configuration for issue tracker connection
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrackerConfig {
    /// Tracker API URL
    pub url: String,
    /// Authentication token
    pub token: String,
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Optional: gitea-robot URL for PageRank
    #[serde(default)]
    pub robot_url: Option<String>,
}

impl TrackerConfig {
    /// Create a new tracker configuration
    pub fn new(
        url: impl Into<String>,
        token: impl Into<String>,
        owner: impl Into<String>,
        repo: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            token: token.into(),
            owner: owner.into(),
            repo: repo.into(),
            robot_url: None,
        }
    }

    /// Set the gitea-robot URL for PageRank
    pub fn with_robot_url(mut self, url: impl Into<String>) -> Self {
        self.robot_url = Some(url.into());
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.url.is_empty() {
            return Err(TrackerError::InvalidConfiguration(
                "URL cannot be empty".to_string(),
            ));
        }
        if self.token.is_empty() {
            return Err(TrackerError::InvalidConfiguration(
                "Token cannot be empty".to_string(),
            ));
        }
        if self.owner.is_empty() {
            return Err(TrackerError::InvalidConfiguration(
                "Owner cannot be empty".to_string(),
            ));
        }
        if self.repo.is_empty() {
            return Err(TrackerError::InvalidConfiguration(
                "Repo cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Parameters for listing issues
#[derive(Debug, Clone, Default)]
pub struct ListIssuesParams {
    /// Filter by state
    pub state: Option<IssueState>,
    /// Filter by labels
    pub labels: Option<Vec<String>>,
    /// Filter by assignee
    pub assignee: Option<String>,
    /// Maximum number of issues to return
    pub limit: Option<u32>,
    /// Page number for pagination
    pub page: Option<u32>,
    /// Sort field
    pub sort: Option<String>,
    /// Sort direction
    pub direction: Option<String>,
}

impl ListIssuesParams {
    /// Create default parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by state
    pub fn with_state(mut self, state: IssueState) -> Self {
        self.state = Some(state);
        self
    }

    /// Filter by labels
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Filter by assignee
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set page
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
}

/// Issue tracker trait
///
/// Implement this trait to add support for a new issue tracker backend.
#[async_trait::async_trait]
pub trait IssueTracker: Send + Sync {
    /// List issues matching the given parameters
    async fn list_issues(&self,
        params: ListIssuesParams,
    ) -> Result<Vec<TrackedIssue>>;

    /// Get a single issue by ID
    async fn get_issue(&self, id: u64) -> Result<TrackedIssue>;

    /// Create a new issue
    async fn create_issue(
        &self,
        title: &str,
        body: Option<&str>,
        labels: Option<Vec<String>>,
    ) -> Result<TrackedIssue>;

    /// Update an existing issue
    async fn update_issue(
        &self,
        id: u64,
        title: Option<&str>,
        body: Option<&str>,
        labels: Option<Vec<String>>,
    ) -> Result<TrackedIssue>;

    /// Close an issue
    async fn close_issue(&self, id: u64) -> Result<TrackedIssue>;

    /// Add labels to an issue
    async fn add_labels(&self,
        id: u64,
        labels: Vec<String>,
    ) -> Result<TrackedIssue>;

    /// Remove labels from an issue
    async fn remove_labels(
        &self,
        id: u64,
        labels: Vec<String>,
    ) -> Result<TrackedIssue>;

    /// Assign issue to users
    async fn assign_issue(
        &self,
        id: u64,
        assignees: Vec<String>,
    ) -> Result<TrackedIssue>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracked_issue_builder() {
        let issue = TrackedIssue::new(42, "Test Issue")
            .with_state(IssueState::Open)
            .with_label("bug")
            .with_label("priority/high")
            .with_assignees(vec!["alice".to_string(), "bob".to_string()])
            .with_priority("P1")
            .with_page_rank_score(0.85)
            .with_body("This is a test issue")
            .with_url("https://git.example.com/issues/42");

        assert_eq!(issue.id, 42);
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.state, IssueState::Open);
        assert_eq!(issue.labels.len(), 2);
        assert!(issue.has_label("bug"));
        assert!(issue.has_label("priority/high"));
        assert!(!issue.has_label("feature"));
        assert_eq!(issue.assignees.len(), 2);
        assert!(issue.is_assigned_to("alice"));
        assert!(issue.is_assigned_to("bob"));
        assert!(!issue.is_assigned_to("charlie"));
        assert_eq!(issue.priority, Some("P1".to_string()));
        assert_eq!(issue.page_rank_score, Some(0.85));
        assert_eq!(issue.body, Some("This is a test issue".to_string()));
        assert_eq!(issue.url, Some("https://git.example.com/issues/42".to_string()));
        assert!(issue.is_open());
        assert!(!issue.is_closed());
    }

    #[test]
    fn test_tracked_issue_closed() {
        let issue = TrackedIssue::new(1, "Closed Issue")
            .with_state(IssueState::Closed);
        
        assert!(issue.is_closed());
        assert!(!issue.is_open());
    }

    #[test]
    fn test_tracker_config_validation() {
        // Valid config
        let config = TrackerConfig::new(
            "https://git.example.com",
            "token123",
            "owner",
            "repo",
        );
        assert!(config.validate().is_ok());

        // Empty URL
        let config = TrackerConfig::new("", "token", "owner", "repo");
        assert!(config.validate().is_err());

        // Empty token
        let config = TrackerConfig::new("https://git.example.com", "", "owner", "repo");
        assert!(config.validate().is_err());

        // Empty owner
        let config = TrackerConfig::new("https://git.example.com", "token", "", "repo");
        assert!(config.validate().is_err());

        // Empty repo
        let config = TrackerConfig::new("https://git.example.com", "token", "owner", "");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tracker_config_with_robot_url() {
        let config = TrackerConfig::new(
            "https://git.example.com",
            "token123",
            "owner",
            "repo",
        ).with_robot_url("https://robot.example.com");

        assert_eq!(config.robot_url, Some("https://robot.example.com".to_string()));
    }

    #[test]
    fn test_list_issues_params_builder() {
        let params = ListIssuesParams::new()
            .with_state(IssueState::Open)
            .with_labels(vec!["bug".to_string(), "urgent".to_string()])
            .with_assignee("alice")
            .with_limit(50)
            .with_page(2);

        assert_eq!(params.state, Some(IssueState::Open));
        assert_eq!(params.labels, Some(vec!["bug".to_string(), "urgent".to_string()]));
        assert_eq!(params.assignee, Some("alice".to_string()));
        assert_eq!(params.limit, Some(50));
        assert_eq!(params.page, Some(2));
    }

    #[test]
    fn test_issue_state_display() {
        assert_eq!(format!("{}", IssueState::Open), "open");
        assert_eq!(format!("{}", IssueState::Closed), "closed");
    }

    #[test]
    fn test_tracked_issue_default() {
        let issue = TrackedIssue::new(1, "Default Issue");
        
        assert_eq!(issue.state, IssueState::Open);
        assert!(issue.labels.is_empty());
        assert!(issue.assignees.is_empty());
        assert!(issue.priority.is_none());
        assert!(issue.page_rank_score.is_none());
        assert!(issue.body.is_none());
        assert!(issue.url.is_none());
    }

    #[test]
    fn test_tracked_issue_has_label_case_insensitive() {
        let issue = TrackedIssue::new(1, "Test")
            .with_label("Bug")
            .with_label("FEATURE");

        assert!(issue.has_label("bug"));
        assert!(issue.has_label("BUG"));
        assert!(issue.has_label("Bug"));
        assert!(issue.has_label("feature"));
        assert!(issue.has_label("Feature"));
        assert!(!issue.has_label("documentation"));
    }

    #[test]
    fn test_tracked_issue_is_assigned_to_case_insensitive() {
        let issue = TrackedIssue::new(1, "Test")
            .with_assignees(vec!["Alice".to_string(), "BOB".to_string()]);

        assert!(issue.is_assigned_to("alice"));
        assert!(issue.is_assigned_to("ALICE"));
        assert!(issue.is_assigned_to("bob"));
        assert!(!issue.is_assigned_to("charlie"));
    }
}
