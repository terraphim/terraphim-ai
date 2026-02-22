//! Token storage trait and utilities for OAuth tokens.

use async_trait::async_trait;

use crate::oauth::error::OAuthResult;
use crate::oauth::types::TokenBundle;

/// Token storage abstraction for persisting OAuth tokens.
///
/// Implementations can store tokens in files, Redis, or other backends.
#[async_trait]
pub trait TokenStore: Send + Sync {
    /// Store a token bundle for a provider and account.
    ///
    /// # Arguments
    /// * `provider` - Provider identifier (e.g., "claude")
    /// * `account_id` - Account identifier (e.g., email)
    /// * `bundle` - Token bundle to store
    async fn store(
        &self,
        provider: &str,
        account_id: &str,
        bundle: &TokenBundle,
    ) -> OAuthResult<()>;

    /// Load a token bundle for a provider and account.
    ///
    /// # Arguments
    /// * `provider` - Provider identifier
    /// * `account_id` - Account identifier
    ///
    /// # Returns
    /// The token bundle if found, None otherwise
    async fn load(&self, provider: &str, account_id: &str) -> OAuthResult<Option<TokenBundle>>;

    /// Delete a token bundle.
    ///
    /// # Arguments
    /// * `provider` - Provider identifier
    /// * `account_id` - Account identifier
    async fn delete(&self, provider: &str, account_id: &str) -> OAuthResult<()>;

    /// List all account IDs for a provider.
    ///
    /// # Arguments
    /// * `provider` - Provider identifier
    ///
    /// # Returns
    /// List of account IDs with stored tokens
    async fn list_accounts(&self, provider: &str) -> OAuthResult<Vec<String>>;

    /// List all providers with stored tokens.
    ///
    /// # Returns
    /// List of provider identifiers
    async fn list_providers(&self) -> OAuthResult<Vec<String>>;

    /// Check if a token exists for a provider and account.
    ///
    /// Default implementation uses `load` but can be overridden for efficiency.
    async fn exists(&self, provider: &str, account_id: &str) -> OAuthResult<bool> {
        Ok(self.load(provider, account_id).await?.is_some())
    }

    /// Get all tokens for a provider.
    ///
    /// # Arguments
    /// * `provider` - Provider identifier
    ///
    /// # Returns
    /// List of (account_id, token_bundle) pairs
    async fn list_all_for_provider(
        &self,
        provider: &str,
    ) -> OAuthResult<Vec<(String, TokenBundle)>> {
        let accounts = self.list_accounts(provider).await?;
        let mut result = Vec::with_capacity(accounts.len());

        for account_id in accounts {
            if let Some(bundle) = self.load(provider, &account_id).await? {
                result.push((account_id, bundle));
            }
        }

        Ok(result)
    }

    /// Delete all tokens for a provider.
    ///
    /// # Arguments
    /// * `provider` - Provider identifier
    async fn delete_all_for_provider(&self, provider: &str) -> OAuthResult<()> {
        let accounts = self.list_accounts(provider).await?;
        for account_id in accounts {
            self.delete(provider, &account_id).await?;
        }
        Ok(())
    }

    /// Get token statistics.
    async fn stats(&self) -> OAuthResult<TokenStoreStats> {
        let providers = self.list_providers().await?;
        let mut total_tokens = 0;
        let mut expired_tokens = 0;
        let mut tokens_by_provider = std::collections::HashMap::new();

        for provider in &providers {
            let accounts = self.list_accounts(provider).await?;
            let count = accounts.len();
            tokens_by_provider.insert(provider.clone(), count);
            total_tokens += count;

            for account_id in accounts {
                if let Some(bundle) = self.load(provider, &account_id).await? {
                    if bundle.is_expired() {
                        expired_tokens += 1;
                    }
                }
            }
        }

        Ok(TokenStoreStats {
            total_tokens,
            expired_tokens,
            providers_count: providers.len(),
            tokens_by_provider,
        })
    }
}

/// Statistics about stored tokens.
#[derive(Debug, Clone)]
pub struct TokenStoreStats {
    /// Total number of stored tokens
    pub total_tokens: usize,
    /// Number of expired tokens
    pub expired_tokens: usize,
    /// Number of providers with tokens
    pub providers_count: usize,
    /// Token count per provider
    pub tokens_by_provider: std::collections::HashMap<String, usize>,
}

/// Token summary for management API responses.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenSummary {
    /// Provider identifier
    pub provider: String,
    /// Account identifier
    pub account_id: String,
    /// When the token expires
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether the token is currently valid
    pub valid: bool,
    /// Whether the token has a refresh token
    pub has_refresh_token: bool,
    /// When the token was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last refresh time
    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&TokenBundle> for TokenSummary {
    fn from(bundle: &TokenBundle) -> Self {
        Self {
            provider: bundle.provider.clone(),
            account_id: bundle.account_id.clone(),
            expires_at: bundle.expires_at,
            valid: !bundle.is_expired(),
            has_refresh_token: bundle.refresh_token.is_some(),
            created_at: bundle.created_at,
            last_refresh: bundle.last_refresh,
        }
    }
}

/// In-memory token store for testing.
#[derive(Default)]
pub struct MemoryTokenStore {
    tokens: tokio::sync::RwLock<std::collections::HashMap<String, TokenBundle>>,
}

impl MemoryTokenStore {
    pub fn new() -> Self {
        Self {
            tokens: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    fn make_key(provider: &str, account_id: &str) -> String {
        format!("{}:{}", provider, account_id)
    }
}

#[async_trait]
impl TokenStore for MemoryTokenStore {
    async fn store(
        &self,
        provider: &str,
        account_id: &str,
        bundle: &TokenBundle,
    ) -> OAuthResult<()> {
        let key = Self::make_key(provider, account_id);
        self.tokens.write().await.insert(key, bundle.clone());
        Ok(())
    }

    async fn load(&self, provider: &str, account_id: &str) -> OAuthResult<Option<TokenBundle>> {
        let key = Self::make_key(provider, account_id);
        Ok(self.tokens.read().await.get(&key).cloned())
    }

    async fn delete(&self, provider: &str, account_id: &str) -> OAuthResult<()> {
        let key = Self::make_key(provider, account_id);
        self.tokens.write().await.remove(&key);
        Ok(())
    }

    async fn list_accounts(&self, provider: &str) -> OAuthResult<Vec<String>> {
        let prefix = format!("{}:", provider);
        let tokens = self.tokens.read().await;
        Ok(tokens
            .keys()
            .filter_map(|k| {
                if k.starts_with(&prefix) {
                    Some(k[prefix.len()..].to_string())
                } else {
                    None
                }
            })
            .collect())
    }

    async fn list_providers(&self) -> OAuthResult<Vec<String>> {
        let tokens = self.tokens.read().await;
        let mut providers: Vec<String> = tokens
            .keys()
            .filter_map(|k| k.split(':').next().map(String::from))
            .collect();
        providers.sort();
        providers.dedup();
        Ok(providers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn create_test_bundle(provider: &str, account_id: &str) -> TokenBundle {
        TokenBundle {
            access_token: "test_token".to_string(),
            refresh_token: Some("refresh".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() + Duration::hours(1)),
            scope: Some("read".to_string()),
            provider: provider.to_string(),
            account_id: account_id.to_string(),
            metadata: Default::default(),
            created_at: Utc::now(),
            last_refresh: None,
        }
    }

    #[tokio::test]
    async fn test_memory_store_store_and_load() {
        let store = MemoryTokenStore::new();
        let bundle = create_test_bundle("claude", "user@example.com");

        store
            .store("claude", "user@example.com", &bundle)
            .await
            .unwrap();

        let loaded = store.load("claude", "user@example.com").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().access_token, "test_token");
    }

    #[tokio::test]
    async fn test_memory_store_load_nonexistent() {
        let store = MemoryTokenStore::new();
        let result = store.load("claude", "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_memory_store_delete() {
        let store = MemoryTokenStore::new();
        let bundle = create_test_bundle("claude", "user@example.com");

        store
            .store("claude", "user@example.com", &bundle)
            .await
            .unwrap();
        assert!(store.exists("claude", "user@example.com").await.unwrap());

        store.delete("claude", "user@example.com").await.unwrap();
        assert!(!store.exists("claude", "user@example.com").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_store_list_accounts() {
        let store = MemoryTokenStore::new();

        store
            .store(
                "claude",
                "user1@example.com",
                &create_test_bundle("claude", "user1@example.com"),
            )
            .await
            .unwrap();
        store
            .store(
                "claude",
                "user2@example.com",
                &create_test_bundle("claude", "user2@example.com"),
            )
            .await
            .unwrap();
        store
            .store(
                "gemini",
                "user3@example.com",
                &create_test_bundle("gemini", "user3@example.com"),
            )
            .await
            .unwrap();

        let claude_accounts = store.list_accounts("claude").await.unwrap();
        assert_eq!(claude_accounts.len(), 2);
        assert!(claude_accounts.contains(&"user1@example.com".to_string()));
        assert!(claude_accounts.contains(&"user2@example.com".to_string()));

        let gemini_accounts = store.list_accounts("gemini").await.unwrap();
        assert_eq!(gemini_accounts.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_store_list_providers() {
        let store = MemoryTokenStore::new();

        store
            .store(
                "claude",
                "user@example.com",
                &create_test_bundle("claude", "user@example.com"),
            )
            .await
            .unwrap();
        store
            .store(
                "gemini",
                "user@example.com",
                &create_test_bundle("gemini", "user@example.com"),
            )
            .await
            .unwrap();

        let providers = store.list_providers().await.unwrap();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&"claude".to_string()));
        assert!(providers.contains(&"gemini".to_string()));
    }

    #[tokio::test]
    async fn test_memory_store_list_all_for_provider() {
        let store = MemoryTokenStore::new();

        store
            .store(
                "claude",
                "user1@example.com",
                &create_test_bundle("claude", "user1@example.com"),
            )
            .await
            .unwrap();
        store
            .store(
                "claude",
                "user2@example.com",
                &create_test_bundle("claude", "user2@example.com"),
            )
            .await
            .unwrap();

        let all = store.list_all_for_provider("claude").await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_store_delete_all_for_provider() {
        let store = MemoryTokenStore::new();

        store
            .store(
                "claude",
                "user1@example.com",
                &create_test_bundle("claude", "user1@example.com"),
            )
            .await
            .unwrap();
        store
            .store(
                "claude",
                "user2@example.com",
                &create_test_bundle("claude", "user2@example.com"),
            )
            .await
            .unwrap();
        store
            .store(
                "gemini",
                "user@example.com",
                &create_test_bundle("gemini", "user@example.com"),
            )
            .await
            .unwrap();

        store.delete_all_for_provider("claude").await.unwrap();

        assert!(store.list_accounts("claude").await.unwrap().is_empty());
        assert_eq!(store.list_accounts("gemini").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_memory_store_stats() {
        let store = MemoryTokenStore::new();

        store
            .store(
                "claude",
                "user1@example.com",
                &create_test_bundle("claude", "user1@example.com"),
            )
            .await
            .unwrap();
        store
            .store(
                "claude",
                "user2@example.com",
                &create_test_bundle("claude", "user2@example.com"),
            )
            .await
            .unwrap();
        store
            .store(
                "gemini",
                "user@example.com",
                &create_test_bundle("gemini", "user@example.com"),
            )
            .await
            .unwrap();

        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total_tokens, 3);
        assert_eq!(stats.providers_count, 2);
        assert_eq!(stats.tokens_by_provider.get("claude"), Some(&2));
        assert_eq!(stats.tokens_by_provider.get("gemini"), Some(&1));
    }

    #[test]
    fn test_token_summary_from_bundle() {
        let bundle = create_test_bundle("claude", "user@example.com");
        let summary = TokenSummary::from(&bundle);

        assert_eq!(summary.provider, "claude");
        assert_eq!(summary.account_id, "user@example.com");
        assert!(summary.valid);
        assert!(summary.has_refresh_token);
    }

    #[test]
    fn test_token_summary_serialization() {
        let bundle = create_test_bundle("claude", "user@example.com");
        let summary = TokenSummary::from(&bundle);

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("claude"));
        assert!(json.contains("user@example.com"));
    }
}
