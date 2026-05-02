# Research Document: Frontend Developer Role + OpenCode Terraphim Experiment

## 1. Problem Restatement and Scope

**Problem**: Assess whether terraphim-agent integration with OpenCode provides measurable benefits for frontend development tasks by comparing two flows:
- **Flow A (With Terraphim)**: OpenCode leverages terraphim-agent via haystack search, knowledge graph concept matching, and MCP tools
- **Flow B (Without Terraphim)**: OpenCode uses default capabilities without terraphim integration

**Research Question**: Does terraphim-agent + haystack integration improve the quality, relevance, or efficiency of frontend development assistance in OpenCode?

**IN SCOPE**:
- Frontend developer role configuration in terraphim-agent
- Haystack types: Ripgrep (local), GrepApp (GitHub code search)
- OpenCode MCP tool integration
- Knowledge graph concepts for frontend (TypeScript, Svelte, CSS, accessibility)
- Two-flow experiment execution and comparison

**OUT OF SCOPE**:
- Building new terraphim crates from source
- Claude Code comparison (only OpenCode)
- Personal Assistant role with JMAP
- Backend Rust development tasks

---

## 2. User & Business Outcomes

**User Outcomes**:
- Developer gets faster access to relevant frontend patterns via haystack search
- Knowledge graph concepts provide context-aware terminology expansion
- MCP tools enable structured tool-calling for search

**Business Outcomes**:
- Evidence-based assessment of terraphim ROI for frontend development
- Quantified improvement (if any) in task completion quality
- Documented best practices for OpenCode + terraphim integration

---

## 3. System Elements and Dependencies

| Component | Location | Role |
|-----------|----------|------|
| terraphim-agent | ~/.cargo/bin/ | CLI search, autocomplete, replace |
| terraphim_mcp_server | Not built | MCP tools (search, autocomplete) |
| embedded_config.json | ~/.config/terraphim/ | Role definitions |
| Knowledge Graph (KG) | ~/.config/terraphim/kg/ | Concept files (.md with synonyms) |
| Frontend KG | ~/.config/terraphim/kg/frontend/ | 18 FE concept files |
| OpenCode config | ~/.config/opencode/opencode.json | MCP server registration |
| Haystack: Ripgrep | Local filesystem | Project file search |
| Haystack: GrepApp | grep.app API | GitHub code search |
| terraphim-hooks | ~/.claude/hooks/ | PreToolUse/PostToolUse |
| local-knowledge skill | ~/.agents/skills/local-knowledge/ | Skill for KG search |

### Current Configuration State

**Already Configured**:
- terraphim-agent v1.17.0 installed
- AI Engineer role active with Ollama + terraphim-graph
- Knowledge graph at ~/.config/terraphim/kg/ (85+ concepts)
- Sessions tracking (10 sessions, 1694 messages)
- OpenCode plugins: learning-capture, safety-guard, dcg-guard, advisory-guard, fff
- OpenCode MCP: cached-context, gitea-robot

**NOT Configured**:
- Frontend Engineer role with dual haystacks (Ripgrep + GrepApp)
- terraphim_mcp_server built or registered
- Frontend-specific KG (only AI Engineer KG exists)
- GrepApp feature flag in built binary

---

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implications |
|------------|----------------|--------------|
| GrepApp requires `--features grepapp` at build time | Optional dependency not in crates.io binary | Must build from source if GrepApp needed |
| MCP server not built | Required for typed tool integration | Need `cargo build -p terraphim_mcp_server` |
| Current KG lacks frontend concepts | Frontend role needs domain-specific KG | Create frontend KG or use existing terraphim-ai docs |
| opencode MCP expects `terraphim__` namespace | Tool naming convention | Config must use double-underscore format |
| Session comparison requires same task | Fair A/B testing | Both flows must solve identical problem |

---

## 5. Risks, Unknowns, and Assumptions

### ASSUMPTIONS (clearly marked)
1. **A1**: The 18-concept frontend KG exists at `data/kg/frontend/` in terraphim-ai repo
2. **A2**: GrepApp haystack provides meaningful code search over alternatives
3. **A3**: OpenCode will actually leverage MCP tools when registered
4. **A4**: A single frontend task (e.g., "add responsive navigation") is representative

### UNKNOWNS
1. **U1**: Whether opencode auto-uses MCP tools or requires explicit invocation
2. **U2**: Whether GrepApp API requires authentication/rate limiting
3. **U3**: Actual latency difference between Flow A and Flow B
4. **U4**: Whether knowledge graph synonyms actually improve search relevance

### RISKS
| Risk | Impact | Mitigation |
|------|--------|------------|
| GrepApp rate limiting or API issues | Flow A degrades to local-only | Monitor stderr, have fallback |
| MCP tools not auto-invoked by OpenCode | Flow A doesn't use KG | Explicit prompt engineering |
| Frontend KG missing key concepts | Irrelevant search results | Extend KG before experiment |
| OpenAI/anthropic model ignores tool calls | Comparison invalid | Use explicit tool calls |

---

## 6. Context Complexity vs. Simplicity Opportunities

### Complexity Sources
1. **Multiple integration paths**: CLI (terraphim-agent search), MCP (mcp__terraphim__search), Hooks (pre_tool_use.sh)
2. **Dual haystack**: Local Ripgrep + remote GrepApp merging
3. **Role auto-routing**: Query scoring against multiple role KGs

### Simplification Strategies
1. **Single role experiment**: Create one "Frontend Developer" role, don't test multi-role routing
2. **CLI-first approach**: Use `/tsearch` command pattern instead of MCP (fewer moving parts)
3. **Identical task for both flows**: Same prompt to OpenCode, only difference is terraphim tool access

---

## 7. Questions for Human Reviewer

1. **Which frontend task should both flows solve?** (e.g., "add accessible navigation component", "implement dark mode toggle")
2. **Should we build terraphim_mcp_server from source**, or use CLI approach only?
3. **What metrics define "better"?** (relevance of results, time to solution, code correctness)
4. **Should we test with or without GrepApp?** (Build complexity vs. real-world value)
5. **Do you want the Frontend Engineer role to use `terraphim-graph` or `bm25plus`?**
6. **Should we create a new frontend KG or use existing docs as haystack?**
7. **How many test queries needed for meaningful comparison?**
8. **Should session messages be captured for later analysis?**
9. **Do you want me to commit results to the terraphim-ai repo?**
10. **What's the acceptable time investment for setup before we see results?**
