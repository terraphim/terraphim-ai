//! Gitea webhook handler for real-time mention dispatch.
//!
//! Replaces poll-based mention detection with push-based webhook delivery.
//! Gitea sends POST requests on issue comment events, which are parsed
//! for @adf: commands and dispatched immediately.

use axum::{body::Bytes, extract::State, http::StatusCode, routing::post, Router};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use tracing::{info, warn};

use crate::adf_commands::AdfCommandParser;
use crate::persona::PersonaRegistry;

type HmacSha256 = Hmac<Sha256>;

/// Gitea webhook payload for issue_comment events.
#[derive(Debug, Deserialize)]
struct GiteaWebhookPayload {
    action: String,
    comment: GiteaComment,
    issue: GiteaIssue,
    repository: GiteaRepository,
}

#[derive(Debug, Deserialize)]
struct GiteaComment {
    id: u64,
    body: String,
    user: GiteaUser,
    created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaUser {
    pub login: String,
}

#[derive(Debug, Deserialize)]
struct GiteaIssue {
    number: u64,
    title: String,
    state: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaRepository {
    pub full_name: String,
}

/// Gitea webhook payload for pull_request events.
#[derive(Debug, Deserialize)]
pub struct GiteaPullRequestPayload {
    pub action: String,
    pub number: u64,
    pub pull_request: PullRequestFields,
    pub repository: GiteaRepository,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestFields {
    pub head: PrRef,
    pub base: PrRef,
    pub user: GiteaUser,
    pub title: String,
    pub draft: bool,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Deserialize)]
pub struct PrRef {
    pub sha: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
}

/// A dispatch request sent from the webhook handler to the orchestrator.
pub enum WebhookDispatch {
    SpawnAgent {
        agent_name: String,
        /// Project extracted from a qualified `@adf:project/name` mention, or
        /// `None` for unqualified `@adf:name` mentions.
        detected_project: Option<String>,
        issue_number: u64,
        comment_id: u64,
        context: String,
    },
    SpawnPersona {
        persona_name: String,
        issue_number: u64,
        comment_id: u64,
        context: String,
    },
    CompoundReview {
        issue_number: u64,
        comment_id: u64,
    },
    ReviewPr {
        pr_number: u64,
        project: String,
        head_sha: String,
        author_login: String,
        title: String,
        diff_loc: u32,
    },
    /// Push event dispatch — triggers the deterministic `build-runner` agent
    /// (Phase 3 of the ADF replaces-Gitea-Actions plan). The orchestrator's
    /// `handle_push` consumes this and spawns the bash agent that shells out
    /// to `rch exec` for `cargo fmt/clippy/test`.
    Push {
        /// Project id resolved from `repository.full_name` (the repo segment
        /// after the `/`, mirroring the `ReviewPr` derivation).
        project: String,
        /// Full git ref, e.g. `refs/heads/main` or `refs/tags/v1.0.0`.
        ref_name: String,
        /// Parent commit SHA (zeros for branch creation).
        before_sha: String,
        /// New tip commit SHA the build-runner must check out.
        after_sha: String,
        /// `pusher.login` from the Gitea payload, used for audit logging.
        pusher_login: String,
        /// Unique union of `added ∪ removed ∪ modified` paths across all
        /// commits in the payload, in stable insertion order.
        files_changed: Vec<String>,
    },
}

impl WebhookDispatch {
    /// Extract the comment_id from any dispatch variant.
    /// `ReviewPr` dispatches are not associated with a comment — returns 0.
    pub fn comment_id(&self) -> u64 {
        match self {
            Self::SpawnAgent { comment_id, .. } => *comment_id,
            Self::SpawnPersona { comment_id, .. } => *comment_id,
            Self::CompoundReview { comment_id, .. } => *comment_id,
            Self::ReviewPr { .. } => 0,
            Self::Push { .. } => 0,
        }
    }
}

/// Gitea webhook payload for `push` events (Phase 3).
#[derive(Debug, Deserialize)]
pub struct GiteaPushPayload {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub before: String,
    pub after: String,
    pub pusher: GiteaPusher,
    pub repository: GiteaRepository,
    #[serde(default)]
    pub commits: Vec<GiteaPushCommit>,
}

#[derive(Debug, Deserialize)]
pub struct GiteaPusher {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaPushCommit {
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
    #[serde(default)]
    pub modified: Vec<String>,
}

/// Shared state for the webhook handler.
#[derive(Clone)]
pub struct WebhookState {
    pub agent_names: Vec<String>,
    pub persona_registry: std::sync::Arc<PersonaRegistry>,
    pub dispatch_tx: tokio::sync::mpsc::Sender<WebhookDispatch>,
    pub secret: Option<String>,
}

/// Create the webhook router.
pub fn webhook_router(state: WebhookState) -> Router {
    Router::new()
        .route("/webhooks/gitea", post(handle_gitea_webhook))
        .with_state(state)
}

/// Handle incoming Gitea webhook.
async fn handle_gitea_webhook(
    State(state): State<WebhookState>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> StatusCode {
    // 1. Validate HMAC signature if secret is configured
    if let Some(ref secret) = state.secret {
        let sig_header = headers
            .get("X-Gitea-Signature")
            .or_else(|| headers.get("X-Hub-Signature-256"));

        match sig_header.and_then(|h| h.to_str().ok()) {
            Some(sig) => {
                if !verify_signature(secret, &body, sig) {
                    warn!("webhook signature verification failed");
                    return StatusCode::UNAUTHORIZED;
                }
            }
            None => {
                warn!("webhook secret configured but no signature header present");
                return StatusCode::BAD_REQUEST;
            }
        }
    }

    // 2. Route by event type header
    let event_type = headers
        .get("X-Gitea-Event")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("issue_comment");

    if event_type == "pull_request" {
        return handle_pull_request_event(&state, &body).await;
    }

    // Push events (Phase 3): explicit `X-Gitea-Event: push` header, or fall
    // back to JSON shape detection (presence of `ref` + `before` + `after` +
    // `commits`) for clients that omit the header. Falls through to the
    // legacy issue-comment parser otherwise.
    if event_type == "push" || looks_like_push_payload(&body) {
        return handle_push_event(&state, &body).await;
    }

    // 3. Parse payload
    let payload: GiteaWebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "failed to parse webhook payload");
            return StatusCode::BAD_REQUEST;
        }
    };

    info!(
        repo = %payload.repository.full_name,
        issue = payload.issue.number,
        issue_title = %payload.issue.title,
        issue_state = %payload.issue.state,
        comment_id = payload.comment.id,
        author = %payload.comment.user.login,
        created_at = %payload.comment.created_at,
        action = %payload.action,
        "received webhook event"
    );

    // 3. Only handle created comments (ignore edited/deleted)
    if payload.action != "created" {
        return StatusCode::OK;
    }

    // 4. Extract @adf: commands using existing parser
    let persona_names: Vec<String> = state
        .persona_registry
        .persona_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let parser = AdfCommandParser::new(&state.agent_names, &persona_names);

    // Parse all mention tokens first to capture project prefixes from qualified
    // mentions (`@adf:project/name`) before the Aho-Corasick parser strips them.
    let mention_tokens = crate::mention::parse_mention_tokens(&payload.comment.body);
    // Map agent_name -> detected project for unqualified mentions resolved by parser.
    let unqualified_project_map: std::collections::HashMap<String, Option<String>> = mention_tokens
        .iter()
        .filter(|t| t.project.is_none())
        .map(|t| (t.agent.clone(), None))
        .collect();

    let commands = parser.parse_commands(
        &payload.comment.body,
        payload.issue.number,
        payload.comment.id,
    );

    // Collect qualified tokens that AdfCommandParser cannot match (it only knows
    // `@adf:{name}` patterns and `@adf:project/name` is not a substring of those).
    let qualified_dispatches: Vec<WebhookDispatch> = mention_tokens
        .into_iter()
        .filter(|t| t.project.is_some())
        .map(|t| WebhookDispatch::SpawnAgent {
            detected_project: t.project,
            agent_name: t.agent,
            issue_number: payload.issue.number,
            comment_id: payload.comment.id,
            context: String::new(),
        })
        .collect();

    if commands.is_empty() && qualified_dispatches.is_empty() {
        return StatusCode::OK;
    }

    // 5. Dispatch each command to the orchestrator
    let mut commands_dispatched: u32 = 0;
    for cmd in commands {
        let dispatch = match cmd {
            crate::adf_commands::AdfCommand::SpawnAgent {
                agent_name,
                issue_number,
                comment_id,
                context,
            } => WebhookDispatch::SpawnAgent {
                detected_project: unqualified_project_map.get(&agent_name).cloned().flatten(),
                agent_name,
                issue_number,
                comment_id,
                context,
            },
            crate::adf_commands::AdfCommand::SpawnPersona {
                persona_name,
                issue_number,
                comment_id,
                context,
            } => WebhookDispatch::SpawnPersona {
                persona_name,
                issue_number,
                comment_id,
                context,
            },
            crate::adf_commands::AdfCommand::CompoundReview {
                issue_number,
                comment_id,
            } => WebhookDispatch::CompoundReview {
                issue_number,
                comment_id,
            },
            crate::adf_commands::AdfCommand::Unknown { raw } => {
                warn!(raw = %raw, "unknown ADF command from webhook");
                continue;
            }
        };

        if let Err(e) = state.dispatch_tx.send(dispatch).await {
            warn!(error = %e, "failed to send webhook dispatch to orchestrator");
            return StatusCode::SERVICE_UNAVAILABLE;
        }
        commands_dispatched += 1;
    }

    // Dispatch qualified mentions separately (AdfCommandParser can't see `@adf:proj/name`).
    for dispatch in qualified_dispatches {
        if let Err(e) = state.dispatch_tx.send(dispatch).await {
            warn!(error = %e, "failed to send qualified mention dispatch to orchestrator");
            return StatusCode::SERVICE_UNAVAILABLE;
        }
        commands_dispatched += 1;
    }

    info!(
        repo = %payload.repository.full_name,
        issue = payload.issue.number,
        comment_id = payload.comment.id,
        author = %payload.comment.user.login,
        commands = commands_dispatched,
        "webhook dispatch complete"
    );
    StatusCode::ACCEPTED
}

/// Handle Gitea `pull_request` event. Returns 200 for all parse/skip cases
/// (Gitea retries on non-2xx, causing spam).
pub async fn handle_pull_request_event(state: &WebhookState, body: &[u8]) -> StatusCode {
    let payload: GiteaPullRequestPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "failed to parse pull_request webhook payload");
            return StatusCode::OK;
        }
    };

    let action = payload.action.as_str();

    // Only enqueue for review-triggering actions on non-draft PRs.
    let is_review_action = matches!(
        action,
        "opened" | "synchronize" | "reopened" | "ready_for_review"
    );

    if !is_review_action || payload.pull_request.draft {
        info!(
            action = action,
            draft = payload.pull_request.draft,
            pr = payload.number,
            "skipped_pr_webhook"
        );
        return StatusCode::OK;
    }

    // Derive project from `owner/repo` → `repo`.
    let project = payload
        .repository
        .full_name
        .split('/')
        .next_back()
        .unwrap_or(&payload.repository.full_name)
        .to_string();

    let diff_loc = payload
        .pull_request
        .additions
        .saturating_add(payload.pull_request.deletions);

    let dispatch = WebhookDispatch::ReviewPr {
        pr_number: payload.number,
        project,
        head_sha: payload.pull_request.head.sha.clone(),
        author_login: payload.pull_request.user.login.clone(),
        title: payload.pull_request.title.clone(),
        diff_loc,
    };

    info!(
        pr = payload.number,
        action = action,
        author = %payload.pull_request.user.login,
        "webhook: enqueuing ReviewPr dispatch"
    );

    match state.dispatch_tx.send(dispatch).await {
        Ok(()) => StatusCode::ACCEPTED,
        Err(e) => {
            warn!(error = %e, "failed to send ReviewPr dispatch");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

/// JSON-shape sniffer for `push` events when the `X-Gitea-Event` header is
/// missing. Returns true when the body parses as an object containing the
/// trio of `ref`, `before`, `after` and a `commits` array — the unique
/// fingerprint of a Gitea push payload.
fn looks_like_push_payload(body: &[u8]) -> bool {
    let v: serde_json::Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return false,
    };
    v.get("ref").is_some()
        && v.get("before").is_some()
        && v.get("after").is_some()
        && v.get("commits").map(|c| c.is_array()).unwrap_or(false)
}

/// Aggregate `added ∪ removed ∪ modified` paths across all commits in a push
/// payload, deduplicating while preserving first-seen insertion order.
pub fn aggregate_files_changed(commits: &[GiteaPushCommit]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in commits {
        for path in c
            .added
            .iter()
            .chain(c.removed.iter())
            .chain(c.modified.iter())
        {
            if seen.insert(path.clone()) {
                out.push(path.clone());
            }
        }
    }
    out
}

/// Handle Gitea `push` event (Phase 3). Returns 200 for parse/skip cases so
/// Gitea does not retry-spam on malformed bodies; emits `ACCEPTED` once a
/// `WebhookDispatch::Push` has been forwarded to the orchestrator.
pub async fn handle_push_event(state: &WebhookState, body: &[u8]) -> StatusCode {
    let payload: GiteaPushPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "failed to parse push webhook payload");
            return StatusCode::OK;
        }
    };

    // Derive project from `owner/repo` → `repo`, mirroring the ReviewPr path.
    let project = payload
        .repository
        .full_name
        .split('/')
        .next_back()
        .unwrap_or(&payload.repository.full_name)
        .to_string();

    let files_changed = aggregate_files_changed(&payload.commits);

    let dispatch = WebhookDispatch::Push {
        project,
        ref_name: payload.ref_name.clone(),
        before_sha: payload.before.clone(),
        after_sha: payload.after.clone(),
        pusher_login: payload.pusher.login.clone(),
        files_changed,
    };

    info!(
        repo = %payload.repository.full_name,
        ref_name = %payload.ref_name,
        before = %payload.before,
        after = %payload.after,
        pusher = %payload.pusher.login,
        commits = payload.commits.len(),
        "webhook: enqueuing Push dispatch"
    );

    match state.dispatch_tx.send(dispatch).await {
        Ok(()) => StatusCode::ACCEPTED,
        Err(e) => {
            warn!(error = %e, "failed to send Push dispatch");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

/// Verify HMAC-SHA256 signature.
pub fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(body);
    let result = mac.finalize();
    let expected = result.into_bytes();

    // Strip "sha256=" prefix if present
    let sig_bytes: Vec<u8> =
        match hex::decode(signature.strip_prefix("sha256=").unwrap_or(signature)) {
            Ok(b) => b,
            Err(_) => return false,
        };

    expected.len() == sig_bytes.len() && expected.iter().zip(sig_bytes.iter()).all(|(a, b)| a == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature_valid() {
        let secret = "test-secret";
        let body = b"hello world";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let result = mac.finalize();
        let sig = hex::encode(result.into_bytes());
        assert!(verify_signature(secret, body, &sig));
    }

    #[test]
    fn test_verify_signature_invalid() {
        assert!(!verify_signature("secret", b"body", "deadbeef"));
    }

    #[test]
    fn test_verify_signature_with_prefix() {
        let secret = "test-secret";
        let body = b"hello world";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let result = mac.finalize();
        let sig = format!("sha256={}", hex::encode(result.into_bytes()));
        assert!(verify_signature(secret, body, &sig));
    }
}
