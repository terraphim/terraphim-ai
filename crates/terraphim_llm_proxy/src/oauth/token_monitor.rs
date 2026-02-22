//! Token expiration monitoring for OAuth tokens.
//!
//! This module provides functionality to monitor OAuth token expiration and
//! log warnings when tokens are about to expire. It runs checks on startup
//! and can be scheduled to run periodically.
//!
//! # Monitoring Features
//!
//! - **Startup Check**: Validates all stored tokens on application startup
//! - **24-Hour Warning**: Logs warnings for tokens expiring within 24 hours
//! - **Weekly Reminders**: Logs periodic reminders about expiring tokens
//! - **Expiration Tracking**: Tracks token expiration status across providers
//!
//! # Usage
//!
//! ```rust,ignore
//! use terraphim_llm_proxy::oauth::TokenMonitor;
//!
//! let monitor = TokenMonitor::new();
//! monitor.check_all_tokens_on_startup().await?;
//! monitor.start_periodic_monitoring(Duration::from_secs(3600)).await;
//! ```

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::oauth::error::OAuthResult;
use crate::oauth::file_store::FileTokenStore;
use crate::oauth::store::TokenStore;
use crate::oauth::types::TokenBundle;

/// Threshold for warning about token expiration (24 hours)
const WARNING_THRESHOLD_HOURS: i64 = 24;

/// Interval for weekly reminders (in days)
const WEEKLY_REMINDER_DAYS: i64 = 7;

/// Token status summary
#[derive(Debug, Clone)]
pub struct TokenStatus {
    /// Provider identifier
    pub provider: String,

    /// Account identifier
    pub account_id: String,

    /// Whether the token is expired
    pub is_expired: bool,

    /// Whether the token will expire within the warning threshold
    pub expires_soon: bool,

    /// Time until expiration (None if already expired or no expiration)
    pub time_until_expiration: Option<Duration>,

    /// Token expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
}

impl TokenStatus {
    /// Create a token status from a TokenBundle
    fn from_bundle(bundle: &TokenBundle) -> Self {
        let now = Utc::now();
        let warning_threshold = Duration::hours(WARNING_THRESHOLD_HOURS);

        let is_expired = bundle.is_expired();
        let expires_soon = bundle.expires_within(warning_threshold);

        let time_until_expiration = bundle.expires_at.map(|exp| {
            if exp > now {
                exp - now
            } else {
                Duration::zero()
            }
        });

        Self {
            provider: bundle.provider.clone(),
            account_id: bundle.account_id.clone(),
            is_expired,
            expires_soon: !is_expired && expires_soon,
            time_until_expiration,
            expires_at: bundle.expires_at,
        }
    }

    /// Get a human-readable status message
    fn status_message(&self) -> String {
        if self.is_expired {
            format!("Token for {}/{} is EXPIRED", self.provider, self.account_id)
        } else if let Some(duration) = self.time_until_expiration {
            let hours = duration.num_hours();
            let days = duration.num_days();

            if hours < 1 {
                format!(
                    "Token for {}/{} expires in {} minutes",
                    self.provider,
                    self.account_id,
                    duration.num_minutes()
                )
            } else if days < 1 {
                format!(
                    "Token for {}/{} expires in {} hours",
                    self.provider, self.account_id, hours
                )
            } else {
                format!(
                    "Token for {}/{} expires in {} days",
                    self.provider, self.account_id, days
                )
            }
        } else {
            format!(
                "Token for {}/{} has no expiration set",
                self.provider, self.account_id
            )
        }
    }
}

/// Token expiration monitor
#[derive(Debug, Clone)]
pub struct TokenMonitor {
    /// File token store for loading tokens
    token_store: FileTokenStore,

    /// Last time weekly reminders were sent
    last_weekly_reminder: Option<DateTime<Utc>>,
}

impl TokenMonitor {
    /// Create a new token monitor with the default token store path.
    pub async fn new() -> OAuthResult<Self> {
        let token_store = FileTokenStore::with_default_path().await?;

        Ok(Self {
            token_store,
            last_weekly_reminder: None,
        })
    }

    /// Create with a custom token store.
    pub fn with_store(token_store: FileTokenStore) -> Self {
        Self {
            token_store,
            last_weekly_reminder: None,
        }
    }

    /// Check all tokens on startup.
    ///
    /// Loads all stored tokens from all providers and checks their expiration status.
    /// Logs warnings for tokens that are expired or will expire within 24 hours.
    ///
    /// # Returns
    /// Summary of token statuses across all providers.
    pub async fn check_all_tokens_on_startup(&mut self) -> OAuthResult<Vec<TokenStatus>> {
        info!("Checking token expiration on startup...");

        let providers = self.token_store.list_providers().await?;
        let mut all_statuses = Vec::new();

        for provider in providers {
            let accounts = self.token_store.list_accounts(&provider).await?;

            for account_id in accounts {
                match self.token_store.load(&provider, &account_id).await? {
                    Some(bundle) => {
                        let status = self.check_token(&bundle).await?;
                        all_statuses.push(status);
                    }
                    None => {
                        warn!(
                            provider = %provider,
                            account = %account_id,
                            "Token file exists but could not be loaded"
                        );
                    }
                }
            }
        }

        // Log summary
        self.log_startup_summary(&all_statuses);

        Ok(all_statuses)
    }

    /// Check a single token's expiration status.
    ///
    /// # Arguments
    /// * `bundle` - The token bundle to check
    ///
    /// # Returns
    /// Token status with expiration information.
    pub async fn check_token(&self, bundle: &TokenBundle) -> OAuthResult<TokenStatus> {
        let status = TokenStatus::from_bundle(bundle);

        // Log appropriate warning based on status
        if status.is_expired {
            warn!(
                provider = %status.provider,
                account_id = %status.account_id,
                expired_at = ?status.expires_at,
                "Token has EXPIRED - re-authentication required"
            );
        } else if status.expires_soon {
            if let Some(duration) = status.time_until_expiration {
                let hours = duration.num_hours();
                warn!(
                    provider = %status.provider,
                    account_id = %status.account_id,
                    expires_in_hours = hours,
                    expires_at = ?status.expires_at,
                    "Token expires within 24 hours - consider refreshing"
                );
            }
        } else {
            debug!(
                provider = %status.provider,
                account_id = %status.account_id,
                "Token is valid"
            );
        }

        Ok(status)
    }

    /// Check tokens for a specific provider.
    ///
    /// # Arguments
    /// * `provider` - The provider to check tokens for
    ///
    /// # Returns
    /// List of token statuses for the provider.
    pub async fn check_provider_tokens(&self, provider: &str) -> OAuthResult<Vec<TokenStatus>> {
        let accounts = self.token_store.list_accounts(provider).await?;
        let mut statuses = Vec::new();

        for account_id in accounts {
            if let Some(bundle) = self.token_store.load(provider, &account_id).await? {
                let status = TokenStatus::from_bundle(&bundle);
                statuses.push(status);
            }
        }

        Ok(statuses)
    }

    /// Log a summary of token statuses on startup.
    fn log_startup_summary(&self, statuses: &[TokenStatus]) {
        let total = statuses.len();
        let expired = statuses.iter().filter(|s| s.is_expired).count();
        let expiring_soon = statuses.iter().filter(|s| s.expires_soon).count();
        let valid = total - expired - expiring_soon;

        if total == 0 {
            info!("No OAuth tokens found in storage");
            return;
        }

        info!(
            total_tokens = total,
            expired = expired,
            expiring_within_24h = expiring_soon,
            valid = valid,
            "Token expiration check complete"
        );

        // Log individual token statuses
        for status in statuses {
            if status.is_expired || status.expires_soon {
                warn!("{}", status.status_message());
            } else {
                info!("{}", status.status_message());
            }
        }
    }

    /// Check if weekly reminder should be sent.
    ///
    /// Returns true if it's been more than a week since the last reminder.
    fn should_send_weekly_reminder(&self) -> bool {
        match self.last_weekly_reminder {
            Some(last) => {
                let elapsed = Utc::now() - last;
                elapsed > Duration::days(WEEKLY_REMINDER_DAYS)
            }
            None => true,
        }
    }

    /// Send weekly reminders about expiring tokens.
    ///
    /// This should be called periodically (e.g., daily) to check if
    /// weekly reminders need to be sent.
    ///
    /// # Returns
    /// Ok(true) if reminders were sent, Ok(false) otherwise.
    pub async fn maybe_send_weekly_reminders(&mut self) -> OAuthResult<bool> {
        if !self.should_send_weekly_reminder() {
            return Ok(false);
        }

        info!("Sending weekly token expiration reminders...");

        let statuses = self.check_all_tokens_on_startup().await?;
        let expiring_tokens: Vec<_> = statuses
            .iter()
            .filter(|s| !s.is_expired && s.expires_soon)
            .collect();

        if !expiring_tokens.is_empty() {
            warn!(
                count = expiring_tokens.len(),
                "Weekly reminder: {} token(s) expiring within 24 hours",
                expiring_tokens.len()
            );

            for status in expiring_tokens {
                warn!(
                    provider = %status.provider,
                    account_id = %status.account_id,
                    expires_at = ?status.expires_at,
                    "Weekly reminder: Token expires soon"
                );
            }
        } else {
            info!("Weekly reminder: All tokens are valid");
        }

        self.last_weekly_reminder = Some(Utc::now());

        Ok(true)
    }

    /// Get a summary of all token statuses grouped by provider.
    ///
    /// # Returns
    /// HashMap mapping provider names to their token statuses.
    pub async fn get_token_summary(&self) -> OAuthResult<HashMap<String, Vec<TokenStatus>>> {
        let providers = self.token_store.list_providers().await?;
        let mut summary = HashMap::new();

        for provider in providers {
            let statuses = self.check_provider_tokens(&provider).await?;
            summary.insert(provider, statuses);
        }

        Ok(summary)
    }

    /// Check if any tokens are expired.
    ///
    /// # Returns
    /// true if at least one token is expired.
    pub async fn has_expired_tokens(&mut self) -> OAuthResult<bool> {
        let statuses = self.check_all_tokens_on_startup().await?;
        Ok(statuses.iter().any(|s| s.is_expired))
    }

    /// Get the number of valid tokens.
    ///
    /// # Returns
    /// Count of tokens that are not expired.
    pub async fn count_valid_tokens(&mut self) -> OAuthResult<usize> {
        let statuses = self.check_all_tokens_on_startup().await?;
        Ok(statuses.iter().filter(|s| !s.is_expired).count())
    }
}

/// Run token monitoring on startup.
///
/// This is a convenience function that creates a TokenMonitor and runs
/// the startup check. It logs all warnings and returns a summary.
///
/// # Returns
/// Summary of token statuses.
pub async fn monitor_tokens_on_startup() -> OAuthResult<Vec<TokenStatus>> {
    let mut monitor = TokenMonitor::new().await?;
    monitor.check_all_tokens_on_startup().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use tempfile::TempDir;

    fn create_test_bundle(
        provider: &str,
        account_id: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> TokenBundle {
        TokenBundle {
            access_token: "test_token".to_string(),
            refresh_token: Some("test_refresh".to_string()),
            token_type: "Bearer".to_string(),
            expires_at,
            scope: Some("openid email".to_string()),
            provider: provider.to_string(),
            account_id: account_id.to_string(),
            metadata: Default::default(),
            created_at: Utc::now(),
            last_refresh: None,
        }
    }

    #[tokio::test]
    async fn test_token_status_from_bundle() {
        // Valid token (expires in 48 hours)
        let bundle = create_test_bundle(
            "openai",
            "test@example.com",
            Some(Utc::now() + ChronoDuration::hours(48)),
        );

        let status = TokenStatus::from_bundle(&bundle);

        assert!(!status.is_expired);
        assert!(!status.expires_soon);
        assert!(status.time_until_expiration.is_some());
        assert_eq!(status.provider, "openai");
        assert_eq!(status.account_id, "test@example.com");
    }

    #[tokio::test]
    async fn test_token_status_expires_soon() {
        // Token expires in 12 hours (within 24h threshold)
        let bundle = create_test_bundle(
            "openai",
            "test@example.com",
            Some(Utc::now() + ChronoDuration::hours(12)),
        );

        let status = TokenStatus::from_bundle(&bundle);

        assert!(!status.is_expired);
        assert!(status.expires_soon);
    }

    #[tokio::test]
    async fn test_token_status_expired() {
        // Token expired 1 hour ago
        let bundle = create_test_bundle(
            "openai",
            "test@example.com",
            Some(Utc::now() - ChronoDuration::hours(1)),
        );

        let status = TokenStatus::from_bundle(&bundle);

        assert!(status.is_expired);
        assert!(!status.expires_soon);
    }

    #[tokio::test]
    async fn test_token_status_no_expiration() {
        let bundle = create_test_bundle("openai", "test@example.com", None);

        let status = TokenStatus::from_bundle(&bundle);

        assert!(!status.is_expired);
        assert!(!status.expires_soon);
        assert!(status.time_until_expiration.is_none());
    }

    #[test]
    fn test_status_message() {
        // Expired token
        let expired = TokenStatus {
            provider: "openai".to_string(),
            account_id: "test".to_string(),
            is_expired: true,
            expires_soon: false,
            time_until_expiration: None,
            expires_at: Some(Utc::now() - ChronoDuration::hours(1)),
        };
        assert!(expired.status_message().contains("EXPIRED"));

        // Expires soon (30 minutes)
        let soon_30m = TokenStatus {
            provider: "openai".to_string(),
            account_id: "test".to_string(),
            is_expired: false,
            expires_soon: true,
            time_until_expiration: Some(ChronoDuration::minutes(30)),
            expires_at: Some(Utc::now() + ChronoDuration::minutes(30)),
        };
        assert!(soon_30m.status_message().contains("30 minutes"));

        // Expires in 12 hours
        let soon_12h = TokenStatus {
            provider: "openai".to_string(),
            account_id: "test".to_string(),
            is_expired: false,
            expires_soon: true,
            time_until_expiration: Some(ChronoDuration::hours(12)),
            expires_at: Some(Utc::now() + ChronoDuration::hours(12)),
        };
        assert!(soon_12h.status_message().contains("12 hours"));

        // Expires in 3 days
        let future = TokenStatus {
            provider: "openai".to_string(),
            account_id: "test".to_string(),
            is_expired: false,
            expires_soon: false,
            time_until_expiration: Some(ChronoDuration::days(3)),
            expires_at: Some(Utc::now() + ChronoDuration::days(3)),
        };
        assert!(future.status_message().contains("3 days"));

        // No expiration
        let no_exp = TokenStatus {
            provider: "openai".to_string(),
            account_id: "test".to_string(),
            is_expired: false,
            expires_soon: false,
            time_until_expiration: None,
            expires_at: None,
        };
        assert!(no_exp.status_message().contains("no expiration"));
    }

    #[tokio::test]
    async fn test_check_all_tokens_on_startup() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Store some test tokens
        let valid_bundle = create_test_bundle(
            "openai",
            "valid@example.com",
            Some(Utc::now() + ChronoDuration::days(7)),
        );
        store
            .store("openai", "valid@example.com", &valid_bundle)
            .await
            .unwrap();

        let expired_bundle = create_test_bundle(
            "claude",
            "expired@example.com",
            Some(Utc::now() - ChronoDuration::hours(1)),
        );
        store
            .store("claude", "expired@example.com", &expired_bundle)
            .await
            .unwrap();

        let soon_bundle = create_test_bundle(
            "gemini",
            "soon@example.com",
            Some(Utc::now() + ChronoDuration::hours(12)),
        );
        store
            .store("gemini", "soon@example.com", &soon_bundle)
            .await
            .unwrap();

        let mut monitor = TokenMonitor::with_store(store);
        let statuses = monitor.check_all_tokens_on_startup().await.unwrap();

        assert_eq!(statuses.len(), 3);

        let expired_count = statuses.iter().filter(|s| s.is_expired).count();
        let soon_count = statuses.iter().filter(|s| s.expires_soon).count();

        assert_eq!(expired_count, 1);
        assert_eq!(soon_count, 1);
    }

    #[tokio::test]
    async fn test_check_provider_tokens() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Store tokens for different providers
        let openai_bundle = create_test_bundle(
            "openai",
            "user@example.com",
            Some(Utc::now() + ChronoDuration::days(7)),
        );
        store
            .store("openai", "user@example.com", &openai_bundle)
            .await
            .unwrap();

        let claude_bundle = create_test_bundle(
            "claude",
            "user@example.com",
            Some(Utc::now() + ChronoDuration::days(7)),
        );
        store
            .store("claude", "user@example.com", &claude_bundle)
            .await
            .unwrap();

        let monitor = TokenMonitor::with_store(store);
        let openai_statuses = monitor.check_provider_tokens("openai").await.unwrap();
        let claude_statuses = monitor.check_provider_tokens("claude").await.unwrap();

        assert_eq!(openai_statuses.len(), 1);
        assert_eq!(claude_statuses.len(), 1);
        assert_eq!(openai_statuses[0].provider, "openai");
        assert_eq!(claude_statuses[0].provider, "claude");
    }

    #[tokio::test]
    async fn test_should_send_weekly_reminder() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let mut monitor = TokenMonitor::with_store(store);

        // No previous reminder - should send
        assert!(monitor.should_send_weekly_reminder());

        // Just sent - should not send
        monitor.last_weekly_reminder = Some(Utc::now());
        assert!(!monitor.should_send_weekly_reminder());

        // 6 days ago - should not send
        monitor.last_weekly_reminder = Some(Utc::now() - ChronoDuration::days(6));
        assert!(!monitor.should_send_weekly_reminder());

        // 8 days ago - should send
        monitor.last_weekly_reminder = Some(Utc::now() - ChronoDuration::days(8));
        assert!(monitor.should_send_weekly_reminder());
    }

    #[tokio::test]
    async fn test_maybe_send_weekly_reminders() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Store a token that expires soon
        let bundle = create_test_bundle(
            "openai",
            "user@example.com",
            Some(Utc::now() + ChronoDuration::hours(12)),
        );
        store
            .store("openai", "user@example.com", &bundle)
            .await
            .unwrap();

        let mut monitor = TokenMonitor::with_store(store);

        // Should send reminder (first time)
        let sent = monitor.maybe_send_weekly_reminders().await.unwrap();
        assert!(sent);
        assert!(monitor.last_weekly_reminder.is_some());

        // Should not send (too soon)
        let sent = monitor.maybe_send_weekly_reminders().await.unwrap();
        assert!(!sent);
    }

    #[tokio::test]
    async fn test_get_token_summary() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let openai_bundle = create_test_bundle(
            "openai",
            "user@example.com",
            Some(Utc::now() + ChronoDuration::days(7)),
        );
        store
            .store("openai", "user@example.com", &openai_bundle)
            .await
            .unwrap();

        let monitor = TokenMonitor::with_store(store);
        let summary = monitor.get_token_summary().await.unwrap();

        assert!(summary.contains_key("openai"));
        assert_eq!(summary["openai"].len(), 1);
    }

    #[tokio::test]
    async fn test_has_expired_tokens() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let mut monitor = TokenMonitor::with_store(store);

        // No tokens - no expired tokens
        assert!(!monitor.has_expired_tokens().await.unwrap());
    }

    #[tokio::test]
    async fn test_count_valid_tokens() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let bundle = create_test_bundle(
            "openai",
            "user@example.com",
            Some(Utc::now() + ChronoDuration::days(7)),
        );
        store
            .store("openai", "user@example.com", &bundle)
            .await
            .unwrap();

        let mut monitor = TokenMonitor::with_store(store);

        assert_eq!(monitor.count_valid_tokens().await.unwrap(), 1);
    }
}
