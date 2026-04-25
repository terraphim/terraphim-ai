# Task Plan: Validate terraphim_rlm with All Examples

## Goal
Validate the terraphim_rlm crate functionality using all available examples found in documentation and tests.

## Phases

### Phase 1: Research (Disciplined Research) -- COMPLETED
- [x] Understand terraphim_rlm architecture
- [x] Identify all examples in documentation (8 rustdoc examples)
- [x] Identify all examples in tests (108 unit tests across 12 modules)
- [x] Map system elements and dependencies
- [x] Identify constraints and risks
- [x] Produce research document (`docs_dev/research-rlm-validation.md`)
- [x] Run quality evaluation on research -- GO (3.83/5)

### Phase 2: Design (Disciplined Design) -- COMPLETED
- [x] Define target behavior for validation
- [x] Design high-level validation approach (4-layer validation)
- [x] Create file/module change plan
- [x] Define step-by-step validation sequence (6 steps)
- [x] Design testing strategy
- [x] Review risks and mitigations
- [x] Produce design document (`docs_dev/design-rlm-validation.md`)
- [x] Run quality evaluation on design -- GO conditional (3.67/5)

### Phase 2.5: Specification (Open Questions) -- BLOCKED
- [ ] Q1: Documentation example strategy (Option A/B/C)
- [ ] Q2: LLM bridge mocking approach
- [ ] Q3: MCP tool validation scope
- [ ] Q4: Validation integration approach
- [ ] Q5: Real VM example handling
- [ ] Q6: Error case testing approach

### Phase 3: Implementation (Future)
- [ ] Create `tests/examples_validation.rs`
- [ ] Implement validation tests for each rustdoc example
- [ ] Run with different feature flags
- [ ] Generate validation report

## Test Baseline (2026-04-25)
- **108 unit tests**: ALL PASS
- **8 doc-tests**: ALL IGNORED (marked `rust,ignore`)
- **Compilation**: PASS with `full` features (requires SSH GitHub auth for fcctl-core)

## Key Documents
| Document | Path | Status |
|----------|------|--------|
| Research | `docs_dev/research-rlm-validation.md` | GO (3.83/5) |
| Design | `docs_dev/design-rlm-validation.md` | GO conditional (3.67/5) |
| Findings | `docs_dev/findings.md` | Complete |
| Quality Report | `docs_dev/quality-evaluation-report.md` | Complete |

## Infrastructure Fix Applied
- Added `infrastructure/firecracker-rust-ci` to workspace exclude (missing Cargo.toml)

## Errors Encountered
| Error | Resolution |
|-------|------------|
| Workspace broken: missing firecracker-rust-ci Cargo.toml | Added to exclude list |
| fcctl-core git fetch fails over HTTPS | Use SSH override: `git config url."git@github.com:".insteadOf` |
| terraphim_rlm excluded from workspace | Temporarily remove from exclude for builds |

## Decisions Made
| Decision | Rationale |
|----------|-----------|
| Use disciplined methodology | Ensures thoroughness and quality |
| Create separate research and design docs | Follows Phase 1 and Phase 2 structure |
| Use MockExecutor pattern | Already established in existing tests (rlm.rs:788-870) |
| Keep doc examples as `ignore` (tentative) | Pending human decision on Q1 |
