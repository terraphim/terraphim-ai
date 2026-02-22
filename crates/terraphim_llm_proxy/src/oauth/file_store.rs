//! File-based token storage for OAuth tokens.
//!
//! Stores tokens in the filesystem with the following structure:
//! ```text
//! ~/.terraphim-llm-proxy/auth/
//! ├── claude/
//! │   ├── user1@example.com.json
//! │   └── user2@example.com.json
//! └── gemini/
//!     └── user@example.com.json
//! ```

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, warn};

use crate::oauth::error::{OAuthError, OAuthResult};
use crate::oauth::store::TokenStore;
use crate::oauth::types::TokenBundle;

/// File-based token storage.
///
/// Stores OAuth tokens as JSON files in a directory structure organized
/// by provider and account ID. Uses atomic writes to prevent corruption.
#[derive(Debug, Clone)]
pub struct FileTokenStore {
    /// Base directory for token storage
    base_path: PathBuf,
}

impl FileTokenStore {
    /// Create a new file token store with the given base path.
    ///
    /// Creates the base directory if it doesn't exist.
    pub async fn new(base_path: PathBuf) -> OAuthResult<Self> {
        // Create base directory if it doesn't exist
        if !base_path.exists() {
            fs::create_dir_all(&base_path).await.map_err(|e| {
                OAuthError::StorageError(format!(
                    "Failed to create token directory {}: {}",
                    base_path.display(),
                    e
                ))
            })?;

            // Set directory permissions to 0700 (owner only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(0o700);
                std::fs::set_permissions(&base_path, permissions).map_err(|e| {
                    OAuthError::StorageError(format!("Failed to set directory permissions: {}", e))
                })?;
            }
        }

        Ok(Self { base_path })
    }

    /// Get the base path of this token store.
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get the default token storage path.
    ///
    /// Returns `~/.terraphim-llm-proxy/auth/`
    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".terraphim-llm-proxy").join("auth")
    }

    /// Create a file token store with the default path.
    pub async fn with_default_path() -> OAuthResult<Self> {
        Self::new(Self::default_path()).await
    }

    /// Get the directory path for a provider.
    fn provider_dir(&self, provider: &str) -> PathBuf {
        self.base_path.join(sanitize_filename(provider))
    }

    /// Get the file path for a token.
    fn token_path(&self, provider: &str, account_id: &str) -> PathBuf {
        self.provider_dir(provider)
            .join(format!("{}.json", sanitize_filename(account_id)))
    }

    /// Ensure the provider directory exists.
    async fn ensure_provider_dir(&self, provider: &str) -> OAuthResult<()> {
        let dir = self.provider_dir(provider);
        if !dir.exists() {
            fs::create_dir_all(&dir).await.map_err(|e| {
                OAuthError::StorageError(format!(
                    "Failed to create provider directory {}: {}",
                    dir.display(),
                    e
                ))
            })?;

            // Set directory permissions to 0700 (owner only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(0o700);
                std::fs::set_permissions(&dir, permissions).ok(); // Best effort
            }
        }
        Ok(())
    }

    /// Write data atomically using a temp file and rename.
    async fn atomic_write(&self, path: &Path, data: &[u8]) -> OAuthResult<()> {
        // Create temp file in the same directory
        let parent = path.parent().ok_or_else(|| {
            OAuthError::StorageError("Invalid path: no parent directory".to_string())
        })?;

        let temp_path = parent.join(format!(
            ".tmp.{}.{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        // Write to temp file
        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(|e| OAuthError::StorageError(format!("Failed to create temp file: {}", e)))?;

        file.write_all(data)
            .await
            .map_err(|e| OAuthError::StorageError(format!("Failed to write temp file: {}", e)))?;

        file.sync_all()
            .await
            .map_err(|e| OAuthError::StorageError(format!("Failed to sync temp file: {}", e)))?;

        drop(file);

        // Set file permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&temp_path, permissions).ok(); // Best effort
        }

        // Atomic rename
        fs::rename(&temp_path, path).await.map_err(|e| {
            // Clean up temp file on error
            let _ = std::fs::remove_file(&temp_path);
            OAuthError::StorageError(format!("Failed to rename temp file: {}", e))
        })?;

        debug!("Wrote token file: {}", path.display());
        Ok(())
    }
}

#[async_trait]
impl TokenStore for FileTokenStore {
    async fn store(
        &self,
        provider: &str,
        account_id: &str,
        bundle: &TokenBundle,
    ) -> OAuthResult<()> {
        self.ensure_provider_dir(provider).await?;

        let path = self.token_path(provider, account_id);
        let data = serde_json::to_vec_pretty(bundle)
            .map_err(|e| OAuthError::StorageError(format!("Failed to serialize token: {}", e)))?;

        self.atomic_write(&path, &data).await
    }

    async fn load(&self, provider: &str, account_id: &str) -> OAuthResult<Option<TokenBundle>> {
        let path = self.token_path(provider, account_id);

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(&path).await.map_err(|e| {
            OAuthError::StorageError(format!(
                "Failed to read token file {}: {}",
                path.display(),
                e
            ))
        })?;

        let bundle: TokenBundle = serde_json::from_slice(&data).map_err(|e| {
            warn!("Failed to parse token file {}: {}", path.display(), e);
            OAuthError::StorageError(format!(
                "Failed to parse token file {}: {}",
                path.display(),
                e
            ))
        })?;

        debug!("Loaded token for {}/{}", provider, account_id);
        Ok(Some(bundle))
    }

    async fn delete(&self, provider: &str, account_id: &str) -> OAuthResult<()> {
        let path = self.token_path(provider, account_id);

        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                OAuthError::StorageError(format!(
                    "Failed to delete token file {}: {}",
                    path.display(),
                    e
                ))
            })?;
            debug!("Deleted token file: {}", path.display());
        }

        Ok(())
    }

    async fn list_accounts(&self, provider: &str) -> OAuthResult<Vec<String>> {
        let dir = self.provider_dir(provider);

        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut accounts = Vec::new();
        let mut entries = fs::read_dir(&dir).await.map_err(|e| {
            OAuthError::StorageError(format!(
                "Failed to read provider directory {}: {}",
                dir.display(),
                e
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            OAuthError::StorageError(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        // Skip temp files
                        if !name.starts_with(".tmp.") {
                            accounts.push(unsanitize_filename(name));
                        }
                    }
                }
            }
        }

        accounts.sort();
        Ok(accounts)
    }

    async fn list_providers(&self) -> OAuthResult<Vec<String>> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut providers = Vec::new();
        let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
            OAuthError::StorageError(format!(
                "Failed to read base directory {}: {}",
                self.base_path.display(),
                e
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            OAuthError::StorageError(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        // Skip hidden directories
                        if !name_str.starts_with('.') {
                            providers.push(unsanitize_filename(name_str));
                        }
                    }
                }
            }
        }

        providers.sort();
        Ok(providers)
    }
}

/// Sanitize a filename by replacing problematic characters.
///
/// Replaces `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|` with `_`.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Reverse the filename sanitization (best effort).
///
/// Note: This is lossy since we can't know which `_` were original.
fn unsanitize_filename(name: &str) -> String {
    // Currently just returns as-is since we can't reliably reverse
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::TempDir;

    fn create_test_bundle(provider: &str, account_id: &str) -> TokenBundle {
        TokenBundle {
            access_token: "test_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() + Duration::hours(1)),
            scope: Some("read write".to_string()),
            provider: provider.to_string(),
            account_id: account_id.to_string(),
            metadata: Default::default(),
            created_at: Utc::now(),
            last_refresh: None,
        }
    }

    #[tokio::test]
    async fn test_file_store_new() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        assert!(store.base_path.exists());
    }

    #[tokio::test]
    async fn test_file_store_new_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("auth");

        let store = FileTokenStore::new(nested_path.clone()).await.unwrap();

        assert!(store.base_path.exists());
        assert!(nested_path.exists());
    }

    #[tokio::test]
    async fn test_file_store_store_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let bundle = create_test_bundle("claude", "user@example.com");
        store
            .store("claude", "user@example.com", &bundle)
            .await
            .unwrap();

        let loaded = store.load("claude", "user@example.com").await.unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.access_token, "test_access_token");
        assert_eq!(loaded.provider, "claude");
        assert_eq!(loaded.account_id, "user@example.com");
    }

    #[tokio::test]
    async fn test_file_store_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let result = store.load("claude", "nonexistent@example.com").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_store_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let bundle = create_test_bundle("claude", "user@example.com");
        store
            .store("claude", "user@example.com", &bundle)
            .await
            .unwrap();

        // Verify it exists
        assert!(store.exists("claude", "user@example.com").await.unwrap());

        // Delete
        store.delete("claude", "user@example.com").await.unwrap();

        // Verify it's gone
        assert!(!store.exists("claude", "user@example.com").await.unwrap());
    }

    #[tokio::test]
    async fn test_file_store_delete_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Should not error when deleting nonexistent token
        let result = store.delete("claude", "nonexistent@example.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_store_list_accounts() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Store multiple accounts
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
        assert!(gemini_accounts.contains(&"user3@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_file_store_list_accounts_empty_provider() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let accounts = store.list_accounts("nonexistent").await.unwrap();
        assert!(accounts.is_empty());
    }

    #[tokio::test]
    async fn test_file_store_list_providers() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

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
        store
            .store(
                "copilot",
                "user@example.com",
                &create_test_bundle("copilot", "user@example.com"),
            )
            .await
            .unwrap();

        let providers = store.list_providers().await.unwrap();
        assert_eq!(providers.len(), 3);
        assert!(providers.contains(&"claude".to_string()));
        assert!(providers.contains(&"gemini".to_string()));
        assert!(providers.contains(&"copilot".to_string()));
    }

    #[tokio::test]
    async fn test_file_store_list_providers_empty() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let providers = store.list_providers().await.unwrap();
        assert!(providers.is_empty());
    }

    #[tokio::test]
    async fn test_file_store_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Store initial token
        let mut bundle = create_test_bundle("claude", "user@example.com");
        bundle.access_token = "token_v1".to_string();
        store
            .store("claude", "user@example.com", &bundle)
            .await
            .unwrap();

        // Overwrite with new token
        bundle.access_token = "token_v2".to_string();
        store
            .store("claude", "user@example.com", &bundle)
            .await
            .unwrap();

        // Verify new token is stored
        let loaded = store
            .load("claude", "user@example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.access_token, "token_v2");
    }

    #[tokio::test]
    async fn test_file_store_atomic_write_creates_valid_json() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let bundle = create_test_bundle("claude", "user@example.com");
        store
            .store("claude", "user@example.com", &bundle)
            .await
            .unwrap();

        // Read the file directly and verify it's valid JSON
        let path = store.token_path("claude", "user@example.com");
        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        let _: TokenBundle = serde_json::from_str(&contents).unwrap();
    }

    #[tokio::test]
    async fn test_file_store_special_characters_in_account_id() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Account ID with special characters that need sanitization
        let account_id = "user+tag@example.com";
        let bundle = create_test_bundle("claude", account_id);

        store.store("claude", account_id, &bundle).await.unwrap();

        let loaded = store.load("claude", account_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().account_id, account_id);
    }

    #[tokio::test]
    async fn test_file_store_stats() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

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

    #[tokio::test]
    async fn test_file_store_delete_all_for_provider() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

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

    #[cfg(unix)]
    #[tokio::test]
    async fn test_file_store_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let bundle = create_test_bundle("claude", "user@example.com");
        store
            .store("claude", "user@example.com", &bundle)
            .await
            .unwrap();

        // Check file permissions
        let path = store.token_path("claude", "user@example.com");
        let metadata = std::fs::metadata(&path).unwrap();
        let permissions = metadata.permissions();

        // Should be 0600 (owner read/write only)
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("simple"), "simple");
        assert_eq!(sanitize_filename("user@example.com"), "user@example.com");
        assert_eq!(sanitize_filename("path/with/slashes"), "path_with_slashes");
        assert_eq!(sanitize_filename("file:name"), "file_name");
        assert_eq!(sanitize_filename("a*b?c"), "a_b_c");
    }

    #[test]
    fn test_default_path() {
        let path = FileTokenStore::default_path();
        assert!(path.to_string_lossy().contains(".terraphim-llm-proxy"));
        assert!(path.to_string_lossy().contains("auth"));
    }
}
