# GrepApp Feature

## Overview

The **grepapp** feature enables integration with [grep.app](https://grep.app), a search engine for code across GitHub repositories. This allows Terraphim AI to search through millions of open-source repositories as part of your knowledge graph.

## Status

**Available for**: Local development only (not in published crates)
**Feature Flag**: `--features grepapp`
**Dependency**: `haystack_grepapp` (local path dependency)

## Why Local Only?

The `grepapp` feature is **not enabled by default** in published crates because the `haystack_grepapp` dependency is not yet published to crates.io. This is intentional to allow active development while preparing for publication.

## Enabling the Feature

### For Development

To enable the grepapp feature during development:

```bash
# Build with grepapp enabled
cargo build --features grepapp

# Run tests with grepapp
cargo test --features grepapp

# Run the server with grepapp support
cargo run --release -- --config your_config.json --features grepapp
```

### For Cargo Workspace

If you're building multiple workspace crates, add grepapp to your feature list:

```bash
cargo build -p terraphim_middleware --features grepapp
cargo build -p terraphim_server --features grepapp
```

## Configuration

Once the feature is enabled, configure GrepApp as a haystack in your role configuration:

```json
{
  "name": "Rust Engineer",
  "haystacks": [
    {
      "location": "https://grep.app",
      "service": "GrepApp",
      "read_only": true,
      "extra_parameters": {
        "language": "Rust",
        "repo": "",
        "path": ""
      }
    }
  ]
}
```

### Configuration Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `location` | string | Yes | Must be `"https://grep.app"` |
| `service` | string | Yes | Must be `"GrepApp"` |
| `language` | string | No | Filter by programming language (e.g., "Rust", "Python") |
| `repo` | string | No | Filter by GitHub repo (e.g., "tokio-rs/tokio") |
| `path` | string | No | Filter by file path (e.g., "src/") |

## Usage Examples

### Search for Code Across All Rust Repositories

```json
{
  "haystacks": [
    {
      "location": "https://grep.app",
      "service": "GrepApp",
      "extra_parameters": {
        "language": "Rust",
        "repo": "",
        "path": ""
      }
    }
  ]
}
```

### Search in a Specific Repository

```json
{
  "haystacks": [
    {
      "location": "https://grep.app",
      "service": "GrepApp",
      "extra_parameters": {
        "language": "Rust",
        "repo": "tokio-rs/tokio",
        "path": "tokio/src/task"
      }
    }
  ]
}
```

## How It Works

1. **Search**: Terraphim sends your search query to grep.app's API
2. **Filter**: Results are filtered by language, repo, and path if specified
3. **Index**: Matching code snippets are indexed with unique IDs
4. **Rank**: Results are ranked using your configured relevance function

### Document ID Format

GrepApp documents use the following ID format:
```
grepapp:{repo}:{branch}:{path}
```

Example: `grepapp:tokio-rs_tokio_master_tokio_src_task_spawn.rs`

### Duplicate Handling

When using GrepApp together with other haystacks (like QueryRs), you may see duplicate results from the same repository. See [`docs/duplicate-handling.md`](../duplicate-handling.md) for details on how duplicates are managed.

## Implementation Details

### Code Location

- **Indexer**: `crates/terraphim_middleware/src/haystack/grep_app.rs`
- **Feature Guard**: `#[cfg(feature = "grepapp")]`
- **Service Type**: `ServiceType::GrepApp` (in `terraphim_config`)

### API Integration

The feature uses the `grepapp_haystack` crate which provides:
- `GrepAppClient`: HTTP client for grep.app API
- `SearchParams`: Query builder with filters
- `SearchResult`: Parsed API responses

### Error Handling

- **Network failures**: Return empty index (graceful degradation)
- **Invalid parameters**: Logged as warnings
- **Missing feature**: Logs clear warning if haystack configured but feature not enabled

## Testing

### Unit Tests

Run the GrepApp unit tests:

```bash
cargo test -p terraphim_middleware --lib --features grepapp
```

Tests include:
- `test_indexer_creation`: Verifies indexer can be instantiated
- `test_filter_extraction`: Tests language/repo/path filter extraction
- `test_empty_filters`: Tests empty string filter handling

### Integration Tests

There are currently no live integration tests for GrepApp. The unit tests use mock haystacks to avoid making real API calls.

## Troubleshooting

### Feature Not Enabled

**Symptom**: Search logs show "GrepApp haystack support not enabled"

**Solution**: Build with `--features grepapp`

### No Results Returned

**Symptom**: GrepApp haystack returns empty index

**Possible Causes**:
1. Network connectivity issues
2. grep.app API rate limiting
3. No matches for your query with specified filters

**Debug**: Check logs for "GrepApp search failed" messages

### Compilation Errors

**Symptom**: `cannot find GrepAppHaystackIndexer`

**Solution**: Ensure you're building with the grepapp feature enabled:
```bash
cargo build --features grepapp
```

## Future Work

### crates.io Publication

To make grepapp available in published crates:
1. Publish `haystack_grepapp` to crates.io
2. Update `Cargo.toml` dependency from path to crates.io version
3. Document the feature in the main README

### Integration Tests

Add live integration tests that:
1. Make real API calls to grep.app
2. Verify response parsing
3. Test error handling with bad queries

## Related Documentation

- [Duplicate Handling](../duplicate-handling.md) - How GrepApp interacts with other haystacks
- [Haystack Configuration](../configuration/haystacks.md) - General haystack setup
- [Service Types](../architecture/service-types.md) - All available haystack types

## Changelog

### 2026-01-13
- **Fixed**: Re-enabled grepapp feature for local development
- **Removed**: Dead code guards for atomic feature
- **Verified**: Zero compiler warnings with grepapp enabled

---

**Last Updated**: 2026-01-13
**Feature Status**: ✅ Available for local development
**Publication Status**: 🔄 Pending (requires haystack_grepapp publication)
