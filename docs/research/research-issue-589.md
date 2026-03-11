# Research Document: Issue #589 - Wire WebToolsConfig to Web Search/Fetch Tools

**Status**: Approved
**Author**: Claude Code
**Date**: 2026-03-11
**Reviewers**: Engineering Team

## Executive Summary

Issue #589 identifies that TinyClaw's web search and fetch tools are initialized without access to the loaded configuration. The `WebToolsConfig` struct exists with `search_provider` and `fetch_mode` fields, but tools only read from environment variables or use hardcoded defaults. This research documents the current state, identifies the wiring gap, and provides recommendations for implementation.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Enables config-driven tool behavior, critical for multi-tenant deployments |
| Leverages strengths? | Yes | Builds on existing Config infrastructure and provider pattern |
| Meets real need? | Yes | Issue #589 explicitly requests this; users need to configure search providers |

**Proceed**: Yes - 3/3 YES

---

## Problem Statement

### Description
The TinyClaw tool system loads configuration from files and environment variables, but the web search (`WebSearchTool`) and web fetch (`WebFetchTool`) tools do not receive this configuration. Instead:
- `WebSearchTool::new()` only calls `from_env()`, ignoring any config file settings
- `WebFetchTool::new()` hardcodes `mode: "raw".to_string()`
- `create_default_registry()` does not accept a config parameter

### Impact
Users cannot configure search providers or fetch modes via configuration files. All tool behavior is determined by environment variables or hardcoded defaults, limiting flexibility.

### Success Criteria
1. Config file `web_tools.search_provider` value is used by `WebSearchTool`
2. Config file `web_tools.fetch_mode` value is used by `WebFetchTool`
3. Environment variable fallback still works when config not specified
4. Provider names in config align with implementation

---

## Current State Analysis

### Existing Implementation

**Config Loading** (`crates/terraphim_tinyclaw/src/main.rs:108-172`):
```rust
let config = match config_path {
    Some(path) if path.exists() => Config::from_file_with_env(&path)?,
    _ => Config::from_env(),
};
```
Config is loaded but only passed to agent modes, not to tool registry.

**Tool Registry Creation** (`crates/terraphim_tinyclaw/src/main.rs:205-225`):
```rust
let tools = Arc::new(create_default_registry(Some(sessions.clone())));
// Config is NOT passed to create_default_registry
```

**create_default_registry** (`crates/terraphim_tinyclaw/src/tools/mod.rs:152-178`):
```rust
pub fn create_default_registry(
    sessions: Option<Arc<tokio::sync::Mutex<SessionManager>>>,
) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(WebSearchTool::new()));  // No config
    registry.register(Box::new(WebFetchTool::new()));   // No config
    // ...
}
```

**WebSearchTool** (`crates/terraphim_tinyclaw/src/tools/web.rs:85-92`):
```rust
impl WebSearchTool {
    pub fn new() -> Self {
        Self::from_env()  // Ignores config
    }
}
```

**WebFetchTool** (`crates/terraphim_tinyclaw/src/tools/web.rs:444-450`):
```rust
impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            mode: "raw".to_string(),  // HARDCODED
        }
    }
}
```

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Config struct | `crates/terraphim_tinyclaw/src/config.rs:388-422` | WebToolsConfig definition |
| Config loading | `crates/terraphim_tinyclaw/src/main.rs:108-172` | Loads config from file/env |
| Tool registry | `crates/terraphim_tinyclaw/src/tools/mod.rs:152-178` | Creates and registers tools |
| WebSearchTool | `crates/terraphim_tinyclaw/src/tools/web.rs:1-496` | Search tool implementation |
| WebFetchTool | `crates/terraphim_tinyclaw/src/tools/web.rs:420-496` | Fetch tool implementation |
| SearchProvider trait | `crates/terraphim_tinyclaw/src/tools/web.rs:191-237` | Provider abstraction |

### Data Flow

```
Config File (web_tools.search_provider)
    |
    v
Config::from_file_with_env()  -->  NOT PASSED TO TOOLS
    |
    v
Agent modes (only)

Tools use:
- WebSearchTool: env vars only (EXA_API_KEY)
- WebFetchTool: hardcoded "raw"
```

---

## Constraints

### Technical Constraints
- **Backward compatibility**: Tools must still work when config not provided
- **Environment fallback**: EXA_API_KEY, KIMI_API_KEY must still be respected
- **Provider availability**: Exa and Kimi providers require API keys
- **Async context**: Tool initialization happens in async context

### Business Constraints
- **Breaking changes**: Minimize API changes to public tool constructors
- **Feature flags**: Optional providers behind feature gates

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Config load time | < 10ms | ~5ms |
| Tool init time | < 1ms | ~0.5ms |
| Backward compat | 100% | N/A (new feature) |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Backward compatibility | Existing users with env vars must not break | Production deployments rely on env vars |
| Minimal API changes | Avoid breaking downstream consumers | Tool constructors are public API |
| Provider name alignment | Config docs say "brave"/"searxng" but impl only has "exa"/"kimi_search" | User confusion |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Add new search providers (Brave, Searxng, Google) | Out of scope - this is wiring, not new providers |
| Tool configuration hot-reload | Not requested, complex lifecycle management |
| Per-session tool configuration | Not requested, significant complexity |

---

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_tinyclaw::Config | Must add WebToolsConfig access | Low - same crate |
| terraphim_tinyclaw::tools::mod | Must modify create_default_registry signature | Low - internal API |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| reqwest | 0.12.x | Low | N/A |
| serde | 1.0.x | Low | N/A |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing env var usage | Low | High | Keep from_env() as fallback |
| Provider name confusion | High | Medium | Document or implement name mapping |
| Config not available at tool init | Low | Medium | Use Option<WebToolsConfig> |

### Open Questions

1. Should we implement provider name mapping ("brave" -> "exa") or update config docs? - Config docs should match implementation
2. Should WebSearchTool::new() be deprecated in favor of from_config()? - Recommend adding from_config(), keeping new() for compat

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| WebToolsConfig is always available when create_default_registry is called | Config loaded before registry creation | Tools won't have config | Yes - main.rs flow |
| Environment variables should override config values | Standard 12-factor app pattern | User confusion | No - needs decision |

---

## Research Findings

### Key Insights

1. **Wiring Gap**: The config is loaded but there's no code path to pass it to tools. Three changes needed:
   - `create_default_registry()` must accept config parameter
   - `WebSearchTool` needs `from_config()` method
   - `WebFetchTool` needs `from_config()` method

2. **Provider Mismatch**: Config docs mention "brave", "searxng", "google" but implementation only has "exa", "kimi_search". This will confuse users.

3. **Clean Separation**: The `SearchProvider` trait already abstracts providers, making config-driven selection straightforward.

### Relevant Prior Art

- Other tools in the registry receive config via `create_default_registry` parameter (e.g., sessions)
- Provider pattern used successfully in other parts of the codebase

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Provider name mapping | Decide on name alignment strategy | 30 minutes |
| Env var precedence | Confirm env vars override config | 15 minutes |

---

## Recommendations

### Proceed/No-Proceed
**Proceed** - This is a straightforward wiring task that improves configuration flexibility.

### Scope Recommendations
1. Add `from_config()` methods to WebSearchTool and WebFetchTool
2. Modify `create_default_registry()` to accept `Option<WebToolsConfig>`
3. Update main.rs to pass config to registry
4. Align provider names in config docs with implementation (or implement mapping)

### Risk Mitigation Recommendations
1. Keep `WebSearchTool::new()` and `WebFetchTool::new()` for backward compatibility
2. Use `Option<WebToolsConfig>` to handle missing config gracefully
3. Add provider name mapping if we want to support documented names

---

## Next Steps

If approved:
1. Create implementation plan using disciplined design skill
2. Implement from_config() methods
3. Update create_default_registry() signature
4. Wire config through main.rs
5. Add integration tests

---

## Appendix

### Provider Name Alignment Options

**Option A: Update Config Docs**
Change config.rs documentation to match implementation:
```rust
/// Web search provider ("exa", "kimi_search")
```

**Option B: Implement Name Mapping**
Map documented names to implementation names:
```rust
match provider {
    "brave" | "exa" => ExaProvider,
    "searxng" | "kimi_search" => KimiSearchProvider,
    _ => PlaceholderProvider,
}
```

### Code Snippets

**Current WebToolsConfig**:
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebToolsConfig {
    /// Web search provider ("brave", "searxng", "google").
    pub search_provider: Option<String>,
    /// Web fetch mode ("readability", "raw").
    pub fetch_mode: Option<String>,
}
```

**Current SearchProvider implementations**:
```rust
pub fn from_env() -> Box<dyn SearchProvider + Send + Sync> {
    if let Ok(api_key) = env::var("EXA_API_KEY") {
        return Box::new(ExaProvider::new(api_key));
    }
    if let Ok(api_key) = env::var("KIMI_API_KEY") {
        return Box::new(KimiSearchProvider::new(api_key));
    }
    Box::new(PlaceholderProvider)
}
```
