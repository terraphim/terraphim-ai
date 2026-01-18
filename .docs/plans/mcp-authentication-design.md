# Design & Implementation Plan: MCP Authentication and Security Enhancements

**Status:** Approved for Implementation
**Priority:** Medium
**Origin:** Closed PR #287 (2 months old, conflicts with current code)
**Date:** 2025-12-31

---

## 1. Summary of Target Behavior

After implementation, the MCP server will:

1. **Authenticate all HTTP/SSE requests** using Bearer tokens with SHA256 validation
2. **Enforce three-layer security**: token exists + token enabled + token not expired
3. **Rate limit requests** per token using sliding window algorithm
4. **Log security events** with comprehensive audit trail for attack detection
5. **Apply authentication to production routes** (fixing the critical vulnerability from PR #287)

The Stdio transport remains unauthenticated (trusted local process).

---

## 2. Key Invariants and Acceptance Criteria

### Security Invariants

| Invariant | Guarantee |
|-----------|-----------|
| I1 | No unauthenticated request can invoke tools via HTTP/SSE |
| I2 | Expired tokens are rejected with 401 Unauthorized |
| I3 | Rate-limited tokens receive 429 Too Many Requests |
| I4 | All authentication failures are logged with client IP |
| I5 | Stdio transport bypasses auth (trusted local process) |

### Acceptance Criteria

| ID | Criterion | Testable |
|----|-----------|----------|
| AC1 | Request without Authorization header returns 401 | Yes |
| AC2 | Request with invalid token returns 401 | Yes |
| AC3 | Request with expired token returns 401 | Yes |
| AC4 | Request with disabled token returns 403 | Yes |
| AC5 | Request exceeding rate limit returns 429 | Yes |
| AC6 | Valid token allows tool invocation | Yes |
| AC7 | Security events logged with timestamp, IP, token_id | Yes |
| AC8 | Stdio transport works without token | Yes |

---

## 3. High-Level Design and Boundaries

### Component Architecture

```
                    +------------------+
                    |   HTTP Request   |
                    +--------+---------+
                             |
                    +--------v---------+
                    | Rate Limit Layer |  <-- Sliding window per token
                    +--------+---------+
                             |
                    +--------v---------+
                    |   Auth Middleware |  <-- Bearer token validation
                    +--------+---------+
                             |
                    +--------v---------+
                    |  Security Logger |  <-- Audit trail
                    +--------+---------+
                             |
                    +--------v---------+
                    |    McpService    |  <-- Existing tool handlers
                    +------------------+
```

### New Components

| Component | Responsibility | Location |
|-----------|----------------|----------|
| `AuthMiddleware` | Extract & validate Bearer tokens | `src/auth/middleware.rs` |
| `TokenValidator` | SHA256 hash comparison, expiry check | `src/auth/validator.rs` |
| `RateLimiter` | Sliding window rate limiting | `src/auth/rate_limit.rs` |
| `SecurityLogger` | Structured audit logging | `src/auth/logger.rs` |
| `AuthConfig` | Token storage, rate limit settings | `src/auth/config.rs` |

### Existing Components (Modified)

| Component | Change |
|-----------|--------|
| `src/main.rs` | Add auth middleware to Axum router (lines 110-138) |
| `Cargo.toml` | Add `tower-http`, `sha2`, `dashmap` dependencies |

### Boundaries

- **Inside scope:** HTTP/SSE transport authentication
- **Outside scope:** Stdio transport (remains unauthenticated)
- **Outside scope:** Tool-level ACLs (future Phase 3)
- **Outside scope:** JWT/OAuth (future enhancement)

---

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_mcp_server/Cargo.toml` | Modify | MCP deps only | Add auth deps | tower-http, sha2, dashmap |
| `crates/terraphim_mcp_server/src/auth/mod.rs` | Create | - | Auth module root | - |
| `crates/terraphim_mcp_server/src/auth/middleware.rs` | Create | - | Axum auth layer | tower-http |
| `crates/terraphim_mcp_server/src/auth/validator.rs` | Create | - | Token validation | sha2 |
| `crates/terraphim_mcp_server/src/auth/rate_limit.rs` | Create | - | Rate limiting | dashmap, tokio |
| `crates/terraphim_mcp_server/src/auth/logger.rs` | Create | - | Audit logging | tracing |
| `crates/terraphim_mcp_server/src/auth/config.rs` | Create | - | Auth configuration | serde |
| `crates/terraphim_mcp_server/src/lib.rs` | Modify | No auth | Export auth module | auth module |
| `crates/terraphim_mcp_server/src/main.rs` | Modify | No middleware | Auth middleware on SSE routes | auth module |
| `crates/terraphim_mcp_server/tests/test_auth.rs` | Create | - | Auth integration tests | - |

---

## 5. Step-by-Step Implementation Sequence

### Phase 1: Foundation (Steps 1-4)

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| 1 | Add dependencies to Cargo.toml | Yes | tower-http, sha2, dashmap |
| 2 | Create `src/auth/mod.rs` with module structure | Yes | Empty modules, compiles |
| 3 | Implement `AuthConfig` with token storage | Yes | Feature-gated `auth` |
| 4 | Implement `TokenValidator` with SHA256 | Yes | Unit tests pass |

### Phase 2: Middleware (Steps 5-7)

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| 5 | Implement `AuthMiddleware` using tower | Yes | Returns 401 without token |
| 6 | Integrate middleware into Axum router | Yes | **Feature flag: `--features auth`** |
| 7 | Add `--token` CLI argument for single-token mode | Yes | Simple bootstrap |

### Phase 3: Rate Limiting (Steps 8-9)

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| 8 | Implement `RateLimiter` with sliding window | Yes | DashMap for concurrent access |
| 9 | Integrate rate limiter into middleware chain | Yes | Returns 429 when exceeded |

### Phase 4: Logging & Hardening (Steps 10-12)

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| 10 | Implement `SecurityLogger` with tracing | Yes | Structured JSON logs |
| 11 | Add comprehensive integration tests | Yes | 40+ tests for auth flows |
| 12 | Documentation and CLI help updates | Yes | README, --help |

---

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1: Missing header -> 401 | Unit | `tests/test_auth.rs::test_missing_auth_header` |
| AC2: Invalid token -> 401 | Unit | `tests/test_auth.rs::test_invalid_token` |
| AC3: Expired token -> 401 | Unit | `tests/test_auth.rs::test_expired_token` |
| AC4: Disabled token -> 403 | Unit | `tests/test_auth.rs::test_disabled_token` |
| AC5: Rate limit -> 429 | Integration | `tests/test_auth.rs::test_rate_limiting` |
| AC6: Valid token works | Integration | `tests/test_auth.rs::test_valid_auth_flow` |
| AC7: Audit logging | Integration | `tests/test_auth.rs::test_security_logging` |
| AC8: Stdio bypasses auth | Integration | `tests/test_auth.rs::test_stdio_no_auth` |

### Test Coverage Target

- Unit tests: 100% for validator, rate limiter
- Integration tests: All acceptance criteria
- Property tests: Token validation edge cases

---

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Breaking existing Stdio users | Feature flag `auth`, Stdio unaffected | Low |
| Performance impact of auth | DashMap for O(1) token lookup | Low |
| Token storage security | SHA256 hashing, never store plaintext | Medium - need secure config |
| Rate limit memory growth | TTL-based cleanup, max tokens config | Low |
| Middleware ordering bugs | Explicit layer ordering in Axum | Low |
| 2-month old PR conflicts | Fresh implementation, no merge | None |

---

## 8. Configuration Schema

```toml
# Example: mcp_auth.toml
[auth]
enabled = true
token_hash_algorithm = "sha256"

[[auth.tokens]]
id = "dev-token-1"
hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
enabled = true
expires_at = "2025-12-31T23:59:59Z"
rate_limit = 100  # requests per minute

[auth.rate_limiting]
window_seconds = 60
default_limit = 100
burst_limit = 10

[auth.logging]
log_successful_auth = true
log_failed_auth = true
include_client_ip = true
```

---

## 9. API Changes

### New CLI Arguments

```bash
# Single token mode (development)
terraphim-mcp-server --token "my-secret-token"

# Config file mode (production)
terraphim-mcp-server --auth-config /path/to/mcp_auth.toml

# Disable auth (local dev, explicitly opt-out)
terraphim-mcp-server --no-auth
```

### New Environment Variables

```bash
MCP_AUTH_TOKEN=my-secret-token
MCP_AUTH_CONFIG=/path/to/mcp_auth.toml
MCP_AUTH_ENABLED=true
```

---

## 10. Dependencies to Add

```toml
# crates/terraphim_mcp_server/Cargo.toml

[dependencies]
tower-http = { version = "0.6", features = ["auth", "trace"] }
sha2 = "0.10"
dashmap = "6.0"
base64 = "0.22"  # already present

[dev-dependencies]
axum-test = "16"  # for integration testing
```

---

## 11. Open Questions / Decisions for Human Review

| Question | Options | Recommendation |
|----------|---------|----------------|
| Token storage format? | TOML file vs SQLite vs environment | TOML for simplicity, SQLite for scale |
| Default auth state? | Enabled by default vs opt-in | Opt-in with `--features auth` initially |
| Rate limit scope? | Per-token vs per-IP vs global | Per-token (most flexible) |
| JWT support? | Now vs later | Later (Phase 2 enhancement) |
| 1Password integration? | For token management | Yes, use `op read` pattern from CI |

---

## 12. Implementation Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Phase 1: Foundation | 2 days | Auth module structure, token validator |
| Phase 2: Middleware | 2 days | Working auth on SSE routes |
| Phase 3: Rate Limiting | 1 day | Sliding window implementation |
| Phase 4: Hardening | 2 days | Logging, tests, documentation |
| **Total** | **7 days** | Production-ready MCP auth |

---

## 13. Success Metrics

| Metric | Target |
|--------|--------|
| Test coverage | > 90% for auth module |
| Auth latency overhead | < 1ms per request |
| Memory per token | < 1KB |
| Security audit | Pass OWASP API Security Top 10 |

---

**Plan Status:** Ready for Implementation

**Next Step:** Create GitHub issue with this plan, then proceed to Phase 3 (Disciplined Implementation)
