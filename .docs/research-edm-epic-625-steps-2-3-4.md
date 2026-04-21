# Research Document: EDM Scanner Epic #625 -- Steps 2, 3, 4

## 1. Problem Restatement and Scope

**Problem**: The EDM scanner crate (#626, DONE) needs integration testing, compound review wiring, and an LSP crate to complete the epic.

**Remaining steps**:
- Step 2 (#627): Integration tests against the terraphim-ai codebase
- Step 3 (#628): Wire `run_static_precheck()` into `CompoundReviewWorkflow`
- Step 4 (#629): `terraphim_lsp` workspace crate with `publishDiagnostics`

**In scope**: All three steps. Sequential dependency: 2 -> 3 -> 4.

**Out of scope**: #735 (TermAction types -- future refactor)

## 2. System Elements and Dependencies

### Existing (from Step 1)
| Element | Location | State |
|---------|----------|-------|
| `terraphim_negative_contribution` | `crates/terraphim_negative_contribution/` | DONE, merged |
| `NegativeContributionScanner` | `scanner.rs` | 40 unit tests passing |
| `data/edm_tier1.json` | `data/` | 6 Tier 1 patterns as KG thesaurus |
| `is_non_production()` | `exclusion.rs` | Path + content exclusion |

### Touchpoints for Step 3
| Element | Location | What Changes |
|---------|----------|-------------|
| `CompoundReviewConfig` | `config.rs:491-532` | Add `stub_scan_enabled`, `allowed_stub_paths` |
| `CompoundReviewWorkflow::run()` | `compound.rs:261-393` | Insert pre-check between file gathering (L277) and agent spawn (L319) |
| `AgentOrchestrator` | `lib.rs:170-189` | Add `scanner: Arc<NegativeContributionScanner>` field |
| `AgentOrchestrator::new()` | `lib.rs:429-430` | Initialise scanner |
| `deduplicate_findings()` | `compound.rs:377` | Already handles merged findings correctly |
| `auto_file_issues` | `lib.rs:3876-3886` | Dedup key `(file, line, category)` prevents double filing |

### Touchpoints for Step 4
| Element | Location | What Changes |
|---------|----------|-------------|
| Workspace `Cargo.toml` | Root `Cargo.toml` | Exclude `terraphim_lsp` from default-members (like symphony) |
| `claude_tests/tui_editor/terraphim-lsp/` | N/A | **Does not exist** -- must create from scratch |

## 3. Constraints

| Constraint | Implication |
|------------|-------------|
| Scanner built once at orchestrator startup | `Arc<NegativeContributionScanner>` in orchestrator, not per review |
| Pre-check runs before LLM swarm | Insert between `get_changed_files` (L277) and agent spawn loop (L309) |
| `deduplicate_findings` key is `(file, line, category)` | Static and LLM findings for same stub at same line auto-dedup |
| No existing LSP prototype | Must create `terraphim_lsp` from scratch using `tower-lsp` |
| Feature-flagged LSP binary | `required-features = ["terraphim-lsp"]` in Cargo.toml |
| 250ms debounce in LSP | Tokio spawn + abort handle pattern |
| Line 0 -> LSP line 0 (not u32::MAX) | Use `if line == 0 { 0 } else { line - 1 }` not `saturating_sub` |
| Step 2 is documentation, not pass/fail gate | Findings are logged, not asserted to zero |

## 4. Risks, Unknowns, Assumptions

### Risks
| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| More than 10 EDM findings in codebase | Step 2 gate triggers audit | Medium | Expected; codebase has known deferred areas |
| `CompoundReviewWorkflow` not `Arc`-safe | Need to restructure | Low | Scanner is Arc-wrapped at orchestrator level |
| LSP crate pulls heavy deps | Workspace build bloat | Medium | Exclude from default-members |

### Unknowns
- How many `todo!()` instances exist in production code (Step 2 will reveal this)
- Whether `tower-lsp` is already in the dependency tree

### Assumptions
- `tower-lsp` is the standard Rust LSP framework (used across ecosystem)
- Compound review config is loaded from TOML at startup
- Changed files list from `get_changed_files()` gives relative paths

## 5. Questions for Human Reviewer

1. **Step 3 wiring**: Should the static pre-check add its findings to the `agent_outputs` vec (as a pseudo-agent output), or merge directly into `all_findings` before dedup?
2. **Step 4 scope**: Should I defer the LSP crate (#629) to a separate session given its complexity (4-6 hours)?
3. **Config defaults**: Should `stub_scan_enabled` default to `true` or `false`?
