# Research Document: PR #502 - plan/kg dynamic routing

## 1. Problem Restatement and Scope

**IN SCOPE:**
This PR introduces a multi-faceted change to the Terraphim AI system focused on improving the developer experience and enabling dynamic knowledge graph routing capabilities. The changes span 47 files with +860/-192 lines across 5 commits:

1. **Agent REPL Default Mode** - Changes the default behavior of `terraphim-agent` from TUI to REPL mode when run without arguments, requiring explicit `--tui` flag for TUI mode
2. **CI/Clippy Fixes** - Addresses compiler warnings, removes deprecated RocksDB tests causing `unexpected_cfgs` errors
3. **Document Directives** - Adds new fields to the `Document` type for enhanced metadata (doc_type, synonyms, route, priority)
4. **Markdown Directives Parser** - New module to parse markdown frontmatter-style directives from knowledge graph source files
5. **Test Settings Ignore** - Adds test settings file to .gitignore

**OUT OF SCOPE:**
- Full TUI mode removal (still available via `--tui` flag)
- Complete RocksDB feature removal (just tests removed)
- LLM proxy integration (handled in separate PR #61)

## 2. User & Business Outcomes

**User-Visible Changes:**
1. **Simplified Agent Usage**: Users running `terraphim-agent` without arguments will now get REPL mode instead of TUI, reducing the "server required" friction for quick interactions
2. **Explicit TUI Mode**: Users wanting TUI must now use `terraphim-agent --tui`, making the dependency on a running server explicit
3. **Enhanced Document Metadata**: Documents can now specify type (KgEntry, Document, ConfigDocument), synonyms for search, routing preferences, and priority scores
4. **Markdown-Driven Configuration**: Knowledge graph source files can include directives like `type::: config_document`, `route:: openai, gpt-4o`, `synonyms:: alias1, alias2`, `priority:: 80`

**Business Value:**
- Lower barrier to entry for CLI users (REPL is lighter weight)
- Foundation for intelligent LLM routing based on document metadata
- More flexible knowledge graph organization with typed documents

## 3. System Elements and Dependencies

### Affected Components:

| Component | Files Changed | Role/Responsibility | Dependencies |
|-----------|---------------|---------------------|--------------|
| **terraphim_agent** | main.rs, repl/handler.rs, repl/mcp_tools.rs, tests/ | CLI entry point, REPL/TUI mode handling | terraphim_types, terraphim_config |
| **terraphim_types** | src/lib.rs | Core type definitions including new Document fields and routing types | serde, ahash |
| **terraphim_automata** | src/lib.rs, src/markdown_directives.rs (NEW) | Markdown directive parsing, automata operations | walkdir, terraphim_types |
| **terraphim_middleware** | haystack/*.rs, indexer/ripgrep.rs, tests/ | Document creation across all haystack sources | terraphim_types |
| **terraphim_persistence** | src/settings.rs, src/thesaurus.rs | Storage layer, removed RocksDB tests | opendal |
| **terraphim_markdown_parser** | src/lib.rs | Markdown to Document conversion | terraphim_types |

### New Dependencies:
- `walkdir = "2.5"` added to `terraphim_automata/Cargo.toml` for directory traversal

### Cross-Cutting Concerns:
- **Document Type System**: All Document instantiations across the codebase must now include `doc_type`, `synonyms`, `route`, `priority` fields
- **Backward Compatibility**: Default values provided (DocumentType::KgEntry, None for optional fields)

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication for Solution |
|------------|----------------|-------------------------|
| **TUI requires server at localhost:8000** | TUI mode needs API connection | REPL as default removes this implicit dependency; users must explicitly opt-in to TUI |
| **RocksDB deprecated** | Tests causing `unexpected_cfgs` compiler errors | Tests removed; feature may still exist but unmaintained |
| **Document fields must be serializable** | Documents stored and transmitted via APIs | All new fields implement Serialize/Deserialize; optional fields use Option<T> |
| **Priority range 0-100** | Routing priority scoring system | Validation in markdown parser ensures bounds; clamping in Priority::new() |
| **Markdown directives are line-based** | Simple parsing approach | Directives must appear at start of lines; no inline directives |

## 5. Risks, Unknowns, and Assumptions

### Risks:

| Risk | Severity | De-risking Strategy |
|------|----------|---------------------|
| **Breaking change**: Users expecting TUI by default will get REPL | Medium | Clear documentation, help text updated, version bump indication |
| **Document field migration**: Existing persisted documents lack new fields | Medium | Default values in struct definition handle missing fields via serde |
| **Route directive format**: `provider, model` split on comma may be fragile | Low | Clear error messages in parser; documentation of expected format |
| **Priority bounds violation**: Values >100 or invalid parsing | Low | Validation in parser (warns and ignores); clamping in Priority type |

### Unknowns:
- **ASSUMPTION**: All haystack sources (ClickUp, Perplexity, QueryRs, Quickwit, Ripgrep) should default to `DocumentType::KgEntry`
- **ASSUMPTION**: Markdown directives should be case-insensitive (implemented via `to_ascii_lowercase()`)
- **ASSUMPTION**: Multiple synonyms declarations should be cumulative (implemented: pushes all to vec)
- **UNKNOWN**: How will the new `route` and `priority` fields be consumed by the routing system? (Not implemented in this PR)

### Technical Debt Introduced:
- **TODO**: `McpToolsHandler` marked with `#[allow(dead_code)]` - feature not currently used
- **TODO**: Route directive validation only checks provider/model non-empty, doesn't validate against allowed values

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity:
1. **47 files changed** - Broad surface area for a single PR
2. **5 distinct features** bundled together (REPL default, CI fixes, directives, parser, gitignore)
3. **Document type proliferation** - 4 new fields added to core Document type affecting all instantiations
4. **Routing system groundwork** - New types (RoutingRule, RoutingDecision, etc.) added but not yet integrated

### Simplification Opportunities:
1. **Could have split**: REPL default change could be separate PR - it's a behavioral change deserving its own review
2. **Could have split**: CI fixes could be separate PR - they block compilation but are orthogonal
3. **Good bundling**: Document directives + markdown parser are tightly coupled and belong together
4. **Future simplification**: Consider derive macro for Document builder pattern to reduce instantiation boilerplate

## 7. Questions for Human Reviewer

1. ~~Is the REPL default change acceptable for the next release?~~ **RESOLVED: Yes, acceptable**

2. **Should we add a migration guide or deprecation warning for TUI default?** Consider users with scripts expecting TUI mode.

3. **Is DocumentType::KgEntry the correct default for ALL haystack sources?** Some sources (like config files) might warrant different defaults.

4. **Should route directive validation be stricter?** Currently only checks non-empty; should it validate against known providers?

5. ~~How will the routing fields (route, priority) be consumed?~~ **RESOLVED: Consumed by terraphim-llm-proxy**

6. **Is removing RocksDB tests sufficient, or should the feature be fully removed?** The tests were causing issues, but the feature code remains.

7. **Should markdown directives support inline/format variations?** Current implementation is strict: `directive:: value` at line start only.

8. **Is 0-100 the right priority range?** Some systems use 1-10 or 1-5; 0-100 provides granularity but may be overkill.

9. **Should we add integration tests for the markdown directives parser with real markdown files?** Current tests use tempdir with synthetic content.

10. **Is the `--tui` flag name intuitive?** Alternative could be `--gui` or `--interactive-tui` for clarity.
