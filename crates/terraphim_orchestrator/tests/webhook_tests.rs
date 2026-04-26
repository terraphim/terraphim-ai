use std::sync::Arc;
use terraphim_orchestrator::webhook::{
    handle_pull_request_event, handle_push_event, verify_signature, WebhookDispatch, WebhookState,
};
use terraphim_orchestrator::PersonaRegistry;

fn make_state(
    secret: Option<&str>,
) -> (WebhookState, tokio::sync::mpsc::Receiver<WebhookDispatch>) {
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    let state = WebhookState {
        agent_names: vec![],
        persona_registry: Arc::new(PersonaRegistry::new()),
        dispatch_tx: tx,
        secret: secret.map(|s| s.to_string()),
    };
    (state, rx)
}

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/webhook")
        .join(name);
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
}

#[tokio::test]
async fn webhook_pr_opened_enqueues_review_dispatch() {
    let (state, mut rx) = make_state(None);
    let body = fixture("pr_opened.json");

    let status = handle_pull_request_event(&state, &body).await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED);

    let dispatch = rx.try_recv().expect("expected a dispatch on the channel");
    match dispatch {
        WebhookDispatch::ReviewPr {
            pr_number,
            project,
            head_sha,
            author_login,
            title,
            diff_loc,
        } => {
            assert_eq!(pr_number, 42);
            assert_eq!(project, "terraphim-ai");
            assert_eq!(head_sha, "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2");
            assert_eq!(author_login, "alex");
            assert_eq!(title, "feat: add semantic search endpoint");
            assert_eq!(diff_loc, 180 + 12);
        }
        other => panic!(
            "unexpected dispatch variant: comment_id={}",
            other.comment_id()
        ),
    }
}

#[tokio::test]
async fn webhook_pr_draft_skipped() {
    let (state, mut rx) = make_state(None);
    let body = fixture("pr_draft.json");

    let status = handle_pull_request_event(&state, &body).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(
        rx.try_recv().is_err(),
        "draft PR should not enqueue a dispatch"
    );
}

#[tokio::test]
async fn webhook_pr_action_other_than_opened_synchronize_skipped() {
    let (state, mut rx) = make_state(None);
    let body = fixture("pr_closed.json");

    let status = handle_pull_request_event(&state, &body).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(
        rx.try_recv().is_err(),
        "closed PR action should not enqueue a dispatch"
    );
}

#[tokio::test]
async fn webhook_pr_hmac_invalid_rejected() {
    // HMAC validation happens at the outer handler level before routing by
    // event type. We test it via verify_signature directly with a bad signature.
    let secret = "my-webhook-secret";
    let body = b"some payload";
    assert!(!verify_signature(secret, body, "sha256=deadbeef0000"));
    assert!(!verify_signature(secret, body, "invalid"));
}

#[tokio::test]
async fn webhook_pr_malformed_payload_logs_returns_200() {
    let (state, mut rx) = make_state(None);
    let bad_body = b"{ not valid json at all :::";

    let status = handle_pull_request_event(&state, bad_body).await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "malformed payload must return 200 to avoid Gitea retry spam"
    );
    assert!(
        rx.try_recv().is_err(),
        "malformed payload must not enqueue anything"
    );
}

// === Phase 3: Push event tests (Refs #929) ===

#[tokio::test]
async fn push_event_parses_correct_shape() {
    let (state, mut rx) = make_state(None);
    let body = fixture("push_main.json");

    let status = handle_push_event(&state, &body).await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED);

    let dispatch = rx.try_recv().expect("expected a Push dispatch");
    match dispatch {
        WebhookDispatch::Push {
            project,
            ref_name,
            before_sha,
            after_sha,
            pusher_login,
            files_changed: _,
        } => {
            assert_eq!(project, "terraphim-ai");
            assert_eq!(ref_name, "refs/heads/main");
            assert_eq!(before_sha, "0000000000000000000000000000000000000001");
            assert_eq!(after_sha, "abc123def456abc123def456abc123def456abcd");
            assert_eq!(pusher_login, "alex");
        }
        other => panic!(
            "unexpected dispatch variant: comment_id={}",
            other.comment_id()
        ),
    }
}

#[tokio::test]
async fn push_event_files_changed_aggregated_from_commits() {
    let (state, mut rx) = make_state(None);
    let body = fixture("push_main.json");

    let _ = handle_push_event(&state, &body).await;

    let dispatch = rx.try_recv().expect("expected a Push dispatch");
    match dispatch {
        WebhookDispatch::Push { files_changed, .. } => {
            // commit 1: modified [a.rs, b.rs]; commit 2: added [c.rs] modified [a.rs]
            // Union, dedup, insertion order: a.rs, b.rs, c.rs.
            assert_eq!(
                files_changed,
                vec!["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()],
            );
        }
        other => panic!(
            "unexpected dispatch variant: comment_id={}",
            other.comment_id()
        ),
    }
}

#[tokio::test]
async fn push_event_zero_commits_yields_empty_files_changed() {
    let (state, mut rx) = make_state(None);
    let body = fixture("push_tag_zero_commits.json");

    let status = handle_push_event(&state, &body).await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED);

    let dispatch = rx.try_recv().expect("expected a Push dispatch");
    match dispatch {
        WebhookDispatch::Push {
            ref_name,
            files_changed,
            ..
        } => {
            assert_eq!(ref_name, "refs/tags/v1.0.0");
            assert!(
                files_changed.is_empty(),
                "tag push with zero commits must yield empty files_changed"
            );
        }
        other => panic!(
            "unexpected dispatch variant: comment_id={}",
            other.comment_id()
        ),
    }
}

#[tokio::test]
async fn push_event_rejected_without_hmac() {
    // Push events are routed through the same outer HMAC gate as every other
    // Gitea webhook event; verify that a wrong/forged signature on the
    // canonical push payload would not validate. (handle_push_event is the
    // post-HMAC inner function — the gate lives in handle_gitea_webhook,
    // exercised here via verify_signature with the same body bytes.)
    let secret = "phase3-push-secret";
    let body = fixture("push_main.json");
    assert!(!verify_signature(secret, &body, "sha256=deadbeef0000"));
    assert!(!verify_signature(secret, &body, "not-hex"));
    assert!(!verify_signature(secret, &body, ""));
}

#[tokio::test]
async fn push_event_malformed_payload_returns_200() {
    let (state, mut rx) = make_state(None);
    let bad_body = b"{ malformed push :::";

    let status = handle_push_event(&state, bad_body).await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "malformed push payload must return 200 to avoid Gitea retry spam"
    );
    assert!(rx.try_recv().is_err());
}
