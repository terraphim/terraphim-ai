use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentIdentity {
    pub agent_name: String,
    #[serde(default)]
    pub gitea_login: Option<String>,
    #[serde(default)]
    pub token_path: Option<PathBuf>,
}

impl AgentIdentity {
    pub fn new(agent_name: impl Into<String>) -> Self {
        Self {
            agent_name: agent_name.into(),
            gitea_login: None,
            token_path: None,
        }
    }

    pub fn resolved_gitea_login(&self) -> &str {
        self.gitea_login.as_deref().unwrap_or(&self.agent_name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationRuleKind {
    Mention,
    Assigned,
    LabelAdded,
    Reopened,
    CommentCreated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationRule {
    pub kind: NotificationRuleKind,
    pub target_agent: String,
    #[serde(default = "default_rule_enabled")]
    pub enabled: bool,
}

fn default_rule_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegationPolicy {
    #[serde(default)]
    pub allowed_specialists: Vec<String>,
    #[serde(default = "default_exclusive_assignment")]
    pub exclusive_assignment: bool,
    #[serde(default = "default_max_fanout_depth")]
    pub max_fanout_depth: u8,
}

fn default_exclusive_assignment() -> bool {
    true
}

fn default_max_fanout_depth() -> u8 {
    1
}

impl Default for DelegationPolicy {
    fn default() -> Self {
        Self {
            allowed_specialists: vec![],
            exclusive_assignment: default_exclusive_assignment(),
            max_fanout_depth: default_max_fanout_depth(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GiteaConnection {
    pub base_url: String,
    pub owner: String,
    pub repo: String,
    #[serde(default)]
    pub token_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListenerConfig {
    pub identity: AgentIdentity,
    #[serde(default)]
    pub gitea: Option<GiteaConnection>,
    #[serde(default)]
    pub claim_strategy: terraphim_tracker::gitea::ClaimStrategy,
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,
    #[serde(default)]
    pub notification_rules: Vec<NotificationRule>,
    #[serde(default)]
    pub delegation: DelegationPolicy,
    #[serde(default)]
    pub repo_scope: Option<String>,
}

fn default_poll_interval_secs() -> u64 {
    30
}

impl ListenerConfig {
    pub fn for_identity(agent_name: impl Into<String>) -> Self {
        Self {
            identity: AgentIdentity::new(agent_name),
            gitea: None,
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
            poll_interval_secs: default_poll_interval_secs(),
            notification_rules: vec![],
            delegation: DelegationPolicy {
                allowed_specialists: vec![],
                exclusive_assignment: true,
                max_fanout_depth: 1,
            },
            repo_scope: None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.identity.agent_name.trim().is_empty() {
            bail!("identity.agent_name must not be empty");
        }
        if let Some(gitea) = &self.gitea {
            if gitea.base_url.trim().is_empty() {
                bail!("gitea.base_url must not be empty when gitea is configured");
            }
            if gitea.owner.trim().is_empty() {
                bail!("gitea.owner must not be empty when gitea is configured");
            }
            if gitea.repo.trim().is_empty() {
                bail!("gitea.repo must not be empty when gitea is configured");
            }
        }
        if self.poll_interval_secs == 0 {
            bail!("poll_interval_secs must be greater than zero");
        }
        if self.delegation.max_fanout_depth == 0 {
            bail!("delegation.max_fanout_depth must be at least 1");
        }
        let mut seen = BTreeSet::new();
        for specialist in &self.delegation.allowed_specialists {
            if specialist.trim().is_empty() {
                bail!("delegation.allowed_specialists cannot contain empty names");
            }
            if !seen.insert(specialist) {
                bail!("delegation.allowed_specialists contains duplicate entry: {specialist}");
            }
        }
        for rule in &self.notification_rules {
            if rule.target_agent.trim().is_empty() {
                bail!("notification_rules.target_agent must not be empty");
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read listener config from {}", path.display()))?;
        let config: Self = serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse listener config JSON from {}",
                path.display()
            )
        })?;
        config.validate()?;
        Ok(config)
    }
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn default_listener_config_uses_identity_as_login() {
        let config = ListenerConfig::for_identity("security-sentinel");
        assert_eq!(config.identity.agent_name, "security-sentinel");
        assert_eq!(config.identity.resolved_gitea_login(), "security-sentinel");
        assert_eq!(config.poll_interval_secs, 30);
        assert!(config.delegation.exclusive_assignment);
        assert_eq!(config.delegation.max_fanout_depth, 1);
        assert!(config.gitea.is_none());
        assert_eq!(
            config.claim_strategy,
            terraphim_tracker::gitea::ClaimStrategy::PreferRobot
        );
    }

    #[test]
    fn listener_config_validation_rejects_empty_identity() {
        let mut config = ListenerConfig::for_identity("security-sentinel");
        config.identity.agent_name = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn listener_config_validation_rejects_duplicate_specialists() {
        let mut config = ListenerConfig::for_identity("security-sentinel");
        config.delegation.allowed_specialists =
            vec!["test-guardian".into(), "test-guardian".into()];
        assert!(config.validate().is_err());
    }

    #[test]
    fn listener_config_validation_rejects_zero_poll_interval() {
        let mut config = ListenerConfig::for_identity("security-sentinel");
        config.poll_interval_secs = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn listener_config_loads_from_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("listener.json");
        let json = serde_json::json!({
            "identity": {
                "agent_name": "security-sentinel",
                "gitea_login": "security-sentinel"
            },
            "gitea": {
                "base_url": "https://git.example.com",
                "owner": "terraphim",
                "repo": "terraphim-ai"
            },
            "claim_strategy": "prefer_robot",
            "poll_interval_secs": 15,
            "notification_rules": [
                {"kind": "mention", "target_agent": "security-sentinel"}
            ],
            "delegation": {
                "allowed_specialists": ["test-guardian"],
                "exclusive_assignment": true,
                "max_fanout_depth": 1
            },
            "repo_scope": "terraphim/terraphim-ai"
        });
        fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();

        let config = ListenerConfig::load_from_path(&path).unwrap();
        assert_eq!(config.identity.agent_name, "security-sentinel");
        assert_eq!(config.poll_interval_secs, 15);
        assert_eq!(config.delegation.allowed_specialists, vec!["test-guardian"]);
        assert_eq!(config.repo_scope.as_deref(), Some("terraphim/terraphim-ai"));
        assert!(config.gitea.is_some());
    }

    #[tokio::test]
    async fn listener_runtime_claims_and_posts_ack() {
        let mock_server = MockServer::start().await;
        let token_dir = tempfile::tempdir().unwrap();
        let token_path = token_dir.path().join("token.txt");
        fs::write(&token_path, "test-token").unwrap();

        let issue_json = serde_json::json!({
            "id": 42,
            "number": 42,
            "title": "Listener target",
            "body": "Needs attention",
            "state": "open",
            "html_url": "https://example.com/issues/42",
            "created_at": "2026-04-04T10:00:00Z",
            "updated_at": "2026-04-04T10:00:00Z",
            "assignees": []
        });

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 100,
                    "issue_url": "https://example.com/api/v1/repos/testowner/testrepo/issues/42",
                    "body": "Please check @adf:security-sentinel",
                    "user": {"login": "alice"},
                    "created_at": "2026-04-04T11:00:00Z",
                    "updated_at": "2026-04-04T11:00:00Z"
                }
            ])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_json))
            .up_to_n_times(3)
            .expect(3)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "number": 42,
                "title": "Listener target",
                "body": "Needs attention",
                "state": "open",
                "html_url": "https://example.com/issues/42",
                "created_at": "2026-04-04T10:00:00Z",
                "updated_at": "2026-04-04T10:00:00Z",
                "assignees": [{"login": "security-sentinel"}]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "number": 42,
                "title": "Listener target",
                "state": "open",
                "assignees": [{"login": "security-sentinel"}]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42/comments"))
            .and(body_string_contains("session="))
            .and(body_string_contains("event="))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 200,
                "body": "ack",
                "user": {"login": "security-sentinel"},
                "created_at": "2026-04-04T12:00:00Z",
                "updated_at": "2026-04-04T12:00:00Z"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = ListenerConfig {
            identity: AgentIdentity {
                agent_name: "security-sentinel".to_string(),
                gitea_login: Some("security-sentinel".to_string()),
                token_path: Some(token_path),
            },
            gitea: Some(GiteaConnection {
                base_url: mock_server.uri(),
                owner: "testowner".to_string(),
                repo: "testrepo".to_string(),
                token_path: None,
            }),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::ApiOnly,
            poll_interval_secs: 1,
            notification_rules: vec![],
            delegation: DelegationPolicy::default(),
            repo_scope: None,
        };

        let mut runtime = ListenerRuntime::new(config).unwrap();
        runtime.poll_once().await.unwrap();
    }

    #[tokio::test]
    async fn listener_handoff_assigns_specialist_and_posts_note() {
        let mock_server = MockServer::start().await;
        let token_dir = tempfile::tempdir().unwrap();
        let token_path = token_dir.path().join("token.txt");
        fs::write(&token_path, "test-token").unwrap();

        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "number": 42,
                "title": "Listener target",
                "state": "open",
                "assignees": [{"login": "test-guardian"}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42/comments"))
            .and(body_string_contains("session=sess-42"))
            .and(body_string_contains("event=evt-42"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 201,
                "body": "handoff note",
                "user": {"login": "security-sentinel"},
                "created_at": "2026-04-04T12:30:00Z",
                "updated_at": "2026-04-04T12:30:00Z"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = ListenerConfig {
            identity: AgentIdentity {
                agent_name: "security-sentinel".to_string(),
                gitea_login: Some("security-sentinel".to_string()),
                token_path: Some(token_path),
            },
            gitea: Some(GiteaConnection {
                base_url: mock_server.uri(),
                owner: "testowner".to_string(),
                repo: "testrepo".to_string(),
                token_path: None,
            }),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::ApiOnly,
            poll_interval_secs: 1,
            notification_rules: vec![],
            delegation: DelegationPolicy {
                allowed_specialists: vec!["test-guardian".to_string()],
                exclusive_assignment: true,
                max_fanout_depth: 1,
            },
            repo_scope: None,
        };

        let runtime = ListenerRuntime::new(config).unwrap();
        runtime
            .handoff_issue_with_context(
                42,
                "test-guardian",
                "handoff note",
                Some("sess-42"),
                Some("evt-42"),
            )
            .await
            .unwrap();
    }

    #[test]
    fn listener_runtime_uses_gitea_token_path_when_identity_token_path_missing() {
        let token_dir = tempfile::tempdir().unwrap();
        let token_path = token_dir.path().join("token.txt");
        fs::write(&token_path, "test-token").unwrap();

        let config = ListenerConfig {
            identity: AgentIdentity {
                agent_name: "security-sentinel".to_string(),
                gitea_login: Some("security-sentinel".to_string()),
                token_path: None,
            },
            gitea: Some(GiteaConnection {
                base_url: "https://git.example.com".to_string(),
                owner: "testowner".to_string(),
                repo: "testrepo".to_string(),
                token_path: Some(token_path),
            }),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::ApiOnly,
            poll_interval_secs: 1,
            notification_rules: vec![],
            delegation: DelegationPolicy::default(),
            repo_scope: None,
        };

        assert!(ListenerRuntime::new(config).is_ok());
    }

    #[tokio::test]
    async fn listener_runtime_paginates_repo_comments_and_advances_cursor_to_latest_comment() {
        let mock_server = MockServer::start().await;
        let token_dir = tempfile::tempdir().unwrap();
        let token_path = token_dir.path().join("token.txt");
        fs::write(&token_path, "test-token").unwrap();

        let page_one: Vec<_> = (1..=50)
            .map(|id| {
                serde_json::json!({
                    "id": id,
                    "issue_url": null,
                    "body": "noise",
                    "user": {"login": "alice"},
                    "created_at": format!("2026-04-04T11:{:02}:00Z", (id - 1) % 60),
                    "updated_at": format!("2026-04-04T11:{:02}:00Z", (id - 1) % 60)
                })
            })
            .collect();

        let page_two = serde_json::json!([
            {
                "id": 51,
                "issue_url": "https://example.com/api/v1/repos/testowner/testrepo/issues/42",
                "body": "Please check @adf:security-sentinel",
                "user": {"login": "alice"},
                "created_at": "2026-04-04T12:30:00Z",
                "updated_at": "2026-04-04T12:30:00Z"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/comments"))
            .and(query_param("limit", "50"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page_one))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/comments"))
            .and(query_param("limit", "50"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page_two))
            .expect(1)
            .mount(&mock_server)
            .await;

        let issue_json = serde_json::json!({
            "id": 42,
            "number": 42,
            "title": "Listener target",
            "body": "Needs attention",
            "state": "open",
            "html_url": "https://example.com/issues/42",
            "created_at": "2026-04-04T10:00:00Z",
            "updated_at": "2026-04-04T10:00:00Z",
            "assignees": []
        });

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_json.clone()))
            .up_to_n_times(3)
            .expect(3)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "number": 42,
                "title": "Listener target",
                "body": "Needs attention",
                "state": "open",
                "html_url": "https://example.com/issues/42",
                "created_at": "2026-04-04T10:00:00Z",
                "updated_at": "2026-04-04T10:00:00Z",
                "assignees": [{"login": "security-sentinel"}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("PATCH"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "number": 42,
                "title": "Listener target",
                "state": "open",
                "assignees": [{"login": "security-sentinel"}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/v1/repos/testowner/testrepo/issues/42/comments"))
            .and(body_string_contains("session="))
            .and(body_string_contains("event="))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 200,
                "body": "ack",
                "user": {"login": "security-sentinel"},
                "created_at": "2026-04-04T12:31:00Z",
                "updated_at": "2026-04-04T12:31:00Z"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = ListenerConfig {
            identity: AgentIdentity {
                agent_name: "security-sentinel".to_string(),
                gitea_login: Some("security-sentinel".to_string()),
                token_path: Some(token_path),
            },
            gitea: Some(GiteaConnection {
                base_url: mock_server.uri(),
                owner: "testowner".to_string(),
                repo: "testrepo".to_string(),
                token_path: None,
            }),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::ApiOnly,
            poll_interval_secs: 1,
            notification_rules: vec![],
            delegation: DelegationPolicy::default(),
            repo_scope: None,
        };

        let mut runtime = ListenerRuntime::new(config).unwrap();
        runtime.last_seen_at = "2026-04-04T10:00:00Z".to_string();
        runtime.poll_once().await.unwrap();
        assert_eq!(runtime.last_seen_at, "2026-04-04T12:30:00+00:00");
    }
}

/// Runtime for a single identity-bound listener.
pub struct ListenerRuntime {
    config: ListenerConfig,
    tracker: terraphim_tracker::gitea::GiteaTracker,
    parser: terraphim_orchestrator::adf_commands::AdfCommandParser,
    repo_full_name: String,
    seen_events: std::collections::HashSet<String>,
    last_seen_at: String,
}

impl ListenerRuntime {
    pub fn new(config: ListenerConfig) -> Result<Self> {
        config.validate()?;

        let gitea = config
            .gitea
            .as_ref()
            .context("listener gitea configuration is required to run")?;

        let token = if let Some(path) = config
            .identity
            .token_path
            .as_ref()
            .or(gitea.token_path.as_ref())
        {
            fs::read_to_string(path)
                .with_context(|| format!("failed to read agent token from {}", path.display()))?
                .trim()
                .to_string()
        } else {
            std::env::var("GITEA_TOKEN")
                .context("GITEA_TOKEN must be set when no token_path is configured")?
        };

        let tracker =
            terraphim_tracker::gitea::GiteaTracker::new(terraphim_tracker::gitea::GiteaConfig {
                base_url: gitea.base_url.clone(),
                token,
                owner: gitea.owner.clone(),
                repo: gitea.repo.clone(),
                active_states: vec!["open".to_string()],
                terminal_states: vec!["closed".to_string()],
                use_robot_api: false,
                robot_path: PathBuf::from("/home/alex/go/bin/gitea-robot"),
                claim_strategy: config.claim_strategy,
            })?;

        let agent_names = vec![config.identity.resolved_gitea_login().to_string()];
        let parser = terraphim_orchestrator::adf_commands::AdfCommandParser::new(&agent_names, &[]);

        Ok(Self {
            repo_full_name: format!("{}/{}", gitea.owner, gitea.repo),
            config,
            tracker,
            parser,
            seen_events: std::collections::HashSet::new(),
            last_seen_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn run_forever(mut self) -> Result<()> {
        loop {
            self.poll_once().await?;
            tokio::time::sleep(std::time::Duration::from_secs(
                self.config.poll_interval_secs,
            ))
            .await;
        }
    }

    #[allow(dead_code)]
    pub async fn run_once(mut self) -> Result<()> {
        self.poll_once().await
    }

    pub async fn poll_once(&mut self) -> Result<()> {
        let mut page = 1u32;
        let mut newest_seen_at: Option<chrono::DateTime<chrono::Utc>> = None;

        loop {
            let comments = self
                .tracker
                .fetch_repo_comments_page(Some(&self.last_seen_at), Some(50), Some(page))
                .await?;
            let comment_count = comments.len();

            for comment in comments {
                if let Some(timestamp) = Self::comment_timestamp(&comment) {
                    newest_seen_at =
                        Some(newest_seen_at.map_or(timestamp, |current| current.max(timestamp)));
                }
                self.process_comment(comment).await?;
            }

            if comment_count < 50 {
                break;
            }
            page += 1;
        }

        if let Some(newest_seen_at) = newest_seen_at {
            self.last_seen_at = newest_seen_at.to_rfc3339();
        }
        Ok(())
    }

    fn comment_timestamp(
        comment: &terraphim_tracker::IssueComment,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        comment
            .updated_at
            .parse::<chrono::DateTime<chrono::FixedOffset>>()
            .or_else(|_| {
                comment
                    .created_at
                    .parse::<chrono::DateTime<chrono::FixedOffset>>()
            })
            .ok()
            .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
    }

    async fn process_comment(&mut self, comment: terraphim_tracker::IssueComment) -> Result<()> {
        if comment.issue_number == 0 {
            return Ok(());
        }

        let issue = match self.tracker.fetch_issue(comment.issue_number).await {
            Ok(issue) => issue,
            Err(e) => {
                tracing::warn!(issue = comment.issue_number, error = %e, "failed to fetch issue for listener event");
                return Ok(());
            }
        };

        let commands = self
            .parser
            .parse_commands(&comment.body, comment.issue_number, comment.id);

        for cmd in commands {
            let maybe_event = terraphim_orchestrator::control_plane::normalize_polled_command(
                &cmd,
                &self.repo_full_name,
                Some(issue.title.clone()),
                Some(issue.state.clone()),
                &comment,
            );

            let event = match maybe_event {
                Some(event) => event,
                None => continue,
            };

            if event.target_agent_name != self.config.identity.resolved_gitea_login() {
                continue;
            }

            if !self.seen_events.insert(event.event_id.clone()) {
                continue;
            }

            let claim = self
                .tracker
                .claim_issue(
                    self.config.identity.resolved_gitea_login(),
                    event.issue_number,
                    self.config.claim_strategy,
                )
                .await?;

            match claim {
                terraphim_tracker::gitea::ClaimResult::Success
                | terraphim_tracker::gitea::ClaimResult::AlreadyAssigned => {
                    let ack = format!(
                        "Terraphim agent `{}` accepted `@adf:{}` on comment #{}. session={} event={}",
                        self.config.identity.resolved_gitea_login(),
                        event.target_agent_name,
                        event.comment_id.unwrap_or(comment.id),
                        event.session_id,
                        event.event_id,
                    );
                    let _ = self.tracker.post_comment(event.issue_number, &ack).await;
                }
                terraphim_tracker::gitea::ClaimResult::AssignedToOther { assignee } => {
                    tracing::info!(
                        issue = event.issue_number,
                        assignee = %assignee,
                        "listener skipped event because the issue is already owned by another agent"
                    );
                }
                terraphim_tracker::gitea::ClaimResult::NotFound => {
                    tracing::warn!(
                        issue = event.issue_number,
                        "listener claim target not found"
                    );
                }
                terraphim_tracker::gitea::ClaimResult::PermissionDenied { reason } => {
                    tracing::warn!(issue = event.issue_number, %reason, "listener claim permission denied");
                }
                terraphim_tracker::gitea::ClaimResult::TransientFailure { reason } => {
                    tracing::warn!(issue = event.issue_number, %reason, "listener claim transient failure");
                }
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn handoff_issue(
        &self,
        issue_number: u64,
        specialist_name: &str,
        note: &str,
    ) -> Result<()> {
        self.handoff_issue_with_context(issue_number, specialist_name, note, None, None)
            .await
    }

    pub async fn handoff_issue_with_context(
        &self,
        issue_number: u64,
        specialist_name: &str,
        note: &str,
        session_id: Option<&str>,
        event_id: Option<&str>,
    ) -> Result<()> {
        if !self
            .config
            .delegation
            .allowed_specialists
            .iter()
            .any(|name| name == specialist_name)
        {
            anyhow::bail!("specialist '{specialist_name}' is not allowlisted for delegation");
        }

        self.tracker
            .assign_issue(issue_number, &[specialist_name])
            .await?;
        let note = match (session_id, event_id) {
            (Some(session_id), Some(event_id)) => {
                format!("{} session={} event={}", note, session_id, event_id)
            }
            (Some(session_id), None) => format!("{} session={}", note, session_id),
            _ => note.to_string(),
        };
        let _ = self.tracker.post_comment(issue_number, &note).await?;
        Ok(())
    }
}

pub async fn run_listener(config: ListenerConfig) -> Result<()> {
    ListenerRuntime::new(config)?.run_forever().await
}
