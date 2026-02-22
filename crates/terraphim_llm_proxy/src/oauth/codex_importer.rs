//! Codex CLI token importer for OpenAI OAuth integration.
//!
//! This module provides functionality to import existing OAuth tokens from the
//! Codex CLI's authentication file (`~/.codex/auth.json`), allowing seamless
//! integration with existing Codex CLI setups.
//!
//! # Codex Auth File Format
//!
//! The `~/.codex/auth.json` file contains:
//! ```json
//! {
//!   "access_token": "eyJhbGciOiJSUzI1NiIs...",
//!   "refresh_token": "def50200...",
//!   "id_token": "eyJhbGciOiJSUzI1NiIs...",
//!   "account": {
//!     "id": "eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa",
//!     "email": "user@example.com"
//!   },
//!   "expires": "2026-02-11T12:00:00.000Z"
//! }
//! ```
//!
//! # Token Storage
//!
//! Imported tokens are stored at: `~/.terraphim-llm-proxy/auth/openai/{account_id}.json`

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::oauth::error::{OAuthError, OAuthResult};
use crate::oauth::file_store::FileTokenStore;
use crate::oauth::store::TokenStore;
use crate::oauth::types::TokenBundle;

/// Default path to Codex CLI auth file
const CODEX_AUTH_PATH: &str = ".codex/auth.json";

/// Codex authentication file structure
#[derive(Debug, Deserialize)]
struct CodexAuthFile {
    /// Access token (JWT)
    access_token: String,

    /// Refresh token
    #[allow(dead_code)]
    refresh_token: Option<String>,

    /// ID token (JWT)
    #[allow(dead_code)]
    id_token: Option<String>,

    /// Account information
    account: CodexAccount,

    /// Expiration timestamp (ISO 8601)
    expires: Option<String>,
}

/// Codex account information
#[derive(Debug, Deserialize)]
struct CodexAccount {
    /// Account ID (UUID)
    id: String,

    /// Email address
    #[allow(dead_code)]
    email: Option<String>,
}

/// JWT Claims structure for parsing access tokens
#[derive(Debug, Deserialize)]
struct JwtClaims {
    /// Subject (account ID)
    sub: String,

    /// Expiration time (Unix timestamp)
    exp: Option<i64>,

    /// Email address
    email: Option<String>,

    /// Display name (present in JWT but not used directly)
    #[allow(dead_code)]
    name: Option<String>,
}

/// Codex token importer
#[derive(Debug, Clone)]
pub struct CodexTokenImporter {
    /// Path to the Codex auth file
    auth_file_path: PathBuf,
}

impl CodexTokenImporter {
    /// Create a new Codex token importer with the default auth file path.
    pub fn new() -> Self {
        Self {
            auth_file_path: Self::default_auth_path(),
        }
    }

    /// Create with a custom auth file path.
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            auth_file_path: path,
        }
    }

    /// Get the default path to the Codex auth file.
    fn default_auth_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(CODEX_AUTH_PATH)
    }

    /// Check if Codex auth file exists.
    pub fn auth_file_exists(&self) -> bool {
        self.auth_file_path.exists()
    }

    /// Read and parse the Codex auth file.
    ///
    /// # Returns
    /// The parsed auth file or an error if reading/parsing fails.
    async fn read_auth_file(&self) -> OAuthResult<CodexAuthFile> {
        if !self.auth_file_exists() {
            return Err(OAuthError::StorageError(format!(
                "Codex auth file not found at: {}",
                self.auth_file_path.display()
            )));
        }

        let content = tokio::fs::read_to_string(&self.auth_file_path)
            .await
            .map_err(|e| {
                OAuthError::StorageError(format!("Failed to read Codex auth file: {}", e))
            })?;

        let auth_file: CodexAuthFile = serde_json::from_str(&content).map_err(|e| {
            OAuthError::StorageError(format!("Failed to parse Codex auth file: {}", e))
        })?;

        Ok(auth_file)
    }

    /// Parse JWT and extract claims.
    ///
    /// # Arguments
    /// * `jwt` - The JWT token string
    ///
    /// # Returns
    /// Parsed JWT claims or None if parsing fails.
    fn parse_jwt(jwt: &str) -> Option<JwtClaims> {
        // JWT structure: header.payload.signature
        let parts: Vec<&str> = jwt.split('.').collect();
        if parts.len() != 3 {
            warn!("Invalid JWT format: expected 3 parts, got {}", parts.len());
            return None;
        }

        // Decode the payload (base64url encoded)
        let payload = parts[1];
        let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;

        serde_json::from_slice(&decoded).ok()
    }

    /// Extract expiration from JWT or parsed ISO string.
    ///
    /// # Arguments
    /// * `jwt` - The access token (JWT)
    /// * `expires_str` - Optional ISO 8601 expiration string from auth file
    ///
    /// # Returns
    /// Expiration timestamp or None if not found.
    fn extract_expiration(jwt: &str, expires_str: Option<&str>) -> Option<DateTime<Utc>> {
        // First try to extract from JWT
        if let Some(claims) = Self::parse_jwt(jwt) {
            if let Some(exp) = claims.exp {
                return Utc.timestamp_opt(exp, 0).single();
            }
        }

        // Fallback to parsing the ISO string from auth file
        if let Some(expires) = expires_str {
            if let Ok(dt) = DateTime::parse_from_rfc3339(expires) {
                return Some(dt.with_timezone(&Utc));
            }
        }

        None
    }

    /// Import tokens from Codex auth file.
    ///
    /// Reads the existing Codex authentication file and converts the tokens
    /// into a TokenBundle suitable for use with the OpenAI OAuth provider.
    ///
    /// # Returns
    /// TokenBundle containing the imported tokens, or an error if import fails.
    pub async fn import_tokens(&self) -> OAuthResult<TokenBundle> {
        info!(
            "Importing tokens from Codex auth file: {}",
            self.auth_file_path.display()
        );

        let auth_file = self.read_auth_file().await?;

        // Extract account ID (prefer JWT sub, fallback to auth file account.id)
        let account_id = Self::parse_jwt(&auth_file.access_token)
            .map(|claims| claims.sub)
            .unwrap_or_else(|| auth_file.account.id.clone());

        // Extract expiration
        let expires_at =
            Self::extract_expiration(&auth_file.access_token, auth_file.expires.as_deref());

        // Extract email from JWT if available
        let email = Self::parse_jwt(&auth_file.access_token).and_then(|claims| claims.email);

        let bundle = TokenBundle {
            access_token: auth_file.access_token,
            refresh_token: auth_file.refresh_token,
            token_type: "Bearer".to_string(),
            expires_at,
            scope: Some("openid email profile api".to_string()),
            provider: "openai".to_string(),
            account_id: account_id.clone(),
            metadata: {
                let mut meta = std::collections::HashMap::new();
                if let Some(email) = email {
                    meta.insert("email".to_string(), serde_json::Value::String(email));
                }
                meta.insert(
                    "imported_from".to_string(),
                    serde_json::Value::String("codex".to_string()),
                );
                meta.insert(
                    "imported_at".to_string(),
                    serde_json::Value::String(Utc::now().to_rfc3339()),
                );
                meta
            },
            created_at: Utc::now(),
            last_refresh: None,
        };

        info!(
            account_id = %account_id,
            expires_at = ?expires_at,
            "Successfully imported Codex tokens"
        );

        Ok(bundle)
    }

    /// Import tokens and store them in the file token store.
    ///
    /// This is a convenience method that imports tokens and immediately
    /// stores them in the default file-based token storage.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if import or storage fails.
    pub async fn import_and_store(&self) -> OAuthResult<TokenBundle> {
        let bundle = self.import_tokens().await?;

        // Create the file token store
        let store = FileTokenStore::with_default_path().await?;

        // Store the token
        store
            .store(&bundle.provider, &bundle.account_id, &bundle)
            .await?;

        info!(
            provider = %bundle.provider,
            account_id = %bundle.account_id,
            "Stored imported Codex tokens"
        );

        Ok(bundle)
    }

    /// Get the account ID from the Codex auth file without importing.
    ///
    /// # Returns
    /// The account ID if the auth file exists and is valid.
    pub async fn get_account_id(&self) -> Option<String> {
        if !self.auth_file_exists() {
            return None;
        }

        match self.read_auth_file().await {
            Ok(auth_file) => Some(auth_file.account.id),
            Err(e) => {
                warn!("Failed to read auth file for account ID: {}", e);
                None
            }
        }
    }
}

impl Default for CodexTokenImporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the token import on startup if Codex auth file exists.
///
/// This function should be called during application startup to automatically
/// import any existing Codex tokens.
///
/// # Returns
/// Ok(Some(bundle)) if tokens were imported, Ok(None) if no auth file exists.
pub async fn import_codex_tokens_on_startup() -> OAuthResult<Option<TokenBundle>> {
    let importer = CodexTokenImporter::new();

    if !importer.auth_file_exists() {
        debug!("No Codex auth file found, skipping import");
        return Ok(None);
    }

    info!("Found Codex auth file, importing tokens...");

    match importer.import_and_store().await {
        Ok(bundle) => {
            info!(
                account_id = %bundle.account_id,
                "Successfully imported and stored Codex tokens on startup"
            );
            Ok(Some(bundle))
        }
        Err(e) => {
            warn!(error = %e, "Failed to import Codex tokens on startup");
            // Don't fail startup if import fails - just log the error
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_auth_file() -> (NamedTempFile, String) {
        // Header: {"alg":"RS256","typ":"JWT"}
        let header = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9";

        // Payload: {"sub":"eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa","exp":1707177600,"email":"test@example.com","name":"Test User"}
        let payload = "eyJzdWIiOiJlYjc4ZmQxZS1mYWQwLTQyZTAtYjliZC0wNjc0YzdlYTk0ZmEiLCJleHAiOjE3MDcxNzc2MDAsImVtYWlsIjoidGVzdEBleGFtcGxlLmNvbSIsIm5hbWUiOiJUZXN0IFVzZXIifQ";

        let access_token = format!("{}.{}", header, payload);

        let auth_content = format!(
            r#"{{
                "access_token": "{}.sig",
                "refresh_token": "def50200...",
                "id_token": "eyJhbGciOiJSUzI1NiIs...",
                "account": {{
                    "id": "eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa",
                    "email": "test@example.com"
                }},
                "expires": "2026-02-11T12:00:00.000Z"
            }}"#,
            access_token
        );

        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), auth_content).unwrap();

        (temp_file, access_token)
    }

    #[test]
    fn test_default_auth_path() {
        let importer = CodexTokenImporter::new();
        let path = importer.auth_file_path;

        assert!(path.to_string_lossy().contains(".codex"));
        assert!(path.to_string_lossy().contains("auth.json"));
    }

    #[test]
    fn test_custom_auth_path() {
        let custom_path = PathBuf::from("/custom/path/auth.json");
        let importer = CodexTokenImporter::with_path(custom_path.clone());

        assert_eq!(importer.auth_file_path, custom_path);
    }

    #[test]
    fn test_auth_file_exists() {
        let (temp_file, _) = create_test_auth_file();
        let importer = CodexTokenImporter::with_path(temp_file.path().to_path_buf());

        assert!(importer.auth_file_exists());
    }

    #[test]
    fn test_auth_file_not_exists() {
        let importer = CodexTokenImporter::with_path(PathBuf::from("/nonexistent/path.json"));
        assert!(!importer.auth_file_exists());
    }

    #[tokio::test]
    async fn test_read_auth_file_success() {
        let (temp_file, _) = create_test_auth_file();
        let importer = CodexTokenImporter::with_path(temp_file.path().to_path_buf());

        let auth = importer.read_auth_file().await.unwrap();

        assert_eq!(auth.account.id, "eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa");
        assert!(auth.refresh_token.is_some());
    }

    #[tokio::test]
    async fn test_read_auth_file_not_found() {
        let importer = CodexTokenImporter::with_path(PathBuf::from("/nonexistent/path.json"));

        let result = importer.read_auth_file().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_jwt_valid() {
        // Header
        let header = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9";

        // Payload with account info
        let payload = "eyJzdWIiOiJlYjc4ZmQxZS1mYWQwLTQyZTAtYjliZC0wNjc0YzdlYTk0ZmEiLCJleHAiOjE3MDcxNzc2MDAsImVtYWlsIjoidGVzdEBleGFtcGxlLmNvbSIsIm5hbWUiOiJUZXN0IFVzZXIifQ";

        // JWT needs 3 parts: header.payload.signature
        let jwt = format!("{}.{}.sig", header, payload);

        let claims = CodexTokenImporter::parse_jwt(&jwt).unwrap();

        assert_eq!(claims.sub, "eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa");
        assert_eq!(claims.exp, Some(1707177600));
        assert_eq!(claims.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_parse_jwt_invalid() {
        assert!(CodexTokenImporter::parse_jwt("invalid").is_none());
        assert!(CodexTokenImporter::parse_jwt("a.b").is_none());
    }

    #[test]
    fn test_extract_expiration_from_jwt() {
        // Header
        let header = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9";

        // Payload with exp: 1707177600 (Feb 5, 2024 20:00:00 UTC)
        let payload = "eyJzdWIiOiJ0ZXN0IiwiZXhwIjoxNzA3MTc3NjAwfQ";

        // JWT needs 3 parts: header.payload.signature
        let jwt = format!("{}.{}.sig", header, payload);

        let exp = CodexTokenImporter::extract_expiration(
            &jwt,
            Some("2025-01-01T00:00:00Z"), // This should be ignored since JWT has exp
        );

        assert!(exp.is_some());
        let expected = Utc.timestamp_opt(1707177600, 0).single().unwrap();
        assert_eq!(exp.unwrap(), expected);
    }

    #[test]
    fn test_extract_expiration_from_iso_string() {
        // JWT without exp claim
        let jwt = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.sig";

        let exp = CodexTokenImporter::extract_expiration(jwt, Some("2026-02-11T12:00:00.000Z"));

        assert!(exp.is_some());
    }

    #[tokio::test]
    async fn test_import_tokens() {
        let (temp_file, _) = create_test_auth_file();
        let importer = CodexTokenImporter::with_path(temp_file.path().to_path_buf());

        let bundle = importer.import_tokens().await.unwrap();

        assert_eq!(bundle.provider, "openai");
        assert_eq!(bundle.account_id, "eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa");
        assert!(bundle.refresh_token.is_some());
        assert_eq!(bundle.token_type, "Bearer");
        assert!(bundle.metadata.contains_key("imported_from"));
        assert!(bundle.metadata.contains_key("imported_at"));
    }

    #[tokio::test]
    async fn test_get_account_id() {
        let (temp_file, _) = create_test_auth_file();
        let importer = CodexTokenImporter::with_path(temp_file.path().to_path_buf());

        let account_id = importer.get_account_id().await;

        assert_eq!(
            account_id,
            Some("eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_account_id_not_found() {
        let importer = CodexTokenImporter::with_path(PathBuf::from("/nonexistent/path.json"));

        let account_id = importer.get_account_id().await;

        assert!(account_id.is_none());
    }

    #[tokio::test]
    async fn test_import_and_store() {
        use tempfile::TempDir;

        // Create a temporary directory for token storage
        let temp_dir = TempDir::new().unwrap();

        // Create auth file
        let (auth_file, _) = create_test_auth_file();

        let importer = CodexTokenImporter::with_path(auth_file.path().to_path_buf());

        // Override the default token store path for testing
        // We'll need to use a custom store
        let store = FileTokenStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let bundle = importer.import_tokens().await.unwrap();

        // Store manually since we can't easily override the path in import_and_store
        store
            .store(&bundle.provider, &bundle.account_id, &bundle)
            .await
            .unwrap();

        // Verify it was stored
        let loaded = store
            .load(&bundle.provider, &bundle.account_id)
            .await
            .unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.account_id, bundle.account_id);
        assert_eq!(loaded.provider, bundle.provider);
    }
}
