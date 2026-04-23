# Research Document: Front-End Developer Agent Walkthrough

**Status**: Draft
**Date**: 2026-04-23
**Author**: AI Agent (Disciplined Research Phase 1)

---

## 1. Problem Restatement and Scope

### Problem Statement

There is no comprehensive, step-by-step walkthrough showing how to create a specialised front-end developer agent using terraphim-agent, grepapp haystack, and a domain-specific knowledge graph. Users need a detailed guide that demonstrates the full lifecycle: defining a role, creating a knowledge graph, configuring haystacks (local + grepapp), and using the agent in practice.

### IN Scope

- Creating the knowledge graph Markdown files for front-end development concepts
- Defining a "Front-End Developer" role with appropriate configuration
- Configuring dual haystacks: local Ripgrep + GrepApp (grep.app) for JavaScript/TypeScript
- Using terraphim-agent CLI commands to interact with the agent
- Explaining the architecture and how each component works together
- Practical usage examples (search, autocomplete, replace, validate)

### OUT Scope

- MCP server integration (separate concern)
- LLM chat configuration (can be added later)
- Agent supervisor / multi-agent orchestration (advanced topic)
- Publishing to crates.io
- Desktop UI (Tauri/Svelte frontend of terraphim-ai)

---

## 2. User and Business Outcomes

### Visible Changes

1. A reader can follow the walkthrough end-to-end and have a working front-end developer agent
2. The agent searches both local project files and GitHub code via grep.app
3. Knowledge graph provides deterministic, privacy-first concept matching for front-end terms
4. Autocomplete and search return ranked, relevant results for front-end queries
5. The walkthrough serves as a template for creating other specialised agents

### Business Value

- Demonstrates terraphim-agent's extensibility to new domains
- Shows the unique value of deterministic KG-based search vs. pure LLM approaches
- Provides a replicable pattern for community members to create their own agents
- Highlights the dual-haystack capability (local + global code search)

---

## 3. System Elements and Dependencies

### 3.1 terraphim-agent Binary

| Aspect | Detail |
|--------|--------|
| Location | `~/.cargo/bin/terraphim-agent` |
| Role | CLI/TUI frontend for the entire Terraphim stack |
| Key Commands | `search`, `suggest`, `replace`, `validate`, `repl`, `setup` |
| Dependencies | terraphim_automata, terraphim_rolegraph, terraphim_service, terraphim_config |

### 3.2 Knowledge Graph (KG) System

| Aspect | Detail |
|--------|--------|
| Format | Directory of Markdown files (`.md`) |
| Syntax | `# Heading`, description paragraph, `synonyms:: term1, term2, ...` |
| Additional Directives | `trigger::`, `pinned::`, `priority::`, `type:::` |
| Matching | Aho-Corasick automata (LeftmostLongest, case-insensitive) |
| Fallback | TF-IDF cosine similarity on `trigger::` descriptions |
| Existing Frontend KG | `data/kg/lux/` (6 files: accessibility, component-design, interaction-patterns, responsive-design, svelte-patterns, visual-design) |

### 3.3 Haystack System

| Aspect | Detail |
|--------|--------|
| Core Trait | `HaystackProvider::async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>>` |
| Ripgrep | Local filesystem search (always available) |
| GrepApp | Wraps `https://grep.app/api/search` - searches GitHub repos |
| GrepApp Filters | `language`, `repo`, `path` via `extra_parameters` |
| Feature Flag | `--features grepapp` required for GrepApp |
| Code | `crates/haystack_grepapp/` (client.rs, lib.rs, models.rs) |

### 3.4 Role Configuration

| Aspect | Detail |
|--------|--------|
| Format | JSON (embedded_config.json or custom) |
| Key Fields | `name`, `shortname`, `relevance_function`, `kg`, `haystacks`, `llm_enabled` |
| Existing Template | `frontend-engineer` in `templates.rs` (BM25Plus, dual haystack, local KG) |
| Setup Command | `terraphim-agent setup --template frontend-engineer --path ~/projects` |

### 3.5 Frontend Engineer Template (Existing)

| Field | Value |
|-------|-------|
| Name | "FrontEnd Engineer" |
| Shortname | "frontend" |
| Relevance Function | BM25Plus |
| Theme | "yeti" |
| KG Path | `docs/frontend` (local markdown) |
| Haystack 1 | Ripgrep at user-specified path |
| Haystack 2 | GrepApp filtered to `language: "javascript"` |
| LLM | Disabled |

---

## 4. Constraints and Their Implications

### 4.1 Feature Flag Constraint

**Constraint**: GrepApp requires `--features grepapp` at build time. The `haystack_grepapp` crate is not published to crates.io.

**Implication**: The walkthrough must explain how to build from source with the grepapp feature. Users cannot use a published binary for this feature.

**De-risking**: Include exact build commands. Document the requirement clearly.

### 4.2 GrepApp API Limitations

**Constraint**: No authentication, rate limits apply (HTTP 429). Only searches public GitHub repos. Max 1000 character query.

**Implication**: Walkthrough should note rate limiting and recommend caching/conservative query strategies. Show how to use `language` filter effectively.

### 4.3 KG Path Configuration

**Constraint**: The template uses a relative path `docs/frontend` for the KG. This must exist at runtime.

**Implication**: Walkthrough must show creating the KG directory and files at the correct location, or configuring an absolute path.

### 4.4 Knowledge Graph Specificity

**Constraint**: The existing `data/kg/lux/` has only 6 concept files. A useful front-end developer agent needs more comprehensive coverage.

**Implication**: The walkthrough should create an expanded KG with 15-20+ concept files covering CSS, JavaScript/TypeScript, accessibility, performance, testing, build tools, and framework patterns.

### 4.5 Dual-Haystack Merging

**Constraint**: Results from Ripgrep and GrepApp are merged and ranked together. Duplicate handling exists but may need tuning.

**Implication**: Walkthrough should explain how results from different sources are combined and how the relevance function affects ranking.

---

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Severity | De-risking |
|------|----------|------------|
| GrepApp API rate limiting during walkthrough | Medium | Use conservative queries, show caching options |
| KG files too generic to be useful | Medium | Use domain-specific synonyms from real front-end projects |
| Walkthrough too long/complex | High | Break into clear sections with checkpoints |
| BM25Plus may not be optimal for code search | Low | Mention TerraphimGraph as alternative, explain trade-offs |

### Unknowns

1. **ASSUMPTION**: The `terraphim-agent setup` command fully works with the frontend-engineer template (it exists in code but may not be tested end-to-end)
2. **ASSUMPTION**: GrepApp extra_parameters `language: "typescript"` works as well as `"javascript"` (likely both supported)
3. **UNKNOWN**: Whether the KG at `docs/frontend` is expected to be relative to CWD, config dir, or user home

### Assumptions

1. The reader has Rust toolchain installed and can build from source
2. The reader has basic familiarity with CLI tools
3. The reader works on front-end projects (JavaScript/TypeScript/Svelte/React)
4. The `grepapp` feature compiles cleanly on the user's platform

---

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Three-layer architecture** (Automata + RoleGraph + Service) - explaining how these interact
2. **Feature flags** - grepapp must be enabled at compile time
3. **Dual haystacks** - merging results from local and remote sources
4. **KG Markdown format** - the directive syntax is custom and needs explanation

### Simplification Opportunities

1. **Use `setup --template`**: The onboarding wizard handles most configuration automatically. The walkthrough can focus on customisation.
2. **Start with existing Lux KG**: The `data/kg/lux/` directory already has 6 relevant files. Start there and extend.
3. **Focus on the 5 most important CLI commands**: `setup`, `search`, `suggest`, `replace`, `repl` - skip advanced features
4. **Single example project**: Use one concrete front-end project as the running example throughout

### Recommended Structure

The walkthrough should follow a "build up" pattern:
1. Quick start (setup command)
2. Understand what was created (role, KG, haystacks)
3. Customise the knowledge graph (add concepts)
4. Use the agent (search, autocomplete, replace)
5. Advanced configuration (LLM, additional haystacks)

---

## 7. Questions for Human Reviewer

1. **Target audience**: Should the walkthrough target developers new to Terraphim, or those already familiar with the basics?
   - *Why it matters*: Determines how much background to include on KG concepts and Aho-Corasick.

2. **Framework specificity**: Should the KG focus on Svelte/SvelteKit (matching the project stack), or be framework-agnostic?
   - *Why it matters*: Affects which concepts and synonyms to include.

3. **LLM integration**: Should the walkthrough include Ollama/LLM Router setup, or keep it deterministic-only?
   - *Why it matters*: Adds significant complexity vs. showing the pure KG approach.

4. **Walkthrough format**: Markdown document, blog post, or interactive REPL session transcript?
   - *Why it matters*: Affects depth, tone, and where the document lives.

5. **GrepApp language filter**: Should it filter to "javascript", "typescript", or both (two GrepApp haystacks)?
   - *Why it matters*: TypeScript results may be more relevant for modern front-end work.

6. **Scope of KG**: How many concept files are sufficient? The Lux KG has 6; should we target 10, 20, or 30+?
   - *Why it matters*: Determines walkthrough length and the depth of KG design guidance.
