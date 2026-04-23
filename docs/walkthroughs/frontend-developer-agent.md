# Creating a Front-End Developer Agent with Terraphim

A step-by-step walkthrough for building a specialised front-end developer agent using terraphim-agent, the GrepApp haystack, and a domain-specific knowledge graph. This guide covers Svelte/SvelteKit development with TypeScript.

## What You Will Build

By the end of this walkthrough you will have a front-end developer agent that:

- Understands 18 front-end concepts via a knowledge graph (responsive design, accessibility, Svelte patterns, TypeScript, and more)
- Searches your local project files via Ripgrep
- Searches millions of GitHub repositories for TypeScript code via grep.app
- Returns deterministic, ranked results without an LLM
- Provides autocomplete suggestions for front-end terminology

## Prerequisites

- Rust toolchain (rustup.rs)
- Git
- A front-end project directory to search (optional but recommended)

## Step 1: Build terraphim-agent with GrepApp Support

Clone the repository and build with the `grepapp` feature flag:

```bash
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release
cargo build --release -p terraphim_middleware --features grepapp
```

The `grepapp` feature flag lives on the `terraphim_middleware` crate, which is a dependency of `terraphim_agent`. Building the middleware with the feature enables GrepApp haystack support.

Install the binary:

```bash
cargo install --path crates/terraphim_agent
```

Verify the installation:

```bash
terraphim-agent --version
```

The `--features grepapp` flag is required because the GrepApp haystack is an optional dependency. Without it, the agent can only use local haystacks (Ripgrep, QueryRs).

## Step 2: Quick Start with the Setup Wizard

Terraphim includes a built-in template for front-end developers. Run:

```bash
terraphim-agent setup --template frontend-engineer --path ~/projects/my-frontend
```

This creates a role called "FrontEnd Engineer" with:
- **Relevance function**: BM25Plus (field-weighted document ranking)
- **Knowledge graph**: Local Markdown files in `docs/frontend/`
- **Haystack 1**: Ripgrep searching your project directory
- **Haystack 2**: GrepApp searching GitHub, filtered to JavaScript

The setup wizard writes the configuration to `~/.config/terraphim/embedded_config.json`.

## Step 3: Understand the Architecture

Before customising, understand the three layers that make the agent work:

### Layer 1: Knowledge Graph

A knowledge graph (KG) is a directory of Markdown files. Each file defines a concept:

```
kg/frontend/
  accessibility.md
  component-design.md
  css-layout.md
  svelte-patterns.md
  typescript.md
  ...
```

Each file has three parts:

```markdown
# Concept Name

A description paragraph explaining the concept.

synonyms:: term1, term2, term3
```

The `synonyms::` directive lists alternative terms that resolve to the same concept. For example, `a11y`, `WCAG`, and `ARIA` all map to the "Accessibility" concept.

### Layer 2: Aho-Corasick Matching

When you search, terraphim-agent builds an Aho-Corasick automaton from all KG terms and their synonyms. This provides:

- **Leftmost-longest matching**: "CSS grid" matches as one term, not "CSS" and "grid" separately
- **Case-insensitive**: "Flexbox" and "flexbox" match identically
- **O(n+m) complexity**: Linear-time matching across all terms simultaneously

If no exact match is found, a TF-IDF fallback uses `trigger::` directives for semantic similarity.

### Layer 3: Haystacks

A haystack is a searchable data source. Each role can have multiple haystacks:

| Haystack | Source | What It Searches |
|----------|--------|-----------------|
| Ripgrep | Local filesystem | Your project files (`.svelte`, `.ts`, `.css`, `.md`) |
| GrepApp | grep.app API | Millions of public GitHub repositories |

Results from all haystacks are merged and ranked using the role's relevance function (BM25Plus).

### The Search Flow

```
Your query: "flexbox responsive layout"
       |
       v
[Auto-route] Agent picks the Front-End Developer role
  (highest Aho-Corasick match count against role KG)
       |
       v
[Aho-Corasick] Finds matching KG nodes:
  "flexbox"    -> CSS Layout, Responsive Design
  "responsive" -> Responsive Design
  "layout"     -> CSS Layout
       |
       v
[Haystack Search]
  Ripgrep:  searches ~/projects for files mentioning these terms
  GrepApp:  searches GitHub TypeScript repos for "flexbox responsive layout"
       |
       v
[Merge + Rank] BM25Plus scoring combines both result sets
       |
       v
[Results] Ranked list of documents with relevance scores
```

## Step 4: Create the Knowledge Graph

The default template uses a small KG. Let us create a comprehensive one for Svelte/SvelteKit front-end development.

### Create the KG Directory

```bash
mkdir -p ~/.config/terraphim/kg/frontend
```

### Create Concept Files

Each concept is a single Markdown file. Here are three examples to show the pattern:

**accessibility.md**:
```markdown
# Accessibility

Designing and building user interfaces that are usable by people with diverse abilities and assistive technologies, following WCAG guidelines with semantic HTML, ARIA attributes, and keyboard navigation support.

synonyms:: a11y, WCAG, WAI-ARIA, ARIA, screen reader, keyboard navigation, focus management, colour contrast, alt text, semantic HTML, accessible, assistive technology, skip link, live region, aria-label, aria-hidden, role, tabindex, focus-visible
```

**svelte-patterns.md**:
```markdown
# Svelte Patterns

Svelte-specific patterns for building reactive, compiled frontend applications using runes, stores, and SvelteKit conventions for routing, loading, and server-side rendering.

synonyms:: Svelte, SvelteKit, rune, $state, $derived, $effect, $props, bind, each block, await block, action, use directive, transition, fly, fade, slide, Vite, store, writable, readable, derived, load function, +page.svelte, +page.ts, +layout.svelte, form action, snapshot, navigator
```

**css-layout.md**:
```markdown
# CSS Layout

Modern CSS layout techniques including Flexbox, CSS Grid, container queries, and the box model for creating responsive, predictable page layouts with minimal code.

synonyms:: flexbox, grid, box model, positioning, float, inline, block, flex, grid-template, grid-area, gap, justify, align, place, auto-fit, auto-fill, minmax, subgrid, container query, @container
```

### Copy the Full KG

The complete knowledge graph with 18 concept files is available in the repository:

```bash
cp -r data/kg/frontend/ ~/.config/terraphim/kg/frontend/
```

The full KG covers:

| Concept | Key Synonyms |
|---------|-------------|
| Responsive Design | flexbox, CSS grid, breakpoint, container query, mobile-first |
| Accessibility | a11y, WCAG, ARIA, screen reader, semantic HTML |
| Component Design | props, slots, events, lifecycle, reactive, composition |
| Svelte Patterns | SvelteKit, rune, $state, $derived, load function, +page |
| Visual Design | design system, theme, oklch, design token, colour space |
| Interaction Patterns | animation, transition, gesture, drag, scroll |
| CSS Layout | flexbox, grid, box model, positioning, @container |
| TypeScript | type safety, interface, generic, type guard, satisfies |
| State Management | store, signal, writable, derived, context, $state |
| Build Tools | Vite, bundler, webpack, HMR, tree shaking |
| Frontend Testing | vitest, playwright, snapshot, visual regression |
| Performance | lazy loading, LCP, FID, CLS, Core Web Vitals |
| CSS Custom Properties | CSS variable, oklch, theme token, dark mode |
| Forms and Validation | form action, zod, superforms, constraint, aria-invalid |
| API Integration | fetch, REST, GraphQL, load function, SSR |
| Browser APIs | DOM, Web Worker, IntersectionObserver, localStorage |
| Package Management | bun, npm, yarn, pnpm, lockfile, semver |

## Step 5: Configure the Role

Create or update the role configuration at `~/.config/terraphim/embedded_config.json`. The key change from the default template is:

1. **TypeScript instead of JavaScript** for the GrepApp filter
2. **Correct KG path** pointing to our new knowledge graph
3. **Svelte/SvelteKit-focused synonyms** in the KG

The full configuration:

```json
{
  "Front-End Developer": {
    "shortname": "fedev",
    "name": "Front-End Developer",
    "relevance_function": "BM25Plus",
    "terraphim_it": false,
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
        "location": "~/projects",
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

### Configuration Fields Explained

| Field | Value | Why |
|-------|-------|-----|
| `relevance_function` | `BM25Plus` | Field-weighted ranking works well for mixed document types (code, markdown, config) |
| `terraphim_it` | `false` | Disables text replacement transformations (we want raw search, not KG rewriting) |
| `kg.path` | `~/.config/terraphim/kg/frontend` | Points to our 18-file front-end knowledge graph |
| `haystacks[0]` | Ripgrep at `~/projects` | Searches your local project files |
| `haystacks[1]` | GrepApp with `language: "typescript"` | Searches GitHub TypeScript repos via grep.app |
| `llm_enabled` | `false` | Deterministic mode: no LLM, no hallucination, fully reproducible |

## Step 6: Use the Agent

### Search

Search across both haystacks with auto-role selection:

```bash
terraphim-agent search "flexbox responsive layout"
```

The agent auto-routes to the Front-End Developer role because "flexbox" and "responsive" match KG terms. It searches your local files and GitHub simultaneously, merges, and ranks the results.

### Autocomplete

Get fuzzy suggestions for front-end terms:

```bash
terraphim-agent suggest "svel" --fuzzy
```

Returns: `Svelte`, `SvelteKit`, and any other matching terms from the KG.

### Replace

Replace terms with knowledge graph links:

```bash
terraphim-agent replace "use npm install to add the package" --format markdown
```

With `terraphim_it: true`, this would transform "npm" to the canonical "bun" term. Keep it `false` for pure search.

### Validate

Check if text mentions connected concepts:

```bash
terraphim-agent validate "The component uses semantic HTML and ARIA attributes" --connectivity
```

This checks whether "semantic HTML" and "ARIA" are connected in the knowledge graph (they both map to the Accessibility node).

### Interactive REPL

For exploratory searching:

```bash
terraphim-agent repl
```

Provides an interactive prompt where you can search, suggest, and navigate without restarting.

### Auto-Route

If you have multiple roles configured, terraphim-agent automatically selects the best role for each query by scoring the query against every role's Aho-Corasick automaton:

```bash
terraphim-agent search "ownership borrowing lifetime"
```

This query would auto-route to the Rust Engineer role (if configured) rather than Front-End Developer, because "ownership" and "borrowing" match the Rust KG, not the front-end KG.

## Step 7: How GrepApp Works

The GrepApp haystack wraps the [grep.app](https://grep.app) API, which indexes over a million public GitHub repositories.

### API Details

| Parameter | Description | Example |
|-----------|-------------|---------|
| `q` | Search query | `flexbox responsive` |
| `f.lang` | Language filter | `typescript` |
| `f.repo` | Repository filter | `sveltejs/kit` |
| `f.path` | Path filter | `src/routes/` |

The `extra_parameters` in your role configuration set the default filters. With `"language": "typescript"`, all GrepApp searches are automatically filtered to TypeScript files.

### Error Handling

- **Rate limiting**: grep.app returns HTTP 429. The agent gracefully degrades to Ripgrep-only results.
- **No results**: HTTP 404 is treated as empty results (not an error).
- **Network failure**: The agent continues with local results only.

### Advanced: Repository-Specific Search

To narrow GrepApp to a specific repository, update the haystack configuration:

```json
{
  "location": "https://grep.app",
  "service": "GrepApp",
  "extra_parameters": {
    "language": "typescript",
    "repo": "sveltejs/kit"
  }
}
```

## Step 8: Adding More Concepts

The knowledge graph is designed to grow. To add a new concept:

1. Create a new `.md` file in `~/.config/terraphim/kg/frontend/`
2. Follow the format: heading, description, synonyms
3. Restart the agent or run `terraphim-agent repl` (the KG is loaded at startup)

Example -- adding a SvelteKit routing concept:

```bash
cat > ~/.config/terraphim/kg/frontend/sveltekit-routing.md << 'EOF'
# SvelteKit Routing

File-based routing in SvelteKit where the directory structure under src/routes/ defines the application URL structure, with dynamic parameters, layout nesting, and server-side load functions.

synonyms:: route, routing, +page.svelte, +layout.svelte, +page.ts, +page.server.ts, params, slug, rest parameter, route group, parallax, navigate, goto, redirect, link, href, a tag
EOF
```

### Tips for Effective KG Entries

- **Be specific with synonyms**: Include both common and formal terms (`a11y` and `accessibility`)
- **Include misspellings**: If users commonly misspell a term, add it as a synonym
- **Include related tools**: `vitest`, `playwright` under testing concepts
- **Use trigger directives** for TF-IDF fallback: `trigger:: when testing Svelte components`
- **Use pinned nodes** sparingly: `pinned:: true` always includes a concept regardless of query

## Next Steps

- **Add LLM chat**: Set `llm_enabled: true` and configure an Ollama model for conversational search
- **MCP server**: Expose the agent as a Model Context Protocol tool for Claude Code or Cursor integration
- **More haystacks**: Add QueryRs for TypeScript documentation, or Quickwit for log analysis
- **Create more roles**: React specialist, CSS expert, or DevOps engineer using the same pattern

## Troubleshooting

### GrepApp returns no results

Check that you built `terraphim_middleware` with `--features grepapp`. Without it, the haystack is silently skipped:

```bash
cargo build --release -p terraphim_middleware --features grepapp
```

### KG terms not matching

Verify the KG path in your config is correct. Run:

```bash
terraphim-agent suggest "flex" --fuzzy
```

If this returns nothing, the KG is not loaded. Check the path and file permissions.

### Build fails with grepapp feature

The `haystack_grepapp` crate requires `reqwest` with TLS. Ensure you have OpenSSL or native-tls development headers installed.

---

**Last Updated**: 2026-04-23
