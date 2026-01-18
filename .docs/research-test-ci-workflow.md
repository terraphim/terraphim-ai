# Research Document: test-ci.yml Workflow Running Zero Real Commands

## 1. Problem Restatement and Scope

### Problem Statement
The `.github/workflows/test-ci.yml` workflow reports "success" but only executes echo statements, providing no actual validation of code quality. This creates a false sense of security where CI appears to pass but no meaningful tests, builds, or checks are performed.

### Evidence
- Workflow completes in ~5 seconds (real CI takes 20-30 minutes)
- Steps only contain `echo "..."` statements
- No `actions/checkout@v6` to get code
- No `cargo` commands for testing/building
- No actual test execution

### IN Scope
- Fixing the test-ci.yml workflow to run actual commands
- Making it consistent with other CI workflows in the project
- Integrating with the GitHub runner integration feature (PR #381)

### OUT of Scope
- Changing other CI workflows (ci-native.yml, ci-pr.yml, etc.)
- Firecracker VM integration in this workflow
- LLM-based workflow parsing

## 2. User & Business Outcomes

### Expected Behavior
When test-ci.yml runs, it should:
1. Checkout the actual repository code
2. Run format/lint checks (`cargo fmt --check`, `cargo clippy`)
3. Run compilation checks (`cargo check`)
4. Execute unit tests (`cargo test --workspace --lib`)
5. Provide meaningful pass/fail status

### Current Behavior
- Workflow always succeeds (just prints text)
- No code is checked out
- No actual validation occurs
- False positive CI status misleads developers

### Business Impact
- PRs may be merged with untested code
- Build failures discovered only after merge
- Reduced confidence in CI/CD pipeline
- GitHub runner integration claims to execute workflows, but example workflow is fake

## 3. System Elements and Dependencies

### Workflow File
| Element | Location | Role |
|---------|----------|------|
| test-ci.yml | `.github/workflows/test-ci.yml` | Demo workflow for GitHub runner integration |

### Related Workflows
| Workflow | Purpose | Real Commands |
|----------|---------|--------------|
| ci-native.yml | Main CI pipeline | Yes - cargo build, test, clippy |
| ci-pr.yml | PR validation | Yes - full validation |
| test-minimal.yml | Quick validation | Partial - checkout + basic checks |
| test-firecracker-runner.yml | VM test | No - also just echo statements |

### CI Scripts Available
| Script | Purpose |
|--------|---------|
| `scripts/ci-quick-check.sh` | Fast pre-commit validation |
| `scripts/ci-check-tests.sh` | Full test suite |
| `scripts/ci-check-format.sh` | Formatting checks |
| `scripts/ci-check-rust.sh` | Rust build/test |

### Dependencies
- Rust toolchain 1.87.0
- cargo, rustfmt, clippy
- For full tests: webkit2gtk-4.1-dev and other system libs

## 4. Constraints and Their Implications

### Performance Constraint
- **Why it matters**: Quick feedback for developers
- **Implication**: Use lightweight checks, not full build
- **Recommendation**: Model after `scripts/ci-quick-check.sh` pattern

### Runner Constraint
- **Why it matters**: GitHub-hosted runners have limited resources
- **Implication**: Cannot run full integration tests requiring Firecracker
- **Recommendation**: Run unit tests and static analysis only

### Consistency Constraint
- **Why it matters**: Must align with GitHub runner integration claims
- **Implication**: If PR claims 35 workflows are active, test-ci should be functional
- **Recommendation**: Make test-ci actually validate something

### Time Constraint
- **Why it matters**: PRs should not wait 30+ minutes for simple checks
- **Implication**: Quick check workflow should complete in 5-10 minutes
- **Recommendation**: Skip heavy integration tests in this workflow

## 5. Risks, Unknowns, and Assumptions

### Unknowns
1. **Intended purpose of test-ci.yml**: Was it meant to be a placeholder or real workflow?
2. **Target runner**: Should it run on ubuntu-latest or self-hosted?
3. **Integration with Firecracker**: Should test-ci be executable by GitHub runner integration?

### Assumptions
1. **ASSUMPTION**: test-ci.yml was created as a quick placeholder and never updated
2. **ASSUMPTION**: It should run basic Rust validation (fmt, clippy, test)
3. **ASSUMPTION**: It should use GitHub-hosted runners (ubuntu-latest)

### Risks
| Risk | Severity | Mitigation |
|------|----------|------------|
| Adding too many checks slows PR feedback | Medium | Use only fast checks |
| System deps missing on ubuntu-latest | Medium | Use cargo check, not full build |
| Integration tests fail on GH runners | Low | Only run unit tests |

## 6. Context Complexity vs. Simplicity Opportunities

### Complexity Sources
1. Many overlapping CI workflows (35 total)
2. Mix of self-hosted and GitHub-hosted runners
3. Heavy system dependencies for Tauri builds

### Simplification Opportunities
1. **Quick Check Pattern**: Use `cargo check` instead of `cargo build`
2. **Unit Tests Only**: Skip integration tests requiring system libs
3. **Existing Scripts**: Leverage `scripts/ci-quick-check.sh` logic
4. **Single Purpose**: Make test-ci focused on quick validation only

## 7. Questions for Human Reviewer

1. **What was the original intent of test-ci.yml?** Was it meant to be a placeholder or did it get created incorrectly?

2. **Should test-ci.yml use self-hosted runners?** This would enable access to system dependencies but may not be appropriate for a quick test workflow.

3. **What specific checks are most valuable?** Options: fmt check, clippy, cargo check, unit tests

4. **Should test-firecracker-runner.yml also be fixed?** It has the same echo-only issue.

5. **Is there a specific reason these workflows don't run real commands?** Perhaps intentional for the GitHub runner integration demo?

---

**Conclusion**: The test-ci.yml workflow is a placeholder that needs to be replaced with actual CI commands. The simplest fix is to add checkout and basic Rust validation (fmt, clippy, check, unit tests) using patterns from existing scripts.

**Recommended Approach**: Transform test-ci.yml to run:
1. `actions/checkout@v6`
2. `cargo fmt --all -- --check`
3. `cargo clippy --workspace -- -W clippy::all`
4. `cargo check --workspace`
5. `cargo test --workspace --lib`

This provides meaningful validation in ~5-10 minutes on GitHub-hosted runners.
