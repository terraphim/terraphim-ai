# Design: OpenCode + Terraphim Flow Comparison Experiment

## Executive Summary

Compare OpenCode frontend development assistance across three configurations:
- **Flow A**: Terraphim + Ripgrep haystack (local content search)
- **Flow B**: Terraphim + FFF haystack (fuzzy file find + KG-scored content grep)
- **Flow C**: Control (no Terraphim - default OpenCode search)

**Task**: Build an accessible navigation component in a Svelte project with iterative refinement.

**Metrics**: Relevance (WCAG compliance, pattern quality), Efficiency (time, iterations, tool calls).

---

## 1. Key Findings from Research

### FFF vs Ripgrep Haystacks

| Aspect | Ripgrep Haystack | FFF Haystack |
|--------|------------------|--------------|
| **Purpose** | Content search (what files contain X) | File finding (by path) + content search |
| **Matching** | Regex-based content search | Fuzzy path matching + Aho-Corasick content |
| **Scoring** | Standard TF-IDF | Frecency (frequency + recency) + KG path boost |
| **KG Integration** | Concept matching in content | Concept matching in file paths AND content |
| **Speed** | Fast for content | Fast for frequent files (frecency cache) |
| **MCP Tools** | Via terraphim_middleware | `terraphim_find_files`, `terraphim_grep`, `terraphim_multi_grep` |

**Conclusion**: FFF is NOT a drop-in Ripgrep replacement. FFF excels at file discovery by path with frecency, while Ripgrep excels at exhaustive content search. For the experiment, we test both as separate haystack configurations.

### Frontend KG Status

- **Source**: `data/kg/frontend/` in terraphim-ai repo (18 concept files)
- **Requires validation**: Each concept file must have valid markdown structure, synonyms, and no broken links
- **Existing documentation**: Blog post at `terraphim.ai/content/posts/frontend-developer-agent-walkthrough.md` + walkthrough at `docs/walkthroughs/frontend-developer-agent.md`

### Documentation Gap

The existing walkthrough is comprehensive but:
1. Assumes existing project (not fresh Svelte creation)
2. Does not compare fff vs ripgrep
3. Does not include iterative refinement scenario
4. Does not validate KG records before use

---

## 2. Experiment Design

### Task: Accessible Navigation Component

**Requirements** (iterative, realistic scenario):
1. Responsive design (mobile hamburger menu)
2. ARIA labels and roles
3. Keyboard navigation (Tab, Escape to close)
4. Semantic HTML (`<nav>`, `<ul>`, `<button>`)
5. Focus management (trap focus in mobile menu)
6. Dark mode support

**Iterative refinement loop**:
1. Initial implementation
2. WCAG audit finding: missing `aria-expanded`
3. Fix: add `aria-expanded` state
4. WCAG audit finding: focus trap not working
5. Fix: implement proper focus trap with Escape key

This 5-step loop mimics real frontend development, not a one-shot prompt.

### Three Flows

| Flow | Terraphim | Haystack | KG | Description |
|------|-----------|----------|-----|-------------|
| **A** | Yes | Ripgrep | Frontend | Local content search + KG concept boost |
| **B** | Yes | FFF | Frontend | Fuzzy file find + KG-scored grep |
| **C** | No | None | None | Control - default OpenCode |

### Metrics Collected

| Metric | Flow A | Flow B | Flow C |
|--------|--------|--------|--------|
| Time to initial solution | | | |
| Total session time | | | |
| Iterations to WCAG compliance | | | |
| Tool calls count | | | |
| MCP tool calls breakdown | | | |
| WCAG violations (initial) | | | |
| WCAG violations (final) | | | |
| Lines of code | | | |
| Cognitive complexity | | | |

---

## 3. Setup Steps

### Step 1: Validate Frontend KG Records

**Action**: Validate each of 18 concept files in `data/kg/frontend/`

**Validation criteria**:
- Valid markdown with `# Heading`
- Has `synonyms::` line with at least 3 terms
- No broken internal links
- Description is 2+ sentences

**Output**: `kg-validation-report.md` - list of valid/invalid concepts with issues

### Step 2: Copy KG to User Config

**Action**: Copy validated KG to `~/.config/terraphim/kg/frontend/`

**Commands**:
```bash
mkdir -p ~/.config/terraphim/kg/frontend
cp -r /home/alex/projects/terraphim/terraphim-ai/data/kg/frontend/* ~/.config/terraphim/kg/frontend/
```

### Step 3: Build terraphim_mcp_server with Features

**Action**: Build MCP server with grepapp + fff features

**Commands**:
```bash
cargo build --release -p terraphim_mcp_server --features grepapp,fff
cp target/release/terraphim_mcp_server ~/.cargo/bin/
```

**Output**: Binary with both GrepApp (GitHub search) and FFF (fuzzy file find) support

### Step 4: Configure Frontend Developer Role

**Action**: Add Frontend Developer role to `~/.config/terraphim/embedded_config.json`

**Two configurations** (for Flow A vs Flow B):

**Ripgrep Config (Flow A)**:
```json
{
  "Frontend Developer (Ripgrep)": {
    "shortname": "fedev-rg",
    "relevance_function": "terraphim-graph",
    "terraphim_it": true,
    "kg": {
      "knowledge_graph_local": {
        "input_type": "markdown",
        "path": "~/.config/terraphim/kg/frontend"
      }
    },
    "haystacks": [
      {
        "location": "~/projects/frontend-test",
        "service": "Ripgrep",
        "read_only": true,
        "extra_parameters": {}
      },
      {
        "location": "https://grep.app",
        "service": "GrepApp",
        "extra_parameters": { "language": "typescript" }
      }
    ],
    "llm_enabled": false
  }
}
```

**FFF Config (Flow B)**:
```json
{
  "Frontend Developer (FFF)": {
    "shortname": "fedev-fff",
    "relevance_function": "terraphim-graph",
    "terraphim_it": true,
    "kg": {
      "knowledge_graph_local": {
        "input_type": "markdown",
        "path": "~/.config/terraphim/kg/frontend"
      }
    },
    "haystacks": [
      {
        "location": "~/projects/frontend-test",
        "service": "FFF",
        "read_only": true,
        "extra_parameters": {}
      },
      {
        "location": "https://grep.app",
        "service": "GrepApp",
        "extra_parameters": { "language": "typescript" }
      }
    ],
    "llm_enabled": false
  }
}
```

### Step 5: Register MCP Servers in OpenCode

**Action**: Update `~/.config/opencode/opencode.json`

**Config**:
```json
{
  "mcp": {
    "terraphim-rg": {
      "type": "local",
      "command": ["~/.cargo/bin/terraphim_mcp_server"],
      "environment": {
        "TERRAPHIM_ROLE": "Frontend Developer (Ripgrep)"
      }
    },
    "terraphim-fff": {
      "type": "local",
      "command": ["~/.cargo/bin/terraphim_mcp_server"],
      "environment": {
        "TERRAPHIM_ROLE": "Frontend Developer (FFF)"
      }
    },
    "gitea-robot": {
      "type": "local",
      "command": ["/home/alex/.local/bin/gtr", "mcp-server"]
    }
  }
}
```

### Step 6: Create Fresh Svelte Project

**Action**: Create minimal SvelteKit project (realistic fresh start)

**Commands**:
```bash
cd ~/projects
npm create svelte@latest frontend-test -- --template minimal --types typescript --no-prettier --no-eslint --no-playwright --no-vitest
cd frontend-test
yarn install
yarn build
```

**Output**: Clean SvelteKit project ready for nav component

### Step 7: Execute Flow A (Ripgrep)

**Session**:
1. Open OpenCode in `~/projects/frontend-test`
2. Enable `terraphim-rg` MCP server
3. Prompt: "Add accessible navigation to this project. Use iterative refinement."
4. Capture all tool calls, responses, code
5. WCAG audit after initial + final

### Step 8: Execute Flow B (FFF)

**Session**:
1. Reset project (git stash if needed, fresh checkout)
2. Switch to `terraphim-fff` MCP server
3. Same prompts as Flow A
4. Capture same metrics

### Step 9: Execute Flow C (Control)

**Session**:
1. Disable both terraphim MCP servers
2. Same prompts as Flow A
3. Capture same metrics

### Step 10: Analysis and Documentation

**Outputs**:
- `experiment-results-flow-a.md` - Flow A detailed log
- `experiment-results-flow-b.md` - Flow B detailed log
- `experiment-results-flow-c.md` - Flow C detailed log
- `experiment-comparison.md` - Side-by-side analysis
- `article-frontend-dev-role-setup.md` - Public article for terraphim.ai
- `article-fff-vs-ripgrep.md` - Technical comparison article

---

## 4. Risk Analysis

| Risk | Impact | Mitigation |
|------|--------|------------|
| FFF haystack not available in terraphim_middleware | Flow B blocked | Check feature flag, build from source |
| GrepApp rate limiting | Flow A/B degraded | Fallback to local-only search |
| KG validation finds many invalid records | Experiment delayed | Subset of valid records still useful |
| OpenCode MCP switch requires restart | Session interruption | Document restart procedure |
| WCAG audit tool not available | Manual audit only | Use axe-core or manual checklist |

---

## 5. Decision Points

1. **FFF ServiceType**: Verify `fff` is a valid haystack ServiceType in terraphim_middleware
2. **MCP role switching**: Does `TERRAPHIM_ROLE` env var switch roles at runtime?
3. **Svelte project location**: `~/projects/frontend-test` or elsewhere?
4. **WCAG tool**: Use axe-core CLI, pa11y, or manual audit?
