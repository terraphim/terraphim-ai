# Grep.app Haystack

Grep.app integration for Terraphim AI, enabling code search across millions of GitHub repositories.

## Overview

This crate provides a haystack provider for [grep.app](https://grep.app), a code search engine by Vercel that indexes millions of public GitHub repositories. It allows you to search for code patterns, functions, and implementations across a massive codebase directly from Terraphim AI.

## Features

- **Fast Code Search**: Search across 500,000+ GitHub repositories
- **Language Filtering**: Filter results by programming language (Rust, Python, JavaScript, etc.)
- **Repository Filtering**: Narrow searches to specific repositories (e.g., "tokio-rs/tokio")
- **Path Filtering**: Search within specific directories
- **Rate Limiting**: Automatic handling of API rate limits
- **Error Handling**: Graceful degradation on failures

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
grepapp_haystack = { path = "../haystack_grepapp" }
haystack_core = { path = "../haystack_core" }
terraphim_types = { path = "../terraphim_types" }
```

## Usage

### Basic Search

```rust
use grepapp_haystack::GrepAppHaystack;
use haystack_core::HaystackProvider;
use terraphim_types::SearchQuery;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let haystack = GrepAppHaystack::new()?;

    let query = SearchQuery {
        search_term: "async fn tokio::spawn".into(),
        ..Default::default()
    };

    let documents = haystack.search(&query).await?;

    for doc in documents {
        println!("{} - {}", doc.title, doc.url);
    }

    Ok(())
}
```

### Search with Filters

```rust
use grepapp_haystack::GrepAppHaystack;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create haystack with default filters
    let haystack = GrepAppHaystack::with_filters(
        Some("Rust".to_string()),           // Language filter
        Some("tokio-rs/tokio".to_string()), // Repository filter
        Some("tokio/src/".to_string()),     // Path filter
    )?;

    let query = SearchQuery {
        search_term: "JoinHandle".into(),
        ..Default::default()
    };

    let documents = haystack.search(&query).await?;

    Ok(())
}
```

### Using the Low-Level Client

```rust
use grepapp_haystack::{GrepAppClient, SearchParams};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = GrepAppClient::new()?;

    let params = SearchParams {
        query: "tokio spawn".to_string(),
        language: Some("Rust".to_string()),
        repo: None,
        path: None,
    };

    let hits = client.search(&params).await?;

    for hit in hits {
        println!("Repo: {}", hit.source.repo.raw);
        println!("File: {}", hit.source.path.raw);
        println!("Branch: {}", hit.source.branch.raw);
        println!("Snippet: {}", hit.source.content.snippet);
    }

    Ok(())
}
```

## Configuration

### Terraphim Role Configuration

Add grep.app as a haystack in your role configuration:

```json
{
  "name": "Code Search Engineer",
  "relevance_function": "BM25",
  "theme": "default",
  "haystacks": [
    {
      "name": "GitHub Code Search",
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

### API Parameters

- **`query`** (required): Search query string (max 1000 characters)
- **`language`** (optional): Programming language filter (e.g., "Rust", "Python", "JavaScript")
- **`repo`** (optional): Repository filter in "owner/repo" format (e.g., "tokio-rs/tokio")
- **`path`** (optional): Path filter for directory-specific searches (e.g., "src/")

## Response Format

Each search result is converted to a `Document` with:

- **`id`**: Unique identifier (format: `repo:branch:path`)
- **`url`**: GitHub blob URL to the file
- **`title`**: Formatted as "repo - filename"
- **`body`**: Code snippet with matches (HTML tags stripped)
- **`description`**: Human-readable description
- **`tags`**: Repository name and filename

## Error Handling

The client handles various error conditions:

- **Rate Limiting (429)**: Returns error with message "Rate limit exceeded"
- **No Results (404)**: Returns empty vector instead of error
- **Network Errors**: Propagates with context
- **Invalid Queries**: Validates query length and emptiness

## Testing

Run the test suite:

```bash
# Run all tests
cargo test -p grepapp_haystack

# Run with output
cargo test -p grepapp_haystack -- --nocapture

# Run specific test
cargo test -p grepapp_haystack test_search_success
```

## Examples

### Search for Error Handling Patterns

```rust
let haystack = GrepAppHaystack::with_filters(
    Some("Rust".to_string()),
    None,
    None,
)?;

let query = SearchQuery {
    search_term: "Result<T, E>".into(),
    ..Default::default()
};

let documents = haystack.search(&query).await?;
```

### Find Specific Function Implementations

```rust
let haystack = GrepAppHaystack::with_filters(
    Some("Go".to_string()),
    Some("kubernetes/kubernetes".to_string()),
    Some("pkg/".to_string()),
)?;

let query = SearchQuery {
    search_term: "func NewController".into(),
    ..Default::default()
};

let documents = haystack.search(&query).await?;
```

## Limitations

- **Rate Limits**: grep.app enforces rate limits on API requests
- **No Authentication**: grep.app API currently doesn't require authentication
- **Public Repositories Only**: Only searches public GitHub repositories
- **No Regex Support**: Search is text-based, not regex-based (though grep.app may support some patterns)

## API Reference

grep.app uses the following API endpoint:

- **Endpoint**: `https://grep.app/api/search`
- **Method**: GET
- **Parameters**: `q`, `f.lang`, `f.repo`, `f.path`
- **Response**: JSON with `facets` and `hits`

For more details, see the [models.rs](src/models.rs) file for the complete response structure.

## Contributing

When extending this crate:

1. Add tests for new functionality
2. Update this README with new features
3. Follow Rust naming conventions (snake_case)
4. Use `tracing` for logging, not `println!`

## License

MIT

## Links

- [grep.app](https://grep.app) - Official website
- [Terraphim AI](https://github.com/terraphim/terraphim-ai) - Main project
