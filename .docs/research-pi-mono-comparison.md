# Research Document: pi-mono vs terraphim-ai Architecture Analysis

## 1. Problem Restatement and Scope

**IN Scope:**
- Comparative analysis of two AI agent toolkit architectures
- Evaluation against software engineering best practices
- Assessment of design patterns, modularity, and maintainability
- Documentation and build tooling comparison
- Agent/AI integration approaches

**OUT of Scope:**
- Implementation recommendations
- Migration strategies between architectures
- Performance benchmarking
- Security vulnerability analysis

## 2. User & Business Outcomes

This research enables:
- Understanding different architectural approaches for AI agent systems
- Identifying best practices applicable to terraphim-ai
- Learning from pi-mono's TypeScript-centric design
- Documenting architectural trade-offs for future decisions

## 3. System Elements and Dependencies

### pi-mono Architecture

| Element | Location | Responsibility | Dependencies |
|---------|----------|----------------|--------------|
| pi-ai | packages/ai/ | Unified LLM API | @anthropic-ai/sdk, openai, @google/genai |
| pi-agent-core | packages/agent/ | Agent runtime | pi-ai, typebox |
| pi-coding-agent | packages/coding-agent/ | CLI tool | pi-agent-core, pi-tui, pi-web-ui |
| pi-tui | packages/tui/ | Terminal UI | chalk, marked |
| pi-web-ui | packages/web-ui/ | Web components | lit, tailwindcss |
| pi-mom | packages/mom/ | Slack bot | pi-coding-agent |
| pi-pods | packages/pods/ | vLLM deployment | - |

### terraphim-ai Architecture

| Element | Location | Responsibility | Dependencies |
|---------|----------|----------------|--------------|
| terraphim_agent | crates/terraphim_agent/ | CLI/TUI binary | 40+ workspace crates |
| terraphim_automata | crates/terraphim_automata/ | Text processing | aho-corasick, wasm-bindgen |
| terraphim_server | terraphim_server/ | HTTP API | axum, tokio |
| terraphim_rolegraph | crates/terraphim_rolegraph/ | Knowledge graph | petgraph |
| terraphim_multi_agent | crates/terraphim_multi_agent/ | Multi-agent system | 13 specialized agents |
| desktop | desktop/ | Svelte + Tauri frontend | bulma, d3 |
| Firecracker | terraphim_firecracker/ | VM execution | firecracker-go-sdk |

## 4. Constraints and Their Implications

### Language/Runtime Constraints

**pi-mono TypeScript/Node.js:**
- GC-based memory management (simpler but less predictable)
- Single-threaded event loop (simpler concurrency but limited parallelism)
- npm ecosystem (vast libraries but dependency complexity)
- Interpreted execution (slower but faster development cycle)

**terraphim-ai Rust:**
- Compile-time memory safety (zero runtime errors but steep learning curve)
- Zero-cost abstractions (performance but compile-time complexity)
- Cargo workspace (excellent modularity but build coordination overhead)
- Native execution (fast but longer compile times)

### Distribution Constraints

**pi-mono:**
- npm packages for Node.js environments
- Compiled binaries for standalone use
- Limited WASM support

**terraphim-ai:**
- Multi-platform distribution (crates.io, npm, PyPI, Homebrew)
- First-class WASM support (terraphim_automata)
- Native binaries across platforms

### Integration Constraints

**pi-mono:**
- Extension system for custom providers (npm/git/local)
- Runtime provider registration
- File-based session storage (.jsonl)

**terraphim-ai:**
- MCP (Model Context Protocol) server integration
- Knowledge graph context enrichment
- Two-stage validation hooks system

## 5. Risks, Unknowns, and Assumptions

### Risks

**Technical Risks:**
1. **pi-mono coupling**: Tight internal dependencies between packages may hinder independent evolution
2. **terraphim-ai complexity**: 40+ crates create cognitive overhead and build coordination challenges
3. **Maintenance burden**: Both projects require keeping up with rapidly evolving LLM APIs

**Product Risks:**
1. **Feature parity**: Different architectural approaches may limit code sharing
2. **Documentation drift**: Large codebases risk documentation becoming outdated
3. **Contributor onboarding**: Complexity may deter new contributors

### Unknowns

1. **Adoption metrics**: How widely is each toolkit used in production?
2. **Performance benchmarks**: No direct performance comparisons available
3. **Long-term maintenance**: Both projects are actively developed (pi-mono 2,669 commits, terraphim-ai extensive)

### Assumptions

1. Both projects prioritize different trade-offs (velocity vs. performance)
2. TypeScript vs. Rust choice is intentional based on use case requirements
3. Both architectures are suitable for their respective domains

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

**pi-mono:**
- Build ordering dependencies (tui -> ai -> agent -> coding-agent)
- Tight coupling between packages via npm workspace references
- Large main.ts file (1000+ lines) in coding-agent
- Extension system complexity

**terraphim-ai:**
- 40+ crates with interdependencies
- Feature flag matrix across crates
- Multiple language bindings (Rust, JS, Python)
- Experimental crates excluded from workspace

### Simplification Opportunities

1. **Clearer boundaries**: Both could benefit from explicit API contracts between modules
2. **Documentation consolidation**: terraphim-ai has excellent docs but could add ADRs
3. **Experimental crate management**: terraphim-ai could move experimental work to separate repo
4. **Main entry point**: pi-mono could modularize main.ts into smaller focused modules

## 7. Questions for Human Reviewer

1. **Scope validation**: Does this analysis cover the architectural aspects you care about most, or should we dive deeper into specific areas like security or testing?

2. **Comparison criteria**: Are there specific best practices or architectural patterns you want emphasized in the comparison (e.g., hexagonal architecture, microservices vs monolith)?

3. **Action orientation**: Should this research lead to specific recommendations for terraphim-ai, or is it purely informational?

4. **Depth preference**: Would you prefer a deeper analysis of one specific aspect (e.g., agent runtime design) or broad coverage across all architectural dimensions?

5. **Documentation standards**: Should we establish specific documentation quality gates for terraphim-ai based on findings from pi-mono?
