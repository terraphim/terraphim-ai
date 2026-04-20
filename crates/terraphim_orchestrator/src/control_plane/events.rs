//! Unified event model for ADF agent events.
//!
//! This module provides a normalized representation of agent-triggering events
//! that works across multiple ingestion paths (webhook, poll, notification).
//! All events are converted to `NormalizedAgentEvent` for consistent processing.

use crate::adf_commands::AdfCommand;
use crate::webhook::WebhookDispatch;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Origin of an agent event - indicates how the event was received.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventOrigin {
    /// Event received via webhook (real-time push)
    Webhook,
    /// Event discovered via polling (pull-based)
    Poll,
    /// Event from notification service
    Notification,
}

/// Type of command that triggered the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandKind {
    /// Direct agent spawn command (@adf:agent-name)
    SpawnAgent,
    /// Persona-based agent spawn (@adf:persona-name)
    SpawnPersona,
    /// Compound review trigger (@adf:compound-review)
    CompoundReview,
    /// Automated PR review triggered by pull_request webhook event
    ReviewPr,
}

/// Normalized representation of an agent-triggering event.
///
/// This struct unifies events from webhooks, polling, and notifications into
/// a single internal format. The `event_id` is stable across different ingestion
/// paths for the same underlying comment/agent combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedAgentEvent {
    /// Stable cross-path event identifier (derived from repo/issue/comment)
    pub event_id: String,
    /// Session identifier for grouping related events (derived from repo/issue)
    pub session_id: String,
    /// How this event was received
    pub origin: EventOrigin,
    /// Full repository name (e.g., "owner/repo")
    pub repo_full_name: String,
    /// Issue number where the command was issued
    pub issue_number: u64,
    /// Issue title (if available)
    pub issue_title: Option<String>,
    /// Issue state (e.g., "open", "closed")
    pub issue_state: Option<String>,
    /// Comment ID that triggered the agent
    pub comment_id: Option<u64>,
    /// When the comment was created (ISO 8601)
    pub comment_created_at: Option<String>,
    /// Author of the comment
    pub comment_author: Option<String>,
    /// Full body of the comment containing the command
    pub comment_body: String,
    /// Name of the agent to be spawned
    pub target_agent_name: String,
    /// Type of command that was issued
    pub command_kind: CommandKind,
    /// Context extracted after the command in the comment
    pub context: String,
    /// Raw command text as it appeared in the comment
    pub raw_command: String,
}

/// Generate a stable event ID from repo/issue/comment.
///
/// This ensures the same comment produces the same event_id regardless of
/// whether it was received via webhook or polling.
fn generate_event_id(repo_full_name: &str, issue_number: u64, comment_id: u64) -> String {
    let mut hasher = DefaultHasher::new();
    repo_full_name.hash(&mut hasher);
    issue_number.hash(&mut hasher);
    comment_id.hash(&mut hasher);
    format!("evt:{:016x}", hasher.finish())
}

/// Generate a session ID from repo/issue.
///
/// All events for the same issue share a session_id for grouping.
fn generate_session_id(repo_full_name: &str, issue_number: u64) -> String {
    let mut hasher = DefaultHasher::new();
    repo_full_name.hash(&mut hasher);
    issue_number.hash(&mut hasher);
    format!("ses:{:016x}", hasher.finish())
}

/// Normalize a polled command (from AdfCommand) into a NormalizedAgentEvent.
///
/// # Arguments
/// * `cmd` - The AdfCommand from the poll-based parser
/// * `repo_full_name` - Full repository name (e.g., "owner/repo")
/// * `issue_title` - Title of the issue (optional)
/// * `issue_state` - State of the issue (optional)
/// * `comment` - The IssueComment containing metadata
///
/// # Returns
/// Some(NormalizedAgentEvent) if the command can be normalized,
/// None for Unknown commands.
pub fn normalize_polled_command(
    cmd: &AdfCommand,
    repo_full_name: &str,
    issue_title: Option<String>,
    issue_state: Option<String>,
    comment: &terraphim_tracker::IssueComment,
) -> Option<NormalizedAgentEvent> {
    match cmd {
        AdfCommand::SpawnAgent {
            agent_name,
            issue_number,
            comment_id,
            context,
        } => {
            let event_id = generate_event_id(repo_full_name, *issue_number, *comment_id);
            let session_id = generate_session_id(repo_full_name, *issue_number);
            let raw_command = format!("@adf:{} {}", agent_name, context);

            Some(NormalizedAgentEvent {
                event_id,
                session_id,
                origin: EventOrigin::Poll,
                repo_full_name: repo_full_name.to_string(),
                issue_number: *issue_number,
                issue_title,
                issue_state,
                comment_id: Some(*comment_id),
                comment_created_at: Some(comment.created_at.clone()),
                comment_author: Some(comment.user.login.clone()),
                comment_body: comment.body.clone(),
                target_agent_name: agent_name.clone(),
                command_kind: CommandKind::SpawnAgent,
                context: context.clone(),
                raw_command,
            })
        }
        AdfCommand::SpawnPersona {
            persona_name,
            issue_number,
            comment_id,
            context,
        } => {
            let event_id = generate_event_id(repo_full_name, *issue_number, *comment_id);
            let session_id = generate_session_id(repo_full_name, *issue_number);
            let raw_command = format!("@adf:{} {}", persona_name, context);

            Some(NormalizedAgentEvent {
                event_id,
                session_id,
                origin: EventOrigin::Poll,
                repo_full_name: repo_full_name.to_string(),
                issue_number: *issue_number,
                issue_title,
                issue_state,
                comment_id: Some(*comment_id),
                comment_created_at: Some(comment.created_at.clone()),
                comment_author: Some(comment.user.login.clone()),
                comment_body: comment.body.clone(),
                target_agent_name: persona_name.clone(),
                command_kind: CommandKind::SpawnPersona,
                context: context.clone(),
                raw_command,
            })
        }
        AdfCommand::CompoundReview {
            issue_number,
            comment_id,
        } => {
            let event_id = generate_event_id(repo_full_name, *issue_number, *comment_id);
            let session_id = generate_session_id(repo_full_name, *issue_number);
            let raw_command = "@adf:compound-review".to_string();

            Some(NormalizedAgentEvent {
                event_id,
                session_id,
                origin: EventOrigin::Poll,
                repo_full_name: repo_full_name.to_string(),
                issue_number: *issue_number,
                issue_title,
                issue_state,
                comment_id: Some(*comment_id),
                comment_created_at: Some(comment.created_at.clone()),
                comment_author: Some(comment.user.login.clone()),
                comment_body: comment.body.clone(),
                target_agent_name: "compound-review".to_string(),
                command_kind: CommandKind::CompoundReview,
                context: String::new(),
                raw_command,
            })
        }
        AdfCommand::Unknown { .. } => None,
    }
}

/// Context needed for webhook normalization that isn't in WebhookDispatch.
///
/// This struct groups the additional metadata needed from the webhook payload
/// to fully populate a NormalizedAgentEvent.
#[derive(Debug, Clone)]
pub struct WebhookContext {
    pub repo_full_name: String,
    pub issue_title: String,
    pub issue_state: String,
    pub comment_created_at: String,
    pub comment_author: String,
    pub comment_body: String,
}

/// Normalize a webhook dispatch into a NormalizedAgentEvent.
///
/// # Arguments
/// * `dispatch` - The WebhookDispatch from the webhook handler
/// * `ctx` - Additional context from the webhook payload
///
/// # Returns
/// NormalizedAgentEvent representing the webhook dispatch.
pub fn normalize_webhook_dispatch(
    dispatch: &WebhookDispatch,
    ctx: &WebhookContext,
) -> NormalizedAgentEvent {
    match dispatch {
        WebhookDispatch::SpawnAgent {
            agent_name,
            issue_number,
            comment_id,
            context,
            ..
        } => {
            let event_id = generate_event_id(&ctx.repo_full_name, *issue_number, *comment_id);
            let session_id = generate_session_id(&ctx.repo_full_name, *issue_number);
            let raw_command = format!("@adf:{} {}", agent_name, context);

            NormalizedAgentEvent {
                event_id,
                session_id,
                origin: EventOrigin::Webhook,
                repo_full_name: ctx.repo_full_name.clone(),
                issue_number: *issue_number,
                issue_title: Some(ctx.issue_title.clone()),
                issue_state: Some(ctx.issue_state.clone()),
                comment_id: Some(*comment_id),
                comment_created_at: Some(ctx.comment_created_at.clone()),
                comment_author: Some(ctx.comment_author.clone()),
                comment_body: ctx.comment_body.clone(),
                target_agent_name: agent_name.clone(),
                command_kind: CommandKind::SpawnAgent,
                context: context.clone(),
                raw_command,
            }
        }
        WebhookDispatch::SpawnPersona {
            persona_name,
            issue_number,
            comment_id,
            context,
        } => {
            let event_id = generate_event_id(&ctx.repo_full_name, *issue_number, *comment_id);
            let session_id = generate_session_id(&ctx.repo_full_name, *issue_number);
            let raw_command = format!("@adf:{} {}", persona_name, context);

            NormalizedAgentEvent {
                event_id,
                session_id,
                origin: EventOrigin::Webhook,
                repo_full_name: ctx.repo_full_name.clone(),
                issue_number: *issue_number,
                issue_title: Some(ctx.issue_title.clone()),
                issue_state: Some(ctx.issue_state.clone()),
                comment_id: Some(*comment_id),
                comment_created_at: Some(ctx.comment_created_at.clone()),
                comment_author: Some(ctx.comment_author.clone()),
                comment_body: ctx.comment_body.clone(),
                target_agent_name: persona_name.clone(),
                command_kind: CommandKind::SpawnPersona,
                context: context.clone(),
                raw_command,
            }
        }
        WebhookDispatch::CompoundReview {
            issue_number,
            comment_id,
        } => {
            let event_id = generate_event_id(&ctx.repo_full_name, *issue_number, *comment_id);
            let session_id = generate_session_id(&ctx.repo_full_name, *issue_number);
            let raw_command = "@adf:compound-review".to_string();

            NormalizedAgentEvent {
                event_id,
                session_id,
                origin: EventOrigin::Webhook,
                repo_full_name: ctx.repo_full_name.clone(),
                issue_number: *issue_number,
                issue_title: Some(ctx.issue_title.clone()),
                issue_state: Some(ctx.issue_state.clone()),
                comment_id: Some(*comment_id),
                comment_created_at: Some(ctx.comment_created_at.clone()),
                comment_author: Some(ctx.comment_author.clone()),
                comment_body: ctx.comment_body.clone(),
                target_agent_name: "compound-review".to_string(),
                command_kind: CommandKind::CompoundReview,
                context: String::new(),
                raw_command,
            }
        }
        WebhookDispatch::ReviewPr {
            pr_number,
            project,
            head_sha,
            author_login,
            title,
            diff_loc,
        } => {
            let event_id = generate_event_id(&ctx.repo_full_name, *pr_number, 0);
            let session_id = generate_session_id(&ctx.repo_full_name, *pr_number);
            let raw_command = format!("pr-review#{}", pr_number);

            NormalizedAgentEvent {
                event_id,
                session_id,
                origin: EventOrigin::Webhook,
                repo_full_name: ctx.repo_full_name.clone(),
                issue_number: *pr_number,
                issue_title: Some(title.clone()),
                issue_state: None,
                comment_id: None,
                comment_created_at: None,
                comment_author: Some(author_login.clone()),
                comment_body: String::new(),
                target_agent_name: format!("review-pr/{}", project),
                command_kind: CommandKind::ReviewPr,
                context: format!("sha={} diff_loc={}", head_sha, diff_loc),
                raw_command,
            }
        }
    }
}

/// Generate a stable deduplication key for an event.
///
/// This key is used to detect duplicate events across different ingestion paths.
/// Events with the same (comment_id, target_agent_name) combination are considered
/// duplicates.
///
/// The key format is stable: `{comment_id}:{agent_name}`
pub fn dedup_key(event: &NormalizedAgentEvent) -> String {
    format!(
        "{}:{}",
        event.comment_id.unwrap_or(0),
        event.target_agent_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_comment() -> terraphim_tracker::IssueComment {
        terraphim_tracker::IssueComment {
            id: 12345,
            body: "Please @adf:security-sentinel review this code".to_string(),
            user: terraphim_tracker::CommentUser {
                login: "alice".to_string(),
            },
            issue_number: 42,
            created_at: "2026-04-10T12:00:00Z".to_string(),
            updated_at: "2026-04-10T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_dedup_key_generation() {
        let event = NormalizedAgentEvent {
            event_id: "evt:test".to_string(),
            session_id: "ses:test".to_string(),
            origin: EventOrigin::Poll,
            repo_full_name: "owner/repo".to_string(),
            issue_number: 42,
            issue_title: Some("Test Issue".to_string()),
            issue_state: Some("open".to_string()),
            comment_id: Some(12345),
            comment_created_at: Some("2026-04-10T12:00:00Z".to_string()),
            comment_author: Some("alice".to_string()),
            comment_body: "Test comment".to_string(),
            target_agent_name: "security-sentinel".to_string(),
            command_kind: CommandKind::SpawnAgent,
            context: "review this code".to_string(),
            raw_command: "@adf:security-sentinel review this code".to_string(),
        };

        let key = dedup_key(&event);
        assert_eq!(key, "12345:security-sentinel");
    }

    #[test]
    fn test_dedup_key_same_for_poll_and_webhook() {
        let ctx = WebhookContext {
            repo_full_name: "owner/repo".to_string(),
            issue_title: "Test Issue".to_string(),
            issue_state: "open".to_string(),
            comment_created_at: "2026-04-10T12:00:00Z".to_string(),
            comment_author: "alice".to_string(),
            comment_body: "Please @adf:security-sentinel review this".to_string(),
        };

        let webhook_dispatch = WebhookDispatch::SpawnAgent {
            agent_name: "security-sentinel".to_string(),
            detected_project: None,
            issue_number: 42,
            comment_id: 12345,
            context: "review this".to_string(),
        };

        let poll_cmd = AdfCommand::SpawnAgent {
            agent_name: "security-sentinel".to_string(),
            issue_number: 42,
            comment_id: 12345,
            context: "review this".to_string(),
        };

        let webhook_event = normalize_webhook_dispatch(&webhook_dispatch, &ctx);
        let poll_event = normalize_polled_command(
            &poll_cmd,
            "owner/repo",
            Some("Test Issue".to_string()),
            Some("open".to_string()),
            &test_comment(),
        )
        .unwrap();

        // Dedup keys should match for same comment/agent
        assert_eq!(dedup_key(&webhook_event), dedup_key(&poll_event));
        assert_eq!(dedup_key(&webhook_event), "12345:security-sentinel");
    }

    #[test]
    fn test_normalize_spawn_agent_from_poll() {
        let comment = test_comment();
        let cmd = AdfCommand::SpawnAgent {
            agent_name: "security-sentinel".to_string(),
            issue_number: 42,
            comment_id: 12345,
            context: "check for vulnerabilities".to_string(),
        };

        let event = normalize_polled_command(
            &cmd,
            "terraphim/terraphim-ai",
            Some("Security Review".to_string()),
            Some("open".to_string()),
            &comment,
        )
        .unwrap();

        assert_eq!(event.origin, EventOrigin::Poll);
        assert_eq!(event.repo_full_name, "terraphim/terraphim-ai");
        assert_eq!(event.issue_number, 42);
        assert_eq!(event.issue_title, Some("Security Review".to_string()));
        assert_eq!(event.issue_state, Some("open".to_string()));
        assert_eq!(event.comment_id, Some(12345));
        assert_eq!(event.comment_author, Some("alice".to_string()));
        assert_eq!(
            event.comment_body,
            "Please @adf:security-sentinel review this code"
        );
        assert_eq!(event.target_agent_name, "security-sentinel");
        assert_eq!(event.command_kind, CommandKind::SpawnAgent);
        assert_eq!(event.context, "check for vulnerabilities");
        assert_eq!(
            event.raw_command,
            "@adf:security-sentinel check for vulnerabilities"
        );

        // Event ID should be stable
        assert!(event.event_id.starts_with("evt:"));
        assert!(event.session_id.starts_with("ses:"));
    }

    #[test]
    fn test_normalize_spawn_persona_from_poll() {
        let comment = test_comment();
        let cmd = AdfCommand::SpawnPersona {
            persona_name: "vigil".to_string(),
            issue_number: 42,
            comment_id: 12345,
            context: "security audit".to_string(),
        };

        let event = normalize_polled_command(&cmd, "owner/repo", None, None, &comment).unwrap();

        assert_eq!(event.origin, EventOrigin::Poll);
        assert_eq!(event.target_agent_name, "vigil");
        assert_eq!(event.command_kind, CommandKind::SpawnPersona);
        assert_eq!(event.raw_command, "@adf:vigil security audit");
    }

    #[test]
    fn test_normalize_compound_review_from_poll() {
        let comment = test_comment();
        let cmd = AdfCommand::CompoundReview {
            issue_number: 42,
            comment_id: 12345,
        };

        let event = normalize_polled_command(
            &cmd,
            "owner/repo",
            Some("Review Needed".to_string()),
            Some("open".to_string()),
            &comment,
        )
        .unwrap();

        assert_eq!(event.origin, EventOrigin::Poll);
        assert_eq!(event.target_agent_name, "compound-review");
        assert_eq!(event.command_kind, CommandKind::CompoundReview);
        assert_eq!(event.context, "");
        assert_eq!(event.raw_command, "@adf:compound-review");
    }

    #[test]
    fn test_normalize_unknown_command_returns_none() {
        let comment = test_comment();
        let cmd = AdfCommand::Unknown {
            raw: "@adf:unknown-cmd".to_string(),
        };

        let result = normalize_polled_command(&cmd, "owner/repo", None, None, &comment);

        assert!(result.is_none());
    }

    #[test]
    fn test_normalize_spawn_agent_from_webhook() {
        let ctx = WebhookContext {
            repo_full_name: "terraphim/terraphim-ai".to_string(),
            issue_title: "Security Review".to_string(),
            issue_state: "open".to_string(),
            comment_created_at: "2026-04-10T12:00:00Z".to_string(),
            comment_author: "alice".to_string(),
            comment_body: "Please @adf:security-sentinel review this".to_string(),
        };

        let dispatch = WebhookDispatch::SpawnAgent {
            agent_name: "security-sentinel".to_string(),
            detected_project: None,
            issue_number: 42,
            comment_id: 12345,
            context: "check for vulnerabilities".to_string(),
        };

        let event = normalize_webhook_dispatch(&dispatch, &ctx);

        assert_eq!(event.origin, EventOrigin::Webhook);
        assert_eq!(event.repo_full_name, "terraphim/terraphim-ai");
        assert_eq!(event.issue_number, 42);
        assert_eq!(event.issue_title, Some("Security Review".to_string()));
        assert_eq!(event.issue_state, Some("open".to_string()));
        assert_eq!(event.comment_id, Some(12345));
        assert_eq!(event.comment_author, Some("alice".to_string()));
        assert_eq!(
            event.comment_body,
            "Please @adf:security-sentinel review this"
        );
        assert_eq!(event.target_agent_name, "security-sentinel");
        assert_eq!(event.command_kind, CommandKind::SpawnAgent);
        assert_eq!(event.context, "check for vulnerabilities");
    }

    #[test]
    fn test_event_id_stability() {
        // Same inputs should always produce the same event_id
        let ctx = WebhookContext {
            repo_full_name: "owner/repo".to_string(),
            issue_title: "Test".to_string(),
            issue_state: "open".to_string(),
            comment_created_at: "2026-04-10T12:00:00Z".to_string(),
            comment_author: "alice".to_string(),
            comment_body: "Test body".to_string(),
        };

        let dispatch = WebhookDispatch::SpawnAgent {
            agent_name: "agent1".to_string(),
            detected_project: None,
            issue_number: 42,
            comment_id: 123,
            context: "do something".to_string(),
        };

        let event1 = normalize_webhook_dispatch(&dispatch, &ctx);
        let event2 = normalize_webhook_dispatch(&dispatch, &ctx);

        assert_eq!(event1.event_id, event2.event_id);
        assert_eq!(event1.session_id, event2.session_id);
    }

    #[test]
    fn test_event_id_different_for_different_comments() {
        let ctx1 = WebhookContext {
            repo_full_name: "owner/repo".to_string(),
            issue_title: "Test".to_string(),
            issue_state: "open".to_string(),
            comment_created_at: "2026-04-10T12:00:00Z".to_string(),
            comment_author: "alice".to_string(),
            comment_body: "Comment 1".to_string(),
        };

        let ctx2 = WebhookContext {
            repo_full_name: "owner/repo".to_string(),
            issue_title: "Test".to_string(),
            issue_state: "open".to_string(),
            comment_created_at: "2026-04-10T12:01:00Z".to_string(),
            comment_author: "bob".to_string(),
            comment_body: "Comment 2".to_string(),
        };

        let dispatch1 = WebhookDispatch::SpawnAgent {
            agent_name: "agent1".to_string(),
            detected_project: None,
            issue_number: 42,
            comment_id: 123,
            context: "do something".to_string(),
        };

        let dispatch2 = WebhookDispatch::SpawnAgent {
            agent_name: "agent1".to_string(),
            detected_project: None,
            issue_number: 42,
            comment_id: 124, // Different comment
            context: "do something".to_string(),
        };

        let event1 = normalize_webhook_dispatch(&dispatch1, &ctx1);
        let event2 = normalize_webhook_dispatch(&dispatch2, &ctx2);

        // Different comments should have different event IDs
        assert_ne!(event1.event_id, event2.event_id);
        // But same session (same issue)
        assert_eq!(event1.session_id, event2.session_id);
    }

    #[test]
    fn test_event_id_different_for_different_issues() {
        let ctx1 = WebhookContext {
            repo_full_name: "owner/repo".to_string(),
            issue_title: "Issue 1".to_string(),
            issue_state: "open".to_string(),
            comment_created_at: "2026-04-10T12:00:00Z".to_string(),
            comment_author: "alice".to_string(),
            comment_body: "Comment".to_string(),
        };

        let ctx2 = WebhookContext {
            repo_full_name: "owner/repo".to_string(),
            issue_title: "Issue 2".to_string(),
            issue_state: "open".to_string(),
            comment_created_at: "2026-04-10T12:00:00Z".to_string(),
            comment_author: "alice".to_string(),
            comment_body: "Comment".to_string(),
        };

        let dispatch1 = WebhookDispatch::SpawnAgent {
            agent_name: "agent1".to_string(),
            detected_project: None,
            issue_number: 42,
            comment_id: 123,
            context: "do something".to_string(),
        };

        let dispatch2 = WebhookDispatch::SpawnAgent {
            agent_name: "agent1".to_string(),
            detected_project: None,
            issue_number: 43, // Different issue
            comment_id: 123,
            context: "do something".to_string(),
        };

        let event1 = normalize_webhook_dispatch(&dispatch1, &ctx1);
        let event2 = normalize_webhook_dispatch(&dispatch2, &ctx2);

        // Different issues should have different event IDs and session IDs
        assert_ne!(event1.event_id, event2.event_id);
        assert_ne!(event1.session_id, event2.session_id);
    }

    #[test]
    fn test_cross_path_event_id_consistency() {
        // The same comment processed via poll vs webhook should have the same event_id
        let comment = test_comment();

        let poll_cmd = AdfCommand::SpawnAgent {
            agent_name: "security-sentinel".to_string(),
            issue_number: 42,
            comment_id: 12345,
            context: "review".to_string(),
        };

        let ctx = WebhookContext {
            repo_full_name: "owner/repo".to_string(),
            issue_title: "Test Issue".to_string(),
            issue_state: "open".to_string(),
            comment_created_at: comment.created_at.clone(),
            comment_author: comment.user.login.clone(),
            comment_body: comment.body.clone(),
        };

        let webhook_dispatch = WebhookDispatch::SpawnAgent {
            agent_name: "security-sentinel".to_string(),
            detected_project: None,
            issue_number: 42,
            comment_id: 12345,
            context: "review".to_string(),
        };

        let poll_event = normalize_polled_command(
            &poll_cmd,
            "owner/repo",
            Some("Test Issue".to_string()),
            Some("open".to_string()),
            &comment,
        )
        .unwrap();

        let webhook_event = normalize_webhook_dispatch(&webhook_dispatch, &ctx);

        // Both should produce identical event_id and session_id
        assert_eq!(poll_event.event_id, webhook_event.event_id);
        assert_eq!(poll_event.session_id, webhook_event.session_id);
    }
}
