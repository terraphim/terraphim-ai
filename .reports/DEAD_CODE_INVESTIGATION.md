# Dead Code Investigation: query_rs.rs

## Issue Summary
The public `terraphim-ai` repository has dead code warnings for several methods in `crates/terraphim_middleware/src/haystack/query_rs.rs`:
- `should_fetch_url()` - line 48
- `get_fetched_count()` - line 72  
- `normalize_document_id()` - line 201
- `fetch_and_scrape_content()` - line 336
- `scrape_content()` - line 417
- `is_critical_url()` - line 1107

## Root Cause Analysis

### Comparison with terraphim-private Repository

The private repository at `/Users/alex/projects/terraphim/terraphim-private` contains a MORE COMPLETE implementation where these methods ARE actively used:

**Line 353 in private repo:**
```rust
match self.fetch_and_scrape_content(&enhanced_doc).await {
```

**Line 373 in private repo:**
```rust
if self.is_critical_url(&enhanced_doc.url) {
```

**Line 403 in private repo:**
```rust
let unique_urls_fetched = self.get_fetched_count();
```

**Line 351 in private repo:**
```rust
&& self.should_fetch_url(&enhanced_doc.url)
```

### What Happened

The public repository has a **SIMPLIFIED** version that:
1. Removed the content fetching/scraping logic (lines 346-399 in private)
2. Kept the helper methods but they're no longer called
3. This creates "dead code" that clippy detects

The private repository has:
- `FetchStats` struct (lines 14-25) - tracks fetch success/failure
- `PersistenceStats` struct (lines 28-41) - tracks cache hits/misses  
- Full content enhancement logic with `disable_content_enhancement` flag
- Active usage of all the "dead" methods

## Resolution Options

### Option 1: Copy Complete Implementation from Private Repo ✅ RECOMMENDED
**Pros:**
- Restores full functionality
- Methods are actually used
- Better feature parity
- More robust content fetching

**Cons:**
- May expose private features
- Larger codebase

### Option 2: Remove Dead Code (Quick Fix)
**Pros:**
- Clean, minimal codebase
- Passes clippy immediately

**Cons:**
- Loses functionality
- Can't easily re-add features later

### Option 3: Keep with `#[allow(dead_code)]` (Current Approach)
**Pros:**
- Preserves methods for future use
- Minimal changes

**Cons:**
- Keeps unused code
- Clutters codebase

## Recommended Action Plan

1. **Sync from Private Repo** - Copy the complete implementation:
   - Add `FetchStats` and `PersistenceStats` structs
   - Add `disable_content_enhancement` configuration support
   - Restore full content fetching logic (lines 246-400 from private)
   - Update logging to use `log::warn!` for better visibility

2. **Test Compatibility** - Ensure the enhanced version works with public config

3. **Update Documentation** - Document the `disable_content_enhancement` flag

## Files to Sync

**Source:** `/Users/alex/projects/terraphim/terraphim-private/crates/terraphim_middleware/src/haystack/query_rs.rs`

**Target:** `/Users/alex/projects/terraphim/terraphim-ai/crates/terraphim_middleware/src/haystack/query_rs.rs`

**Key Sections:**
- Lines 13-41: Add FetchStats and PersistenceStats structs
- Lines 74-103: Methods already present (remove `#[allow(dead_code)]`)
- Lines 246-400: Full content enhancement logic with stats tracking

## Immediate Fix (Before Release)

For the v1.0.0 release, we should either:
1. **Quickly sync from private** (15-20 min) - Better long-term
2. **Keep `#[allow(dead_code)]` annotations** - Current state works

Given time constraints for release, Option 2 is acceptable for now, but schedule Option 1 for v1.0.1.
