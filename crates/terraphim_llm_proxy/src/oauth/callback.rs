//! OAuth callback handling for browser-based authentication flows.
//!
//! This module provides HTTP handlers for OAuth callbacks and status polling,
//! enabling CLI tools to complete browser-based OAuth flows.
//!
//! # Flow
//!
//! 1. CLI calls provider.start_auth() to get authorization URL and registers flow state
//! 2. User is redirected to browser for authentication
//! 3. Provider redirects back to `/oauth/{provider}/callback` with code and state
//! 4. Callback handler exchanges code for tokens and stores them
//! 5. CLI polls `/oauth/{provider}/status` until completion
//! 6. Browser shows success page and auto-closes

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::oauth::provider::OAuthProvider;
use crate::oauth::store::TokenStore;
use crate::oauth::types::{AuthFlowState, FlowStatus, FlowStatusResponse};

/// State for tracking active OAuth flows.
#[derive(Debug)]
pub struct OAuthFlowManager {
    /// Active flows indexed by state token
    flows: RwLock<HashMap<String, FlowEntry>>,
    /// Completed flows with their status (kept for status polling)
    completed: RwLock<HashMap<String, CompletedFlow>>,
}

/// Entry for an active OAuth flow.
#[derive(Debug, Clone)]
pub struct FlowEntry {
    /// The flow state from the provider
    pub state: AuthFlowState,
    /// Provider ID for this flow
    pub provider_id: String,
}

/// Entry for a completed OAuth flow.
#[derive(Debug, Clone)]
struct CompletedFlow {
    /// Final status
    status: FlowStatus,
    /// Account ID if successful
    account_id: Option<String>,
    /// Error message if failed
    error: Option<String>,
    /// When the flow completed
    completed_at: chrono::DateTime<chrono::Utc>,
}

impl OAuthFlowManager {
    /// Create a new flow manager.
    pub fn new() -> Self {
        Self {
            flows: RwLock::new(HashMap::new()),
            completed: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new OAuth flow.
    pub async fn register_flow(&self, state: AuthFlowState, provider_id: String) {
        let state_token = state.state.clone();
        debug!("Registered OAuth flow for provider: {}", provider_id);
        let entry = FlowEntry { state, provider_id };
        self.flows.write().await.insert(state_token, entry);
    }

    /// Get a pending flow by state token.
    pub async fn get_flow(&self, state: &str) -> Option<FlowEntry> {
        self.flows.read().await.get(state).cloned()
    }

    /// Remove a pending flow and mark it as completed.
    pub async fn complete_flow(
        &self,
        state: &str,
        status: FlowStatus,
        account_id: Option<String>,
        error: Option<String>,
    ) {
        // Remove from active flows
        self.flows.write().await.remove(state);

        // Add to completed flows
        let completed = CompletedFlow {
            status,
            account_id,
            error,
            completed_at: chrono::Utc::now(),
        };
        self.completed
            .write()
            .await
            .insert(state.to_string(), completed);
        debug!("Completed OAuth flow, state: {}", state);
    }

    /// Get the status of a flow.
    pub async fn get_status(&self, state: &str) -> FlowStatusResponse {
        // Check if flow is still pending
        if self.flows.read().await.contains_key(state) {
            return FlowStatusResponse::pending();
        }

        // Check if flow is completed
        if let Some(completed) = self.completed.read().await.get(state) {
            return match completed.status {
                FlowStatus::Completed => {
                    FlowStatusResponse::completed(completed.account_id.clone().unwrap_or_default())
                }
                FlowStatus::Error => {
                    FlowStatusResponse::error(completed.error.clone().unwrap_or_default())
                }
                FlowStatus::Expired => FlowStatusResponse::expired(),
                FlowStatus::Pending => FlowStatusResponse::pending(),
            };
        }

        // Unknown flow
        FlowStatusResponse::error("Unknown or expired flow".to_string())
    }

    /// Clean up expired flows (older than 10 minutes).
    pub async fn cleanup_expired(&self) {
        let now = chrono::Utc::now();

        // Clean up pending flows
        let mut flows = self.flows.write().await;
        flows.retain(|_, entry| !entry.state.is_expired());

        // Clean up old completed flows (keep for 1 hour)
        let mut completed = self.completed.write().await;
        completed.retain(|_, entry| now - entry.completed_at < chrono::Duration::hours(1));
    }
}

impl Default for OAuthFlowManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared state for OAuth handlers.
pub struct OAuthState<S: TokenStore> {
    /// Flow manager for tracking active flows
    pub flow_manager: Arc<OAuthFlowManager>,
    /// Token store for persisting tokens
    pub token_store: Arc<S>,
    /// Registered OAuth providers (trait objects for multi-provider support)
    pub providers: Arc<RwLock<HashMap<String, Arc<dyn OAuthProvider>>>>,
    /// Per-provider callback ports (provider_id -> port)
    pub callback_ports: HashMap<String, u16>,
}

impl<S: TokenStore> Clone for OAuthState<S> {
    fn clone(&self) -> Self {
        Self {
            flow_manager: self.flow_manager.clone(),
            token_store: self.token_store.clone(),
            providers: self.providers.clone(),
            callback_ports: self.callback_ports.clone(),
        }
    }
}

/// Query parameters for OAuth callback.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    /// Authorization code from provider
    pub code: Option<String>,
    /// State token for CSRF validation
    pub state: Option<String>,
    /// Error code if authorization failed
    pub error: Option<String>,
    /// Error description
    pub error_description: Option<String>,
}

/// Query parameters for status polling.
#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    /// State token to check status for
    pub state: String,
}

/// HTML template for success page.
const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Authentication Successful</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }
        .container {
            text-align: center;
            padding: 40px;
            background: white;
            border-radius: 16px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
        }
        .checkmark {
            font-size: 64px;
            margin-bottom: 20px;
        }
        h1 {
            color: #333;
            margin-bottom: 10px;
        }
        p {
            color: #666;
            margin-bottom: 20px;
        }
        .countdown {
            color: #999;
            font-size: 14px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="checkmark">✓</div>
        <h1>Authentication Successful</h1>
        <p>You have been authenticated as <strong>{{ACCOUNT}}</strong></p>
        <p class="countdown">This window will close in <span id="countdown">3</span> seconds...</p>
    </div>
    <script>
        let seconds = 3;
        const countdown = document.getElementById('countdown');
        const timer = setInterval(() => {
            seconds--;
            countdown.textContent = seconds;
            if (seconds <= 0) {
                clearInterval(timer);
                window.close();
            }
        }, 1000);
    </script>
</body>
</html>"#;

/// HTML template for error page.
const ERROR_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Authentication Failed</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%);
        }
        .container {
            text-align: center;
            padding: 40px;
            background: white;
            border-radius: 16px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
        }
        .error-icon {
            font-size: 64px;
            margin-bottom: 20px;
        }
        h1 {
            color: #333;
            margin-bottom: 10px;
        }
        p {
            color: #666;
            margin-bottom: 20px;
        }
        .error-details {
            color: #e74c3c;
            font-size: 14px;
            background: #ffeaea;
            padding: 10px;
            border-radius: 8px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="error-icon">✗</div>
        <h1>Authentication Failed</h1>
        <p>Something went wrong during authentication.</p>
        <p class="error-details">{{ERROR}}</p>
        <p>You can close this window and try again.</p>
    </div>
</body>
</html>"#;

/// Handle OAuth callback from provider.
///
/// This is called when the OAuth provider redirects the user back after authentication.
pub async fn handle_callback<S: TokenStore + 'static>(
    State(state): State<OAuthState<S>>,
    Path(provider_id): Path<String>,
    Query(query): Query<CallbackQuery>,
) -> Response {
    info!("OAuth callback received for provider: {}", provider_id);

    // Check for OAuth error response
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_else(|| error.clone());
        warn!("OAuth error from provider {}: {}", provider_id, description);

        if let Some(state_token) = query.state {
            state
                .flow_manager
                .complete_flow(
                    &state_token,
                    FlowStatus::Error,
                    None,
                    Some(description.clone()),
                )
                .await;
        }

        return Html(ERROR_HTML.replace("{{ERROR}}", &description)).into_response();
    }

    // Validate required parameters
    let code = match query.code {
        Some(c) => c,
        None => {
            error!("Missing authorization code in callback");
            return Html(ERROR_HTML.replace("{{ERROR}}", "Missing authorization code"))
                .into_response();
        }
    };

    let state_token = match query.state {
        Some(s) => s,
        None => {
            error!("Missing state parameter in callback");
            return Html(ERROR_HTML.replace("{{ERROR}}", "Missing state parameter"))
                .into_response();
        }
    };

    // Look up the flow
    let flow_entry = match state.flow_manager.get_flow(&state_token).await {
        Some(entry) => entry,
        None => {
            error!("Unknown or expired OAuth flow: {}", state_token);
            return Html(ERROR_HTML.replace("{{ERROR}}", "Unknown or expired OAuth flow"))
                .into_response();
        }
    };

    // Validate provider matches
    if flow_entry.provider_id != provider_id {
        error!(
            "Provider mismatch: expected {}, got {}",
            flow_entry.provider_id, provider_id
        );
        state
            .flow_manager
            .complete_flow(
                &state_token,
                FlowStatus::Error,
                None,
                Some("Provider mismatch".to_string()),
            )
            .await;
        return Html(ERROR_HTML.replace("{{ERROR}}", "Provider mismatch")).into_response();
    }

    // Get the provider
    let providers = state.providers.read().await;
    let provider = match providers.get(&provider_id) {
        Some(p) => p.clone(),
        None => {
            error!("Provider not registered: {}", provider_id);
            state
                .flow_manager
                .complete_flow(
                    &state_token,
                    FlowStatus::Error,
                    None,
                    Some("Provider not registered".to_string()),
                )
                .await;
            return Html(ERROR_HTML.replace("{{ERROR}}", "Provider not registered"))
                .into_response();
        }
    };
    drop(providers);

    // Exchange code for tokens
    let token_bundle = match provider.exchange_code(&code, &flow_entry.state).await {
        Ok(bundle) => bundle,
        Err(e) => {
            error!("Token exchange failed: {}", e);
            state
                .flow_manager
                .complete_flow(&state_token, FlowStatus::Error, None, Some(e.to_string()))
                .await;
            return Html(ERROR_HTML.replace("{{ERROR}}", &e.to_string())).into_response();
        }
    };

    let account_id = token_bundle.account_id.clone();

    // Store the tokens
    if let Err(e) = state
        .token_store
        .store(&provider_id, &account_id, &token_bundle)
        .await
    {
        error!("Failed to store tokens: {}", e);
        state
            .flow_manager
            .complete_flow(&state_token, FlowStatus::Error, None, Some(e.to_string()))
            .await;
        return Html(ERROR_HTML.replace("{{ERROR}}", &e.to_string())).into_response();
    }

    // Mark flow as completed
    info!(
        "OAuth flow completed successfully for {} ({})",
        provider_id, account_id
    );
    state
        .flow_manager
        .complete_flow(
            &state_token,
            FlowStatus::Completed,
            Some(account_id.clone()),
            None,
        )
        .await;

    // Return success page
    Html(SUCCESS_HTML.replace("{{ACCOUNT}}", &account_id)).into_response()
}

/// Handle OAuth status polling.
///
/// CLI tools poll this endpoint to check if authentication is complete.
pub async fn handle_status<S: TokenStore + 'static>(
    State(state): State<OAuthState<S>>,
    Path(provider_id): Path<String>,
    Query(query): Query<StatusQuery>,
) -> Json<FlowStatusResponse> {
    debug!(
        "Status poll for provider {} with state {}",
        provider_id, query.state
    );
    Json(state.flow_manager.get_status(&query.state).await)
}

/// Response for login initiation.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// URL to open in the browser for authentication
    pub auth_url: String,
    /// State token for polling status
    pub state: String,
    /// Provider ID
    pub provider: String,
}

/// Response for provider listing.
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    /// Provider ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Whether this provider uses browser-based OAuth
    pub uses_browser_callback: bool,
}

/// Handle OAuth login initiation.
///
/// Starts an OAuth flow for the specified provider and returns the
/// authorization URL for the user to open in their browser.
pub async fn handle_login<S: TokenStore + 'static>(
    State(state): State<OAuthState<S>>,
    Path(provider_id): Path<String>,
) -> Response {
    info!("OAuth login requested for provider: {}", provider_id);

    let providers = state.providers.read().await;
    let provider = match providers.get(&provider_id) {
        Some(p) => p.clone(),
        None => {
            warn!(
                "OAuth login failed: provider not registered: {}",
                provider_id
            );
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Provider '{}' not found or not enabled", provider_id)
                })),
            )
                .into_response();
        }
    };
    drop(providers);

    // Get the configured callback port for this provider
    let callback_port = state
        .callback_ports
        .get(&provider_id)
        .copied()
        .unwrap_or(9999);

    // Start the auth flow
    match provider.start_auth(callback_port).await {
        Ok((auth_url, flow_state)) => {
            let state_token = flow_state.state.clone();
            state
                .flow_manager
                .register_flow(flow_state, provider_id.clone())
                .await;

            info!(
                "OAuth flow started for {}, state: {}",
                provider_id, state_token
            );

            Json(LoginResponse {
                auth_url,
                state: state_token,
                provider: provider_id,
            })
            .into_response()
        }
        Err(e) => {
            error!("Failed to start OAuth flow for {}: {}", provider_id, e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to start OAuth flow: {}", e)
                })),
            )
                .into_response()
        }
    }
}

/// Handle listing of available OAuth providers.
pub async fn handle_providers<S: TokenStore + 'static>(
    State(state): State<OAuthState<S>>,
) -> Json<Vec<ProviderInfo>> {
    let providers = state.providers.read().await;
    let provider_list: Vec<ProviderInfo> = providers
        .values()
        .map(|p| ProviderInfo {
            id: p.provider_id().to_string(),
            name: p.display_name().to_string(),
            uses_browser_callback: p.uses_browser_callback(),
        })
        .collect();
    Json(provider_list)
}

/// Create OAuth routes for a specific token store type.
pub fn oauth_routes<S: TokenStore + 'static>() -> Router<OAuthState<S>> {
    Router::new()
        .route("/oauth/providers", get(handle_providers::<S>))
        .route("/oauth/:provider/callback", get(handle_callback::<S>))
        .route("/oauth/:provider/status", get(handle_status::<S>))
        .route("/oauth/:provider/login", post(handle_login::<S>))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::error::OAuthResult;
    use crate::oauth::types::TokenBundle;
    use async_trait::async_trait;
    use chrono::Utc;

    /// Mock provider for testing (kept for future integration tests)
    #[allow(dead_code)]
    struct MockProvider {
        id: String,
    }

    #[allow(dead_code)]
    impl MockProvider {
        fn new(id: &str) -> Self {
            Self { id: id.to_string() }
        }
    }

    #[async_trait]
    impl OAuthProvider for MockProvider {
        fn provider_id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            "Mock Provider"
        }

        async fn start_auth(&self, callback_port: u16) -> OAuthResult<(String, AuthFlowState)> {
            let state =
                AuthFlowState::new("test_state".to_string(), self.id.clone(), callback_port);
            Ok((format!("https://mock.auth/{}", callback_port), state))
        }

        async fn exchange_code(
            &self,
            _code: &str,
            state: &AuthFlowState,
        ) -> OAuthResult<TokenBundle> {
            Ok(TokenBundle {
                access_token: "mock_access".to_string(),
                refresh_token: Some("mock_refresh".to_string()),
                token_type: "Bearer".to_string(),
                expires_at: None,
                scope: None,
                provider: state.provider.clone(),
                account_id: "test@example.com".to_string(),
                metadata: Default::default(),
                created_at: Utc::now(),
                last_refresh: None,
            })
        }

        async fn refresh_token(&self, _refresh_token: &str) -> OAuthResult<TokenBundle> {
            Ok(TokenBundle::new(
                "refreshed".to_string(),
                "Bearer".to_string(),
                self.id.clone(),
                "test@example.com".to_string(),
            ))
        }

        async fn validate_token(
            &self,
            _access_token: &str,
        ) -> OAuthResult<crate::oauth::types::TokenValidation> {
            Ok(crate::oauth::types::TokenValidation::valid(
                Some(3600),
                vec![],
            ))
        }

        fn token_endpoint(&self) -> &str {
            "https://mock.auth/token"
        }

        fn authorization_endpoint(&self) -> &str {
            "https://mock.auth/authorize"
        }
    }

    #[tokio::test]
    async fn test_flow_manager_register_and_get() {
        let manager = OAuthFlowManager::new();
        let state = AuthFlowState::new("test_state".to_string(), "claude".to_string(), 54545);

        manager
            .register_flow(state.clone(), "claude".to_string())
            .await;

        let retrieved = manager.get_flow("test_state").await.unwrap();
        assert_eq!(retrieved.provider_id, "claude");
        assert_eq!(retrieved.state.callback_port, 54545);
    }

    #[tokio::test]
    async fn test_flow_manager_complete_flow() {
        let manager = OAuthFlowManager::new();
        let state = AuthFlowState::new("test_state".to_string(), "claude".to_string(), 54545);

        manager.register_flow(state, "claude".to_string()).await;

        // Initially pending
        let status = manager.get_status("test_state").await;
        assert_eq!(status.status, FlowStatus::Pending);

        // Complete the flow
        manager
            .complete_flow(
                "test_state",
                FlowStatus::Completed,
                Some("user@example.com".to_string()),
                None,
            )
            .await;

        // Now completed
        let status = manager.get_status("test_state").await;
        assert_eq!(status.status, FlowStatus::Completed);
        assert_eq!(status.account_id, Some("user@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_flow_manager_error_flow() {
        let manager = OAuthFlowManager::new();
        let state = AuthFlowState::new("test_state".to_string(), "claude".to_string(), 54545);

        manager.register_flow(state, "claude".to_string()).await;

        // Mark as error
        manager
            .complete_flow(
                "test_state",
                FlowStatus::Error,
                None,
                Some("Token exchange failed".to_string()),
            )
            .await;

        let status = manager.get_status("test_state").await;
        assert_eq!(status.status, FlowStatus::Error);
        assert_eq!(status.error, Some("Token exchange failed".to_string()));
    }

    #[tokio::test]
    async fn test_flow_manager_unknown_flow() {
        let manager = OAuthFlowManager::new();

        let status = manager.get_status("nonexistent").await;
        assert_eq!(status.status, FlowStatus::Error);
        assert!(status.error.unwrap().contains("Unknown"));
    }

    #[test]
    fn test_success_html_template() {
        let html = SUCCESS_HTML.replace("{{ACCOUNT}}", "user@test.com");
        assert!(html.contains("user@test.com"));
        assert!(html.contains("Authentication Successful"));
        assert!(html.contains("window.close()"));
    }

    #[test]
    fn test_error_html_template() {
        let html = ERROR_HTML.replace("{{ERROR}}", "Access denied");
        assert!(html.contains("Access denied"));
        assert!(html.contains("Authentication Failed"));
    }

    #[tokio::test]
    async fn test_callback_query_parsing() {
        let query: CallbackQuery =
            serde_json::from_str(r#"{"code": "auth_code", "state": "csrf_token"}"#).unwrap();

        assert_eq!(query.code, Some("auth_code".to_string()));
        assert_eq!(query.state, Some("csrf_token".to_string()));
        assert!(query.error.is_none());
    }

    #[tokio::test]
    async fn test_callback_query_with_error() {
        let query: CallbackQuery = serde_json::from_str(
            r#"{"error": "access_denied", "error_description": "User denied access", "state": "csrf_token"}"#
        ).unwrap();

        assert_eq!(query.error, Some("access_denied".to_string()));
        assert_eq!(
            query.error_description,
            Some("User denied access".to_string())
        );
        assert!(query.code.is_none());
    }

    #[tokio::test]
    async fn test_status_query_parsing() {
        let query: StatusQuery = serde_json::from_str(r#"{"state": "csrf_token"}"#).unwrap();

        assert_eq!(query.state, "csrf_token");
    }

    #[test]
    fn test_flow_entry_clone() {
        let state = AuthFlowState::new("test".to_string(), "claude".to_string(), 54545);
        let entry = FlowEntry {
            state,
            provider_id: "claude".to_string(),
        };

        let cloned = entry.clone();
        assert_eq!(cloned.provider_id, "claude");
    }

    #[test]
    fn test_completed_flow_status_variants() {
        let completed = CompletedFlow {
            status: FlowStatus::Completed,
            account_id: Some("user@test.com".to_string()),
            error: None,
            completed_at: Utc::now(),
        };

        assert_eq!(completed.status, FlowStatus::Completed);
        assert!(completed.account_id.is_some());
        assert!(completed.error.is_none());
    }
}
