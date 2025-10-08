# Documentation Standards

## Code Comments
#documentation #comments #code-clarity

Comments should explain **why**, not **what**. Code should be self-explanatory for the "what".

**Good Examples:**
```rust
// Bad: States the obvious
// Set timeout to 30 seconds
let timeout = Duration::from_secs(30);

// Good: Explains reasoning
// Use 30 second timeout to prevent hanging on slow networks
// while still allowing large file uploads to complete
let timeout = Duration::from_secs(30);

// Good: Explains non-obvious business logic
// We retry up to 3 times because the payment gateway
// sometimes returns 500 errors on successful charges
const MAX_RETRY_ATTEMPTS: usize = 3;
```

**When to Comment:**
1. **Why** a particular approach was chosen
2. **Trade-offs** and alternatives considered
3. **Workarounds** for bugs or limitations
4. **Context** from requirements or business rules
5. **TODOs** with issue tracker references

**When NOT to Comment:**
1. Obvious code that matches function name
2. Restating what types already convey
3. Commented-out code (delete it, use git)
4. Obsolete comments that don't match code

---

## Function Documentation
#documentation #rustdoc #docstrings #jsdoc

Document public APIs with examples and edge cases.

### Rust (rustdoc)
#rust #rustdoc #documentation

```rust
/// Searches for documents matching the given query.
///
/// This function performs semantic search using the knowledge graph
/// to expand queries with related concepts.
///
/// # Arguments
///
/// * `query` - The search term to look for
/// * `role` - The role context for search (uses default if None)
/// * `limit` - Maximum number of results to return
///
/// # Returns
///
/// A `Result` containing:
/// - `Ok(SearchResults)` - Documents matching the query with relevance scores
/// - `Err(ServiceError)` - If the search fails or role is invalid
///
/// # Errors
///
/// This function will return an error if:
/// - The role does not exist
/// - The knowledge graph is not initialized
/// - The search backend is unavailable
///
/// # Examples
///
/// ```rust
/// # use terraphim_service::*;
/// # async fn example() -> Result<()> {
/// let service = TerraphimService::new(config);
/// let results = service.search("async patterns", Some("Rust Engineer"), 10).await?;
///
/// for doc in results.documents {
///     println!("{}: {}", doc.url, doc.description);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Performance
///
/// This function typically completes in 50-200ms depending on:
/// - Knowledge graph size (larger graphs take longer)
/// - Number of matching documents
/// - Whether results are cached
///
/// # See Also
///
/// - [`autocomplete_terms`] for term suggestions
/// - [`find_related_concepts`] for graph traversal
pub async fn search(
    &self,
    query: &str,
    role: Option<&str>,
    limit: usize,
) -> Result<SearchResults>
```

### Python (docstrings)
#python #docstrings #documentation

```python
def search(
    query: str,
    role: str | None = None,
    limit: int = 10
) -> SearchResults:
    """Search for documents matching the given query.
    
    This function performs semantic search using the knowledge graph
    to expand queries with related concepts.
    
    Args:
        query: The search term to look for
        role: The role context for search (uses default if None)
        limit: Maximum number of results to return
    
    Returns:
        SearchResults containing documents with relevance scores
    
    Raises:
        ValueError: If the role does not exist
        ServiceError: If the search backend is unavailable
    
    Examples:
        >>> service = TerraphimService(config)
        >>> results = service.search("async patterns", role="Rust Engineer")
        >>> for doc in results.documents:
        ...     print(f"{doc.url}: {doc.description}")
    
    Note:
        This function typically completes in 50-200ms depending on
        knowledge graph size and result caching.
    
    See Also:
        - autocomplete_terms: For term suggestions
        - find_related_concepts: For graph traversal
    """
```

### TypeScript (JSDoc)
#typescript #jsdoc #documentation

```typescript
/**
 * Searches for documents matching the given query.
 *
 * This function performs semantic search using the knowledge graph
 * to expand queries with related concepts.
 *
 * @param query - The search term to look for
 * @param role - The role context for search (uses default if undefined)
 * @param limit - Maximum number of results to return
 *
 * @returns A Promise resolving to SearchResults with matching documents
 *
 * @throws {ValueError} If the role does not exist
 * @throws {ServiceError} If the search backend is unavailable
 *
 * @example
 * ```typescript
 * const service = new TerraphimService(config);
 * const results = await service.search("async patterns", "Rust Engineer", 10);
 *
 * for (const doc of results.documents) {
 *   console.log(`${doc.url}: ${doc.description}`);
 * }
 * ```
 *
 * @remarks
 * This function typically completes in 50-200ms depending on
 * knowledge graph size and result caching.
 *
 * @see {@link autocompleteTerms} for term suggestions
 * @see {@link findRelatedConcepts} for graph traversal
 */
async function search(
  query: string,
  role?: string,
  limit: number = 10
): Promise<SearchResults>
```

---

## Module Documentation
#modules #packages #overview

Every module should have a top-level comment explaining its purpose.

### Rust
```rust
//! # User Session Management
//!
//! This module provides functionality for managing user sessions,
//! including authentication, authorization, and session lifecycle.
//!
//! ## Features
//!
//! - JWT-based authentication
//! - Role-based access control (RBAC)
//! - Automatic session refresh
//! - Multi-device session management
//!
//! ## Usage
//!
//! ```rust
//! use user_session::{SessionManager, SessionConfig};
//!
//! let config = SessionConfig::default();
//! let manager = SessionManager::new(config);
//!
//! let session = manager.create_session(&user).await?;
//! ```
//!
//! ## Architecture
//!
//! Sessions are stored in Redis with TTL-based expiration.
//! Each session contains:
//! - User ID
//! - Device fingerprint
//! - Permissions snapshot
//! - Expiration timestamp

pub mod session_manager;
pub mod session_store;
pub mod authentication;
```

### Python
```python
"""User Session Management

This module provides functionality for managing user sessions,
including authentication, authorization, and session lifecycle.

Features:
    - JWT-based authentication
    - Role-based access control (RBAC)
    - Automatic session refresh
    - Multi-device session management

Example:
    >>> from user_session import SessionManager, SessionConfig
    >>> config = SessionConfig()
    >>> manager = SessionManager(config)
    >>> session = manager.create_session(user)

Architecture:
    Sessions are stored in Redis with TTL-based expiration.
    Each session contains:
    - User ID
    - Device fingerprint
    - Permissions snapshot
    - Expiration timestamp
"""

from .session_manager import SessionManager
from .session_store import SessionStore
from .authentication import authenticate
```

---

## Type Documentation
#types #structs #interfaces

Document complex types with field explanations.

### Rust
```rust
/// Configuration for the Terraphim service.
///
/// This struct defines all runtime settings including knowledge graph
/// configuration, LLM providers, and haystack data sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// The role name for this configuration.
    /// Must match an entry in the roles map.
    pub role: RoleName,
    
    /// Knowledge graph configuration.
    /// If None, knowledge graph features are disabled.
    pub kg: Option<KnowledgeGraphConfig>,
    
    /// Data sources for search and indexing.
    /// At least one haystack is required for search to work.
    pub haystacks: Vec<HaystackConfig>,
    
    /// LLM provider: "ollama", "openrouter", or custom.
    pub llm_provider: String,
    
    /// Base URL for LLM API.
    /// For Ollama: "http://127.0.0.1:11434"
    /// For OpenRouter: "https://openrouter.ai/api/v1"
    pub llm_base_url: String,
    
    /// Model identifier.
    /// Examples: "llama3.2:3b", "anthropic/claude-3-opus"
    pub llm_model: String,
}
```

### TypeScript
```typescript
/**
 * Configuration for the Terraphim service.
 *
 * This interface defines all runtime settings including knowledge graph
 * configuration, LLM providers, and haystack data sources.
 */
interface ServiceConfig {
  /**
   * The role name for this configuration.
   * Must match an entry in the roles map.
   */
  role: string;
  
  /**
   * Knowledge graph configuration.
   * If undefined, knowledge graph features are disabled.
   */
  kg?: KnowledgeGraphConfig;
  
  /**
   * Data sources for search and indexing.
   * At least one haystack is required for search to work.
   */
  haystacks: HaystackConfig[];
  
  /**
   * LLM provider: "ollama", "openrouter", or custom.
   */
  llmProvider: string;
  
  /**
   * Base URL for LLM API.
   * - For Ollama: "http://127.0.0.1:11434"
   * - For OpenRouter: "https://openrouter.ai/api/v1"
   */
  llmBaseUrl: string;
  
  /**
   * Model identifier.
   * Examples: "llama3.2:3b", "anthropic/claude-3-opus"
   */
  llmModel: string;
}
```

---

## TODO Comments
#todo #fixme #issues

Use TODO comments with issue tracker references.

**Good Examples:**
```rust
// TODO(#123): Implement pagination for large result sets
// FIXME(#456): Memory leak in session cleanup - need to investigate
// HACK(#789): Temporary workaround for upstream bug in dependency X
// NOTE: This approach was chosen to avoid breaking API compatibility
```

**Bad Examples:**
```rust
// TODO: fix this
// FIXME: broken
// NOTE: weird
```

**Categories:**
- `TODO`: Feature or improvement to implement
- `FIXME`: Known bug that needs fixing
- `HACK`: Workaround that should be replaced
- `NOTE`: Important context for maintainers
- `WARN`: Dangerous code requiring caution

**Format:** `CATEGORY(#issue): Description`

---

## Example Documentation
#examples #tutorials #guides

Provide runnable examples for complex functionality.

```rust
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use terraphim_service::TerraphimService;
///
/// # async fn example() -> Result<()> {
/// let service = TerraphimService::new(config);
/// let results = service.search("async").await?;
/// # Ok(())
/// # }
/// ```
///
/// ## With Custom Role
///
/// ```rust
/// # use terraphim_service::*;
/// # async fn example() -> Result<()> {
/// let service = TerraphimService::new(config);
/// let results = service.search_with_role("async", "Rust Engineer").await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Error Handling
///
/// ```rust
/// # use terraphim_service::*;
/// # async fn example() {
/// let service = TerraphimService::new(config);
///
/// match service.search("query").await {
///     Ok(results) => {
///         println!("Found {} results", results.len());
///     }
///     Err(ServiceError::RoleNotFound { role }) => {
///         eprintln!("Invalid role: {}", role);
///     }
///     Err(e) => {
///         eprintln!("Search failed: {}", e);
///     }
/// }
/// # }
/// ```
```

---

## README Files
#readme #documentation #getting-started

Every project and significant module should have a README.

**Structure:**
1. **Title and Description**: What is this?
2. **Installation**: How to get it?
3. **Quick Start**: Simplest working example
4. **Usage**: Common scenarios
5. **Configuration**: Options and settings
6. **API Reference**: Link to full docs
7. **Examples**: More complex use cases
8. **Contributing**: How to help
9. **License**: Legal information

**Example:**
```markdown
# Terraphim Service

Privacy-first AI assistant with knowledge graph search.

## Installation

\`\`\`bash
cargo add terraphim_service
\`\`\`

## Quick Start

\`\`\`rust
use terraphim_service::TerraphimService;

#[tokio::main]
async fn main() -> Result<()> {
    let service = TerraphimService::new(config);
    let results = service.search("async patterns").await?;
    
    for doc in results.documents {
        println!("{}", doc.description);
    }
    
    Ok(())
}
\`\`\`

## Features

- Semantic search with knowledge graphs
- Multiple LLM providers (Ollama, OpenRouter)
- Local-first privacy
- MCP integration for Claude Desktop

## Documentation

See [docs.rs/terraphim_service](https://docs.rs/terraphim_service)

## License

MIT
```

---

## Changelog
#changelog #versioning #releases

Maintain a CHANGELOG.md following [Keep a Changelog](https://keepachangelog.com/).

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Knowledge graph visualization in UI
- Support for multiple haystacks per role

### Fixed
- Memory leak in session cleanup (#456)

## [0.2.0] - 2025-01-15

### Added
- MCP server for Claude Desktop integration
- Autocomplete with semantic expansion
- New relevance function: TerraphimGraph

### Changed
- Improved search performance (50ms → 20ms average)
- Updated to Tokio 1.35

### Deprecated
- `search_simple` function (use `search` instead)

### Removed
- Legacy REST API v1 endpoints

### Fixed
- Race condition in concurrent indexing (#234)
- Panic on empty knowledge graph (#245)

### Security
- Updated dependencies to patch CVE-2024-XXXX

## [0.1.0] - 2024-12-01

Initial release
```

---

## Related Patterns

See also:
- [[naming-conventions]] - Choosing good names
- [[code-organization]] - Structuring code
- [[testing]] - Test documentation
- [[api-design]] - Public API documentation
