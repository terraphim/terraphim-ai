# Research Document: Open GitHub Issues Analysis

**Date**: 2025-12-10 (Updated: 2025-12-11)
**Methodology**: Disciplined Research (Phase 1)
**Issues Analyzed**: 20 open issues

---

## Update Log

### 2025-12-11: CI/CD Infrastructure Fix
- **#328 RESOLVED**: Path expansion bug in `twelf/shellexpand` identified and fixed
- **Root Cause**: Nested `${VAR:-${OTHER}}` syntax not supported by shellexpand
- **Fix**: Changed settings files to use `~` instead of `${HOME}` (commits `01ee2c86`, `e297d591`)
- **Result**: CI Native workflow now PASSES
- **Impact**: Package publishing (#318, #315) UNBLOCKED

---

## 1. Problem Restatement and Scope

### IN SCOPE
- 20 open GitHub issues requiring triage and prioritization
- CI/CD infrastructure failures blocking development
- Package publishing (npm, PyPI)
- Feature development (MCP aggregation, LLM linter, code assistant)
- Self-hosted runner configuration

### OUT OF SCOPE
- Closed issues
- Implementation details (Phase 2/3)
- External dependency issues outside project control

---

## 2. Issue Categories and Dependencies

### Category A: CI/CD Infrastructure (PARTIALLY RESOLVED)
| Issue | Title | Status | Dependencies |
|-------|-------|--------|--------------|
| #328 | CI/CD Infrastructure failures | **RESOLVED** | CI Native passes |
| #289 | Release workflows failing | BLOCKING | Blocks releases |
| #307 | Update GitHub Actions config | Related | Depends on #306 |
| #306 | Use self-hosted runner | In Progress | Runner deployed |

**Analysis**: Major progress made on 2025-12-11. Issue #328 root cause identified and fixed:
- **Root Cause**: `twelf/shellexpand` doesn't support nested `${VAR:-${OTHER}}` syntax
- **Fix**: Changed settings files to use `~` instead of `${HOME}` in defaults (commits `01ee2c86`, `e297d591`)
- **Result**: CI Native workflow now PASSES

Issue #289 (release workflows) remains blocking for package releases. Self-hosted runner (#306) is deployed and working.

### Category B: Package Publishing (UNBLOCKED)
| Issue | Title | Status | Dependencies |
|-------|-------|--------|--------------|
| #318 | Publish @terraphim/autocomplete to npm | **CAN PROCEED** | CI Native passes |
| #315 | Release Python Library to PyPI | **CAN PROCEED** | CI Native passes |

**Analysis**: Both packages are feature-complete with tests passing. With CI Native now passing, package publishing can proceed. Manual publishing available if automated release workflows (#289) still have issues.

### Category C: TUI Development
| Issue | Title | Status | Dependencies |
|-------|-------|--------|--------------|
| #301 | TUI Remediation Phase 1 | COMPLETED | None |

**Analysis**: Phase 1 (Emergency Stabilization) is complete. Build system operational. Ready for Phase 2 (Test Infrastructure Recovery).

### Category D: Security & Auth
| Issue | Title | Status | Dependencies |
|-------|-------|--------|--------------|
| #285 | Authentication Middleware | COMPLETED | TDD success |

**Analysis**: 7/7 tests passing. Authentication middleware implemented using TDD. Demonstrates value of test-first approach.

### Category E: MCP Aggregation (Feature)
| Issue | Title | Status | Dependencies |
|-------|-------|--------|--------------|
| #278 | Phase 1: Core MCP Aggregation | Not Started | None |
| #279 | Phase 2: Endpoint Management | Not Started | #278 |
| #280 | Phase 3: Tool Management | Not Started | #279 |
| #281 | Phase 4: Multi-tenancy & UI | Not Started | #280 |

**Analysis**: 4-phase feature for MCP server aggregation. Similar to MetaMCP. Well-defined task breakdown.

### Category F: Enhanced Code Assistant (EPIC)
| Issue | Title | Status | Dependencies |
|-------|-------|--------|--------------|
| #270 | EPIC: Beat Aider & Claude Code | Active | All sub-issues |
| #271 | Phase 1: MCP File Editing | Not Started | None |
| #272 | Phase 2: Validation Pipeline | Not Started | #271 |
| #273 | Phase 3: REPL Implementation | Not Started | #272 |
| #274 | Phase 4: KG for Code | Not Started | #273 |
| #275 | Phase 5: Recovery & Advanced | Not Started | #274 |
| #276 | Phase 6: Integration & Polish | Not Started | #275 |

**Analysis**: 6-week ambitious project to build code assistant. Well-documented requirements. Leverages existing terraphim infrastructure.

### Category G: Advanced Features
| Issue | Title | Status | Dependencies |
|-------|-------|--------|--------------|
| #292 | LLM Linter for Markdown KG | Design Complete | terraphim_automata |

**Analysis**: Comprehensive design document created. 5-phase implementation plan. Integrates with existing validation infrastructure.

---

## 3. System Elements and Dependencies

### Critical Path Analysis

```
CI/CD Infrastructure (#328, #289)
    └── #328: ✅ RESOLVED (2025-12-11) - CI Native passes
    └── #289: ⚠️ Release workflows still need fixes
    └── Package Publishing (#318, #315) - UNBLOCKED, can proceed

Self-Hosted Runner (#306, #307)
    └── ✅ Runner deployed and working
    └── CI Native uses self-hosted runner successfully

TUI Remediation (#301)
    └── Phase 1 Complete
    └── Ready for Phase 2

MCP Aggregation (#278-281)
    └── Sequential dependency chain
    └── UNBLOCKED - can start immediately

Enhanced Code Assistant (#270-276)
    └── Sequential 6-week plan
    └── UNBLOCKED - can start immediately
```

### Affected Components

| Component | Issues | Risk Level |
|-----------|--------|------------|
| `.github/workflows/` | #328, #289, #306, #307 | HIGH |
| `terraphim_automata_py/` | #328, #315 | MEDIUM |
| `terraphim_ai_nodejs/` | #318 | MEDIUM |
| `terraphim_tui/` | #301, #270-276 | LOW |
| `terraphim_mcp_server/` | #278-281 | LOW |

---

## 4. Constraints and Their Implications

### Business Constraints
- **Package Publishing**: npm and PyPI releases blocked by CI
- **Developer Experience**: False CI failures eroding confidence
- **Time Investment**: 6-week code assistant project requires sustained focus

### Technical Constraints
- **Python Bindings**: Black formatter and Maturin build issues
- **Tauri Tests**: Platform-specific dependency issues (webkit2gtk-4.1-dev)
- **Self-Hosted Runner**: Only macOS X64 available (Klarian-147)

### Security Constraints
- **1Password CLI**: Installation failures on Windows
- **API Keys**: Authentication middleware requires proper key management

---

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS
1. Why did CI/CD suddenly start failing on 2025-11-17?
2. Is the self-hosted runner (Klarian-147) still active?
3. What is the actual state of PR #288 (release workflow fixes)?

### ASSUMPTIONS
1. **ASSUMPTION**: Self-hosted runner can resolve CI issues
2. **ASSUMPTION**: Python bindings are correctly implemented
3. **ASSUMPTION**: Node.js package is ready for npm publish
4. **ASSUMPTION**: TUI Phase 1 fixes are stable

### RISKS

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| CI remains broken | HIGH | MEDIUM | Use self-hosted runner |
| Self-hosted runner offline | MEDIUM | LOW | Check tmux session |
| Python package incompatibility | MEDIUM | MEDIUM | Skip SQLite, use file persistence |
| 6-week code assistant scope creep | HIGH | HIGH | Strict phase gates |

---

## 6. Context Complexity vs. Simplicity Opportunities

### Complexity Sources
1. **Multiple CI Workflows**: 5+ failing workflows with different root causes
2. **Cross-Platform Builds**: Windows, macOS, Ubuntu with different dependencies
3. **Feature Branches**: Multiple EPICs running in parallel

### Simplification Strategies

1. **Focus on Self-Hosted Runner First**
   - Runner already deployed
   - Could bypass GitHub-hosted runner issues
   - Immediate impact on CI stability

2. **Strangler Pattern for CI**
   - Keep failing workflows but make them non-blocking
   - Gradually migrate to self-hosted runner
   - Re-enable blocking once stable

3. **Package Publishing Independence**
   - Create manual publish scripts
   - Don't block on CI for npm/PyPI releases
   - Automate after CI stabilizes

---

## 7. Questions for Human Reviewer

1. **CI Priority**: Should we disable failing CI workflows temporarily to unblock PR merges?

2. **Self-Hosted Runner**: Is the Klarian-147 runner still active? Should we verify its status?

3. **Package Publishing**: Can we do manual npm/PyPI releases while CI is broken?

4. **Feature Prioritization**: Should MCP Aggregation (#278-281) or Code Assistant (#270-276) take priority?

5. **TUI Phase 2**: What is the timeline expectation for TUI test infrastructure recovery?

6. **LLM Linter**: Is the 5-week implementation plan realistic given CI issues?

7. **PR #288**: What happened to the release workflow fixes PR? Is it merged or abandoned?

---

## 8. Prioritization Recommendation

### Immediate (This Week) - UPDATED 2025-12-11
1. ~~**#328**: Fix or disable blocking CI workflows~~ ✅ **DONE**
2. **#318/#315**: Package publishing - NOW UNBLOCKED
3. **#289**: Fix remaining release workflows

### Short-Term (Next 2 Weeks)
4. **#301**: TUI Phase 2 - Test Infrastructure Recovery
5. **#278**: Begin MCP Aggregation Phase 1
6. **#270**: Start Enhanced Code Assistant EPIC

### Medium-Term (Month)
7. **#270-276**: Complete Enhanced Code Assistant phases
8. **#292**: LLM Linter implementation

---

## 9. Summary Statistics

| Category | Count | Blocking | Ready | In Progress | Completed |
|----------|-------|----------|-------|-------------|-----------|
| CI/CD | 4 | 1 | 1 | 1 | 1 |
| Publishing | 2 | 0 | 2 | 0 | 0 |
| TUI | 1 | 0 | 0 | 0 | 1 |
| Security | 1 | 0 | 0 | 0 | 1 |
| MCP | 4 | 0 | 4 | 0 | 0 |
| Code Assistant | 7 | 0 | 7 | 0 | 0 |
| LLM Linter | 1 | 0 | 1 | 0 | 0 |
| **Total** | **20** | **1** | **15** | **1** | **3** |

*Updated 2025-12-11: #328 resolved, blocking count reduced from 2 to 1*

---

*Research completed using disciplined-research methodology. Ready for Phase 2 (Design) and Phase 3 (Implementation) on approved priorities.*
