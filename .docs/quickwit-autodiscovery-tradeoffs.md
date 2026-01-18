# Quickwit Index Auto-Discovery: Trade-off Analysis

**Date:** 2026-01-13
**Context:** Design decision for Q2 in Quickwit haystack integration

---

## Configuration Context from try_search

**Quickwit Server:**
- URL: `https://logs.terraphim.cloud/api/`
- Authentication: Basic Auth (username: "cloudflare", password: secret)
- Development proxy: Trunk proxies `/api/` to Quickwit server
- Available indexes: `workers-logs`, `cadro-service-layer`

**API Pattern:**
```rust
// Auto-discovery endpoint
GET /v1/indexes
// Returns: array of index metadata including index_id

// Frontend implementation (from try_search/src/api.rs)
pub async fn get_available_indexes() -> Result<Vec<IndexInfo>, String> {
    let url = format!("{}/v1/indexes", QUICKWIT_URL);
    let response = Request::get(&url).send().await?;
    let indexes: Vec<serde_json::Value> = response.json().await?;

    // Extract index_id from each index
    let available = indexes.into_iter()
        .filter_map(|idx| {
            idx.get("index_config")
               .and_then(|c| c.get("index_id"))
               .and_then(|v| v.as_str())
               .map(|s| IndexInfo { index_id: s.to_string(), num_docs: 0 })
        })
        .collect();
    Ok(available)
}
```

---

## Option A: Explicit Configuration (Original Design)

### Description
Users explicitly specify index name in `extra_parameters`:
```json
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    "default_index": "workers-logs",
    "auth_token": "Bearer xyz"
  }
}
```

### Pros
1. **Performance:** No extra API call on initialization (one less network round-trip)
2. **Predictable:** Users know exactly which index will be searched
3. **Simpler error handling:** Missing index = clear error message immediately
4. **Configuration as code:** Index selection version-controlled, auditable
5. **Multi-index via multi-haystack:** Users can add multiple Quickwit haystacks for different indexes
6. **Fails fast:** Invalid index name errors immediately vs. silently excluded from results
7. **Lower API usage:** Doesn't query `/v1/indexes` on every search initialization

### Cons
1. **Manual setup:** Users must know index names beforehand
2. **No discovery:** Can't browse available indexes from Terraphim
3. **Stale config:** If index renamed/deleted, config becomes invalid
4. **Verbose for multiple indexes:** Requires N haystack configs for N indexes
5. **No validation:** Can't verify index exists until search time

---

## Option B: Auto-Discovery Only

### Description
Always fetch available indexes from Quickwit and search all of them:
```json
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    "auth_token": "Bearer xyz"
  }
}
```

### Pros
1. **Zero configuration:** Users only provide Quickwit URL + auth
2. **Discovery:** Automatically finds all searchable indexes
3. **Resilient to changes:** New indexes automatically included
4. **Simpler config:** One haystack config searches all indexes
5. **User-friendly:** No need to know index names beforehand

### Cons
1. **Performance overhead:** Extra API call (`GET /v1/indexes`) on every search
2. **Thundering herd:** Searches ALL indexes concurrently (N HTTP requests per query)
3. **Result pollution:** Irrelevant indexes mixed with desired results
4. **Timeout risk:** If one index is slow, entire search delayed (unless timeout per-index)
5. **Error handling complexity:** One index failing shouldn't fail entire search
6. **Unclear UX:** Users don't know which indexes were searched
7. **Higher API usage:** More requests to Quickwit = higher load/costs
8. **No filtering:** Can't limit to specific indexes (everything searches)

---

## Option C: Hybrid (Auto-Discovery with Optional Override) ⭐ RECOMMENDED

### Description
Support both patterns with auto-discovery as default:
```json
// Explicit index (Option A)
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    "default_index": "workers-logs",  // ← Specified
    "auth_token": "Bearer xyz"
  }
}

// Auto-discovery (Option B)
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    // No default_index = auto-discover
    "auth_token": "Bearer xyz"
  }
}

// Filtered auto-discovery (Option C enhanced)
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    "index_filter": "logs-*",  // ← Glob pattern
    "auth_token": "Bearer xyz"
  }
}
```

### Implementation Logic
```rust
async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
    let config = self.parse_config(haystack);

    let indexes_to_search: Vec<String> = if let Some(index) = config.default_index {
        // Explicit: search single index
        vec![index]
    } else if let Some(pattern) = config.index_filter {
        // Filtered auto-discovery
        let all_indexes = self.fetch_available_indexes(&config).await?;
        all_indexes.into_iter()
            .filter(|idx| matches_glob(&idx.index_id, &pattern))
            .map(|idx| idx.index_id)
            .collect()
    } else {
        // Full auto-discovery
        let all_indexes = self.fetch_available_indexes(&config).await?;
        all_indexes.into_iter()
            .map(|idx| idx.index_id)
            .collect()
    };

    // Search all selected indexes concurrently
    let mut all_results = Index::new();
    for index in indexes_to_search {
        let index_results = self.search_single_index(needle, &index, &config).await?;
        all_results.extend(index_results);
    }
    Ok(all_results)
}
```

### Pros
1. **Flexibility:** Users choose based on their needs
2. **Best of both worlds:** Performance when explicit, convenience when auto-discover
3. **Progressive enhancement:** Start explicit, enable discovery later
4. **Filtered discovery:** `index_filter` pattern (e.g., "logs-*") balances discovery and control
5. **Backward compatible:** Existing configs work (explicit default_index)
6. **Graceful degradation:** If discovery fails, fall back to configured index

### Cons
1. **Implementation complexity:** Three code paths instead of one
2. **More testing required:** Test all three scenarios
3. **Documentation burden:** Must explain all three modes
4. **Potential confusion:** Users might not understand when discovery happens

---

## Performance Comparison

### Scenario: Search query "error" with 3 indexes available

| Approach | API Calls | Latency | Network Impact |
|----------|-----------|---------|----------------|
| **Explicit** | 1 search request | 100ms | Minimal |
| **Auto-discovery** | 1 list + 3 search = 4 requests | 100ms (list) + 300ms (3 parallel searches) = 400ms | High |
| **Hybrid (explicit)** | 1 search request | 100ms | Minimal |
| **Hybrid (discovery)** | 1 list + 3 search = 4 requests | 400ms | High |
| **Hybrid (filtered)** | 1 list + 2 search = 3 requests | 300ms | Medium |

**Impact:** Auto-discovery adds 3-4x latency and API load.

---

## Recommendation Matrix

| User Type | Recommended Approach | Rationale |
|-----------|---------------------|-----------|
| **Single index user** | Explicit | Fastest, simplest, no overhead |
| **Developer exploring logs** | Auto-discovery | Convenience over performance |
| **Production monitoring** | Explicit | Predictable, fast, controlled |
| **Multi-tenant system** | Filtered discovery | Balance control and convenience |
| **CI/CD logs** | Explicit | Known index names, performance critical |

---

## Final Recommendation

**Implement Option C (Hybrid) with smart defaults:**

1. **v1 Implementation:** Support explicit `default_index` (simple, fast)
2. **v1.1 Enhancement:** Add auto-discovery when `default_index` absent
3. **v2 Feature:** Add `index_filter` glob pattern support

**Rationale:**
- Incremental implementation reduces risk
- v1 delivers value immediately (explicit mode)
- v1.1 adds convenience without breaking existing configs
- v2 adds advanced filtering for power users
- Each version is independently useful

**Configuration Validation:**
```rust
// v1: Explicit only
if default_index.is_none() {
    return Err(Error::MissingParameter("default_index"));
}

// v1.1: Auto-discovery fallback
let indexes = if let Some(idx) = default_index {
    vec![idx]
} else {
    self.fetch_available_indexes(&config).await?
        .into_iter()
        .map(|i| i.index_id)
        .collect()
};

// v2: Add index_filter
let indexes = if let Some(idx) = default_index {
    vec![idx]
} else if let Some(pattern) = index_filter {
    self.fetch_and_filter_indexes(&config, &pattern).await?
} else {
    self.fetch_available_indexes(&config).await?
        .into_iter()
        .map(|i| i.index_id)
        .collect()
};
```

---

## User Decision Required

**Question:** Which version timeline do you prefer?

- **A:** Ship v1 with explicit only, add auto-discovery in v1.1 (safer, incremental)
- **B:** Ship v1.1 with both explicit and auto-discovery (faster to feature-complete)
- **C:** Skip explicit, ship auto-discovery only (simplest code, higher latency)

**My recommendation:** Option A - ship explicit mode first, validate with real usage, then add auto-discovery based on feedback.
