use anyhow::Result;
use std::env;
use std::path::PathBuf;

/// Configuration for the GitHub runner server
#[derive(Clone)]
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

impl std::fmt::Debug for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Settings")
            .field("port", &self.port)
            .field("host", &self.host)
            .field("github_webhook_secret", &"***REDACTED***")
            .field(
                "github_token",
                &self.github_token.as_ref().map(|_| "***REDACTED***"),
            )
            .field("firecracker_api_url", &self.firecracker_api_url)
            .field("firecracker_auth_token", &"***REDACTED***")
            .field("repository_path", &self.repository_path)
            .field("workflow_dir", &self.workflow_dir)
            .finish()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_debug_redacts_all_secrets() {
        let s = Settings {
            port: 3000,
            host: "127.0.0.1".into(),
            github_webhook_secret: "webhook-secret-aaaa".into(),
            github_token: Some("github-token-bbbb".into()),
            firecracker_api_url: "http://127.0.0.1:8080".into(),
            firecracker_auth_token: "firecracker-token-cccc".into(),
            repository_path: PathBuf::from("/tmp/repo"),
            workflow_dir: PathBuf::from("/tmp/repo/.github/workflows"),
        };
        let out = format!("{:?}", s);
        assert!(!out.contains("webhook-secret-aaaa"));
        assert!(!out.contains("github-token-bbbb"));
        assert!(!out.contains("firecracker-token-cccc"));
        // Each secret field rendered as redacted
        assert!(out.matches("***REDACTED***").count() >= 3);
        // Non-secret fields render
        assert!(out.contains("127.0.0.1"));
    }

    #[test]
    fn settings_debug_redacts_none_github_token_safely() {
        let s = Settings {
            port: 3000,
            host: "127.0.0.1".into(),
            github_webhook_secret: "webhook-secret".into(),
            github_token: None,
            firecracker_api_url: "http://127.0.0.1:8080".into(),
            firecracker_auth_token: "fc-token".into(),
            repository_path: PathBuf::from("/tmp/repo"),
            workflow_dir: PathBuf::from("/tmp/repo/.github/workflows"),
        };
        let out = format!("{:?}", s);
        assert!(out.contains("None"));
        assert!(!out.contains("github-token"));
    }
    }
}
