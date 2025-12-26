use anyhow::Result;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{Level, error, info};

mod config;
mod github;
mod webhook;
mod workflow;

use config::Settings;
use github::post_pr_comment;
use webhook::verify_signature;
use workflow::discover_workflows_for_event;

/// GitHub webhook payload structure
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GitHubWebhook {
    #[serde(default)]
    action: String,
    #[serde(default)]
    number: i64,
    #[serde(rename = "ref")]
    git_ref: Option<String>,
    pull_request: Option<PullRequestDetails>,
    repository: Option<Repository>,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct PullRequestDetails {
    title: String,
    html_url: String,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Repository {
    full_name: String,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct WebhookResponse {
    message: String,
    status: String,
}

/// Execute workflows for a GitHub event
async fn execute_workflows_for_event(
    webhook: &GitHubWebhook,
    settings: &Settings,
) -> Result<String> {
    use terraphim_github_runner::{GitHubEvent, GitHubEventType, RepositoryInfo};

    // Determine event type
    let event_type = if !webhook.action.is_empty() {
        "pull_request"
    } else if webhook.git_ref.is_some() {
        "push"
    } else {
        return Ok(format!(
            "Event type not supported: action={}",
            webhook.action
        ));
    };

    let branch = webhook
        .git_ref
        .as_ref()
        .and_then(|r| r.strip_prefix("refs/heads/"));

    info!("Processing {} event for branch: {:?}", event_type, branch);

    // Discover relevant workflows
    let workflows =
        discover_workflows_for_event(&settings.workflow_dir, event_type, branch).await?;

    if workflows.is_empty() {
        return Ok("No workflows found for this event".to_string());
    }

    info!("Found {} workflow(s) to execute", workflows.len());

    // Convert GitHub webhook to terraphim_github_runner event format
    // TODO: Actually execute workflows using gh_event
    let _gh_event = GitHubEvent {
        event_type: match event_type {
            "pull_request" => GitHubEventType::PullRequest,
            "push" => GitHubEventType::Push,
            _ => GitHubEventType::Unknown(event_type.to_string()),
        },
        action: if webhook.action.is_empty() {
            None
        } else {
            Some(webhook.action.clone())
        },
        repository: webhook
            .repository
            .as_ref()
            .map(|repo| RepositoryInfo {
                full_name: repo.full_name.clone(),
                clone_url: None,
                default_branch: None,
            })
            .unwrap_or_else(|| RepositoryInfo {
                full_name: String::new(),
                clone_url: None,
                default_branch: None,
            }),
        pull_request: webhook.pull_request.as_ref().map(|pr| {
            terraphim_github_runner::PullRequestInfo {
                title: pr.title.clone(),
                html_url: pr.html_url.clone(),
                number: webhook.number as u64,
                head_branch: None, // Not available in webhook payload
                base_branch: None, // Not available in webhook payload
            }
        }),
        git_ref: webhook.git_ref.clone(),
        sha: None, // Not in webhook payload
        extra: std::collections::HashMap::new(),
    };

    // TODO: Execute workflows using terraphim_github_runner
    // For now, just list what we found
    let mut results = vec![];
    for workflow_path in &workflows {
        let workflow_name = workflow_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        results.push(format!("- {} (would execute)", workflow_name));
    }

    if results.is_empty() {
        Ok("No workflows executed".to_string())
    } else {
        Ok(format!("Workflows processed:\n{}", results.join("\n")))
    }
}

/// Handle incoming webhook requests
#[handler]
async fn handle_webhook(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    // Load settings
    let settings = match Settings::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to load settings: {}", e);
            return Err(StatusError::internal_server_error());
        }
    };

    // Verify signature
    let signature = match req
        .headers()
        .get("x-hub-signature-256")
        .and_then(|h| h.to_str().ok())
    {
        Some(sig) => sig.to_string(),
        None => {
            error!("Missing X-Hub-Signature-256 header");
            return Err(StatusError::bad_request());
        }
    };

    let body = match req.payload().await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return Err(StatusError::bad_request());
        }
    };

    match verify_signature(&settings.github_webhook_secret, &signature, body).await {
        Ok(true) => (),
        Ok(false) => {
            error!("Invalid webhook signature");
            return Err(StatusError::forbidden());
        }
        Err(e) => {
            error!("Signature verification error: {}", e);
            return Err(StatusError::internal_server_error());
        }
    }

    // Parse webhook payload
    let webhook: GitHubWebhook = match serde_json::from_slice(body) {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to parse webhook payload: {}", e);
            return Err(StatusError::bad_request());
        }
    };

    info!(
        "Received webhook: action={}, number={}",
        webhook.action, webhook.number
    );

    // Handle pull_request events
    if webhook.action == "opened" || webhook.action == "synchronize" {
        let pr_number = webhook.number;
        let pr_title = webhook
            .pull_request
            .as_ref()
            .map(|pr| pr.title.clone())
            .unwrap_or_default();
        let pr_url = webhook
            .pull_request
            .as_ref()
            .map(|pr| pr.html_url.clone())
            .unwrap_or_default();
        let _repo_full_name = webhook
            .repository
            .as_ref()
            .map(|repo| repo.full_name.clone())
            .unwrap_or_default();

        // Spawn background task for workflow execution
        let settings_clone = settings.clone();
        let webhook_clone = webhook.clone();
        tokio::spawn(async move {
            match execute_workflows_for_event(&webhook_clone, &settings_clone).await {
                Ok(output) => {
                    let comment = format!(
                        "## GitHub Runner Execution Results\n\n**PR**: #{} - {}\n**URL**: {}\n\n{}\n\n✅ _Powered by terraphim-github-runner_",
                        pr_number, pr_title, pr_url, output
                    );

                    if !_repo_full_name.is_empty() {
                        if let Err(e) =
                            post_pr_comment(&_repo_full_name, pr_number as u64, &comment).await
                        {
                            error!("Failed to post comment: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Workflow execution failed: {}", e);

                    if !_repo_full_name.is_empty() {
                        let error_comment = format!(
                            "## ❌ GitHub Runner Execution Failed\n\n**PR**: #{}\n\n```\n{}\n```",
                            pr_number, e
                        );
                        if let Err(e) =
                            post_pr_comment(&_repo_full_name, pr_number as u64, &error_comment)
                                .await
                        {
                            error!("Failed to post error comment: {}", e);
                        }
                    }
                }
            }
        });

        // Return immediately
        let response = WebhookResponse {
            message: "Pull request webhook received and workflow execution started".to_string(),
            status: "success".to_string(),
        };
        res.render(Json(response));
    }
    // Handle push events
    else if webhook.action.is_empty() && webhook.git_ref.is_some() {
        let _repo_full_name = webhook
            .repository
            .as_ref()
            .map(|repo| repo.full_name.clone())
            .unwrap_or_default();
        let git_ref = webhook.git_ref.clone().unwrap_or_default();

        // Spawn background task for workflow execution
        let settings_clone = settings.clone();
        let webhook_clone = webhook.clone();
        tokio::spawn(async move {
            match execute_workflows_for_event(&webhook_clone, &settings_clone).await {
                Ok(output) => {
                    info!("Push workflow execution completed:\n{}", output);
                }
                Err(e) => {
                    error!("Push workflow execution failed: {}", e);
                }
            }
        });

        let response = WebhookResponse {
            message: format!("Push webhook received for {}", git_ref),
            status: "success".to_string(),
        };
        res.render(Json(response));
    }
    // Other events - just acknowledge
    else {
        let response = WebhookResponse {
            message: format!("Webhook received (action={})", webhook.action),
            status: "acknowledged".to_string(),
        };
        res.render(Json(response));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Load configuration
    let settings = Settings::from_env()?;
    info!("Configuration loaded successfully");
    info!("Repository path: {:?}", settings.repository_path);
    info!("Workflow directory: {:?}", settings.workflow_dir);

    // Setup router
    let router = Router::new().push(Router::with_path("webhook").post(handle_webhook));

    let addr = format!("{}:{}", settings.host, settings.port);
    info!("Terraphim GitHub Runner Server starting on {}", addr);

    let acceptor = TcpListener::new(&addr).bind().await;
    Server::new(acceptor).serve(router).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use salvo::test::TestClient;

    fn create_test_settings() -> Settings {
        use std::path::PathBuf;
        Settings {
            port: 3000,
            host: "127.0.0.1".to_string(),
            github_webhook_secret: "test_secret".to_string(),
            github_token: None,
            firecracker_api_url: "http://127.0.0.1:8080".to_string(),
            firecracker_auth_token: String::new(),
            repository_path: PathBuf::from("."),
            workflow_dir: PathBuf::from(".github/workflows"),
        }
    }

    #[tokio::test]
    async fn test_valid_webhook_signature() {
        unsafe {
            std::env::set_var("GITHUB_WEBHOOK_SECRET", "test_secret");
        }
        let settings = create_test_settings();
        let payload = r#"{"action":"opened","number":1,"repository":{"full_name":"test/repo"}}"#;

        // Generate valid signature
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut mac =
            Hmac::<Sha256>::new_from_slice(settings.github_webhook_secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        let service =
            Service::new(Router::new().push(Router::with_path("webhook").post(handle_webhook)));
        let resp = TestClient::post("http://127.0.0.1:5800/webhook")
            .add_header("content-type", "application/json", false)
            .add_header("x-hub-signature-256", signature, false)
            .body(payload)
            .send(&service)
            .await;

        assert_eq!(resp.status_code, Some(salvo::http::StatusCode::OK));
        unsafe {
            std::env::remove_var("GITHUB_WEBHOOK_SECRET");
        }
    }

    #[tokio::test]
    async fn test_invalid_webhook_signature() {
        unsafe {
            std::env::set_var("GITHUB_WEBHOOK_SECRET", "test_secret");
        }
        let payload = r#"{"action":"opened","number":1,"repository":{"full_name":"test/repo"}}"#;

        let service =
            Service::new(Router::new().push(Router::with_path("webhook").post(handle_webhook)));
        let resp = TestClient::post("http://127.0.0.1:5800/webhook")
            .add_header("content-type", "application/json", false)
            .add_header("x-hub-signature-256", "sha256=invalid", false)
            .body(payload)
            .send(&service)
            .await;

        assert_eq!(resp.status_code, Some(salvo::http::StatusCode::FORBIDDEN));
        unsafe {
            std::env::remove_var("GITHUB_WEBHOOK_SECRET");
        }
    }
}
