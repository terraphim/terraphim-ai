# Design & Implementation Plan: OpenCode + Terraphim Flow Comparison Experiment

## 1. Summary of Target Behavior

**Experiment Goal**: Compare two OpenCode flows for frontend development assistance:
- **Flow A (With Terraphim)**: OpenCode uses `terraphim_mcp_server` with Frontend Developer role, dual haystacks (Ripgrep + GrepApp), and knowledge graph concept matching
- **Flow B (Without Terraphim)**: OpenCode uses default search without terraphim integration

**Task**: Add an accessible navigation component to a Svelte project (responsive, ARIA, keyboard navigation)

**Success Metrics**:
- **Relevance**: Quality of suggestions, code patterns, accessibility compliance
- **Efficiency**: Time to solution, number of tool calls, iteration count

## 2. Key Invariants and Acceptance Criteria

### Invariants (Must Hold for Both Flows)
- Same Svelte project target (same codebase, same file locations)
- Identical initial prompt to OpenCode
- Both flows produce working, compilable code
- Results captured for comparison analysis

### Acceptance Criteria

| Criterion | Test Type | Verification |
|-----------|-----------|--------------|
| Flow A: terraphim_mcp_server builds successfully | Build | `cargo build -p terraphim_mcp_server` succeeds |
| Flow A: Frontend Developer role configured | Config | `~/.config/terraphim/embedded_config.json` has role |
| Flow A: GrepApp haystack responds | Integration | `terraphim-agent search --role "Frontend Developer" --limit 3 "aria"` returns results |
| Flow A: OpenCode MCP tools available | Tool discovery | `terraphim__search` tool visible in OpenCode |
| Flow B: No terraphim MCP tools used | Control | Confirmed via session logs |
| Both: Identical task prompt | Control | Same prompt string |
| Both: Code compiles | Build | `cd desktop && yarn build` succeeds |
| Both: Navigation is accessible | Audit | WCAG 2.1 AA checklist passed |

## 3. High-Level Design and Boundaries

### Architecture: Flow A (With Terraphim)

```
OpenCode Session
      │
      ▼
┌─────────────────────────────────────────────────────────────┐
│  terraphim_mcp_server (MCP tools)                         │
│  ├── mcp__terraphim__search     (KG-powered search)        │
│  ├── mcp__terraphim__autocomplete_terms                   │
│  ├── mcp__terraphim__find_files                           │
│  └── mcp__terraphim__terraphim_grep                      │
└─────────────────────────────────────────────────────────────┘
      │
      ▼
┌─────────────────────────────────────────────────────────────┐
│  Frontend Developer Role                                    │
│  ├── KG: ~/.config/terraphim/kg/frontend/ (18 concepts)    │
│  ├── Haystack 1: Ripgrep (local project files)             │
│  └── Haystack 2: GrepApp (GitHub TypeScript repos)        │
└─────────────────────────────────────────────────────────────┘
      │
      ▼
  Ranked Results (KG concept boost + TF-IDF)
```

### Architecture: Flow B (Without Terraphim)

```
OpenCode Session
      │
      ▼
  Default OpenCode search (no terraphim tools)
      │
      ▼
  Standard code suggestions (no KG, no haystack)
```

### Boundary: What Changes

| Component | Flow A | Flow B |
|-----------|--------|--------|
| MCP server | terraphim_mcp_server running | Not used |
| Knowledge Graph | Frontend KG with 18 concepts | Not used |
| Haystack search | Ripgrep + GrepApp | Not used |
| Tool calls | mcp__terraphim__* | Default only |

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `~/.config/terraphim/embedded_config.json` | Modify | AI Engineer role only | Add Frontend Developer role with dual haystacks | None |
| `~/.config/terraphim/kg/frontend/` | Create | Does not exist | 18 frontend concept .md files | terraphim-ai repo `data/kg/frontend/` |
| `terraphim_mcp_server` | Build | Not built | Binary at `target/release/` | `cargo build -p terraphim_mcp_server --features grepapp` |
| `~/.config/opencode/opencode.json` | Modify | gitea-robot MCP only | Add terraphim MCP server entry | terraphim_mcp_server binary |
| `desktop/src/lib/Navigation.svelte` | Create | No nav component | Accessible nav with ARIA | Svelte project |

## 5. Step-by-Step Implementation Sequence

### Phase 1: Setup Terraphim Infrastructure (Flow A prerequisites)

1. **Create Frontend Developer Role Config**
   - Add "Frontend Developer" role to `~/.config/terraphim/embedded_config.json`
   - Configure dual haystacks: Ripgrep (local) + GrepApp (GitHub, TypeScript)
   - Set relevance_function: `terraphim-graph`
   - Deployable state: Role exists but KG empty until step 2

2. **Create Frontend Knowledge Graph**
   - Create directory `~/.config/terraphim/kg/frontend/`
   - Copy 18 concept files from `data/kg/frontend/` in terraphim-ai repo
   - Concepts: accessibility, svelte-patterns, css-layout, typescript, etc.
   - Deployable state: KG files exist, role can load them

3. **Build terraphim_mcp_server with GrepApp**
   - Run: `cargo build --release -p terraphim_mcp_server --features grepapp`
   - Copy: `cp target/release/terraphim_mcp_server ~/.cargo/bin/`
   - Deployable state: Binary exists and responds to MCP handshake

4. **Register MCP in OpenCode**
   - Add terraphim entry to `~/.config/opencode/opencode.json`
   - Restart OpenCode
   - Verify tools appear: `terraphim__search`, `terraphim__autocomplete_terms`, etc.
   - Deployable state: OpenCode can call MCP tools

### Phase 2: Prepare Svelte Project

5. **Create/Identify Test Project**
   - Use existing `desktop/` directory in terraphim-ai
   - Or create minimal SvelteKit project for testing
   - Ensure `yarn install` and `yarn build` work
   - Deployable state: Clean project ready for nav component

### Phase 3: Execute Flow A (With Terraphim)

6. **OpenCode Session - Flow A**
   - Launch OpenCode in project directory
   - Prompt: "Add an accessible navigation component to this Svelte project. Requirements: responsive design, ARIA labels, keyboard navigation (Tab, Escape), semantic HTML nav element, mobile hamburger menu"
   - OpenCode uses `terraphim__search` to find Svelte patterns, accessibility best practices
   - OpenCode uses `terraphim__autocomplete_terms` for terminology
   - Capture: All tool calls, responses, final code

7. **Verify Flow A Output**
   - Check code compiles: `cd desktop && yarn build`
   - Audit accessibility: Manual WCAG checklist
   - Log metrics: Time, tool calls count, iterations

### Phase 4: Execute Flow B (Without Terraphim)

8. **Reset Environment**
   - Disable/unregister terraphim MCP in OpenCode (comment out in opencode.json)
   - Restart OpenCode
   - Clear any session context

9. **OpenCode Session - Flow B**
   - Same project directory (git stash any Flow A changes first)
   - Same prompt: "Add an accessible navigation component..."
   - OpenCode uses default search only (no terraphim tools)
   - Capture: All tool calls, responses, final code

10. **Verify Flow B Output**
    - Check code compiles
    - Audit accessibility
    - Log same metrics as Flow A

### Phase 5: Analysis

11. **Compare Results**
    - Side-by-side code quality review
    - Metric comparison: time, tool calls, iterations
    - WCAG compliance check
    - KG concept utilization (Flow A only)

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Location |
|--------------------|-----------|----------|
| terraphim_mcp_server builds | Build verification | CI or manual |
| Frontend role configured | Config check | `grep "Frontend Developer" ~/.config/terraphim/embedded_config.json` |
| GrepApp responds | Integration test | `terraphim-agent search --role "Frontend Developer" --limit 3 "flexbox"` |
| OpenCode MCP tools visible | Tool discovery | OpenCode `/tools` or equivalent |
| Both flows compile | Build test | `cd desktop && yarn build` |
| WCAG compliance | Manual audit | Accessibility checklist |
| Relevance comparison | Expert review | Code review against requirements |
| Efficiency comparison | Metrics | Time, tool calls, iterations |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|--------------|
| GrepApp rate limiting | Monitor stderr, fallback to Ripgrep-only | Low - graceful degradation |
| MCP tools not auto-invoked | Explicit prompts to use terraphim tools | Medium - requires prompt engineering |
| Flow A/B contamination | Git stash between flows, clean session | Low - disciplined reset |
| KG concepts missing | Pre-populate with existing 18 concepts | Low - can extend mid-experiment |
| Build complexity | Use existing documented steps | Low - proven walkthrough exists |

## 8. Open Questions / Decisions for Human Review

1. **Svelte project location**: Use existing `desktop/` or create fresh project?
2. **KG source**: Copy from `data/kg/frontend/` or create custom?
3. **Session capture**: Save OpenCode session logs for later analysis?
4. **Expert reviewer**: Who validates WCAG compliance for both outputs?
5. **Iteration limit**: Max iterations per flow to prevent endless loops?
6. **Success threshold**: What improvement margin defines "terraphim helps"?

---

## Appendix: Frontend Developer Role Configuration

```json
{
  "Frontend Developer": {
    "shortname": "frontend-dev",
    "name": "Frontend Developer",
    "relevance_function": "terraphim-graph",
    "terraphim_it": true,
    "theme": "yeti",
    "kg": {
      "automata_path": null,
      "knowledge_graph_local": {
        "input_type": "markdown",
        "path": "~/.config/terraphim/kg/frontend"
      },
      "public": false,
      "publish": false
    },
    "haystacks": [
      {
        "location": "~/projects/terraphim/terraphim-ai/desktop",
        "service": "Ripgrep",
        "read_only": true,
        "fetch_content": false,
        "extra_parameters": {}
      },
      {
        "location": "https://grep.app",
        "service": "GrepApp",
        "read_only": true,
        "fetch_content": false,
        "extra_parameters": {
          "language": "typescript"
        }
      }
    ],
    "llm_enabled": false
  }
}
```
