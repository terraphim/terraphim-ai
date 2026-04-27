# Research Document: Stable Release Plan for Terraphim AI

## 1. Problem Restatement and Scope

**Problem**: The Terraphim AI project has accumulated 1,824 commits since the last stable release (v1.16.37), with significant changes across the ADF orchestrator, agent system, CI infrastructure, and security posture. The codebase currently fails to compile due to a `zlob` dependency issue with Zig 0.16. We need to identify the most stable release candidate from recent changes and create a plan to achieve a stable, releasable state.

**IN Scope**:
- Analysing commit history and change categories since v1.16.37
- Identifying critical bugs, security fixes, and breaking changes
- Assessing compilation and test status
- Creating a release readiness plan

**OUT of Scope**:
- Implementing new features
- Major architectural refactoring
- Resolving all historical technical debt

## 2. User & Business Outcomes

- **Stable release**: A version that compiles, passes tests, and has no known critical security vulnerabilities
- **Clear upgrade path**: Documentation of breaking changes and migration steps
- **Confidence in quality**: Evidence-based assessment of release readiness

## 3. System Elements and Dependencies

| Component | Changes Since v1.16.37 | Risk Level |
|-----------|------------------------|------------|
| ADF Orchestrator | 327 commits (feat/fix) | High - Major new system |
| Agent System | 47 feat + 26 fix commits | Medium - Mature but evolving |
| CI/Infrastructure | 151 fix(ci) + 42 ci commits | Medium - Firecracker, sccache |
| Security | 6 fix(security) + RUSTSEC fixes | Low - Proactively addressed |
| Session Connectors | OpenCode, Codex, Aider, Cline | Medium - New integrations |
| Learning Store | SharedLearningStore, markdown backend | Medium - Recently refactored |
| zlob dependency | Build failure with Zig 0.16 | **Critical - Blocks compilation** |

## 4. Constraints and Their Implications

- **Compilation must succeed**: Current `zlob` build failure blocks everything
- **Security patches applied**: RUSTSEC-2024-0421, RUSTSEC-2026-0098/0099/0104 addressed
- **CI stability**: 151 CI fixes suggest the pipeline has been volatile
- **Feature completeness**: ADF orchestrator is a major new system (327 commits)
- **Test coverage**: 666 test-related commits suggest heavy investment in quality

## 5. Risks, Unknowns, and Assumptions

**Critical Risks**:
1. **zlob compilation failure**: Blocks all builds. Assumption: Can be fixed by updating zlob or Zig version.
2. **ADF complexity**: 327 commits in a new orchestrator system may harbour undiscovered bugs.
3. **CI instability**: 151 CI fixes indicate the build pipeline has been fragile.

**Unknowns**:
- Test pass rate (compilation blocked by zlob)
- Integration test status for new session connectors
- Performance impact of LearningStore changes

**Assumptions**:
- The `zlob` issue is resolvable without major code changes
- Security fixes are complete and effective
- ADF orchestrator is feature-complete enough for release

## 6. Context Complexity vs. Simplicity Opportunities

**Complexity Sources**:
- 1,824 commits across multiple major systems
- New orchestrator, session connectors, learning store, CI infrastructure
- Multiple RUSTSEC security patches

**Simplification Strategies**:
1. **Fix zlob first**: Unblock compilation to enable testing
2. **Scope the release**: Consider whether ADF orchestrator should be in this release or held back
3. **Stabilise CI**: Ensure the 151 CI fixes have actually resolved pipeline issues

## 7. Questions for Human Reviewer

1. Should the ADF orchestrator (327 commits, major new system) be included in this release or held for a subsequent one?
2. What is the priority for fixing the `zlob` Zig 0.16 compilation issue?
3. Are there any specific integration tests that must pass before release?
4. Should we target v1.17.0 or v1.16.38 for the next release?
5. What is the minimum acceptable test coverage threshold?
