//! Example: Agent with Gitea authentication
//!
//! This example demonstrates how agents authenticate to Gitea and log
//! their actions as issues.

use std::collections::HashMap;
use terraphim_spawner::{AgentStatus, GiteaAgentAuth};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Gitea authentication
    // In production, load from environment variables:
    // export GITEA_URL="https://git.terraphim.cloud"
    // export GITEA_TOKEN="your-token"
    // export GITEA_SPONSOR="your-username"
    // export GITEA_LOG_REPO="your-username/agent-logs"
    
    let auth = GiteaAgentAuth::new(
        "terraphim-agent-001",
        "alex",
        "https://git.terraphim.cloud",
        std::env::var("GITEA_TOKEN").unwrap_or_else(|_| "demo-token".to_string()),
        "alex/agent-logs",
    );

    println!("=== Gitea Agent Authentication Example ===\n");

    // Example 1: Verify authentication
    println!("1. Verifying Gitea authentication...");
    match auth.verify().await {
        Ok(user) => {
            println!("   ✓ Authenticated as: {} ({})", user.login, user.email);
        }
        Err(e) => {
            println!("   ✗ Authentication failed: {}", e);
            println!("   (This is expected with a demo token)");
        }
    }

    // Example 2: Log agent action
    println!("\n2. Logging agent action...");
    let mut metadata = HashMap::new();
    metadata.insert("task_id".to_string(), "task-123".to_string());
    metadata.insert("duration_ms".to_string(), "1500".to_string());
    
    match auth.log_action(
        "Code Review Completed",
        "Reviewed pull request #42\n\nFound 3 issues:\n1. Missing error handling\n2. Unused imports\n3. Documentation needed",
        Some(metadata),
    ).await {
        Ok(issue_number) => {
            println!("   ✓ Action logged as issue #{}", issue_number);
        }
        Err(e) => {
            println!("   ✗ Failed to log action: {}", e);
            println!("   (This is expected without a valid token)");
        }
    }

    // Example 3: Update agent status
    println!("\n3. Updating agent status...");
    match auth.update_status(
        AgentStatus::Running,
        "Processing code review tasks",
    ).await {
        Ok(()) => {
            println!("   ✓ Status updated");
        }
        Err(e) => {
            println!("   ✗ Failed to update status: {}", e);
        }
    }

    println!("\n=== Example Complete ===");
    println!("\nTo use with a real Gitea instance:");
    println!("  1. Create a Personal Access Token in Gitea");
    println!("  2. Set environment variables:");
    println!("     export GITEA_URL=\"https://your-gitea.com\"");
    println!("     export GITEA_TOKEN=\"your-token\"");
    println!("     export GITEA_SPONSOR=\"your-username\"");
    println!("  3. Run the example again");

    Ok(())
}
