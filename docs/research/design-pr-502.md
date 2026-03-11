# Design & Implementation Plan Review: PR #502

## 1. Summary of Target Behavior

After this PR, the Terraphim AI system will have:

1. **User-Friendly Agent Defaults**: The `terraphim-agent` CLI defaults to REPL mode (lightweight, no server required) instead of TUI mode (requires server at localhost:8000). TUI mode is accessible via explicit `--tui` flag.

2. **Clean Compilation**: All clippy warnings resolved, deprecated RocksDB tests removed that were causing `unexpected_cfgs` compiler errors.

3. **Rich Document Metadata**: The `Document` type includes four new optional fields:
   - `doc_type`: Classification (KgEntry, Document, ConfigDocument)
   - `synonyms`: Alternative names for search matching
   - `route`: LLM routing preferences (provider, model)
   - `priority`: Routing priority score (0-100)

4. **Markdown-Driven Configuration**: A new parser in `terraphim_automata` reads markdown files and extracts directives:
   - `type::: <kg_entry|document|config_document>`
   - `synonyms:: alias1, alias2`
   - `route:: <provider>, <model>`
   - `priority:: <0-100>`

5. **Ignored Test Artifacts**: Test settings file auto-added to `.gitignore` to prevent accidental commits.

## 2. Key Invariants and Acceptance Criteria

### Invariants:

| Invariant | Rationale | Verification |
|-----------|-----------|--------------|
| All Documents have valid DocumentType | Type system enforcement | Unit test: Document serialization/deserialization |
| Priority values clamped to 0-100 | Prevent invalid routing scores | Unit test: Priority::new() bounds checking |
| Markdown directives case-insensitive | User experience | Unit test: mixed-case directive parsing |
| REPL mode works without server | Core requirement | Integration test: repl::run_repl_offline_mode() |
| TUI mode requires explicit opt-in | Clear dependency communication | Manual test: --tui flag behavior |
| All haystack sources populate new Document fields | Data consistency | Compile-time: type system enforces field presence |

### Acceptance Criteria:

1. Running `terraphim-agent` without arguments starts REPL mode
2. Running `terraphim-agent --tui` starts TUI mode (requires server)
3. `cargo clippy --workspace` passes without warnings
4. Markdown file with `route:: openai, gpt-4o` parses correctly
5. Document with `priority:: 150` warns and ignores invalid value
6. All existing tests pass with new Document field defaults

## 3. High-Level Design and Boundaries

### Architecture Overview:

```
┌─────────────────────────────────────────────────────────────────┐
│                     terraphim_agent (CLI)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ main.rs      │  │ REPL Mode    │  │ TUI Mode (--tui)     │  │
│  │ Entry point  │  │ (default)    │  │ (requires server)    │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     terraphim_types                             │
│  ┌────────────────────────────────────────────────────────────┐│
│  │ Document (enhanced)                                        ││
│  │ - doc_type: DocumentType                                   ││
│  │ - synonyms: Vec<String>                                    ││
│  │ - route: Option<RouteDirective>                            ││
│  │ - priority: Option<u8>                                     ││
│  └────────────────────────────────────────────────────────────┘│
│  ┌────────────────────────────────────────────────────────────┐│
│  │ Routing Types (new)                                        ││
│  │ - RoutingRule, RoutingDecision, Priority                   ││
│  │ - RoutingScenario enum                                     ││
│  └────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  terraphim_automata                             │
│  ┌────────────────────────────────────────────────────────────┐│
│  │ markdown_directives.rs (NEW)                               ││
│  │ - parse_markdown_directives_dir()                          ││
│  │ - parse_markdown_directives_content()                      ││
│  │ - Directive parsing with warning collection                ││
│  └────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              terraphim_middleware (haystacks)                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐│
│  │ ClickUp  │ │Perplexity│ │ QueryRs  │ │ Quickwit │ │Ripgrep││
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └────────┘│
│              All populate new Document fields                   │
└─────────────────────────────────────────────────────────────────┘
```

### Component Boundaries:

**Inside Existing Components:**
- `terraphim_agent`: Modified argument parsing and mode selection logic
- `terraphim_types`: Extended Document struct, added routing types
- All haystack indexers: Updated Document instantiation sites

**New Components:**
- `terraphim_automata::markdown_directives`: Self-contained parsing module
  - Clear input: directory path or markdown content string
  - Clear output: HashMap<concept_name, MarkdownDirectives> + warnings
  - No dependencies on other terraphim modules except types

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `terraphim_agent/src/main.rs` | Modify | TUI default when no command | REPL default; `--tui` flag for TUI | atty crate for TTY detection |
| `terraphim_agent/src/repl/handler.rs` | Modify | Document without new fields | Document with doc_type=KgEntry | terraphim_types |
| `terraphim_agent/src/repl/mcp_tools.rs` | Modify | Clippy warnings | Clean; `#[allow(dead_code)]` on unused handler | - |
| `terraphim_types/src/lib.rs` | Modify | Document {id, url, title...} | Document + doc_type, synonyms, route, priority | serde |
| `terraphim_types/src/lib.rs` | Add | No routing types | RoutingRule, RoutingDecision, Priority, RoutingScenario | chrono, serde |
| `terraphim_automata/src/markdown_directives.rs` | Create | (new file) | Full directive parsing implementation | walkdir, terraphim_types |
| `terraphim_automata/src/lib.rs` | Modify | No markdown_directives module | `pub mod markdown_directives` + re-exports | markdown_directives |
| `terraphim_automata/Cargo.toml` | Modify | No walkdir dep | `walkdir = "2.5"` | - |
| `terraphim_middleware/src/haystack/clickup.rs` | Modify | Document instantiation | Document with new field defaults | terraphim_types |
| `terraphim_middleware/src/haystack/perplexity.rs` | Modify | Document instantiation | Document with new field defaults | terraphim_types |
| `terraphim_middleware/src/haystack/query_rs.rs` | Modify | Document instantiation (5 sites) | Document with new field defaults | terraphim_types |
| `terraphim_middleware/src/haystack/quickwit.rs` | Modify | Document instantiation (2 sites) | Document with new field defaults | terraphim_types |
| `terraphim_middleware/src/indexer/ripgrep.rs` | Modify | Document instantiation | Document with new field defaults | terraphim_types |
| `terraphim_markdown_parser/src/lib.rs` | Modify | Document instantiation | Document with doc_type=KgEntry | terraphim_types |
| `terraphim_persistence/src/settings.rs` | Modify | RocksDB tests present | RocksDB tests removed | - |
| `terraphim_persistence/src/thesaurus.rs` | Modify | RocksDB tests present | RocksDB tests removed | - |
| `terraphim_update/src/lib.rs` | Modify | needless_borrows warnings | Fixed clippy warnings | - |
| `terraphim_agent/tests/*.rs` | Modify | Document test fixtures | Document with new fields | terraphim_types |
| Multiple test files | Modify | Document test fixtures (20+ sites) | Document with new fields | terraphim_types |
| `.gitignore` | Modify | Standard ignores | + `crates/terraphim_settings/test_settings/settings.toml` | - |
| `AGENTS.md` | Modify | UBS documentation | + `Use 'bd' for task tracking` | - |

## 5. Step-by-Step Implementation Sequence

### Step 1: CI/Clippy Fixes (Foundation)
**Purpose**: Clean compilation baseline
**Changes**:
- Fix clippy warnings in terraphim_update, terraphim-session-analyzer, terraphim_agent
- Remove deprecated RocksDB tests
**Deployable**: Yes - no functional changes
**Risk**: Low

### Step 2: Document Type Enhancement (Data Model)
**Purpose**: Extend core Document type with new fields
**Changes**:
- Add DocumentType enum (KgEntry, Document, ConfigDocument)
- Add MarkdownDirectives and RouteDirective types
- Extend Document struct with 4 new fields
- Add Priority and routing types (RoutingRule, RoutingDecision, PatternMatch)
**Deployable**: Yes - backward compatible with defaults
**Risk**: Medium - affects all Document instantiations

### Step 3: Update All Document Instantiations (Propagation)
**Purpose**: Ensure all code creates Documents with new fields
**Changes**:
- Update all 25+ Document instantiation sites across haystacks, parsers, tests
- Set appropriate defaults (doc_type=KgEntry, None for optional)
**Deployable**: Yes - required for compilation
**Risk**: Medium - high surface area, risk of missing sites

### Step 4: Markdown Directives Parser (New Feature)
**Purpose**: Parse markdown files for configuration directives
**Changes**:
- Create markdown_directives.rs with parsing logic
- Add walkdir dependency
- Export public API from automata/lib.rs
- Unit tests for parsing scenarios
**Deployable**: Yes - additive feature
**Risk**: Low - isolated module

### Step 5: Agent CLI Default Change (User-Facing)
**Purpose**: Change default mode from TUI to REPL
**Changes**:
- Add `--tui` flag to CLI
- Swap default behavior in main.rs match statement
- Update help text and usage info
**Deployable**: Yes - behavioral change
**Risk**: Medium - user-visible breaking change

### Step 6: Cleanup (Polish)
**Purpose**: Repository hygiene
**Changes**:
- Add test settings to .gitignore
- Update AGENTS.md
**Deployable**: Yes
**Risk**: None

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| REPL starts by default | Integration | `terraphim_agent/tests/integration_test.rs` (new) |
| TUI starts with --tui flag | Integration | `terraphim_agent/tests/integration_test.rs` (new) |
| Document serializes with new fields | Unit | `terraphim_types/src/lib.rs` (existing test module) |
| Document deserializes missing fields gracefully | Unit | `terraphim_types/src/lib.rs` |
| Priority clamps to 0-100 | Unit | `terraphim_types/src/lib.rs` |
| Markdown directive parsing | Unit | `terraphim_automata/src/markdown_directives.rs` |
| Invalid directive warnings | Unit | `terraphim_automata/src/markdown_directives.rs` |
| Clippy passes | CI | `.github/workflows/ci.yml` |
| All existing tests pass | Integration | `cargo test --workspace` |

### Test Gaps Identified:
1. **No integration test** for CLI mode switching (REPL vs TUI)
2. **No E2E test** for markdown directive parsing with real files
3. **No test** for Document migration (old JSON → new struct)

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| User confusion from TUI→REPL default | Clear documentation, help text, changelog | Low - discoverable via --help |
| Document field migration issues | Serde default values, comprehensive test coverage | Low - defaults handle missing fields |
| Route directive format fragility | Clear error messages, documented format | Low - simple comma-split pattern |
| Priority bounds violations | Parser validation + type clamping | Very Low - double protection |
| Missing Document instantiation sites | Compiler enforcement (type system) | Very Low - won't compile if missed |
| RocksDB removal impacts | Feature still exists, just tests removed | Low - gradual deprecation |
| PR scope creep (47 files) | Clear separation of concerns in commits | Medium - should have been 3 PRs |

### Complexity Assessment:
- **Syntactic complexity**: Medium - many files touched but changes are repetitive
- **Semantic complexity**: Low - straightforward type additions and CLI change
- **Behavioral complexity**: Medium - user-facing default change needs communication

## 8. Open Questions / Decisions for Human Review

1. ~~Should the routing types (RoutingRule, RoutingDecision) be in a separate PR?~~ **RESOLVED: Types are consumed by terraphim-llm-proxy, appropriately bundled**

2. **Is the DocumentType enum naming clear?**
   - `KgEntry` vs `KnowledgeGraphEntry` (more explicit but longer)
   - `ConfigDocument` vs `Configuration` (clearer purpose?)

3. **Should we feature-gate the new Document fields?**
   Behind a `"document-directives"` feature to reduce compile time for users not using this?

4. **Is the priority range (0-100) appropriate?**
   HTTP status codes (0-999), syslog (0-7), custom range?

5. **Should markdown directives support nesting or sections?**
   Current: flat file-level directives only
   Alternative: YAML frontmatter style with structured sections?

## Design Quality Assessment

### Strengths:
1. **Clear separation**: Markdown parser is self-contained module
2. **Backward compatibility**: Serde defaults handle migration gracefully
3. **Type safety**: DocumentType enum prevents invalid types
4. **Validation**: Parser validates and warns on invalid directives
5. **Test coverage**: Unit tests for parser scenarios included

### Concerns:
1. **Scope**: 47 files in one PR is large; could have been 3 separate PRs
2. **Unused code**: Routing types defined but not yet consumed
3. **Breaking change**: TUI default change may surprise users
4. **Validation gap**: Route directive doesn't validate provider/model against allowed values

### Recommendations:
1. ~~Split future PRs~~: Acceptable as-is since routing types are consumed by llm-proxy
2. **Add integration tests**: For CLI mode switching and markdown parsing
3. **Document migration**: Add guide for users upgrading from previous versions (REPL default change)
4. ~~Consider feature flags~~: Routing system is actively consumed by llm-proxy
