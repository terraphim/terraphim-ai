# Research Document: Underutilized Terraphim Features for Pre/Post-LLM Knowledge Graph Workflows

## 1. Problem Restatement and Scope

### Problem Statement
Terraphim has powerful knowledge graph capabilities that are currently underutilized. Four specific features could be leveraged to create a local-first workflow that:
1. **Pre-LLM**: Validates and enriches context before sending to LLMs
2. **Post-LLM**: Validates domain model compliance in LLM outputs

### IN Scope
- Graph connectivity (`is_all_terms_connected_by_path`) for semantic coherence validation
- Fuzzy autocomplete for suggesting alternatives when no exact match exists
- Role-based replacement with different thesauruses per role
- Paragraph extraction for smarter commit message handling
- New/updated skills and hooks leveraging these capabilities
- Local-first knowledge graph validation workflows

### OUT of Scope
- Changes to core automata algorithms (already optimized)
- New LLM integrations (use existing OpenRouter/Ollama)
- Remote/cloud knowledge graph storage
- UI/frontend changes

---

## 2. User & Business Outcomes

### For AI Coding Agents (Primary User)
| Outcome | Benefit |
|---------|---------|
| Pre-LLM semantic validation | Catch nonsensical queries before wasting LLM tokens |
| Post-LLM domain checklist | Verify outputs use correct terminology |
| Fuzzy term suggestions | Recover from typos/near-matches gracefully |
| Role-aware context | Different domains get appropriate knowledge graphs |

### For Developers Using Terraphim
| Outcome | Benefit |
|---------|---------|
| Smarter commit messages | Auto-extract relevant concepts from changed files |
| Hook-based validation | Prevent commits that violate domain model |
| Skill-based workflows | Reusable patterns for pre/post-LLM validation |

### Business Value
- Reduced LLM API costs (filter bad queries)
- Higher quality AI outputs (domain-validated)
- Better knowledge retention (local-first graphs)
- Improved developer experience (intelligent suggestions)

---

## 3. System Elements and Dependencies

### Current Feature Implementations

#### 3.1 Graph Connectivity
| Element | Location | Status |
|---------|----------|--------|
| Core algorithm | `terraphim_rolegraph/src/lib.rs:204-277` | ✅ Complete |
| MCP tool wrapper | `terraphim_mcp_server/src/lib.rs:1027-1138` | ⚠️ Placeholder (doesn't call real implementation) |
| Unit tests | `terraphim_rolegraph/src/lib.rs:1226-1246` | ⚠️ 1 ignored test |
| Integration tests | `terraphim_mcp_server/tests/test_advanced_automata_functions.rs` | ✅ Multiple scenarios |
| Benchmarks | `terraphim_rolegraph/benches/throughput.rs:190-196` | ✅ Available |
| CLI exposure | None | ❌ Missing |

**Algorithm**: DFS backtracking to find if single path connects all matched terms. O(n!) worst case but optimized for ≤8 nodes with fast-fail isolation check.

#### 3.2 Fuzzy Autocomplete
| Element | Location | Status |
|---------|----------|--------|
| Jaro-Winkler (default) | `terraphim_automata/src/autocomplete.rs:328-412` | ✅ Complete |
| Levenshtein (baseline) | `terraphim_automata/src/autocomplete.rs:236-321` | ✅ Complete |
| MCP tools | `terraphim_mcp_server/src/lib.rs:471-620` | ✅ 4 tools exposed |
| CLI exposure | None | ❌ Missing |
| Hook integration | None | ❌ Missing |

**Performance**: Jaro-Winkler is 2.3x faster than Levenshtein with better prefix weighting.

#### 3.3 Role-Based Replacement
| Element | Location | Status |
|---------|----------|--------|
| Role configuration | `terraphim_config/src/lib.rs:175-249` | ✅ Complete |
| KnowledgeGraph per role | `terraphim_config/src/lib.rs:393-420` | ✅ Complete |
| RoleGraph loading | `terraphim_config/src/lib.rs:865-930` | ✅ Complete |
| PreToolUse hook | `.claude/hooks/npm_to_bun_guard.sh` | ✅ Single role only |
| Multi-role hook support | None | ❌ Missing |
| Role selection in replace | `terraphim-agent replace` | ⚠️ Uses default role only |

**Current Hook Flow**:
```
PreToolUse → detect npm/yarn/pnpm → terraphim-agent replace → KG lookup → modified command
```

#### 3.4 Paragraph Extraction
| Element | Location | Status |
|---------|----------|--------|
| Core function | `terraphim_automata/src/matcher.rs:101-125` | ✅ Complete |
| find_paragraph_end | `terraphim_automata/src/matcher.rs:130-148` | ✅ Complete |
| MCP tool | `terraphim_mcp_server/src/lib.rs:843-911` | ✅ Complete |
| CLI exposure | None | ❌ Missing |
| Commit message integration | None | ❌ Missing |

### Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Skills & Hooks Layer                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ pre-llm-     │  │ post-llm-    │  │ smart-commit │              │
│  │ validation   │  │ checklist    │  │ hook         │              │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘              │
└─────────┼─────────────────┼─────────────────┼───────────────────────┘
          │                 │                 │
          ▼                 ▼                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      terraphim-agent CLI                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ replace      │  │ validate     │  │ extract      │              │
│  │ --role X     │  │ --checklist  │  │ --paragraphs │              │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘              │
└─────────┼─────────────────┼─────────────────┼───────────────────────┘
          │                 │                 │
          ▼                 ▼                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Core Crate Layer                                  │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   terraphim_service                          │   │
│  │  - orchestrates config, rolegraph, automata                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│           │                    │                    │               │
│           ▼                    ▼                    ▼               │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐       │
│  │terraphim_config│  │terraphim_role- │  │ terraphim_     │       │
│  │  - Role struct │  │graph           │  │ automata       │       │
│  │  - KG loading  │  │  - connectivity│  │  - fuzzy AC    │       │
│  │                │  │  - query_graph │  │  - paragraph   │       │
│  └────────────────┘  └────────────────┘  └────────────────┘       │
└─────────────────────────────────────────────────────────────────────┘
```

### Cross-Cutting Concerns
- **Thesaurus format**: JSON with `{id, nterm, url}` structure
- **Aho-Corasick**: LeftmostLongest matching (longer patterns win)
- **Role resolution**: Case-insensitive via RoleName struct
- **Async boundaries**: RoleGraph behind `Arc<Mutex<>>` (RoleGraphSync)

---

## 4. Constraints and Their Implications

### Technical Constraints

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| Graph connectivity O(n!) | Exponential for >8 matched terms | Must limit term count or use heuristics |
| Hooks are shell scripts | Must pipe through terraphim-agent | Need CLI commands for all features |
| MCP placeholder for connectivity | Current MCP tool doesn't call real impl | Must fix before MCP-based workflows |
| Role loading at startup | ConfigState builds all RoleGraphs | Heavy startup if many roles with large KGs |
| WASM compatibility | terraphim_automata targets wasm32 | Cannot use filesystem in WASM builds |

### Business/UX Constraints

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| Local-first requirement | Privacy, offline capability | Cannot require network for validation |
| Sub-second latency | Hooks must not slow down coding | Optimize hot paths, cache aggressively |
| Backward compatibility | Existing hooks/skills must work | Additive changes only |

### Security Constraints

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| Hooks run arbitrary commands | Could be exploited if input not sanitized | Validate all hook inputs |
| Knowledge graphs contain URLs | Could leak sensitive paths | Sanitize KG content |

---

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS

1. **U1**: What is the typical matched term count in real queries?
   - Risk: If >8 terms common, connectivity check becomes slow
   - De-risk: Add telemetry to measure in production

2. **U2**: Which roles need different thesauruses?
   - Currently only "Terraphim Engineer" has KG
   - Need to understand user role patterns

3. **U3**: What paragraph boundaries work for code vs docs?
   - Current: blank lines only
   - Code uses different conventions (function boundaries, etc.)

4. **U4**: MCP placeholder - why wasn't real implementation connected?
   - Need to investigate technical blockers

### ASSUMPTIONS

1. **A1**: Users want pre-LLM validation to reduce costs *(needs validation)*
2. **A2**: Fuzzy autocomplete threshold of 0.6 is appropriate default *(based on tests)*
3. **A3**: Role-based replacement is more valuable than global replacement *(needs validation)*
4. **A4**: Commit messages benefit from concept extraction *(hypothesis)*
5. **A5**: Existing hook infrastructure can handle additional complexity *(likely true)*

### RISKS

#### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Connectivity check too slow | Medium | High | Add term count limit, timeout |
| MCP fix breaks existing tests | Low | Medium | Run full test suite before/after |
| Role loading increases startup time | Medium | Medium | Lazy loading, caching |
| Paragraph extraction misses code boundaries | High | Low | Add code-aware extraction mode |

#### Product/UX Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Pre-LLM validation too strict | Medium | High | Allow bypass, tunable thresholds |
| Fuzzy suggestions irrelevant | Medium | Medium | User feedback loop, adjust similarity |
| Hook complexity confuses users | Low | Medium | Clear documentation, examples |

#### Security Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Malicious KG injection | Low | High | Validate KG sources, sanitize |
| Hook command injection | Low | High | Input validation, sandboxing |

---

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Multiple thesaurus loading paths**
   - Remote URL (automata_path)
   - Local markdown (knowledge_graph_local)
   - Direct JSON
   - Each role can use different path

2. **Async/sync boundary in RoleGraph**
   - RoleGraphSync wraps in Arc<Mutex<>>
   - Can cause contention with many concurrent queries

3. **MCP vs CLI vs Direct Rust API**
   - Three ways to access same functionality
   - Inconsistent feature availability across interfaces

4. **Hook shell script complexity**
   - JSON parsing with jq
   - Agent discovery logic
   - Error handling scattered

### Simplification Opportunities

#### S1: Unified CLI Interface
Create consistent `terraphim-agent` subcommands that expose ALL features:
```bash
terraphim-agent validate --connectivity --role "Engineer"
terraphim-agent suggest --fuzzy --threshold 0.6
terraphim-agent replace --role "Engineer"
terraphim-agent extract --paragraphs --code-aware
```

#### S2: Single Hook Entry Point
Replace multiple shell scripts with single Rust-based hook handler:
```bash
terraphim-agent hook --type pre-tool-use --input "$JSON"
```
Benefits: Better error handling, type safety, testability

#### S3: Phased Validation Pipeline
Create composable validation stages:
```
Input → [Term Extraction] → [Connectivity Check] → [Fuzzy Fallback] → [Role Replacement] → Output
```
Each stage can be enabled/disabled, making workflows flexible.

#### S4: Checklist as Knowledge Graph
Model checklists as specialized KG entries:
```markdown
# code_review_checklist

Required validation steps for code review.

synonyms:: review checklist, pr checklist
checklist:: security_check, test_coverage, documentation
```

---

## 7. Questions for Human Reviewer

### Critical Questions

1. **Q1**: Should pre-LLM validation be blocking (reject query) or advisory (add warnings)?
   - Affects UX and implementation complexity

2. **Q2**: What's the acceptable latency budget for hook-based validation?
   - Current hooks are <100ms; adding connectivity check may exceed this

3. **Q3**: Should we fix the MCP connectivity placeholder before building skills on top?
   - Blocking for MCP-based workflows

### Design Questions

4. **Q4**: Should fuzzy suggestions be automatic (always try) or opt-in?
   - Trade-off: convenience vs. unexpected behavior

5. **Q5**: How should role selection work in hooks?
   - Options: config file, env var, auto-detect from project

6. **Q6**: What code boundary detection is needed for paragraph extraction?
   - Options: language-aware (complex) vs. heuristic (simpler)

### Validation Questions

7. **Q7**: Do you have specific use cases for post-LLM domain validation?
   - Need concrete examples to design checklist format

8. **Q8**: Which existing skills should be updated vs. creating new ones?
   - Affects scope and backward compatibility

---

## Appendix: Current Feature Usage Summary

| Feature | Core Impl | MCP | CLI | Hooks | Tests |
|---------|-----------|-----|-----|-------|-------|
| Graph Connectivity | ✅ | ⚠️ Placeholder | ❌ | ❌ | ✅ |
| Fuzzy Autocomplete | ✅ | ✅ | ❌ | ❌ | ✅ |
| Role-Based Replacement | ✅ | ✅ | ⚠️ Default only | ⚠️ Single role | ✅ |
| Paragraph Extraction | ✅ | ✅ | ❌ | ❌ | ✅ |

**Legend**: ✅ Complete | ⚠️ Partial | ❌ Missing
