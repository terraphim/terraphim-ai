# Unit Test Traceability Matrix

**Feature**: ADF Configuration Validation
**Phase 2 Doc**: `.docs/design-adf-validation.md`
**Phase 2.5 Doc**: `.docs/design-adf-validation.md` (Specification Interview Findings)

## Coverage Summary

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Functions validated | 1 (validate) | 1 | PASS |
| Edge cases covered | 7 | 7 | PASS |
| Error paths covered | 6 | 6 | PASS |

## Traceability

### Design Elements → Tests

| Design Section | Code Location | Test | Edge Cases | Status |
|----------------|--------------|------|------------|--------|
| D2: grace_period_secs range (5s-300s) | config.rs:1385-1398 | `test_validate_grace_period_too_low`, `test_validate_grace_period_too_high`, `test_validate_grace_period_in_range` | <5s rejected, >300s rejected, 5-300s accepted | PASS |
| D3: max_cpu_seconds range (60s-7200s) | config.rs:1400-1413 | `test_validate_max_cpu_too_low`, `test_validate_max_cpu_too_high`, `test_validate_max_cpu_in_range` | <60s rejected, >7200s rejected, 60-7200s accepted | PASS |
| D4: probe_ttl_secs minimum (≥60s) | config.rs:1415-1423 | `test_validate_probe_ttl_too_short`, `test_validate_probe_ttl_in_range`, `test_validate_no_routing_no_probe_validation` | <60s rejected, ≥60s accepted, no routing = no probe validation | PASS |

### Specification Findings → Tests

| Spec Finding | Test | Status |
|--------------|------|--------|
| grace_period_secs field exists | `test_validate_grace_period_*` | PASS |
| max_cpu_seconds field exists | `test_validate_max_cpu_*` | PASS |
| probe_ttl_secs on RoutingConfig | `test_validate_probe_ttl_*` | PASS |
| Validation is additive only | `test_validate_grace_period_in_range`, `test_validate_max_cpu_in_range`, `test_validate_probe_ttl_in_range` all pass alongside existing tests | PASS |

### Error Paths

| Error Variant | Test | Status |
|--------------|------|--------|
| AgentFieldOutOfRange (grace_period) | `test_validate_grace_period_too_low`, `test_validate_grace_period_too_high` | PASS |
| AgentFieldOutOfRange (max_cpu) | `test_validate_max_cpu_too_low`, `test_validate_max_cpu_too_high` | PASS |
| ProbeTtlTooShort | `test_validate_probe_ttl_too_short` | PASS |

## Gaps Identified

| Gap | Severity | Action | Status |
|-----|----------|--------|--------|
| None | - | - | - |

## Validation Tests Added

```
test_validate_grace_period_too_low        - Rejects grace_period_secs < 5s
test_validate_grace_period_too_high       - Rejects grace_period_secs > 300s
test_validate_grace_period_in_range        - Accepts grace_period_secs in [5, 300]
test_validate_max_cpu_too_low             - Rejects max_cpu_seconds < 60s
test_validate_max_cpu_too_high            - Rejects max_cpu_seconds > 7200s
test_validate_max_cpu_in_range             - Accepts max_cpu_seconds in [60, 7200]
test_validate_probe_ttl_too_short         - Rejects probe_ttl_secs < 60s
test_validate_probe_ttl_in_range          - Accepts probe_ttl_secs ≥ 60s
test_validate_no_routing_no_probe_validation - Passes when no routing config
```

## Integration with Existing Tests

| Existing Test | New Validation Impact | Status |
|---------------|---------------------|--------|
| All existing config tests | New validation blocks execute after existing checks | PASS |
| test_config_validate_gitea_issue_with_workflow_ok | New checks pass (no grace_period/max_cpu set) | PASS |
| test_config_validate_gitea_issue_requires_workflow | New checks pass (no grace_period/max_cpu set) | PASS |
| test_validate_model_provider_rejects_bare_banned | New checks pass (provider validation is independent) | PASS |
