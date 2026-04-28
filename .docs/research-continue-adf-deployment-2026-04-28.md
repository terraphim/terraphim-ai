# Research Document: Continue ADF PR Fan-Out Deployment

**Status**: Draft
**Author**: opencode
**Date**: 2026-04-28
**Parent**: Epic #230 (ADF orchestration remediation)

---

## Executive Summary

The ADF PR fan-out system was successfully deployed on 2026-04-27 (Phase 2). PR #1053 documents the deployment with an operations guide and blog post. The system now runs 6 agents on every PR with branch protection enforcing required status checks. This research determines the optimal next work item by analysing the current state, open issues, and PageRank priorities.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Does this problem energise us to solve it? | **Yes** | ADF is the CI backbone; each improvement unblocks downstream work |
| Does solving this leverage our unique capabilities? | **Yes** | We have the orchestrator codebase, bigbox infrastructure, and Gitea integration |
| Does this meet a significant, validated need? | **Yes** | 392 open issues, highest PageRank issue (#149) is ADF-related |

**Proceed**: **Yes** - 3/3 YES

---

## Problem Statement

### Current State
- ADF PR fan-out (Phase 2) is live with 6 agents: `adf/build`, `adf/pr-reviewer`, `adf/spec`, `adf/security`, `adf/compliance`, `adf/test`
- Branch protection on `main` requires all 6 status checks
- PR #1053 (docs) is open and awaiting ADF checks
- Epic #230 (agent remediation) has child issue #238 still open

### What Blocks Full Deployment
1. **Issue #238**: test-guardian was disabled during remediation and never re-enabled
2. **Phase 2b-2e gap**: spec-validator, security-sentinel, compliance-watchdog, test-guardian PR variants are partially shipped but test-guardian is not fully operational
3. **Issue #149 (deferred)**: Webhook-driven mention detection has highest PageRank but is blocked by prior phases

### Impact
- test-guardian being disabled means PRs lack automated test coverage validation
- Without all agents healthy, the full promise of ADF as CI replacement is unrealised
- 382 open issues suggest the system is under active development and needs robust CI

---

## Current State Analysis

### Code Locations
| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| ADF orchestrator binary | `crates/terraphim_orchestrator/src/bin/adf.rs` | Main orchestrator entry point | Live |
| Webhook handler | `crates/terraphim_orchestrator/src/webhook.rs` | HMAC-verified Gitea webhook receiver | Live |
| PR dispatch | `crates/terraphim_orchestrator/src/pr_dispatch.rs` | Fan-out logic for PR events | Live |
| Agent config | `/opt/ai-dark-factory/conf.d/terraphim.toml` | Agent definitions on bigbox | Live |
| test-guardian config | `/opt/ai-dark-factory/orchestrator.toml` | Commented out (disabled) | **Disabled** |
| adf-ctl CLI | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Operational CLI | Live |

### Data Flow
```
Gitea PR event -> webhook.rs -> pr_dispatch.rs -> spawn agents -> status checks
                                         |
                                    test-guardian (DISABLED)
```

### Integration Points
- **Gitea API**: Status checks posted via gitea-robot (`gtr set-status`)
- **rch**: Build dispatch for deterministic checks
- **SeaweedFS S3**: Cache for cargo builds (82.83% hit rate)
- **systemd**: `adf-orchestrator.service` on bigbox

---

## Constraints

### Technical Constraints
1. **Branch protection is enforced**: Cannot push directly to main; must use PRs
2. **Subscription-only models**: C1 invariant - no pay-per-use LLM providers
3. **bigbox single-host**: All compute runs on one machine; memory constrained
4. **Gitea is canonical**: GitHub is manual/opt-in only

### Business Constraints
1. **Cost control**: Per-agent monthly budgets must not be exceeded
2. **7-day soak rule**: Each new required check must soak before next is added
3. **Operator judgement**: < 5% false-positive rate required before promoting to required

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| PR status latency | < 5 min (warm cache) | Achieved for build-runner |
| Agent spawn success | > 99% | Unknown (test-guardian disabled) |
| Disk usage | < 85% | Unknown (need to verify) |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| test-guardian must be re-enabled | Blocks full PR validation; part of agreed Phase 2 scope | Issue #238 open since 2026-04-03 |
| Branch protection rules | Cannot bypass; all changes must pass ADF checks | Enforced on main |
| Cost budget per agent | Prevents runaway LLM spend | C1 invariant in plan |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Phase 5 (webhook-driven mention) | Deferred per issue #149; blocked by prior work |
| Phase 4 (GH check-run mirror) | Dropped per D1 decision |
| New agent templates (2c, 2d, 2e) | test-guardian must be healthy first |
| Polling listener decommission | Phase 5 dependency |

---

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| Issue #231-237 (all closed) | Prerequisite work for #238 | Low (all marked complete) |
| PR #1053 (docs) | Should merge first to document current state | Low |
| PR #1045 (#664 fix) | Memory exhaustion fix; improves stability | Medium (not yet merged) |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Gitea API | Latest | Low | N/A |
| rch | Latest | Low | N/A |
| systemd | systemd 250+ | Low | N/A |

---

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| test-guardian still fails after re-enable | Medium | High | Monitor journalctl; fallback to disable if spawn-die cycle |
| Disk pressure returns | Medium | High | Verify cleanup scripts; add monitoring |
| PR #1053 blocks on ADF checks | Low | Low | Docs-only PR should pass quickly |
| Cost overrun if all agents fire | Low | Medium | Per-agent budgets already configured |

### Open Questions
1. **What is current disk usage on bigbox?** - Need to verify < 85%
2. **Did test-guardian fail due to code or config?** - Need to check if fix in PR #1045 resolves it
3. **Are all prerequisite issues (#231-237) truly complete?** - Need to verify deployment

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| PR #1045 fixes the memory issue | PR description says "fixes critical memory exhaustion" | test-guardian still fails | No (not merged) |
| test-guardian config is the only blocker | Issue #238 says "Uncomment [[agents]] block" | Could be code issue too | Partial (config disabled) |
| 7-day soak not needed for test-guardian | It's re-enabling existing agent, not new agent | Could destabilise PR checks | No |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Re-enable test-guardian immediately | Fastest path to full Phase 2 | **Chosen** - prerequisites complete |
| Wait for PR #1045 merge first | More conservative | Rejected - #1045 is separate concern |
| Redesign test-guardian from scratch | Safer but slower | Rejected - existing agent should work |

---

## Research Findings

### Key Insights
1. **test-guardian is the last disabled agent** - All other Phase 2 agents are operational
2. **PR #1053 should merge first** - It documents the current deployment state
3. **PR #1045 should be considered** - It fixes memory exhaustion which could affect test-guardian
4. **Highest PageRank issue (#149) is blocked** - Cannot proceed until basic agents are healthy

### Relevant Prior Art
- `.docs/plan-adf-agents-replace-gitea-actions.md` - Full deployment plan
- `.docs/research-adf-orchestration-remediation.md` - Previous research on agent failures
- `.docs/design-adf-orchestration-remediation.md` - Design for remediation

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify bigbox disk usage | Ensure < 85% before re-enable | 5 min |
| Check test-guardian logs | Confirm failure mode | 10 min |
| Verify PR #1053 check status | Determine if ready to merge | 5 min |

---

## Recommendations

### Proceed/No-Proceed
**Proceed** - Re-enabling test-guardian is the logical next step to complete Phase 2 deployment.

### Scope Recommendations
1. Merge PR #1053 first (docs-only, low risk)
2. Re-enable test-guardian per issue #238
3. Monitor for 24 hours
4. If stable, consider PR #1045 merge for stability

### Risk Mitigation Recommendations
1. Monitor journalctl for spawn-die cycles after re-enable
2. Have disable procedure ready if issues occur
3. Verify disk usage before any restart

---

## Next Steps

If approved:
1. Verify PR #1053 status checks are green; merge if so
2. Check bigbox disk usage and test-guardian failure logs
3. Re-enable test-guardian in orchestrator.toml
4. Restart adf-orchestrator service
5. Monitor for 24 hours
6. Close issue #238 if stable

---

## Appendix

### Reference Materials
- PR #1053: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1053
- Issue #238: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/238
- Issue #230 (Epic): https://git.terraphim.cloud/terraphim/terraphim-ai/issues/230
- PR #1045: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1045
- Plan: `.docs/plan-adf-agents-replace-gitea-actions.md`
