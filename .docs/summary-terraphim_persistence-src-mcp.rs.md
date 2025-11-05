# Summary: crates/terraphim_persistence/src/mcp.rs

## Purpose
Implements persistence layer for MCP (Model Context Protocol) data using OpenDAL for multi-backend storage (Memory, File, S3, etc.). Provides async CRUD operations for namespaces, endpoints, API keys, tools, tool caching, and audit trails.

## Core Data Models

### McpNamespaceRecord
- **uuid**: Unique identifier
- **name**: Namespace display name
- **description**: Optional description
- **user_id**: Owner identifier
- **config_json**: MCP configuration as JSON string
- **created_at**: Creation timestamp
- **enabled**: Active/inactive status
- **visibility**: Public or Private (default: Private)

### McpEndpointRecord
- **uuid**: Unique identifier
- **name**: Endpoint display name
- **namespace_uuid**: Parent namespace reference
- **auth_type**: Authentication method (e.g., "bearer")
- **user_id**: Owner identifier
- **created_at**: Creation timestamp
- **enabled**: Active/inactive status

### McpApiKeyRecord
- **uuid**: Unique identifier
- **key_hash**: SHA256 hash of actual API key (never stores plaintext)
- **endpoint_uuid**: Associated endpoint
- **user_id**: Owner identifier
- **created_at**: Creation timestamp
- **expires_at**: Optional expiration timestamp
- **enabled**: Active/inactive status (can be disabled without deletion)

### McpToolRecord
- **uuid**: Unique identifier
- **namespace_uuid**: Parent namespace
- **server_name**: MCP server name (e.g., "filesystem")
- **tool_name**: Namespaced tool name (e.g., "filesystem__read_file")
- **original_name**: Original tool name from server
- **status**: Active or Inactive
- **override_name**: Optional custom display name
- **override_description**: Optional custom description
- **created_at / updated_at**: Timestamps

### ToolDiscoveryCache
- **namespace_uuid**: Namespace identifier
- **tools_json**: Cached tool list as JSON string
- **cached_at**: Cache creation time
- **expires_at**: Cache expiration time
- Purpose: Reduce repeated tool discovery API calls

### McpAuditRecord
- **uuid**: Unique identifier
- **user_id**: User who executed the action
- **endpoint_uuid**: Endpoint used
- **namespace_uuid**: Namespace context
- **tool_name**: Tool that was called
- **arguments**: Optional serialized arguments JSON
- **response**: Optional serialized response JSON
- **is_error**: Boolean flag for error responses
- **latency_ms**: Execution time in milliseconds
- **created_at**: Audit timestamp

## McpPersistence Trait

### Namespace Operations
- `save_namespace(&self, record) -> Result<()>`
- `get_namespace(&self, uuid) -> Result<Option<McpNamespaceRecord>>`
- `list_namespaces(&self, user_id) -> Result<Vec<McpNamespaceRecord>>`
- `list_namespaces_with_visibility(&self, user_id, include_public) -> Result<Vec<McpNamespaceRecord>>`
- `delete_namespace(&self, uuid) -> Result<()>`

### Endpoint Operations
- `save_endpoint(&self, record) -> Result<()>`
- `get_endpoint(&self, uuid) -> Result<Option<McpEndpointRecord>>`
- `list_endpoints(&self, user_id) -> Result<Vec<McpEndpointRecord>>`
- `delete_endpoint(&self, uuid) -> Result<()>`

### API Key Operations
- `save_api_key(&self, record) -> Result<()>`
- `get_api_key(&self, uuid) -> Result<Option<McpApiKeyRecord>>`
- **`verify_api_key(&self, key_hash) -> Result<Option<McpApiKeyRecord>>`** - Critical for authentication
- `list_api_keys(&self, user_id) -> Result<Vec<McpApiKeyRecord>>`
- `delete_api_key(&self, uuid) -> Result<()>`

### Tool Operations
- `save_tool(&self, record) -> Result<()>`
- `get_tool(&self, uuid) -> Result<Option<McpToolRecord>>`
- `list_tools(&self, namespace_uuid) -> Result<Vec<McpToolRecord>>`
- `update_tool_status(&self, uuid, status) -> Result<()>`
- `delete_tool(&self, uuid) -> Result<()>`

### Tool Cache Operations
- `save_tool_cache(&self, cache) -> Result<()>`
- `get_tool_cache(&self, namespace_uuid) -> Result<Option<ToolDiscoveryCache>>`
- `delete_tool_cache(&self, namespace_uuid) -> Result<()>`

### Audit Operations
- `save_audit(&self, record) -> Result<()>`
- `get_audit(&self, uuid) -> Result<Option<McpAuditRecord>>`
- `list_audits(&self, user_id, endpoint_uuid, limit) -> Result<Vec<McpAuditRecord>>`
- `delete_audit(&self, uuid) -> Result<()>`

## McpPersistenceImpl

### Storage Architecture
- **Backend**: OpenDAL Operator wrapped in `Arc<RwLock<Operator>>`
- **Path Structure**: Hierarchical JSON file organization
  - `mcp/namespaces/{uuid}.json`
  - `mcp/endpoints/{uuid}.json`
  - `mcp/api_keys/{uuid}.json`
  - `mcp/tools/{uuid}.json`
  - `mcp/tool_cache/{namespace_uuid}.json`
  - `mcp/audit/{uuid}.json`

### Key Implementation Details

#### verify_api_key (Authentication Critical)
```rust
async fn verify_api_key(&self, key_hash: &str) -> Result<Option<McpApiKeyRecord>> {
    // Lists all API keys and searches for matching hash
    // Validates enabled=true
    // Checks expiration if expires_at is set
    // Returns None if key not found, disabled, or expired
}
```
- **Performance Note**: Linear search through all keys (acceptable for small datasets)
- **Security**: Returns None for expired/disabled keys (no error indication)

#### list_audits with Sorting
```rust
async fn list_audits(...) -> Result<Vec<McpAuditRecord>> {
    // Collects matching audits
    // Sorts by created_at descending (newest first)
    // Respects limit parameter
}
```

#### Namespace Visibility Filtering
```rust
async fn list_namespaces_with_visibility(&self, user_id, include_public) -> Result<...> {
    // If include_public=true: returns user's namespaces + all public namespaces
    // If include_public=false: returns only user's namespaces
    // Enables multi-tenant public/private namespace sharing
}
```

### Thread Safety
- `Arc<RwLock<Operator>>` allows concurrent reads, exclusive writes
- All trait methods are `async` for non-blocking I/O
- Trait requires `Send + Sync` for multi-threaded usage

## Test Coverage (7 comprehensive tests)

1. **test_namespace_persistence** - CRUD operations for namespaces
2. **test_api_key_verification** - Authentication verification logic
3. **test_tool_management** - Tool CRUD and status updates
4. **test_tool_cache** - Cache save/retrieve/delete
5. **test_audit_trail** - Audit record management and filtering
6. **test_namespace_visibility** - Public/private namespace filtering

All tests use Memory backend for fast, isolated testing.

## Critical Design Decisions

### Why Arc<RwLock<Operator>>?
- **Arc**: Enables cloning for shared ownership across threads
- **RwLock**: Allows multiple concurrent readers, single writer
- **Operator**: OpenDAL's backend abstraction

### Why JSON Files?
- **Flexibility**: Works with any OpenDAL backend (Memory, FS, S3, Azure, etc.)
- **Human-Readable**: Easy debugging and inspection
- **Schema Evolution**: JSON allows adding fields without migration
- **Limitation**: No relational queries, linear search for some operations

### Why verify_api_key Scans All Keys?
- **Simplicity**: No secondary index needed
- **Performance Trade-off**: Acceptable for <1000 keys
- **Future Improvement**: Could add in-memory cache or use database backend

## Known Limitations

1. **No Pagination**: List operations return all matching records
2. **Linear Search**: API key verification O(n) complexity
3. **No Transactions**: Multi-step operations not atomic
4. **No Indexing**: Cannot efficiently query by fields other than UUID
5. **SQLite Incompatibility**: Hierarchical paths don't map to SQLite blob storage

## Backend Compatibility

### Tested Backends
- **Memory**: In-memory HashMap (tests, development)
- **File System**: Local directory storage (production)

### Theoretically Supported (via OpenDAL)
- AWS S3
- Azure Blob Storage
- Google Cloud Storage
- HTTP/WebDAV

### Known Issues
- **SQLite**: Blob storage model incompatible with hierarchical paths
- **RocksDB**: Previously caused locking issues (deprecated)

## File Location
`crates/terraphim_persistence/src/mcp.rs` (744 lines)

## Dependencies
- `async_trait`: Async trait definitions
- `chrono`: Timestamp handling
- `futures`: Stream utilities
- `serde`: JSON serialization
- `tokio::sync::RwLock`: Async read-write lock
- `opendal`: Multi-backend storage abstraction

## Related Files
- `terraphim_server/src/api_mcp.rs`: HTTP API handlers using this persistence
- `terraphim_server/src/mcp_auth.rs`: Authentication middleware calling verify_api_key
- `terraphim_server/src/lib.rs`: AppState with shared McpPersistenceImpl instance
- `terraphim_server/tests/mcp_auth_tests.rs`: Integration tests using this persistence

## Security Considerations
- **API Key Storage**: Only hashes stored, never plaintext
- **Expiration Enforcement**: Expired keys automatically rejected by verify_api_key
- **Enabled Status**: Allows disabling keys without deletion
- **Audit Trail**: Comprehensive logging for security monitoring
- **User Isolation**: Optional user_id filtering for multi-tenancy

## Performance Characteristics
- **Read Operations**: O(1) for get by UUID, O(n) for list/search
- **Write Operations**: O(1) for save/delete
- **verify_api_key**: O(n) linear search (could be optimized with cache)
- **list_audits**: O(n log n) due to sorting

## Future Enhancements
1. Add in-memory cache for API key verification
2. Implement pagination for list operations
3. Add secondary indexes for common queries
4. Support for database backends (PostgreSQL, MySQL)
5. Batch operations for bulk imports
6. Metrics and monitoring integration
