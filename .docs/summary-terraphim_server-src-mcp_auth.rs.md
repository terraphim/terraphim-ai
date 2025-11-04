# Summary: terraphim_server/src/mcp_auth.rs

## Purpose
Implements Bearer token authentication middleware for MCP (Model Context Protocol) API endpoints using SHA256-hashed API keys with expiration and enabled status validation.

## Key Components

### `validate_api_key` Middleware Function
- **Signature**: `async fn validate_api_key(State(state): State<AppState>, headers: HeaderMap, request: Request, next: Next) -> Result<Response, StatusCode>`
- **Authentication Flow**:
  1. Extracts Authorization header from request
  2. Validates "Bearer " prefix (case-sensitive)
  3. Extracts and hashes API key using SHA256
  4. Verifies key hash against shared `AppState.mcp_persistence`
  5. Checks `enabled` status - rejects disabled keys
  6. Checks `expires_at` timestamp - rejects expired keys
  7. Logs security events for all failures
  8. Allows request to proceed if all checks pass

### `hash_api_key` Function
- **Signature**: `pub fn hash_api_key(key: &str) -> String`
- Converts raw API key to SHA256 hex string
- Made public for test helper usage
- Uses `sha2` crate with lowercase hex formatting

## Security Features

### Multi-Layer Validation
1. **Authentication**: Key must exist in persistence
2. **Enabled Status**: `record.enabled` must be `true`
3. **Expiration**: `record.expires_at` must be None or future timestamp
4. **Logging**: All failures logged with truncated key hash (first 8 chars)

### Critical Design Decisions
- **Shared Persistence**: Uses `Arc<McpPersistenceImpl>` from AppState (not new instances)
- **Case-Sensitive Bearer**: Only accepts "Bearer " prefix (not "bearer")
- **Structured Logging**: Uses `log::warn!` for security events, `log::error!` for system failures
- **Status Codes**: Returns `401 UNAUTHORIZED` for auth failures, `500 INTERNAL_SERVER_ERROR` for DB errors

## Integration Points
- **AppState**: Requires `mcp_persistence: Arc<McpPersistenceImpl>` field
- **Router Configuration**: Applied via `route_layer(middleware::from_fn_with_state(app_state, validate_api_key))`
- **Public Endpoints**: Health and OpenAPI routes bypass this middleware

## Test Coverage
Validated by 11 comprehensive tests in `tests/mcp_auth_tests.rs`:
- Unauthenticated request rejection
- Invalid API key rejection
- Valid API key acceptance
- Expired key rejection
- Disabled key rejection
- Malformed header rejection
- Case sensitivity behavior
- Whitespace handling

## Key Learnings from Implementation
1. **Shared State Critical**: Initial implementation created new persistence instances, causing data loss
2. **Production vs Test Gap**: Middleware was initially only applied in tests, not production routes
3. **Security Layering**: Multiple validation checks (exists + enabled + not expired) required
4. **Logging Essential**: Security events must be logged for attack detection

## File Location
`terraphim_server/src/mcp_auth.rs` (70 lines)

## Dependencies
- `axum`: Request/Response handling, middleware framework
- `chrono`: Timestamp comparison for expiration
- `sha2`: SHA256 hashing for API keys
- `terraphim_persistence::mcp`: McpPersistence trait and types

## Related Files
- `terraphim_server/src/lib.rs`: Router configuration with middleware application
- `terraphim_server/tests/mcp_auth_tests.rs`: Comprehensive test suite
- `terraphim_persistence/src/mcp.rs`: Persistence layer with `verify_api_key` method
