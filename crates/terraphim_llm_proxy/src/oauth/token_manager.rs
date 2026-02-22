//! Token lifecycle manager with file-locked refresh.
//!
//! Provides a unified `get_or_refresh_token()` that loads tokens, checks expiry,
//! and refreshes with advisory file locking to prevent concurrent refresh storms
//! in multi-instance deployments.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::oauth::error::{OAuthError, OAuthResult};
use crate::oauth::file_store::FileTokenStore;
use crate::oauth::provider::OAuthProvider;
use crate::oauth::store::TokenStore;
use crate::oauth::types::TokenBundle;

/// How far before expiry to proactively refresh (5 minutes).
const REFRESH_BUFFER_SECONDS: i64 = 300;

/// Maximum time to wait for lock acquisition (seconds).
const LOCK_TIMEOUT_SECONDS: u64 = 30;

/// Manages OAuth token lifecycle: load, validate, refresh with file locking.
pub struct TokenManager {
    store: Arc<FileTokenStore>,
    providers: Arc<RwLock<HashMap<String, Arc<dyn OAuthProvider>>>>,
}

impl TokenManager {
    /// Create a new TokenManager.
    pub fn new(
        store: Arc<FileTokenStore>,
        providers: Arc<RwLock<HashMap<String, Arc<dyn OAuthProvider>>>>,
    ) -> Self {
        Self { store, providers }
    }

    /// Create a TokenManager with a single provider.
    pub fn with_single_provider(
        store: Arc<FileTokenStore>,
        provider_id: String,
        provider: Arc<dyn OAuthProvider>,
    ) -> Self {
        let mut map = HashMap::new();
        map.insert(provider_id, provider);
        Self {
            store,
            providers: Arc::new(RwLock::new(map)),
        }
    }

    /// Get a reference to the underlying token store.
    pub fn store(&self) -> &Arc<FileTokenStore> {
        &self.store
    }

    /// Get a valid token, refreshing if expired or expiring within 5 minutes.
    ///
    /// Fast path: if the token is valid and not expiring soon, returns immediately
    /// without any locking. Only acquires a file lock when refresh is needed.
    pub async fn get_or_refresh_token(
        &self,
        provider_id: &str,
        account_id: &str,
    ) -> OAuthResult<TokenBundle> {
        // Load token from store
        let bundle = self
            .store
            .load(provider_id, account_id)
            .await?
            .ok_or_else(|| OAuthError::TokenNotFound {
                provider: provider_id.to_string(),
                account_id: account_id.to_string(),
            })?;

        // Fast path: token is valid and not expiring soon
        if !Self::token_needs_refresh(&bundle) {
            debug!(
                "Token for {}/{} is valid, returning immediately",
                provider_id, account_id
            );
            return Ok(bundle);
        }

        // Slow path: need to refresh with file locking
        info!(
            "Token for {}/{} needs refresh, acquiring lock",
            provider_id, account_id
        );

        let provider = {
            let providers = self.providers.read().await;
            providers
                .get(provider_id)
                .cloned()
                .ok_or_else(|| OAuthError::ProviderNotConfigured(provider_id.to_string()))?
        };

        self.locked_refresh(provider_id, account_id, provider).await
    }

    /// Check if a token needs refresh (expired or expiring within buffer).
    fn token_needs_refresh(bundle: &TokenBundle) -> bool {
        bundle.expires_within(chrono::Duration::seconds(REFRESH_BUFFER_SECONDS))
    }

    /// Acquire file lock, re-read token, refresh if still needed, store result.
    ///
    /// Uses atomic lock file creation (O_EXCL) for cross-process safety,
    /// with stale lock detection and automatic cleanup.
    async fn locked_refresh(
        &self,
        provider_id: &str,
        account_id: &str,
        provider: Arc<dyn OAuthProvider>,
    ) -> OAuthResult<TokenBundle> {
        let lock_path = self.lock_path(provider_id);

        // Ensure lock file parent directory exists
        if let Some(parent) = lock_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                OAuthError::LockError(format!("Failed to create lock directory: {}", e))
            })?;
        }

        // Acquire cross-process file lock
        let lock_path_clone = lock_path.clone();
        let _lock_guard = tokio::task::spawn_blocking(move || acquire_lock_file(&lock_path_clone))
            .await
            .map_err(|e| OAuthError::LockError(format!("Lock task panicked: {}", e)))??;

        // Re-read token after acquiring lock (another instance may have refreshed)
        let bundle = self.store.load(provider_id, account_id).await?;

        match bundle {
            Some(bundle) if !Self::token_needs_refresh(&bundle) => {
                debug!(
                    "Token for {}/{} was refreshed by another instance",
                    provider_id, account_id
                );
                Ok(bundle)
            }
            Some(bundle) => {
                // Still needs refresh
                let refresh_token = bundle
                    .refresh_token
                    .as_deref()
                    .ok_or(OAuthError::TokenExpiredNoRefresh)?;

                info!("Refreshing token for {}/{}", provider_id, account_id);
                let new_bundle = provider.refresh_token(refresh_token).await.map_err(|e| {
                    OAuthError::RefreshFailed(format!(
                        "Provider {} refresh failed: {}",
                        provider_id, e
                    ))
                })?;

                // Store refreshed token
                self.store
                    .store(provider_id, account_id, &new_bundle)
                    .await?;

                info!(
                    "Token for {}/{} refreshed successfully",
                    provider_id, account_id
                );
                Ok(new_bundle)
            }
            None => Err(OAuthError::TokenNotFound {
                provider: provider_id.to_string(),
                account_id: account_id.to_string(),
            }),
        }
        // _lock_guard dropped here, which removes the lock file
    }

    /// Get the lock file path for a provider.
    fn lock_path(&self, provider_id: &str) -> PathBuf {
        self.store
            .base_path()
            .join(provider_id)
            .join(".refresh.lock")
    }
}

/// RAII guard that removes the lock file on drop.
struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Acquire a cross-process lock using atomic file creation (O_EXCL).
///
/// Creates a lock file atomically. If the file already exists, retries with
/// exponential backoff. Detects stale locks by checking file modification time.
fn acquire_lock_file(lock_path: &std::path::Path) -> OAuthResult<LockGuard> {
    use std::fs::OpenOptions;
    use std::io::ErrorKind;
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_secs(LOCK_TIMEOUT_SECONDS);
    let mut wait_ms = 50u64;

    loop {
        // Try to create lock file exclusively (atomic on all platforms)
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)
        {
            Ok(_file) => {
                return Ok(LockGuard {
                    path: lock_path.to_path_buf(),
                });
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                // Check for stale lock
                if let Ok(metadata) = std::fs::metadata(lock_path) {
                    if let Ok(modified) = metadata.modified() {
                        let age = modified.elapsed().unwrap_or(Duration::ZERO);
                        if age > timeout {
                            warn!("Stale lock detected (age: {:?}), removing", age);
                            let _ = std::fs::remove_file(lock_path);
                            continue;
                        }
                    }
                }

                if start.elapsed() >= timeout {
                    return Err(OAuthError::LockError(format!(
                        "Lock acquisition timed out after {}s",
                        LOCK_TIMEOUT_SECONDS
                    )));
                }

                std::thread::sleep(Duration::from_millis(wait_ms));
                wait_ms = (wait_ms * 2).min(2000);
            }
            Err(e) => {
                return Err(OAuthError::LockError(format!(
                    "Failed to create lock file: {}",
                    e
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::error::OAuthResult;
    use crate::oauth::provider::OAuthProvider;
    use crate::oauth::types::{AuthFlowState, TokenBundle, TokenValidation};
    use async_trait::async_trait;
    use chrono::{Duration as ChronoDuration, Utc};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tempfile::TempDir;

    /// Test provider that tracks refresh calls and returns predictable tokens.
    struct TestProvider {
        refresh_count: Arc<AtomicUsize>,
        should_fail: Arc<AtomicBool>,
    }

    impl TestProvider {
        fn new() -> Self {
            Self {
                refresh_count: Arc::new(AtomicUsize::new(0)),
                should_fail: Arc::new(AtomicBool::new(false)),
            }
        }

        fn set_should_fail(&self, fail: bool) {
            self.should_fail.store(fail, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl OAuthProvider for TestProvider {
        fn provider_id(&self) -> &str {
            "test"
        }

        fn display_name(&self) -> &str {
            "Test Provider"
        }

        async fn start_auth(&self, _callback_port: u16) -> OAuthResult<(String, AuthFlowState)> {
            unimplemented!("not needed for token_manager tests")
        }

        async fn exchange_code(
            &self,
            _code: &str,
            _state: &AuthFlowState,
        ) -> OAuthResult<TokenBundle> {
            unimplemented!("not needed for token_manager tests")
        }

        async fn refresh_token(&self, _refresh_token: &str) -> OAuthResult<TokenBundle> {
            self.refresh_count.fetch_add(1, Ordering::SeqCst);

            if self.should_fail.load(Ordering::SeqCst) {
                return Err(OAuthError::RefreshFailed("test failure".to_string()));
            }

            Ok(TokenBundle {
                access_token: "refreshed_token".to_string(),
                refresh_token: Some("new_refresh_token".to_string()),
                token_type: "Bearer".to_string(),
                expires_at: Some(Utc::now() + ChronoDuration::hours(1)),
                scope: Some("read write".to_string()),
                provider: "test".to_string(),
                account_id: "test_account".to_string(),
                metadata: Default::default(),
                created_at: Utc::now(),
                last_refresh: Some(Utc::now()),
            })
        }

        async fn validate_token(&self, _access_token: &str) -> OAuthResult<TokenValidation> {
            unimplemented!("not needed for token_manager tests")
        }

        fn token_endpoint(&self) -> &str {
            "https://test.example.com/token"
        }

        fn authorization_endpoint(&self) -> &str {
            "https://test.example.com/auth"
        }
    }

    fn create_test_bundle(
        provider: &str,
        account_id: &str,
        hours_until_expiry: i64,
    ) -> TokenBundle {
        TokenBundle {
            access_token: "original_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() + ChronoDuration::hours(hours_until_expiry)),
            scope: Some("read write".to_string()),
            provider: provider.to_string(),
            account_id: account_id.to_string(),
            metadata: Default::default(),
            created_at: Utc::now(),
            last_refresh: None,
        }
    }

    async fn setup() -> (TempDir, Arc<FileTokenStore>, TestProvider) {
        let temp_dir = TempDir::new().unwrap();
        let store = Arc::new(
            FileTokenStore::new(temp_dir.path().to_path_buf())
                .await
                .unwrap(),
        );
        let provider = TestProvider::new();
        (temp_dir, store, provider)
    }

    #[tokio::test]
    async fn test_get_valid_token_returns_immediately() {
        let (_dir, store, provider) = setup().await;
        let refresh_count = provider.refresh_count.clone();

        let bundle = create_test_bundle("test", "acct1", 1);
        store.store("test", "acct1", &bundle).await.unwrap();

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        let result = manager.get_or_refresh_token("test", "acct1").await.unwrap();
        assert_eq!(result.access_token, "original_token");
        assert_eq!(refresh_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_get_expired_token_refreshes() {
        let (_dir, store, provider) = setup().await;
        let refresh_count = provider.refresh_count.clone();

        let bundle = create_test_bundle("test", "acct1", -1);
        store.store("test", "acct1", &bundle).await.unwrap();

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        let result = manager.get_or_refresh_token("test", "acct1").await.unwrap();
        assert_eq!(result.access_token, "refreshed_token");
        assert_eq!(refresh_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_get_expiring_soon_refreshes() {
        let (_dir, store, provider) = setup().await;
        let refresh_count = provider.refresh_count.clone();

        let mut bundle = create_test_bundle("test", "acct1", 1);
        bundle.expires_at = Some(Utc::now() + ChronoDuration::minutes(3));
        store.store("test", "acct1", &bundle).await.unwrap();

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        let result = manager.get_or_refresh_token("test", "acct1").await.unwrap();
        assert_eq!(result.access_token, "refreshed_token");
        assert_eq!(refresh_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_get_token_expiring_in_10_minutes_no_refresh() {
        let (_dir, store, provider) = setup().await;
        let refresh_count = provider.refresh_count.clone();

        let mut bundle = create_test_bundle("test", "acct1", 1);
        bundle.expires_at = Some(Utc::now() + ChronoDuration::minutes(10));
        store.store("test", "acct1", &bundle).await.unwrap();

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        let result = manager.get_or_refresh_token("test", "acct1").await.unwrap();
        assert_eq!(result.access_token, "original_token");
        assert_eq!(refresh_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_get_expired_token_no_refresh_token_errors() {
        let (_dir, store, provider) = setup().await;

        let mut bundle = create_test_bundle("test", "acct1", -1);
        bundle.refresh_token = None;
        store.store("test", "acct1", &bundle).await.unwrap();

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        let err = manager
            .get_or_refresh_token("test", "acct1")
            .await
            .unwrap_err();
        assert!(matches!(err, OAuthError::TokenExpiredNoRefresh));
    }

    #[tokio::test]
    async fn test_get_nonexistent_token_errors() {
        let (_dir, store, provider) = setup().await;

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        let err = manager
            .get_or_refresh_token("test", "nonexistent")
            .await
            .unwrap_err();
        assert!(matches!(err, OAuthError::TokenNotFound { .. }));
    }

    #[tokio::test]
    async fn test_provider_not_configured_errors() {
        let (_dir, store, provider) = setup().await;

        let bundle = create_test_bundle("unknown", "acct1", -1);
        store.store("unknown", "acct1", &bundle).await.unwrap();

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        let err = manager
            .get_or_refresh_token("unknown", "acct1")
            .await
            .unwrap_err();
        assert!(matches!(err, OAuthError::ProviderNotConfigured(_)));
    }

    #[tokio::test]
    async fn test_lock_file_created_during_refresh() {
        let (dir, store, provider) = setup().await;

        let bundle = create_test_bundle("test", "acct1", -1);
        store.store("test", "acct1", &bundle).await.unwrap();

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        manager.get_or_refresh_token("test", "acct1").await.unwrap();

        // Lock file should have been cleaned up after refresh (LockGuard dropped)
        let lock_path = dir.path().join("test").join(".refresh.lock");
        assert!(
            !lock_path.exists(),
            "Lock file should be cleaned up after refresh completes"
        );
    }

    #[tokio::test]
    async fn test_refreshed_token_is_persisted() {
        let (_dir, store, provider) = setup().await;

        let bundle = create_test_bundle("test", "acct1", -1);
        store.store("test", "acct1", &bundle).await.unwrap();

        let store_clone = store.clone();
        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        manager.get_or_refresh_token("test", "acct1").await.unwrap();

        // Load directly from store to verify persistence
        let persisted = store_clone.load("test", "acct1").await.unwrap().unwrap();
        assert_eq!(persisted.access_token, "refreshed_token");
        assert_eq!(
            persisted.refresh_token.as_deref(),
            Some("new_refresh_token")
        );
    }

    #[tokio::test]
    async fn test_refresh_failure_propagates_error() {
        let (_dir, store, provider) = setup().await;
        provider.set_should_fail(true);

        let bundle = create_test_bundle("test", "acct1", -1);
        store.store("test", "acct1", &bundle).await.unwrap();

        let manager =
            TokenManager::with_single_provider(store, "test".to_string(), Arc::new(provider));

        let err = manager
            .get_or_refresh_token("test", "acct1")
            .await
            .unwrap_err();
        assert!(matches!(err, OAuthError::RefreshFailed(_)));
    }
}
