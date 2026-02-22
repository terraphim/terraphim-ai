# Terraphim LLM Proxy - AI Agent Reference

**Format Version:** 1.0
**Last Updated:** 2026-01-12
**Target Audience:** AI coding agents (Claude Code, Cursor, Copilot)

---

## Purpose

This document provides structured, machine-parseable information for AI agents working on this codebase. It contains:
- Precise file locations and line references
- API signatures for key functions
- Configuration schemas
- Test coverage mapping
- Issue/commit traceability

---

## Codebase Structure

```
src/
├── config.rs              # Configuration types and loading
├── server.rs              # HTTP server and request handling
├── router.rs              # 6-phase routing logic
├── routing/
│   ├── mod.rs             # Routing module exports
│   ├── aliasing.rs        # Model mapping/aliasing logic
│   ├── exclusion.rs       # Model exclusion patterns
│   └── strategy.rs        # Routing strategies
├── oauth/
│   ├── mod.rs             # OAuth module exports
│   ├── types.rs           # OAuthToken, OAuthProvider trait
│   ├── storage.rs         # FileTokenStorage implementation
│   ├── claude.rs          # ClaudeOAuthProvider
│   ├── gemini.rs          # GeminiOAuthProvider
│   ├── copilot.rs         # CopilotOAuthProvider (device flow)
│   └── callback.rs        # OAuth callback routes
├── management/
│   ├── mod.rs             # Management module exports
│   ├── auth.rs            # Management auth middleware
│   ├── error.rs           # ManagementError enum
│   ├── config_manager.rs  # ConfigManager with hot-reload
│   ├── routes.rs          # Route definitions
│   ├── config_handler.rs  # Config CRUD handlers
│   ├── keys_handler.rs    # API key handlers
│   └── logs_handler.rs    # Log control handlers
├── webhooks/
│   ├── mod.rs             # WebhookDispatcher
│   ├── events.rs          # WebhookEvent, WebhookEventType
│   └── signing.rs         # HMAC-SHA256 signing
└── openrouter_client.rs   # OpenRouter provider client
```

---

## Feature Implementation Map

### Model Aliasing (Issue #50)

**Status:** CLOSED
**Files:**
- `src/routing/aliasing.rs` - Core logic
- `src/config.rs` - ModelMapping struct in RouterSettings
- `src/server.rs` - apply_model_mappings() integration

**Key Types:**
```rust
// src/routing/aliasing.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMapping {
    pub from: String,       // Glob pattern to match
    pub to: String,         // Target "provider,model"
    #[serde(default)]
    pub bidirectional: bool,
}

// Key functions
pub fn resolve_model<'a>(
    requested: &str,
    mappings: &'a [ModelMapping],
) -> (String, Option<&'a ModelMapping>);

pub fn reverse_resolve<'a>(
    actual: &str,
    mappings: &'a [ModelMapping],
) -> Option<&'a str>;
```

**Tests:** `src/routing/aliasing.rs` (17 unit tests), `tests/model_mapping_integration_tests.rs` (8 integration tests)

**Commits:**
- `070fc27` - feat(anthropic): implement Phase 3 extended streaming features
- `7bde9f4` - fix(openrouter): update test to use new function signature
- `204fd19` - docs: update routing architecture with model mappings (v3.0)

---

### Model Exclusion (Issue #51)

**Status:** CLOSED
**Files:**
- `src/routing/exclusion.rs` - Exclusion logic

**Key Types:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelExclusion {
    pub provider: String,
    pub patterns: Vec<String>,
}

pub fn is_excluded(
    model: &str,
    exclusions: &[ModelExclusion],
    provider: &str,
) -> bool;
```

**Tests:** 7 unit tests in `src/routing/exclusion.rs`

---

### Routing Strategies (Issue #52)

**Status:** CLOSED
**Files:**
- `src/routing/strategy.rs` - Strategy implementations
- `src/router.rs` - Integration point

**Key Types:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RoutingStrategy {
    #[default]
    FillFirst,
    RoundRobin,
    LatencyOptimized,
    CostOptimized,
}

pub trait RoutingStrategyImpl: Send + Sync {
    fn select_provider<'a>(
        &self,
        candidates: &[&'a Provider],
        request: &ChatRequest,
        health_monitor: &ProviderHealthMonitor,
        state: &mut StrategyState,
    ) -> Result<&'a Provider>;
}
```

**Tests:** 6 unit tests in `src/routing/strategy.rs`

---

### Webhooks (Issue #53)

**Status:** CLOSED
**Files:**
- `src/webhooks/mod.rs` - WebhookDispatcher
- `src/webhooks/events.rs` - Event types
- `src/webhooks/signing.rs` - HMAC signing

**Key Types:**
```rust
// src/webhooks/events.rs
#[derive(Debug, Clone, Serialize)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub event_type: WebhookEventType,
    pub timestamp: DateTime<Utc>,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WebhookEventType {
    OAuthTokenRefreshed { provider: String, account_id: String },
    ProviderCircuitOpen { provider: String, reason: String },
    ProviderCircuitClosed { provider: String },
    QuotaExceeded { provider: String, model: String },
    ConfigUpdated { changed_sections: Vec<String> },
    ApiKeyRevoked { key_id: String },
}

// src/webhooks/signing.rs
pub fn sign_payload(payload: &[u8], secret: &str) -> String;
pub fn verify_signature(payload: &[u8], signature: &str, secret: &str) -> bool;

// src/webhooks/mod.rs
pub struct WebhookDispatcher {
    config: WebhookSettings,
    http_client: reqwest::Client,
}

impl WebhookDispatcher {
    pub fn new(config: WebhookSettings) -> Self;
    pub async fn dispatch(&self, event: WebhookEvent);
    pub fn is_enabled(&self, event_type: &WebhookEventType) -> bool;
}
```

**Commits:** `15d9124` - feat(webhooks): add webhook event system with HMAC signing

---

### OAuth Foundation (Issue #47)

**Status:** CLOSED
**Files:**
- `src/management/mod.rs`
- `src/management/auth.rs`
- `src/management/error.rs`

**Key Types:**
```rust
// src/management/auth.rs
pub async fn management_auth_middleware<B>(
    State(secret_hash): State<String>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, ManagementError>;

// src/management/error.rs
#[derive(Debug, Error)]
pub enum ManagementError {
    #[error("Authentication required")]
    Unauthorized,
    #[error("Invalid management secret")]
    InvalidSecret,
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
    #[error("Operation not allowed: {0}")]
    NotAllowed(String),
}
```

---

### Config Manager (Issue #48)

**Status:** CLOSED
**Files:**
- `src/management/config_manager.rs`
- `src/config.rs` (YAML support added)

**Key Types:**
```rust
pub struct ConfigManager {
    config: Arc<RwLock<ProxyConfig>>,
    file_path: PathBuf,
    on_change_callbacks: Vec<Box<dyn Fn(&ProxyConfig) + Send + Sync>>,
}

impl ConfigManager {
    pub fn new(file_path: PathBuf) -> Result<Self>;
    pub async fn get(&self) -> tokio::sync::RwLockReadGuard<'_, ProxyConfig>;
    pub async fn update(&self, new_config: ProxyConfig) -> Result<()>;
    pub async fn reload(&self) -> Result<()>;
    pub fn on_change<F>(&mut self, callback: F)
    where F: Fn(&ProxyConfig) + Send + Sync + 'static;
}

// Auto-detection
impl ProxyConfig {
    pub fn load_auto(path: &Path) -> Result<Self>;
    pub fn load_yaml(path: &Path) -> Result<Self>;
}
```

---

### Management Routes (Issue #49)

**Status:** CLOSED
**Files:**
- `src/management/routes.rs`
- `src/management/config_handler.rs`
- `src/management/keys_handler.rs`
- `src/management/logs_handler.rs`

**Endpoints:**
```
GET  /v0/management/config           → config_handler::get_config
PUT  /v0/management/config           → config_handler::update_config
POST /v0/management/config/reload    → config_handler::reload_config
GET  /v0/management/api-keys         → keys_handler::list_keys
POST /v0/management/api-keys         → keys_handler::create_key
DELETE /v0/management/api-keys/{id}  → keys_handler::delete_key
GET  /v0/management/logs/level       → logs_handler::get_level
PUT  /v0/management/logs/level       → logs_handler::set_level
GET  /v0/management/logs             → logs_handler::get_logs
GET  /v0/management/health           → health_handler::detailed_health
GET  /v0/management/metrics          → metrics_handler::get_metrics
```

---

### OAuth Providers (Issues #43-46)

**Status:** ALL CLOSED

| Provider | File | Trait |
|----------|------|-------|
| Claude | `src/oauth/claude.rs` | `OAuthProvider` |
| Gemini | `src/oauth/gemini.rs` | `OAuthProvider` |
| Copilot | `src/oauth/copilot.rs` | `OAuthProvider` (device flow) |
| Callback | `src/oauth/callback.rs` | HTTP handlers |

**Common Trait:**
```rust
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn display_name(&self) -> &str;
    async fn start_auth(&self, callback_port: u16) -> Result<(String, AuthFlowState)>;
    async fn exchange_code(&self, code: &str, state: &AuthFlowState) -> Result<OAuthToken>;
    async fn refresh_token(&self, token: &OAuthToken) -> Result<OAuthToken>;
}
```

---

## Configuration Schema

### RouterSettings (src/config.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouterSettings {
    pub default: String,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub think: Option<String>,
    #[serde(default)]
    pub plan_implementation: Option<String>,
    #[serde(default)]
    pub long_context: Option<String>,
    #[serde(default = "default_long_context_threshold")]
    pub long_context_threshold: usize,
    #[serde(default)]
    pub web_search: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub model_mappings: Vec<ModelMapping>,
    #[serde(default)]
    pub model_exclusions: Vec<ModelExclusion>,
    #[serde(default)]
    pub strategy: RoutingStrategy,
}
```

### WebhookSettings (src/webhooks/mod.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSettings {
    #[serde(default)]
    pub enabled: bool,
    pub url: Option<String>,
    pub secret: Option<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default = "default_retry")]
    pub retry_count: u32,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}
```

---

## Test Coverage Map

### Unit Tests

| Module | File | Test Count | Key Tests |
|--------|------|------------|-----------|
| Model Mapping | `src/routing/aliasing.rs` | 17 | `test_model_mapping_exact_match`, `test_model_mapping_glob_star`, `test_model_mapping_bidirectional` |
| Model Exclusion | `src/routing/exclusion.rs` | 7 | `test_exclusion_wildcard_prefix`, `test_exclusion_provider_scoped` |
| Routing Strategy | `src/routing/strategy.rs` | 6 | `test_fill_first_order`, `test_round_robin_distribution` |
| Webhooks | `src/webhooks/` | 6 | `test_hmac_signing`, `test_webhook_event_filter` |
| Management Auth | `src/management/auth.rs` | 5 | `test_management_auth_valid_header`, `test_secret_hashing` |
| Config Manager | `src/management/config_manager.rs` | 7 | `test_config_manager_update`, `test_config_yaml_loading` |
| OpenRouter Client | `src/openrouter_client.rs` | 9 | `test_convert_to_openrouter_request_basic`, `test_convert_to_openrouter_request_different_models` |

### Integration Tests

| File | Test Count | Purpose |
|------|------------|---------|
| `tests/model_mapping_integration_tests.rs` | 8 | End-to-end model mapping |
| `tests/management_api_tests.rs` | ~10 | Management API flows |
| `tests/oauth_integration_tests.rs` | ~8 | OAuth provider flows |

**Total:** 446 tests (438 unit + 8 integration)

---

## Routing Flow Reference

### Request Processing Order

```
1. Request received at server.rs
2. apply_model_mappings() - Model alias resolution
   → If match: model becomes "provider,model" format
3. Router Phase 0: Explicit provider check
   → Detects ":" or "," separator
   → If found: route directly to provider
4. Router Phase 1: Pattern-based routing
5. Router Phase 2: Session-aware pattern routing
6. Router Phase 3: Cost optimization
7. Router Phase 4: Performance optimization
8. Router Phase 5: Scenario-based fallback
   → Phases 1-5 only run if no explicit provider
9. Provider client sends request
10. Response processing
   → Bidirectional mapping applied if configured
```

### Key Code Paths

**Model Mapping Application:**
- Entry: `src/server.rs:apply_model_mappings()`
- Core: `src/routing/aliasing.rs:resolve_model()`

**Explicit Provider Parsing:**
- Entry: `src/router.rs:parse_explicit_provider_model()`
- Supports: `:` and `,` separators

**Semantic Routing:**
- Entry: `src/router.rs:route_by_taxonomy()`
- Config: `[router] think = "...", plan_implementation = "...", long_context = "...", web_search = "...", image = "..."`

---

## Issue Traceability

| Issue | Title | Phase | Status | Key Commits |
|-------|-------|-------|--------|-------------|
| #50 | Model Aliasing | 3 | CLOSED | `070fc27`, `7bde9f4`, `204fd19` |
| #51 | Model Exclusion | 3 | CLOSED | - |
| #52 | Routing Strategies | 3 | CLOSED | - |
| #53 | Webhooks | 3 | CLOSED | `15d9124` |
| #49 | Management Routes | 2 | CLOSED | - |
| #48 | Config Manager | 2 | CLOSED | - |
| #47 | Management Foundation | 2 | CLOSED | - |
| #46 | Copilot OAuth | 1 | CLOSED | - |
| #45 | Gemini OAuth | 1 | CLOSED | - |
| #44 | OAuth Callback | 1 | CLOSED | - |
| #43 | Claude OAuth | 1 | CLOSED | - |

---

## V-Model Documentation

For detailed development artifacts, see `.docs/`:

| Document | Purpose |
|----------|---------|
| `.docs/research-*.md` | Phase 1 research documents |
| `.docs/design-*.md` | Phase 2 implementation plans |
| `.docs/verification-*.md` | Phase 4 test traceability |
| `.docs/validation-*.md` | Phase 5 acceptance testing |

---

## Common Agent Tasks

### Adding a New Model Mapping

1. Edit config file, add to `[[router.model_mappings]]`
2. Ensure target model is in provider's `models` list
3. Test with curl or integration test
4. No code changes required

### Adding a New Webhook Event

1. Add variant to `WebhookEventType` enum in `src/webhooks/events.rs`
2. Add dispatch call at appropriate location
3. Add to `events` config option documentation
4. Add unit test for new event

### Adding a New OAuth Provider

1. Create `src/oauth/{provider}.rs`
2. Implement `OAuthProvider` trait
3. Register in `src/oauth/mod.rs`
4. Add callback route handling if browser-based
5. Add configuration section

### Modifying Routing Logic

1. Review 6-phase routing in `src/router.rs`
2. Model mappings: `src/routing/aliasing.rs`
3. Exclusions: `src/routing/exclusion.rs`
4. Strategies: `src/routing/strategy.rs`
5. Run full test suite: `cargo test --all-features`

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `glob-match` | 0.2 | Pattern matching for model mappings/exclusions |
| `oauth2` | 4.4 | OAuth2 client flows |
| `argon2` | 0.5 | Password/secret hashing |
| `hmac` | 0.12 | Webhook signing |
| `sha2` | 0.10 | SHA-256 for HMAC |
| `serde_yaml` | 0.9 | YAML config support |
| `redis` | 0.24 | Optional token storage |

---

## Performance Targets

| Operation | Target | Actual | Method |
|-----------|--------|--------|--------|
| Model mapping lookup | < 1ms | < 0.1ms | Linear scan with glob-match |
| Config reload | < 100ms | ~10ms | TOML/YAML parsing |
| Webhook delivery | < 5s | ~1s | Async HTTP with retry |
| Memory per mapping | < 200B | ~154B | Static analysis |

---

## Error Handling Patterns

### ManagementError

Used for all management API errors:
```rust
ManagementError::Unauthorized      // 401
ManagementError::InvalidSecret     // 403
ManagementError::ValidationError   // 400
ManagementError::NotAllowed        // 403
```

### Routing Errors

Model not found → falls through to next phase
Provider unhealthy → skipped by strategy
All providers fail → returns error to client

### OAuth Errors

Partial state → discarded (fresh start recovery)
Token expired → automatic refresh attempt
Refresh fails → requires re-authentication
