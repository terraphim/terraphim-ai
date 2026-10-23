# Design & Implementation Plan: Terraphim Knowledge Graph Workflows

## 1. Summary of Target Behavior

After implementation, Terraphim will provide a complete **local-first knowledge graph validation pipeline** for AI coding workflows:

### Pre-LLM Validation
- Before sending queries to LLMs, validate that input terms are semantically connected
- Suggest fuzzy alternatives when exact terms aren't found
- Apply role-specific knowledge graphs for domain validation

### Post-LLM Validation
- Verify LLM outputs against domain checklists stored in knowledge graph
- Extract relevant concepts and validate terminology compliance
- Flag outputs that use non-standard terms

### Smart Commit Integration
- Extract concepts from changed files for commit message enrichment
- Validate commit messages against project knowledge graph

### Unified CLI & Hook Interface
- All features accessible via `terraphim-agent` subcommands
- Role selection via `--role` flag across all commands
- Single hook entry point for Claude Code integration

---

## 2. Key Invariants and Acceptance Criteria

### Invariants (Must Always Hold)

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| I1 | Hooks complete in <200ms for typical inputs | Timeout + early exit |
| I2 | All validation is local-first (no network required) | Use only local KG files |
| I3 | Existing hooks continue to work unchanged | Backward-compatible CLI |
| I4 | Role graphs are loaded lazily | Only load when role is accessed |
| I5 | Connectivity check limits to ≤10 matched terms | Hard limit with warning |

### Acceptance Criteria

| ID | Criterion | Test Type |
|----|-----------|-----------|
| AC1 | `terraphim-agent validate --connectivity "text"` returns true/false with matched terms | Unit |
| AC2 | `terraphim-agent suggest --fuzzy "typo"` returns top 5 suggestions with similarity scores | Unit |
| AC3 | `terraphim-agent replace --role "X" "text"` uses role X's thesaurus | Integration |
| AC4 | `terraphim-agent extract --paragraphs "text"` returns matched term + paragraph pairs | Unit |
| AC5 | `terraphim-agent validate --checklist "output"` validates against domain checklist | Integration |
| AC6 | Pre-LLM hook enriches context with KG concepts before LLM call | E2E |
| AC7 | Post-LLM hook validates output and adds warnings if terms not in KG | E2E |
| AC8 | MCP `is_all_terms_connected_by_path` calls real RoleGraph implementation | Integration |
| AC9 | Smart commit extracts concepts from git diff and suggests commit message elements | E2E |

---

## 3. High-Level Design and Boundaries

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Skills Layer (New)                                 │
│  ┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐       │
│  │ pre-llm-validate  │  │ post-llm-check    │  │ smart-commit      │       │
│  │ (skill file)      │  │ (skill file)      │  │ (skill file)      │       │
│  └─────────┬─────────┘  └─────────┬─────────┘  └─────────┬─────────┘       │
└────────────┼────────────────────────┼────────────────────────┼──────────────┘
             │                        │                        │
             ▼                        ▼                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Hooks Layer (Updated)                              │
│  ┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐       │
│  │ pre-tool-use.sh   │  │ post-tool-use.sh  │  │ prepare-commit.sh │       │
│  │ (calls agent)     │  │ (calls agent)     │  │ (calls agent)     │       │
│  └─────────┬─────────┘  └─────────┬─────────┘  └─────────┬─────────┘       │
└────────────┼────────────────────────┼────────────────────────┼──────────────┘
             │                        │                        │
             ▼                        ▼                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      terraphim-agent CLI (Extended)                          │
│                                                                              │
│  Existing Commands:        New Commands:                                     │
│  ┌──────────┐             ┌──────────────┐  ┌──────────────┐               │
│  │ replace  │ ──extend──▶ │ validate     │  │ suggest      │               │
│  │ extract  │             │ --connectivity│  │ --fuzzy      │               │
│  │ search   │             │ --checklist  │  │ --threshold  │               │
│  └──────────┘             │ --role       │  │ --role       │               │
│                           └──────────────┘  └──────────────┘               │
│                                                                              │
│  New Subcommand:          Extended:                                          │
│  ┌──────────────┐        ┌──────────────┐                                   │
│  │ hook         │        │ replace      │                                   │
│  │ --type X     │        │ --role X     │                                   │
│  │ --input JSON │        │ --suggest    │                                   │
│  └──────────────┘        └──────────────┘                                   │
└─────────────────────────────────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      terraphim_tui/src/ (CLI Implementation)                 │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     commands/ (New Module)                           │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │   │
│  │  │ validate.rs  │  │ suggest.rs   │  │ hook.rs      │              │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Core Crates (Minimal Changes)                        │
│                                                                              │
│  terraphim_mcp_server:     terraphim_rolegraph:    terraphim_automata:      │
│  ┌──────────────────┐     ┌──────────────────┐    ┌──────────────────┐     │
│  │ Fix connectivity │     │ No changes       │    │ No changes       │     │
│  │ placeholder      │     │ (already works)  │    │ (already works)  │     │
│  └──────────────────┘     └──────────────────┘    └──────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Component Boundaries

| Component | Responsibility | Changes |
|-----------|---------------|---------|
| Skills (`~/.claude/plugins/`) | Workflow orchestration, user-facing patterns | New skill files |
| Hooks (`.claude/hooks/`) | Claude Code integration, JSON I/O | Extend existing, add new |
| terraphim-agent CLI | Feature exposure, argument parsing | New subcommands |
| terraphim_tui | CLI implementation | New command modules |
| terraphim_mcp_server | MCP tool exposure | Fix connectivity placeholder |
| terraphim_rolegraph | Core graph operations | No changes needed |
| terraphim_automata | Core text matching | No changes needed |

---

## 4. File/Module-Level Change Plan

### 4.1 MCP Connectivity Fix

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_mcp_server/src/lib.rs` | Modify | Placeholder returns matched terms only | Calls `RoleGraph::is_all_terms_connected_by_path` | terraphim_rolegraph |
| `crates/terraphim_mcp_server/tests/test_advanced_automata_functions.rs` | Modify | Tests expect placeholder behavior | Tests verify actual connectivity result | None |

### 4.2 CLI Commands

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_tui/src/commands/mod.rs` | Create | - | Module exports for new commands | None |
| `crates/terraphim_tui/src/commands/validate.rs` | Create | - | `validate` subcommand with `--connectivity`, `--checklist`, `--role` | terraphim_rolegraph |
| `crates/terraphim_tui/src/commands/suggest.rs` | Create | - | `suggest` subcommand with `--fuzzy`, `--threshold`, `--role` | terraphim_automata |
| `crates/terraphim_tui/src/commands/hook.rs` | Create | - | `hook` subcommand with `--type`, `--input` for unified hook handling | All core crates |
| `crates/terraphim_tui/src/main.rs` | Modify | Current subcommands | Add new subcommand routing | commands module |
| `crates/terraphim_tui/src/replace.rs` | Modify | No `--role` flag | Add `--role` and `--suggest` flags | terraphim_config |

### 4.3 Skills

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `skills/pre-llm-validate/skill.md` | Create | - | Pre-LLM validation workflow skill | terraphim-agent CLI |
| `skills/post-llm-check/skill.md` | Create | - | Post-LLM checklist validation skill | terraphim-agent CLI |
| `skills/smart-commit/skill.md` | Create | - | Commit message enrichment skill | terraphim-agent CLI |

### 4.4 Hooks

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `.claude/hooks/pre-llm-validate.sh` | Create | - | Calls `terraphim-agent validate --connectivity` | terraphim-agent |
| `.claude/hooks/post-llm-check.sh` | Create | - | Calls `terraphim-agent validate --checklist` | terraphim-agent |
| `.claude/hooks/prepare-commit-msg` | Modify | Basic replacement | Add concept extraction via `terraphim-agent extract` | terraphim-agent |
| `.claude/hooks/npm_to_bun_guard.sh` | Modify | Hardcoded role | Use `--role` from env or config | terraphim-agent |

### 4.5 Knowledge Graph Extensions

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `docs/src/kg/checklists/` | Create | - | Directory for domain checklists | None |
| `docs/src/kg/checklists/code_review.md` | Create | - | Code review checklist as KG | None |
| `docs/src/kg/checklists/security.md` | Create | - | Security validation checklist as KG | None |

---

## 5. Step-by-Step Implementation Sequence

### Phase A: Foundation (Fix MCP, Add CLI Infrastructure)

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| A1 | Fix MCP connectivity placeholder to call real RoleGraph | Yes | Critical blocker |
| A2 | Update MCP tests to verify actual connectivity | Yes | Validates A1 |
| A3 | Create `commands/` module structure in terraphim_tui | Yes | Infrastructure |
| A4 | Add `--role` flag to existing `replace` command | Yes | Backward compatible |

### Phase B: New CLI Commands

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| B1 | Implement `validate --connectivity` command | Yes | Core feature |
| B2 | Implement `suggest --fuzzy` command | Yes | Core feature |
| B3 | Implement `validate --checklist` command | Yes | Requires B1 |
| B4 | Implement `hook` unified handler command | Yes | Simplifies hooks |
| B5 | Add unit tests for all new commands | Yes | Quality gate |

### Phase C: Skills & Hooks

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| C1 | Create pre-llm-validate skill | Yes | Uses B1 |
| C2 | Create pre-llm-validate.sh hook | Yes | Integrates C1 |
| C3 | Create post-llm-check skill | Yes | Uses B3 |
| C4 | Create post-llm-check.sh hook | Yes | Integrates C3 |
| C5 | Update prepare-commit-msg with concept extraction | Yes | Uses existing extract |
| C6 | Create smart-commit skill | Yes | Orchestrates C5 |

### Phase D: Knowledge Graph Extensions

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| D1 | Create checklists/ directory structure | Yes | Infrastructure |
| D2 | Create code_review checklist KG | Yes | Example checklist |
| D3 | Create security checklist KG | Yes | Example checklist |
| D4 | Document checklist format in docs | Yes | User guidance |

### Phase E: Integration & Documentation

| Step | Purpose | Deployable? | Notes |
|------|---------|-------------|-------|
| E1 | Update CLAUDE.md with new skills/hooks | Yes | Discovery |
| E2 | Create integration tests for full workflows | Yes | E2E validation |
| E3 | Update install-terraphim-hooks.sh | Yes | Easy onboarding |
| E4 | Performance benchmark hooks | Yes | Validate I1 invariant |

---

## 6. Testing & Verification Strategy

### Unit Tests

| Acceptance Criterion | Test Location | Description |
|---------------------|---------------|-------------|
| AC1 (validate --connectivity) | `terraphim_tui/tests/validate_test.rs` | Test connected/disconnected text cases |
| AC2 (suggest --fuzzy) | `terraphim_tui/tests/suggest_test.rs` | Test typo suggestions, threshold variations |
| AC3 (replace --role) | `terraphim_tui/tests/replace_test.rs` | Test role-specific thesaurus selection |
| AC4 (extract --paragraphs) | Existing tests | Already covered in terraphim_automata |

### Integration Tests

| Acceptance Criterion | Test Location | Description |
|---------------------|---------------|-------------|
| AC5 (validate --checklist) | `terraphim_tui/tests/checklist_test.rs` | Test against sample checklists |
| AC8 (MCP connectivity) | `terraphim_mcp_server/tests/` | Update existing tests |

### E2E Tests

| Acceptance Criterion | Test Location | Description |
|---------------------|---------------|-------------|
| AC6 (pre-LLM hook) | `tests/e2e/pre_llm_hook_test.sh` | Full hook invocation with sample input |
| AC7 (post-LLM hook) | `tests/e2e/post_llm_hook_test.sh` | Full hook invocation with LLM output |
| AC9 (smart commit) | `tests/e2e/smart_commit_test.sh` | Git diff to enriched commit message |

### Performance Tests

| Invariant | Test | Threshold |
|-----------|------|-----------|
| I1 (hook latency) | `benches/hook_latency.rs` | <200ms p99 |
| I5 (term limit) | `tests/validate_term_limit_test.rs` | Warning at >10 terms |

---

## 7. Risk & Complexity Review

| Risk (from Phase 1) | Mitigation in Design | Residual Risk |
|---------------------|---------------------|---------------|
| Connectivity check too slow | Hard limit of 10 terms (I5), timeout in hook | Low - bounded complexity |
| MCP fix breaks existing tests | Step A2 updates tests alongside fix | Low - tested together |
| Role loading increases startup | Lazy loading (I4) in CLI commands | Low - on-demand only |
| Paragraph extraction misses code | Out of scope for v1, document limitation | Medium - future enhancement |
| Pre-LLM validation too strict | Skills use advisory mode (warnings, not blocking) | Low - user control |
| Hook complexity confuses users | Unified `hook` command, clear docs | Low - simplified interface |

### Complexity Assessment

| Component | Complexity | Justification |
|-----------|------------|---------------|
| MCP fix | Low | Single function replacement |
| CLI commands | Medium | New module structure, argument parsing |
| Skills | Low | Markdown files with workflow docs |
| Hooks | Low | Shell scripts calling CLI |
| Checklists | Low | Markdown KG files |

**Total estimated complexity**: Medium (mostly additive, minimal changes to core crates)

---

## 8. Open Questions / Decisions for Human Review

### Design Decisions Needed

1. **Pre-LLM mode**: Should validation be **advisory** (add warnings) or **blocking** (reject)?
   - *Recommendation*: Advisory by default, blocking opt-in via `--strict` flag

2. **Role detection**: How should hooks determine which role to use?
   - *Recommendation*: Priority order: `--role` flag > `TERRAPHIM_ROLE` env > project config > default

3. **Checklist format**: Should checklists use existing KG synonyms format or new `checklist::` directive?
   - *Recommendation*: New `checklist::` directive for explicit semantics

4. **Hook timeout**: What's the acceptable timeout for hook operations?
   - *Recommendation*: 200ms default, configurable via `TERRAPHIM_HOOK_TIMEOUT`

### Scope Confirmation

5. **Smart commit scope**: Should commit enrichment be automatic or skill-invoked?
   - *Recommendation*: Skill-invoked initially, automatic as optional future enhancement

6. **Existing skill updates**: Update terraphim-hooks skill or create separate skills?
   - *Recommendation*: Create separate focused skills, update terraphim-hooks to reference them

---

## Appendix: Proposed CLI Interface

```bash
# Validate semantic connectivity
terraphim-agent validate --connectivity "system operator trained for life cycle"
# Output: { "connected": true, "terms": [...], "path_exists": true }

terraphim-agent validate --connectivity --role "Security" "authentication protocol"
# Output: Uses Security role's knowledge graph

# Validate against checklist
terraphim-agent validate --checklist code_review "implemented feature with tests"
# Output: { "passed": ["has_tests"], "missing": ["security_check", "docs"] }

# Fuzzy suggestions
terraphim-agent suggest --fuzzy "terraphm" --threshold 0.7
# Output: [{ "term": "terraphim", "similarity": 0.92 }, ...]

# Role-aware replacement
terraphim-agent replace --role "DevOps" "run npm install"
# Output: "run bun install" (using DevOps role's thesaurus)

# Unified hook handler
terraphim-agent hook --type pre-tool-use --input '{"command": "npm test"}'
# Output: Processed JSON for Claude Code

# Extract concepts for commit
terraphim-agent extract --paragraphs --from-diff HEAD~1
# Output: Matched concepts from changed files
```

---

**Do you approve this plan as-is, or would you like to adjust any part?**
