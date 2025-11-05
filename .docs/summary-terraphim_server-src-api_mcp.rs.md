# Summary: terraphim_server/src/api_mcp.rs

## Purpose
Implements HTTP API handlers for MCP (Model Context Protocol) management, providing CRUD operations for namespaces, endpoints, API keys, tools, and audit trails.

## API Structure

### Response Types
All responses follow consistent pattern with status, data, and error fields:
- `McpNamespaceResponse` / `McpNamespaceListResponse`
- `McpEndpointResponse` / `McpEndpointListResponse`
- `McpApiKeyResponse` - Includes plaintext `key_value` on creation (one-time only)
- `McpAuditListResponse` - Paginated audit records
- `McpHealthResponse` - System health with namespace/endpoint counts

### Request Types
- `CreateNamespaceRequest` - name, description, user_id, config_json, enabled, visibility
- `CreateEndpointRequest` - name, namespace_uuid, auth_type, user_id, enabled
- `CreateApiKeyRequest` - endpoint_uuid, user_id, expires_at, enabled

## API Endpoints

### Namespace Management
1. **list_namespaces** - `GET /metamcp/namespaces`
   - Lists all namespaces, optionally filtered by user_id
   - Returns: `McpNamespaceListResponse`

2. **get_namespace** - `GET /metamcp/namespaces/{uuid}`
   - Retrieves single namespace by UUID
   - Returns: `McpNamespaceResponse` with 404 handling

3. **create_namespace** - `POST /metamcp/namespaces`
   - Creates new namespace with auto-generated UUID
   - Sets `created_at` timestamp automatically
   - Returns: Created namespace or error

4. **delete_namespace** - `DELETE /metamcp/namespaces/{uuid}`
   - Removes namespace permanently
   - Returns: `204 NO_CONTENT` or `500 INTERNAL_SERVER_ERROR`

### Endpoint Management
5. **list_endpoints** - `GET /metamcp/endpoints`
   - Lists all endpoints, optionally filtered by user_id
   - Returns: `McpEndpointListResponse`

6. **get_endpoint** - `GET /metamcp/endpoints/{uuid}`
   - Retrieves single endpoint by UUID
   - Returns: `McpEndpointResponse` with 404 handling

7. **create_endpoint** - `POST /metamcp/endpoints`
   - Creates new MCP endpoint with auto-generated UUID
   - Sets `created_at` timestamp automatically
   - Returns: Created endpoint or error

8. **delete_endpoint** - `DELETE /metamcp/endpoints/{uuid}`
   - Removes endpoint permanently
   - Returns: `204 NO_CONTENT` or `500 INTERNAL_SERVER_ERROR`

### API Key Management
9. **create_api_key** - `POST /metamcp/api_keys`
   - Generates API key with format: `tpai_{uuid_without_hyphens}`
   - Hashes key using SHA256 before storage (only hash persisted)
   - Returns: API key record AND plaintext key_value (one-time retrieval)
   - Security: Key value never stored, only hash

### Audit Trail
10. **list_audits** - `GET /metamcp/audits`
    - Lists up to 100 most recent audit records
    - Optionally filtered by user_id or endpoint_uuid
    - Sorted by created_at descending
    - Returns: Audit records with total count

### Health Check
11. **get_mcp_health** - `GET /metamcp/health`
    - Returns system version, timestamp, namespace/endpoint counts
    - Non-blocking health check for monitoring
    - Returns: `McpHealthResponse`

## Key Design Patterns

### Shared State Access
All handlers use `State(app_state): State<AppState>` extraction:
```rust
let persistence = app_state.mcp_persistence.clone();
```
- Uses `Arc<McpPersistenceImpl>` for thread-safe shared access
- Critical for authentication middleware compatibility

### UUID Generation
Auto-generates UUIDs for all resources:
```rust
uuid: uuid::Uuid::new_v4().to_string()
```

### Timestamp Management
Automatically sets `created_at` using `chrono::Utc::now()`:
```rust
created_at: chrono::Utc::now()
```

### Error Handling Strategy
- **Graceful Degradation**: Most endpoints return `Status::Error` in response body
- **HTTP Status Codes**: DELETE operations use proper status codes (204, 500)
- **Logging**: Errors logged before returning to client
- **User-Friendly Messages**: Generic error messages to avoid leaking internals

### API Key Security
Private `hash_api_key()` function in this module:
```rust
fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}
```
Note: Duplicates logic from `mcp_auth.rs` (could be refactored to shared module)

## Authentication Integration
All endpoints (except health) protected by `validate_api_key` middleware via:
```rust
.route_layer(middleware::from_fn_with_state(app_state, mcp_auth::validate_api_key))
```

## File Location
`terraphim_server/src/api_mcp.rs` (372 lines)

## Dependencies
- `axum`: HTTP framework, routing, JSON extraction/responses
- `chrono`: Timestamp management
- `serde`: Serialization/deserialization
- `sha2`: API key hashing
- `uuid`: UUID generation
- `terraphim_persistence::mcp`: Persistence trait and record types

## Related Files
- `terraphim_server/src/mcp_auth.rs`: Authentication middleware
- `terraphim_server/src/api_mcp_tools.rs`: Tool execution endpoints
- `terraphim_server/src/api_mcp_openapi.rs`: OpenAPI spec generation
- `terraphim_persistence/src/mcp.rs`: Persistence layer implementation
- `terraphim_server/src/lib.rs`: Router configuration

## Potential Improvements
1. **Deduplicate hash_api_key**: Share implementation with `mcp_auth.rs`
2. **Pagination**: Add pagination for list endpoints (currently unbounded)
3. **Batch Operations**: Support bulk create/delete for efficiency
4. **Rate Limiting**: Add per-user rate limits for creation endpoints
5. **Validation**: Add input validation (e.g., name length, config JSON schema)
6. **CORS**: Configure CORS headers for browser clients
