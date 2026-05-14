# Research Document: ADF Configuration Validation

**Status**: Draft
**Author**: Claude (AI Agent)
**Date**: 2026-05-12
**Reviewers**: [Pending]
**Parent Issue**: Issues 1412, 256, 172, 1430

## Executive Summary

The AI Dark Factory (ADF) orchestrator's configuration validation is incomplete. While `validate()` in `config.rs:1285` checks workflow config, pre-check strategies, project IDs, and C1 banned subscription providers, it does NOT validate:

1. Provider probe rate-limit configuration (issue 1412)
2. Agent timeout values (issue 256 - 52% timeout rate)
3. Fallback model chains (issue 172)
4. Stewardship/evaluation criteria (issue 1430)

This research documents the current state, identifies gaps, and recommends a validation framework.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | 52% timeout rate burns budget; probe rate-limits cause API waste |
| Leverages strengths? | Yes | We're already doing C1 provider validation; extend it |
| Meets real need? | Yes | 4 open issues directly cite validation gaps |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
The ADF orchestrator validates configuration at load time via `OrchestratorConfig::validate()` (`config.rs:1285`), but the validation is incomplete. Gaps allow misconfigured agents to run, causing:

- **API budget waste**: Provider probes burn rate-limit budget every probe cycle (1412)
- **Agent timeouts**: 52% timeout rate due to improper timeout config (256)
- **Fallback failures**: No validation of fallback chain completeness (172)
- **Stewardship gaps**: No validation of agent evaluation criteria (1430)

### Impact
- Production budget overrun from probe API calls
- Failed agent runs from timeout misconfiguration
- Cascading failures when fallback chains are broken
- No visibility into agent quality/evaluation

### Success Criteria
1. `validate()` catches all 4 gap categories before runtime
2. `adf --check` returns non-zero exit code for invalid config
3. Validation errors are actionable (clear message + location)
4. No false positives (valid configs pass)

## Current State Analysis

### Existing Implementation

```rust
// config.rs:1285 - Current validate() function
pub fn validate(&self) -> Result<(), OrchestratorError> {
    // 1. Workflow validation (api_key, endpoint when enabled)
    // 2. Pre-check strategy validation (GiteaIssue requires workflow)
    // 3. Project ID uniqueness
    // 4. Agent/flow project references
    // 5. C1 banned subscription providers (model/fallback_model)
}
```

**What validate() checks:**
| Check | Line | Description |
|-------|------|-------------|
| Workflow config | 1287-1300 | api_key and endpoint required when enabled |
| Pre-check deps | 1302-1312 | GiteaIssue pre-check requires workflow |
| Project ID uniqueness | 1314-1322 | No duplicate project IDs |
| Agent project refs | 1326-1347 | Valid project references; no mixed mode |
| Flow project refs | 1349-1367 | Valid flow project references |
| C1 providers | 1369-1377 | model/fallback_model against allowlist |

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Config struct | `config.rs` | `OrchestratorConfig`, `AgentDefinition` |
| validate() | `config.rs:1285` | Main validation entry point |
| validate_model_provider() | `config.rs:1034` | C1 provider allowlist check |
| adf --check | `bin/adf.rs:147` | CLI that runs validation |

### Data Flow

```
adf --check CONFIG
    └── OrchestratorConfig::from_file(path)
    └── OrchestratorConfig::validate()
            ├── Workflow validation
            ├── Pre-check validation
            ├── Project ID validation
            ├── Agent/Flow reference validation
            └── Provider validation (C1)
    └── print_routing_table()
```

### Integration Points

- **OrchestratorError** (`error.rs`): All validation errors funnel here
- **AgentDefinition** (`config.rs`): Contains `model`, `fallback_model`, `timeout`, `pre_check`
- **Provider allowlists** (`config.rs:1034`): `ALLOWED_PROVIDER_PREFIXES`, `BANNED_PROVIDER_PREFIXES`

## Constraints

### Technical Constraints
1. **Sync validation only**: `validate()` is synchronous; cannot make async probe calls
2. **Load-time only**: Validation happens at config load; no runtime re-validation
3. **TOML-based**: Configuration is TOML; validation is struct-based
4. **No external deps at validation time**: Cannot call external APIs (rate-limit status)

### Business Constraints
1. **Backward compatibility**: Existing valid configs must continue to pass
2. **Actionable errors**: Error messages must point to exact field + reason
3. **Zero false positives**: Cannot break existing deployments

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Validation coverage | 4 gap categories | 0/4 (none covered) |
| Error clarity | Field + reason | Generic errors |
| Performance | < 100ms | N/A (config only) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| **Sync-only validation** | Cannot make async API calls at load time | Rust sync fn signature |
| **Backward compatibility** | Must not break existing valid configs | Production deployment |
| **Actionable errors** | Operators need clear fix instructions | Issue 256 (52% timeout rate) |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Runtime probe validation | Requires external API calls; belongs in health check |
| Evaluation criteria schema | Too broad; separate issue 1430 tracks this |
| Timeout auto-tuning | Requires ML/historical analysis; not validation |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `OrchestratorError` enum | All validation errors extend this | Low |
| `AgentDefinition` struct | Timeout/probe fields defined here | Low |
| `OrchestratorConfig::validate()` | Entry point for all validation | Medium (must not break) |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `toml` crate | 0.8.x | Low | None needed |
| No external API calls | N/A | N/A | Probes validated by schema, not runtime |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Gaps in timeout validation** | Confirmed (issue 256) | High - 52% timeout rate | Add timeout range checks |
| **Rate-limit not checked** | Confirmed (issue 1412) | High - burns budget | Validate probe intervals |
| **Fallback chain untested** | Confirmed (issue 172) | Medium - silent failures | Validate chain completeness |

### Open Questions

1. **What is a "valid" timeout range?** - Is 5s ever valid? 1h? Need domain expertise.
2. **What probe intervals respect rate limits?** - Varies by provider; need allowlist.
3. **What constitutes a valid fallback chain?** - Is single fallback enough? Three-level chain?
4. **Who approves validation rules?** - Need sign-off before implementation.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Timeout must be > 0 | Zero timeout = instant failure | Immediate production incident | No |
| Probe interval > 60s | Rate-limit respect | Budget burn | No |
| Fallback chain max 2 deep | Complexity/traceability | Broken chains | No |
| Validation at load is sufficient | Current architecture | Runtime config drift | No |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **A. Validate everything at load** | Simpler, but can't check external state (rate limits) | Current approach; can't validate external state |
| **B. Hybrid: schema + runtime health check** | More complete but complex | Better for rate-limits; separate issue |
| **C. Defer to operator** | Minimal work, max risk | Rejected - 4 open issues prove this fails |

**Chosen**: Option A (schema validation at load) + Option B (runtime health check as separate issue)

## Research Findings

### Key Insights

1. **C1 provider validation exists**: `validate_model_provider()` at `config.rs:1034` already checks model/fallback_model against allowlist/banlist. This pattern should be extended.

2. **Timeout is a first-class field**: `AgentDefinition` has a `timeout` field but it's NOT validated. We need range checks (e.g., 30s ≤ timeout ≤ 1h).

3. **Probe configuration is missing from schema**: There is NO probe configuration in `AgentDefinition` or `OrchestratorConfig`. Issue 1412 implies this should exist.

4. **Fallback chain is a first-class concern**: Issue 172 (compound-review fallback validation) indicates fallback chains need validation - but there's no `fallback_chain` field, only `fallback_model`.

5. **Stewardship is out of scope for validate()**: Issue 1430 is about evaluation criteria - this is a runtime concern, not load-time validation.

### Relevant Prior Art

- **Kubernetes validation webhooks**: Validate CRD at admission time; reject invalid configs
- **Terraform validate command**: Local-only schema validation before apply
- **Ansible ansible-lint**: Rule-based validation of playbook semantics

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Probe config schema | Find where probe config should live | 2 hours |
| Timeout range research | Determine reasonable min/max | 4 hours |
| Fallback chain pattern | Define valid chain structure | 2 hours |

## Recommendations

### Proceed/No-Proceed

**Proceed**: Yes. Four open issues cite validation gaps. The C1 provider validation pattern exists and should be extended to cover timeout, probes, and fallback chains.

### Scope Recommendations

1. **Include in Phase 2 (this work)**:
   - Timeout range validation (30s ≤ timeout ≤ 1h)
   - Fallback model validation (if present, must be valid provider)
   - Probe interval minimum (if configured, must be ≥ 60s)

2. **Exclude (separate issues)**:
   - Runtime probe health check (issue 1412 - separate design)
   - Stewardship evaluation criteria (issue 1430 - separate design)
   - Auto-tuning timeouts (defer to ML/historical analysis)

### Risk Mitigation Recommendations

1. **Add validation incrementally**: Don't add all 3 validations at once; start with timeout
2. **Use additive validation**: New rules should only fail NEW invalid configs, not existing valid ones
3. **Test with production configs**: Validate against real orchestrator.toml files before shipping

## Next Steps

If approved:

1. **Create design document** (Phase 2 - disciplined-design skill)
2. **Implement timeout validation first** (highest impact per issue 256)
3. **Add fallback model validation** (extends existing C1 pattern)
4. **Design probe schema** (requires spike - where does probe config live?)

## Appendix

### Reference Materials

- Issue 1412: Provider probes are not rate-limit aware
- Issue 256: 52% agent timeout rate
- Issue 172: compound-review fallback validation
- Issue 1430: Stewardship Agent Evaluation
- `config.rs:1285` - Current validate() implementation
- `config.rs:1034` - validate_model_provider() pattern

### Code Snippets

**Current validate() signature** (`config.rs:1285`):
```rust
pub fn validate(&self) -> Result<(), OrchestratorError> {
    // ... existing checks ...
    Ok(())
}
```

**Existing provider validation pattern** (`config.rs:1034`):
```rust
pub(crate) fn validate_model_provider(
    agent_name: &str,
    field: &str,
    model: &str,
) -> Result<(), OrchestratorError> {
    // Check against ALLOWED_PROVIDER_PREFIXES
    // Check against BANNED_PROVIDER_PREFIXES
    // Return OrchestratorError::BannedProvider if invalid
}
```
