# Research Document: Quickwit Haystack Integration for Terraphim AI

**Date:** 2026-01-13
**Phase:** 1 - Research and Problem Understanding
**Status:** Draft - Awaiting Quality Evaluation

---

## 1. Problem Restatement and Scope

### Problem Statement
Integrate Quickwit search engine as a new haystack type in Terraphim AI to enable log and observability data search alongside existing haystacks (Ripgrep, QueryRs, ClickUp, etc.). The integration should follow established Terraphim patterns and enable users to search their Quickwit indexes from terraphim-agent CLI.

### IN Scope
1. Creating a `QuickwitHaystackIndexer` that implements the `IndexMiddleware` trait
2. Adding `Quickwit` variant to the `ServiceType` enum in terraphim_config
3. Implementing Quickwit REST API client for:
   - Listing available indexes (`GET /v1/indexes`)
   - Searching indexes (`GET /v1/{index_id}/search`)
4. Configuration support for Quickwit connection parameters (URL, authentication)
5. Integration with existing search orchestration in `terraphim_middleware::indexer::search_haystacks`
6. Unit tests for the Quickwit indexer in `terraphim_middleware`
7. Integration tests in terraphim-agent demonstrating end-to-end search
8. Documentation of configuration format and usage patterns

### OUT of Scope
1. Quickwit server installation or deployment automation
2. Index creation, management, or ingestion pipelines
3. Quickwit cluster configuration or multi-node setup
4. Real-time log streaming or tailing functionality
5. Quickwit-specific query syntax beyond basic search
6. UI components for Quickwit integration (terraphim-cli is CLI-only)
7. Modifications to the try_search frontend code (separate codebase)
8. Advanced Quickwit features (aggregations, faceting, time-series analytics)

---

## 2. User & Business Outcomes

### User-Visible Changes
1. Users can add Quickwit as a haystack in their role configuration
2. Search queries return results from Quickwit indexes alongside other sources
3. Users can configure Quickwit connection via JSON config or environment variables
4. The terraphim-agent CLI displays Quickwit search results with:
   - Timestamp-sorted log entries
   - Hit count and query performance metrics
   - Full JSON document content when available

### Business Value
1. Extends Terraphim AI's search capabilities to observability and log data
2. Enables unified search across code, docs, issues, and operational logs
3. Leverages Quickwit's cloud-native search for large-scale log volumes
4. Maintains consistent UX across all haystack types

---

## 3. System Elements and Dependencies

### New Components
| Component | Location | Responsibility |
|-----------|----------|---------------|
| `QuickwitHaystackIndexer` | `crates/terraphim_middleware/src/haystack/quickwit.rs` | Implements IndexMiddleware for Quickwit REST API |
| `ServiceType::Quickwit` | `crates/terraphim_config/src/lib.rs` | Enum variant for Quickwit service type |
| Quickwit integration tests | `crates/terraphim_middleware/tests/quickwit_haystack_test.rs` | Integration tests for Quickwit indexer |
| Agent CLI tests | `crates/terraphim_agent/tests/quickwit_integration_test.rs` | End-to-end tests via terraphim-agent |

### Existing Components (Modified)
| Component | Modifications Required |
|-----------|----------------------|
| `terraphim_config::ServiceType` | Add `Quickwit` variant |
| `terraphim_middleware::indexer::mod.rs` | Add `ServiceType::Quickwit` match arm |
| `terraphim_middleware::haystack::mod.rs` | Export `QuickwitHaystackIndexer` |
| `Cargo.toml` (workspace) | Ensure reqwest with json/rustls-tls features |

### Dependencies
- **Internal:** terraphim_types, terraphim_config, terraphim_persistence, terraphim_middleware
- **External:**
  - `reqwest` (HTTP client) - already in use
  - `serde_json` (JSON parsing) - already in use
  - `async_trait` - already in use

### Data Flow
```
User Query (terraphim-agent)
    ↓
terraphim_middleware::indexer::search_haystacks()
    ↓
QuickwitHaystackIndexer::index(needle, haystack)
    ↓
Quickwit REST API (GET /v1/{index_id}/search?query=...)
    ↓
Parse JSON response → Vec<Document>
    ↓
Return Index (HashMap<String, Document>)
    ↓
Merge with other haystack results
    ↓
Display in terraphim-agent CLI
```

---

## 4. Constraints and Their Implications

### Technical Constraints
1. **REST API Only:** Quickwit client must use REST API (no native gRPC client)
   - *Implication:* Simpler implementation but potentially higher latency than gRPC

2. **Async Rust:** Must follow tokio async patterns used throughout Terraphim
   - *Implication:* All HTTP calls must be async, consistent with existing indexers

3. **No Mocks in Tests:** Project policy forbids mocks
   - *Implication:* Integration tests require running Quickwit server or use conditional `#[ignore]` tests

4. **Feature Gates:** Optional dependencies should use feature flags
   - *Implication:* Consider if Quickwit should be optional (probably not - reqwest already required)

### Configuration Constraints
1. **JSON-based Config:** Must fit existing Haystack structure
   - *Implication:* Use `extra_parameters` for Quickwit-specific settings (URL, auth token, index name)

2. **Secret Management:** API keys/tokens should not be serialized inappropriately
   - *Implication:* Follow atomic_server_secret pattern with conditional serialization

### Performance Constraints
1. **Search Timeout:** Quickwit queries should timeout gracefully
   - *Implication:* Use reqwest timeout (10s default like QueryRsHaystackIndexer)

2. **Result Limits:** Prevent overwhelming results from large log indexes
   - *Implication:* Default to `max_hits=100` like try_search, make configurable

### Security Constraints
1. **Authentication:** Quickwit may require bearer token authentication
   - *Implication:* Support optional authentication header in requests

2. **HTTPS:** Production Quickwit deployments use HTTPS
   - *Implication:* Use rustls-tls (not native-tls) for consistent TLS handling

---

## 5. Risks, Unknowns, and Assumptions

### Unknowns
1. **Quickwit Response Schema Variations:** Do different Quickwit versions return different JSON schemas?
   - *De-risking:* Test with Quickwit 0.7+ (latest stable), document version compatibility

2. **Authentication Mechanisms:** Beyond bearer tokens, does Quickwit support other auth?
   - *De-risking:* Start with bearer token (most common), add OAuth2/mTLS later if needed

3. **Query Syntax Compatibility:** Does Quickwit query syntax differ significantly from other search engines?
   - *De-risking:* Document supported query patterns, default to simple text search

4. **Performance at Scale:** How does Quickwit perform with millions of documents?
   - *De-risking:* Test with realistic dataset sizes, implement pagination if needed

### Assumptions
1. **ASSUMPTION:** Quickwit servers are deployed and accessible via HTTP(S)
   - *Validation:* Document setup prerequisites in README

2. **ASSUMPTION:** Users know their Quickwit index names beforehand
   - *Validation:* Could implement index discovery (`GET /v1/indexes`) if needed

3. **ASSUMPTION:** Quickwit REST API is stable across 0.7.x versions
   - *Validation:* Test with Quickwit 0.7.0, 0.7.1, 0.7.2

4. **ASSUMPTION:** JSON response fields (num_hits, hits, elapsed_time_micros) are consistent
   - *Validation:* Parse with serde, handle missing fields gracefully

5. **ASSUMPTION:** terraphim-agent is the correct binary name (not terraphim-cli)
   - *Validation:* CONFIRMED - binary is `terraphim-agent` from `terraphim_agent` crate

### Risks

#### Technical Risks
| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| Quickwit API breaking changes | High | Low | Version pin in docs, handle errors gracefully |
| Network timeouts with large indexes | Medium | Medium | Implement configurable timeouts, return partial results |
| JSON parsing failures from unexpected schema | Medium | Low | Use `serde(default)` and `Option<T>` for all non-essential fields |
| Concurrent request limits | Low | Low | Document rate limiting, implement retry with backoff |

#### Product/UX Risks
| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| Confusing configuration for users | Medium | Medium | Provide example configs, clear error messages |
| Slow searches frustrate users | Medium | Medium | Show progress indicators, implement timeout warnings |
| Results formatting doesn't match log UX expectations | Low | Medium | Test with real users, iterate on display format |

#### Security Risks
| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| API tokens exposed in logs/errors | High | Low | Redact tokens in error messages, follow atomic_server_secret pattern |
| Unvalidated URLs allow SSRF | High | Low | Validate Quickwit base URL format, use allow-list if possible |
| Insecure HTTP exposes credentials | Medium | Low | Enforce HTTPS in production, warn on HTTP connections |

---

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Multiple Haystacks:** Terraphim supports 8 haystack types (Ripgrep, Atomic, QueryRs, ClickUp, Mcp, Perplexity, GrepApp, AiAssistant)
   - Adding Quickwit increases maintenance burden

2. **Async Coordination:** Search orchestration coordinates multiple concurrent haystack queries
   - Quickwit must integrate without blocking other searches

3. **Error Handling Diversity:** Each haystack fails differently (network errors, auth failures, rate limits)
   - Quickwit errors must be handled consistently with graceful degradation

### Simplification Strategies
1. **Reuse Existing Patterns:**
   - Follow `QueryRsHaystackIndexer` structure (similar HTTP API integration)
   - Reuse `reqwest::Client` configuration with timeout/user-agent
   - Use `cached::proc_macro::cached` for result caching if beneficial

2. **Minimal Configuration:**
   - Default to sensible values (max_hits=100, timeout=10s, sort_by=-timestamp)
   - Only require base URL and optional auth token in config
   - Auto-discover indexes vs. requiring explicit index names

3. **Graceful Degradation:**
   - Return empty Index on failure (like other haystacks)
   - Log warnings but don't crash search pipeline
   - Provide clear error messages for common issues (connection refused, auth failure)

---

## 7. Questions for Human Reviewer

1. **Quickwit Deployment Context:** Do you have a running Quickwit instance available for testing? If yes, what version and how is authentication configured?

2. **Index Discovery vs. Configuration:** Should users explicitly configure index names in their haystacks, or should we auto-discover available indexes from `GET /v1/indexes`?

3. **Authentication Priority:** Which authentication method is most important to support first?
   - Bearer token (HTTP header)
   - Basic auth (username/password)
   - No auth (development/localhost)

4. **Error Handling Philosophy:** For network/auth failures, should we:
   - Return empty results silently (current haystack pattern)
   - Display warnings to user (better UX but more noise)
   - Make configurable per-role

5. **Testing Strategy:** For integration tests requiring a running Quickwit server, should we:
   - Use Docker to spin up Quickwit in CI/CD
   - Mark tests as `#[ignore]` and document manual testing
   - Use fixtures with pre-recorded JSON responses (violates no-mocks policy?)

6. **Result Caching:** Should Quickwit results be cached like QueryRsHaystackIndexer does (1-hour TTL)? Logs are time-sensitive but caching improves performance.

7. **Field Mapping:** How should Quickwit log fields map to Terraphim's Document structure?
   - `id`: Use Quickwit document ID or generate from timestamp+index?
   - `title`: Extract from log message or use index name?
   - `body`: Full JSON document or just the message field?
   - `description`: First N chars of message or structured summary?

8. **Time Range Queries:** Should we support time-based filtering (start_time/end_time from try_search)? This would require:
   - Passing time parameters through SearchQuery
   - Modifying needle to include time range
   - Or add time range to extra_parameters

9. **Query Syntax:** Should we pass user queries directly to Quickwit or sanitize/transform them? Quickwit supports:
   - Simple text search: `error`
   - Boolean operators: `error AND auth`
   - Field-specific: `level:ERROR`
   - Range queries: `timestamp:[2024-01-01 TO 2024-01-31]`

10. **Naming Conventions:** Confirm naming:
    - Haystack type: `Quickwit` (not `QuickWit` or `quickwit`)
    - Indexer: `QuickwitHaystackIndexer`
    - Module: `crates/terraphim_middleware/src/haystack/quickwit.rs`
    - Feature flag: None (always compiled) or `quickwit` optional?

---

## Implementation Reference: try_search Analysis

### Key Findings from try_search/src/api.rs

**API Patterns:**
```rust
// Endpoint format
GET /api/v1/{index_id}/search?query={query}&max_hits=100&sort_by=-timestamp

// Response structure
{
  "num_hits": 1234,
  "hits": [ {...}, {...} ],  // Array of JSON documents
  "elapsed_time_micros": 45000,
  "errors": []
}

// Index listing
GET /api/v1/indexes
// Returns array of index metadata
```

**Time Range Handling:**
```rust
// Format: "timestamp:[start TO end]"
// Dates: RFC3339 with :00Z suffix
format!("timestamp:[{}:00Z TO {}:00Z]", start_time, end_time)
```

**Query Construction:**
```rust
// Combine text search + time range with AND
let query_parts = vec![
    "error",  // Text search
    "timestamp:[2024-01-01:00Z TO 2024-01-31:00Z]"
];
let final_query = query_parts.join(" AND ");
```

**Authentication:**
- try_search uses proxy (`/api`) which handles Quickwit auth
- For direct integration, need to support Bearer token in HTTP headers

---

## Next Steps (Phase 2: Design)

After this research document is approved:

1. **Design Document:** Create detailed implementation plan with:
   - File-by-file changes
   - API interface definitions
   - Configuration schema
   - Error handling strategy
   - Test plan with specific test cases

2. **Prototype:** Quick spike implementation to validate:
   - Quickwit API client basic functionality
   - JSON parsing with real Quickwit responses
   - Integration into search pipeline

3. **Test Infrastructure:** Set up:
   - Docker Compose for local Quickwit testing
   - Sample dataset for realistic testing
   - CI/CD integration strategy

---

## Appendix: Reference Implementations

### A. QueryRsHaystackIndexer Patterns to Reuse
- HTTP client configuration (timeout, user-agent)
- Async trait implementation
- Document ID normalization
- Persistence/caching strategy
- Error handling with graceful degradation
- Progress logging

### B. Haystack Configuration Example
```json
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "read_only": true,
  "fetch_content": false,
  "extra_parameters": {
    "auth_token": "Bearer xyz123",
    "default_index": "workers-logs",
    "max_hits": "100",
    "sort_by": "-timestamp"
  }
}
```

### C. Expected Document Transformation
```
Quickwit Hit (JSON):
{
  "timestamp": "2024-01-13T10:30:00Z",
  "level": "ERROR",
  "message": "Database connection failed",
  "service": "api-server",
  "request_id": "abc-123"
}

↓ Transform to ↓

Terraphim Document:
{
  "id": "quickwit_workers_logs_abc123",
  "title": "[ERROR] Database connection failed",
  "body": "{\"timestamp\":\"2024-01-13T10:30:00Z\",...}",  // Full JSON
  "url": "http://localhost:7280/v1/workers-logs/...",
  "description": "2024-01-13 10:30:00 - Database connection failed",
  "tags": ["quickwit", "logs", "ERROR"],
  "rank": Some(timestamp_micros),  // For sorting
  "source_haystack": "http://localhost:7280"
}
```

---

**End of Research Document**

*This document represents Phase 1 understanding and will be followed by Phase 2 design after approval and quality evaluation.*
