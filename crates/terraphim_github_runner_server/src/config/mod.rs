use anyhow::Result;
use std::env;
use std::path::PathBuf;

/// Configuration for the GitHub runner server
#[derive(Debug, Clone)]
pub struct Settings {
    /// Server port (default: 3000)
    pub port: u16,

    /// Server host (default: 127.0.0.1)
    pub host: String,

    /// GitHub webhook secret for signature verification
    pub github_webhook_secret: String,

    /// GitHub token for API calls (octocrab)
    #[allow(dead_code)]
    pub github_token: Option<String>,

    /// Firecracker API URL
    #[allow(dead_code)]
    pub firecracker_api_url: String,

    /// Firecracker auth token
    #[allow(dead_code)]
    pub firecracker_auth_token: String,

    /// Repository path (default: current directory)
    pub repository_path: PathBuf,

    /// Workflow directory (default: .github/workflows)
    pub workflow_dir: PathBuf,
}

impl Settings {
    /// Load settings from environment variables
    pub fn from_env() -> Result<Self> {
        let repository_path = env::var("REPOSITORY_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        Ok(Settings {
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            github_webhook_secret: env::var("GITHUB_WEBHOOK_SECRET")?,
            github_token: env::var("GITHUB_TOKEN").ok(),
            firecracker_api_url: env::var("FIRECRACKER_API_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string()),
            firecracker_auth_token: env::var("FIRECRACKER_AUTH_TOKEN")
                .unwrap_or_else(|_| String::new()),
            repository_path: repository_path.clone(),
            workflow_dir: repository_path.join(".github/workflows"),
        })
    }
}
