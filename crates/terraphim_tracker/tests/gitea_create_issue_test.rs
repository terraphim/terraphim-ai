//! Integration tests for Gitea issue creation API
//!
//! These tests verify the create_issue functionality against our Gitea fork.
//! Run with: cargo test --test gitea_create_issue_test -- --ignored
//! (marked as ignored to avoid running in CI without proper credentials)

use std::env;

/// Test creating an issue with various label formats
/// This helps debug the 422 Unprocessable Entity error
#[tokio::test]
#[ignore = "requires live Gitea instance"]
async fn test_create_issue_with_labels() {
    let base_url = env::var("GITEA_URL").expect("GITEA_URL not set");
    let token = env::var("GITEA_TOKEN").expect("GITEA_TOKEN not set");
    let owner = env::var("GITEA_OWNER").unwrap_or_else(|_| "terraphim".to_string());
    let repo = env::var("GITEA_REPO").unwrap_or_else(|_| "test-repo".to_string());

    let client = reqwest::Client::new();

    // Test 1: Empty labels array
    println!("Test 1: Empty labels array");
    let resp = client
        .post(format!(
            "{}/api/v1/repos/{}/{}/issues",
            base_url, owner, repo
        ))
        .header("Authorization", format!("token {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "title": "Test issue - empty labels",
            "body": "Testing empty labels array",
            "labels": [],
        }))
        .send()
        .await
        .expect("Failed to send request");

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    println!("  Status: {}", status);
    println!("  Body: {}", body);

    // Test 2: String labels array
    println!("\nTest 2: String labels array");
    let resp = client
        .post(format!(
            "{}/api/v1/repos/{}/{}/issues",
            base_url, owner, repo
        ))
        .header("Authorization", format!("token {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "title": "Test issue - string labels",
            "body": "Testing string labels array",
            "labels": ["bug", "critical"],
        }))
        .send()
        .await
        .expect("Failed to send request");

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    println!("  Status: {}", status);
    println!("  Body: {}", body);

    // Test 3: No labels field
    println!("\nTest 3: No labels field");
    let resp = client
        .post(format!(
            "{}/api/v1/repos/{}/{}/issues",
            base_url, owner, repo
        ))
        .header("Authorization", format!("token {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "title": "Test issue - no labels",
            "body": "Testing without labels field",
        }))
        .send()
        .await
        .expect("Failed to send request");

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    println!("  Status: {}", status);
    println!("  Body: {}", body);

    // Test 4: Integer labels (to see error)
    println!("\nTest 4: Integer labels (expected to fail)");
    let resp = client
        .post(format!(
            "{}/api/v1/repos/{}/{}/issues",
            base_url, owner, repo
        ))
        .header("Authorization", format!("token {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "title": "Test issue - int labels",
            "body": "Testing integer labels",
            "labels": [1, 2, 3],
        }))
        .send()
        .await
        .expect("Failed to send request");

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    println!("  Status: {}", status);
    println!("  Body: {}", body);
}

/// Test the actual GiteaTracker::create_issue method
#[tokio::test]
#[ignore = "requires live Gitea instance"]
async fn test_tracker_create_issue() {
    use terraphim_tracker::gitea::{GiteaConfig, GiteaTracker};

    let base_url = env::var("GITEA_URL").expect("GITEA_URL not set");
    let token = env::var("GITEA_TOKEN").expect("GITEA_TOKEN not set");
    let owner = env::var("GITEA_OWNER").unwrap_or_else(|_| "terraphim".to_string());
    let repo = env::var("GITEA_REPO").unwrap_or_else(|_| "test-repo".to_string());

    let config = GiteaConfig {
        base_url,
        token,
        owner,
        repo,
        active_states: vec!["open".to_string()],
        terminal_states: vec!["closed".to_string()],
        use_robot_api: false,
    };

    let tracker = GiteaTracker::new(config).expect("Failed to create tracker");

    // Test with string labels
    println!("Testing create_issue with string labels...");
    let result = tracker
        .create_issue(
            "Test from tracker - string labels",
            "This is a test issue created by GiteaTracker",
            &["test", "automation"],
        )
        .await;

    match result {
        Ok(issue) => println!(
            "✅ Success! Created issue #{}: {}",
            issue.number, issue.title
        ),
        Err(e) => println!("❌ Failed: {}", e),
    }

    // Test with empty labels
    println!("\nTesting create_issue with empty labels...");
    let result = tracker
        .create_issue(
            "Test from tracker - empty labels",
            "This is a test issue with no labels",
            &[],
        )
        .await;

    match result {
        Ok(issue) => println!(
            "✅ Success! Created issue #{}: {}",
            issue.number, issue.title
        ),
        Err(e) => println!("❌ Failed: {}", e),
    }
}
