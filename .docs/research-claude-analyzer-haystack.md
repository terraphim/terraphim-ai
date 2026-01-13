# Research Document: Claude Log Analyzer as Terraphim Haystack

## 1. Problem Restatement and Scope

**Problem**: The `terraphim-session-analyzer` crate provides rich analysis of Claude Code session logs but is currently not integrated into Terraphim's search infrastructure. Users cannot search across their Claude session history to find past conversations, agent decisions, or file modifications.

**IN Scope**:
- Implement `terraphim-session-analyzer` as a searchable haystack in Terraphim
- Index session metadata, agent invocations, file operations, and tool usage
- Use `terraphim_automata` for efficient text matching and concept extraction
- Follow existing haystack patterns (Ripgrep, MCP, QueryRs)

**OUT of Scope**:
- Real-time session monitoring (watch mode)
- Modifying Claude Code's logging format
- Storing session data in external databases

## 2. User & Business Outcomes

Users will be able to:
- Search across all Claude Code sessions: "When did I implement the login feature?"
- Find which agent was used for specific tasks: "Show all architect agent invocations"
- Track file modification history: "Which agents modified config.rs?"
- Discover patterns in their development workflow

## 3. System Elements and Dependencies

### Haystack Architecture

| Component | Location | Role |
|-----------|----------|------|
| `HaystackProvider` trait | `haystack_core/src/lib.rs` | Base trait for search providers |
| `IndexMiddleware` trait | `terraphim_middleware/src/indexer/mod.rs` | Index haystack and return Documents |
| `ServiceType` enum | `terraphim_config/src/lib.rs` | Registry of available haystack types |
| `search_haystacks()` | `terraphim_middleware/src/indexer/mod.rs` | Orchestrates search across haystacks |
| `Document` type | `terraphim_types/src/lib.rs` | Standard document format for indexing |

### Claude Log Analyzer Data Model

| Data Type | Fields | Searchable Content |
|-----------|--------|-------------------|
| `SessionAnalysis` | session_id, project_path, start/end_time, duration | Session metadata |
| `AgentInvocation` | agent_type, task_description, prompt, files_modified | Agent decisions, prompts |
| `FileOperation` | file_path, operation, agent_context | File modification history |
| `ToolInvocation` | tool_name, category, command_line, arguments | Tool usage patterns |
| `CollaborationPattern` | pattern_type, agents, description | Agent collaboration |

### Dependencies

```
terraphim_middleware
├── haystack/
│   ├── mod.rs (add ClaudeLogAnalyzerHaystackIndexer)
│   └── claude_analyzer.rs (new)
└── indexer/mod.rs (add ServiceType::ClaudeLogAnalyzer handling)

terraphim_config
└── lib.rs (add ClaudeLogAnalyzer to ServiceType enum)

terraphim-session-analyzer
└── (no changes - use as library)
```

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| Session files are JSONL | Line-by-line parsing required | Stream processing, not full file load |
| Sessions can be large (100MB+) | Memory constraints | Index incrementally, cache results |
| File paths are encoded | `-home-alex-projects-` format | Need to decode for display |
| Timestamps are ISO 8601 | Consistent parsing | Use jiff for parsing |
| Sessions are read-only | Cannot modify source files | Mark haystack as `read_only: true` |

## 5. Risks, Unknowns, and Assumptions

### Unknowns
- **U1**: How large are typical user session directories? (Need profiling)
- **U2**: What query patterns will users use most? (Affects indexing strategy)
- **U3**: Should we index full prompts or just task descriptions? (Content vs metadata)

### Assumptions
- **A1**: Session files follow Claude Code JSONL format (validated by existing parser)
- **A2**: Users have read access to `~/.claude/projects/` directory
- **A3**: Session IDs are UUIDs and unique across all projects

### Risks

| Risk | De-risking Strategy | Residual |
|------|---------------------|----------|
| Large session directories slow down search | Implement caching, limit scan depth | Some initial slowness |
| Memory usage for large sessions | Stream parsing, don't load all into memory | Moderate |
| Stale cache after new sessions | Add file watcher or cache invalidation | Manual refresh needed |

## 6. Context Complexity vs. Simplicity Opportunities

### Complexity Sources
1. Multiple data types to index (sessions, agents, files, tools)
2. Nested JSON structure in JSONL files
3. Encoded project paths need decoding

### Simplification Strategies

1. **Single Document Type**: Map all searchable content to `Document` type
   - `id`: session_id + entry_uuid
   - `title`: agent_type or task_description
   - `body`: prompt + command details
   - `url`: file path or session path
   - `tags`: agent_type, tool_category

2. **Follow Existing Patterns**: MCP and QueryRs haystacks show the pattern:
   ```rust
   pub struct ClaudeLogAnalyzerHaystackIndexer;

   impl IndexMiddleware for ClaudeLogAnalyzerHaystackIndexer {
       fn index(&self, needle: &str, haystack: &Haystack) -> impl Future<Output = Result<Index>>
   }
   ```

3. **Use Existing Parser**: `terraphim-session-analyzer` already parses sessions perfectly

## 7. Questions for Human Reviewer

1. **Search Scope**: Should we search only agent invocations, or include user messages too?

2. **Indexing Depth**: Index just session metadata (fast) or full prompts/commands (comprehensive)?

3. **Default Location**: Use `~/.claude/projects/` by default, or require explicit path in haystack config?

4. **Caching Strategy**: Should we persist an index between Terraphim restarts, or rebuild each time?

5. **terraphim_automata Usage**: Use for:
   - Autocomplete on agent types/tool names?
   - Fuzzy matching on file paths?
   - Building thesaurus from session content?

6. **Document Structure**: Map 1:1 (one document per session) or 1:N (one per agent invocation)?

## 8. Implementation Pattern (from MCP Haystack)

```rust
// terraphim_middleware/src/haystack/claude_analyzer.rs
use crate::{indexer::IndexMiddleware, Result};
use terraphim_config::Haystack;
use terraphim_types::{Document, Index};
use terraphim_session_analyzer::{Analyzer, SessionAnalysis};

pub struct ClaudeLogAnalyzerHaystackIndexer;

#[async_trait::async_trait]
impl IndexMiddleware for ClaudeLogAnalyzerHaystackIndexer {
    fn index(
        &self,
        needle: &str,
        haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        async move {
            // 1. Get session directory from haystack.location
            let session_dir = expand_path(&haystack.location);

            // 2. Parse sessions using terraphim-session-analyzer
            let analyzer = Analyzer::from_directory(&session_dir)?;
            let analyses = analyzer.analyze(None)?;

            // 3. Convert to Documents and filter by needle
            let mut index = Index::new();
            for analysis in analyses {
                for agent in &analysis.agents {
                    if matches_needle(needle, &agent, &analysis) {
                        let doc = agent_to_document(&agent, &analysis);
                        index.insert(doc.id.clone(), doc);
                    }
                }
            }

            Ok(index)
        }
    }
}

fn agent_to_document(agent: &AgentInvocation, session: &SessionAnalysis) -> Document {
    Document {
        id: format!("{}:{}", session.session_id, agent.parent_message_id),
        title: format!("[{}] {}", agent.agent_type, agent.task_description),
        url: session.project_path.clone(),
        body: agent.prompt.clone(),
        description: Some(agent.task_description.chars().take(180).collect()),
        tags: vec![agent.agent_type.clone()],
        ..Document::default()
    }
}
```

## 9. Next Steps

1. Add `ClaudeLogAnalyzer` to `ServiceType` enum in `terraphim_config`
2. Create `claude_analyzer.rs` in `terraphim_middleware/src/haystack/`
3. Add dependency on `terraphim-session-analyzer` crate
4. Implement `IndexMiddleware` following MCP pattern
5. Add to `search_haystacks()` match statement
6. Write integration tests
7. Add example configuration to default configs
