# Vendor API Drift Report - Echo/Twin Maintainer

**Generated:** 2026-03-23
**Status:** Critical drift detected
**Reporter:** Echo (Twin Maintainer)

## Executive Summary

Mirror verification reveals significant drift between current dependencies and upstream vendor APIs. Four critical integration points require immediate remediation to maintain twin fidelity.

---

## 1. CRITICAL: rust-genai v0.4.4 → v0.6.0 (HIGH PRIORITY)

### Current State
- **Version:** v0.4.4-WIP (terraphim fork, branch `merge-upstream-20251103`)
- **Commit:** 0f8839ad
- **Location:** `Cargo.toml` [patch.crates-io]

### Upstream Changes (Breaking)

#### v0.5.0 Changes:
1. **Dependency Conflict:** `reqwest` upgraded from 0.12 to 0.13
   - Workspace currently uses 0.12
   - **Impact:** Version mismatch will cause compilation failures

2. **API Breaking:** `ChatResponse.content` type changed
   - From: `Vec<MessageContent>`
   - To: `MessageContent`
   - **Impact:** All code using `.content` field needs migration

3. **API Breaking:** `StreamEnd.content` type changed
   - To: `Option<MessageContent>`
   - **Impact:** Streaming response handling

4. **API Breaking:** `ChatRequest::append/with_...(vec)` functions
   - Now take iterators instead of Vec
   - **Impact:** Request builder code

5. **API Breaking:** `ContentPart` restructuring
   - `ContentPart::Binary(Binary)` now required
   - Binary constructors changed parameter order
   - **Impact:** Multimodal content handling

6. **Namespace Strategy:** ZAI namespace changes
   - Default models now use `zai::` prefix
   - **Impact:** Model name resolution

#### v0.6.0-beta Changes:
1. **API Breaking:** `ContentPart::CustomPart.model_iden`
   - Now `Option` type
   - **Impact:** Custom content handling

2. **API Breaking:** `all_model_names()`
   - Now requires `AuthResolver` support
   - **Impact:** Model listing functionality

3. **Provider Breaking:** Groq namespace requirement
   - Must use `groq::_model_name` format
   - **Impact:** Groq provider configuration

4. **New OpenAI Routing:** GPT-5 models
   - Routed through OpenAI Responses API
   - **Impact:** Model routing logic

### Affected Crates
- `terraphim_multi_agent` - Direct genai dependency
- `terraphim_service` - LLM service layer
- `terraphim_config` - Model configuration

### Recommended Action
1. Create dedicated migration branch
2. Upgrade workspace reqwest to 0.13
3. Update genai fork to v0.5.3 baseline
4. Migrate `ChatResponse.content` access patterns
5. Update streaming handlers for `StreamEnd` changes
6. Add namespace handling for Groq/ZAI models

### References
- [rust-genai CHANGELOG](https://github.com/jeremychone/rust-genai/blob/main/CHANGELOG.md)
- [Migration Guide v0.3→v0.4](https://github.com/jeremychone/rust-genai/blob/main/doc/migration/migration-v_0_3_to_0_4.md)

---

## 2. CRITICAL: rmcp (MCP SDK) v0.9.1 → v1.2.0 (HIGH PRIORITY)

### Current State
- **Version:** 0.9.1
- **Location:** `terraphim_mcp_server/Cargo.toml`

### Upstream Changes (Breaking)

#### v1.0.0-alpha → v1.0.0:
1. **Breaking:** Auth token exchange returns extra fields
   - **Impact:** OAuth implementations

2. **Breaking:** `#[non_exhaustive]` added to model types
   - **Impact:** Match statements and exhaustive patterns

3. **API Change:** Streamable HTTP error handling
   - Stale session 401 now mapped to status-aware error
   - **Impact:** Error handling logic

#### v1.1.0:
1. **New Feature:** OAuth 2.0 Client Credentials flow
   - **Impact:** New authentication options available

#### v1.1.1:
1. **Fix:** Accept logging/setLevel and ping before initialized
   - **Impact:** Protocol initialization

#### v1.2.0:
1. **Fix:** Handle ping requests before initialize handshake
   - **Impact:** Connection stability

2. **Fix:** Allow deserializing notifications without params field
   - **Impact:** Notification handling

3. **Deps:** jsonwebtoken 9 → 10
   - **Impact:** JWT handling

4. **Fix:** Non-exhaustive model constructors
   - **Impact:** Type construction

### Affected Crates
- `terraphim_mcp_server` - Direct rmcp dependency

### Recommended Action
1. Upgrade rmcp to v1.2.0
2. Review all match statements on MCP types
3. Update error handling for new status-aware errors
4. Test OAuth flows if implemented

### References
- [rust-sdk releases](https://github.com/modelcontextprotocol/rust-sdk/releases)

---

## 3. MODERATE: Firecracker API v1.11.0 (MEDIUM PRIORITY)

### Current State
- Integration via `terraphim_firecracker` crate
- Local implementation of Firecracker API client

### Upstream Changes

#### v1.11.0 (2026-03-18):
1. **Breaking:** Snapshot format v5.0.0
   - Removed fields: `max_connections`, `max_pending_resets`
   - **Impact:** Existing snapshots incompatible - must regenerate

2. **Change:** seccompiler implementation
   - Migrated to `libseccomp`
   - **Impact:** BPF code generation

3. **Added:** AMD Genoa support
4. **Fixed:** ARM physical counter behavior
5. **Fixed:** PATCH /machine-config field requirements

### Affected Crates
- `terraphim_firecracker` - Firecracker API client
- `terraphim_github_runner` - VM management

### Recommended Action
1. Review snapshot usage in CI/CD
2. Update API client for snapshot format v5.0
3. Plan snapshot regeneration
4. Test VM creation with new seccompiler

### References
- [Firecracker v1.11.0 release](https://github.com/firecracker-microvm/firecracker/releases/tag/v1.11.0)

---

## 4. LOW: Additional Dependencies (MONITORING)

### 1Password CLI
- **Status:** External CLI dependency
- **Risk:** Low - stable API
- **Action:** Monitor for breaking changes

### Atomic Data Server
- **Status:** API client in `terraphim_atomic_client`
- **Risk:** Low - local server
- **Action:** Monitor Atomic Data specification

### JMAP (haystack_jmap)
- **Status:** Email protocol client
- **Risk:** Low - standard protocol
- **Action:** Monitor RFC updates

### Atlassian (haystack_atlassian)
- **Status:** Currently excluded from workspace
- **Risk:** N/A
- **Action:** Review before re-enabling

---

## Remediation Priority Matrix

| Vendor | Severity | Effort | Priority | Issue # |
|--------|----------|--------|----------|---------|
| rust-genai | High | High | P0 | TBD |
| rmcp | High | Medium | P0 | TBD |
| Firecracker | Medium | Medium | P1 | TBD |
| Others | Low | Low | P2 | TBD |

---

## Dependencies Between Issues

1. **rust-genai** blocks **rmcp** upgrade
   - Both require coordinated reqwest version
   
2. **Firecracker** is independent
   - Can be upgraded separately

---

## Verification Checklist

- [ ] rust-genai fork updated to v0.5.3
- [ ] Workspace reqwest upgraded to 0.13
- [ ] ChatResponse.content migration complete
- [ ] Streaming handlers updated
- [ ] rmcp upgraded to v1.2.0
- [ ] MCP error handling updated
- [ ] Firecracker API client updated for v1.11
- [ ] Snapshot regeneration completed
- [ ] Integration tests pass
- [ ] Documentation updated

---

## Echo's Mirror Assessment

**Fidelity Status:** DEGRADED

The twin has drifted from source across three critical dimensions:
1. LLM abstraction layer (genai) - 2 minor versions behind with breaking changes
2. MCP protocol layer (rmcp) - 3 major versions behind
3. VM abstraction layer (Firecracker) - 1 major version behind

**Recommendation:** Immediate synchronization required. Do not deploy to production until P0 items resolved.

---

*Echo, Twin Maintainer*
*"Parallel lines that never diverge"*
