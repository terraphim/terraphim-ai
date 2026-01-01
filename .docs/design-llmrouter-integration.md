# Implementation Plan: LLM Router Integration for Terraphim AI

**Status**: Draft
**Research Doc**: [.docs/research-llmrouter-integration.md](.docs/research-llmrouter-integration.md)
**Author**: AI Design Specialist
**Date**: 2026-01-01
**Estimated Effort**: 6-8 weeks (Phase 1-3)

## Overview

### Summary

Integrate the existing, production-ready `terraphim-llm-proxy` (Phase 2 Week 1 COMPLETE, 186 tests, 0.21ms overhead) into the main Terraphim AI codebase as a library, replacing static LLM model selection with intelligent 6-phase routing.

### Approach

**Library Integration Strategy (Option 2 from Research)**

Instead of building new routing logic or integrating external LLMRouter, leverage the existing sophisticated LLM proxy implementation:

1. **Phase 1**: External service mode (minimal changes, prove value)
2. **Phase 2**: Library integration (in-process routing, full benefits)
3. **Phase 3**: Advanced features (cost optimization, performance metrics)

This approach provides:
- ✅ **Immediate value** with minimal risk
- ✅ **Zero network overhead** (in-process routing)
- ✅ **Proven code** (186 tests, production-ready)
- ✅ **Terraphim-specific** (RoleGraph integration)
- ✅ **Consistent architecture** (native Rust)

### Scope

**In Scope:**
- Add `terraphim_llm_proxy` as workspace dependency
- Create adapter layer between `LlmClient` trait and proxy routing
- Implement external service mode (HTTP proxy on port 3456)
- Implement library integration mode (in-process routing)
- Merge configuration (proxy TOML + Role extra fields)
- Unify session management
- Add routing transparency (logs, API responses)
- Maintain backward compatibility with static model selection
- Comprehensive testing (reuse proxy's 186 tests + integration tests)

**Out of Scope:**
- Multi-modal routing beyond image scenario (proxy handles images)
- Reinforcement learning routing
- Custom plugin system (proxy already extensible)
- Web UI for routing configuration (use existing TOML)
- Port conflicts resolution (proxy runs on 3456, external mode)

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Terraphim AI Main                     │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  terraphim_config                                 │    │
│  │  └─ Role {                                       │    │
│  │       ├─ llm_enabled: bool                         │    │
│  │       ├─ llm_model: Option<String> (static)          │    │
│  │       ├─ llm_router_enabled: bool (NEW)              │    │
│  │       ├─ llm_router_config: RouterConfig (NEW)       │    │
│  │       └─ extra: HashMap (existing)                  │    │
│  └────────────────────────────────────────────────────────┘    │
│                          ↓                                     │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  terraphim_service::llm                           │    │
│  │  └─ LlmClient::build_llm_from_role()        │    │
│  │     │                                               │    │
│  │     ├─ Check llm_router_enabled flag             │    │
│  │     ├─ Build static router if disabled               │    │
│  │     └─ Build intelligent router if enabled        │    │
│  │           │                                       │    │
│  │           ├─ Library Mode:                         │    │
│  │           │  terraphim_llm_proxy               │    │
│  │           │  └─ RouterAgent::route()              │    │
│  │           │     ↓                                  │    │
│  │           │  6-Phase Routing                     │    │
│  │           │     ↓                                  │    │
│  │           │  terraphim_multi_agent::GenAiLlmClient │    │
│  │           └─ genai 0.4 exec_chat()           │    │
│  │           ↓                                        │    │
│  │           ├─ Provider Selection                     │    │
│  │           └─ LLM API Call                      │    │
│  │                                                  │    │
│  │           └─ Service Mode (fallback):              │    │
│  │               ↓                                  │    │
│  │               HTTP to proxy:3456              │    │
│  └────────────────────────────────────────────────────────┘    │
│                          ↓                                     │
│              LLM Providers                              │
│  (OpenRouter, Ollama, Anthropic, etc.)               │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

**Phase 1: External Service Mode**

```
Client Request
    ↓
Terraphim AI Main (terraphim_server)
    ↓
LlmClient::build_llm_from_role()
    ↓
Check: llm_router_enabled?
    ├─ YES → HTTP to terraphim-llm-proxy:3456
    │     ↓
    │  Proxy: 6-Phase Routing (0.21ms)
    │     ↓
    │  LLM Provider (genai 0.4)
    │     ↓
    │  Response via SSE stream
    └─ NO → Static routing (existing behavior)
          ↓
          Direct LlmClient construction
```

**Phase 2: Library Integration Mode**

```
Client Request
    ↓
Terraphim AI Main (terraphim_service)
    ↓
LlmClient::build_llm_from_role()
    ↓
Check: llm_router_enabled?
    ├─ YES → RoutedLlmClient (wrapper)
    │     ↓
    │  terraphim_llm_proxy::RouterAgent
    │     ↓
    │  6-Phase Routing (in-process, <1ms)
    │     ├── Phase 0: Explicit provider
    │     ├── Phase 1: Pattern matching (RoleGraph)
    │     ├── Phase 2: Session-aware routing
    │     ├── Phase 3: Cost optimization
    │     ├── Phase 4: Performance optimization
    │     └── Phase 5: Scenario fallback
    │     ↓
    │  Provider Selection (RoutingDecision)
    │     ↓
    │  ProviderTransformers
    │     ↓
    │  GenAiLlmClient (genai 0.4)
    │     ↓
    │  LLM Provider (API call)
    │     ↓
    │  Response (SSE or JSON)
    └─ NO → Static routing (existing behavior)
          ↓
          Direct LlmClient construction
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| **Library Integration vs External Service** | In-process routing eliminates network overhead, single deployment unit, unified architecture | External service (adds 50ms+ network hop, distributed complexity) |
| **Reuse Existing Proxy** | Proven code (186 tests), production-ready, Terraphim-specific (RoleGraph), eliminates development risk | Build from scratch (months of work), integrate external LLMRouter (Python bridge overhead) |
| **Adapter Pattern** | Clean separation, maintains existing `LlmClient` interface, easy to disable/enable | Direct modification of `LlmClient` implementations (breaking changes) |
| **Backward Compatibility** | Existing roles with static `llm_model` still work, gradual migration | Breaking change (all roles require config update) |
| **Phase 1+2 Delivery** | Prove value incrementally, reduce risk, clear rollback path | Full library integration upfront (big bang, higher risk) |
| **Reuse Proxy Tests** | 186 tests already pass, no rewriting, focus on integration | Rewrite all tests (massive effort, risk of bugs) |
| **Feature Flag Control** | Users can disable routing if issues, no breaking changes | Force routing on all roles (risk, no fallback) |

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_service/src/llm/routed_adapter.rs` | Adapter wrapping proxy routing for `LlmClient` trait |
| `crates/terraphim_service/src/llm/proxy_client.rs` | External service mode HTTP client |
| `crates/terraphim_service/src/llm/router_config.rs` | Configuration merging (proxy TOML + Role extra) |
| `crates/terraphim_config/src/llm_router.rs` | Router configuration types for Role |
| `tests/integration/llm_router_integration_test.rs` | End-to-end integration tests |
| `docs/LLM_ROUTER_INTEGRATION.md` | User-facing integration guide |

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `terraphim_llm_proxy` workspace dependency |
| `crates/terraphim_service/Cargo.toml` | Add proxy dependency, feature flag `llm_router` |
| `crates/terraphim_config/Cargo.toml` | Add `terraphim_llm_proxy` config types dependency |
| `crates/terraphim_service/src/llm.rs` | Modify `build_llm_from_role()`, add router detection |
| `crates/terraphim_config/src/lib.rs` | Add router config fields to `Role` struct |
| `terraphim_server/src/api.rs` | Add routing decision to API responses (if requested) |
| `README.md` | Add LLM routing section, examples |

### Dependency Changes

**New Workspace Dependency:**

```toml
# Cargo.toml
[workspace.dependencies]
# Existing dependencies remain...
terraphim_llm_proxy = { path = "../terraphim-llm-proxy" }
```

**Conditional Features:**

```toml
# crates/terraphim_service/Cargo.toml
[features]
default = ["llm_router"]
llm_router = ["terraphim_llm_proxy"]
```

**Proxy Dependencies (inherited):**

| Crate | Version | Justification |
|--------|---------|---------------|
| `tiktoken-rs` | 0.5 | Token counting (2.8M tokens/sec) |
| `aho-corasick` | 1.1 | Pattern matching (200+ patterns) |
| `genai` | git fork | Already in workspace, compatible |
| `tokio` | 1.0 | Already in workspace |

## API Design

### Public Types

```rust
/// Router configuration from Role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRouterConfig {
    /// Enable intelligent routing (default: true)
    pub enabled: bool,

    /// Routing mode: "library" (in-process) or "service" (HTTP proxy)
    pub mode: RouterMode,

    /// Proxy URL for service mode (default: http://127.0.0.1:3456)
    pub proxy_url: Option<String>,

    /// Taxonomy path for pattern-based routing (default: docs/taxonomy)
    pub taxonomy_path: Option<String>,

    /// Enable cost optimization phase
    pub cost_optimization_enabled: bool,

    /// Enable performance optimization phase
    pub performance_optimization_enabled: bool,

    /// Routing strategy preference
    pub strategy: RouterStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouterMode {
    /// In-process library routing (fast, single deployment)
    #[serde(rename = "library")]
    Library,

    /// External HTTP service (slower, separate deployment)
    #[serde(rename = "service")]
    Service,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouterStrategy {
    /// Cost-first optimization
    #[serde(rename = "cost_first")]
    CostFirst,

    /// Quality-first (performance metrics)
    #[serde(rename = "quality_first")]
    QualityFirst,

    /// Balanced (cost + quality)
    #[serde(rename = "balanced")]
    Balanced,

    /// Static model selection (backward compatibility)
    #[serde(rename = "static")]
    Static,
}
```

### Public Functions

```rust
/// Build LLM client with optional intelligent routing
///
/// # Arguments
/// * `role` - Role configuration with router settings
///
/// # Returns
/// LLM client with routing if enabled, static client if disabled
///
/// # Routing Behavior
/// If `role.llm_router_enabled == true`:
///   - Library mode: In-process 6-phase routing (<1ms)
///   - Service mode: HTTP proxy (additional ~5ms network hop)
///
/// If `role.llm_router_enabled == false` or config missing:
///   - Fallback to existing static model selection
///   - Uses `role.llm_model` field
///
/// # Examples
/// ```rust,no_run
/// use terraphim_service::llm::build_llm_from_role;
/// use terraphim_config::Role;
///
/// let role = Role { /* ... */ };
/// let client = build_llm_from_role(&role)?;
///
/// // Client uses intelligent routing if enabled
/// let summary = client.summarize(content, opts).await?;
/// ```
pub fn build_llm_from_role(role: &Role) -> Option<Arc<dyn LlmClient>> {
    // Existing implementation with router detection
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum LlmRouterError {
    #[error("Router not enabled: {0}")]
    NotEnabled(String),

    #[error("Invalid router mode: {0}")]
    InvalidMode(String),

    #[error("Router configuration error: {0}")]
    ConfigError(String),

    #[error("Proxy connection failed: {0}")]
    ProxyConnectionError(String),

    #[error("Proxy request failed: {0}")]
    ProxyRequestError(String),

    #[error(transparent)]
    TerraphimProxy(#[from] terraphim_llm_proxy::ProxyError),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}
```

## Test Strategy

### Unit Tests

Reuse Proxy's 186 Tests (No Rewrite)

| Test | Location | Purpose |
|------|----------|---------|
| `router::tests::test_pattern_matching_routing` | Proxy tests | Existing (proxy/src/router.rs) |
| `router::tests::test_route_default_scenario` | Proxy tests | Existing |
| `cost::tests::test_cost_calculation` | Proxy tests | Existing |
| `rolegraph_client::tests::test_pattern_matching` | Proxy tests | Existing |

**New Integration Tests:**

| Test | Location | Purpose |
|------|----------|---------|
| `test_router_adapter_construction` | `llm/routed_adapter.rs` | Verify adapter builds correctly |
| `test_library_mode_routing` | Integration tests | In-process routing works |
| `test_service_mode_routing` | Integration tests | HTTP proxy mode works |
| `test_backward_compatibility` | Integration tests | Static model selection still works |
| `test_configuration_merging` | Integration tests | Role config merges correctly |
| `test_routing_transparency` | Integration tests | Routing decisions visible |
| `test_feature_flag_disabling` | Integration tests | Can disable routing |

### Integration Tests

```rust
#[tokio::test]
async fn test_library_mode_routing() {
    // Given: Role with routing enabled, library mode
    let role = create_test_role_with_routing(RouterMode::Library);

    // When: Build LLM client
    let client = build_llm_from_role(&role).unwrap();

    // Then: Should be routed client
    assert!(client.is::<RoutedLlmClient>());

    // When: Send request
    let request = LlmRequest::new(messages);
    let response = client.generate(request).await.unwrap();

    // Then: Should route intelligently
    assert_eq!(response.model, "deepseek-chat"); // Example pattern match
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Given: Role with static model (no routing)
    let role = create_test_role_static_model();

    // When: Build LLM client
    let client = build_llm_from_role(&role).unwrap();

    // Then: Should be existing static client
    assert!(client.is::<OpenRouterClient>());
}

#[tokio::test]
async fn test_service_mode_routing() {
    // Given: Role with routing enabled, service mode
    let role = create_test_role_with_routing(RouterMode::Service);

    // When: Build LLM client and send request
    let client = build_llm_from_role(&role).unwrap();

    // Then: Should route via HTTP proxy
    let response = client.generate(request).await.unwrap();

    // Then: Should receive response
    assert!(!response.content.is_empty());
}
```

### Performance Tests

```rust
#[bench]
fn bench_router_adapter_overhead(b: &mut Bencher) {
    let adapter = RoutedLlmClient::new(config);

    b.iter(|| {
        // Measure routing decision time
        let _decision = adapter.route_request(test_query());
    });
}

// Target: <1ms (proxy achieves 0.21ms)
```

## Implementation Steps

### Step 1: Workspace Integration (2 days)

**Files:** `Cargo.toml`, `crates/terraphim_service/Cargo.toml`
**Description:** Add proxy as workspace dependency, configure feature flags
**Tests:** Compilation succeeds
**Dependencies:** None

```bash
# Add to Cargo.toml workspace dependencies
terraphim_llm_proxy = { path = "../terraphim-llm-proxy", optional = true }

# Add feature flag
[features]
default = ["llm_router"]
llm_router = ["terraphim_llm_proxy"]

# Add to terraphim_service/Cargo.toml
[dependencies]
terraphim_llm_proxy = { workspace = true, optional = true }
```

### Step 2: Configuration Types (1 day)

**Files:** `crates/terraphim_config/src/llm_router.rs`, `crates/terraphim_config/src/lib.rs`
**Description:** Add router configuration types to Role struct
**Tests:** Unit tests for config construction and validation
**Dependencies:** Step 1

```rust
// Add to Role struct
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Role {
    // ... existing fields

    /// NEW: Enable intelligent LLM routing
    #[serde(default)]
    pub llm_router_enabled: bool,

    /// NEW: Router configuration
    #[serde(default)]
    pub llm_router_config: Option<LlmRouterConfig>,
}
```

### Step 3: Adapter Layer - Library Mode (3 days)

**Files:** `crates/terraphim_service/src/llm/routed_adapter.rs`
**Description:** Create adapter wrapping proxy routing for LlmClient trait
**Tests:** Unit tests for all routing scenarios
**Dependencies:** Steps 1-2

```rust
pub struct RoutedLlmClient {
    router: RouterAgent,
    base_client: GenAiLlmClient,
}

impl LlmClient for RoutedLlmClient {
    fn name(&self) -> &'static str {
        "routed_llm" // or actual selected provider
    }

    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> Result<String> {
        // Generate routing hints
        let hints = self.analyze_request(content);

        // Route to optimal provider/model
        let decision = self.router.route(request, &hints).await?;

        // Call LLM via genai
        let response = self.base_client.generate(request).await?;

        Ok(response.content)
    }
}
```

### Step 4: Adapter Layer - Service Mode (2 days)

**Files:** `crates/terraphim_service/src/llm/proxy_client.rs`
**Description:** Create HTTP client for external proxy mode
**Tests:** Integration tests with proxy running on port 3456
**Dependencies:** Steps 1-3

```rust
pub struct ProxyLlmClient {
    proxy_url: String,
    http: reqwest::Client,
}

impl LlmClient for ProxyLlmClient {
    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> Result<String> {
        let body = serde_json::json!({
            "model": "auto", // Let proxy route
            "messages": [{"role": "user", "content": content}]
        });

        let response = self.http
            .post(&format!("{}/v1/messages", self.proxy_url))
            .json(&body)
            .send()
            .await?;

        Ok(response.json()?["summary"].as_str()?.to_string())
    }
}
```

### Step 5: Integration Point (2 days)

**Files:** `crates/terraphim_service/src/llm.rs`
**Description:** Modify `build_llm_from_role()` to detect routing config and select appropriate client
**Tests:** Integration tests for all combinations (enabled/disabled, library/service modes)
**Dependencies:** Steps 1-4

```rust
pub fn build_llm_from_role(role: &Role) -> Option<Arc<dyn LlmClient>> {
    // Check for router configuration
    let router_config = role.llm_router_config.as_ref();

    match router_config {
        Some(config) if config.enabled => {
            // Router enabled
            match config.mode {
                RouterMode::Library => {
                    // In-process routing (fast)
                    build_routed_client(role, config)
                }
                RouterMode::Service => {
                    // External HTTP proxy
                    build_proxy_client(role, config.proxy_url?)
                }
            }
        }
        _ => {
            // No router config or disabled - use static routing
            build_static_client(role)
        }
    }
}
```

### Step 6: Configuration Merging (1 day)

**Files:** `crates/terraphim_service/src/llm/router_config.rs`
**Description:** Implement merging of Role extra fields with proxy configuration
**Tests:** Unit tests for config merging logic
**Dependencies:** Steps 1-2

```rust
impl LlmRouterConfig {
    pub fn merge_with_role(&self, role: &Role) -> Self {
        let mut merged = self.clone();

        // Override from Role extra
        if let Some(proxy_url) = role.extra.get("proxy_url").and_then(|v| v.as_str()) {
            merged.proxy_url = Some(proxy_url.to_string());
        }

        if let Some(strategy) = role.extra.get("router_strategy").and_then(|v| v.as_str()) {
            merged.strategy = serde_json::from_str(strategy).unwrap_or(RouterStrategy::Balanced);
        }

        merged
    }
}
```

### Step 7: Integration Tests (3 days)

**Files:** `tests/integration/llm_router_integration_test.rs`
**Description:** End-to-end tests for routing with real LLM providers
**Tests:** All integration scenarios, backward compatibility
**Dependencies:** Steps 1-6

```rust
#[tokio::test]
async fn test_e2e_library_mode_routing() {
    // Start mock LLM server
    let server = start_mock_llm_server().await;

    // Create role with routing enabled
    let role = Role {
        llm_router_enabled: true,
        llm_router_config: Some(LlmRouterConfig {
            mode: RouterMode::Library,
            enabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    // Build client
    let client = build_llm_from_role(&role).unwrap();

    // Send request
    let response = client.summarize("test content", SummarizeOptions::default()).await;

    // Verify routing occurred
    assert!(response.is_ok());
}
```

### Step 8: Documentation (2 days)

**Files:** `docs/LLM_ROUTER_INTEGRATION.md`, inline docs, examples
**Description:** User-facing documentation, configuration examples, troubleshooting
**Tests:** Doc tests compile and run
**Dependencies:** Steps 1-7

```markdown
# LLM Router Integration Guide

## Enabling Intelligent Routing

Add to your Role configuration:

```json
{
  "llm_router_enabled": true,
  "llm_router_config": {
    "mode": "library",
    "enabled": true,
    "strategy": "balanced"
  }
}
```

## Routing Modes

### Library Mode (Recommended)
- **Performance**: <1ms overhead, in-process routing
- **Use Case**: Production deployments, single binary
- **Configuration**: Set `mode: "library"`

### Service Mode
- **Performance**: ~5ms overhead (network hop)
- **Use Case**: Separate proxy deployment, development
- **Configuration**: Set `mode: "service"`, provide `proxy_url`
```

### Step 9: Backward Compatibility (1 day)

**Files:** `crates/terraphim_service/src/llm.rs`
**Description:** Ensure existing roles with static `llm_model` continue to work
**Tests:** Backward compatibility integration tests
**Dependencies:** Steps 1-8

```rust
pub fn build_llm_from_role(role: &Role) -> Option<Arc<dyn LlmClient>> {
    // Check for router config
    if let Some(router_config) = role.llm_router_config.as_ref() {
        if router_config.enabled {
            // Use intelligent routing
            return Some(build_routed_client(role, router_config));
        }
    }

    // Fallback: Check for static model
    if let Some(model_name) = role.llm_model.as_ref() {
        log::info!("Using static model selection: {}", model_name);
        return Some(build_static_client(role, model_name));
    }

    log::debug!("No LLM configuration for role: {}", role.name);
    None
}
```

### Step 10: Production Readiness (2 days)

**Files:** All modified files, `README.md`, integration tests
**Description:** Zero warnings, clippy passes, benchmarks, security audit
**Tests:** Full test suite, performance benchmarks
**Dependencies:** Steps 1-9

**Tasks:**
1. Run `cargo clippy -- -D warnings`
2. Fix all clippy warnings
3. Run `cargo fmt`
4. Run performance benchmarks
5. Test with real LLM providers
6. Check for security issues (API keys, secrets)

## Rollback Plan

If issues discovered:

1. **Feature Flag Rollback**
   ```bash
   # Disable routing globally
   # In Role config: set llm_router_enabled = false
   ```

2. **Library Mode Rollback**
   ```bash
   # Keep service mode (external proxy) while fixing library integration
   ```

3. **Complete Rollback**
   ```bash
   git revert <integration-commit>
   # All changes are behind feature flag
   ```

Feature Flag: `llm_router` (can be disabled in Cargo.toml or at runtime)

## Migration (No Database Changes)

**Configuration Migration:**

Existing roles without routing config:
```json
{
  "llm_model": "anthropic/claude-sonnet-4.5"
}
```

**Migrate to:**
```json
{
  "llm_router_enabled": true,
  "llm_router_config": {
    "mode": "library",
    "enabled": true,
    "strategy": "quality_first"
  }
}
```

**Backward Compatibility:**
- Existing `llm_model` field still respected if no router config
- No breaking changes for existing roles
- Gradual migration (can enable routing per role)

**Session Management:**

Use existing Terraphim session system:
```rust
// Library mode: Share session state
let session = terraphim_sessions::get_or_create(session_id);

// Pass to proxy router
router.route_with_session(request, hints, session).await?;

// Service mode: HTTP includes session_id in request
proxy_url? + "?session_id=" + session_id
```

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|--------|---------|---------------|
| `terraphim_llm_proxy` | path | Production routing logic (186 tests) |
| `tiktoken-rs` | 0.5 | Token counting (via proxy) |

### Dependency Updates

| Crate | From | To | Reason |
|--------|------|-----|--------|
| `tokio` | 1.0 | 1.0 (unchanged) | Already in workspace |

### Optional Dependencies (Feature-Gated)

```toml
[dependencies.terraphim_llm_proxy]
# Only needed for service mode
axum = { version = "0.7", optional = true }

[features]
# Service mode requires axum for HTTP (optional)
service_mode = ["terraphim_llm_proxy/axum"]
```

## Performance Considerations

### Expected Performance

| Metric | Target | Proxy Baseline | Main Codebase Current |
|---------|--------|----------------|---------------------|
| Routing Overhead | <1ms | 0.21ms (library mode) | 0ms (no routing) |
| Request Throughput | >4,000 req/sec | >4,000 req/sec | Limited by LLM latency |
| Memory Footprint | <10MB additional | <2MB per request | <500MB per request |
| Cold Start Time | <100ms | <50ms (proxy startup) | 0ms |

### Benchmarks to Add

```rust
#[bench]
fn bench_library_mode_overhead(b: &mut Bencher) {
    let client = RoutedLlmClient::new(config);
    let request = test_request();

    b.iter(|| {
        let _ = client.route_request(&request);
    });
}

// Target: <1ms (proxy achieves 0.21ms)
```

### Performance Monitoring

Add to existing metrics:
```rust
// Track routing decisions
metrics::counter("llm_router_phase_usage", &["phase=pattern"]).increment(1);

// Track routing overhead
metrics::histogram("llm_router_overhead_ms", vec![]).record(overhead);

// Track cost savings
metrics::counter("llm_router_cost_saved_usd", vec![]).increment(savings);
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Define exact configuration merge strategy | Pending | Configuration team |
| Resolve session unification approach | Pending | Session team |
| Decide on service mode default (library vs service) | Pending | Product team |
| Cost tracking implementation details | Pending | Backend team |
| Performance benchmarking with real providers | Pending | QA team |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
- [ ] Feature flag strategy approved
- [ ] Rollback plan verified
- [ ] Migration plan reviewed

---

**Next Steps After Approval:**

1. Conduct specification interview (Phase 2.5) using `disciplined-specification` skill
2. Implement Step 1 (Workspace Integration)
3. Proceed with Step 2 (Configuration Types)
4. Continue through Steps 3-10
5. Each step includes tests before proceeding
6. Continuous code reviews and integration testing

**Estimated Timeline:**
- Week 1: Steps 1-5 (Workspace + Types + Adapters + Integration)
- Week 2: Steps 6-8 (Config + Tests + Docs)
- Week 3: Steps 9-10 (Compatibility + Production Ready)
- Week 4: Buffer, A/B testing, bug fixes

**Total Effort: 6-8 weeks (full delivery with buffer)**

---

## Specification Interview Findings

**Interview Date**: 2026-01-01
**Dimensions Covered**: All 10 (Concurrency, Failure Modes, Edge Cases, User Mental Models, Scale & Performance, Security & Privacy, Integration Effects, Migration & Compatibility, Accessibility & Internationalization, Operational Concerns)
**Convergence Status**: Complete

### Key Decisions from Interview

#### Concurrency & Race Conditions

**Q1.1: Multiple users, same pattern** → **Option A: Deterministic routing**
- Both users route to same provider/model for same pattern
- Easier debugging, consistent behavior
- Tradeoff: No load balancing across multiple good options

**Q1.2: User cancels during routing** → **Option A: Continue routing and cache result**
- Wastes computation on canceled request
- Populates cache for future similar requests
- Tradeoff: Resource waste for better cache hit rate

**Q1.3: Rapid-fire requests from same session** → **Option A: Cache routing decision with TTL**
- Cache routing decision per session
- TTL: ~5 minutes (configurable)
- Tradeoff: Doesn't adapt to changing conditions within cache window

#### Failure Modes & Recovery

**Q2.1: Provider returns 429 rate limit** → **Option B: Fallback to Phase 2**
- Graceful degradation through routing phases
- Tries next routing phase instead of immediate failure
- Tradeoff: Slower fallback but user gets response

**Q2.2: Proxy routing logic panics** → **Option B: Fallback to static LLM client**
- Adapter layer catches panic, uses existing static routing
- Maximum resilience, existing behavior maintained
- Tradeoff: No routing during that request (acceptable fallback)

**Q2.3: Optimization phases enabled but no data** → **Option B: Use default values**
- Use average pricing, 50th percentile performance
- Proactively enable phases with reasonable defaults
- Tradeoff: May not be optimal initially, but works

#### Edge Cases & Boundaries

**Q3.1: Explicit provider not configured** → **Option A: Return 400 Bad Request**
- Clear error message with configured providers list
- Fail fast, user knows exactly what to fix
- Tradeoff: No fallback, but explicit failure

**Q3.2: Multiple equal-scoring matches** → **Option B: Select higher priority**
- Use priority metadata from taxonomy files
- Deterministic tie-breaking
- Tradeoff: Requires priority configuration in patterns

**Q3.3: Request exceeds all context limits** → **Option B: Return error, ask to reduce**
- Clear error message about size limits
- User action required to proceed
- Tradeoff: Poor UX but prevents silent truncation/failure

#### User Mental Models

**Q4.1: Unexpected model selected** → **Option C: API metadata with routing reason**
- Return in response: `{"model": "deepseek-chat", "routing_reason": "Pattern matched: low_cost_routing (priority: 50)"}`
- API-level transparency, developers can expose in UI if desired
- Tradeoff: More response data, but full transparency

**Q4.2: User feedback on routing quality** → **Option C: Adjust routing weights + User-editable KG**
- Leverage Terraphim Knowledge Graph for routing patterns
- Users can edit/create taxonomy files (Terraphim infrastructure)
- Global routing adjustments based on user feedback
- Tradeoff: Learning complexity, but powerful customization

**Q4.3: Terminology** → **Options A + D: Dual terminology**
- User-facing/marketing: "Intelligent Routing"
- Technical/developer: "Dynamic Routing"
- Clear audience segmentation
- Tradeoff: Two terms to maintain, but clearer communication

#### Scale & Performance

**Q5.1: Acceptable routing overhead** → **Option D: <10ms maximum**
- Very lenient target
- Routing overhead negligible compared to 500-5000ms LLM latency
- Tradeoff: May not require heavy optimization

**Q5.2: Pattern database grows to 2000+** → **Options C + A: Hybrid approach**
- Implement pattern hierarchy using Terraphim KG
- Keep Aho-Corasick for active set (categorization)
- Criterion benchmarks: <1% difference between 200 and 2000 patterns
- Leverage `terraphim-automata` crate
- Tradeoff: More complex categorization, but proven scalability

**Q5.3: Request batching** → **Option C: No batching**
- Always process immediately
- No queuing or batching complexity
- Tradeoff: Missed cost optimization opportunity, but simpler

#### Security & Privacy

**Q6.1: Routing decision logging** → **Option C: Log full routing decision (security auditing)**
- Log provider, model, reasoning, phase used
- Full observability for security team
- Tradeoff: More logs, but necessary for auditing

**Q6.2: User lacks API keys for routed provider** → **Option B: Check credentials, skip routing decision**
- Validate user has keys before attempting routing
- Try next routing phase instead of failing
- Tradeoff: More validation overhead, but graceful fallback

**Q6.3: External service mode auth** → **Option B with fallback: User key first, then proxy key**
- User's original API key takes precedence
- Fallback to proxy's configured keys if user's key fails
- User control with safety net
- Tradeoff: More auth logic, but better UX

#### Integration Effects

**Q7.1: Session tracking of routing** → **Option A: Existing sessions store last routing decision**
- Add routing decision field to existing session objects
- Minimal change, leverage current infrastructure
- Tradeoff: Session object grows, but acceptable

**Q7.2: Existing features with routing** → **Option A: Always go through routing**
- Consistent behavior across all features
- All features benefit from intelligent routing
- Tradeoff: No bypass mechanism in current design

**Q7.3: Multiple roles with different strategies** → **Option A: Cached per-role**
- Each role maintains its own routing cache/state
- Respects different strategies across roles
- Tradeoff: More cache storage, but necessary

#### Migration & Compatibility

**Q8.1: Existing roles with static model** → **Option B: Auto-upgrade to intelligent routing**
- Existing roles automatically benefit from routing
- Previously configured model becomes fallback if routing fails
- Tradeoff: Automatic behavior change, but beneficial

**Q8.2: Proxy crashes, in-flight requests** → **Option B: Fallback to static LLM client**
- Graceful degradation, existing static routing continues
- Maximum resilience
- Tradeoff: May temporarily lose routing, but functionality preserved

**Q8.3: Rollback strategy** → **Option A: Runtime feature flag**
- `llm_router_enabled` can be set to false at runtime
- No code deploy needed for rollback
- Instant rollback capability
- Tradeoff: Need config reload mechanism

#### Accessibility & Internationalization

**Q9.1: Routing decision exposure for screen readers** → **Option B: API response metadata**
- Expose in API response (developers can read)
- Screen reader users can inspect via dev tools
- UI stays simple
- Tradeoff: Not directly in UI, but accessible

**Q9.2: Pattern language** → **Option C: Language hint metadata in taxonomy files**
- Patterns stay in English (global baseline)
- Taxonomy files include metadata for UI localization
- Routing layer unchanged
- Tradeoff: Pattern files more complex, but routing simple

#### Operational Readiness

**Q10.1: Metrics tracking** → **Option C: Comprehensive**
- Routing phase used, time spent routing
- Success rate per provider/model
- Cost savings (tracked vs baseline)
- Performance metrics (latency per provider)
- Fallback rates (Phase 5 usage)
- Tradeoff: Higher logging overhead, but full visibility

**Q10.2: Alerting on routing issues** → **Option D: Alert on high fallback rate**
- Alert if fallback rate (Phase 5) exceeds threshold (e.g., >20%)
- High fallback rate means earlier phases failing
- Specific alerting for routing problems
- Tradeoff: More alerting rules, but catches routing failures

**Q10.3: Debugging tools** → **Decision: Provide all options (A, B, C)**
- Structured logs with all phase results and scores
- Debug API endpoint to test routing without LLM calls
- Interactive routing explorer tool (CLI or web UI)
- Comprehensive debugging
- Tradeoff: More tooling to maintain

### Deferred Items

- **Reinforcement learning routing**: Defer to Phase 3 or future work
- **Automatic request batching**: Not included in current scope (user can batch via API)
- **Per-feature routing bypass**: All features go through routing (consistent design)
- **Multi-provider load balancing**: Deterministic routing prioritized for now

### Interview Summary

The specification interview successfully surfaced critical requirements and design decisions across 10 dimensions. Key findings that will significantly impact implementation:

**Most Critical Decisions:**
1. **Graceful degradation architecture**: Multiple fallback layers (routing phases → static client) ensure maximum resilience
2. **Terraphim KG integration**: User-editable routing patterns leverage existing infrastructure, powerful customization
3. **Runtime rollback**: Feature flag enables instant rollback without code deployment
4. **Comprehensive observability**: Full logging, metrics, and alerting for production confidence
5. **Session-aware caching**: Per-role and per-session caching balances performance with adaptability

**Architecture Implications:**
- Router must implement panic recovery to static client fallback
- Adapter layer needs credential validation before routing
- Session objects require routing decision field
- Taxonomy files need language hint metadata
- Metrics collection must track cost savings and fallback rates

**User Experience:**
- API-level routing transparency (metadata in responses)
- Clear error messages for edge cases (unconfigured providers, oversized requests)
- Dual terminology for audience segmentation
- User-editable patterns for personalization

**Operational Readiness:**
- <10ms routing overhead (very lenient target)
- Alerting on >20% fallback rate
- Debug API endpoint for routing testing
- Comprehensive logging with all phase decisions

The implementation plan is now complete with specific behavioral requirements for all edge cases and failure modes. No hidden assumptions remain that would require clarification during coding.
