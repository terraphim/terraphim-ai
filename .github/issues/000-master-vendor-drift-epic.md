---
title: "MASTER: Vendor API Drift Remediation - Q1 2026"
labels: ["priority/P0", "type/epic", "component/integration", "echo/drift-detected"]
assignees: []
milestone: "Q1-2026"
---

## Summary

**Echo, Twin Maintainer** - Critical drift detected across multiple vendor API boundaries. This epic tracks all remediation efforts to restore twin fidelity.

## Drift Overview

| Vendor | Current | Target | Severity | Status |
|--------|---------|--------|----------|--------|
| rust-genai | v0.4.4-WIP | v0.5.3/v0.6.0 | **CRITICAL** | 🔴 Open |
| rmcp (MCP SDK) | v0.9.1 | v1.2.0 | **CRITICAL** | 🔴 Open |
| Firecracker | v1.10.x | v1.11.0 | MODERATE | 🟡 Open |
| 1Password CLI | Unknown | Latest | LOW | 🟢 Monitoring |
| Atomic Data | Unknown | Latest | LOW | 🟢 Monitoring |

## Issue Tracker

### P0 - Critical (Blocking)
- [ ] #1 - rust-genai v0.4.4 → v0.6.0 upgrade
- [ ] #2 - rmcp v0.9.1 → v1.2.0 upgrade

### P1 - Moderate
- [ ] #3 - Firecracker v1.11.0 upgrade

### P2 - Low (Monitoring)
- [ ] #4 - Vendor API monitoring dashboard

## Dependency Graph

```
#1 (rust-genai)
    │
    ├── blocks: #2 (rmcp) - coordinated reqwest version
    │
    └── independent: #3 (Firecracker)

#2 (rmcp)
    │
    └── blocked by: #1

#3 (Firecracker)
    │
    └── independent
```

## Execution Order

### Phase 1: P0 Items (Parallel where possible)
1. **Week 1-2:** #1 rust-genai upgrade
   - Update reqwest workspace-wide
   - Migrate API usage patterns
   - Test all LLM providers

2. **Week 2-3:** #2 rmcp upgrade
   - Update MCP SDK
   - Fix match statements
   - Test MCP server functionality

### Phase 2: P1 Items
3. **Week 3-4:** #3 Firecracker upgrade
   - Update API client
   - Regenerate snapshots
   - Test VM operations

### Phase 3: P2 Items
4. **Ongoing:** #4 Monitoring setup
   - Automated changelog scanning
   - Drift detection alerts

## Success Criteria

- [ ] All P0 issues closed
- [ ] All integration tests passing
- [ ] LLM providers functional (OpenAI, Anthropic, Groq)
- [ ] MCP server operational
- [ ] GitHub runner VMs working
- [ ] No security advisories from cargo-deny
- [ ] Documentation updated

## Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| reqwest 0.13 breaks other deps | HIGH | HIGH | Test all crates before merge |
| LLM API changes affect prompts | MEDIUM | MEDIUM | Integration test suite |
| MCP breaking changes | HIGH | MEDIUM | Extensive testing |
| Firecracker snapshot regeneration fails | LOW | HIGH | Backup snapshots first |
| Coordinated upgrade complexity | HIGH | MEDIUM | Clear dependency chain |

## Communication Plan

- **Daily:** Standup on progress
- **Weekly:** Review blockers
- **Milestone:** Post-mortem on drift detection

## Definition of Done

- All sub-issues closed
- cargo-deny passes
- Integration tests pass
- CHANGELOG updated
- Migration guide published

## Echo's Notes

**Mirror Status:** Currently DEGRADED
- rust-genai: 2 minor versions behind (breaking API changes)
- rmcp: 3 major versions behind (non_exhaustive breaking)
- Firecracker: 1 major version behind (snapshot breaking)

**Zero-deviation principle violated.** Synchronization required before production deployment.

**Recommended:** 
- Assign #1 and #2 to same engineer (coordinated reqwest upgrade)
- #3 can be parallel but coordinate CI/CD changes
- Consider pinning policy for vendor deps

---

*"Parallel lines that never diverge" - Echo*
