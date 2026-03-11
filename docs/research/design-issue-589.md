# Implementation Plan: Issue #589 - Wire WebToolsConfig to Web Search/Fetch Tools

**Status**: Draft
**Research Doc**: `.docs/research-issue-589.md`
**Author**: Claude Code
**Date**: 2026-03-11
**Estimated Effort**: 4-6 hours

---

## Overview

### Summary
Wire the existing `WebToolsConfig` through to `WebSearchTool` and `WebFetchTool` so that configuration file settings for search provider and fetch mode are respected. Maintain backward compatibility with environment variable fallback.

### Approach
Add `from_config()` constructor methods to both tools that accept `&WebToolsConfig`, modify `create_default_registry()` to accept an optional config parameter, and update the call site in `main.rs` to pass the loaded configuration.

### Scope

**In Scope:**
1. Add `from_config()` to `WebSearchTool` with provider selection logic
2. Add `from_config()` to `WebFetchTool` with mode selection
3. Modify `create_default_registry()` signature to accept `Option<&WebToolsConfig>`
4. Update `main.rs` to pass config when creating the registry
5. Align provider names in config docs with implementation
6. Add unit tests for new methods
7. Add integration test for config wiring

**Out of Scope:**
- Adding new search providers (Brave, Searxng, Google)
- Hot-reloading of tool configuration
- Per-session tool configuration overrides
- Configuration validation beyond basic provider name matching

**Avoid At All Cost** (from 5/25 analysis):
- Breaking changes to existing `new()` constructors
- Complex provider registration/management systems
- Async initialization of tools (keep it simple/synchronous)
- Feature flags for provider selection (use config instead)

---

## Architecture

### Component Diagram

```
┌─────────────────┐     ┌─────────────────────┐     ┌──────────────────┐
│   Config File   │────▶│  Config::load()     │────▶│  main.rs         │
│   (web_tools)   │     │  (WebToolsConfig)   │     │  (pass to tools) │
└─────────────────┘     └─────────────────────┘     └────────┬─────────┘
                                                             │
                                                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    create_default_registry()                         │
│  ┌─────────────────────┐    ┌─────────────────────┐                 │
│  │ WebSearchTool       │    │ WebFetchTool        │                 │
│  │ ::from_config()     │    │ ::from_config()     │                 │
│  │                     │    │                     │                 │
│  │ - search_provider   │    │ - fetch_mode        │                 │
│  │ - env fallback      │    │ - env fallback      │                 │
│  └─────────────────────┘    └─────────────────────┘                 │
└─────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
main.rs
    │
    ├──▶ Load Config (from file + env)
    │
    ├──▶ create_default_registry(config.web_tools.as_ref())
    │        │
    │        ├──▶ WebSearchTool::from_config(config)
    │        │        ├──▶ Check config.search_provider
    │        │        ├──▶ Fall back to env vars (EXA_API_KEY, KIMI_API_KEY)
    │        │        └──▶ Return configured provider
    │        │
    │        └──▶ WebFetchTool::from_config(config)
    │                 ├──▶ Check config.fetch_mode
    │                 ├──▶ Fall back to "raw"
    │                 └──▶ Return configured tool
    │
    └──▶ Registry ready for use
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Keep `new()` as env-only fallback | Backward compatibility - existing code continues to work | Removing `new()` would break API |
| `from_config()` takes `Option<&WebToolsConfig>` | Handles case where config is not available | Requiring `&WebToolsConfig` would force caller to construct dummy config |
| Provider selection by name string | Simple matching, easy to extend | Enum-based selection would require config changes |
| Update config docs to match impl | Less code, less confusion than name mapping | Name mapping adds unnecessary complexity |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Provider name mapping (brave->exa) | Adds code complexity for little value | Maintenance burden, confusion about which names work |
| Async tool initialization | Tools don't need async for init | Unnecessary complexity, compat issues |
| Builder pattern for tools | Overkill for two config fields | API surface expansion, more code to maintain |
| Trait-based provider registration | Too complex for current needs | Premature abstraction |

### Simplicity Check

**What if this could be easy?**

The simplest design is:
1. Add a constructor that takes config
2. If config has a value, use it
3. Otherwise, fall back to env vars (existing behavior)
4. Update the registry creation to pass config through

This is exactly what this plan implements. No traits, no builders, no async - just straightforward wiring of existing config to existing tools.

---

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_tinyclaw/src/tools/web.rs` | Add `from_config()` to `WebSearchTool` and `WebFetchTool` |
| `crates/terraphim_tinyclaw/src/tools/mod.rs` | Update `create_default_registry()` signature |
| `crates/terraphim_tinyclaw/src/main.rs` | Pass config to `create_default_registry()` |
| `crates/terraphim_tinyclaw/src/config.rs` | Update WebToolsConfig doc comments for provider names |

### No New Files
This change modifies existing files only.

### No Deleted Files

---

## API Design

### Public Functions - WebSearchTool

```rust
impl WebSearchTool {
    /// Creates a new WebSearchTool from environment variables.
    ///
    /// Checks EXA_API_KEY and KIMI_API_KEY environment variables.
    /// If neither is set, returns a tool with a placeholder provider.
    pub fn new() -> Self;

    /// Creates a new WebSearchTool from configuration.
    ///
    /// If config specifies a search provider, uses it.
    /// Otherwise falls back to environment variables (same as `new()`).
    ///
    /// # Arguments
    /// * `config` - Optional web tools configuration
    ///
    /// # Supported Providers
    /// - "exa" - Exa search API
    /// - "kimi_search" - Kimi search API
    pub fn from_config(config: Option<&WebToolsConfig>) -> Self;
}
```

### Public Functions - WebFetchTool

```rust
impl WebFetchTool {
    /// Creates a new WebFetchTool with default settings.
    ///
    /// Defaults to "raw" fetch mode.
    pub fn new() -> Self;

    /// Creates a new WebFetchTool from configuration.
    ///
    /// If config specifies a fetch mode, uses it.
    /// Otherwise defaults to "raw".
    ///
    /// # Arguments
    /// * `config` - Optional web tools configuration
    ///
    /// # Supported Modes
    /// - "raw" - Fetch raw HTML
    /// - "readability" - Extract readable content
    pub fn from_config(config: Option<&WebToolsConfig>) -> Self;
}
```

### Modified Function - create_default_registry

```rust
/// Creates the default tool registry with all standard tools.
///
/// # Arguments
/// * `sessions` - Optional session manager for session-aware tools
/// * `web_tools_config` - Optional web tools configuration
pub fn create_default_registry(
    sessions: Option<Arc<tokio::sync::Mutex<SessionManager>>>,
    web_tools_config: Option<&WebToolsConfig>,
) -> ToolRegistry;
```

### Updated Config Documentation

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebToolsConfig {
    /// Web search provider ("exa", "kimi_search").
    ///
    /// If not specified, falls back to environment variables
    /// (EXA_API_KEY or KIMI_API_KEY).
    pub search_provider: Option<String>,

    /// Web fetch mode ("raw", "readability").
    ///
    /// Defaults to "raw" if not specified.
    pub fetch_mode: Option<String>,
}
```

---

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_web_search_from_config_exa` | `tools/web.rs` | Verify exa provider selection from config |
| `test_web_search_from_config_kimi` | `tools/web.rs` | Verify kimi_search provider selection from config |
| `test_web_search_from_config_fallback` | `tools/web.rs` | Verify env fallback when config is None |
| `test_web_search_from_config_unknown` | `tools/web.rs` | Verify behavior with unknown provider name |
| `test_web_fetch_from_config_raw` | `tools/web.rs` | Verify raw mode selection from config |
| `test_web_fetch_from_config_readability` | `tools/web.rs` | Verify readability mode selection from config |
| `test_web_fetch_from_config_fallback` | `tools/web.rs` | Verify default when config is None |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_config_wired_to_tools` | `tests/config_wiring.rs` | End-to-end test that config values reach tools |

```rust
// Integration test outline
#[test]
fn test_config_wired_to_tools() {
    // Create a config with specific values
    let config = WebToolsConfig {
        search_provider: Some("exa".to_string()),
        fetch_mode: Some("readability".to_string()),
    };

    // Create registry with config
    let registry = create_default_registry(None, Some(&config));

    // Verify tools were created with correct configuration
    // (This may require adding introspection methods or using mock providers)
}
```

### Doc Tests

Add doc test examples to `from_config()` methods showing usage.

---

## Implementation Steps

### Step 1: Update WebToolsConfig Documentation
**Files:** `crates/terraphim_tinyclaw/src/config.rs`
**Description:** Update doc comments to match actual provider names
**Tests:** N/A (documentation only)
**Estimated:** 15 minutes

```rust
// Change from:
/// Web search provider ("brave", "searxng", "google").

// To:
/// Web search provider ("exa", "kimi_search").
```

### Step 2: Add from_config() to WebSearchTool
**Files:** `crates/terraphim_tinyclaw/src/tools/web.rs`
**Description:** Implement config-based constructor with provider selection
**Tests:** Unit tests for provider selection and fallback
**Estimated:** 1 hour

```rust
impl WebSearchTool {
    pub fn from_config(config: Option<&WebToolsConfig>) -> Self {
        let provider: Box<dyn SearchProvider + Send + Sync> = match config {
            Some(cfg) => match cfg.search_provider.as_deref() {
                Some("exa") => {
                    env::var("EXA_API_KEY")
                        .map(|key| Box::new(ExaProvider::new(key)) as Box<dyn SearchProvider + Send + Sync>)
                        .unwrap_or_else(|_| Box::new(PlaceholderProvider))
                }
                Some("kimi_search") => {
                    env::var("KIMI_API_KEY")
                        .map(|key| Box::new(KimiSearchProvider::new(key)) as Box<dyn SearchProvider + Send + Sync>)
                        .unwrap_or_else(|_| Box::new(PlaceholderProvider))
                }
                Some(_) | None => Self::from_env_inner(),
            },
            None => Self::from_env_inner(),
        };

        Self { provider }
    }

    // Extract current from_env() logic to reuse
    fn from_env_inner() -> Box<dyn SearchProvider + Send + Sync> {
        // Existing from_env() logic
    }
}
```

### Step 3: Add from_config() to WebFetchTool
**Files:** `crates/terraphim_tinyclaw/src/tools/web.rs`
**Description:** Implement config-based constructor with mode selection
**Tests:** Unit tests for mode selection and fallback
**Dependencies:** Step 2
**Estimated:** 45 minutes

```rust
impl WebFetchTool {
    pub fn from_config(config: Option<&WebToolsConfig>) -> Self {
        let mode = config
            .and_then(|c| c.fetch_mode.clone())
            .unwrap_or_else(|| "raw".to_string());

        Self {
            client: Client::new(),
            mode,
        }
    }
}
```

### Step 4: Update create_default_registry Signature
**Files:** `crates/terraphim_tinyclaw/src/tools/mod.rs`
**Description:** Add web_tools_config parameter and use it when creating tools
**Tests:** Update existing tests to pass None for new parameter
**Dependencies:** Step 2, Step 3
**Estimated:** 30 minutes

```rust
pub fn create_default_registry(
    sessions: Option<Arc<tokio::sync::Mutex<SessionManager>>>,
    web_tools_config: Option<&WebToolsConfig>,
) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Use from_config instead of new()
    registry.register(Box::new(WebSearchTool::from_config(web_tools_config)));
    registry.register(Box::new(WebFetchTool::from_config(web_tools_config)));

    // ... rest of existing tools

    registry
}
```

### Step 5: Update main.rs Call Site
**Files:** `crates/terraphim_tinyclaw/src/main.rs`
**Description:** Pass config.web_tools reference when creating registry
**Tests:** Manual verification that config is wired
**Dependencies:** Step 4
**Estimated:** 15 minutes

```rust
// Change from:
let tools = Arc::new(create_default_registry(Some(sessions.clone())));

// To:
let web_tools_config = config.as_ref().and_then(|c| c.web_tools.as_ref());
let tools = Arc::new(create_default_registry(Some(sessions.clone()), web_tools_config));
```

### Step 6: Add Unit Tests
**Files:** `crates/terraphim_tinyclaw/src/tools/web.rs` (in #[cfg(test)] module)
**Description:** Add comprehensive tests for new constructors
**Tests:** 6-7 new unit tests
**Dependencies:** Step 2, Step 3
**Estimated:** 1 hour

### Step 7: Add Integration Test
**Files:** `crates/terraphim_tinyclaw/tests/config_wiring.rs` (new file)
**Description:** End-to-end test verifying config reaches tools
**Tests:** 1 integration test
**Dependencies:** Step 4
**Estimated:** 45 minutes

### Step 8: Run Full Test Suite
**Files:** N/A
**Description:** Verify no regressions
**Tests:** `cargo test -p terraphim_tinyclaw`
**Dependencies:** All previous steps
**Estimated:** 15 minutes

---

## Rollback Plan

If issues discovered:
1. Revert changes to `web.rs`, `mod.rs`, `main.rs`, `config.rs`
2. Restore original `create_default_registry(Some(sessions.clone()))` call in main.rs
3. Verify tests pass: `cargo test -p terraphim_tinyclaw`

No feature flags required - the change is purely additive to the API.

---

## Dependencies

### No New Dependencies
This change uses existing crates only.

### No Dependency Updates

---

## Performance Considerations

### Expected Performance

| Metric | Target | Current |
|--------|--------|---------|
| Tool initialization | < 1ms | ~0.5ms |
| Config lookup overhead | < 0.1ms | N/A (new) |

### No Benchmarks Required
This is a simple wiring change with minimal overhead.

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm env var precedence (config vs env) | Needs Decision | Engineering |
| Verify integration test approach | Pending | Engineering |

---

## Approval Checklist

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Backward compatibility verified
- [ ] Human approval received

---

## Appendix: Provider Name Decision

**Decision:** Update config docs to match implementation ("exa", "kimi_search")

**Rationale:**
- Simplest solution - just documentation change
- No code complexity added
- Users can see actual available providers
- When new providers are added, just update docs

**Rejected:** Name mapping
- Adds unnecessary code complexity
- Creates confusion about "which name is canonical"
- More code to maintain and test
