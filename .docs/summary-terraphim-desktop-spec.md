# Summary: Terraphim Desktop Technical Specification

**File**: `docs/specifications/terraphim-desktop-spec.md`
**Type**: Technical Specification Document
**Version**: 1.0.0
**Size**: ~12,000 words, 16 major sections
**Last Updated**: 2025-11-24

## Document Purpose

Comprehensive technical specification for the Terraphim Desktop application, serving as the authoritative reference for architecture, features, implementation details, testing, and deployment.

## Key Sections Overview

### 1. Executive Summary
- **Privacy-first** AI assistant with local execution
- **Multi-source search** across personal, team, and public knowledge
- **Semantic understanding** via knowledge graphs
- **Native performance** with Tauri + Svelte

### 2. System Architecture

**Technology Stack**:
- Frontend: Svelte 5.2.8 + TypeScript + Vite 5.3.4
- UI: Bulma CSS 1.0.4 (22 themes)
- Desktop: Tauri 2.9.4 (Rust-based)
- Backend: 29+ Rust crates (terraphim_service, terraphim_middleware, etc.)
- Rich Text: Novel Svelte + TipTap
- Visualization: D3.js 7.9.0

**Component Architecture**:
```
Frontend (Svelte + TypeScript)
  ↓ Tauri IPC Layer
Backend Services (Rust)
  ↓ Data Sources
9+ Haystack Integrations
  ↓ External Integrations
MCP, Ollama, 1Password CLI
```

### 3. Core Features

#### Semantic Search
- Real-time autocomplete from knowledge graph
- Multi-haystack parallel search
- Configurable relevance ranking (TitleScorer, BM25, TerraphimGraph)
- Logical operators (AND, OR, NOT, quotes)
- Tag filtering

#### Knowledge Graph
- D3.js force-directed visualization
- Thesaurus-based concept relationships
- Document associations per concept
- Path finding between terms
- Automata for fast text matching

#### AI Chat
- Conversation management (create, list, switch, persist)
- Context management (add/edit/delete)
- Search integration (add results as context)
- KG integration (add terms/indices as context)
- Novel editor with MCP autocomplete
- Streaming LLM responses
- Session persistence and statistics

#### Role-Based Configuration
- User profiles with domain-specific settings
- Per-role haystacks and relevance functions
- Per-role knowledge graphs
- Theme customization
- LLM provider settings (Ollama/OpenRouter)

#### Multi-Source Integration (9+ Haystacks)
- **Ripgrep**: Local filesystem search
- **MCP**: Model Context Protocol for AI tools
- **Atomic Server**: Atomic Data protocol
- **ClickUp**: Task management integration
- **Logseq**: Personal knowledge management
- **QueryRs**: Rust docs + Reddit
- **Atlassian**: Confluence/Jira
- **Discourse**: Forum integration
- **JMAP**: Email integration

#### Native Desktop Features
- System tray with role switching
- Global keyboard shortcuts
- Auto-update from GitHub releases
- Window management (show/hide/minimize)
- Bundled content initialization

### 4. User Interface Specification

#### Main Layout
- Top navigation: Search, Chat, Graph tabs
- Logo back button
- Theme switcher (22 themes)
- Responsive design (desktop-focused)

#### Search Page
- KGSearchInput with autocomplete
- ResultItem display with tags
- ArticleModal for full content
- Atomic Server save integration

#### Chat Page
- Collapsible session list sidebar
- Context management panel (3+ types)
- Message display with markdown rendering
- Novel editor for composition
- Role selection dropdown

#### Graph Page
- Force-directed D3.js visualization
- Interactive nodes and edges
- Zoom/pan controls
- Node selection and focus

#### Configuration Pages
- Visual wizard for role setup
- JSON editor with schema validation
- Import/export functionality

### 5. Backend Integration

#### Tauri Commands (30+)
**Search**: `search`, `search_kg_terms`, `get_autocomplete_suggestions`
**Config**: `get_config`, `update_config`, `select_role`, `get_config_schema`
**KG**: `get_rolegraph`, `find_documents_for_kg_term`, `add_kg_term_context`
**Chat**: `chat`, `create_conversation`, `list_conversations`, `add_message_to_conversation`
**Persistent**: `create_persistent_conversation`, `list_persistent_conversations`, `delete_persistent_conversation`
**Integration**: `onepassword_status`, `onepassword_resolve_secret`, `publish_thesaurus`

#### Service Layer
- **TerraphimService**: High-level orchestration
- **SearchService**: Multi-haystack coordination
- **RoleGraphService**: Knowledge graph management
- **AutocompleteService**: Real-time suggestions
- **LLM Service**: Ollama/OpenRouter integration

#### Persistence Layer
- Multiple backends: Memory, SQLite, RocksDB, Atomic Data, Redb
- Persistable trait for save/load/delete operations
- Configuration, thesaurus, conversations, documents

### 6. Data Models

**Core Types**: Config, Role, Haystack, Document, SearchQuery
**Chat Models**: Conversation, Message, ContextItem, ConversationSummary, ConversationStatistics
**KG Models**: KnowledgeGraph, KGNode, KGEdge, KGTermDefinition

### 7. Configuration System

#### Load Priority
1. Environment variables
2. Saved configuration from persistence
3. Default desktop configuration
4. Fallback minimal configuration

#### Secret Management
- 1Password CLI integration
- Secret references: `op://vault/item/field`
- Automatic resolution on config load
- Memory-only caching

### 8. Testing Strategy

#### Test Pyramid
- **Unit Tests**: >85% frontend, >90% backend coverage
- **Integration Tests**: Cross-crate functionality, service tests
- **E2E Tests**: 50+ Playwright specs covering major workflows
- **Visual Regression**: Theme consistency across 22 themes
- **Performance Tests**: Vitest benchmarks for response times

#### Test Categories
- Component rendering and interaction
- Store mutations and state management
- Command handlers and IPC
- Search functionality and operators
- Chat workflows and context management
- Knowledge graph operations
- Configuration wizards
- Atomic server integration
- Ollama/LLM integration

### 9. Performance Requirements

| Operation | Target | Maximum |
|-----------|--------|---------|
| Autocomplete | <50ms | 100ms |
| Search (single) | <200ms | 500ms |
| Search (multi) | <500ms | 1000ms |
| KG load | <1s | 2s |
| Theme switch | <100ms | 200ms |

**Resource Limits**:
- Memory: 200MB baseline, 1GB peak
- CPU (idle): <1%
- Disk: 100MB app + variable data

**Scalability**:
- 100k-1M documents indexed
- 10k-100k knowledge graph nodes
- 100-1000 persistent conversations

### 10. Security Considerations

#### Threat Model
- **Assets**: User config, indexed documents, chat history, KG data
- **Actors**: Malicious apps, network attackers, physical access

#### Security Measures
- **Data Protection**: Sandboxing, secret management, process isolation
- **Network Security**: HTTPS only, certificate validation, token storage in memory
- **Input Validation**: Query sanitization, path validation, config validation
- **Tauri Allowlist**: Minimal permissions (dialog, path, fs, globalShortcut)

#### Privacy
- Local-first processing (no cloud by default)
- Opt-in external haystacks
- No telemetry or tracking
- Local-only logging

### 11. Build and Deployment

#### Development
```bash
cd desktop
yarn install
yarn run dev              # Vite dev server
yarn run tauri:dev        # Full Tauri app
```

#### Production
```bash
yarn run build            # Vite build
yarn run tauri build      # Create installers
```

**Output Formats**:
- Linux: .deb, .AppImage, .rpm
- macOS: .dmg, .app (signed + notarized)
- Windows: .msi, .exe (signed)

**Bundle Size**: ~50MB (includes Rust runtime)

#### Release Process
1. Update version in package.json and Cargo.toml
2. Update CHANGELOG.md
3. Commit and tag
4. GitHub Actions builds for all platforms
5. Create GitHub release with artifacts
6. Generate latest.json for auto-updater

#### Distribution
- Desktop installers for Windows/macOS/Linux
- MCP server mode: `terraphim-desktop mcp-server`
- Web version (limited features)

### 12. Extensibility

#### Plugin Architecture
- **HaystackIndexer trait**: Add new data sources
- **RelevanceScorer trait**: Custom ranking algorithms
- **ThesaurusBuilder trait**: Custom concept extraction
- **LlmProvider trait**: Additional LLM backends

#### Extension Points
- Theme system (Bulma-based CSS)
- MCP tool registration
- Custom relevance functions
- Knowledge graph builders

### 13. Key Differentiators

1. **Privacy-First**: Local processing, no cloud dependencies
2. **Knowledge Graph Intelligence**: Semantic understanding beyond text search
3. **Multi-Source Integration**: 9+ haystack types unified search
4. **Native Performance**: Tauri desktop with system integration
5. **MCP Integration**: AI development tools interoperability
6. **Production Quality**: Comprehensive testing and error handling

## Target Audiences

### Primary Users
- **Software Engineers**: Code docs, Stack Overflow, GitHub
- **Researchers**: Academic papers, notes, references
- **Knowledge Workers**: Wikis, email, task management
- **System Operators**: Infrastructure docs, runbooks, logs

### Use Cases
- Multi-source semantic search
- Knowledge graph exploration
- AI-assisted research and writing
- Role-based work contexts
- Secure local AI assistance

## Related Documentation

- **Implementation**: See individual component files in `desktop/src/`
- **Backend Services**: See crate documentation in `crates/*/README.md`
- **Testing**: `desktop/README.md` for test organization
- **Deployment**: `docs/deployment.md` for production setup
- **MCP Integration**: `docs/mcp-file-context-tools.md`

## Technical Highlights

### Innovation
- Novel editor with MCP autocomplete
- Knowledge graph-based semantic search
- Sub-millisecond autocomplete with automata
- Multi-haystack parallel search
- Persistent conversation management

### Engineering Excellence
- 50+ E2E tests with Playwright
- 22 UI themes with consistent UX
- Comprehensive error handling
- Type-safe IPC with Tauri
- WebAssembly support for autocomplete

### Production Readiness
- Auto-update mechanism
- 1Password secret management
- Multi-backend persistence
- Graceful degradation
- Comprehensive logging

## Statistics

**Document Metrics**:
- 16 major sections with detailed subsections
- ~12,000 words of technical documentation
- 50+ code examples and snippets
- 20+ tables and specifications
- Component diagrams and architecture flows

**Coverage Areas**:
- Complete system architecture
- All 30+ Tauri commands documented
- All 9+ haystack integrations detailed
- Full data model specifications
- Comprehensive testing strategy
- Performance targets and benchmarks
- Security threat model and mitigations

**Reference Value**:
- Authoritative technical specification
- Onboarding documentation for new developers
- API reference for frontend/backend integration
- Testing requirements and strategies
- Deployment and release procedures
- Extensibility guidelines for plugins

---

**Note**: This specification document is the single source of truth for Terraphim Desktop architecture and implementation. All development, testing, and deployment decisions should reference this document.

**Last Generated**: 2025-11-24
