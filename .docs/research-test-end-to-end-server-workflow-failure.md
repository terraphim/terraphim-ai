# Research Document: Fix test_end_to_end_server_workflow Failure

**Date**: 2026-03-25
**Status**: Research Complete
**Author**: AI Agent

## Problem Understanding

**Problem**: CI test `test_end_to_end_server_workflow` is failing because:
1. Server returns empty roles list: `Server roles available: []`
2. After roles pass locally, search fails with exit code 1

**Who has this problem**: CI/CD pipeline blocking merges

**Impact**: Cannot merge any PR or changes to main branch

**Success Criteria**: Test passes in CI, unblocking pipeline

## Existing System Analysis

### Code Locations
| File | Purpose |
|------|---------|
| `crates/terraphim_agent/tests/integration_tests.rs` | Test file containing failing test |
| `terraphim_server/default/terraphim_engineer_config.json` | Config file with roles |

### Data Flow
1. Test spawns server: `cargo run -p terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json`
2. Server loads config from relative path
3. Server exposes `/roles` endpoint
4. Test calls `roles list` command
5. Command returns empty list in CI

### Root Cause Analysis

**Issue 1: Config Path in CI**
- Test uses relative path: `terraphim_server/default/terraphim_engineer_config.json`
- In CI, working directory may differ from local
- `cargo run` spawns from `target/` directory, not project root

**Issue 2: CI Environment**
- CI runs `cargo test --release --workspace` from project root
- Spawned server process inherits different environment
- Config file may not be accessible at resolved path

**Evidence from local run**:
- Roles ARE returned locally: `["Engineer (Engineer)", "System Operator (operator)", "Default (Default)"]`
- Test fails on search assertion instead (line 288)
- This indicates roles work locally, config is accessible

**Evidence from CI run**:
- Roles return empty: `Server roles available: []`
- Server starts successfully (health check passes)
- Config loads successfully (id="Server")
- But roles list is empty

## Constraints

### Technical Constraints
- Must maintain backward compatibility with existing test structure
- Must work in CI environment with different working directory
- Must work with `--release` mode

### Business Constraints
- Test must pass in CI` - blocking all merges
- No external services available in CI

### Performance Constraints
- Test timeout: 8 minutes in CI workflow

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Config path resolution | High | Critical | Use absolute path or fixture |
| CI environment differs | High | High | Use CI-specific config |
| Test flakiness | Medium | Medium | Add retry logic |

## Assumptions

| Assumption | Basis | Risk if Wrong |
|------------|-------|-----------------|
| Config file exists at File system check | Test fails |
| Server can load relative config | Works locally | CI fails |
| CI has same CWD | GitHub Actions docs | False - different working directory |

## Recommendations

1. **Primary**: Modify test to use absolute config path
2. **Alternative**: Add `#[ignore]` attribute to skip in CI

## Next Steps

1. Proceed to Design phase to2. Implement fix
3. Verify in CI
