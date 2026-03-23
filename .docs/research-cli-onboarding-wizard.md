# Research Document: CLI Onboarding Wizard for terraphim-agent

**Status**: Draft
**Author**: AI Research Agent
**Date**: 2026-01-28
**Reviewers**: Alex

## Executive Summary

This research document analyzes the desktop configuration wizard and underlying data models to design an equivalent or better CLI onboarding wizard for terraphim-agent. The wizard should allow users to add roles to current configuration with haystacks and other options, select from sane defaults, or create new configurations.

## Problem Statement

### Description
First-time users of `terraphim-agent` currently face a steep learning curve. The CLI falls back to a default embedded configuration without guiding users through setup. In contrast, the desktop application provides a 3-step configuration wizard that walks users through creating roles, configuring haystacks, and setting up LLM providers.

### Impact
- New users may not understand how to configure roles and haystacks
- Users miss out on powerful features like knowledge graphs and LLM integration
- CLI-only users have no guided setup experience
- Reduces adoption by users who prefer terminal-based workflows

### Success Criteria
1. CLI wizard provides feature parity with desktop ConfigWizard.svelte
2. Users can add roles to existing configuration (additive, not replacement)
3. Pre-built templates available for quick start (sane defaults)
4. All configuration options from Role/Haystack types are accessible
5. Configuration is validated before saving
6. Wizard is skippable for experienced users

## Current State Analysis

### Existing Implementation

#### Desktop ConfigWizard (desktop/src/lib/ConfigWizard.svelte)
3-step wizard flow:
1. **Global Settings**: Config ID, global shortcut, default theme, default role selection
2. **Roles Configuration**: Add/edit/remove roles with full settings
3. **Review**: JSON preview and save

Role editing includes:
- Name and shortname
- Theme selection
- Relevance function (title-scorer, terraphim-graph, bm25, bm25f, bm25plus)
- Terraphim It toggle (knowledge graph enhancement)
- Knowledge graph configuration (remote URL or local path)
- Multiple haystacks with weight
- LLM provider configuration (OpenRouter/Ollama with model selection)

#### Current CLI Configuration (crates/terraphim_agent/src/service.rs)
- Uses `ConfigBuilder::new_with_id(ConfigId::Embedded)` for defaults
- Falls back to `build_default_embedded()` when no saved config exists
- No interactive setup process
- Configuration loaded from persistence layer or defaults

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Desktop Wizard | `desktop/src/lib/ConfigWizard.svelte` | 3-step setup wizard UI |
| Generated Types | `desktop/src/lib/generated/types.ts` | TypeScript type definitions |
| Config Crate | `crates/terraphim_config/src/lib.rs` | Config, Role, Haystack, KnowledgeGraph structs |
| TUI Service | `crates/terraphim_agent/src/service.rs` | CLI service layer |
| TUI Main | `crates/terraphim_agent/src/main.rs` | CLI entry point with clap |
| Default Configs | `terraphim_server/default/*.json` | Pre-built configuration templates |

### Data Models

#### Config
```rust
pub struct Config {
    pub id: ConfigId,                          // Server | Desktop | Embedded
    pub global_shortcut: String,               // e.g., "Ctrl+Shift+T"
    pub roles: AHashMap<RoleName, Role>,       // Map of role name to role
    pub default_role: RoleName,                // Default role to use
    pub selected_role: RoleName,               // Currently active role
}
```

#### Role
```rust
pub struct Role {
    pub shortname: Option<String>,             // e.g., "TerraEng"
    pub name: RoleName,                        // e.g., "Terraphim Engineer"
    pub relevance_function: RelevanceFunction, // terraphim-graph | title-scorer | bm25 | bm25f | bm25plus
    pub terraphim_it: bool,                    // Enable KG enhancement
    pub theme: String,                         // UI theme name
    pub kg: Option<KnowledgeGraph>,            // Knowledge graph config
    pub haystacks: Vec<Haystack>,              // Document sources
    // LLM settings
    pub llm_enabled: bool,
    pub llm_api_key: Option<String>,
    pub llm_model: Option<String>,
    pub llm_auto_summarize: bool,
    pub llm_chat_enabled: bool,
    pub llm_chat_system_prompt: Option<String>,
    pub llm_chat_model: Option<String>,
    pub llm_context_window: Option<u64>,       // Default: 32768
    pub extra: AHashMap<String, Value>,        // Extension fields
    pub llm_router_enabled: bool,
    pub llm_router_config: Option<LlmRouterConfig>,
}
```

#### Haystack
```rust
pub struct Haystack {
    pub location: String,                      // File path or URL
    pub service: ServiceType,                  // Ripgrep | Atomic | QueryRs | ClickUp | Mcp | Perplexity | GrepApp | AiAssistant | Quickwit
    pub read_only: bool,                       // Default: false
    pub fetch_content: bool,                   // Default: false
    pub atomic_server_secret: Option<String>,  // For Atomic service
    pub extra_parameters: HashMap<String, String>,
}
```

#### ServiceType (9 types)
- **Ripgrep**: Local filesystem search
- **Atomic**: Atomic Data server integration
- **QueryRs**: Rust docs and Reddit search
- **ClickUp**: Task management
- **Mcp**: Model Context Protocol servers
- **Perplexity**: AI-powered web search
- **GrepApp**: GitHub code search
- **AiAssistant**: AI session logs
- **Quickwit**: Log/observability search

#### KnowledgeGraph
```rust
pub struct KnowledgeGraph {
    pub automata_path: Option<AutomataPath>,   // Remote URL or local file
    pub knowledge_graph_local: Option<KnowledgeGraphLocal>,
    pub public: bool,
    pub publish: bool,
}

pub struct KnowledgeGraphLocal {
    pub input_type: KnowledgeGraphInputType,   // markdown | json
    pub path: PathBuf,
}
```

#### RelevanceFunction (5 options)
- `terraphim-graph`: Semantic graph-based ranking (requires KG)
- `title-scorer`: Basic text matching
- `bm25`: Classic information retrieval
- `bm25f`: BM25 with field boosting
- `bm25plus`: Enhanced BM25

### Available Themes
From desktop and default configs:
- spacelab, cosmo, lumen, darkly, united, journal, readable, pulse, superhero, default

### Sane Defaults (from terraphim_server/default/)

| Config File | Primary Role | Use Case |
|-------------|--------------|----------|
| terraphim_engineer_config.json | Terraphim Engineer | Knowledge graph + local docs |
| rust_engineer_config.json | Rust Engineer | QueryRs for Rust docs |
| ollama_llama_config.json | Multiple agents | Local Ollama LLM |
| system_operator_config.json | System Operator | DevOps/sysadmin tasks |
| quickwit_engineer_config.json | Quickwit Engineer | Log analysis |
| ai_engineer_config.json | AI Engineer | AI/ML development |
| python_engineer_config.json | Python Engineer | Python development |
| frontend_engineer_config.json | Frontend Engineer | Svelte/web development |
| devops_cicd_config.json | DevOps Engineer | CI/CD and infrastructure |

### Integration Points
- **dialoguer**: Interactive CLI prompts (Select, MultiSelect, Input, Confirm)
- **indicatif**: Progress bars and spinners
- **console**: Styled terminal output
- **terraphim_persistence**: Save/load configuration
- **terraphim_config::ConfigBuilder**: Programmatic config construction

## Constraints

### Technical Constraints
- Must work in headless/TTY environments
- No GUI dependencies
- Cross-platform (Linux, macOS, Windows)
- Must integrate with existing clap CLI structure
- Configuration must be compatible with desktop app

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| First-run detection | < 100ms | N/A |
| Wizard completion | < 2 min | N/A |
| Config save | < 500ms | ~200ms |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_config | Core config structures | Low |
| terraphim_persistence | Save/load config | Low |
| terraphim_types | Shared types | Low |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| dialoguer | 0.11+ | Low | inquire crate |
| indicatif | existing | Low | N/A |
| console | existing | Low | N/A |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Complex nested configuration | Medium | Medium | Break into simple steps |
| LLM provider auth complexity | Medium | Low | Skip LLM setup initially |
| Service connectivity testing | Low | Medium | Make async, show progress |

### Open Questions
1. Should wizard persist completion status to avoid re-prompting? - **Recommend: Yes, use config presence**
2. What is the minimum configuration for a working setup? - **Answer: One role with one haystack**
3. Should wizard be skippable with `--no-setup` flag? - **Recommend: Yes, and also `--setup` to force**

### Assumptions
1. Users have terminal with TTY support (fallback to non-interactive mode)
2. dialoguer crate provides adequate UX for complex forms
3. Default templates cover most common use cases

## Research Findings

### Key Insights

1. **Additive Configuration**: Desktop wizard allows adding roles to existing config. CLI must support both:
   - Adding new roles to existing config
   - Creating fresh config from templates
   - Modifying existing roles

2. **Template System**: 18+ pre-built configurations exist in `terraphim_server/default/`. These should be selectable templates.

3. **LLM Integration Patterns**:
   - Ollama (local): requires base_url + model name
   - OpenRouter (cloud): requires API key + model selection
   - Both support auto-summarization and chat

4. **Knowledge Graph Options**:
   - Remote URL (pre-built automata)
   - Local markdown files (build at startup)
   - None (disable KG features)

5. **Haystack Service Complexity**: Each service type has different required/optional parameters

### Wizard Flow Design

**Option A: Step-by-Step (Desktop Parity)**
```
Step 1: Setup Mode
  - [x] Add role to existing config
  - [ ] Start from template
  - [ ] Create new config from scratch

Step 2: Role Configuration
  - Name: [________]
  - Shortname: [________]
  - Theme: [spacelab v]
  - Relevance: [title-scorer v]

Step 3: Haystacks
  - Add haystack...
    - Service: [Ripgrep v]
    - Location: [~/documents]

Step 4: LLM (Optional)
  - Provider: [Ollama v]
  - Model: [llama3.2:3b]

Step 5: Review & Save
```

**Option B: Quick Start + Advanced**
```
Quick Start:
  1. Rust Developer (QueryRs + local docs)
  2. Local Notes (Ripgrep + local folder)
  3. AI Engineer (Ollama + KG)
  4. Custom setup...
```

**Recommendation**: Implement Option B as primary flow, with Option A accessible via "Custom setup"

### CLI Command Design

```bash
# First run - auto-detect and prompt
terraphim-agent

# Force setup wizard
terraphim-agent setup

# Skip setup, use defaults
terraphim-agent --no-setup

# Quick start with template
terraphim-agent setup --template rust-engineer

# Add role to existing config
terraphim-agent setup --add-role

# List available templates
terraphim-agent setup --list-templates
```

## Recommendations

### Proceed/No-Proceed
**Proceed** - The CLI onboarding wizard fills a clear gap and has well-defined data models.

### Scope Recommendations
1. **Phase 1**: Quick start templates + basic role addition
2. **Phase 2**: Full custom wizard with all options
3. **Phase 3**: Service connectivity testing and validation

### Implementation Approach
1. Add `dialoguer` and `console` dependencies
2. Create `crates/terraphim_agent/src/onboarding/` module
3. Implement template loading from embedded JSON
4. Implement interactive prompts for custom setup
5. Add `setup` subcommand to clap CLI
6. Add first-run detection logic

## Next Steps

If approved:
1. Create design document with detailed UI mockups
2. Add dependencies to Cargo.toml
3. Implement onboarding module structure
4. Create embedded templates from best default configs
5. Implement step-by-step wizard flow
6. Add integration tests

## Appendix

### Reference Materials
- Desktop Wizard: `desktop/src/lib/ConfigWizard.svelte`
- Generated Types: `desktop/src/lib/generated/types.ts`
- Default Configs: `terraphim_server/default/`
- dialoguer docs: https://docs.rs/dialoguer

### Embedded Template Candidates
Best templates to embed for quick start:
1. `terraphim_engineer_config.json` - Full-featured with KG
2. `rust_engineer_config.json` - Rust developer quick start
3. `ollama_llama_config.json` - Local LLM setup
4. `default_role_config.json` - Minimal baseline
