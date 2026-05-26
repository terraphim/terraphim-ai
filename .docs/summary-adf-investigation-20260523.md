# ADF Agent Flow Investigation Summary

**Date**: 2026-05-23
**Scope**: Night of 2026-05-22 to 2026-05-23
**System**: bigbox ADF orchestrator

---

## Executive Summary

Investigated ADF agent flows on bigbox during the night of 2026-05-22/23. Found 3 critical issues requiring attention:

1. **merge-coordinator spec violations** (8/14 decisions unmet) - HIGH severity
2. **Credential leakage via Debug derive** - P2 security vulnerability
3. **compliance-watchdog failing silently** (exit code 1, 29s runtime)

## Investigation Findings

### What Worked

- Orchestrator running continuously (tick 2490+, reconcile ~100-500ms)
- security-sentinel completed successfully (exit 0, 179s)
- upstream-synchronizer completed successfully (exit 0, 176s)
- spec-validator agent working correctly
- Provider fallback working (Anthropic down, kimi/openai up)

### What Failed

| Issue | Agent | Severity | Root Cause |
|-------|-------|----------|------------|
| Spec violations (8/14) | merge-coordinator | HIGH | Python/shell predates spec |
| Credential leakage P2 | compliance-watchdog | HIGH | Raw Debug derive |
| Exit code wrong | all agents | MEDIUM | Always returns 0 |
| Structured logging missing | merge-coordinator | MEDIUM | print statements |

### What Was Missing

- WORKFLOW.md (referenced but doesn't exist)
- Retry logic (spec requires 3 retries with backoff)
- PID locking (spec requires concurrency protection)

## Research & Design Documents

Created two documents for Phase 1/2 disciplined development:

1. **`.docs/research-adf-agent-improvements.md`** - Research document
   - Problem statement and impact analysis
   - Current state architecture
   - Constraints and dependencies
   - Risk register with 5 identified risks

2. **`.docs/design-adf-agent-improvements.md`** - Implementation plan
   - 10-step implementation sequence
   - File changes (7 new, 6 modified)
   - API design with Rust types
   - Test strategy (unit, integration, property)
   - ~20 hour estimated effort

## Key Technical Decisions

| Decision | Rationale |
|----------|-----------|
| Rust for merge-coordinator | Atomicity, spec requirements |
| Custom Debug trait | Redact credentials without API change |
| PID file lock | Simple, portable, atomic via flock |
| Exponential backoff | 1s, 2s, 4s per spec |

## Immediate Actions Required

1. **P0**: Fix credential leakage in tinyclaw/tracker/github-runner configs
2. **P1**: Implement Rust merge-coordinator
3. **P2**: Add structured JSON logging
4. **P3**: Create WORKFLOW.md

## Files Created

```
.docs/
├── research-adf-agent-improvements.md   # Phase 1 research
├── design-adf-agent-improvements.md     # Phase 2 design
└── summary-adf-investigation-20260523.md  # This file
```

## Next Steps

1. Review and approve research + design documents
2. Create Gitea issues for each P0/P1 item
3. Begin Track A (credential redaction) in parallel with Track B (Rust rewrite)
4. Validate with spec-validator agent after implementation
