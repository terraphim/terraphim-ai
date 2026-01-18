# Design & Implementation Plan: Quickwit Haystack Integration

**Date:** 2026-01-13
**Phase:** 2 - Design and Planning
**Status:** Draft - Awaiting Quality Evaluation
**Based On:** [Research Document](research-quickwit-haystack-integration.md) (Phase 1 - Approved)

---

## 1. Summary of Target Behavior

### What Changes
After implementation, Terraphim AI will support Quickwit as a searchable haystack alongside existing sources (Ripgrep, QueryRs, ClickUp, etc.).

### User Experience
1. **Configuration:** Users add Quickwit haystack to role configuration via JSON:
   ```json
   {
     "location": "http://localhost:7280",
     "service": "Quickwit",
     "extra_parameters": {
       "auth_token": "Bearer token123",
       "default_index": "workers-logs",
       "max_hits": "100"
     }
   }
   ```

2. **Search:** When user searches via `terraphim-agent`, the query:
   - Hits Quickwit REST API (`GET /v1/{index}/search`)
   - Returns log entries as Terraphim Documents
   - Merges with other haystack results
   - Displays in CLI with timestamp, level, message

3. **Error Handling:** Network failures or auth errors return empty results with logged warnings (graceful degradation)

### System Behavior
- Quickwit indexer executes asynchronously alongside other haystacks
- Results cached for 1 hour (configurable via persistence layer)
- Timeouts after 10 seconds (configurable)
- Supports bearer token authentication
- Sorts results by timestamp descending (most recent first)

---

## 2. Key Invariants and Acceptance Criteria

### Invariants

#### Data Consistency
- **INV-1:** Every Document must have unique `id` derived from `{index_name}_{document_id}`
- **INV-2:** Document `source_haystack` field must be set to Quickwit base URL
- **INV-3:** Empty/failed searches return `Index::new()` (empty HashMap), never `Err`

#### Security & Privacy
- **INV-4:** Auth tokens MUST NOT appear in logs or error messages (redact after first 4 chars)
- **INV-5:** HTTP connections to non-localhost MUST use HTTPS or log security warning
- **INV-6:** Follow `atomic_server_secret` pattern - tokens in `extra_parameters` not serialized by default

#### Performance
- **INV-7:** HTTP requests timeout after 10 seconds (default, configurable)
- **INV-8:** Result limit defaults to 100 hits (prevent memory exhaustion)
- **INV-9:** Concurrent searches don't block - each haystack executes independently

#### API Contract
- **INV-10:** Implements `IndexMiddleware` trait with signature: `async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index>`
- **INV-11:** Compatible with Quickwit 0.7+ REST API schema
- **INV-12:** Handles missing JSON fields gracefully (use `Option<T>` and `serde(default)`)

### Acceptance Criteria

| ID | Criterion | Verification Method |
|----|-----------|---------------------|
| **AC-1** | User can configure Quickwit haystack in role JSON | Manual: Add config, reload, verify no errors |
| **AC-2** | Search query "error" returns matching log entries from Quickwit | Integration test: Query known index, assert hits > 0 |
| **AC-3** | Results include timestamp, level, message fields | Unit test: Parse sample response, verify Document fields |
| **AC-4** | Auth token from extra_parameters sent as Bearer header | Integration test: Mock server verifies Authorization header |
| **AC-5** | Network timeout returns empty results, logs warning | Integration test: Point to non-existent host, verify empty Index |
| **AC-6** | Invalid JSON response returns empty results, logs error | Unit test: Feed malformed JSON, verify graceful handling |
| **AC-7** | Multiple indexes can be searched via multiple haystack configs | Integration test: Two haystack configs, different indexes |
| **AC-8** | Results sorted by timestamp descending | Integration test: Verify hits[0].rank > hits[1].rank |
| **AC-9** | Works without authentication for localhost development | Integration test: No auth_token, localhost Quickwit |
| **AC-10** | Auth tokens redacted in logs | Unit test: Trigger error with token, verify log output |
| **AC-11** | Auto-discovery fetches all indexes when default_index absent | Integration test: Config without default_index, verify multiple indexes searched |
| **AC-12** | Explicit index searches only that index | Integration test: Config with default_index, verify single index searched |
| **AC-13** | Index filter pattern filters auto-discovered indexes | Integration test: index_filter="workers-*", verify only matching indexes |

---

## 3. High-Level Design and Boundaries

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│  terraphim-agent CLI (User Interface)                       │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│  terraphim_middleware::indexer::search_haystacks()          │
│  - Orchestrates concurrent haystack queries                 │
│  - Merges results from all haystacks                        │
└────────┬────────────────────────────────┬───────────────────┘
         │                                │
         ▼                                ▼
┌────────────────────┐         ┌─────────────────────────────┐
│ RipgrepIndexer     │         │ QuickwitHaystackIndexer     │ ◄─ NEW
│ QueryRsIndexer     │         │ - HTTP client (reqwest)     │
│ ClickUpIndexer     │         │ - JSON parsing (serde)      │
│ ... (existing)     │         │ - Document transformation   │
└────────────────────┘         │ - Error handling            │
                               └──────────┬──────────────────┘
                                          │
                                          ▼
                               ┌─────────────────────────────┐
                               │ Quickwit REST API           │
                               │ GET /v1/indexes             │
                               │ GET /v1/{index}/search      │
                               └─────────────────────────────┘
```

### Component Boundaries

#### New Component: QuickwitHaystackIndexer
**Location:** `crates/terraphim_middleware/src/haystack/quickwit.rs`

**Responsibilities:**
- Parse Quickwit configuration from `Haystack::extra_parameters`
- Build HTTP request with query parameters and authentication
- Execute async HTTP call to Quickwit REST API
- Parse JSON response into `Vec<Document>`
- Transform Quickwit hits to Terraphim Document structure
- Handle errors gracefully (timeouts, auth failures, malformed JSON)
- Normalize document IDs for persistence layer

**Does NOT:**
- Manage Quickwit server lifecycle
- Create or modify Quickwit indexes
- Implement query syntax validation (pass-through to Quickwit)
- Cache at indexer level (handled by persistence layer)

#### Modified Component: ServiceType Enum
**Location:** `crates/terraphim_config/src/lib.rs`

**Change:** Add `Quickwit` variant to enum

**Dependencies:** None (simple enum addition)

#### Modified Component: Haystack Orchestration
**Location:** `crates/terraphim_middleware/src/indexer/mod.rs`

**Change:** Add match arm for `ServiceType::Quickwit`

**Pattern:** Follow existing pattern (instantiate indexer, call `.index()`)

### Design Decisions

#### Decision 1: Configuration via extra_parameters
**Rationale:** Consistent with other haystacks (ClickUp, QueryRs). Avoids modifying core Haystack struct.

**Parameters:**
- `auth_token` (optional): Bearer token for authentication (e.g., "Bearer xyz123")
- `auth_username` (optional): Basic auth username (use with auth_password)
- `auth_password` (optional): Basic auth password (use with auth_username)
- `default_index` (optional): Specific index name to search. If absent, auto-discovers all available indexes
- `index_filter` (optional): Glob pattern to filter auto-discovered indexes (e.g., "logs-*", "workers-*")
- `max_hits` (optional, default: "100"): Result limit per index
- `timeout_seconds` (optional, default: "10"): HTTP timeout
- `sort_by` (optional, default: "-timestamp"): Sort order

#### Decision 2: Follow QueryRsHaystackIndexer Pattern
**Rationale:** Similar HTTP API integration, proven caching strategy, consistent error handling.

**Reused Patterns:**
- `reqwest::Client` configuration with timeout and user-agent
- Document ID normalization via `Persistable::normalize_key()`
- Graceful error handling returning empty `Index`
- Progress logging at info/warn/debug levels
- `async_trait` implementation

#### Decision 3: Authentication - Bearer Token and Basic Auth
**Rationale:** try_search uses Basic Auth. Support both for maximum compatibility.

**Implementation:**
- If `auth_token` present: use as `Authorization: Bearer {token}` header
- If `auth_username` + `auth_password` present: use as `Authorization: Basic {base64(user:pass)}` header
- If neither: no authentication (development/localhost)
- Priority: Check auth_token first, then username/password

#### Decision 4: No Result Caching in Indexer
**Rationale:** Persistence layer already handles caching. Avoids duplication and TTL management complexity.

#### Decision 5: Hybrid Index Discovery Strategy
**Rationale:** Balances performance (explicit config) with user convenience (auto-discovery). Follows try_search implementation pattern.

**Implementation:**
- If `default_index` specified: Search only that index (1 API call - fast)
- If `default_index` absent: Auto-discover via `GET /v1/indexes`, search all (N+1 API calls - convenient)
- Optional `index_filter` glob pattern filters auto-discovered indexes

**Trade-offs Accepted:**
- Auto-discovery adds ~300ms latency (acceptable for convenience)
- Multiple concurrent index searches (mitigated by tokio::join! parallelization)
- Complexity of three code paths (mitigated by clear branching logic)

**User Preference:** Explicit option B selected - ship with full hybrid support in v1.

---

## 4. File/Module-Level Change Plan

| File/Module | Action | Responsibility Before | Responsibility After | Dependencies |
|-------------|--------|----------------------|---------------------|--------------|
| `crates/terraphim_config/src/lib.rs` | **Modify** | Define ServiceType enum with 8 variants | Add `Quickwit` as 9th variant | None |
| `crates/terraphim_middleware/src/haystack/quickwit.rs` | **Create** | N/A - file doesn't exist | Implement QuickwitHaystackIndexer with IndexMiddleware trait | reqwest, serde_json, async_trait, terraphim_types, terraphim_config |
| `crates/terraphim_middleware/src/haystack/mod.rs` | **Modify** | Export 7 haystack indexers | Export QuickwitHaystackIndexer (add `pub use quickwit::QuickwitHaystackIndexer;`) | None |
| `crates/terraphim_middleware/src/indexer/mod.rs` | **Modify** | Match on 8 ServiceType variants in search_haystacks() | Add `ServiceType::Quickwit` match arm | QuickwitHaystackIndexer |
| `crates/terraphim_middleware/tests/quickwit_haystack_test.rs` | **Create** | N/A - file doesn't exist | Integration tests for Quickwit indexer | tokio, serde_json, terraphim_middleware |
| `crates/terraphim_agent/tests/quickwit_integration_test.rs` | **Create** | N/A - file doesn't exist | End-to-end tests via terraphim-agent CLI | tokio, terraphim_agent |
| `terraphim_server/default/quickwit_engineer_config.json` | **Create** | N/A | Example role configuration with Quickwit haystack | None |
| `crates/terraphim_middleware/Cargo.toml` | **Verify** | Existing dependencies | Ensure reqwest features include "json", "rustls-tls" | None (likely no change needed) |

### Detailed File Specifications

#### File 1: `crates/terraphim_config/src/lib.rs`
**Line Range:** Around line 259 (after existing ServiceType variants)

**Change:**
```rust
pub enum ServiceType {
    Ripgrep,
    Atomic,
    QueryRs,
    ClickUp,
    Mcp,
    Perplexity,
    GrepApp,
    AiAssistant,
    Quickwit,  // ← ADD THIS LINE
}
```

**Testing:** Ensure deserialization from JSON works: `serde_json::from_str::<ServiceType>("\"Quickwit\"")`

---

#### File 2: `crates/terraphim_middleware/src/haystack/quickwit.rs` (NEW)
**Structure:**
```rust
use crate::indexer::IndexMiddleware;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use terraphim_config::Haystack;
use terraphim_persistence::Persistable;
use terraphim_types::{Document, Index};

// Response structures
#[derive(Debug, Deserialize)]
struct QuickwitSearchResponse {
    num_hits: u64,
    hits: Vec<serde_json::Value>,
    elapsed_time_micros: u64,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct QuickwitIndexInfo {
    index_id: String,
}

// Main indexer
#[derive(Debug, Clone)]
pub struct QuickwitHaystackIndexer {
    client: Client,
}

impl Default for QuickwitHaystackIndexer {
    fn default() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Terraphim/1.0 (Quickwit integration)")
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client }
    }
}

impl QuickwitHaystackIndexer {
    // Helper: Extract config from extra_parameters
    fn parse_config(&self, haystack: &Haystack) -> QuickwitConfig { ... }

    // Helper: Fetch available indexes from Quickwit API
    async fn fetch_available_indexes(&self, base_url: &str, auth_token: Option<&str>) -> Result<Vec<QuickwitIndexInfo>> { ... }

    // Helper: Filter indexes by glob pattern
    fn filter_indexes(&self, indexes: Vec<QuickwitIndexInfo>, pattern: &str) -> Vec<QuickwitIndexInfo> { ... }

    // Helper: Search single index and return results
    async fn search_single_index(&self, needle: &str, index: &str, base_url: &str, config: &QuickwitConfig) -> Result<Index> { ... }

    // Helper: Build search URL with query params
    fn build_search_url(&self, base_url: &str, index: &str, query: &str, config: &QuickwitConfig) -> String { ... }

    // Helper: Transform Quickwit hit to Terraphim Document
    fn hit_to_document(&self, hit: &serde_json::Value, index_name: &str, base_url: &str) -> Option<Document> { ... }

    // Helper: Normalize document ID
    fn normalize_document_id(&self, index_name: &str, doc_id: &str) -> String { ... }

    // Helper: Redact auth token for logging
    fn redact_token(&self, token: &str) -> String { ... }
}

#[async_trait]
impl IndexMiddleware for QuickwitHaystackIndexer {
    async fn index(&self, needle: &str, haystack: &Haystack) -> crate::Result<Index> {
        // 1. Parse configuration from extra_parameters
        // 2. Determine indexes to search:
        //    - If default_index present: use it (explicit)
        //    - Else: fetch_available_indexes() and optionally filter (auto-discovery)
        // 3. For each index, search_single_index() concurrently using tokio::join!
        // 4. Merge all results into single Index
        // 5. Handle errors gracefully (empty Index on failure)
        // 6. Return merged Index
    }
}

// Note: search_single_index() performs:
// - Build search URL with query params
// - Add authentication header if token present
// - Execute HTTP request with timeout
// - Parse JSON response
// - Transform hits to Documents
// - Return Index for this specific index
```

**Key Implementation Notes:**
- **QuickwitConfig structure:**
  ```rust
  struct QuickwitConfig {
      auth_token: Option<String>,        // Bearer token
      auth_username: Option<String>,     // Basic auth username
      auth_password: Option<String>,     // Basic auth password
      default_index: Option<String>,     // If None, auto-discover
      index_filter: Option<String>,      // Glob pattern for filtering
      max_hits: u64,                     // Default: 100
      timeout_seconds: u64,              // Default: 10
      sort_by: String,                   // Default: "-timestamp"
  }
  ```
- **Auto-discovery logic:**
  ```rust
  let indexes = if let Some(idx) = config.default_index {
      vec![idx]  // Explicit: single index
  } else {
      let all = self.fetch_available_indexes(base_url, auth_token).await?;
      if let Some(pattern) = config.index_filter {
          self.filter_indexes(all, &pattern)  // Filtered discovery
      } else {
          all  // Full auto-discovery
      }
  };
  // Search all indexes concurrently with tokio::join!
  ```
- Use `serde(default)` for all optional response fields
- Redact tokens: `format!("{}...", &token[..4.min(token.len())])`
- Document ID: `format!("quickwit_{}_{}", index_name, quickwit_doc_id)` or hash if no _id field
- Title from log `message` field or `[index_name] {timestamp}`
- Body: full JSON as string `serde_json::to_string(&hit)`
- Tags: `["quickwit", "logs", level]` extracted from hit if present
- Rank: timestamp as microseconds for sorting

---

#### File 3: `crates/terraphim_middleware/src/haystack/mod.rs`
**Line Range:** After line 10

**Change:**
```rust
#[cfg(feature = "ai-assistant")]
pub mod ai_assistant;
#[cfg(feature = "atomic")]
pub mod atomic;
pub mod clickup;
#[cfg(feature = "grepapp")]
pub mod grep_app;
pub mod mcp;
pub mod perplexity;
pub mod query_rs;
pub mod quickwit;  // ← ADD THIS LINE

// ... existing pub use statements ...
pub use query_rs::QueryRsHaystackIndexer;
pub use quickwit::QuickwitHaystackIndexer;  // ← ADD THIS LINE
```

---

#### File 4: `crates/terraphim_middleware/src/indexer/mod.rs`
**Line Range:** Around line 83-140 (in search_haystacks function)

**Change:**
```rust
// Add to imports at top
use crate::haystack::QuickwitHaystackIndexer;

// Add match arm after line 107 (after Perplexity case)
ServiceType::Quickwit => {
    let quickwit = QuickwitHaystackIndexer::default();
    quickwit.index(needle, haystack).await?
}
```

---

#### File 5: `crates/terraphim_middleware/tests/quickwit_haystack_test.rs` (NEW)
**Structure:**
```rust
use terraphim_config::{Haystack, ServiceType};
use terraphim_middleware::haystack::QuickwitHaystackIndexer;
use terraphim_middleware::indexer::IndexMiddleware;
use std::collections::HashMap;

#[tokio::test]
async fn test_quickwit_indexer_initialization() {
    let indexer = QuickwitHaystackIndexer::default();
    // Verify client configured with timeout
}

#[tokio::test]
async fn test_parse_quickwit_config() {
    let mut extra_params = HashMap::new();
    extra_params.insert("auth_token".to_string(), "Bearer test123".to_string());
    extra_params.insert("default_index".to_string(), "logs".to_string());

    let haystack = Haystack {
        location: "http://localhost:7280".to_string(),
        service: ServiceType::Quickwit,
        extra_parameters: extra_params,
        // ... other fields
    };

    // Test config parsing
}

#[tokio::test]
async fn test_document_transformation() {
    let sample_hit = serde_json::json!({
        "timestamp": "2024-01-13T10:30:00Z",
        "level": "ERROR",
        "message": "Test error message",
        "service": "test-service"
    });

    // Test hit_to_document transformation
}

#[tokio::test]
async fn test_token_redaction() {
    // Verify tokens redacted in logs
}

#[tokio::test]
#[ignore] // Requires running Quickwit server
async fn test_quickwit_live_search() {
    // Integration test with real Quickwit
    // Set QUICKWIT_URL environment variable
    // Query for known data, verify results
}

#[tokio::test]
async fn test_error_handling_timeout() {
    // Point to non-existent host, verify timeout handling
}

#[tokio::test]
async fn test_error_handling_invalid_json() {
    // Mock server returning invalid JSON
    // Verify graceful handling
}
```

---

#### File 6: `crates/terraphim_agent/tests/quickwit_integration_test.rs` (NEW)
**Structure:**
```rust
use terraphim_agent::/* appropriate modules */;

#[tokio::test]
#[ignore] // Requires running Quickwit + terraphim-agent
async fn test_end_to_end_quickwit_search() {
    // 1. Start terraphim-agent with quickwit_engineer_config.json
    // 2. Execute search query
    // 3. Verify Quickwit results in output
    // 4. Verify no errors logged
}

#[tokio::test]
#[ignore]
async fn test_quickwit_with_auth() {
    // Test authenticated Quickwit access
}

#[tokio::test]
#[ignore]
async fn test_quickwit_mixed_with_other_haystacks() {
    // Config with Ripgrep + Quickwit
    // Verify both return results
}
```

---

#### File 7: `terraphim_server/default/quickwit_engineer_config.json` (NEW)
**Content:**
```json
{
  "name": "Quickwit Engineer",
  "shortname": "QuickwitEngineer",
  "relevance_function": "BM25",
  "theme": "observability",
  "haystacks": [
    {
      "location": "http://localhost:7280",
      "service": "Quickwit",
      "read_only": true,
      "fetch_content": false,
      "extra_parameters": {
        "default_index": "workers-logs",
        "max_hits": "100",
        "sort_by": "-timestamp",
        "timeout_seconds": "10"
      }
    }
  ],
  "llm_enabled": false
}
```

**Alternative: Auto-Discovery Mode**
```json
{
  "name": "Quickwit Multi-Index Explorer",
  "shortname": "QuickwitExplorer",
  "relevance_function": "BM25",
  "theme": "observability",
  "haystacks": [
    {
      "location": "https://logs.terraphim.cloud/api",
      "service": "Quickwit",
      "read_only": true,
      "fetch_content": false,
      "extra_parameters": {
        "auth_username": "cloudflare",
        "auth_password": "from_env_or_1password",
        "index_filter": "workers-*",
        "max_hits": "50",
        "sort_by": "-timestamp"
      }
    }
  ],
  "llm_enabled": false
}
```

**Note:** Auth parameters support both Bearer token and Basic Auth:
- Bearer: `"auth_token": "Bearer xyz123"`
- Basic: `"auth_username": "user"` + `"auth_password": "pass"`

---

## 5. Step-by-Step Implementation Sequence

### Prerequisites
- [ ] Verify Quickwit 0.7+ server available for testing (localhost:7280 or remote)
- [ ] Confirm reqwest dependency has json and rustls-tls features enabled

### Phase A: Core Implementation (Deployable at each step)

#### Step 1: Add ServiceType::Quickwit enum variant
**Purpose:** Enable configuration parsing
**Files:** `crates/terraphim_config/src/lib.rs`
**Actions:**
1. Add `Quickwit` variant to `ServiceType` enum
2. Run `cargo build -p terraphim_config`
3. Verify no compilation errors

**Deployable:** ✅ Yes - enum addition is backward compatible
**Rollback:** Remove variant, rebuild

---

#### Step 2: Create QuickwitHaystackIndexer skeleton
**Purpose:** Establish structure and trait implementation
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs` (new)
**Actions:**
1. Create file with module structure (imports, structs, trait impl)
2. Implement `Default` for QuickwitHaystackIndexer (HTTP client setup)
3. Implement `IndexMiddleware::index()` - return empty `Index::new()` initially
4. Add unit test for initialization

**Deployable:** ✅ Yes - unused code, no integration yet
**Rollback:** Delete file

---

#### Step 3: Integrate Quickwit into module system
**Purpose:** Wire up exports and match arm
**Files:**
- `crates/terraphim_middleware/src/haystack/mod.rs`
- `crates/terraphim_middleware/src/indexer/mod.rs`

**Actions:**
1. Export `QuickwitHaystackIndexer` in `haystack/mod.rs`
2. Add `ServiceType::Quickwit` match arm in `indexer/mod.rs`
3. Run `cargo build -p terraphim_middleware`
4. Verify compilation succeeds

**Deployable:** ✅ Yes - returns empty results, doesn't crash
**Rollback:** Remove export and match arm

---

#### Step 4: Implement configuration parsing
**Purpose:** Extract Quickwit settings from extra_parameters
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Add `QuickwitConfig` struct (auth_token, default_index, index_filter, max_hits, timeout, sort_by)
2. Implement `parse_config()` helper with defaults
3. Add unit tests for config parsing with various parameter combinations
4. Handle missing parameters with defaults

**Deployable:** ✅ Yes - config parsing isolated, no network calls yet
**Rollback:** Revert file changes

---

#### Step 4a: Implement index auto-discovery
**Purpose:** Fetch available indexes from Quickwit API when default_index not specified
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Implement `fetch_available_indexes(base_url, auth_token)` async method
2. Call `GET /v1/indexes` API endpoint
3. Parse response to extract `index_config.index_id` from each index
4. Return `Vec<QuickwitIndexInfo>` with index_id fields
5. Handle network errors gracefully (return empty vec, log warning)
6. Add unit test with sample /v1/indexes JSON response

**Deployable:** ✅ Yes - method not called yet, no behavior change
**Rollback:** Revert file changes

---

#### Step 4b: Implement index filtering (optional glob pattern)
**Purpose:** Filter auto-discovered indexes by pattern
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Implement `filter_indexes(indexes, pattern)` method
2. Use simple glob matching (e.g., "logs-*" matches "logs-workers", "logs-api")
3. Return filtered list of indexes
4. Add unit tests for glob pattern matching

**Deployable:** ✅ Yes - method not called yet
**Rollback:** Revert file changes

---

#### Step 5: Implement search_single_index helper
**Purpose:** Search one specific index and return results
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Extract single-index search logic into helper method
2. Implement `search_single_index(needle, index, base_url, config)` async method
3. Build search URL, execute HTTP request, parse response, transform to Documents
4. Return `Result<Index>` for this specific index
5. Add unit test calling this method directly

**Deployable:** ✅ Yes - helper method, can be tested independently
**Rollback:** Revert file changes

---

#### Step 6: Implement hybrid index selection in main index() method
**Purpose:** Wire up explicit vs auto-discovery logic
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Update `index()` method with branching logic:
   - If `config.default_index.is_some()`: search single index
   - Else: call `fetch_available_indexes()`, optionally filter, search all
2. Use `tokio::join!` or futures concurrency for parallel index searches
3. Merge results from all indexes into single `Index`
4. Log which indexes were searched
5. Add unit tests for all three paths (explicit, filtered, full auto-discovery)

**Deployable:** ⚠️ Partial - requires Quickwit server, but degrades gracefully
**Rollback:** Revert file changes
**Testing:** Test all three configuration modes

---

#### Step 7: Implement HTTP request construction
**Purpose:** Build search URL with query parameters
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Implement `build_search_url()` helper (URL encoding, query params)
2. Format: `{base_url}/v1/{index}/search?query={encoded}&max_hits={n}&sort_by={sort}`
3. Add authentication header if token present in search_single_index()
4. Handle HTTP errors (timeout, connection refused, 401, 404, 500)
5. Log redacted errors (never log full auth token)
6. Add unit tests for URL construction

**Deployable:** ✅ Yes - helper methods, no side effects
**Rollback:** Revert file changes

---

#### Step 8: Implement JSON response parsing
**Purpose:** Deserialize Quickwit API response
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Add `QuickwitSearchResponse` struct with `#[serde(default)]` on optional fields
2. Parse response JSON with error handling
3. Log parse errors with redacted response snippet
4. Add unit tests with sample Quickwit JSON responses
5. Test edge cases: empty hits array, missing fields, unexpected structure

**Deployable:** ✅ Yes - handles parse errors gracefully
**Rollback:** Revert file changes

---

#### Step 9: Implement Document transformation
**Purpose:** Convert Quickwit hits to Terraphim Documents
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Implement `hit_to_document()` helper
2. Extract fields: timestamp, level, message, service, etc.
3. Build Document with proper ID, title, body, tags
4. Implement `normalize_document_id()` helper (follow QueryRs pattern)
5. Set `source_haystack` field to base URL
6. Convert timestamp to rank for sorting (parse RFC3339, convert to micros)
7. Add unit tests for various log formats (ERROR, WARN, INFO levels)

**Deployable:** ✅ Yes - transformation is pure function
**Rollback:** Revert file changes

---

#### Step 10: Complete integration and logging
**Purpose:** Full end-to-end functionality with observability
**Files:** `crates/terraphim_middleware/src/haystack/quickwit.rs`
**Actions:**
1. Wire up all helpers in `index()` method
2. Add info/debug/warn logging at key points
3. Implement token redaction in logs
4. Add error context (which step failed, why)
5. Return populated `Index` on success

**Deployable:** ✅ Yes - fully functional
**Rollback:** Revert to Step 7

---

### Phase B: Testing and Documentation

#### Step 11: Add middleware integration tests
**Purpose:** Verify indexer behavior in isolation
**Files:** `crates/terraphim_middleware/tests/quickwit_haystack_test.rs` (new)
**Actions:**
1. Unit tests for config parsing (explicit, auto-discovery, filtered)
2. Unit tests for document transformation
3. Unit tests for token redaction
4. Unit tests for index filtering with glob patterns
5. Integration test with `#[ignore]` for live Quickwit (both explicit and auto-discovery)
6. Error handling tests (timeout, invalid JSON, failed index fetch)

**Deployable:** ✅ Yes - tests don't affect runtime
**Rollback:** Delete test file

---

#### Step 12: Add agent end-to-end tests
**Purpose:** Verify full system integration
**Files:** `crates/terraphim_agent/tests/quickwit_integration_test.rs` (new)
**Actions:**
1. E2E test with `#[ignore]` for full search workflow (explicit mode)
2. E2E test for auto-discovery mode
3. Test with auth token (Basic Auth from try_search: username/password)
4. Test mixed haystacks (Ripgrep + Quickwit)
5. Add Docker Compose file for CI/CD

**Deployable:** ✅ Yes - tests don't affect runtime
**Rollback:** Delete test file

---

#### Step 13: Add example configurations
**Purpose:** Provide user documentation for both modes
**Files:** `terraphim_server/default/quickwit_engineer_config.json` (new)
**Actions:**
1. Create example role config with explicit index (primary example)
2. Add comments showing auto-discovery variant
3. Add example with index_filter pattern
4. Document all extra_parameters options
5. Test loading all config variants with terraphim-agent

**Deployable:** ✅ Yes - example file doesn't affect existing configs
**Rollback:** Delete file

---

#### Step 14: Documentation and README updates
**Purpose:** User and developer documentation
**Files:** `README.md`, `docs/` (various)
**Actions:**
1. Add Quickwit to supported haystacks list
2. Document configuration options
3. Add troubleshooting section (connection errors, auth failures)
4. Update architecture diagrams
5. Add example queries and expected output

**Deployable:** ✅ Yes - documentation changes only
**Rollback:** Revert documentation changes

---

### Deployment Order
1. Deploy Steps 1-10 together (core functionality with hybrid index support)
2. Deploy Steps 11-12 (tests) in parallel with Step 13
3. Deploy Step 14 (docs) after user testing

### Feature Flags
**Not Required:** Quickwit always compiled (reqwest already a dependency)

### Database Migrations
**Not Required:** No schema changes

### Careful Rollout Considerations
- Test with non-production Quickwit instance first
- Verify auth token security (not logged, not serialized inappropriately)
- Monitor for performance impact (10s timeout default)
- Start with small result limits (max_hits=10) in testing

---

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location | Implementation Notes |
|---------------------|-----------|---------------|---------------------|
| **AC-1:** User can configure Quickwit haystack | Manual | N/A | Load example config, verify no errors |
| **AC-2:** Search returns matching log entries | Integration | `middleware/tests/quickwit_haystack_test.rs::test_quickwit_live_search` | Requires Quickwit server, use #[ignore] |
| **AC-3:** Results include timestamp, level, message | Unit | `middleware/tests/quickwit_haystack_test.rs::test_document_transformation` | Parse sample JSON, assert fields present |
| **AC-4:** Auth token sent as Bearer header | Integration | `middleware/tests/quickwit_haystack_test.rs::test_auth_header` | Mock HTTP server or log request headers |
| **AC-5:** Network timeout returns empty results | Integration | `middleware/tests/quickwit_haystack_test.rs::test_error_handling_timeout` | Point to 127.0.0.1:9999 (unused port) |
| **AC-6:** Invalid JSON returns empty results | Unit | `middleware/tests/quickwit_haystack_test.rs::test_error_handling_invalid_json` | Feed `"{invalid json"` to parser |
| **AC-7:** Multiple indexes via multiple configs | Integration | `agent/tests/quickwit_integration_test.rs::test_multi_index` | Role with 2 Quickwit haystacks |
| **AC-8:** Results sorted by timestamp desc | Integration | `middleware/tests/quickwit_haystack_test.rs::test_sorting` | Verify rank field decreases |
| **AC-9:** Works without auth for localhost | Integration | `middleware/tests/quickwit_haystack_test.rs::test_no_auth` | Config without auth_token |
| **AC-10:** Auth tokens redacted in logs | Unit | `middleware/tests/quickwit_haystack_test.rs::test_token_redaction` | Trigger error, capture log, assert no full token |
| **AC-11:** Auto-discovery fetches all indexes | Integration | `middleware/tests/quickwit_haystack_test.rs::test_auto_discovery` | Config without default_index, verify GET /v1/indexes called, multiple indexes searched |
| **AC-12:** Explicit index searches only that index | Integration | `middleware/tests/quickwit_haystack_test.rs::test_explicit_index` | Config with default_index, verify single search call |
| **AC-13:** Index filter pattern filters indexes | Integration | `middleware/tests/quickwit_haystack_test.rs::test_index_filter` | index_filter="workers-*", verify only matching indexes searched |
| **AC-14:** Basic Auth (username/password) works | Integration | `middleware/tests/quickwit_haystack_test.rs::test_basic_auth` | Config with auth_username/auth_password, verify Authorization header |

### Invariant Verification Tests

| Invariant | Test Method |
|-----------|-------------|
| **INV-1:** Unique document IDs | Unit test: Generate IDs for same index+doc, assert uniqueness |
| **INV-2:** source_haystack set | Integration test: Verify field populated after search |
| **INV-3:** Empty Index on failure | Unit test: All error paths return `Ok(Index::new())` |
| **INV-4:** Token redaction | Unit test: Log capture, assert token masked |
| **INV-5:** HTTPS enforcement | Unit test: HTTP URL triggers warning log |
| **INV-6:** Token serialization | Unit test: Serialize haystack config, assert token not in JSON |
| **INV-7:** Timeout | Integration test: Slow server, verify 10s max |
| **INV-8:** Result limit | Integration test: Large index, verify ≤100 results |
| **INV-9:** Concurrent execution | Integration test: Multiple haystacks, measure total time < sum of individual times |
| **INV-10:** IndexMiddleware trait | Compilation test: Trait bounds verified at compile time |
| **INV-11:** Quickwit API compatibility | Integration test: Real Quickwit 0.7+, parse all response fields |
| **INV-12:** Graceful field handling | Unit test: Missing optional fields parse without error |

### Test Data Requirements

#### Sample Quickwit Response (for unit tests)
```json
{
  "num_hits": 3,
  "hits": [
    {
      "timestamp": "2024-01-13T10:30:00Z",
      "level": "ERROR",
      "message": "Database connection failed",
      "service": "api-server",
      "request_id": "req-123"
    },
    {
      "timestamp": "2024-01-13T10:29:55Z",
      "level": "WARN",
      "message": "Slow query detected",
      "service": "api-server"
    },
    {
      "timestamp": "2024-01-13T10:29:50Z",
      "level": "INFO",
      "message": "Request processed",
      "service": "api-server"
    }
  ],
  "elapsed_time_micros": 12500,
  "errors": []
}
```

#### Docker Compose for CI/CD (optional)
```yaml
version: '3.8'
services:
  quickwit:
    image: quickwit/quickwit:0.7
    ports:
      - "7280:7280"
    command: ["quickwit", "run", "--service", "searcher"]
    # Add test data initialization
```

---

## 7. Risk & Complexity Review

### Risks from Phase 1 - Mitigations Applied

| Risk | Phase 1 Mitigation | Design Implementation | Residual Risk |
|------|-------------------|----------------------|---------------|
| Quickwit API breaking changes | Version pin in docs, handle errors gracefully | Use `serde(default)` for all optional fields, log parse errors | LOW - Can only affect new Quickwit versions, doesn't break existing functionality |
| Network timeouts with large indexes | Configurable timeouts, return partial results | 10s default timeout, empty results on failure, log warning | LOW - Users can increase timeout in config |
| JSON parsing failures | Use `Option<T>` for non-essential fields | All response fields optional except `hits`, graceful parse error handling | VERY LOW - Defensive parsing |
| Concurrent request limits | Document rate limiting, implement retry | No retry (keep simple), return empty on 429 status, log warning | MEDIUM - Users must configure Quickwit capacity appropriately |
| API tokens exposed in logs/errors | Redact tokens, follow atomic_server_secret pattern | `redact_token()` helper shows only first 4 chars, never log full token | VERY LOW - Token security enforced |
| Unvalidated URLs allow SSRF | Validate base URL format, use allow-list | Log security warning for non-localhost HTTP, document HTTPS requirement | LOW - User responsibility for internal network security |
| Insecure HTTP exposes credentials | Enforce HTTPS, warn on HTTP | Log warning when HTTP used with non-localhost, don't block | MEDIUM - Can't enforce HTTPS (user might have valid dev setup) |
| Confusing configuration | Example configs, clear error messages | Example file with comments, descriptive error messages | LOW - Good documentation mitigates |
| Slow searches frustrate users | Progress indicators, timeout warnings | Log search start/complete, timeout after 10s with warning | LOW - Standard haystack behavior |
| Results formatting mismatch | Test with real users, iterate | Follow QueryRs pattern (proven), extract meaningful fields | LOW - Can iterate based on feedback |

### New Design-Phase Risks

| Risk | Impact | Likelihood | Mitigation | Residual |
|------|--------|-----------|------------|----------|
| Document ID collisions across indexes | MEDIUM | LOW | Include index name in ID: `quickwit_{index}_{doc_id}` | VERY LOW |
| Memory exhaustion with large JSON responses | HIGH | LOW | Default max_hits=100 per index, configurable, timeout prevents hanging | LOW |
| Auth token accidentally committed to git | HIGH | MEDIUM | Document: use environment variables, .gitignore example configs with real tokens | MEDIUM - User responsibility |
| Performance regression in search orchestration | MEDIUM | LOW | Async execution prevents blocking, 10s timeout limits impact | VERY LOW |
| Quickwit version incompatibility | MEDIUM | MEDIUM | Test with 0.7+, document version requirements, handle missing fields | LOW |
| Auto-discovery latency overhead | MEDIUM | HIGH | Explicit mode available for performance-critical use, parallel index searches with tokio::join! | LOW - Users choose mode based on needs |
| Failed index discovery breaks all searches | HIGH | LOW | Return empty vec on /v1/indexes failure, log warning, graceful degradation | VERY LOW |
| Glob pattern complexity confuses users | LOW | MEDIUM | Document pattern syntax clearly, provide examples, optional feature | LOW |

### Complexity Assessment

| Area | Complexity | Justification |
|------|-----------|---------------|
| HTTP API Integration | LOW | Similar to QueryRsHaystackIndexer, proven reqwest patterns |
| JSON Parsing | LOW | Well-structured Quickwit API, serde handles complexity |
| Document Transformation | LOW | Simple field mapping, no complex logic |
| Auto-Discovery Logic | MEDIUM | Three code paths (explicit, filtered, full), but clear branching |
| Concurrent Index Searches | MEDIUM | tokio::join! for parallelization, error handling per-index |
| Glob Pattern Matching | LOW | Simple string pattern matching, well-defined behavior |
| Error Handling | MEDIUM | Multiple failure modes (network, auth, parse, discovery), but pattern established |
| Testing | MEDIUM-HIGH | Requires external Quickwit server, multiple modes to test, Docker mitigates |
| Configuration | LOW | Reuses extra_parameters pattern, well-documented |

**Overall Complexity:** MEDIUM - Auto-discovery and concurrent searches add moderate complexity, but follow proven async Rust patterns and try_search reference implementation.

---

## 8. Open Questions / Decisions for Human Review

### High Priority (Blocking Implementation)

**Q1:** Quickwit Server Availability ✅ RESOLVED
Available Quickwit instance for testing:
- **URL:** `https://logs.terraphim.cloud/api/`
- **Authentication:** Basic Auth (username: "cloudflare", password: secret via wrangler)
- **Available Indexes:** `workers-logs`, `cadro-service-layer`
- **Version:** 0.7+ (inferred from API compatibility)
- **Development:** Use Trunk proxy to `/api/` or direct connection with auth

**Design Implication:** Support both Basic Auth and Bearer token. Test with real instance available.

---

**Q2:** Index Configuration Strategy ✅ RESOLVED
**Decision:** Option B selected - Implement hybrid approach in v1 with both explicit and auto-discovery.

**Implementation:**
- If `default_index` present in extra_parameters: search only that index (fast, explicit)
- If `default_index` absent: auto-discover via `GET /v1/indexes` and search all (convenient)
- Optional `index_filter` glob pattern for filtered auto-discovery

**Rationale:** Ship feature-complete from start, users choose mode based on needs (performance vs convenience).

**Trade-off Analysis:** See `.docs/quickwit-autodiscovery-tradeoffs.md` for detailed analysis.

---

**Q3:** Testing Strategy with External Dependencies ✅ CONFIRMED
For integration tests requiring Quickwit server:

**Options:**
- **A:** Docker Compose in CI/CD + mark tests with #[ignore] for local dev
- **B:** Only #[ignore] tests, document manual testing procedure
- **C:** Mock HTTP responses (violates no-mocks policy)

**Recommendation:** Option A - Docker Compose provides best balance of automation and policy compliance.

**Design Decision:** Proceeding with Option A - Docker Compose.

---

### Medium Priority (Can proceed with assumption)

**Q4:** Result Caching TTL
Should Quickwit results be cached? If yes, for how long?

**Options:**
- **A:** No caching (logs are time-sensitive)
- **B:** Short cache (5 minutes)
- **C:** Configurable cache (default 1 hour like QueryRs)

**Assumption:** Use Option C - let persistence layer handle caching with 1-hour default. Users can disable if needed.

---

**Q5:** Time Range Query Support
Phase 1 identified time range filtering from try_search. Should initial implementation support this?

**Options:**
- **A:** Include time range support in v1 (more complete but complex)
- **B:** Defer to v2, focus on basic search (simpler, faster to ship)

**Assumption:** Option B - defer time ranges to v2. Basic text search sufficient for initial release.

---

**Q6:** Error Notification Strategy
When Quickwit is unavailable, should users see:
- **A:** Silent empty results (current pattern)
- **B:** Warning message in CLI output
- **C:** Configurable per-role

**Assumption:** Option A - silent empty results with logged warnings. Consistent with other haystacks.

---

### Low Priority (Informational)

**Q7:** Field Mapping Details
Document structure confirmed:
- `id`: `quickwit_{index}_{quickwit_doc_id}`
- `title`: `[{level}] {message}` (first 100 chars)
- `body`: Full JSON string from hit
- `description`: `{timestamp} - {message}` (first 200 chars)
- `tags`: `["quickwit", "logs", "{level}"]`
- `rank`: Timestamp as microseconds (for sorting)

**Approved:** Proceed with this mapping.

---

**Q8:** Authentication Methods ✅ RESOLVED
**Decision:** Support both Bearer token and Basic Auth in v1.

**Rationale:** try_search uses Basic Auth (cloudflare/password), production systems often use Bearer tokens.

**Implementation:**
- Check `auth_token` first (Bearer)
- Fall back to `auth_username` + `auth_password` (Basic)
- Redact both in logs

---

**Q9:** Query Syntax Handling
Pass user queries directly to Quickwit without transformation.

**Rationale:** Quickwit handles query parsing, no need to reimplement. Document supported syntax for users.

**Approved:** Pass-through queries.

---

**Q10:** Naming Confirmation
- Haystack type: `Quickwit`
- Indexer: `QuickwitHaystackIndexer`
- Module: `crates/terraphim_middleware/src/haystack/quickwit.rs`
- Feature flag: None (always compiled)

**Approved:** These names follow Terraphim conventions.

---

## Summary

This design document provides a complete implementation plan for Quickwit haystack integration with hybrid index discovery and dual authentication support. Key characteristics:

- **Scope:** Well-bounded, follows established patterns, enhanced with auto-discovery
- **Complexity:** Medium - auto-discovery and concurrent index searches add moderate complexity
- **Risk:** Low-medium, mitigations in place for all identified risks
- **Testing:** Comprehensive strategy with Docker Compose, 14 acceptance criteria, 12 invariants
- **Deployment:** Incremental, 14 steps, each step deployable
- **Maintainability:** Reuses QueryRs patterns, follows try_search reference implementation
- **Flexibility:** Users choose explicit (fast) or auto-discovery (convenient) modes

**Key Features:**
- Hybrid index selection (explicit vs auto-discovery with optional glob filtering)
- Dual authentication (Bearer token + Basic Auth)
- Concurrent index searches with tokio parallelization
- Graceful error handling with detailed logging
- Compatible with Quickwit 0.7+ REST API

**Configuration from try_search:**
- Production URL: `https://logs.terraphim.cloud/api/`
- Basic Auth: username "cloudflare", password from secrets
- Available indexes: `workers-logs`, `cadro-service-layer`

**Next Phase:** After approval, proceed to Phase 3 (disciplined-implementation) to execute 14-step plan.

---

**End of Design Document**

*This document represents Phase 2 design and requires approval before implementation begins.*
