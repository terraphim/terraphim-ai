---
title: "CRITICAL: rmcp (MCP SDK) v0.9.1 → v1.2.0 upgrade"
labels: ["priority/P0", "type/breaking-change", "component/mcp", "vendor/rmcp"]
assignees: []
milestone: ""
---

## Summary

**Echo reports critical drift** in the Model Context Protocol (MCP) Rust SDK. Current version is 3 major versions behind with breaking API changes.

## Current State

- **Version:** 0.9.1
- **Location:** `crates/terraphim_mcp_server/Cargo.toml`
- **Upstream:** v1.2.0 (latest stable)
- **Drift:** 3 major versions behind

## Breaking Changes

### v1.0.0-alpha → v1.0.0

#### 1. Auth Token Exchange Breaking Change
- **Change:** Token exchange now returns extra fields
- **PR:** [#700](https://github.com/modelcontextprotocol/rust-sdk/pull/700)
- **Impact:** OAuth implementations in MCP server
- **Migration:** Update token exchange handling

#### 2. Non-Exhaustive Types
- **Change:** `#[non_exhaustive]` added to model types
- **PR:** [#715](https://github.com/modelcontextprotocol/rust-sdk/pull/715)
- **Impact:** Match statements and exhaustive pattern matching
- **Migration:** Add wildcard patterns or use constructors

#### 3. Streamable HTTP Error Handling
- **Change:** Stale session 401 mapped to status-aware error
- **PR:** [#709](https://github.com/modelcontextprotocol/rust-sdk/pull/709)
- **Impact:** Error handling logic
- **Migration:** Update error matching

### v1.1.0

#### 4. OAuth 2.0 Client Credentials
- **Change:** New OAuth 2.0 Client Credentials flow support
- **PR:** [#707](https://github.com/modelcontextprotocol/rust-sdk/pull/707)
- **Impact:** New authentication option available
- **Note:** Not breaking, but adds capability

### v1.1.1

#### 5. Pre-Initialization Messages
- **Change:** Accept logging/setLevel and ping before initialized notification
- **PR:** [#730](https://github.com/modelcontextprotocol/rust-sdk/pull/730)
- **Impact:** Protocol initialization handling
- **Migration:** Update initialization state machine

### v1.2.0

#### 6. Ping Request Handling
- **Change:** Handle ping requests before initialize handshake
- **PR:** [#745](https://github.com/modelcontextprotocol/rust-sdk/pull/745)
- **Impact:** Connection stability
- **Migration:** Update connection handling

#### 7. Optional Notification Params
- **Change:** Allow deserializing notifications without params field
- **PR:** [#729](https://github.com/modelcontextprotocol/rust-sdk/pull/729)
- **Impact:** Notification handling
- **Migration:** Update notification deserialization

#### 8. JSON Web Token Upgrade
- **Change:** jsonwebtoken 9 → 10
- **PR:** [#737](https://github.com/modelcontextprotocol/rust-sdk/pull/737)
- **Impact:** JWT handling
- **Migration:** Verify JWT operations still work

#### 9. Model Constructors
- **Change:** Missing constructors added for non-exhaustive types
- **PR:** [#739](https://github.com/modelcontextprotocol/rust-sdk/pull/739)
- **Impact:** Type construction
- **Migration:** Can now use new constructors

## Affected Crates

- [ ] `terraphim_mcp_server` - Direct rmcp dependency

## Reproduction

```bash
# Check current version
cargo tree -p rmcp | head -5

# Check for outdated dependencies
cargo outdated -p rmcp
```

## Proposed Migration Plan

1. **Phase 1: Version Update**
   - [ ] Create `feat/rmcp-v1.2-migration` branch
   - [ ] Update rmcp from 0.9.1 to 1.2.0
   - [ ] Update rmcp-macros from 0.9.1 to 1.2.0

2. **Phase 2: API Migration**
   - [ ] Fix match statements on MCP types (add wildcard arms)
   - [ ] Update error handling for status-aware errors
   - [ ] Update initialization handling
   - [ ] Update notification handling

3. **Phase 3: OAuth Evaluation**
   - [ ] Evaluate OAuth 2.0 Client Credentials flow
   - [ ] Implement if needed for MCP server security

4. **Phase 4: Testing**
   - [ ] Run MCP integration tests
   - [ ] Test tool registration
   - [ ] Test resource access
   - [ ] Test SSE transport
   - [ ] Test stdio transport

## Code Migration Examples

### Before (v0.9.1):
```rust
match notification {
    ClientNotification::ToolCall(params) => { ... },
    ClientNotification::ResourceAccess(params) => { ... },
    // Exhaustive match
}
```

### After (v1.2.0):
```rust
match notification {
    ClientNotification::ToolCall(params) => { ... },
    ClientNotification::ResourceAccess(params) => { ... },
    _ => {
        // Handle new variants or ignore
        tracing::debug!("Unhandled notification");
    }
}
```

## References

- [rust-sdk releases](https://github.com/modelcontextprotocol/rust-sdk/releases)
- [MCP Specification](https://modelcontextprotocol.io)

## Dependencies

- Blocked by: #1 (rust-genai upgrade - coordinated reqwest version)
- Related to: Firecracker upgrade (independent)

## Verification

```bash
# After upgrade
cargo test -p terraphim_mcp_server
cargo test -p terraphim_mcp_server --features client
cargo test -p terraphim_mcp_server --features server
```

---

**Echo's Assessment:** MCP protocol layer drift detected. Non-exhaustive types will cause compilation failures. Synchronize immediately to maintain twin fidelity.
