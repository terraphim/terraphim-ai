# Verification and Validation Report: Issue #589

**Issue**: TinyClaw: Implement provider-backed web_search and config-driven web tooling
**Repository**: terraphim/terraphim-ai
**Date**: 2026-03-11
**Status**: PARTIAL IMPLEMENTATION - Verification Complete

---

## Executive Summary

The implementation for issue #589 has been partially completed. The core provider-backed `web_search` functionality is implemented with multiple provider support (Exa, Kimi), and the `web_fetch` tool has basic configuration structure. However, **critical gaps remain** in config integration and integration testing.

| Criterion | Status | Evidence |
|-----------|--------|----------|
| web_search returns real results | PARTIAL | Providers implemented, but config not wired |
| Config supports provider selection | NOT MET | Config struct exists but not used by tools |
| Output structured for LLM use | MET | Consistent formatting implemented |
| web_fetch uses configured mode | NOT MET | Hardcoded to "raw", config ignored |
| Integration tests | NOT MET | No integration tests for providers |

---

## Detailed Verification

### 1. web_search Returns Real Results from Configured Provider

**Status**: PARTIALLY MET

**Evidence**:
- File: `crates/terraphim_tinyclaw/src/tools/web.rs`
- Lines: 14-231

**Implementation Found**:
- `SearchProvider` trait defined (lines 6-12) with `search()` and `name()` methods
- `ExaProvider` implemented (lines 15-109) with full Exa.ai API integration
- `KimiSearchProvider` implemented (lines 112-209) with Moonshot AI integration
- `PlaceholderProvider` implemented (lines 212-231) for graceful degradation

**Gap Identified**:
The `WebSearchTool::new()` method (line 240-242) only uses `from_env()` which reads environment variables directly. The `WebToolsConfig` struct in `config.rs` (lines 414-422) defines `search_provider` and `fetch_mode` fields, but these are **never consumed** by the tool registry.

```rust
// Current implementation (web.rs:250-267)
pub fn from_env() -> Self {
    // Only checks environment variables, ignores config
    if let Ok(api_key) = std::env::var("EXA_API_KEY") { ... }
    if let Ok(api_key) = std::env::var("KIMI_API_KEY") { ... }
}
```

**Required Fix**:
Add a `from_config()` method to `WebSearchTool` that accepts `WebToolsConfig` and creates the appropriate provider based on config settings.

---

### 2. Config Supports Provider Selection and Required Credentials

**Status**: NOT MET

**Evidence**:
- File: `crates/terraphim_tinyclaw/src/config.rs`
- Lines: 388-422

**Implementation Found**:
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolsConfig {
    pub shell: Option<ShellToolConfig>,
    pub web: Option<WebToolsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebToolsConfig {
    /// Web search provider ("brave", "searxng", "google").
    pub search_provider: Option<String>,
    /// Web fetch mode ("readability", "raw").
    pub fetch_mode: Option<String>,
}
```

**Issues Identified**:
1. **Provider mismatch**: Config documents "brave", "searxng", "google" but implementation only supports "exa" and "kimi_search"
2. **Missing API key storage**: `WebToolsConfig` has no field for API keys - only environment variables are supported
3. **Config not wired**: `create_default_registry()` in `tools/mod.rs:152-178` does not accept config parameter

**Required Fix**:
Update `WebToolsConfig` to match implemented providers and add API key fields, or document that API keys must use environment variables.

---

### 3. Output is Structured and Consistently Formatted for LLM/Tool-Loop Use

**Status**: MET

**Evidence**:
- File: `crates/terraphim_tinyclaw/src/tools/web.rs`

**ExaProvider Output Format** (lines 87-107):
```rust
let mut output = format!("Search results for '{}' via Exa:\n\n", query);
for (i, result) in results.iter().take(num_results).enumerate() {
    let title = result["title"].as_str().unwrap_or("No title");
    let url = result["url"].as_str().unwrap_or("No URL");
    let text = result["text"].as_str().unwrap_or("");
    output.push_str(&format!("{}. {}\n{}\n{}\n\n", i + 1, title, url, text));
}
```

**WebFetchTool Output Format** (lines 387-399):
- Returns raw content with truncation at 10,000 characters
- Includes "[Content truncated]" message when limit exceeded

**Assessment**: Output is consistently formatted and suitable for LLM consumption.

---

### 4. web_fetch Uses Configured Fetch Mode and Limits

**Status**: NOT MET

**Evidence**:
- File: `crates/terraphim_tinyclaw/src/tools/web.rs`
- Lines: 326-401

**Implementation Found**:
```rust
pub struct WebFetchTool {
    client: Client,
    mode: String,
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            mode: "raw".to_string(),  // HARDCODED
        }
    }
}
```

**Issues Identified**:
1. `WebFetchTool::new()` hardcodes mode to "raw" (line 336)
2. `create_default_registry()` creates `WebFetchTool::new()` without config (line 167)
3. Config's `fetch_mode` field is never read
4. Comment at line 387 indicates "readability" mode is not implemented: "In 'readability' mode, we'd extract main content"

**Required Fix**:
- Implement `from_config()` method for `WebFetchTool`
- Wire config through `create_default_registry()`
- Implement readability mode content extraction

---

### 5. Integration Tests Validate Provider Behavior and Fallback Handling

**Status**: NOT MET

**Evidence**:
- Unit tests in `web.rs` lines 446-495: 6 tests, all passing
- Integration tests in `tests/` directory: 0 web-related tests

**Unit Tests Found**:
```
tools::web::tests::test_exa_provider_name
 tools::web::tests::test_kimi_provider_name
tools::web::tests::test_placeholder_provider_name
tools::web::tests::test_web_fetch_tool_schema
tools::web::tests::test_web_search_placeholder
tools::web::tests::test_web_search_tool_schema
```

**Test Execution**:
```bash
$ cargo test -p terraphim_tinyclaw tools::web::
running 6 tests
test tools::web::tests::test_placeholder_provider_name ... ok
test tools::web::tests::test_web_search_tool_schema ... ok
test tools::web::tests::test_web_search_placeholder ... ok
test tools::web::tests::test_web_fetch_tool_schema ... ok
test tools::web::tests::test_kimi_provider_name ... ok
test tools::web::tests::test_exa_provider_name ... ok
test result: ok. 6 passed; 0 failed; 0 ignored
```

**Gap Identified**:
- No integration tests for actual provider API calls (even with mocking)
- No tests for config-driven tool creation
- No tests for fallback behavior when API keys are missing
- No tests for `WebToolsConfig` integration

---

## Static Analysis Results

### UBS Scanner
```bash
$ ubs /home/alex/projects/terraphim/terraphim-ai/crates/terraphim_tinyclaw/src/tools/web.rs --only=rust
Files scanned: 1
Critical issues: 2
Warning issues: 9
Info items: 5
```

**Critical Findings** (require investigation):
- 2 critical issues detected (details require verbose output for full classification)
- 9 warnings including panic surfaces from assert macros

### Clippy
```bash
$ cargo clippy -p terraphim_tinyclaw --all-targets
# No warnings or errors for web.rs
```

---

## Traceability Matrix

| Requirement | Design Element | Code Location | Test Coverage | Status |
|-------------|----------------|---------------|---------------|--------|
| Provider-backed search | SearchProvider trait | web.rs:6-12 | Unit tests only | PARTIAL |
| Exa provider | ExaProvider struct | web.rs:15-109 | test_exa_provider_name | PARTIAL |
| Kimi provider | KimiSearchProvider struct | web.rs:112-209 | test_kimi_provider_name | PARTIAL |
| Config provider selection | WebToolsConfig struct | config.rs:416-422 | NONE | NOT MET |
| Config credentials | MISSING | - | NONE | NOT MET |
| Structured output | format! macros | web.rs:87-107, 387-399 | test_web_search_placeholder | MET |
| Config-driven fetch mode | WebFetchTool::mode | web.rs:328, 336 | NONE | NOT MET |
| Integration tests | MISSING | tests/ | NONE | NOT MET |

---

## Defect Register

| ID | Description | Origin | Severity | Resolution | Status |
|----|-------------|--------|----------|------------|--------|
| D001 | Config not wired to WebSearchTool | Phase 3 Implementation | High | Add from_config() method | OPEN |
| D002 | Config provider list mismatches implementation | Phase 2 Design | Medium | Update config or implementation | OPEN |
| D003 | WebToolsConfig missing API key fields | Phase 2 Design | Medium | Add api_key fields or document env-only | OPEN |
| D004 | WebFetchTool ignores config fetch_mode | Phase 3 Implementation | High | Wire config through registry | OPEN |
| D005 | Readability mode not implemented | Phase 3 Implementation | Medium | Implement content extraction | OPEN |
| D006 | No integration tests for providers | Phase 4 Verification | High | Add integration tests | OPEN |

---

## Recommendations

### Critical (Block Release)
1. **Wire config to tools**: Modify `create_default_registry()` to accept `Config` and pass to tool constructors
2. **Implement from_config methods**: Add `WebSearchTool::from_config()` and `WebFetchTool::from_config()`
3. **Add integration tests**: Create tests in `tests/web_integration.rs` for provider behavior

### High Priority
4. **Align config with implementation**: Update `WebToolsConfig` provider options to match implemented providers ("exa", "kimi_search")
5. **Document credential handling**: Clarify whether API keys should be in config file or environment variables

### Medium Priority
6. **Implement readability mode**: Add HTML content extraction for `fetch_mode = "readability"`
7. **Address UBS critical findings**: Investigate and resolve the 2 critical issues flagged by UBS

---

## Conclusion

The implementation of issue #589 is **incomplete** and requires additional work before release. While the core provider architecture is sound and unit tests pass, the critical gap is that **configuration is not wired to the tools**. The tools currently only read environment variables, ignoring the `WebToolsConfig` settings.

### GO/NO-GO Decision: NO-GO

**Reasoning**:
- Acceptance criterion #2 (config supports provider selection) is not met
- Acceptance criterion #4 (web_fetch uses configured mode) is not met
- Acceptance criterion #5 (integration tests) is not met

**Next Steps**:
1. Return to Phase 3 (Implementation) to wire config to tools
2. Add integration tests for provider behavior
3. Re-run verification before validation

---

## Appendix: File Locations

| File | Path | Purpose |
|------|------|---------|
| Web tools implementation | `crates/terraphim_tinyclaw/src/tools/web.rs` | SearchProvider trait, ExaProvider, KimiSearchProvider, WebSearchTool, WebFetchTool |
| Config definition | `crates/terraphim_tinyclaw/src/config.rs` | ToolsConfig, WebToolsConfig structs |
| Tool registry | `crates/terraphim_tinyclaw/src/tools/mod.rs` | create_default_registry() function |
| Main entry | `crates/terraphim_tinyclaw/src/main.rs` | Config loading, tool registry initialization |
| Unit tests | Embedded in web.rs | 6 tests for schema and provider names |
| Integration tests | `crates/terraphim_tinyclaw/tests/` | No web-related integration tests |
