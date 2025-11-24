# Terraphim Desktop Application - Technical Specification

**Version:** 1.0.0
**Last Updated:** 2025-11-24
**Status:** Production

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [System Overview](#system-overview)
3. [Architecture](#architecture)
4. [Core Features](#core-features)
5. [User Interface](#user-interface)
6. [Backend Integration](#backend-integration)
7. [Data Models](#data-models)
8. [Configuration System](#configuration-system)
9. [Testing Strategy](#testing-strategy)
10. [Build and Deployment](#build-and-deployment)
11. [Performance Requirements](#performance-requirements)
12. [Security Considerations](#security-considerations)
13. [Extensibility](#extensibility)

---

## 1. Executive Summary

Terraphim Desktop is a privacy-first, locally-running AI assistant that provides semantic search across multiple knowledge repositories. Built with Tauri and Svelte, it combines native desktop capabilities with modern web technologies to deliver a fast, secure, and user-friendly experience.

### Key Value Propositions

- **Privacy-First**: All data processing occurs locally; no cloud dependencies
- **Multi-Source Search**: Unified search across personal, team, and public knowledge sources
- **Semantic Understanding**: Knowledge graph-based search with concept relationships
- **Customizable Roles**: User profiles with domain-specific search preferences
- **Native Performance**: Desktop integration with system tray and global shortcuts
- **Extensible Architecture**: MCP (Model Context Protocol) integration for AI tooling

---

## 2. System Overview

### 2.1 Purpose

Terraphim Desktop enables users to:
- Search across multiple data sources (local files, Notion, email, documentation)
- Navigate knowledge graphs to discover related concepts
- Interact with AI for contextual assistance and chat
- Manage role-based configurations for different work contexts
- Visualize relationships between concepts and documents

### 2.2 Target Users

- **Software Engineers**: Searching code documentation, Stack Overflow, GitHub
- **Researchers**: Academic papers, notes, reference materials
- **Knowledge Workers**: Company wikis, email, task management systems
- **System Operators**: Infrastructure documentation, runbooks, logs

### 2.3 System Requirements

#### Minimum Requirements
- **OS**: Windows 10+, macOS 10.15+, Linux (Ubuntu 20.04+)
- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 500MB for application + variable for data
- **CPU**: Dual-core 2GHz or better

#### Optional Requirements
- **Ollama**: For local LLM inference (chat features)
- **Atomic Server**: For persistent storage backend
- **1Password CLI**: For secret management integration

---

## 3. Architecture

### 3.1 Technology Stack

#### Frontend
- **Framework**: Svelte 5.2.8 with TypeScript
- **UI Library**: Bulma CSS 1.0.4 (no Tailwind)
- **Routing**: Tinro 0.6.12
- **Build Tool**: Vite 5.3.4
- **Rich Text Editor**: Novel Svelte + TipTap
- **Visualization**: D3.js 7.9.0 for knowledge graphs
- **Testing**: Vitest + Playwright + Testing Library

#### Backend
- **Runtime**: Tauri 2.9.4 (Rust-based)
- **Core Service**: terraphim_service (Rust)
- **Configuration**: terraphim_config (Rust)
- **Persistence**: terraphim_persistence (multi-backend)
- **Search Engine**: terraphim_middleware
- **Knowledge Graph**: terraphim_rolegraph
- **Autocomplete**: terraphim_automata

#### Integration Layers
- **MCP Server**: Model Context Protocol for AI tool integration
- **IPC**: Tauri commands for frontend-backend communication
- **Storage Backends**: Memory, SQLite, RocksDB, Atomic Data

### 3.2 System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Terraphim Desktop                           │
├─────────────────────────────────────────────────────────────────┤
│  Frontend (Svelte + TypeScript)                                 │
│  ├─ Search Interface                                            │
│  ├─ Chat Interface (with Novel Editor)                          │
│  ├─ Knowledge Graph Visualization                               │
│  ├─ Configuration Wizard/Editor                                 │
│  └─ Theme Switcher (22 themes)                                  │
├─────────────────────────────────────────────────────────────────┤
│  Tauri IPC Layer                                                │
│  ├─ Commands (search, config, chat, KG operations)             │
│  ├─ State Management (ConfigState, Conversations)              │
│  └─ Event System (global shortcuts, system tray)               │
├─────────────────────────────────────────────────────────────────┤
│  Backend Services (Rust)                                        │
│  ├─ TerraphimService (orchestration)                           │
│  ├─ SearchService (multi-haystack search)                      │
│  ├─ RoleGraphService (knowledge graph)                         │
│  ├─ AutocompleteService (terraphim_automata)                   │
│  ├─ LLM Service (Ollama/OpenRouter integration)                │
│  └─ Persistence Layer (storage abstraction)                    │
├─────────────────────────────────────────────────────────────────┤
│  Data Sources (Haystacks)                                       │
│  ├─ Ripgrep (local filesystem)                                 │
│  ├─ MCP (Model Context Protocol)                               │
│  ├─ Atomic Server (Atomic Data)                                │
│  ├─ ClickUp (task management)                                  │
│  ├─ Logseq (personal knowledge)                                │
│  ├─ QueryRs (Rust docs + Reddit)                               │
│  ├─ Atlassian (Confluence/Jira)                                │
│  ├─ Discourse (forums)                                         │
│  └─ JMAP (email)                                               │
├─────────────────────────────────────────────────────────────────┤
│  External Integrations                                          │
│  ├─ MCP Server (stdio/SSE/HTTP)                                │
│  ├─ Ollama (local LLM)                                         │
│  ├─ 1Password CLI (secrets)                                    │
│  └─ System APIs (shortcuts, tray, filesystem)                  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.3 Component Responsibilities

#### Frontend Components

**App.svelte**
- Main application shell
- Top-level routing (Search, Chat, Graph tabs)
- Navigation controls and layout
- Theme integration

**Search Component**
- Real-time typeahead search
- Result display with ranking
- Tag filtering and logical operators (AND, OR, NOT)
- Integration with knowledge graph terms

**Chat Component**
- Conversation management (create, list, switch)
- Message composition with Novel editor
- Context management (add/edit/delete)
- LLM integration with streaming responses
- Session list sidebar

**RoleGraphVisualization Component**
- D3.js-based force-directed graph
- Node/edge rendering with zooming/panning
- Interactive node selection
- Document associations

**ConfigWizard/ConfigJsonEditor**
- Visual configuration builder
- JSON schema validation
- Role management (create, edit, switch)
- Haystack configuration

**ThemeSwitcher**
- 22 Bulma theme variants
- Persistent theme selection
- Dynamic CSS loading

#### Backend Commands (Tauri)

**Search Commands**
- `search(query, role)`: Multi-haystack search with relevance ranking
- `search_kg_terms(query)`: Knowledge graph term search
- `get_autocomplete_suggestions(prefix)`: Real-time autocomplete

**Configuration Commands**
- `get_config()`: Retrieve current configuration
- `update_config(config)`: Update and persist configuration
- `select_role(role_name)`: Switch active role
- `get_config_schema()`: JSON schema for validation

**Knowledge Graph Commands**
- `get_rolegraph(role)`: Load knowledge graph for role
- `find_documents_for_kg_term(term)`: Get documents associated with term
- `add_kg_term_context(term)`: Add KG term to conversation context
- `add_kg_index_context(index)`: Add KG index to conversation context

**Chat Commands**
- `chat(messages, role)`: LLM chat completion
- `create_conversation(role)`: Create new conversation
- `list_conversations()`: List all conversations
- `get_conversation(id)`: Get conversation details
- `add_message_to_conversation(id, message)`: Add message
- `add_context_to_conversation(id, context)`: Add context item
- `add_search_context_to_conversation(id, query)`: Add search results as context
- `delete_context(conversation_id, context_id)`: Remove context
- `update_context(conversation_id, context_id, content)`: Edit context

**Persistent Conversation Commands**
- `create_persistent_conversation(role, title)`: Create persistent conversation
- `list_persistent_conversations()`: List all saved conversations
- `get_persistent_conversation(id)`: Get conversation with messages
- `update_persistent_conversation(id, data)`: Update conversation
- `delete_persistent_conversation(id)`: Delete conversation
- `search_persistent_conversations(query)`: Search conversations
- `export_persistent_conversation(id)`: Export to JSON
- `import_persistent_conversation(data)`: Import from JSON
- `get_conversation_statistics()`: Get usage statistics

**Integration Commands**
- `onepassword_status()`: Check 1Password CLI availability
- `onepassword_resolve_secret(reference)`: Resolve secret reference
- `onepassword_process_config(config)`: Process config with secrets
- `onepassword_load_settings()`: Load settings with secret resolution
- `publish_thesaurus(thesaurus)`: Publish knowledge graph
- `create_document(document)`: Create document
- `get_document(id)`: Retrieve document

---

## 4. Core Features

### 4.1 Semantic Search

#### Search Capabilities
- **Real-time Autocomplete**: Typeahead suggestions from knowledge graph
- **Multi-Haystack**: Parallel search across configured data sources
- **Relevance Ranking**: Configurable scoring (TitleScorer, BM25, TerraphimGraph)
- **Logical Operators**: AND, OR, NOT, exact phrases (quotes)
- **Tag Filtering**: Filter results by tags
- **Knowledge Graph Integration**: Concept-based semantic expansion

#### Search Flow
1. User types query in search input
2. Autocomplete suggestions from terraphim_automata
3. On submit: query sent to all configured haystacks
4. Results aggregated and ranked by relevance function
5. Display with title, description, URL, tags, rank
6. Click result to open ArticleModal with full content

#### Search Configuration
```json
{
  "relevance_function": "TerraphimGraph|BM25|BM25Plus|BM25F|TitleScorer",
  "haystacks": [
    {
      "name": "Local Docs",
      "service": "Ripgrep",
      "extra_parameters": {
        "path": "/path/to/docs",
        "glob": "*.md"
      }
    }
  ]
}
```

### 4.2 Knowledge Graph

#### Graph Structure
- **Nodes**: Concepts/terms from thesaurus
- **Edges**: Relationships between concepts
- **Documents**: Associated content for each concept
- **Metadata**: ID, normalized term, URL

#### Graph Operations
- **Thesaurus Building**: Extract concepts from documents/URLs
- **Automata Construction**: Fast text matching with Aho-Corasick
- **Graph Visualization**: D3.js force-directed layout
- **Path Finding**: Verify connectivity between matched terms
- **Document Association**: Link documents to concepts

#### Graph Workflow
1. Load thesaurus for selected role
2. Build automata for fast matching
3. Index documents with concept extraction
4. Construct graph with nodes/edges
5. Process queries with semantic expansion
6. Visualize relationships in RoleGraphVisualization

### 4.3 AI Chat

#### Chat Features
- **Conversation Management**: Create, list, switch, delete conversations
- **Context Management**: Add/edit/remove context items
- **Search Integration**: Add search results as context
- **KG Integration**: Add knowledge graph terms/indices as context
- **Streaming Responses**: Real-time LLM output
- **Session Persistence**: Save/load conversations
- **Statistics**: Track usage by role

#### Chat Context Types
- **Document**: Full document content
- **SearchResult**: Aggregated search results
- **KGTerm**: Knowledge graph term definition
- **KGIndex**: Knowledge graph index entry
- **Manual**: User-provided text

#### Novel Editor Integration
- **Rich Text Editing**: TipTap-based editor
- **MCP Autocomplete**: Real-time suggestions from MCP server
- **Slash Commands**: `/search`, `/context`, etc.
- **Markdown Support**: Export/import markdown format

#### Chat Flow
1. User creates conversation or selects existing
2. Add context via search, KG, or manual input
3. Compose message in Novel editor
4. Submit to LLM with context
5. Stream response to UI
6. Save message pair to conversation
7. Update statistics

### 4.4 Role-Based Configuration

#### Role Concept
A role represents a user profile with:
- **Name**: Human-readable identifier
- **Relevance Function**: Scoring algorithm preference
- **Theme**: UI theme name
- **Haystacks**: Configured data sources
- **Extra Settings**: LLM provider, API keys, custom parameters

#### Role Management
- **Default Role**: Loaded on startup
- **Selected Role**: Currently active role
- **Role Switching**: Via UI or system tray
- **Per-Role Knowledge Graph**: Separate thesaurus and automata
- **Per-Role Settings**: Independent configurations

#### Example Roles
```json
{
  "roles": {
    "Terraphim Engineer": {
      "name": "Terraphim Engineer",
      "relevance_function": "TerraphimGraph",
      "theme": "darkly",
      "haystacks": [
        { "name": "Local Rust Docs", "service": "Ripgrep" },
        { "name": "GitHub Issues", "service": "MCP" }
      ],
      "extra": {
        "llm_provider": "ollama",
        "ollama_model": "llama3.2:3b"
      }
    }
  }
}
```

### 4.5 Multi-Source Integration

#### Haystack Types

**Ripgrep (Local Filesystem)**
- Fast text search using ripgrep command
- Glob patterns for file filtering
- Tag extraction from markdown frontmatter
- Path-based organization

**MCP (Model Context Protocol)**
- Integration with AI development tools
- SSE/HTTP/stdio transports
- OAuth bearer token authentication
- Tool discovery and invocation

**Atomic Server**
- Atomic Data protocol integration
- Collection-based search
- Base64-encoded secret authentication
- Real-time updates

**ClickUp**
- Task and project management
- List and team search
- API token authentication
- Custom field support

**Logseq**
- Personal knowledge management
- Markdown parsing
- Block-level references
- Graph relationships

**QueryRs**
- Rust std documentation search
- Reddit community integration
- Smart type detection
- Suggest API (~300ms response)

**Atlassian (Confluence/Jira)**
- Enterprise wiki search
- Issue tracking integration
- OAuth authentication
- Space/project filtering

**Discourse**
- Forum integration
- Topic and post search
- Category filtering
- User reputation

**JMAP (Email)**
- Email integration via JMAP protocol
- Mailbox search
- Thread grouping
- Attachment handling

### 4.6 System Integration

#### Native Desktop Features

**System Tray**
- Show/hide window toggle
- Role switching menu
- Quit application
- Dynamic menu updates

**Global Shortcuts**
- Configurable keyboard shortcut (e.g., `Cmd+Shift+Space`)
- Toggle window visibility
- Works across all applications
- Persistent registration

**Window Management**
- Resizable main window (1024x768 default)
- Splashscreen for first-run setup
- Hide on close (minimize to tray)
- Focus on show

**Auto-Update**
- GitHub releases integration
- Automatic update checking
- User-prompted installation
- Version verification with public key

**Data Initialization**
- Bundled default content (docs/src)
- First-run data folder setup
- Check for existing data
- Copy bundled content if missing

---

## 5. User Interface

### 5.1 Layout Structure

#### Main Application Layout
```
┌─────────────────────────────────────────────────────────────┐
│  [Logo]  [Search] [Chat] [Graph]            [Theme Switcher]│
├─────────────────────────────────────────────────────────────┤
│                                                               │
│                     Content Area                              │
│                  (Route-based content)                        │
│                                                               │
├─────────────────────────────────────────────────────────────┤
│  Footer (hover to show)                                       │
│  [Home] [Wizard] [JSON Editor] [Graph] [Chat]               │
└─────────────────────────────────────────────────────────────┘
```

#### Responsive Design
- **Desktop**: Full layout with all features
- **Tablet**: Condensed navigation, full content
- **Mobile**: Not primary target, but functional

### 5.2 Search Page

#### Search Input
- **Component**: KGSearchInput.svelte
- **Features**:
  - Real-time autocomplete dropdown
  - Keyboard navigation (arrows, enter, escape)
  - Logical operator support (AND, OR, NOT, quotes)
  - Tag chip display
  - Clear button

#### Search Results
- **Component**: ResultItem.svelte
- **Display**:
  - Title (clickable link)
  - Description/excerpt
  - URL
  - Tags (colored chips)
  - Rank score
  - Actions: Open, Add to Context

#### Article Modal
- **Component**: ArticleModal.svelte
- **Features**:
  - Full document content
  - Markdown rendering
  - Close button
  - Optional: Save to Atomic Server

### 5.3 Chat Page

#### Layout
```
┌─────────────────────────────────────────────────────────────┐
│  [☰ Sessions] [New Conversation ▼]           [Role: Eng ▼] │
├───────────────┬─────────────────────────────────────────────┤
│               │  Context: [3 items]  [+ Add Context ▼]      │
│  Session List │  ┌──────────────────────────────────────┐   │
│  (collapsible)│  │ Context Item 1  [Edit] [Delete]     │   │
│               │  │ Context Item 2  [Edit] [Delete]     │   │
│  - Session 1  │  │ Context Item 3  [Edit] [Delete]     │   │
│  - Session 2  │  └──────────────────────────────────────┘   │
│  - Session 3  │                                              │
│               │  Messages:                                   │
│               │  ┌──────────────────────────────────────┐   │
│               │  │ User: query about X                  │   │
│               │  └──────────────────────────────────────┘   │
│               │  ┌──────────────────────────────────────┐   │
│               │  │ Assistant: response...               │   │
│               │  └──────────────────────────────────────┘   │
│               │                                              │
│               │  [Novel Editor for input]                   │
│               │  [Send] [Clear]                             │
└───────────────┴─────────────────────────────────────────────┘
```

#### Session List
- **Component**: SessionList.svelte
- **Features**:
  - List persistent conversations
  - Show title, role, message count, preview
  - Click to load conversation
  - Delete confirmation
  - Create new button

#### Context Management
- **Component**: ContextEditModal.svelte
- **Actions**:
  - Add: Document, SearchResult, KGTerm, KGIndex, Manual
  - Edit: Inline editing with textarea
  - Delete: Remove from conversation
  - Reorder: Drag-and-drop (future)

#### Message Display
- **User Messages**: Right-aligned, blue background
- **Assistant Messages**: Left-aligned, gray background
- **Markdown Rendering**: svelte-markdown
- **Code Highlighting**: Syntax highlighting (future)

#### Novel Editor
- **Component**: NovelWrapper.svelte
- **Features**:
  - Rich text editing with TipTap
  - MCP autocomplete integration
  - Slash commands
  - Markdown export
  - Placeholder text

### 5.4 Graph Page

#### Knowledge Graph Visualization
- **Component**: RoleGraphVisualization.svelte
- **Rendering**: D3.js force-directed graph
- **Interactions**:
  - Zoom/pan with mouse wheel and drag
  - Click node to select
  - Hover for tooltip
  - Double-click to focus
- **Display**:
  - Nodes: Circles with concept labels
  - Edges: Lines connecting related concepts
  - Colors: By category/type
  - Size: By document count

### 5.5 Configuration Pages

#### Configuration Wizard
- **Component**: ConfigWizard.svelte
- **Workflow**:
  1. Select role template
  2. Configure haystacks
  3. Set LLM provider
  4. Choose theme
  5. Save configuration
- **Validation**: Client-side schema validation

#### JSON Editor
- **Component**: ConfigJsonEditor.svelte
- **Features**:
  - Syntax-highlighted JSON editor
  - Schema validation
  - Error highlighting
  - Save/revert buttons
  - Import/export

### 5.6 Theme System

#### Theme Management
- **Storage**: localStorage persistence
- **Themes**: 22 Bulma Bootswatch variants
- **Switching**: Dropdown selector in header
- **Dynamic Loading**: CSS loaded on-demand
- **Dark Mode**: Automatic color scheme detection

#### Available Themes
- cerulean, cosmo, cyborg, darkly, flatly, journal
- litera, lumen, lux, materia, minty, morph, pulse
- quartz, sandstone, simplex, sketchy, slate, solar
- spacelab, superhero, united, vapor, yeti, zephyr

---

## 6. Backend Integration

### 6.1 Tauri IPC Architecture

#### Command Pattern
```rust
#[command]
async fn search(
    query: String,
    role: Option<String>,
    config_state: State<'_, ConfigState>,
) -> Result<SearchResponse> {
    // 1. Get current configuration
    // 2. Select role (use provided or default)
    // 3. Initialize TerraphimService
    // 4. Execute search across haystacks
    // 5. Rank and aggregate results
    // 6. Return SearchResponse
}
```

#### State Management
- **ConfigState**: Shared Arc<Mutex<Config>>
- **DeviceSettings**: Arc<Mutex<DeviceSettings>>
- **Conversation State**: In-memory HashMap (non-persistent)
- **Persistent Conversations**: Via persistence layer

#### Error Handling
- **Custom Error Type**: TerraphimTauriError
- **Error Variants**: Middleware, Persistence, Service, Settings, OnePassword
- **Serialization**: Manual Serialize implementation
- **Frontend Error Display**: User-friendly error messages

### 6.2 Service Layer

#### TerraphimService
- **Responsibility**: High-level orchestration
- **Operations**:
  - Search coordination
  - LLM chat completion
  - Document summarization
  - Conversation management
- **Dependencies**: Config, Persistence, Middleware

#### SearchService (terraphim_middleware)
- **Responsibility**: Multi-haystack search orchestration
- **Operations**:
  - Parallel haystack queries
  - Result aggregation
  - Relevance scoring
  - Deduplication
- **Indexers**: Ripgrep, Atomic, ClickUp, QueryRs, MCP, etc.

#### RoleGraphService (terraphim_rolegraph)
- **Responsibility**: Knowledge graph management
- **Operations**:
  - Thesaurus loading
  - Graph construction
  - Node/edge traversal
  - Document association
- **Automata**: terraphim_automata for fast matching

#### AutocompleteService (terraphim_automata)
- **Responsibility**: Real-time autocomplete
- **Operations**:
  - Prefix matching
  - Fuzzy search (Jaro-Winkler)
  - Snippet generation
  - WASM compilation
- **Performance**: Sub-millisecond response times

#### LLM Service
- **Providers**: Ollama (local), OpenRouter (cloud)
- **Operations**:
  - Chat completion
  - Streaming responses
  - Context formatting
  - Token management
- **Configuration**: Per-role provider settings

### 6.3 Persistence Layer

#### Storage Backends
- **Memory**: In-memory HashMap (default, fast)
- **SQLite**: Persistent relational database
- **RocksDB**: High-performance key-value store
- **Atomic Data**: Distributed persistence
- **Redb**: Embedded LMDB alternative

#### Persistable Trait
```rust
#[async_trait]
pub trait Persistable {
    async fn save(&mut self) -> Result<()>;
    async fn load(&mut self) -> Result<Self>;
    async fn delete(&mut self) -> Result<()>;
}
```

#### Persistence Operations
- **Configuration**: Save/load entire config
- **Thesaurus**: Save/load knowledge graph
- **Conversations**: CRUD operations
- **Documents**: Create/read/update/delete

---

## 7. Data Models

### 7.1 Core Types

#### Config
```typescript
interface Config {
  id: "Desktop";
  global_shortcut: string;
  roles: Record<string, Role>;
  default_role: RoleName;
  selected_role: RoleName;
}
```

#### Role
```typescript
interface Role {
  name: string;
  relevance_function: "TerraphimGraph" | "BM25" | "BM25Plus" | "BM25F" | "TitleScorer";
  theme: string;
  haystacks: Haystack[];
  terraphim_it: boolean; // Enable knowledge graph
  kg?: KnowledgeGraph;
  extra?: Record<string, any>;
}
```

#### Haystack
```typescript
interface Haystack {
  name: string;
  service: "Ripgrep" | "AtomicServer" | "ClickUp" | "Logseq" | "QueryRs" | "MCP" | "Atlassian" | "Discourse" | "JMAP";
  extra_parameters?: Record<string, any>;
}
```

#### Document
```typescript
interface Document {
  id: string;
  url: string;
  body: string;
  description: string;
  tags: string[];
  rank?: number;
}
```

#### SearchQuery
```typescript
interface SearchQuery {
  query: string;
  role?: string;
  limit?: number;
  offset?: number;
  filters?: Record<string, any>;
}
```

### 7.2 Chat Models

#### Conversation
```typescript
interface Conversation {
  id: string;
  role: string;
  messages: Message[];
  contexts: ContextItem[];
  created_at: string;
  updated_at: string;
}
```

#### Message
```typescript
interface Message {
  role: "user" | "assistant";
  content: string;
  timestamp: string;
}
```

#### ContextItem
```typescript
interface ContextItem {
  id: string;
  title: string;
  content: string;
  context_type: "Document" | "SearchResult" | "KGTerm" | "KGIndex" | "Manual";
  metadata?: Record<string, any>;
}
```

#### ConversationSummary
```typescript
interface ConversationSummary {
  id: string;
  title: string;
  role: string;
  message_count: number;
  preview: string | null;
  created_at: string;
  updated_at: string;
}
```

#### ConversationStatistics
```typescript
interface ConversationStatistics {
  total_conversations: number;
  total_messages: number;
  conversations_by_role: Record<string, number>;
}
```

### 7.3 Knowledge Graph Models

#### KnowledgeGraph
```typescript
interface KnowledgeGraph {
  nodes: KGNode[];
  edges: KGEdge[];
  documents: Record<string, Document[]>;
}
```

#### KGNode
```typescript
interface KGNode {
  id: string;
  term: string;
  normalized_term: string;
  url?: string;
  metadata?: Record<string, any>;
}
```

#### KGEdge
```typescript
interface KGEdge {
  source: string;
  target: string;
  weight?: number;
  relationship?: string;
}
```

#### KGTermDefinition
```typescript
interface KGTermDefinition {
  term: string;
  definition: string;
  related_terms: string[];
  document_count: number;
}
```

---

## 8. Configuration System

### 8.1 Configuration Hierarchy

#### Load Priority
1. Environment variables (`TERRAPHIM_CONFIG`, `TERRAPHIM_DATA_DIR`)
2. Saved configuration from persistence layer
3. Default desktop configuration
4. Fallback minimal configuration

#### Configuration Files
- **Location**: Platform-specific app data directory
- **Format**: JSON
- **Schema**: Validated via schemars
- **Backup**: Automatic backup before updates

### 8.2 Device Settings

#### DeviceSettings
```rust
pub struct DeviceSettings {
    pub initialized: bool,
    pub default_data_path: String,
    pub config_path: String,
    pub log_level: String,
}
```

#### Settings File
- **Location**: `~/.config/terraphim/settings.toml` (Linux/macOS)
- **Format**: TOML
- **Persistence**: Saved on update
- **Environment Overrides**: `TERRAPHIM_*` variables

### 8.3 Secret Management

#### 1Password Integration
- **CLI Tool**: `op` command
- **Secret References**: `op://vault/item/field`
- **Resolution**: Automatic on config load
- **Caching**: Memory cache for session
- **Status Check**: Verify CLI availability

#### Secret Processing
```typescript
// Example config with secret reference
{
  "haystacks": [
    {
      "name": "Atomic Server",
      "service": "AtomicServer",
      "extra_parameters": {
        "secret": "op://Private/atomic-server/api-key"
      }
    }
  ]
}
```

---

## 9. Testing Strategy

### 9.1 Test Pyramid

```
                    ╱╲
                   ╱  ╲
                  ╱ E2E╲
                 ╱──────╲
                ╱        ╲
               ╱Integration╲
              ╱────────────╲
             ╱              ╲
            ╱   Unit Tests   ╲
           ╱──────────────────╲
```

### 9.2 Unit Tests

#### Frontend Unit Tests (Vitest)
- **Coverage Target**: >85%
- **Framework**: Vitest + Testing Library
- **Location**: `src/**/*.test.ts`
- **Run**: `yarn test`

**Test Categories**:
- Component rendering
- Store mutations
- Service functions
- Utility functions
- Search operators

#### Backend Unit Tests (Rust)
- **Coverage Target**: >90%
- **Framework**: cargo test
- **Location**: `src-tauri/tests/`
- **Run**: `cargo test -p terraphim_desktop`

**Test Categories**:
- Command handlers
- Service operations
- State management
- Error handling
- Async functionality

### 9.3 Integration Tests

#### Component Integration
- **Framework**: Testing Library + Vitest
- **Scope**: Component interactions
- **Examples**:
  - Search input → results display
  - Context modal → conversation update
  - Theme switcher → CSS loading

#### Service Integration
- **Framework**: cargo test with integration feature
- **Scope**: Cross-crate functionality
- **Examples**:
  - Search service → indexers
  - Config service → persistence
  - LLM service → providers

### 9.4 End-to-End Tests

#### Playwright E2E
- **Coverage**: Major user workflows
- **Location**: `tests/e2e/*.spec.ts`
- **Run**: `yarn e2e`

**Test Suites**:
- `search.spec.ts`: Search functionality
- `chat-functionality.spec.ts`: Chat workflows
- `kg-graph-functionality.spec.ts`: Knowledge graph
- `navigation.spec.ts`: Routing and navigation
- `config-wizard.spec.ts`: Configuration
- `atomic-server-haystack.spec.ts`: Atomic integration
- `ollama-integration.spec.ts`: LLM integration
- `major-user-journey.spec.ts`: Complete workflows
- `performance-stress.spec.ts`: Performance validation

#### Visual Regression Tests
- **Framework**: Playwright visual comparisons
- **Location**: `tests/visual/*.spec.ts`
- **Run**: `npx playwright test tests/visual`

**Test Coverage**:
- Theme consistency (all 22 themes)
- Responsive layouts
- Component rendering
- Accessibility visual checks

#### Tauri E2E
- **Framework**: Tauri's built-in test harness
- **Location**: `src-tauri/tests/e2e_*.rs`
- **Run**: `cargo test --test e2e_*`

**Test Coverage**:
- Command invocation
- State persistence
- Window management
- System tray interaction

### 9.5 Performance Tests

#### Benchmarks
- **Framework**: Vitest benchmark mode
- **Location**: `vitest.benchmark.config.ts`
- **Run**: `yarn benchmark`

**Metrics**:
- Search response time (<200ms target)
- Autocomplete latency (<50ms target)
- Graph rendering (60fps target)
- Memory usage (< 500MB baseline)

#### Load Testing
- **Tool**: Custom Playwright script
- **Scenarios**:
  - Concurrent searches (10 parallel)
  - Large result sets (1000+ documents)
  - Rapid role switching
  - Knowledge graph with 10k+ nodes

### 9.6 CI/CD Testing

#### GitHub Actions Workflow
```yaml
jobs:
  test-frontend:
    runs-on: ubuntu-latest
    steps:
      - checkout
      - setup node
      - yarn install
      - yarn test:coverage
      - upload coverage

  test-backend:
    runs-on: ubuntu-latest
    steps:
      - checkout
      - setup rust
      - cargo test --workspace
      - upload coverage

  test-e2e:
    runs-on: ubuntu-latest
    steps:
      - checkout
      - setup node + rust
      - yarn install
      - yarn e2e:ci
      - upload screenshots

  test-multiplatform:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - build + test on platform
```

---

## 10. Build and Deployment

### 10.1 Development Build

#### Frontend Development
```bash
cd desktop
yarn install
yarn run dev      # Vite dev server on http://localhost:5173
```

#### Tauri Development
```bash
cd desktop
yarn run tauri:dev  # Full Tauri app with hot reload
```

#### Backend Development
```bash
cargo run -p terraphim_desktop -- mcp-server  # MCP server mode
```

### 10.2 Production Build

#### Frontend Production
```bash
cd desktop
yarn run build  # Vite build to desktop/dist/
```

#### Tauri Production
```bash
cd desktop
yarn run tauri build  # Creates installers in src-tauri/target/release/bundle/
```

**Output Formats**:
- **Linux**: .deb, .AppImage, .rpm
- **macOS**: .dmg, .app
- **Windows**: .msi, .exe

#### Build Optimizations
- **Vite**: Code splitting, tree shaking, minification
- **Rust**: Release profile with opt-level=3, LTO
- **Assets**: Image optimization, CSS minification
- **Bundle Size**: ~50MB installer (includes Rust runtime)

### 10.3 Release Process

#### Version Management
- **Versioning**: Semantic versioning (MAJOR.MINOR.PATCH)
- **Changelog**: Automated from git commits
- **Tagging**: Git tags trigger releases

#### Release Workflow
1. Update version in `package.json` and `Cargo.toml`
2. Update `CHANGELOG.md` with release notes
3. Commit: `git commit -m "chore: release v1.0.0"`
4. Tag: `git tag -a v1.0.0 -m "Release v1.0.0"`
5. Push: `git push origin main --tags`
6. GitHub Actions: Build for all platforms
7. Create GitHub release with artifacts
8. Generate `latest.json` for auto-updater

#### Auto-Update
- **Endpoint**: GitHub releases API
- **Signature**: minisign public key verification
- **Dialog**: User-prompted installation
- **Rollback**: Automatic on failure

### 10.4 Distribution

#### Desktop Installers
- **Linux**: .deb for Debian/Ubuntu, .AppImage universal
- **macOS**: Signed .dmg with notarization
- **Windows**: Signed .msi with SmartScreen bypass

#### MCP Server Distribution
- **Binary**: Single executable with embedded resources
- **Invocation**: `terraphim-desktop mcp-server`
- **Integration**: Works with Claude Code, Cline, etc.
- **Documentation**: MCP configuration examples

#### Web Version
- **Deployment**: Vite build served statically
- **Limitations**: No Tauri features (system tray, shortcuts)
- **Use Case**: Demo, testing, minimal access

---

## 11. Performance Requirements

### 11.1 Response Time Targets

| Operation | Target | Maximum | Notes |
|-----------|--------|---------|-------|
| Autocomplete | <50ms | 100ms | From keypress to suggestions |
| Search (single haystack) | <200ms | 500ms | Simple text query |
| Search (multi-haystack) | <500ms | 1000ms | Parallel aggregation |
| Knowledge graph load | <1s | 2s | Initial graph construction |
| Chat message send | <100ms | 200ms | Excluding LLM latency |
| LLM streaming start | <2s | 5s | Time to first token |
| Config load | <200ms | 500ms | From disk to UI |
| Theme switch | <100ms | 200ms | CSS load and apply |

### 11.2 Resource Limits

| Resource | Baseline | Peak | Notes |
|----------|----------|------|-------|
| Memory | 200MB | 1GB | With large knowledge graph |
| CPU (idle) | <1% | - | Background with no activity |
| CPU (search) | <50% | 100% | During active search |
| Disk | 100MB | 5GB | App + data + cache |
| Network | 0 | 10Mbps | External haystack queries |

### 11.3 Scalability Targets

| Metric | Target | Maximum | Notes |
|--------|--------|---------|-------|
| Documents indexed | 100k | 1M | Local filesystem |
| Knowledge graph nodes | 10k | 100k | With acceptable render time |
| Conversations | 100 | 1000 | Persistent storage |
| Messages per conversation | 100 | 1000 | With pagination |
| Concurrent searches | 10 | 50 | Parallel user operations |
| Haystacks per role | 5 | 20 | Configured data sources |

### 11.4 Optimization Strategies

#### Frontend Optimizations
- **Virtual Scrolling**: For large result sets
- **Lazy Loading**: Load images/content on demand
- **Debouncing**: Autocomplete and search input
- **Memoization**: Computed values and components
- **Code Splitting**: Route-based chunks

#### Backend Optimizations
- **Caching**: Thesaurus, automata, search results
- **Parallelism**: Tokio async for concurrent operations
- **Indexing**: Pre-built indices for fast lookup
- **Batch Processing**: Aggregate operations
- **Connection Pooling**: Reuse HTTP clients

#### Database Optimizations
- **Indices**: Primary keys, search columns
- **Denormalization**: Flatten for faster reads
- **Compression**: Store compressed text
- **Vacuuming**: Periodic cleanup (SQLite)
- **Write Batching**: Bulk inserts/updates

---

## 12. Security Considerations

### 12.1 Threat Model

#### Assets to Protect
- User configuration (roles, haystacks, API keys)
- Indexed documents and content
- Chat conversations and context
- Knowledge graph data
- System integration (shortcuts, tray)

#### Threat Actors
- **Malicious Applications**: Reading app data
- **Network Attackers**: MitM on external APIs
- **Physical Access**: Unauthorized local access
- **Supply Chain**: Compromised dependencies

### 12.2 Security Measures

#### Data Protection
- **Encryption at Rest**: Not implemented (user responsible)
- **Secret Management**: 1Password CLI integration
- **Sandboxing**: Tauri security context
- **Process Isolation**: Separate frontend/backend

#### Network Security
- **HTTPS Only**: External API calls
- **Certificate Validation**: No self-signed certs
- **Token Storage**: Memory only, not persisted
- **OAuth Flow**: Standard authorization code

#### Input Validation
- **Query Sanitization**: Prevent injection
- **Path Validation**: No directory traversal
- **Config Validation**: JSON schema enforcement
- **Command Validation**: Whitelist allowed operations

#### Tauri Allowlist
```json
{
  "allowlist": {
    "all": false,
    "dialog": { "all": true },
    "path": { "all": true },
    "fs": { "all": true },
    "globalShortcut": { "all": true }
  }
}
```

### 12.3 Compliance

#### Privacy Considerations
- **Local-First**: No cloud data transmission (default)
- **Opt-In**: External haystacks require explicit config
- **Telemetry**: None (no usage tracking)
- **Logging**: Local files only, user-controlled

#### License Compliance
- **Dependencies**: All MIT/Apache-2.0 compatible
- **Attributions**: Included in about dialog
- **Source Code**: Open source (check LICENSE file)

---

## 13. Extensibility

### 13.1 Plugin Architecture

#### Haystack Plugin Interface
```rust
#[async_trait]
pub trait HaystackIndexer: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>>;
    fn name(&self) -> &str;
    fn supports_tags(&self) -> bool { false }
    fn supports_pagination(&self) -> bool { false }
}
```

**Adding New Haystack**:
1. Implement `HaystackIndexer` trait
2. Add to `terraphim_middleware/src/indexer/`
3. Register in service dispatcher
4. Update config schema
5. Add tests

#### MCP Tool Registration
```rust
// In terraphim_mcp_server
pub fn register_tools(server: &mut McpServer) {
    server.add_tool(
        "my_custom_tool",
        "Description of the tool",
        schema,
        handler_fn,
    );
}
```

### 13.2 Custom Relevance Functions

#### Scorer Interface
```rust
pub trait RelevanceScorer: Send + Sync {
    fn score(&self, query: &str, document: &Document) -> f64;
    fn name(&self) -> &str;
}
```

**Adding Custom Scorer**:
1. Implement `RelevanceScorer` trait
2. Add to `terraphim_service/src/score/`
3. Update `RelevanceFunction` enum
4. Register in search orchestration

### 13.3 Theme Extension

#### Custom Theme
1. Create Bulma-based CSS file
2. Place in `desktop/public/assets/bulmaswatch/`
3. Add theme name to `themeManager.ts`
4. Theme automatically available in switcher

### 13.4 Knowledge Graph Extensions

#### Custom Thesaurus Sources
```rust
pub trait ThesaurusBuilder: Send + Sync {
    async fn build(&self, source: &str) -> Result<Vec<ThesaurusEntry>>;
    fn source_type(&self) -> &str;
}
```

**Adding Thesaurus Builder**:
1. Implement `ThesaurusBuilder` trait
2. Add to `terraphim_rolegraph/src/builder/`
3. Register builder in factory
4. Update config schema

### 13.5 LLM Provider Extension

#### Provider Interface
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat_completion(
        &self,
        messages: Vec<Message>,
        stream: bool,
    ) -> Result<Box<dyn Stream<Item = String>>>;

    fn name(&self) -> &str;
    fn supports_streaming(&self) -> bool;
}
```

**Adding LLM Provider**:
1. Implement `LlmProvider` trait
2. Add to `terraphim_service/src/llm/`
3. Update role config schema
4. Add provider-specific settings

### 13.6 Future Extension Points

#### Planned Extensions
- **Cloud Sync**: Optional backup/sync service
- **Browser Extension**: Save web pages to haystacks
- **Mobile App**: iOS/Android companion apps
- **API Server**: RESTful API for external access
- **Collaborative Features**: Shared knowledge graphs
- **Advanced Analytics**: Usage insights and recommendations

#### Extension Guidelines
- **Backward Compatibility**: Maintain config schema compatibility
- **Performance**: Sub-100ms overhead target
- **Testing**: 100% test coverage for new features
- **Documentation**: Inline docs + user guide updates
- **Examples**: Provide working examples

---

## 14. Appendices

### 14.1 Glossary

| Term | Definition |
|------|------------|
| **Haystack** | Data source for search (local files, APIs, databases) |
| **Knowledge Graph** | Structured representation of concepts and relationships |
| **Role** | User profile with specific search preferences and data sources |
| **Thesaurus** | Collection of terms and concepts for semantic search |
| **Automata** | Fast text matching engine (Aho-Corasick algorithm) |
| **MCP** | Model Context Protocol for AI tool integration |
| **Relevance Function** | Algorithm for ranking search results |
| **Tauri** | Rust-based framework for building desktop apps |
| **Terraphim** | Privacy-first AI assistant (this application) |

### 14.2 Acronyms

| Acronym | Full Form |
|---------|-----------|
| **API** | Application Programming Interface |
| **BM25** | Best Matching 25 (ranking function) |
| **CI/CD** | Continuous Integration/Continuous Deployment |
| **CRUD** | Create, Read, Update, Delete |
| **CSS** | Cascading Style Sheets |
| **D3** | Data-Driven Documents (visualization library) |
| **E2E** | End-to-End |
| **HTTP** | Hypertext Transfer Protocol |
| **HTTPS** | HTTP Secure |
| **IPC** | Inter-Process Communication |
| **JMAP** | JSON Meta Application Protocol (email) |
| **JSON** | JavaScript Object Notation |
| **KG** | Knowledge Graph |
| **LLM** | Large Language Model |
| **MCP** | Model Context Protocol |
| **OAuth** | Open Authorization |
| **REST** | Representational State Transfer |
| **SQL** | Structured Query Language |
| **SSE** | Server-Sent Events |
| **UI** | User Interface |
| **URL** | Uniform Resource Locator |
| **WASM** | WebAssembly |

### 14.3 References

#### Documentation
- [Tauri Documentation](https://tauri.app/v2/guides/)
- [Svelte Documentation](https://svelte.dev/docs)
- [Bulma CSS Framework](https://bulma.io/documentation/)
- [D3.js Documentation](https://d3js.org/)
- [Model Context Protocol Spec](https://github.com/anthropics/mcp)

#### Related Projects
- [terraphim-ai Repository](https://github.com/terraphim/terraphim-ai)
- [Atomic Data](https://atomicdata.dev/)
- [Ollama](https://ollama.ai/)
- [Novel Editor](https://github.com/steven-tey/novel)

#### Rust Crates
- [tokio](https://tokio.rs/) - Async runtime
- [serde](https://serde.rs/) - Serialization
- [anyhow](https://docs.rs/anyhow/) - Error handling
- [tracing](https://docs.rs/tracing/) - Logging

---

## 15. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-11-24 | Initial specification document |

---

## 16. Document Metadata

**Author**: Claude (Anthropic)
**Specification Version**: 1.0.0
**Document Format**: Markdown
**Word Count**: ~12,000 words
**Last Review**: 2025-11-24
**Next Review**: 2025-12-24

---

**End of Specification**
