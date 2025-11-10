# Terraphim AI - Egui Migration Implementation Task List

## Project Overview

This task list provides a detailed roadmap for migrating Terraphim AI to an egui-based desktop application. The implementation leverages existing crates (terraphim_automata, terraphim_rolegraph, terraphim_service, etc.) and incorporates the WASM-based autocomplete functionality from terraphim-editor for sub-50ms search performance.

## Technology Stack

- **GUI Framework**: egui 0.27+ with eframe
- **Async Runtime**: tokio (existing)
- **WASM Integration**: wasm-bindgen (already in terraphim_automata)
- **Markdown Parsing**: pulldown-cmark (existing in terraphim-markdown-parser)
- **State Management**: egui's state + Arc<Mutex<>> for shared state
- **Graphics**: egui rendering + custom painters for knowledge graphs

---

## Phase 1: Foundation & Project Setup

### Module: Project Setup

#### Task 1.1: Create Egui Application Crate
**Location**: `crates/terraphim_egui/` (new directory)
**Description**: Set up the main egui application with eframe integration
**Implementation**:
- Create new crate with eframe dependency
- Set up main application struct `EguiApp` with required traits
- Configure window settings and event loop
- Initialize tokio runtime for async operations
- Set up logging infrastructure
**Dependencies**: None
**Effort**: M (3-4 days)
**Priority**: High
**Files to Create**:
```
crates/terraphim_egui/
├── Cargo.toml
├── src/
│   ├── main.rs (entry point with eframe::run_native)
│   ├── app.rs (main EguiApp struct)
│   ├── lib.rs (crate root)
│   └── state.rs (application state management)
```

#### Task 1.2: Integrate Existing Crate Dependencies
**Location**: `crates/terraphim_egui/Cargo.toml`
**Description**: Add all required Terraphim crate dependencies
**Implementation**:
- Add terraphim_automata (with wasm and tokio features)
- Add terraphim_rolegraph
- Add terraphim_service
- Add terraphim_config
- Add terraphim_types
- Add terraphim_persistence
- Configure feature flags appropriately
**Dependencies**: Task 1.1
**Effort**: S (1 day)
**Priority**: High

#### Task 1.3: Create Application State Management
**Location**: `crates/terraphim_egui/src/state.rs`
**Description**: Design shared state architecture for the application
**Implementation**:
- Create `AppState` struct with:
  - `Arc<Mutex<CurrentRole>>` - active role configuration
  - `Arc<Mutex<Vec<SearchResult>>>` - last search results
  - `Arc<Mutex<ContextManager>>` - context for LLM
  - `Arc<Mutex<ConversationHistory>>` - chat history
  - `AutocompleteIndex` - WASM-based autocomplete index
- Implement thread-safe access patterns
- Use egui's `Context::set_state()` for UI state
- Add initialization and cleanup methods
**Dependencies**: Task 1.1
**Effort**: M (3-4 days)
**Priority**: High

#### Task 1.4: Set Up Global Shortcut Handling
**Location**: `crates/terraphim_egui/src/app.rs`
**Description**: Implement global hotkey for application activation
**Implementation**:
- Configure `Ctrl+Shift+T` global shortcut
- Use eframe's native window events
- Implement show/hide functionality
- Handle window focus and activation
- Support multiple monitor setups
**Dependencies**: Task 1.1
**Effort**: S (2 days)
**Priority**: Medium

---

## Phase 2: Core UI Framework

### Module: Panel System

#### Task 2.1: Create Tabbed Panel Infrastructure
**Location**: `crates/terraphim_egui/src/ui/panels.rs`
**Description**: Build the central tab docking system
**Implementation**:
- Implement `CentralPanel` with tab management
- Create tab states: Search, Chat, KnowledgeGraph, Context, Configure, Sessions
- Design responsive layout with flexible panel sizes
- Implement tab switching logic
- Add panel persistence (remember open tabs)
- Support drag-and-drop tab reordering
**Dependencies**: Task 1.3
**Effort**: M (3-4 days)
**Priority**: High
**Key Files**:
```
src/ui/
├── panels.rs (TabContainer widget)
├── layout.rs (Responsive layout management)
└── mod.rs (module exports)
```

#### Task 2.2: Design Visual Design System
**Location**: `crates/terraphim_egui/src/ui/theme.rs`
**Description**: Create consistent theming and styling
**Implementation**:
- Define color palette (light/dark themes)
- Create typography system (fonts, sizes, weights)
- Design component styles (buttons, inputs, panels)
- Implement theme switching
- Support role-specific color schemes:
  - Default: Blue gradient
  - Rust Engineer: Orange/red accents
  - Terraphim Engineer: Purple/green accents
- Add contrast mode for accessibility
**Dependencies**: Task 2.1
**Effort**: M (3 days)
**Priority**: Medium

#### Task 2.3: Implement Status Bar
**Location**: `crates/terraphim_egui/src/ui/status_bar.rs`
**Description**: Create bottom status bar with system information
**Implementation**:
- Display current role name
- Show search result count
- Display active LLM provider
- Show connection status indicator
- Add progress bars for async operations
- Implement clickable status items for quick access
**Dependencies**: Task 2.1
**Effort**: S (2 days)
**Priority**: Medium

---

## Phase 3: Search & Autocomplete

### Module: Search Panel

#### Task 3.1: Create Search Input Component
**Location**: `crates/terraphim_egui/src/ui/search/input.rs`
**Description**: Build search bar with real-time autocomplete
**Implementation**:
- Create `SearchInput` widget with:
  - Text input field with placeholder
  - Real-time text change detection
  - Debounced autocomplete triggers (50ms)
  - Keyboard navigation (up/down/enter/tab)
  - Clear button and history
- Use egui's `TextEdit` with custom styling
- Implement input validation and sanitization
- Add search operators support (AND, OR, quotes)
- Support saved searches and quick filters
**Dependencies**: Task 2.1
**Effort**: M (3-4 days)
**Priority**: High

#### Task 3.2: Integrate WASM Autocomplete
**Location**: `crates/terraphim_egui/src/ui/search/autocomplete.rs`
**Description**: Integrate terraphim-editor's WASM autocomplete for <50ms search
**Implementation**:
- Load AutocompleteIndex from terraphim_automata
- Use existing WASM bindings (already in terraphim_automata)
- Implement autocomplete dropdown widget:
  - Show 3-5 top suggestions
  - Display with type indicators (article, concept, tag)
  - Add keyboard selection (↑/↓/Enter)
  - Implement mouse hover and click selection
  - Show relevance scores
- Optimize for 50ms response time
- Cache frequent searches
- Add fuzzy matching with Jaro-Winkler distance
**Dependencies**: Task 3.1
**Effort**: L (5-6 days)
**Priority**: High
**Key Integration Points**:
```rust
// Use existing functions from terraphim_automata:
use terraphim_automata::{AutocompleteIndex, fuzzy_autocomplete_search_jaro_winkler};
```

#### Task 3.3: Build Search Results Display
**Location**: `crates/terraphim_egui/src/ui/search/results.rs`
**Description**: Create search results panel with ranked articles
**Implementation**:
- Design `SearchResults` widget with:
  - Virtual scrolling for large result sets
  - Result items showing:
    - Title (clickable)
    - Excerpt/description
    - Relevance score (visual indicator)
    - Source haystack badge
    - Tags and metadata
  - Sort options (relevance, date, source)
  - Filter by source/type
  - Multi-select for context building
- Implement async result loading
- Add result caching and pagination
- Support "View in new tab" functionality
**Dependencies**: Task 3.2
**Effort**: M (4-5 days)
**Priority**: High

#### Task 3.4: Implement Search Execution Logic
**Location**: `crates/terraphim_egui/src/logic/search.rs`
**Description**: Connect UI to terraphim_service search functionality
**Implementation**:
- Create `SearchService` wrapper around terraphim_service
- Implement async search with progress indication
- Handle search cancellation
- Support multiple relevance functions (BM25, TitleScorer, TerraphimGraph)
- Implement result transformation for UI
- Add search history and suggestions
- Cache frequent searches
- Handle errors gracefully with user feedback
**Dependencies**: Task 3.3
**Effort**: M (3-4 days)
**Priority**: High

---

## Phase 4: Knowledge Graph Visualization

### Module: Knowledge Graph Panel

#### Task 4.1: Design Knowledge Graph Data Structures
**Location**: `crates/terraphim_egui/src/kg/graph.rs`
**Description**: Define data structures for KG visualization
**Implementation**:
- Create `GraphNode` struct:
  - id, label, term, url
  - position (x, y), velocity
  - type (document, concept, tag)
  - selected, highlighted states
- Create `GraphEdge` struct:
  - source, target node IDs
  - weight, relationship type
  - visible state
- Define layout algorithms:
  - Force-directed layout
  - Hierarchical layout for trees
  - Circular layout for tags
- Implement graph filtering and search
**Dependencies**: Task 2.1
**Effort**: M (3-4 days)
**Priority**: High

#### Task 4.2: Build Custom Knowledge Graph Painter
**Location**: `crates/terraphim_egui/src/kg/painter.rs`
**Description**: Create custom egui painter for graph rendering
**Implementation**:
- Implement `GraphPainter` with:
  - Node rendering (circles, rectangles, icons)
  - Edge rendering (lines, arrows, curves)
  - Zoom and pan controls
  - Collision detection for overlapping nodes
  - Smooth animations (50ms frame rate)
- Support interactive features:
  - Click to select nodes
  - Drag to reposition
  - Multi-select with Ctrl+click
  - Box select with drag
- Add minimap for large graphs
- Implement level-of-detail (LOD) for performance
**Dependencies**: Task 4.1
**Effort**: L (5-6 days)
**Priority**: High

#### Task 4.3: Create Knowledge Graph Controls
**Location**: `crates/terraphim_egui/src/kg/controls.rs`
**Description**: Build UI controls for graph manipulation
**Implementation**:
- Zoom controls:
  - Zoom slider (10% - 500%)
  - Zoom to fit button
  - Reset zoom button
  - Mouse wheel zoom support
- Pan controls:
  - Arrow buttons for precise positioning
  - Center on selection
  - View history (back/forward)
- Filter controls:
  - Node type checkboxes
  - Search/filter input
  - Tag-based filtering
  - Path highlighting
- Layout controls:
  - Force-directed, hierarchical, circular options
  - Auto-layout toggle
  - Spacing adjustment
**Dependencies**: Task 4.2
**Effort**: S (2 days)
**Priority**: Medium

#### Task 4.4: Integrate RoleGraph with Visualization
**Location**: `crates/terraphim_egui/src/kg/integration.rs`
**Description**: Connect KG visualization to terraphim_rolegraph
**Implementation**:
- Load knowledge graph from terraphim_rolegraph
- Build node/edge data from graph structure
- Implement real-time updates as data changes
- Support subgraph extraction
- Add path finding and highlighting
- Implement "expand node" functionality
- Support collaborative filtering
- Add graph statistics panel (node count, edge count, density)
**Dependencies**: Task 4.3
**Effort**: M (3-4 days)
**Priority**: Medium

---

## Phase 5: Markdown Editor & Viewer

### Module: Document Viewer

#### Task 5.1: Create Markdown Renderer
**Location**: `crates/terraphim_egui/src/ui/markdown/renderer.rs`
**Description**: Convert markdown to egui-renderable content
**Implementation**:
- Adapt terraphim-markdown-parser's pulldown-cmark usage
- Create `MarkdownRenderer` that converts pulldown-cmark events to egui widgets
- Support markdown features:
  - Headers (H1-H6) with anchor links
  - Paragraphs with text formatting
  - Code blocks with syntax highlighting
  - Blockquotes with styling
  - Lists (ordered/unordered)
  - Links (regular and wikilinks [[]])
  - Tables with alignment
  - Images (if needed)
- Implement custom styling for dark/light themes
- Add copy code button for code blocks
- Support [[wikilink]] navigation to knowledge graph
**Dependencies**: Task 2.1
**Effort**: M (3-4 days)
**Priority**: High
**Key Integration**:
```rust
// Use existing pulldown-cmark from terraphim-markdown-parser
use pulldown_cmark::{Parser, Options, Event, Tag};
```

#### Task 5.2: Build Document Viewer Widget
**Location**: `crates/terraphim_egui/src/ui/markdown/viewer.rs`
**Description**: Create scrollable document viewer panel
**Implementation**:
- Create `DocumentViewer` with:
  - Scrollable content area
  - Table of contents sidebar (auto-generated from headers)
  - Breadcrumb navigation
  - Reading progress indicator
  - Full-text search within document
  - Print styles
- Add reading preferences:
  - Font size adjustment
  - Line height adjustment
  - Page width control
  - Focus mode (hide chrome)
- Implement document metadata display
- Add sharing functionality
**Dependencies**: Task 5.1
**Effort**: M (3 days)
**Priority**: High

#### Task 5.3: Create Markdown Editor (Optional)
**Location**: `crates/terraphim_egui/src/ui/markdown/editor.rs`
**Description**: Build in-place markdown editor with live preview
**Implementation**:
- Create split-pane editor:
  - Left: text editor with markdown syntax highlighting
  - Right: live preview using MarkdownRenderer
- Add toolbar with markdown shortcuts
- Implement auto-save functionality
- Support multiple documents/tabs
- Add document outline/structure view
- Implement find and replace
- Support drag-and-drop file opening
- Add document templates
**Dependencies**: Task 5.2
**Effort**: L (7-10 days)
**Priority**: Low (Phase 2)

---

## Phase 6: Context Management

### Module: Context Panel

#### Task 6.1: Design Context Data Structures
**Location**: `crates/terraphim_egui/src/context/mod.rs`
**Description**: Define context management architecture
**Implementation**:
- Create `ContextItem` enum:
  - `Document { id, title, excerpt, source }`
  - `Concept { term, definition, related_docs }`
  - `KGTerm { node_id, label, metadata }`
  - `Note { id, content, created_at }`
- Create `ContextManager`:
  - Vector of ContextItems with order
  - Character count tracking (with limits)
  - Duplicate detection and merging
  - Export functionality (JSON, Markdown, plain text)
  - Persistence to disk
- Add context validation (max size, type restrictions)
**Dependencies**: Task 3.3 (needs search results)
**Effort**: S (2 days)
**Priority**: High

#### Task 6.2: Build Context Panel UI
**Location**: `crates/terraphim_egui/src/ui/context/panel.rs`
**Description**: Create context management interface
**Implementation**:
- Create `ContextPanel` with:
  - Item list with icons for each type
  - Drag-and-drop reordering
  - Multi-select with checkboxes
  - Preview on hover
  - Remove button for each item
  - "Clear All" button with confirmation
- Display context statistics:
  - Total character count
  - Item count by type
  - Size gauge/indicator
- Add bulk actions:
  - "Add All Search Results"
  - "Remove Selected"
  - "Export Selected"
- Implement context searching and filtering
**Dependencies**: Task 6.1
**Effort**: M (3 days)
**Priority**: High

#### Task 6.3: Implement "Add to Context" Workflows
**Location**: `crates/terraphim_egui/src/context/integration.rs`
**Description**: Connect context building to search and KG
**Implementation**:
- Add "Add to Context" button to search results
- Add context menu to KG nodes (right-click)
- Implement batch selection:
  - "Select All Visible"
  - "Select by Type"
  - "Select by Source"
- Add keyboard shortcuts:
  - Ctrl+Click for multi-select
  - A to add all visible
  - Delete to remove selected
- Support automatic context from LLM responses
- Add context templates (e.g., "Code Review", "Research")
- Implement context sharing between tabs
**Dependencies**: Task 6.2
**Effort**: M (3-4 days)
**Priority**: High

---

## Phase 7: LLM Chat Interface

### Module: Chat Panel

#### Task 7.1: Create Chat Message Components
**Location**: `crates/terraphim_egui/src/ui/chat/messages.rs`
**Description**: Build chat message widgets
**Implementation**:
- Create `Message` struct with:
  - role (user, assistant, system)
  - content (markdown formatted)
  - timestamp
  - metadata (tokens, model, etc.)
- Create message widgets:
  - `UserMessage`: right-aligned, user-colored bubble
  - `AssistantMessage`: left-aligned, assistant-colored bubble
  - `SystemMessage`: centered, neutral styling
- Implement markdown rendering in messages
- Add copy button for each message
- Add message actions (regenerate, edit, delete)
- Support code syntax highlighting
- Add message threading/replies
**Dependencies**: Task 2.1
**Effort**: M (3-4 days)
**Priority**: High

#### Task 7.2: Build Chat Input Interface
**Location**: `crates/terraphim_egui/src/ui/chat/input.rs`
**Description**: Create chat input with context integration
**Implementation**:
- Create `ChatInput` with:
  - Multi-line text editor
  - Send button (Enter to send, Shift+Enter for newline)
  - Context indicator (shows context size)
  - "Use Context" toggle
  - Character count and limits
  - Auto-resize as user types
- Add quick action buttons:
  - "Summarize Context"
  - "Ask about Context"
  - "Generate from Selection"
- Implement input history
- Add keyboard shortcuts
- Support pasting images (for multi-modal models)
- Add command palette (/help, /context, /clear)
**Dependencies**: Task 6.3
**Effort**: M (3 days)
**Priority**: High

#### Task 7.3: Implement LLM Integration
**Location**: `crates/terraphim_egui/src/chat/llm.rs`
**Description**: Connect chat to terraphim_service LLM functionality
**Implementation**:
- Create `LLMService` wrapper around terraphim_service
- Support multiple providers:
  - OpenRouter (with openrouter feature flag)
  - Ollama (local models)
  - Generic LLM interface
- Implement streaming responses:
  - Real-time token streaming
  - Typing indicator
  - Cancel button
  - Progress tracking
- Handle conversation context:
  - Automatic context inclusion
  - Context summary for long conversations
  - Token limit management
- Add conversation persistence
- Implement model switching
- Add error handling and retry logic
**Dependencies**: Task 7.2
**Effort**: M (4-5 days)
**Priority**: High
**Key Integration**:
```rust
// Use existing terraphim_service chat functionality
use terraphim_service::chat_with_context;
```

#### Task 7.4: Add Chat Features
**Location**: `crates/terraphim_egui/src/chat/features.rs`
**Description**: Implement advanced chat functionality
**Implementation**:
- Conversation management:
  - New conversation
  - Load/save conversations
  - Delete conversations
  - Conversation history sidebar
- Message features:
  - Regenerate last response
  - Edit and resend messages
  - Copy message as code/markdown
  - Bookmark important messages
- Advanced context features:
  - Context preview
  - Context editing
  - Context templates
  - Context sharing
- Export conversations (Markdown, JSON)
- Conversation search
- Conversation analytics (token usage, cost)
**Dependencies**: Task 7.3
**Effort**: M (3-4 days)
**Priority**: Medium

---

## Phase 8: Role-Based Configuration

### Module: Configuration Panel

#### Task 8.1: Create Role Selection UI
**Location**: `crates/terraphim_egui/src/ui/config/roles.rs`
**Description**: Build role management interface
**Implementation**:
- Create `RoleSelector` with:
  - List of available roles
  - Role descriptions and metadata
  - Current role indicator
  - Role preview (theme, settings)
- Implement role switching:
  - Instant application of new role
  - Confirmation for destructive changes
  - Rollback option
- Add role creation wizard:
  - Choose role template
  - Configure name, description, theme
  - Select relevance function
  - Configure haystacks
- Support role importing/exporting
- Add role validation and error handling
**Dependencies**: Task 2.1
**Effort**: M (3-4 days)
**Priority**: Medium
**Key Integration**:
```rust
// Use existing terraphim_config for role management
use terraphim_config::{load_role, list_roles, save_role};
```

#### Task 8.2: Build LLM Configuration Interface
**Location**: `crates/terraphim_egui/src/ui/config/llm.rs`
**Description**: Create LLM provider settings panel
**Implementation**:
- Create `LLMConfig` panel with:
  - Provider selection (OpenRouter, Ollama, Generic)
  - Provider-specific settings:
    - OpenRouter: API key, model selection
    - Ollama: base URL, local model name
    - Generic: endpoint, authentication
  - Connection test button
  - Model list with descriptions
  - Token limits and pricing info
- Add advanced settings:
  - Temperature, top-p, max tokens
  - System prompt editor
  - Custom parameters
- Implement settings validation
- Add settings templates
- Test connection on save
**Dependencies**: Task 8.1
**Effort**: M (3 days)
**Priority**: Medium

#### Task 8.3: Create Theme Configuration
**Location**: `crates/terraphim_egui/src/ui/config/theme.rs`
**Description**: Build theme customization interface
**Implementation**:
- Create `ThemeConfig` with:
  - Predefined theme selection
  - Custom color picker
  - Font family and size controls
  - Accent color settings
  - Preview panel
- Support role-specific themes:
  - Auto-apply theme on role switch
  - Custom themes per role
  - Import/export theme files
- Add accessibility options:
  - High contrast mode
  - Reduced motion
  - Font size scaling
  - Color blind friendly palettes
- Implement theme reset
- Add random theme generator
**Dependencies**: Task 8.2
**Effort**: S (2 days)
**Priority**: Low

#### Task 8.4: Keyboard Shortcuts Configuration
**Location**: `crates/terraphim_egui/src/ui/config/shortcuts.rs`
**Description**: Create keyboard shortcut customization
**Implementation**:
- Create `ShortcutsConfig` with:
  - List of all shortcuts with descriptions
  - Key combination display
  - "Press key to rebind" functionality
  - Search shortcuts
  - Reset to defaults
- Predefined shortcuts:
  - Global: Ctrl+Shift+T (show/hide)
  - Search: Ctrl+F, Enter, Tab
  - Chat: Ctrl+Enter (send), Ctrl+L (clear)
  - Context: Ctrl+A (add all), Delete (remove)
  - Navigation: Ctrl+1-6 (switch tabs)
  - KG: Ctrl+= (zoom in), Ctrl+- (zoom out)
- Add shortcut conflicts detection
- Support multiple profiles
- Export/import shortcut configs
**Dependencies**: Task 8.3
**Effort**: S (2 days)
**Priority**: Low

---

## Phase 9: Session Management

### Module: Sessions Panel

#### Task 9.1: Design Session Data Model
**Location**: `crates/terraphim_egui/src/sessions/mod.rs`
**Description**: Define session persistence structure
**Implementation**:
- Create `Session` struct with:
  - id (ULID), name, created_at, last_modified
  - current_role
  - search_history
  - context_items
  - conversations
  - layout_state (open tabs, panel sizes)
  - bookmarks and notes
- Implement `SessionManager`:
  - Save/load sessions
  - Auto-save on changes
  - Session templates
  - Session export/import
  - Session search and filtering
- Add session validation and migration
- Implement session versioning
**Dependencies**: Task 7.4
**Effort**: M (3 days)
**Priority**: Medium

#### Task 9.2: Build Sessions Panel UI
**Location**: `crates/terraphim_egui/src/ui/sessions/panel.rs`
**Description**: Create session management interface
**Implementation**:
- Create `SessionsPanel` with:
  - Session list with:
    - Session name
    - Last modified timestamp
    - Role indicator
    - Preview of recent activity
  - New session button
  - Duplicate session button
  - Delete session with confirmation
  - Export session button
  - Search/filter sessions
- Implement session actions:
  - Load session (apply state)
  - Rename session
  - Create from template
  - Create snapshot
- Add recent sessions sidebar
- Implement drag-and-drop reordering
- Add keyboard shortcuts
**Dependencies**: Task 9.1
**Effort**: M (3 days)
**Priority**: Medium

#### Task 9.3: Auto-Save & Recovery System
**Location**: `crates/terraphim_egui/src/sessions/persistence.rs`
**Description**: Implement automatic session saving
**Implementation**:
- Implement auto-save:
  - Save on every change (debounced)
  - Background save without blocking UI
  - Incremental saves (only changed data)
  - Save status indicator
- Add recovery features:
  - Crash recovery on startup
  - Unsaved changes warning
  - Backup session files
  - Session history (multiple checkpoints)
- Implement session locking:
  - Prevent concurrent modifications
  - Show last saved timestamp
  - Force save option
- Add data integrity checks
- Implement session cleanup (old sessions)
**Dependencies**: Task 9.2
**Effort**: M (3-4 days)
**Priority**: Medium

---

## Phase 10: Testing, Polish & Performance

### Module: Testing

#### Task 10.1: Unit Testing Suite
**Location**: `crates/terraphim_egui/tests/unit/`
**Description**: Create comprehensive unit tests
**Implementation**:
- Test state management:
  - State initialization
  - Thread-safe access
  - State transitions
- Test UI components:
  - Widget rendering
  - User interactions
  - Event handling
- Test business logic:
  - Search integration
  - Context management
  - Chat functionality
- Test configuration:
  - Role switching
  - Settings persistence
  - Theme application
- Use tokio::test for async tests
- Add golden file tests for UI rendering
- Mock external dependencies
**Effort**: L (5-7 days)
**Priority**: High

#### Task 10.2: Integration Testing
**Location**: `crates/terraphim_egui/tests/integration/`
**Description**: Test cross-crate integration
**Implementation**:
- End-to-end workflow tests:
  - Search → Add to Context → Chat
  - Role switch → Verify settings
  - KG exploration → Add to Context
  - Session save/load
- Performance tests:
  - Autocomplete latency (<50ms)
  - Search response time (<200ms)
  - UI frame rate (60 FPS)
  - Memory usage
- API integration tests:
  - terraphim_service integration
  - terraphim_automata integration
  - terraphim_rolegraph integration
- Error handling tests:
  - Network failures
  - Invalid data
  - Edge cases
**Dependencies**: Tasks 10.1
**Effort**: L (5-7 days)
**Priority**: High

#### Task 10.3: UI/UX Testing & Refinement
**Location**: `crates/terraphim_egui/tests/e2e/`
**Description**: Comprehensive user journey testing
**Implementation**:
- User journey validation:
  - Complete search workflow
  - Full LLM chat session
  - Knowledge graph exploration
  - Role switching
  - Session management
- Accessibility testing:
  - Keyboard navigation
  - Screen reader compatibility
  - Color contrast
  - Focus management
- Multi-platform testing:
  - Windows
  - macOS
  - Linux
- Performance profiling:
  - Memory leaks
  - CPU usage
  - GPU rendering
- User feedback integration
- A/B testing for UI changes
**Dependencies**: Tasks 10.2
**Effort**: M (3-5 days)
**Priority**: High

### Module: Performance Optimization

#### Task 10.4: Autocomplete Performance
**Location**: `crates/terraphim_egui/src/ui/search/performance.rs`
**Description**: Optimize autocomplete for <50ms target
**Implementation**:
- Profile WASM autocomplete performance
- Optimize AutocompleteIndex loading:
  - Pre-load on startup
  - Cache in memory
  - Background loading
- Implement debouncing (50ms)
- Add request cancellation
- Use worker threads for heavy operations
- Implement progressive enhancement:
  - Show instant results
  - Refine with better matches
- Add performance metrics
- Document performance budget
**Dependencies**: Task 3.2
**Effort**: M (3-4 days)
**Priority**: High

#### Task 10.5: Memory Management
**Location**: `crates/terraphim_egui/src/memory.rs`
**Description**: Optimize memory usage
**Implementation**:
- Profile memory usage:
  - Total memory footprint
  - Per-panel memory
  - Cache efficiency
- Implement memory optimization:
  - Lazy loading of panels
  - Virtual scrolling for large lists
  - Cache eviction policies
  - Resource pooling
- Add memory monitoring:
  - Real-time memory display
  - Memory warnings
  - Garbage collection triggers
- Optimize data structures:
  - Use appropriate collections
  - Avoid unnecessary cloning
  - Implement Copy where possible
- Clean up resources on close
- Test on low-memory systems
**Dependencies**: Task 10.4
**Effort**: M (3-4 days)
**Priority**: High

#### Task 10.6: UI Responsiveness
**Location**: `crates/terraphim_egui/src/ui/perf.rs`
**Description**: Ensure 60 FPS UI rendering
**Implementation**:
- Profile UI rendering:
  - Identify slow widgets
  - Measure frame times
  - Find rendering bottlenecks
- Optimize rendering:
  - Cache complex widgets
  - Use dirty regions
  - Minimize repaints
  - Batch draw calls
- Implement async UI updates:
  - Non-blocking operations
  - Progress indicators
  - Background tasks
- Add frame rate monitoring
- Optimize layout calculations
- Use egui's built-in optimizations
- Test on low-end hardware
**Dependencies**: Task 10.5
**Effort**: M (3-4 days)
**Priority**: High

### Module: Error Handling & UX Polish

#### Task 10.7: Comprehensive Error Handling
**Location**: `crates/terraphim_egui/src/error.rs`
**Description**: Add robust error handling
**Implementation**:
- Create error hierarchy:
  - `EguiError` enum
  - User-friendly error messages
  - Error context and suggestions
- Implement error UI:
  - Error notifications
  - Error dialogs
  - Toast notifications
  - Error history
- Handle specific errors:
  - Network failures
  - Invalid configurations
  - Missing files
  - LLM API errors
  - Parsing errors
- Add error recovery:
  - Auto-retry mechanisms
  - Fallback strategies
  - Graceful degradation
- Log errors for debugging
- Add error reporting (opt-in)
**Dependencies**: Task 7.3
**Effort**: M (3 days)
**Priority**: High

#### Task 10.8: Loading States & Progress
**Location**: `crates/terraphim_egui/src/ui/loading.rs`
**Description**: Add loading indicators and progress feedback
**Implementation**:
- Create loading widgets:
  - Spinner animation
  - Progress bars
  - Skeleton screens
  - Pulse indicators
- Add loading states for:
  - Autocomplete loading
  - Search execution
  - LLM response streaming
  - Document loading
  - Session save/load
  - Configuration changes
- Implement progressive loading:
  - Show partial results
  - Update in real-time
  - Cancel support
- Add estimated time remaining
- Customize loading messages
- Support custom loading animations
- Test loading performance
**Dependencies**: Task 10.7
**Effort**: S (2 days)
**Priority**: Medium

#### Task 10.9: Accessibility & Internationalization
**Location**: `crates/terraphim_egui/src/ui/a11y.rs`
**Description**: Ensure accessibility and add i18n support
**Implementation**:
- Accessibility features:
  - Screen reader support
  - Keyboard navigation
  - Focus management
  - High contrast mode
  - Font size scaling
  - Color blind friendly
- Internationalization:
  - Text externalization
  - Locale detection
  - RTL language support
  - Date/time formatting
  - Number formatting
- Add ARIA labels
- Implement keyboard shortcuts help
- Test with accessibility tools
- Document accessibility features
- Add accessibility testing suite
**Dependencies**: Task 10.8
**Effort**: M (3-4 days)
**Priority**: Medium

---

## Critical Path & Dependencies Map

### Phase Dependencies (Critical Path)
```
Phase 1 (Foundation)
  ↓
Phase 2 (UI Framework)
  ↓
Phase 3 (Search & Autocomplete) ← Critical: Performance target <50ms
  ↓
Phase 6 (Context Management) ← Depends on search results
  ↓
Phase 7 (LLM Chat) ← Depends on context
  ↓
Phase 10 (Testing & Polish)
```

### Module Interdependencies
```
Search Panel
  ├── Needs: WASM Autocomplete (Task 3.2)
  └── Feeds: Context Management (Task 6.3)

Knowledge Graph
  ├── Uses: terraphim_rolegraph
  └── Feeds: Context Management (KG nodes)

Chat
  ├── Needs: Context Manager
  ├── Uses: terraphim_service LLM
  └── Feeds: Session Management

Configuration
  ├── Affects: All panels (theme, role)
  └── Uses: terraphim_config

Sessions
  ├── Saves: All application state
  └── Needs: All features implemented
```

### Parallelizable Tasks
Tasks that can be developed in parallel after Phase 1-2:
- Search Panel (3) and Knowledge Graph (4) - both UI panels
- Context (6) and Chat (7) - can develop independently
- Configuration (8) and Sessions (9) - can develop independently
- All testing tasks (10.1-10.3) can run in parallel with implementation

---

## Performance Targets

| Feature | Target | Measurement |
|---------|--------|-------------|
| Autocomplete Response | <50ms | WASM function call time |
| Search Execution | <200ms | Query to results display |
| UI Frame Rate | 60 FPS | Consistent rendering |
| Memory Usage | <500MB | Normal operation |
| Startup Time | <3s | Cold start to usable |
| LLM Streaming | Real-time | Token-by-token display |
| KG Rendering | 60 FPS | Smooth zoom/pan |
| Context Building | <100ms | Add item to context |

---

## Testing Requirements

### Unit Tests (per task)
- All public APIs must have unit tests
- Edge cases and error conditions
- Async operations with tokio::test
- Mock external dependencies
- Coverage target: 80%+

### Integration Tests
- Cross-crate functionality
- End-to-end workflows
- Performance benchmarks
- Error handling paths
- Test in CI pipeline

### E2E Tests
- Complete user journeys
- Multi-platform testing
- Accessibility validation
- Performance profiling
- Manual testing checklist

### Test Commands
```bash
# Unit tests
cargo test -p terraphim_egui

# Integration tests
cargo test -p terraphim_egui --test integration

# E2E tests
cargo test -p terraphim_egui --test e2e

# With features
cargo test -p terraphim_egui --features openrouter
cargo test -p terraphim_egui --features ollama

# Performance tests
cargo test -p terraphim_egui --release --features performance
```

---

## Integration Points with Existing Crates

### terraphim_automata
**Purpose**: WASM-based autocomplete
**Key Functions**:
- `fuzzy_autocomplete_search_jaro_winkler()`
- `load_autocomplete_index()`
- `AutocompleteIndex` data structure
**Usage**: Task 3.2 (Autocomplete Integration)

### terraphim_rolegraph
**Purpose**: Knowledge graph management
**Key Functions**:
- Build and query knowledge graphs
- Node/edge relationships
- Graph algorithms
**Usage**: Task 4.4 (KG Integration)

### terraphim_service
**Purpose**: Search and LLM integration
**Key Functions**:
- `search_documents()`
- `chat_with_context()`
- Role-based search
**Usage**: Task 3.4 (Search Logic), Task 7.3 (LLM Integration)

### terraphim_config
**Purpose**: Configuration management
**Key Functions**:
- `load_role()`, `save_role()`
- `list_roles()`
- Role validation
**Usage**: Task 8.1 (Role Selection)

### terraphim_types
**Purpose**: Shared data structures
**Key Types**:
- `Document`, `SearchQuery`, `SearchResult`
- `RoleName`, `NormalizedTermValue`
**Usage**: Throughout application

### terraphim_persistence
**Purpose**: Data persistence
**Key Functions**:
- Save/load application state
- Session management
- Conversation history
**Usage**: Task 9.1 (Session Management)

### terraphim-markdown-parser
**Purpose**: Markdown parsing
**Key Components**:
- pulldown-cmark integration
- Custom markdown extensions
**Usage**: Task 5.1 (Markdown Renderer)

---

## File Structure Reference

```
crates/terraphim_egui/
├── Cargo.toml
├── src/
│   ├── main.rs              (Entry point)
│   ├── lib.rs               (Crate root)
│   ├── app.rs               (Main application)
│   ├── state.rs             (Application state)
│   ├── error.rs             (Error handling)
│   ├── memory.rs            (Memory management)
│   ├── ui/                  (All UI components)
│   │   ├── mod.rs
│   │   ├── theme.rs         (Visual design system)
│   │   ├── status_bar.rs    (Bottom status bar)
│   │   ├── panels.rs        (Tab management)
│   │   ├── search/          (Search panel)
│   │   │   ├── mod.rs
│   │   │   ├── input.rs     (Search input)
│   │   │   ├── autocomplete.rs
│   │   │   ├── results.rs   (Results display)
│   │   │   └── performance.rs
│   │   ├── kg/              (Knowledge graph)
│   │   │   ├── mod.rs
│   │   │   ├── graph.rs     (Data structures)
│   │   │   ├── painter.rs   (Custom rendering)
│   │   │   ├── controls.rs  (UI controls)
│   │   │   └── integration.rs
│   │   ├── markdown/        (Document viewer)
│   │   │   ├── mod.rs
│   │   │   ├── renderer.rs  (Markdown → egui)
│   │   │   ├── viewer.rs    (Viewer widget)
│   │   │   └── editor.rs    (Optional editor)
│   │   ├── context/         (Context management)
│   │   │   ├── mod.rs
│   │   │   ├── panel.rs     (Context UI)
│   │   │   └── integration.rs
│   │   ├── chat/            (LLM chat)
│   │   │   ├── mod.rs
│   │   │   ├── messages.rs  (Message widgets)
│   │   │   ├── input.rs     (Chat input)
│   │   │   └── ui.rs        (Chat panel)
│   │   ├── config/          (Configuration)
│   │   │   ├── mod.rs
│   │   │   ├── roles.rs     (Role selection)
│   │   │   ├── llm.rs       (LLM settings)
│   │   │   ├── theme.rs     (Theme config)
│   │   │   └── shortcuts.rs (Keyboard shortcuts)
│   │   ├── sessions/        (Session management)
│   │   │   ├── mod.rs
│   │   │   ├── panel.rs     (Sessions UI)
│   │   │   └── persistence.rs
│   │   ├── loading.rs       (Loading states)
│   │   ├── a11y.rs          (Accessibility)
│   │   └── perf.rs          (Performance monitoring)
│   ├── logic/               (Business logic)
│   │   ├── mod.rs
│   │   ├── search.rs        (Search orchestration)
│   │   ├── chat.rs          (Chat orchestration)
│   │   └── context.rs       (Context logic)
│   ├── kg/                  (KG logic)
│   │   ├── mod.rs
│   │   ├── layout.rs        (Graph layout algorithms)
│   │   └── algorithms.rs    (Graph algorithms)
│   ├── chat/                (Chat logic)
│   │   ├── mod.rs
│   │   ├── llm.rs           (LLM integration)
│   │   └── features.rs      (Advanced features)
│   └── sessions/            (Session logic)
│       ├── mod.rs
│       └── manager.rs       (Session management)
└── tests/
    ├── unit/
    ├── integration/
    └── e2e/
```

---

## Effort Estimates Summary

| Phase | Module | Tasks | Total Effort |
|-------|--------|-------|--------------|
| 1 | Project Setup | 4 | 10-13 days |
| 2 | UI Framework | 3 | 8-9 days |
| 3 | Search & Autocomplete | 4 | 12-15 days |
| 4 | Knowledge Graph | 4 | 12-14 days |
| 5 | Markdown Editor | 3 | 10-17 days (3 optional) |
| 6 | Context Management | 3 | 8-9 days |
| 7 | LLM Chat | 4 | 13-15 days |
| 8 | Configuration | 4 | 10-11 days |
| 9 | Session Management | 3 | 9-10 days |
| 10 | Testing & Polish | 9 | 25-33 days |

**Total Estimated Effort: 120-156 days** (24-31 weeks, ~6-8 months)

**Critical Path: 90-110 days** (18-22 weeks, ~4.5-5.5 months)

---

## Priority Matrix

### High Priority (Must Have - Phase 1)
- Task 1.1-1.3: Foundation
- Task 2.1: Tabbed Panel Infrastructure
- Task 3.1-3.4: Search & Autocomplete
- Task 6.1-6.3: Context Management
- Task 7.1-7.3: LLM Chat Interface
- Task 10.1-10.3: Core Testing
- Task 10.4-10.6: Performance Optimization

### Medium Priority (Should Have - Phase 2)
- Task 1.4: Global Shortcuts
- Task 2.2-2.3: UI Polish
- Task 4.1-4.4: Knowledge Graph
- Task 8.1-8.2: Configuration
- Task 9.1-9.3: Sessions
- Task 10.7-10.8: UX Polish

### Low Priority (Nice to Have - Phase 3)
- Task 5.3: Markdown Editor
- Task 8.3-8.4: Advanced Configuration
- Task 10.9: Accessibility & i18n

---

## Key Success Criteria

1. **Autocomplete Performance**: <50ms response time consistently
2. **Search Experience**: <200ms from query to results, smooth selection
3. **UI Responsiveness**: 60 FPS rendering, no stuttering
4. **Memory Efficiency**: <500MB typical usage, no memory leaks
5. **Context Building**: Intuitive multi-select, clear visual feedback
6. **LLM Chat**: Real-time streaming, context-aware responses
7. **Role Switching**: Instant application of role settings
8. **Session Management**: Seamless save/restore, auto-recovery
9. **Cross-Platform**: Works on Windows, macOS, Linux
10. **Testing**: 80%+ code coverage, all critical paths tested

---

## Notes for Developers

### Development Tips
1. **Start with State**: Get the state management right first (Task 1.3)
2. **Profile Early**: Use profiling tools to identify bottlenecks
3. **Incremental**: Build and test each panel independently
4. **Async Patterns**: Use tokio::spawn for background tasks
5. **Error Handling**: Always handle errors gracefully
6. **UI Testing**: Test UI changes in all themes (light/dark)
7. **Performance**: Profile autocomplete performance throughout
8. **Documentation**: Document complex algorithms and data structures

### Common Patterns
- Use `Arc<Mutex<T>>` for shared state across UI and async tasks
- Implement widgets as structs with `update()` and `show()` methods
- Use egui's `Response` for interaction tracking
- Cache expensive computations
- Debounce user input (especially search)
- Use `tokio::time::timeout` for async operations
- Implement proper cleanup in `Drop` traits

### Integration Guidelines
- Prefer composing existing Terraphim crates over rewriting
- Use feature flags to enable/disable functionality
- Keep UI logic separate from business logic
- Use existing error types when possible
- Follow Rust naming conventions (snake_case for functions/variables)

### Testing Strategy
- Write tests as you implement (TDD for critical paths)
- Mock external services for unit tests
- Use integration tests for real service calls
- Profile performance in tests
- Test on all three platforms regularly

---

## Conclusion

This implementation task list provides a comprehensive roadmap for migrating Terraphim AI to an egui-based desktop application. The tasks are organized in 10 phases with clear dependencies, effort estimates, and priorities. By following this roadmap, developers can systematically build a high-performance, native desktop application that leverages all existing Terraphim crates while providing an excellent user experience.

**Key Focus Areas**:
1. **Performance**: Sub-50ms autocomplete, 60 FPS UI
2. **Usability**: Intuitive workflows, clear visual feedback
3. **Integration**: Seamless use of existing crates
4. **Reliability**: Comprehensive testing, error handling
5. **Extensibility**: Modular architecture for future features

**Next Steps**:
1. Review and approve this task list
2. Begin with Phase 1 (Foundation)
3. Set up development environment and build infrastructure
4. Start implementing high-priority tasks
5. Establish testing and CI/CD pipeline
6. Iterate and refine based on feedback

---

*Generated: 2025-11-09*
*Project: Terraphim AI Egui Migration*
*Version: 1.0*
