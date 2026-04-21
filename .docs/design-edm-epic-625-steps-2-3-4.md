# Design & Implementation Plan: EDM Scanner Epic #625 -- Steps 2, 3, 4

## 1. Target Behavior

After implementation:
- Integration tests scan the terraphim-ai codebase and document all EDM findings
- Compound review runs a zero-LLM static pre-check before spawning the LLM swarm
- Static findings merge with LLM findings and dedup prevents double issue filing
- `terraphim_lsp` crate provides editor diagnostics with 250ms debounce

## 2. Key Invariants and Acceptance Criteria

### Step 2 (#627): Integration Test Baseline
- [ ] `test_scan_terraphim_ai_codebase` scans all non-test Rust files, logs findings, never panics
- [ ] `test_scan_known_stub_file` scans fixture with known patterns at expected lines
- [ ] `test_scan_multi_agent_agent_rs` verifies suppression on legitimate stubs
- [ ] Findings count documented (gate: audit if > 10 in non-suppressed production code)

### Step 3 (#628): Compound Review Pre-check
- [ ] `CompoundReviewConfig` has `stub_scan_enabled: bool` (default true) and `allowed_stub_paths: Vec<String>`
- [ ] `AgentOrchestrator` holds `Arc<NegativeContributionScanner>` initialised at startup
- [ ] `run_static_precheck()` runs on changed files before agent spawn
- [ ] Static findings injected as pseudo-agent output `ReviewAgentOutput { agent: "edm-scanner" }`
- [ ] `deduplicate_findings()` runs on merged (static + LLM) findings
- [ ] `stub_scan_enabled = false` skips the pre-check entirely
- [ ] Tests: diff with `todo!()` produces finding, dedup prevents duplicates, disabled config skips

### Step 4 (#629): LSP Crate
- [ ] `crates/terraphim_lsp/` in workspace, excluded from default-members
- [ ] Binary gated behind `terraphim-lsp` feature flag
- [ ] `publishDiagnostics` wired to EDM scanner
- [ ] 250ms debounce coalesces rapid changes
- [ ] Line 0 -> LSP line 0 (not u32::MAX)
- [ ] Graceful fallback when `~/.config/terraphim/` absent
- [ ] Tests: line zero, severity mapping, debounce, stub detection, test file exclusion

## 3. High-Level Design

### Step 2: Integration Tests
```
tests/integration_test.rs
    -> walkdir iterates crates/ and terraphim_server/
    -> scanner.scan_file() on each .rs file
    -> log findings, count by file
    -> no assertion on count (documentation test)

tests/fixtures/stub_sample.rs
    -> known todo!() at line 3
    -> known unimplemented!() at line 7
    -> suppression at line 11
```

### Step 3: Compound Review Wiring
```
AgentOrchestrator::new()
    -> Arc::new(NegativeContributionScanner::new())
    -> stored in orchestrator struct

CompoundReviewWorkflow::run()
    -> get_changed_files() [L277]
    -> ** NEW: run_static_precheck() **
    -> spawn agents [L309]
    -> collect results
    -> merge static + LLM findings
    -> deduplicate_findings() [L377] -- already handles this
    -> CompoundReviewResult
```

The pre-check result is injected as a `ReviewAgentOutput` with `agent: "edm-scanner"`, so it flows through the same dedup and issue filing path as LLM findings.

### Step 4: LSP Crate
```
terraphim_lsp
    -> tower-lsp server
    -> on didChange: debounce 250ms -> scan -> publishDiagnostics
    -> finding_to_diagnostic(): line mapping, severity mapping
    -> config fallback: EDM-only if no terraphim config
```

## 4. File/Module Change Plan

### Step 2: New Files
| File | Action | Responsibility |
|------|--------|---------------|
| `crates/terraphim_negative_contribution/tests/integration_test.rs` | Create | Codebase scan, fixture scan, suppression test |
| `crates/terraphim_negative_contribution/tests/fixtures/stub_sample.rs` | Create | Known EDM patterns at known lines |

### Step 2: Modified Files
| File | Change |
|------|--------|
| `crates/terraphim_negative_contribution/Cargo.toml` | Add `walkdir` to dev-dependencies |

### Step 3: New Files
None.

### Step 3: Modified Files
| File | Change |
|------|--------|
| `crates/terraphim_orchestrator/Cargo.toml` | Add `terraphim_negative_contribution` dep |
| `crates/terraphim_orchestrator/src/compound.rs` | Add `run_static_precheck()`, call in `run()` before agent spawn |
| `crates/terraphim_orchestrator/src/config.rs` | Add `stub_scan_enabled`, `allowed_stub_paths` to `CompoundReviewConfig` |
| `crates/terraphim_orchestrator/src/lib.rs` | Add `scanner: Arc<NegativeContributionScanner>` to `AgentOrchestrator`, init in `new()` |

### Step 4: New Files
| File | Action | Responsibility |
|------|--------|---------------|
| `crates/terraphim_lsp/Cargo.toml` | Create | tower-lsp, terraphim_negative_contribution deps |
| `crates/terraphim_lsp/src/lib.rs` | Create | Crate root, re-exports |
| `crates/terraphim_lsp/src/server.rs` | Create | LSP server with didOpen/didChange handlers |
| `crates/terraphim_lsp/src/diagnostic.rs` | Create | finding_to_diagnostic, severity mapping |
| `crates/terraphim_lsp/src/config.rs` | Create | Graceful config loading |
| `crates/terraphim_lsp/src/bin/terraphim-lsp.rs` | Create | Binary entry point |
| `crates/terraphim_lsp/tests/integration_test.rs` | Create | LSP integration tests |

### Step 4: Modified Files
| File | Change |
|------|--------|
| `Cargo.toml` (root) | Add `terraphim_lsp` to excludes (like symphony) |

## 5. Step-by-Step Implementation Sequence

### Step 2: Integration Tests (~30 min)

**2.1**: Create `tests/fixtures/stub_sample.rs` with known patterns at known lines

**2.2**: Create `tests/integration_test.rs`:
- `test_scan_terraphim_ai_codebase`: walkdir over workspace, scan each .rs, log findings
- `test_scan_known_stub_file`: scan fixture, assert line numbers and counts
- `test_scan_multi_agent_agent_rs`: scan a known file, verify suppression works

**2.3**: Add `walkdir` dev-dep, run tests, document findings count

**Deployable**: `cargo test -p terraphim_negative_contribution` passes

### Step 3: Compound Review Wiring (~2 hours)

**3.1**: Add `terraphim_negative_contribution` dep to orchestrator Cargo.toml

**3.2**: Add `stub_scan_enabled: bool` (default true) and `allowed_stub_paths: Vec<String>` (default empty) to `CompoundReviewConfig` in `config.rs`

**3.3**: Add `scanner: Arc<NegativeContributionScanner>` to `AgentOrchestrator` struct, initialise in `new()`

**3.4**: Add `run_static_precheck()` method to `CompoundReviewWorkflow`:
- Takes changed files list + scanner ref
- For each changed .rs file: read content, call `scanner.scan_file()`
- Skip files in `allowed_stub_paths`
- Return `ReviewAgentOutput { agent: "edm-scanner", findings, summary, pass }`

**3.5**: Modify `CompoundReviewWorkflow::run()`:
- After `get_changed_files()` (L277), before agent spawn (L309)
- If `stub_scan_enabled`: call `run_static_precheck()`
- Push static output to `agent_outputs` vec before the existing flatten/dedup

**3.6**: Tests: pre-check on diff with stub, dedup verification, disabled config

**Deployable**: `cargo test -p terraphim_orchestrator` passes

### Step 4: LSP Crate (~4 hours)

**4.1**: Create `crates/terraphim_lsp/Cargo.toml` with tower-lsp, terraphim_negative_contribution deps, feature flag

**4.2**: Add to root Cargo.toml excludes

**4.3**: Create `diagnostic.rs`: `finding_to_diagnostic()` with line 0 fix, severity mapping

**4.4**: Create `config.rs`: graceful config loading with fallback

**4.5**: Create `server.rs`: tower-lsp backend with didOpen/didChange, debounce, publishDiagnostics

**4.6**: Create `bin/terraphim-lsp.rs`: entry point

**4.7**: Integration tests

**Deployable**: `cargo build -p terraphim_lsp --features terraphim-lsp` succeeds

## 6. Testing Strategy

| Criteria | Type | Location |
|----------|------|----------|
| Codebase scan completes without panic | Integration | `tests/integration_test.rs` |
| Known stub at expected line | Integration | `tests/integration_test.rs` |
| Suppression works on real file | Integration | `tests/integration_test.rs` |
| Pre-check produces finding on diff | Unit | `compound.rs` tests |
| Dedup merges static + LLM | Unit | `compound.rs` tests |
| `stub_scan_enabled = false` skips | Unit | `compound.rs` tests |
| LSP line 0 -> 0 | Unit | `diagnostic.rs` |
| LSP severity mapping | Unit | `diagnostic.rs` |
| LSP debounce coalesces | Integration | `tests/integration_test.rs` |
| LSP stub emits diagnostic | Integration | `tests/integration_test.rs` |
| LSP test file no diagnostic | Integration | `tests/integration_test.rs` |

## 7. Risk Review

| Risk | Mitigation | Residual |
|------|------------|----------|
| > 10 EDM findings in codebase | Audit each; add suppression or `allowed_stub_paths` | Low -- known deferred areas |
| Orchestrator test requires git worktree | Use mock/in-memory approach | Medium |
| LSP crate adds build time | Excluded from default-members | None |
| tower-lsp version compatibility | Use latest stable 0.20.x | Low |

## 8. Open Questions

1. **Pre-check injection**: Inject as pseudo-agent output (recommended) or merge directly?
2. **Step 4 timing**: LSP crate is 4-6 hours. Same session or separate?
3. **Config default**: `stub_scan_enabled` default true (recommended) or false?
