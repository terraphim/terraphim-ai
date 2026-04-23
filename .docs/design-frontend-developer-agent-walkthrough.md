# Implementation Plan: Front-End Developer Agent Walkthrough

**Status**: Draft
**Research Doc**: `.docs/research-frontend-developer-agent-walkthrough.md`
**Date**: 2026-04-23
**Estimated Effort**: 1 day (writing)

---

## Overview

### Summary

Create a detailed, step-by-step walkthrough document showing how to create a front-end developer agent using terraphim-agent, grepapp haystack, and a domain-specific knowledge graph. The walkthrough covers the full lifecycle from setup to daily use.

### Approach

Use the existing `frontend-engineer` template as the starting point, extend the knowledge graph from the 6-file Lux KG to a comprehensive front-end KG, and demonstrate dual-haystack search (local Ripgrep + GrepApp). Written as a practical, hands-on guide with runnable commands.

### Scope

**In Scope:**
- Creating a front-end developer knowledge graph (15-20 Markdown concept files)
- Configuring the role with dual haystacks (Ripgrep + GrepApp)
- Building terraphim-agent with grepapp feature
- Running setup, search, autocomplete, replace, and validate commands
- Explaining the architecture (KG -> Aho-Corasick -> Haystack -> Ranked results)

**Out of Scope:**
- MCP server integration
- LLM chat setup (Ollama)
- Multi-agent orchestration
- Desktop UI (Tauri/Svelte)

**Avoid At All Cost:**
- Writing a general "what is AI agents" essay
- Creating an exhaustive KG with 50+ files (diminishing returns)
- Duplicating existing documentation (grepapp-feature.md, user-guide)
- Adding LLM configuration complexity to a deterministic-focused walkthrough

### 5/25 Analysis

**Top 5 (IN scope):**
1. Knowledge graph creation with domain-specific concepts
2. Role configuration with dual haystacks
3. Build and setup instructions
4. Practical CLI usage examples
5. Architecture explanation linking all components

**Avoid At All Cost (20 rejected):**
- LLM chat, MCP server, agent supervisor, multi-agent coordination, Tauri desktop, crate publication, CI/CD, performance benchmarking, security audit, i18n, visual regression testing, accessibility audit tools, CSS-in-JS patterns, specific framework deep-dives (React hooks, Vue composition API), WebGL/3D, animation libraries, testing frameworks (Jest, Vitest), E2E testing (Playwright), deployment pipelines, monitoring

---

## Architecture

### Component Diagram

```
+---------------------------+
|     Walkthrough Document   |
|  (Markdown in docs/)       |
+---------------------------+
         |
         | describes how to create:
         v
+---------------------------+      +---------------------------+
|   Knowledge Graph (KG)     |      |     Role Configuration     |
|  kg/frontend/*.md          |      |  (JSON config)             |
|  - Each .md = concept      |      |  - name, shortname         |
|  - heading + description   |      |  - relevance_function      |
|  - synonyms:: directive    |      |  - kg path                 |
+---------------------------+      |  - haystacks[]              |
         |                         +---------------------------+
         | builds                           |              |
         v                                  |              |
+---------------------------+               |              |
|  Aho-Corasick Automata    |               |              |
|  - LeftmostLongest match  |               |              |
|  - case-insensitive       |               |              |
|  - TF-IDF fallback        |               |              |
+---------------------------+               |              |
         |                                  |              |
         | matches query terms              |              |
         v                                  v              v
+---------------------------+      +---------------------------+
|    Haystack: Ripgrep       |      |  Haystack: GrepApp        |
|    (local files)           |      |  (grep.app API)           |
|    - user project dir      |      |  - GitHub repos           |
|    - markdown, TS, JS      |      |  - filter: language=ts    |
+---------------------------+      +---------------------------+
         |                                  |
         +---------- merged + ranked -------+
                      |
                      v
              +---------------------------+
              |   terraphim-agent CLI      |
              |   search, suggest, replace |
              +---------------------------+
```

### Data Flow

```
User query "flexbox responsive"
    -> Auto-route to Front-End Developer role (best Aho-Corasick match)
    -> Aho-Corasick finds: "flexbox" -> responsive-design node, "responsive" -> responsive-design node
    -> Search both haystacks:
       - Ripgrep: find local files mentioning flexbox/responsive
       - GrepApp: find GitHub code with flexbox patterns
    -> Merge results, rank by relevance (BM25Plus)
    -> Return ranked Document list
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use `setup --template` as starting point | Fastest path to working agent | Manual JSON config (error-prone) |
| Extend Lux KG rather than start fresh | 6 files already exist with correct format | Start from zero (wasted effort) |
| BM25Plus relevance function | Good for field-weighted document search | TerraphimGraph (needs more nodes to be effective) |
| Filter GrepApp to `language: "typescript"` | Modern front-end is TypeScript-first | `"javascript"` (too broad), both (two haystacks = complex) |
| 15-20 KG concept files | Sufficient coverage without overwhelming | 6 (too few), 50+ (diminishing returns) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| LLM chat integration | Not core to deterministic agent walkthrough | Confuses the KG-first message, adds Ollama dependency |
| MCP server setup | Separate concern, advanced topic | Doubles walkthrough length for marginal value |
| React/Vue specific KG entries | Framework-agnostic is more broadly useful | Alienates non-React/Vue readers |
| 50+ KG concept files | Diminishing returns, overwhelms reader | Makes walkthrough tedious rather than instructive |

### Simplicity Check

> "Minimum code that solves the problem."

**What if this could be easy?** The simplest walkthrough is: run `setup`, copy KG files, search. That's 3 steps. The design adds architecture explanation and KG creation guidance because readers need to understand *why* each piece exists to customise it. No speculative content.

---

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `docs/walkthroughs/frontend-developer-agent.md` | The walkthrough document |
| `data/kg/frontend/*.md` (15-20 files) | Front-end developer knowledge graph concepts |
| `config/frontend-engineer-config.json` | Example full role configuration |

### Modified Files

| File | Changes |
|------|---------|
| None | No code changes -- this is a documentation-only task |

### Existing Reference Files (read-only)

| File | Purpose |
|------|---------|
| `data/kg/lux/*.md` | Template for KG file format |
| `crates/terraphim_agent/src/onboarding/templates.rs` | Frontend engineer template definition |
| `docs/development/grepapp-feature.md` | GrepApp feature documentation |
| `blog/2026-02-16-multi-haystack-roles-grepapp.md` | Multi-haystack blog post |

---

## Knowledge Graph Design

### Concept Files to Create

The KG will live in `data/kg/frontend/` with these concept files:

| File | Concept | Key Synonyms |
|------|---------|-------------|
| `responsive-design.md` | Responsive Design | responsive, media query, breakpoint, mobile-first, viewport, flexbox, CSS grid, container query |
| `accessibility.md` | Accessibility (a11y) | a11y, WCAG, ARIA, screen reader, keyboard navigation, focus management, semantic HTML |
| `component-design.md` | Component Design | component, props, slots, events, lifecycle, reactive, state, composition |
| `svelte-patterns.md` | Svelte Patterns | Svelte, SvelteKit, rune, $state, $derived, $effect, bind, store, writable |
| `visual-design.md` | Visual Design | design system, theme, colour, typography, spacing, shadow, depth |
| `interaction-patterns.md` | Interaction Patterns | animation, transition, gesture, drag, scroll, hover, focus |
| `css-layout.md` | CSS Layout | flexbox, grid, box model, positioning, float, inline, block |
| `typescript.md` | TypeScript | TS, type safety, interface, generic, enum, type guard, declaration |
| `state-management.md` | State Management | store, signal, observable, reducer, action, dispatch, reactive state |
| `build-tools.md` | Build Tools | Vite, bundler, webpack, rollup, esbuild, HMR, tree shaking |
| `testing-frontend.md` | Frontend Testing | unit test, integration test, E2E, snapshot, visual regression, coverage |
| `performance.md` | Performance | lazy loading, code splitting, bundle size, rendering, paint, layout shift |
| `css-custom-properties.md` | CSS Custom Properties | CSS variable, custom property, theme token, oklch, colour space |
| `forms-validation.md` | Forms and Validation | form, input, validation, schema, constraint, submit, error handling |
| `api-integration.md` | API Integration | fetch, REST, GraphQL, endpoint, request, response, caching |
| `browser-apis.md` | Browser APIs | DOM, event, storage, worker, intersection, mutation, resize |
| `package-management.md` | Package Management | bun, npm, yarn, pnpm, dependency, lockfile, semver |
| `developer-experience.md` | Developer Experience | DX, linting, formatting, debugging, hot reload, source map |

### KG File Format

Each file follows this template:

```markdown
# Concept Name

[One-paragraph description explaining the concept and its relevance to front-end development.]

synonyms:: term1, term2, term3, term4
```

Optional directives for enhanced matching:
```markdown
trigger:: when working with responsive layouts or adapting to screen sizes
pinned:: false
priority:: 50
```

---

## Walkthrough Document Structure

### Section 1: Introduction (what and why)

**Content**: Explain what a front-end developer agent is, why Terraphim's deterministic approach matters, and what the reader will achieve.

**Key points:**
- Agent = Role + Knowledge Graph + Haystack(s)
- Deterministic KG matching vs. probabilistic LLM
- Privacy-first: all processing local, GrepApp searches public code only

### Section 2: Prerequisites and Build

**Content**: Install terraphim-agent from source with grepapp feature.

**Commands:**
```bash
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release --features grepapp
cargo install --path crates/terraphim_agent --features grepapp
```

### Section 3: Quick Start with Setup Wizard

**Content**: Use the built-in template for instant setup.

**Commands:**
```bash
terraphim-agent setup --template frontend-engineer --path ~/projects/my-frontend
```

**Explanation**: What this creates -- role config, KG path, dual haystack.

### Section 4: Understanding the Knowledge Graph

**Content**: Deep dive into KG structure, format, and how to create concept files.

**Subsections:**
- 4.1 What is a knowledge graph in Terraphim?
- 4.2 KG Markdown format (heading, description, synonyms)
- 4.3 How Aho-Corasick matching works (leftmost-longest, case-insensitive)
- 4.4 TF-IDF fallback with trigger directives
- 4.5 Creating new concept files with examples

### Section 5: Creating the Front-End Knowledge Graph

**Content**: Step-by-step creation of 15-20 concept files.

**Approach**: Show 3-4 files in full, then list the rest with a table.

### Section 6: Understanding Haystacks

**Content**: Explain the haystack abstraction, dual-haystack configuration.

**Subsections:**
- 6.1 What is a haystack?
- 6.2 Ripgrep haystack (local files)
- 6.3 GrepApp haystack (GitHub code search)
- 6.4 How results from multiple haystacks are merged

### Section 7: Configuring the Role

**Content**: Show the full role JSON configuration.

**Key config:**
```json
{
  "Front-End Developer": {
    "shortname": "fedev",
    "name": "Front-End Developer",
    "relevance_function": "BM25Plus",
    "kg": {
      "knowledge_graph_local": {
        "input_type": "markdown",
        "path": "~/.config/terraphim/kg/frontend"
      }
    },
    "haystacks": [
      {"location": "~/projects", "service": "Ripgrep", "read_only": true},
      {"location": "https://grep.app", "service": "GrepApp", "extra_parameters": {"language": "typescript"}}
    ]
  }
}
```

### Section 8: Using the Agent

**Content**: Practical CLI usage with examples.

**Commands demonstrated:**
```bash
# Search with auto-route
terraphim-agent search "flexbox responsive layout"

# Fuzzy autocomplete
terraphim-agent suggest "flex" --fuzzy

# Replace text with KG links
terraphim-agent replace "use npm to install" --format markdown

# Validate text against KG
terraphim-agent validate "The component uses semantic HTML and ARIA attributes"

# Interactive REPL
terraphim-agent repl
```

### Section 9: Architecture Deep Dive

**Content**: How all the pieces fit together end-to-end.

**Flow**: Query -> Auto-route -> Aho-Corasick -> Graph traversal -> Haystack search -> Merge -> Rank -> Results

### Section 10: Next Steps and Customisation

**Content**: Ideas for extending the agent.

- Add more KG concepts
- Add LLM chat (Ollama)
- Add MCP server for AI assistant integration
- Create additional roles (React specialist, CSS expert)

---

## Implementation Steps

### Step 1: Create Knowledge Graph Files

**Files**: `data/kg/frontend/*.md` (18 files)
**Description**: Create front-end concept Markdown files following the established format
**Estimated**: 2 hours

Create each file with:
1. H1 heading (concept name)
2. One-paragraph description
3. `synonyms::` line with relevant terms
4. Optional `trigger::` directive for TF-IDF fallback

### Step 2: Create Example Configuration

**Files**: `config/frontend-engineer-config.json`
**Description**: Write a complete, commented role configuration JSON file
**Estimated**: 30 minutes

### Step 3: Write Walkthrough Document

**Files**: `docs/walkthroughs/frontend-developer-agent.md`
**Description**: Write the full walkthrough following the 10-section structure above
**Estimated**: 4-5 hours

**Writing guidelines:**
- Every command must be runnable (test first)
- Include expected output where helpful
- Use the Terraphim project itself as the example project
- Keep British English throughout
- No emoji
- Use FontAwesome icons in HTML if needed
- Link to existing docs (grepapp-feature.md, user-guide/) rather than duplicating

### Step 4: Verify Walkthrough

**Description**: Follow the walkthrough end-to-end on a fresh setup
**Estimated**: 1 hour

**Verification steps:**
1. Build with `--features grepapp` succeeds
2. `setup --template frontend-engineer` creates valid config
3. `search "flexbox"` returns results from both haystacks
4. `suggest "flex" --fuzzy` returns autocomplete suggestions
5. `replace "use npm"` produces correct replacement
6. All KG concept files parse without errors

---

## Rollback Plan

If issues discovered:
1. Remove `docs/walkthroughs/frontend-developer-agent.md`
2. Remove `data/kg/frontend/` directory
3. No code changes to roll back

---

## Performance Considerations

N/A -- documentation task. The walkthrough describes a CLI tool, not a performance-critical system.

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide on TypeScript vs JavaScript filter for GrepApp | Pending | Human |
| Confirm walkthrough location (docs/walkthroughs/ vs blog/) | Pending | Human |
| Verify `setup --template frontend-engineer` works end-to-end | Pending | Implementer |

---

## Approval

- [ ] Research document reviewed and approved
- [ ] Walkthrough structure approved
- [ ] KG concept list approved
- [ ] Human approval received
