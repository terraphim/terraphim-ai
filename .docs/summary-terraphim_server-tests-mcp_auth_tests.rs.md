# Summary: terraphim_server/tests/mcp_auth_tests.rs

## Purpose
Comprehensive TDD test suite for MCP authentication middleware, validating Bearer token authentication, expiration handling, and security requirements.

## Test Organization

### Core Authentication Tests (Tests 1-7)
1. **test_unauthenticated_request_returns_401** - Verifies requests without Authorization header are rejected
2. **test_invalid_api_key_returns_401** - Confirms invalid/unknown API keys return 401
3. **test_valid_api_key_grants_access** - Validates properly authenticated requests succeed
4. **test_health_endpoint_is_public** - Ensures `/health` endpoint works without auth
5. **test_openapi_endpoint_is_public** - Ensures `/openapi.json` is publicly accessible
6. **test_all_mcp_endpoints_require_auth** - Systematically tests 6 MCP endpoints require authentication
7. **test_malformed_auth_header_returns_401** - Rejects headers without "Bearer " prefix

### Security Enhancement Tests (Tests 8-11)
8. **test_expired_api_key_returns_401** - API keys with past `expires_at` rejected
9. **test_disabled_api_key_returns_401** - API keys with `enabled: false` rejected
10. **test_case_insensitive_bearer_scheme** - Documents case-sensitive "Bearer" requirement
11. **test_bearer_token_with_whitespace** - Extra whitespace in token rejected

## Test Infrastructure

### Test Server Setup
- **create_test_server_with_auth()**: Returns configured router with authentication
- **create_test_server_with_auth_and_persistence()**: Returns (router, Arc<McpPersistenceImpl>)
  - Uses `Memory` backend via OpenDAL
  - Creates minimal `ConfigState` and `AppState`
  - Applies authentication middleware to protected MCP routes
  - Leaves `/health` and `/openapi.json` public

### Helper Functions
- **create_api_key_for_test()**: Creates and saves hashed API key for testing
  - Generates `McpApiKeyRecord` with enabled=true, no expiration
  - Uses `terraphim_server::mcp_auth::hash_api_key()`
  - Saves to persistence for middleware verification

## Route Configuration

### Protected Routes (with authentication)
```rust
Router::new()
    .route("/metamcp/namespaces", get(api_mcp::list_namespaces))
    .route("/metamcp/namespaces", post(api_mcp::create_namespace))
    .route("/metamcp/endpoints", get(api_mcp::list_endpoints))
    .route("/metamcp/endpoints", post(api_mcp::create_endpoint))
    .route("/metamcp/api_keys", post(api_mcp::create_api_key))
    .route("/metamcp/audits", get(api_mcp::list_audits))
    .route("/metamcp/endpoints/{endpoint_uuid}/tools", get(api_mcp_tools::list_tools_for_endpoint))
    .route("/metamcp/endpoints/{endpoint_uuid}/tools/{tool_name}", post(api_mcp_tools::execute_tool))
    .route_layer(middleware::from_fn_with_state(app_state, mcp_auth::validate_api_key))
```

### Public Routes (no authentication)
- `/health` - Server health check
- `/openapi.json` - API documentation

## Test Patterns

### Bearer Token Authentication
```rust
.add_header(
    axum::http::HeaderName::from_static("authorization"),
    axum::http::HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
)
```

### Expiration Testing
```rust
let record = McpApiKeyRecord {
    expires_at: Some(Utc::now() - Duration::days(1)), // Past date
    enabled: true,
    // ...
};
```

### Disabled Key Testing
```rust
let record = McpApiKeyRecord {
    expires_at: None,
    enabled: false, // Disabled
    // ...
};
```

## Critical Discoveries

### TDD Success: Prevented Production Bug
- Tests correctly validated authentication logic
- Revealed shared persistence requirement (middleware was creating new instances)
- **Critical Gap**: Tests passed but production routes lacked authentication middleware
- **Lesson**: Test middleware configuration, not just middleware behavior

### Security Validation
- Comprehensive edge case coverage (expired, disabled, malformed)
- Documents current behavior (case-sensitive Bearer, no whitespace trimming)
- Validates all MCP endpoints systematically

## Test Results
- **Total Tests**: 11
- **Pass Rate**: 11/11 (100%)
- **Execution Time**: ~0.01s
- **Coverage**: Authentication, expiration, enabled status, public endpoints, edge cases

## Integration with Production

### Production Router Differences
Initial implementation had authentication ONLY in tests:
```rust
// Test file: ✅ Has authentication middleware
protected_mcp_routes.route_layer(middleware::from_fn_with_state(..., validate_api_key))

// Production lib.rs: ❌ Missing authentication (fixed in commit 35c9cfc0)
.route("/metamcp/namespaces", post(api_mcp::create_namespace))
```

Fixed by applying same middleware pattern to production routes in `lib.rs`.

## File Location
`terraphim_server/tests/mcp_auth_tests.rs` (473 lines)

## Dependencies
- `axum`: HTTP server framework and test helpers
- `axum_test::TestServer`: Integration testing
- `serde_json`: Request/response JSON handling
- `tokio`: Async test runtime
- `chrono`: Timestamp manipulation for expiration tests
- `uuid`: UUID generation for test records

## Related Files
- `terraphim_server/src/mcp_auth.rs`: Middleware implementation being tested
- `terraphim_server/src/lib.rs`: Production router configuration
- `terraphim_persistence/src/mcp.rs`: Persistence layer for API key storage
- `/tmp/retrospective_improvements.md`: Security analysis and lessons learned
- `/tmp/tdd_learnings.md`: TDD process documentation

## Key Learnings
1. **TDD validates logic, not configuration**: Tests passed but production lacked middleware
2. **Integration tests need production parity**: Test router should match production exactly
3. **Helper abstraction**: Separate setup functions improve test maintainability
4. **Systematic validation**: Loop through endpoints to ensure consistent auth requirements
5. **Document behavior**: Tests clarify case-sensitivity and whitespace handling decisions
