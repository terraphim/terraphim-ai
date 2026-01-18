use anyhow::Result;
use octocrab::Octocrab;
use tracing::info;

/// Post a comment to a GitHub pull request
///
/// # Arguments
/// * `repo_full_name` - Repository in format "owner/repo"
/// * `pr_number` - Pull request number
/// * `comment` - Comment body text
///
/// # Returns
/// * `Ok(())` if comment posted successfully
/// * `Err` if posting fails
pub async fn post_pr_comment(repo_full_name: &str, pr_number: u64, comment: &str) -> Result<()> {
    let github_token = match std::env::var("GITHUB_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            info!("GITHUB_TOKEN not set, skipping comment posting");
            return Ok(());
        }
    };

    let (repo_owner, repo_name) = repo_full_name.split_once('/').ok_or_else(|| {
        anyhow::anyhow!("Invalid repository full name format: {}", repo_full_name)
    })?;

    let octocrab = Octocrab::builder()
        .personal_token(github_token)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create GitHub client: {}", e))?;

    octocrab
        .issues(repo_owner, repo_name)
        .create_comment(pr_number, comment)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to post comment: {}", e))?;

    info!("Successfully posted comment to PR #{}", pr_number);
    Ok(())
}
