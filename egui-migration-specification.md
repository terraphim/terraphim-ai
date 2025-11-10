# Terraphim AI Egui Migration Specification

## Table of Contents
1. [Executive Summary](#executive-summary)
2. [Core User Journey](#core-user-journey)
3. [Role-Based Features](#role-based-features)
4. [Technical Architecture](#technical-architecture)
5. [Key Features](#key-features)
6. [UI/UX Design](#uiux-design)
7. [Integration Points](#integration-points)
8. [Performance Targets](#performance-targets)
9. [Implementation Phases](#implementation-phases)
10. [Migration Strategy](#migration-strategy)
11. [Testing Strategy](#testing-strategy)
12. [Risk Assessment](#risk-assessment)

---

## Executive Summary

### Project Goals
Migrate Terraphim AI from its current multi-frontend architecture (TUI, Web, Desktop) to a unified **egui-based** GUI application. The goal is to create a native, responsive desktop application that leverages Rust's egui framework for optimal performance and seamless integration with existing Terraphim crates.

### Key Benefits
- **Unified Architecture**: Single codebase for all desktop functionality
- **Native Performance**: Direct Rust/WASM integration without web browser overhead
- **WASM Autocomplete**: Leveraging existing WASM-based autocomplete for <50ms search
- **Memory Efficiency**: Lower memory footprint compared to Electron/Tauri
- **Deterministic UI**: Immediate mode GUI ensures consistent, predictable rendering
- **Simplified Deployment**: Single binary distribution

### Technology Stack
- **GUI Framework**: egui 0.27+ with eframe
- **Async Runtime**: tokio (existing)
- **WASM Integration**: wasm-bindgen for autocomplete
- **Graphics**: egui's built-in rendering + custom painters for knowledge graphs
- **Text Rendering**: egui's text system with syntax highlighting via pulldown-cmark
- **State Management**: egui's built-in state management + Arc<Mutex<>> for shared state

---

## Core User Journey

### Journey 1: Search Workflow
```
User Action                          System Response
─────────────────────────────────────────────────────────────────
1. Launch Application                → Load active role configuration
2. Click search bar                  → Activate WASM autocomplete
3. Type "async rust"                 → Display autocomplete suggestions (<50ms)
   ├─ "asynchronous programming"     → Show 3-5 top matches
   ├─ "async/await syntax"
   └─ "async runtime tokio"
4. Select result                     → Execute semantic search
5. View results panel                → Display ranked articles with:
                                      - Title
                                      - Excerpt/description
                                      - Relevance score
                                      - Source (haystack)
6. Click article                     → Open article viewer
7. Add to context                    → Add to context panel
```

**Success Criteria**:
- Autocomplete latency < 50ms
- Search results displayed within 200ms
- Smooth selection and navigation

### Journey 2: Context Management
```
User Action                          System Response
─────────────────────────────────────────────────────────────────
1. Search and find articles          → Articles appear in results
2. Select article                    → Highlight in results panel
3. Click "Add to Context" button     → Add to Context Builder panel
4. Review context items              → Show selected items:
                                      - Articles (title, excerpt)
                                      - Individual concepts
                                      - Knowledge graph nodes
5. Add concepts from KG              → Click KG node, add to context
6. Add entire KG subgraph            → Select multiple nodes, add as group
7. Review context summary            → Display:
                                      - Total characters
                                      - Article count
                                      - Concept count
```

**Success Criteria**:
- Easy multi-selection of articles and concepts
- Visual feedback for selected items
- Context size tracking and limits

### Journey 3: LLM Interaction
```
User Action                          System Response
─────────────────────────────────────────────────────────────────
1. Build context                     → Context ready in panel
2. Switch to LLM Chat tab            → Open chat interface
3. Enter question                    → "Explain async/await in Rust"
4. Click "Ask"                       → Send context + question to LLM
5. View streaming response           → Real-time token-by-token display
6. Copy response                     → Click copy button
7. Save to context                   → Add response as new article
8. Continue conversation             → History maintained
```

**Success Criteria**:
- Streaming response display
- Context automatically included in LLM calls
- Copy-to-clipboard functionality
- Conversation history persisted

### Journey 4: Knowledge Graph Exploration
```
User Action                          System Response
─────────────────────────────────────────────────────────────────
1. Open Knowledge Graph tab          → Render KG visualization
2. Pan graph (drag)                  → Smooth navigation
3. Zoom (mouse wheel)                → Zoom in/out smoothly
4. Click node                        → Highlight node + connected edges
5. Hover node                        → Show tooltip with:
                                      - Node ID
                                      - Connected documents
                                      - Related concepts
6. Double-click node                 → Expand to show:
                                      - Associated documents
                                      - Neighboring nodes
7. Select multiple nodes             → Rectangular selection
8. Add to context                    → Batch add to context builder
```

**Success Criteria**:
- 60 FPS rendering for smooth interaction
- Nodes clearly visible at all zoom levels
- Responsive hover and selection feedback

---

## Role-Based Features

### Role: Default
**Configuration Example** (from `terraphim_engineer_config.json`):
```json
{
  "name": "Default",
  "relevance_function": "title-scorer",
  "theme": "spacelab",
  "kg": null,
  "haystacks": [
    {
      "location": "docs/src",
      "service": "Ripgrep",
      "read_only": true
    }
  ],
  "llm_provider": "ollama",
  "ollama_model": "llama3.2:3b"
}
```

**UI Characteristics**:
- **Theme**: Light/dark toggle (spacelab theme)
- **Autocomplete**: Basic FST-based from local haystacks
- **Knowledge Graph**: Minimal or disabled
- **Search Algorithm**: TitleScorer (fast, simple)
- **LLM**: Ollama with 3B model
- **Layout**: Simple two-panel (search + results)

### Role: Rust-Engineer
**Configuration Example**:
```json
{
  "name": "Rust Engineer",
  "relevance_function": "bm25",
  "theme": "dark",
  "kg": {
    "input_type": "markdown",
    "path": "docs/rust",
    "public": true
  },
  "haystacks": [
    {
      "location": ".",
      "service": "Ripgrep",
      "read_only": true
    },
    {
      "service": "QueryRs",
      "extra_parameters": {}
    }
  ],
  "llm_provider": "ollama",
  "ollama_model": "llama3.2:3b",
  "llm_system_prompt": "You are a Rust expert..."
}
```

**UI Characteristics**:
- **Theme**: Dark mode optimized for code
- **Autocomplete**: Enhanced with Rust-specific terms
- **Knowledge Graph**: Full visualization with code symbols
- **Search Algorithm**: BM25 (better for code/documentation)
- **LLM**: Ollama with Rust-focused system prompt
- **Layout**: Three-panel (search + KG + context)
- **Special Features**: Syntax highlighting in results

### Role: Terraphim-Engineer
**Configuration Example**:
```json
{
  "name": "Terraphim Engineer",
  "relevance_function": "terraphim-graph",
  "theme": "lumen",
  "kg": {
    "input_type": "markdown",
    "path": "docs/src/kg",
    "public": true
  },
  "haystacks": [
    {
      "location": "docs/src",
      "service": "Ripgrep",
      "read_only": true
    }
  ],
  "llm_provider": "ollama",
  "ollama_model": "llama3.2:3b",
  "llm_system_prompt": "You are a Terraphim expert..."
}
```

**UI Characteristics**:
- **Theme**: Lumen (bright, documentation-focused)
- **Autocomplete**: Full thesaurus-based with URL links
- **Knowledge Graph**: Interactive with full node/edge details
- **Search Algorithm**: TerraphimGraph (semantic, relationship-aware)
- **LLM**: Ollama with Terraphim-specific prompts
- **Layout**: Four-panel (search + KG + context + chat)
- **Special Features**: URL navigation, concept relationships

### Role-Based UI Customization
```rust
// Role-specific UI configuration
#[derive(Clone)]
pub struct RoleUIConfig {
    pub theme: ThemeVariant,
    pub layout: LayoutVariant,
    pub autocomplete_enabled: bool,
    pub kg_visualization_level: KGDetailLevel,
    pub default_search_algorithm: RelevanceFunction,
    pub llm_integration_enabled: bool,
}

// Themes
enum ThemeVariant {
    Spacelab,    // Light, clean
    Dark,        // Dark for code
    Lumen,       // Bright for docs
    Custom(String),
}

// Layouts
enum LayoutVariant {
    Simple,      // 2 panels
    Standard,    // 3 panels
    Advanced,    // 4 panels
}
```

---

## Technical Architecture

### System Architecture Overview
```
┌─────────────────────────────────────────────────────────────────┐
│                     Egui Application                            │
│  ┌───────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │  Search Panel │  │  KG Panel    │  │  Context Builder     │ │
│  │  (Autocomplete│  │  (Graph Viz) │  │  (Articles+Concepts) │ │
│  │   + Results)  │  │              │  │                      │ │
│  └───────────────┘  └──────────────┘  └──────────────────────┘ │
│  ┌───────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │  Role Manager │  │  LLM Chat    │  │  Settings/Config     │ │
│  │  (Switch Role)│  │  (Streaming) │  │                      │ │
│  └───────────────┘  └──────────────┘  └──────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Integration Layer (Rust)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ Terraphim    │  │ Automata     │  │ Rolegraph              │ │
│  │ Service      │  │ (WASM)       │  │ (Knowledge Graph)      │ │
│  └──────────────┘  └──────────────┘  └────────────────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ Config       │  │ LLM Client   │  │ Persistence            │ │
│  │ (Roles)      │  │ (Ollama/     │  │ (Documents)            │ │
│  │              │  │  OpenRouter) │  │                        │ │
│  └──────────────┘  └──────────────┘  └────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. Egui Application Structure
```rust
// main.rs
use eframe::egui;

struct TerraphimApp {
    // Core services
    service: TerraphimService,
    role_manager: RoleManager,
    autocomplete: AutocompleteEngine,
    
    // UI state
    search_query: String,
    search_results: Vec<SearchResult>,
    selected_role: RoleName,
    context_items: Vec<ContextItem>,
    kg_state: KnowledgeGraphState,
    
    // Panels visibility
    show_kg_panel: bool,
    show_context_panel: bool,
    show_chat_panel: bool,
}

impl eframe::App for TerraphimApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Central update loop
        self.handle_input(ctx);
        self.render_ui(ctx);
        self.update_services(ctx);
    }
}
```

#### 2. Search Panel with Autocomplete
```rust
// search_panel.rs
pub struct SearchPanel {
    query: String,
    autocomplete_results: Vec<AutocompleteResult>,
    search_results: Vec<SearchResult>,
    is_loading: bool,
    selected_result_idx: usize,
}

impl SearchPanel {
    fn render(&mut self, ctx: &egui::Context, service: &mut TerraphimService) {
        egui::TopBottomPanel::top("search_panel").show(ctx, |ui| {
            // Search input with autocomplete
            ui.horizontal(|ui| {
                let search_response = egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search articles...")
                    .show(ui);
                
                if search_response.changed() {
                    // Trigger WASM autocomplete (async)
                    self.trigger_autocomplete();
                }
                
                if ui.button("🔍").clicked() {
                    self.execute_search(service);
                }
            });
            
            // Autocomplete dropdown
            if !self.autocomplete_results.is_empty() {
                self.render_autocomplete_dropdown(ui);
            }
        });
        
        // Results list
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_search_results(ui);
        });
    }
    
    fn render_autocomplete_dropdown(&self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            for (idx, result) in self.autocomplete_results.iter().enumerate() {
                if ui.selectable_label(false, &result.term).clicked() {
                    // Select autocomplete result
                    self.select_autocomplete_result(idx);
                }
            }
        });
    }
}
```

#### 3. Knowledge Graph Visualization
```rust
// kg_panel.rs
pub struct KnowledgeGraphPanel {
    nodes: Vec<KGNode>,
    edges: Vec<KGEdge>,
    selected_nodes: HashSet<u64>,
    zoom: f32,
    pan: egui::Pos2,
    show_labels: bool,
}

impl KnowledgeGraphPanel {
    fn render(&mut self, ctx: &egui::Context, rolegraph: &RoleGraph) {
        egui::SidePanel::right("kg_panel")
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("Knowledge Graph");
                
                // Controls
                ui.horizontal(|ui| {
                    if ui.button("Fit").clicked() {
                        self.fit_to_view();
                    }
                    ui.checkbox(&mut self.show_labels, "Labels");
                });
                
                // Graph visualization area
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    self.render_graph_canvas(ui, ctx);
                });
                
                // Selected node details
                if let Some(selected_id) = self.selected_nodes.iter().next() {
                    self.render_node_details(ui, selected_id);
                }
            });
    }
    
    fn render_graph_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(380.0, 600.0),
            egui::Sense::click_and_drag()
        );
        
        // Handle pan and zoom
        self.handle_input(&response, rect);
        
        // Custom painter for graph
        let painter = ui.painter();
        
        // Draw edges
        for edge in &self.edges {
            self.draw_edge(painter, edge, rect);
        }
        
        // Draw nodes
        for node in &self.nodes {
            self.draw_node(painter, node, rect);
        }
        
        // Request repaint for smooth interaction
        ctx.request_repaint();
    }
    
    fn draw_node(&self, painter: &egui::Painter, node: &KGNode, rect: egui::Rect) {
        let screen_pos = self.world_to_screen(node.position, rect);
        let radius = 5.0 * self.zoom;
        
        // Node circle
        let color = if self.selected_nodes.contains(&node.id) {
            egui::Color32::from_rgb(0, 150, 255)
        } else {
            egui::Color32::from_rgb(100, 100, 100)
        };
        
        painter.circle(screen_pos, radius, color, egui::Stroke::none());
        
        // Label
        if self.show_labels && self.zoom > 0.5 {
            painter.text(
                screen_pos + egui::vec2(10, 0),
                egui::Align2::LEFT_CENTER,
                &node.label,
                egui::FontId::default(),
                egui::Color32::WHITE
            );
        }
    }
}
```

#### 4. Context Builder
```rust
// context_panel.rs
pub struct ContextPanel {
    items: Vec<ContextItem>,
    max_chars: usize,
    current_chars: usize,
}

#[derive(Clone)]
pub enum ContextItem {
    Article {
        id: String,
        title: String,
        excerpt: String,
        url: Option<String>,
    },
    Concept {
        id: u64,
        value: NormalizedTermValue,
        url: Option<String>,
    },
    KnowledgeGraph {
        nodes: HashSet<u64>,
        description: String,
    },
}

impl ContextPanel {
    fn render(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("context_panel")
            .default_width(350.0)
            .show(ui, |ui| {
                ui.heading("Context Builder");
                
                // Character count
                ui.label(format!(
                    "Characters: {}/{}",
                    self.current_chars, self.max_chars
                ));
                
                // Progress bar
                ui.add(egui::ProgressBar::new(
                    self.current_chars as f32 / self.max_chars as f32
                ));
                
                // Items list
                scroll_area().show(ui, |ui| {
                    for (idx, item) in self.items.iter().enumerate() {
                        self.render_context_item(ui, idx, item);
                    }
                });
                
                // Actions
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        self.clear();
                    }
                    if ui.button("Export").clicked() {
                        self.export_context();
                    }
                });
            });
    }
    
    fn render_context_item(&self, ui: &mut egui::Ui, idx: usize, item: &ContextItem) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            match item {
                ContextItem::Article { title, excerpt, .. } => {
                    ui.label(egui::RichText::new(title).strong());
                    ui.label(egui::RichText::new(excerpt).small());
                }
                ContextItem::Concept { value, url, .. } => {
                    ui.label(format!("Concept: {}", value));
                    if let Some(url) = url {
                        ui.hyperlink(url);
                    }
                }
                ContextItem::KnowledgeGraph { description, .. } => {
                    ui.label(egui::RichText::new(description).strong());
                }
            }
            
            if ui.button("Remove").clicked() {
                // Remove item
            }
        });
    }
}
```

#### 5. LLM Chat Interface
```rust
// chat_panel.rs
pub struct ChatPanel {
    messages: Vec<ChatMessage>,
    input_text: String,
    is_streaming: bool,
    current_response: String,
}

#[derive(Clone)]
pub struct ChatMessage {
    role: MessageRole,  // System, User, Assistant
    content: String,
    timestamp: DateTime,
}

impl ChatPanel {
    fn render(&mut self, ui: &mut egui::Ui, service: &TerraphimService) {
        egui::TopBottomPanel::bottom("chat_panel")
            .min_height(300.0)
            .max_height(500.0)
            .show(ui, |ui| {
                ui.heading("LLM Chat");
                
                // Messages area
                egui::CentralPanel::default().show(ui, |ui| {
                    scroll_area().vertical().show(ui, |ui| {
                        for message in &self.messages {
                            self.render_message(ui, message);
                        }
                        
                        if self.is_streaming && !self.current_response.is_empty() {
                            self.render_message(ui, &ChatMessage {
                                role: MessageRole::Assistant,
                                content: self.current_response.clone(),
                                timestamp: Utc::now(),
                            });
                        }
                    });
                });
                
                // Input area
                ui.separator();
                ui.horizontal(|ui| {
                    let text_edit = egui::TextEdit::multiline(&mut self.input_text)
                        .hint_text("Ask a question with current context...")
                        .desired_rows(2);
                    
                    ui.add(text_edit);
                    
                    if ui.button("Send").clicked() && !self.input_text.is_empty() {
                        self.send_message(service);
                    }
                });
            });
    }
    
    fn render_message(&self, ui: &mut egui::Ui, message: &ChatMessage) {
        let (bg_color, align) = match message.role {
            MessageRole::User => (egui::Color32::from_rgb(50, 100, 200), egui::Align2::RIGHT),
            MessageRole::Assistant => (egui::Color32::from_rgb(60, 60, 60), egui::Align2::LEFT),
            MessageRole::System => (egui::Color32::from_rgb(80, 80, 80), egui::Align2::CENTER),
        };
        
        egui::Frame::group(ui.style())
            .fill(bg_color)
            .show(ui, |ui| {
                ui.add_space(4.0);
                
                // Render markdown content
                let mut content = message.content.clone();
                ui.horizontal(|ui| {
                    if align == egui::Align2::RIGHT {
                        ui.add_space(ui.available_width());
                    }
                    ui.label(egui::RichText::new(&content).wrap());
                });
                
                ui.add_space(4.0);
            });
    }
    
    async fn send_message(&mut self, service: &TerraphimService) {
        let user_message = self.input_text.clone();
        self.input_text.clear();
        
        // Add user message
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: user_message.clone(),
            timestamp: Utc::now(),
        });
        
        // Build context from context builder
        let context = self.build_context_for_llm();
        
        // Start streaming response
        self.is_streaming = true;
        self.current_response = String::new();
        
        // Stream tokens
        let stream = service.chat_completion(context, user_message).await;
        for token in stream {
            self.current_response.push_str(&token);
            // UI will update on next frame
        }
        
        // Finalize message
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: self.current_response.clone(),
            timestamp: Utc::now(),
        });
        
        self.is_streaming = false;
        self.current_response.clear();
    }
}
```

#### 6. Role Manager
```rust
// role_manager.rs
pub struct RoleManager {
    current_role: RoleName,
    available_roles: Vec<RoleName>,
    config_state: ConfigState,
}

impl RoleManager {
    fn render(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("role_manager").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Current Role:");
                
                // Role dropdown
                egui::ComboBox::from_id_source("role_selector")
                    .selected_text(self.current_role.as_str())
                    .show_ui(ui, |ui| {
                        for role in &self.available_roles {
                            ui.selectable_value(
                                &mut self.current_role,
                                role.clone(),
                                role.as_str()
                            );
                        }
                    });
                
                ui.add_space(20.0);
                
                // Role-specific info
                if let Some(role_config) = self.get_current_role_config() {
                    ui.label(format!(
                        "Search: {} | LLM: {}",
                        role_config.relevance_function,
                        role_config.llm_provider
                    ));
                }
            });
        });
    }
}
```

#### 7. WASM Autocomplete Integration
```rust
// autocomplete.rs
use wasm_bindgen::prelude::*;
use terraphim_automata::{AutocompleteIndex, AutocompleteResult};

// Load WASM module
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["wasm", "terraphim"])]
    fn autocomplete_search(query: &str, max_results: usize) -> JsValue;
}

pub struct AutocompleteEngine {
    index: Option<AutocompleteIndex>,
    cache: AHashMap<String, Vec<AutocompleteResult>>,
}

impl AutocompleteEngine {
    pub fn new() -> Self {
        Self {
            index: None,
            cache: AHashMap::new(),
        }
    }
    
    pub async fn load_thesaurus(&mut self, thesaurus: Thesaurus) -> Result<()> {
        // Build FST-based index
        let index = build_autocomplete_index(thesaurus, None)?;
        self.index = Some(index);
        Ok(())
    }
    
    pub fn search(&self, query: &str) -> Vec<AutocompleteResult> {
        if query.len() < 2 {
            return Vec::new();
        }
        
        // Check cache first
        if let Some(cached) = self.cache.get(query) {
            return cached.clone();
        }
        
        // Query FST index
        if let Some(index) = &self.index {
            let results = index.search(query, 5); // Max 5 results
            self.cache.insert(query.to_string(), results.clone());
            return results;
        }
        
        Vec::new()
    }
}
```

### State Management

```rust
// app_state.rs
pub struct AppState {
    // Core application state
    pub current_role: RoleName,
    pub search_panel: SearchPanel,
    pub kg_panel: KnowledgeGraphPanel,
    pub context_panel: ContextPanel,
    pub chat_panel: ChatPanel,
    
    // Services (shared across UI)
    pub service: Arc<Mutex<TerraphimService>>,
    pub config_state: Arc<ConfigState>,
    pub autocomplete_engine: Arc<RwLock<AutocompleteEngine>>,
    
    // UI state
    pub show_kg_panel: bool,
    pub show_context_panel: bool,
    pub show_chat_panel: bool,
    pub theme: ThemeVariant,
}

impl AppState {
    pub fn new(service: TerraphimService, config_state: ConfigState) -> Self {
        let current_role = config_state.get_default_role().unwrap_or_else(|| "Default".into());
        
        Self {
            current_role,
            search_panel: SearchPanel::new(),
            kg_panel: KnowledgeGraphPanel::new(),
            context_panel: ContextPanel::new(),
            chat_panel: ChatPanel::new(),
            service: Arc::new(Mutex::new(service)),
            config_state: Arc::new(config_state),
            autocomplete_engine: Arc::new(RwLock::new(AutocompleteEngine::new())),
            show_kg_panel: true,
            show_context_panel: true,
            show_chat_panel: true,
            theme: ThemeVariant::default(),
        }
    }
    
    pub fn update(&mut self, ctx: &egui::Context) {
        // Handle async updates
        if ctx.input(|i| i.raw-events.iter().any(|e| matches!(e, egui::Event::Key { .. }))) {
            // Process keyboard shortcuts
        }
        
        // Repaint at 60 FPS for smooth interaction
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
```

### Async Integration
```rust
// async_utils.rs
pub async fn load_role_data(
    role_name: &RoleName,
    service: Arc<Mutex<TerraphimService>>,
    autocomplete: Arc<RwLock<AutocompleteEngine>>,
) -> Result<()> {
    let mut service = service.lock().await;
    
    // Load thesaurus
    let thesaurus = service.ensure_thesaurus_loaded(role_name).await?;
    
    // Update autocomplete index
    let mut autocomplete = autocomplete.write().await;
    autocomplete.load_thesaurus(thesaurus).await?;
    
    // Preload knowledge graph
    // (Implementation depends on RoleGraph API)
    
    Ok(())
}

// Spawn background tasks
tokio::spawn(async move {
    if let Err(e) = load_role_data(role_name, service.clone(), autocomplete.clone()).await {
        log::error!("Failed to load role data: {}", e);
    }
});
```

---

## Key Features

### 1. Autocomplete Search (WASM-Optimized)
**Implementation**:
- FST-based (Finite State Transducer) autocomplete
- WASM integration for sub-50ms response times
- Cached results with LRU eviction
- Fuzzy matching with Jaro-Winkler distance
- Real-time suggestions as user types

**UI Components**:
```rust
// Dropdown with suggestions
for (idx, result) in autocomplete_results.iter().enumerate() {
    let selected = idx == self.selected_index;
    if ui.selectable_label(selected, &result.term).clicked() {
        self.select_result(idx);
    }
    
    // Show metadata on hover
    if ui.is_hovering() {
        self.show_tooltip(result);
    }
}
```

**Performance Target**: < 50ms for 10,000+ term thesaurus

### 2. Knowledge Graph Visualization
**Implementation**:
- Force-directed graph layout
- Custom painter for nodes and edges
- Interactive pan/zoom/select
- Node clustering for large graphs
- Edge bundling for reduced visual complexity

**Key Features**:
- **Pan**: Click and drag background
- **Zoom**: Mouse wheel with zoom-to-cursor
- **Select**: Click node, rectangular multi-select
- **Hover**: Tooltip with node metadata
- **Double-click**: Expand/collapse neighbors
- **Filter**: Show only nodes matching search

**Renderer**:
```rust
fn render_graph(&self, painter: &egui::Painter, rect: egui::Rect) {
    // Apply pan/zoom transform
    let transform = self.get_transform(rect);
    
    // Draw edges with curves
    for edge in &self.edges {
        self.draw_curved_edge(painter, edge, transform);
    }
    
    // Draw nodes with varying sizes
    for node in &self.nodes {
        let size = self.get_node_size(node);
        self.draw_node(painter, node, size, transform);
    }
}
```

### 3. Context Builder
**Purpose**: Curate documents and concepts for LLM context

**Components**:
- **Articles Panel**: Selected search results
- **Concepts Panel**: Individual terms from thesaurus
- **KG Subgraph**: Selected node clusters
- **Size Monitor**: Track character/word limits
- **Export**: JSON/markdown format

**Data Structure**:
```rust
pub struct Context {
    articles: Vec<Document>,
    concepts: Vec<NormalizedTerm>,
    kg_subgraphs: Vec<Subgraph>,
    metadata: ContextMetadata,
}

pub struct ContextMetadata {
    total_characters: usize,
    total_tokens: usize,
    included_at: DateTime<Utc>,
    role: RoleName,
}
```

### 4. LLM Chat Interface
**Features**:
- **Streaming Response**: Token-by-token display
- **Context Injection**: Automatic context inclusion
- **Message History**: Persistent across sessions
- **Role-based Prompts**: System prompts from role config
- **Provider Support**: Ollama, OpenRouter
- **Copy-to-clipboard**: One-click copy responses

**Implementation**:
```rust
pub async fn stream_chat_completion(
    &self,
    context: Context,
    message: String,
) -> impl Stream<Item = String> {
    let mut stream = self.llm_client.chat_completion_stream(
        context,
        message,
    ).await;
    
    StreamExt::chunks(10) // Batch tokens for UI efficiency
        .map(|chunk| chunk.join(" "))
}
```

### 5. Role Switching
**Implementation**:
- Dropdown in top panel
- Instant role switch
- Preserve context across switches
- Load role-specific configurations
- Update UI theme/layout
- Refresh autocomplete index
- Rebuild knowledge graph

**State Preservation**:
```rust
fn switch_role(&mut self, new_role: RoleName) {
    // Save current context
    self.save_current_context();
    
    // Switch role
    self.current_role = new_role;
    
    // Reload data in background
    let service = self.service.clone();
    tokio::spawn(async move {
        load_role_data(&new_role, service, autocomplete).await;
    });
}
```

### 6. Configuration Management
**Screens**:
- **Role Configuration**: Create/edit roles
- **Haystack Management**: Add/remove data sources
- **LLM Settings**: Configure providers
- **KG Settings**: Automata path, public/private
- **Theme Settings**: Colors, layout preferences

**Implementation**:
```rust
pub struct ConfigPanel {
    config_state: Arc<ConfigState>,
    edited_role: Option<Role>,
    dirty: bool,
}

impl ConfigPanel {
    fn render(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Configuration");
            
            // Tabs for different config sections
            egui::Tabs::new("config_tabs")
                .tab("Roles", self.render_roles_tab(ui))
                .tab("Haystacks", self.render_haystacks_tab(ui))
                .tab("LLM", self.render_llm_tab(ui))
                .tab("Themes", self.render_themes_tab(ui))
                .show(ui);
        });
    }
}
```

### 7. Persistence
**Data to Persist**:
- Selected role
- Context builder items
- Chat history
- Panel visibility
- Window size/position
- UI preferences

**Storage Options**:
- **Local JSON**: Simple, human-readable
- **SQLite**: Structured, queryable
- **RocksDB**: High-performance key-value
- **Sled**: Rust-native, ergonomic

**Implementation**:
```rust
pub struct Persistence {
    storage: Arc<dyn Persistable>,
}

impl Persistence {
    pub async fn save(&self, state: &AppState) -> Result<()> {
        let data = serde_json::to_string(state)?;
        self.storage.put("app_state", data).await?;
        Ok(())
    }
    
    pub async fn load(&self) -> Result<AppState> {
        let data = self.storage.get("app_state").await?;
        let state = serde_json::from_str(&data)?;
        Ok(state)
    }
}
```

---

## UI/UX Design

### Main Window Layout

```
┌──────────────────────────────────────────────────────────────────────────┐
│ [Role Selector] [Theme] [Settings]                         [X] [─] [□]  │
├──────────────────────────────────────────────────────────────────────────┤
│ Search: [____________________________________________] [🔍]               │
│        ┌─────────────────────────────────────────────────────┐           │
│        │ autocomplete suggestions appear here               │           │
│        └─────────────────────────────────────────────────────┘           │
│                                                                              │
│ ┌────────────────┐ ┌─────────────────────────────────────────┐             │
│ │ Context        │ │                                         │             │
│ │ Builder        │ │     Search Results                      │             │
│ │                │ │                                         │             │
│ │ ┌─Articles─┐   │ │ ┌─────────────────────────────────────┐ │             │
│ │ │ • Doc 1  │   │ │ │ Rust Async/Await                     │ │             │
│ │ │ • Doc 2  │   │ │ │ Understanding async programming...   │ │             │
│ │ │ • Doc 3  │   │ │ │ Score: 0.95 | Source: docs           │ │             │
│ │ └──────────┘   │ │ └─────────────────────────────────────┘ │             │
│ │                │ │                                         │             │
│ │ ┌─Concepts─┐   │ │ ┌─────────────────────────────────────┐ │             │
│ │ │ async    │   │ │ │ Asynchronous Programming in Rust     │ │             │
│ │ │ await    │   │ │ │ Deep dive into async/await...        │ │             │
│ │ └──────────┘   │ │ │ Score: 0.88 | Source: docs           │ │             │
│ │                │ │ └─────────────────────────────────────┘ │             │
│ │ Total: 2.1k    │ │                                         │             │
│ └────────────────┘ └─────────────────────────────────────────┘             │
│                                                                              │
│ ┌────────────────────────────────────────────────────────────────────┐   │
│ │ Knowledge Graph Visualization                                    │   │
│ │                                                                    │   │
│ │     ●───●          Drag to pan, scroll to zoom                    │   │
│ │      \ / \         Click nodes to view details                    │   │
│ │       ●   ●        Select multiple to add to context              │   │
│ │                                                                    │   │
│ └────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│ ┌────────────────────────────────────────────────────────────────────┐   │
│ │ LLM Chat                                                         │   │
│ │ User: Explain async/await in Rust                                │   │
│ │                                                                    │   │
│ │ Assistant: (streaming response)                                   │   │
│ │ Async/await is syntactic sugar for Rust's async primitives...     │   │
│ │                                                                    │   │
│ │ [Explain async/await in Rust________________________________] [Send]│   │
│ └────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────┘
```

### Responsive Design

**Panel Resizing**:
- All panels are resizable (drag to resize)
- Minimum panel widths enforced
- Automatic layout adjustment
- Saved panel sizes

**Window States**:
- **Normal**: Default layout
- **Maximized**: Full window
- **Compact**: Hide KG panel (2-panel layout)
- **Full Screen**: All panels visible

### Theme System

```rust
// themes.rs
pub enum ThemeVariant {
    Spacelab {
        background: Color32,
        foreground: Color32,
        accent: Color32,
        search_results: Color32,
        selected: Color32,
    },
    Dark {
        // Dark mode colors
    },
    Lumen {
        // Bright colors for documentation
    },
}

impl ThemeVariant {
    pub fn apply(&self, ctx: &egui::Context) {
        match self {
            ThemeVariant::Spacelab { background, foreground, .. } => {
                ctx.set_visuals(egui::Visuals {
                    dark_mode: false,
                    ..Default::default()
                });
            }
            ThemeVariant::Dark { .. } => {
                ctx.set_visuals(egui::Visuals {
                    dark_mode: true,
                    ..Default::default()
                });
            }
        }
    }
}
```

### Search Interface Design

**Components**:
1. **Search Bar**: Prominent at top
   - Placeholder text
   - Keyboard shortcut (Ctrl+K)
   - Clear button
   - Search button

2. **Autocomplete Dropdown**:
   - Appears after 2+ characters
   - Max 5-7 suggestions
   - Keyboard navigation (↑/↓)
   - Selection (Enter)
   - Highlight matching text

3. **Results List**:
   - Scrollable
   - Infinite scroll or pagination
   - Loading indicator
   - Result count display

**Example**:
```
┌────────────────────────────────────────┐
│ Search: rust async [________________] 🔍│
├────────────────────────────────────────┤
│ ┌────────────────────────────────────┐ │
│ │ ▶ async programming                │ │
│ │   Concept | docs/src/kg/async      │ │
│ │                                    │ │
│ │ ▶ asynchronous                     │ │
│ │   Concept | docs/src/kg/async      │ │
│ │                                    │ │
│ │ ▶ await                            │ │
│ │   Concept | docs/src/kg/await      │ │
│ └────────────────────────────────────┘ │
│                                        │
│ Results: 42 articles (23ms)            │
│ ┌────────────────────────────────────┐ │
│ │ ▶ Rust Async/Await                 │ │
│ │    Understanding async/await...    │ │
│ │    Score: 0.95 | Source: docs      │ │
│ └────────────────────────────────────┘ │
│ ┌────────────────────────────────────┐ │
│ │ ▶ The Tokio Crate                  │ │
│ │    Async runtime for Rust...       │ │
│ │    Score: 0.88 | Source: docs      │ │
│ └────────────────────────────────────┘ │
```

### Knowledge Graph Viewer

**Layout**:
- Full-width bottom panel (or right panel)
- Canvas-based rendering
- Controls overlay (top-left)
- Mini-map (optional, top-right)

**Visual Design**:
- **Nodes**: 
  - Size: Based on degree centrality
  - Color: Based on node type (concepts, documents, code)
  - Shape: Circles for concepts, squares for documents
  
- **Edges**:
  - Width: Based on relationship strength
  - Color: Gray (normal), Blue (selected)
  - Style: Straight lines with optional curves

- **Labels**:
  - Font size: Scales with zoom
  - Visibility: Only shown when zoomed in
  - Truncation: Smart truncation for long labels

**Interaction**:
- Pan: Click and drag background
- Zoom: Mouse wheel (zoom to cursor)
- Select: Click node
- Multi-select: Ctrl+click or drag rectangle
- Hover: Show tooltip
- Double-click: Expand neighbors

### Context Manager UI

**Layout**:
- Left sidebar panel (350px default)
- Sections for different item types
- Summary at top
- Actions at bottom

**Design**:
```
┌────────────────────────────────────────┐
│ Context Builder                        │
│                                        │
│ Characters: 2,145 / 32,000            │
│ [████████░░░░░░░░░░░░░░░░░░░] 6.7%     │
│                                        │
│ ┌─ Articles ─────────────────────────┐ │
│ │ ✓ Rust Async/Await                 │ │
│ │   Understanding async/await...     │ │
│ │   [Remove]                         │ │
│ │                                    │ │
│ │ ✓ The Tokio Crate                  │ │
│ │   Async runtime for Rust...        │ │
│ │   [Remove]                         │ │
│ └────────────────────────────────────┘ │
│                                        │
│ ┌─ Concepts ─────────────────────────┐ │
│ │ ✓ async                           │ │
│ │   [Remove]                         │ │
│ │                                    │ │
│ │ ✓ await                            │ │
│ │   [Remove]                         │ │
│ └────────────────────────────────────┘ │
│                                        │
│ ┌─ Knowledge Graph ──────────────────┐ │
│ │ ✓ Rust Async Cluster (12 nodes)   │ │
│ │   [Remove]                         │ │
│ └────────────────────────────────────┘ │
│                                        │
│ ┌─ Summary ─────────────────────────┐ │
│ │ 2 articles, 2 concepts, 1 KG      │ │
│ │ ~2,145 characters                 │ │
│ └────────────────────────────────────┘ │
│                                        │
│ [Clear All] [Export] [Send to LLM]    │
└────────────────────────────────────────┘
```

### LLM Chat Interface

**Layout**:
- Bottom panel (300-500px height)
- Messages area (scrollable)
- Input area (bottom)

**Design**:
```
┌──────────────────────────────────────────────────────────────────────────┐
│ LLM Chat                                                                 │
│                                                                          │
│ ┌─ User ────────────────────────────────────────────────────────────────┐ │
│ │ Explain async/await in Rust                                           │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│ ┌─ Assistant ──────────────────────────────────────────────────────────┐ │
│ │ Async/await is syntactic sugar that makes asynchronous Rust code     │ │
│ │ much more readable. Here's what you need to know:                    │ │
│ │                                                                       │ │
│ │ **Basics**                                                            │ │
│ │ The `async` keyword transforms a function into a future. When you    │ │
│ │ call an async function, it returns immediately with a future.        │ │
│ │                                                                       │ │
│ │ **Key Points**                                                        │ │
│ │ 1. Async functions return `Future` types                             │ │
│ │ 2. You must `.await` to run the future to completion                 │ │
│ │ 3. `await` can only be used inside `async` functions                 │ │
│ │                                                                       │ │
│ │ [Copy] [Add to Context] [Regenerate]                                 │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│ ┌─ User ────────────────────────────────────────────────────────────────┐ │
│ │ How do I use it with Tokio?                                          │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│ ┌─ Assistant (typing...) ──────────────────────────────────────────────┐ │
│ │ To use async/await with Tokio, you need to:                          │ │
│ │                                                                       │ │
│ │ 1. Add `tokio` to your `Cargo.toml`                                 │ │
│ │ 2. Mark your `main` function as `#[tokio::main]`                    │ │
│ │ 3. Use `.await` to call async functions                             │ │
│ │                                                                       │ │
│ │ Example:                                                             │ │
│ │ ```rust                                                              │ │
│ │ #[tokio::main]                                                       │ │
│ │ async fn main() {                                                    │ │
│ │     let data = fetch_data().await;                                   │ │
│ │     println!("{}", data);                                            │ │
│ │ }                                                                    │ │
│ │ ```                                                                  │ │
│ │ [Copy] [Add to Context]                                              │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│ ┌────────────────────────────────────────────────────────────────────┐   │
│ │ How do I use async/await with Tokio? [_________________________]  │   │
│ │                                                                    │   │
│ │ [Attach Context] [Send]                              [●] [■] [×] │   │
│ └────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────┘
```

### Settings/Configuration Screens

**Role Configuration**:
```
┌────────────────────────────────────────────────────────────────────┐
│ Configuration → Role: "Rust Engineer"                        [Save] │
├────────────────────────────────────────────────────────────────────┤
│ ┌─ Basic Settings ─────────────────────────────────────────────────┐ │
│ │ Name:        [Rust Engineer________________________]              │ │
│ │ Short Name:  [rust-eng________________________________]            │ │
│ │                                                              │ │
│ │ Theme:       [Dark                    ▼]                        │ │
│ │ Search:      [BM25                    ▼]                        │ │
│ └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│ ┌─ LLM Configuration ──────────────────────────────────────────────┐ │
│ │ Provider:    [Ollama                  ▼]                        │ │
│ │ Model:       [llama3.2:3b________________________________]      │ │
│ │ Base URL:    [http://127.0.0.1:11434________________]          │ │
│ │                                                              │ │
│ │ System Prompt:                                                │ │
│ │ ┌────────────────────────────────────────────────────────────┐ │ │
│ │ │ You are a Rust expert specializing in async programming... │ │ │
│ │ │ Provide clear, practical examples...                      │ │ │
│ │ └────────────────────────────────────────────────────────────┘ │ │
│ └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│ ┌─ Haystacks ─────────────────────────────────────────────────────┐ │
│ │ ✓ Local Ripgrep                                               │ │
│ │   Location: [./docs/src________________] [Browse...]         │ │
│ │                                                              │ │
│ │ ✓ QueryRs (Reddit + Rust docs)                              │ │
│ │   [Configure]                                                │ │
│ └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│ ┌─ Knowledge Graph ───────────────────────────────────────────────┐ │
│ │ Enable: ✓                                                     │ │
│ │ Path:     [./docs/src/kg_________________________________]     │ │
│ │ [Browse]                                                      │ │
│ │                                                              │ │
│ │ Public:  ☑   Publish: ☑                                      │ │
│ └────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────┘
```

### Keyboard Shortcuts

```
Global Shortcuts:
- Ctrl+N        - New search
- Ctrl+K        - Focus search bar
- Ctrl+1..5     - Switch to panel (1=Search, 2=KG, 3=Context, 4=Chat, 5=Settings)
- Ctrl+T        - Switch role
- Ctrl+Q        - Quit

Search Panel:
- Enter         - Execute search
- ↑/↓           - Navigate autocomplete
- Esc           - Clear search
- Tab           - Accept autocomplete

Knowledge Graph:
- Space (hold)  - Pan
- Ctrl + Wheel  - Zoom
- Click         - Select node
- Ctrl + Click  - Multi-select
- Double-click  - Expand neighbors
- F             - Fit to view
- 0             - Reset zoom

Context Panel:
- Del           - Remove selected item
- Ctrl+A        - Select all
- Ctrl+E        - Export context

Chat Panel:
- Enter         - Send message (with Shift for newline)
- Ctrl+Enter    - Force send
- Up Arrow      - Edit previous message
```

---

## Integration Points

### 1. terraphim_service

**Purpose**: Main service layer for search, LLM integration, and document management

**Key APIs**:
```rust
impl TerraphimService {
    // Search functionality
    pub async fn search(
        &mut self,
        query: &SearchQuery,
        role: &RoleName,
    ) -> Result<Vec<SearchResult>>;
    
    // LLM chat completion
    pub async fn chat_completion(
        &self,
        context: Context,
        message: String,
    ) -> Result<impl Stream<Item = Result<String>>>;
    
    // Summarization
    pub async fn summarize(
        &self,
        documents: Vec<Document>,
    ) -> Result<String>;
    
    // Context management
    pub async fn add_to_context(
        &mut self,
        item: ContextItem,
    ) -> Result<()>;
    
    // Role management
    pub async fn switch_role(
        &mut self,
        role: RoleName,
    ) -> Result<()>;
}
```

**Egui Integration**:
```rust
// In egui update loop
let search_query = self.search_panel.get_query();
if !search_query.is_empty() && self.search_panel.is_search_triggered() {
    let service = self.service.clone();
    let role = self.current_role.clone();
    
    tokio::spawn(async move {
        let results = service.lock().await.search(&search_query, &role).await;
        // Update UI with results
    });
}
```

### 2. terraphim_automata (WASM)

**Purpose**: Autocomplete, text matching, thesaurus management

**Key APIs**:
```rust
impl AutocompleteIndex {
    pub fn search(&self, query: &str, max_results: usize) -> Vec<AutocompleteResult>;
    pub fn autocomplete_with_snippets(&self, query: &str) -> Vec<AutocompleteResult>;
    pub fn get_metadata(&self, term: &str) -> Option<&AutocompleteMetadata>;
}

// Thesaurus loading
pub async fn load_thesaurus(path: &Path) -> Result<Thesaurus>;
pub fn build_thesaurus_from_documents(docs: &[Document]) -> Result<Thesaurus>;
```

**WASM Integration**:
```rust
// Compile to WASM for browser/egui use
#[wasm_bindgen]
pub fn autocomplete_search(query: &str, max_results: usize) -> JsValue {
    let results = index.search(query, max_results);
    JsValue::from_serde(&results).unwrap()
}

// In egui app
pub struct AutocompleteWasm {
    module: Option<WasmModule>,
}

impl AutocompleteWasm {
    pub fn search(&self, query: &str) -> Vec<AutocompleteResult> {
        if let Some(module) = &self.module {
            let results: Vec<AutocompleteResult> = 
                module.autocomplete_search(query, 5).from_serde().unwrap();
            results
        } else {
            Vec::new()
        }
    }
}
```

**Build WASM**:
```bash
# In terraphim_automata directory
wasm-pack build --target web --release

# This produces:
# - pkg/terraphim_automata.js (JS wrapper)
# - pkg/terraphim_automata_bg.wasm (binary)
# - pkg/terraphim_automata.d.ts (TypeScript definitions)
```

### 3. terraphim_rolegraph

**Purpose**: Knowledge graph operations, node/edge management

**Key APIs**:
```rust
impl RoleGraph {
    pub async fn new(role: RoleName, thesaurus: Thesaurus) -> Result<Self>;
    pub fn get_nodes(&self) -> Vec<Node>;
    pub fn get_edges(&self) -> Vec<Edge>;
    pub fn get_connected_nodes(&self, node_id: u64) -> Vec<u64>;
    pub fn get_node_documents(&self, node_id: u64) -> Vec<Document>;
    pub fn is_connected(&self, node1: u64, node2: u64) -> bool;
}
```

**Egui Integration**:
```rust
pub struct KnowledgeGraphState {
    rolegraph: Option<RoleGraph>,
    layout: ForceDirectedLayout,
    selected_nodes: HashSet<u64>,
    visible_nodes: HashSet<u64>,
}

impl KnowledgeGraphState {
    pub fn load_rolegraph(&mut self, rolegraph: RoleGraph) {
        self.rolegraph = Some(rolegraph);
        self.compute_layout();
    }
    
    pub fn render(&self, painter: &egui::Painter) {
        if let Some(rolegraph) = &self.rolegraph {
            for edge in rolegraph.get_edges() {
                self.draw_edge(painter, edge);
            }
            
            for node in rolegraph.get_nodes() {
                self.draw_node(painter, node);
            }
        }
    }
}
```

### 4. terraphim_config

**Purpose**: Role management, configuration loading

**Key APIs**:
```rust
impl ConfigState {
    pub async fn load_config(&mut self) -> Result<()>;
    pub fn get_role(&self, name: &RoleName) -> Option<&Role>;
    pub async fn save_config(&self) -> Result<()>;
    pub fn list_roles(&self) -> Vec<RoleName>;
    pub fn get_default_role(&self) -> Option<RoleName>;
}

pub struct Role {
    pub name: RoleName,
    pub relevance_function: RelevanceFunction,
    pub theme: String,
    pub kg: Option<KnowledgeGraphConfig>,
    pub haystacks: Vec<Haystack>,
    pub llm_provider: String,
    pub ollama_base_url: Option<String>,
    pub ollama_model: Option<String>,
    pub llm_system_prompt: Option<String>,
}
```

**Egui Integration**:
```rust
pub struct RoleManager {
    config_state: Arc<ConfigState>,
    current_role: RoleName,
    available_roles: Vec<RoleName>,
}

impl RoleManager {
    pub async fn load_roles(&mut self) -> Result<()> {
        let config = self.config_state.config.lock().await;
        self.available_roles = config.list_roles();
        drop(config);
        Ok(())
    }
    
    pub async fn switch_role(&mut self, role: RoleName) -> Result<()> {
        let mut service = self.service.lock().await;
        service.switch_role(role.clone()).await?;
        self.current_role = role;
        Ok(())
    }
    
    pub fn render(&self, ui: &mut egui::Ui) {
        egui::ComboBox::from_label("Role")
            .selected_text(self.current_role.as_str())
            .show_ui(ui, |ui| {
                for role in &self.available_roles {
                    ui.selectable_value(
                        &mut self.current_role.clone(),
                        role.clone(),
                        role.as_str()
                    );
                }
            });
    }
}
```

### 5. Existing Configuration Files

**Configuration Loading**:
```rust
pub async fn load_configuration() -> Result<ConfigState> {
    // Load from default location
    let config_path = "terraphim_server/default/terraphim_engineer_config.json";
    
    // Or from TERRAPHIM_CONFIG env var
    if let Ok(env_path) = std::env::var("TERRAPHIM_CONFIG") {
        load_from_path(&env_path).await
    } else {
        load_from_path(config_path).await
    }
}

async fn load_from_path(path: &str) -> Result<ConfigState> {
    let file = tokio::fs::File::open(path).await?;
    let reader = tokio::io::BufReader::new(file);
    let config: Config = serde_json::from_reader(reader)?;
    Ok(ConfigState::new(config))
}
```

**Thesaurus Loading**:
```rust
// Load thesaurus for autocomplete
pub async fn load_thesaurus_for_role(
    role: &Role,
    config_state: &ConfigState,
) -> Result<Thesaurus> {
    if let Some(kg) = &role.kg {
        if let Some(automata_path) = &kg.automata_path {
            return load_thesaurus(Path::new(automata_path)).await;
        }
        
        if let Some(kg_config) = &kg.knowledge_graph_local {
            // Load from local path
            return load_thesaurus_from_path(&kg_config.path).await;
        }
    }
    
    // Default: build from haystacks
    let documents = load_documents_from_haystacks(&role.haystacks).await?;
    build_thesaurus_from_documents(&documents)
}
```

### 6. LLM Provider Integration

**Ollama Integration**:
```rust
pub struct OllamaClient {
    base_url: String,
    model: String,
    http_client: reqwest::Client,
}

impl OllamaClient {
    pub async fn chat_completion(
        &self,
        context: Context,
        message: String,
    ) -> Result<impl Stream<Item = Result<String>>> {
        let prompt = self.build_prompt(context, message);
        
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
            stream: true,
        };
        
        let response = self.http_client
            .post(&format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;
            
        Ok(self.parse_stream(response).await)
    }
    
    fn build_prompt(&self, context: Context, message: String) -> String {
        let context_text = context.to_markdown();
        format!(
            "Context:\n{}\n\nUser Question:\n{}",
            context_text, message
        )
    }
}
```

**OpenRouter Integration**:
```rust
#[cfg(feature = "openrouter")]
pub struct OpenRouterClient {
    api_key: String,
    model: String,
}

#[cfg(feature = "openrouter")]
impl OpenRouterClient {
    pub async fn chat_completion(
        &self,
        context: Context,
        message: String,
    ) -> Result<impl Stream<Item = Result<String>>> {
        // Similar implementation with OpenRouter API
    }
}
```

**Egui Integration**:
```rust
pub struct LLMService {
    ollama: Option<OllamaClient>,
    openrouter: Option<OpenRouterClient>,
    active_provider: LLMProvider,
}

impl LLMService {
    pub async fn chat(
        &self,
        provider: LLMProvider,
        context: Context,
        message: String,
    ) -> Result<impl Stream<Item = Result<String>>> {
        match provider {
            LLMProvider::Ollama => {
                if let Some(client) = &self.ollama {
                    client.chat_completion(context, message).await
                } else {
                    Err(LLMError::ProviderNotConfigured)
                }
            }
            #[cfg(feature = "openrouter")]
            LLMProvider::OpenRouter => {
                if let Some(client) = &self.openrouter {
                    client.chat_completion(context, message).await
                } else {
                    Err(LLMError::ProviderNotConfigured)
                }
            }
        }
    }
}
```

### 7. Persistence Layer

**Storage Interface**:
```rust
pub trait Persistable {
    async fn put(&self, key: &str, value: String) -> Result<()>;
    async fn get(&self, key: &str) -> Result<String>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
}

// Implementations: Sled, SQLite, JSON file, etc.
```

**Egui Integration**:
```rust
pub struct Persistence {
    storage: Arc<dyn Persistable>,
}

impl Persistence {
    pub async fn save_app_state(&self, state: &AppState) -> Result<()> {
        let data = serde_json::to_string_pretty(state)?;
        self.storage.put("app_state", data).await?;
        
        // Save separate sections
        self.storage.put("context_items", serde_json::to_string(&state.context_panel.items)?)?;
        self.storage.put("chat_history", serde_json::to_string(&state.chat_panel.messages)?)?;
        self.storage.put("ui_preferences", serde_json::to_string(&state.ui_prefs)?)?;
        
        Ok(())
    }
    
    pub async fn load_app_state(&self) -> Result<AppState> {
        let state_data = self.storage.get("app_state").await?;
        let mut state: AppState = serde_json::from_str(&state_data)?;
        
        // Load context items
        if let Ok(items_data) = self.storage.get("context_items").await {
            state.context_panel.items = serde_json::from_str(&items_data)?;
        }
        
        // Load chat history
        if let Ok(messages_data) = self.storage.get("chat_history").await {
            state.chat_panel.messages = serde_json::from_str(&messages_data)?;
        }
        
        Ok(state)
    }
}
```

---

## Performance Targets

### 1. Autocomplete Performance
**Target**: < 50ms response time for 10,000+ term thesaurus

**Optimization Strategies**:
- **FST (Finite State Transducer)**: O(k) lookup where k = query length
- **WASM Execution**: Native performance, no JS overhead
- **Result Caching**: LRU cache for frequent queries
- **Prefetching**: Load thesaurus in background
- **Early Termination**: Stop search at max results

**Benchmark**:
```rust
// bench_autocomplete.rs
fn benchmark_autocomplete() {
    let thesaurus = load_large_thesaurus(100_000);
    let index = AutocompleteIndex::new(thesaurus);
    
    // Test queries
    let queries = vec!["async", "rust", "tokio", "await", "future"];
    
    for query in queries {
        let start = Instant::now();
        let results = index.search(query, 5);
        let duration = start.elapsed();
        
        assert!(duration < Duration::from_millis(50));
        assert!(!results.is_empty());
    }
}
```

### 2. Knowledge Graph Rendering
**Target**: 60 FPS for smooth interaction (16ms per frame)

**Optimization Strategies**:
- **Culling**: Only render visible nodes
- **Level of Detail (LOD)**: Simplified rendering at low zoom
- **Batched Drawing**: Combine multiple draw calls
- **Async Layout**: Compute layout in background thread
- **Incremental Updates**: Only redraw changed elements

**Performance Monitoring**:
```rust
struct FPSCounter {
    last_time: Instant,
    frame_count: u32,
    current_fps: f32,
}

impl FPSCounter {
    fn update(&mut self) {
        self.frame_count += 1;
        let elapsed = self.last_time.elapsed();
        
        if elapsed >= Duration::from_secs(1) {
            self.current_fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_time = Instant::now();
        }
    }
    
    fn render(&self, ui: &mut egui::Ui) {
        ui.label(format!("FPS: {:.1}", self.current_fps));
    }
}
```

### 3. Search Performance
**Target**: < 200ms for search across haystacks

**Components**:
- **Index Lookup**: < 50ms
- **Scoring**: < 100ms
- **Result Formatting**: < 50ms

**Optimization**:
- **Precomputed Indexes**: BM25, FST
- **Parallel Search**: Search haystacks concurrently
- **Result Caching**: Cache frequent queries
- **Incremental Updates**: Update index as documents change

```rust
pub async fn search_with_timing(
    &self,
    query: &str,
    role: &RoleName,
) -> (Vec<SearchResult>, Duration) {
    let start = Instant::now();
    
    let mut service = self.service.lock().await;
    let results = service.search(query, role).await;
    
    let duration = start.elapsed();
    (results, duration)
}
```

### 4. Memory Usage
**Target**: < 500MB for typical usage

**Memory Distribution**:
- **Thesaurus**: ~100MB (100k terms)
- **Knowledge Graph**: ~200MB (10k nodes, 50k edges)
- **Documents**: ~100MB (1k documents)
- **UI State**: ~50MB
- **Overhead**: ~50MB

**Optimization**:
- **Lazy Loading**: Load data on demand
- **Pagination**: Don't load all results at once
- **Unload Unused**: Free memory when switching roles
- **Compact Representations**: Use efficient data structures

### 5. LLM Streaming
**Target**: < 100ms time-to-first-token

**Optimization**:
- **Connection Pooling**: Reuse HTTP connections
- **Fast Startup**: Pre-warm LLM if possible
- **Batched Tokens**: Group tokens for efficiency
- **Cancellation**: Stop stream when user cancels

```rust
pub async fn stream_with_timing(
    &self,
    message: String,
) -> impl Stream<Item = Result<String>> {
    let start = Instant::now();
    
    let mut stream = self.llm_service.chat(message).await;
    
    // Wait for first token
    if let Some(token_result) = stream.next().await {
        let time_to_first = start.elapsed();
        log::info!("Time to first token: {:?}", time_to_first);
        
        // Yield first token
        yield token_result;
        
        // Yield remaining tokens
        for token in stream {
            yield token;
        }
    }
}
```

### 6. Startup Time
**Target**: < 3 seconds cold start

**Phases**:
- **GUI Init**: 200ms (eframe startup)
- **Config Load**: 300ms (JSON parsing)
- **Service Init**: 500ms (TerraphimService)
- **Thesaurus Load**: 1000ms (WASM module)
- **First Role Load**: 1000ms (KG construction)

**Optimization**:
- **Background Loading**: Load non-critical data after UI is ready
- **Progress Indicators**: Show loading progress
- **Incremental Loading**: Load roles on demand
- **Precompute**: Pre-warm common operations

### 7. Responsive UI
**Target**: No UI freezes, < 16ms input-to-response

**Strategies**:
- **Async Operations**: Never block UI thread
- **Chunked Updates**: Process large operations in chunks
- **Request Repaint Only When Needed**: Avoid unnecessary redraws
- **Use tokio::spawn**: Run heavy tasks in background

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // Spawn async tasks, don't block
    if self.need_to_search {
        let service = self.service.clone();
        tokio::spawn(async move {
            // Heavy work here
            let _ = service.lock().await.search(query).await;
        });
        self.need_to_search = false;
    }
    
    // Only request repaint if something changed
    if self.dirty {
        ctx.request_repaint();
        self.dirty = false;
    }
}
```

---

## Implementation Phases

### Phase 1: Project Setup and Core Infrastructure (Week 1-2)
**Goals**: Establish egui project structure, basic UI, service integration

**Deliverables**:
1. New egui project (`terraphim-egui-ui`)
2. Basic window with panels
3. Integration with `terraphim_service`
4. Configuration loading
5. Role selection UI

**Tasks**:
- [ ] Create new crate: `terraphim-egui-ui`
- [ ] Add eframe/egui dependencies
- [ ] Set up basic app structure
- [ ] Create panel abstractions
- [ ] Integrate `terraphim_config`
- [ ] Implement role selector
- [ ] Basic window layout
- [ ] Compile and run minimal app

**Acceptance Criteria**:
- App launches successfully
- Can switch between roles
- Panels render correctly
- No crashes on startup

### Phase 2: Search and Autocomplete (Week 3-4)
**Goals**: Implement search interface with WASM autocomplete

**Deliverables**:
1. Search panel with input
2. WASM autocomplete integration
3. Search results display
4. Performance optimization

**Tasks**:
- [ ] Build terraphim_automata as WASM
- [ ] Create search input component
- [ ] Implement autocomplete dropdown
- [ ] Connect to `terraphim_service::search()`
- [ ] Display search results
- [ ] Add loading indicators
- [ ] Optimize for < 50ms autocomplete
- [ ] Add result selection

**Acceptance Criteria**:
- Autocomplete responds < 50ms
- Search returns results < 200ms
- UI remains responsive during search
- Can select and view results

### Phase 3: Context Builder (Week 5-6)
**Goals**: Implement context management and curation

**Deliverables**:
1. Context panel UI
2. Article selection and management
3. Concept addition
4. Context size tracking
5. Export functionality

**Tasks**:
- [ ] Create context panel layout
- [ ] Add articles to context
- [ ] Add individual concepts
- [ ] Display context size
- [ ] Implement remove items
- [ ] Export context to JSON/markdown
- [ ] Persist context across sessions
- [ ] Add drag-and-drop support

**Acceptance Criteria**:
- Can add/remove context items
- Size tracking accurate
- Can export context
- Context persists on restart

### Phase 4: Knowledge Graph Visualization (Week 7-8)
**Goals**: Interactive KG visualization

**Deliverables**:
1. Custom graph painter
2. Pan/zoom/select
3. Node/edge rendering
4. Interactive features

**Tasks**:
- [ ] Create custom painter for KG
- [ ] Implement force-directed layout
- [ ] Add pan (drag) and zoom (wheel)
- [ ] Render nodes and edges
- [ ] Implement node selection
- [ ] Add hover tooltips
- [ ] Double-click to expand
- [ ] Performance optimization

**Acceptance Criteria**:
- Smooth 60 FPS interaction
- Can pan and zoom
- Can select nodes
- Tooltips display correctly

### Phase 5: LLM Chat Interface (Week 9-10)
**Goals**: Chat with context injection

**Deliverables**:
1. Chat panel UI
2. Message display (with markdown)
3. Streaming response
4. Context inclusion
5. Copy-to-clipboard

**Tasks**:
- [ ] Create chat panel layout
- [ ] Render markdown messages
- [ ] Implement text input
- [ ] Connect to LLM service
- [ ] Stream responses
- [ ] Auto-include context
- [ ] Add copy button
- [ ] Persist chat history

**Acceptance Criteria**:
- Messages display with markdown
- Streaming works smoothly
- Context automatically included
- Can copy responses

### Phase 6: Role-Based Features (Week 11-12)
**Goals**: Complete role customization

**Deliverables**:
1. Theme system
2. Role-specific UI
3. LLM configuration
4. Haystack management

**Tasks**:
- [ ] Implement theme variants
- [ ] Role-specific layouts
- [ ] Configure LLM per role
- [ ] Haystack management UI
- [ ] KG settings per role
- [ ] System prompt editing
- [ ] Test all three roles

**Acceptance Criteria**:
- Themes apply correctly
- Role switching works
- LLM configured per role
- All features work in each role

### Phase 7: Settings and Configuration (Week 13-14)
**Goals**: User configuration management

**Deliverables**:
1. Settings panel
2. Role creation/editing
3. Haystack configuration
4. LLM provider setup

**Tasks**:
- [ ] Create settings window
- [ ] Role editor
- [ ] Haystack manager
- [ ] LLM provider config
- [ ] Theme customization
- [ ] Keyboard shortcuts
- [ ] Import/export config

**Acceptance Criteria**:
- Can create/edit roles
- Can configure haystacks
- Can set up LLM providers
- Settings persist

### Phase 8: Polish and Optimization (Week 15-16)
**Goals**: Performance, UX, stability

**Deliverables**:
1. Performance optimization
2. Error handling
3. User feedback
4. Documentation

**Tasks**:
- [ ] Profile and optimize hot paths
- [ ] Add error handling everywhere
- [ ] Add progress indicators
- [ ] Add tooltips and help text
- [ ] Keyboard shortcuts
- [ ] Accessibility features
- [ ] User guide/documentation
- [ ] Bug fixes

**Acceptance Criteria**:
- All performance targets met
- No crashes or panics
- Smooth user experience
- Complete documentation

### Phase 9: Testing and QA (Week 17-18)
**Goals**: Thorough testing

**Deliverables**:
1. Unit tests
2. Integration tests
3. E2E tests
4. Performance tests

**Tasks**:
- [ ] Unit test all components
- [ ] Integration tests
- [ ] E2E test user journeys
- [ ] Performance benchmarks
- [ ] Memory leak tests
- [ ] Cross-platform testing
- [ ] User acceptance testing
- [ ] Final bug fixes

**Acceptance Criteria**:
- 80%+ test coverage
- All tests pass
- Performance targets met
- User acceptance confirmed

### Phase 10: Release Preparation (Week 19-20)
**Goals**: Production release

**Deliverables**:
1. Binary builds
2. Installer packages
3. Release notes
4. Migration guide

**Tasks**:
- [ ] Release builds (Linux, macOS, Windows)
- [ ] Create installers
- [ ] Sign binaries
- [ ] Write release notes
- [ ] Migration guide from old UI
- [ ] Website updates
- [ ] Announcement
- [ ] Support channels

**Acceptance Criteria**:
- Binaries work on all platforms
- Installers install correctly
- Migration path documented
- Release announced

---

## Migration Strategy

### From Current TUI to Egui

**Current TUI Architecture** (ratatui):
```
┌──────────────────────────┐
│ Terminal (ratatui +      │
│       crossterm)         │
│                          │
│ ┌──────────────────────┐ │
│ │ REPL Commands        │ │
│ │ /search, /chat, etc  │ │
│ └──────────────────────┘ │
│                          │
│ ┌──────────────────────┐ │
│ │ ASCII Graph          │ │
│ │ Visualization        │ │
│ └──────────────────────┘ │
└──────────────────────────┘
```

**Target Egui Architecture**:
```
┌────────────────────────────────────┐
│ Native Window (eframe + egui)      │
│                                    │
│ ┌──────────┐ ┌────────────────────┐│
│ │ Search   │ │ Knowledge Graph    ││
│ │ Panel    │ │ Visualization      ││
│ └──────────┘ └────────────────────┘│
│                                    │
│ ┌──────────┐ ┌────────────────────┐│
│ │ Context  │ │ LLM Chat           ││
│ │ Builder  │ │ Interface          ││
│ └──────────┘ └────────────────────┘│
└────────────────────────────────────┘
```

**Migration Steps**:

1. **Preserve Backend Services**
   - All existing crates (service, automata, rolegraph, config) remain unchanged
   - Only replace the UI layer
   - Maintain all APIs and data structures

2. **Shared Code Extraction**
   ```rust
   // Extract common logic
   pub mod shared {
       pub mod search;      // Search logic (used by TUI and egui)
       pub mod context;     // Context management
       pub mod kg_utils;    // KG utilities
   }
   ```

3. **Gradual Replacement**
   - Keep TUI running during development
   - Implement egui features side-by-side
   - Test both interfaces with same backend
   - Deprecate TUI after egui is complete

4. **Configuration Compatibility**
   - Same JSON config files
   - Same role definitions
   - Same haystack configurations
   - No config migration needed

5. **Data Migration**
   - Same persistence layer
   - Same document storage
   - Same chat history format
   - No data conversion required

### From Svelte/Tauri to Egui

**Current Web Architecture**:
```
┌──────────────────────────┐
│ Browser (Chromium)       │
│                          │
│ ┌──────────────────────┐ │
│ │ Svelte Frontend      │ │
│ │ (JavaScript)         │ │
│ └──────────────────────┘ │
│            │             │
│ ┌──────────────────────┐ │
│ │ Tauri Backend        │ │
│ │ (Rust + WebView)     │ │
│ └──────────────────────┘ │
└──────────────────────────┘
```

**Target Egui Architecture**:
```
┌──────────────────────────┐
│ Native Desktop App       │
│ (Direct Rust)            │
│                          │
│ ┌──────────────────────┐ │
│ │ Egui UI              │ │
│ │ (Rust + WASM)        │ │
│ └──────────────────────┘ │
│                          │
│ ┌──────────────────────┐ │
│ │ Backend Services     │ │
│ │ (Direct Integration) │ │
│ └──────────────────────┘ │
└──────────────────────────┘
```

**Migration Benefits**:
- **No Browser Overhead**: Direct rendering
- **Single Binary**: No webview dependency
- **Better Performance**: No JavaScript bridge
- **Simpler Distribution**: One executable
- **Rust Native**: Everything in one language

**Migration Approach**:
1. Extract UI logic from Svelte components
2. Rewrite in Rust using egui
3. Keep backend services identical
4. Test feature parity
5. Remove old frontend

### Compatibility Layer

```rust
// Maintains compatibility during migration
#[cfg(feature = "egui")]
pub mod egui_ui {
    pub struct EguiApp {
        service: TerraphimService,
        // ... egui-specific state
    }
}

#[cfg(feature = "tui")]
pub mod tui_ui {
    pub struct TuiApp {
        service: TerraphimService,
        // ... tui-specific state
    }
}

pub enum UIMode {
    Egui,
    Tui,
}

pub fn start_app(mode: UIMode, service: TerraphimService) {
    match mode {
        UIMode::Egui => start_egui_app(service),
        UIMode::Tui => start_tui_app(service),
    }
}
```

---

## Testing Strategy

### Unit Tests

**Test Autocomplete**:
```rust
#[cfg(test)]
mod autocomplete_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_autocomplete_performance() {
        let thesaurus = build_test_thesaurus(10_000);
        let index = AutocompleteIndex::new(thesaurus);
        
        let start = Instant::now();
        let results = index.search("async", 5);
        let duration = start.elapsed();
        
        assert!(duration < Duration::from_millis(50));
        assert_eq!(results.len(), 5);
    }
    
    #[test]
    fn test_fuzzy_matching() {
        let thesaurus = build_test_thesaurus(100);
        let index = AutocompleteIndex::new(thesaurus);
        
        // Test typo tolerance
        let results = index.search("astnc", 5); // typo of "async"
        assert!(!results.is_empty());
    }
}
```

**Test Search Integration**:
```rust
#[tokio::test]
async fn test_search_with_role() {
    let mut service = TerraphimService::new(config_state).await;
    let role = RoleName::new("Default");
    
    let results = service
        .search(&SearchQuery::new("rust".to_string()), &role)
        .await
        .unwrap();
    
    assert!(!results.is_empty());
    assert!(results[0].score >= results[1].score);
}
```

**Test Context Builder**:
```rust
#[test]
fn test_context_item_add_remove() {
    let mut context = ContextBuilder::new();
    
    // Add article
    let article = ContextItem::Article {
        id: "test".to_string(),
        title: "Test".to_string(),
        excerpt: "Test excerpt".to_string(),
        url: None,
    };
    
    context.add_item(article.clone());
    assert_eq!(context.items.len(), 1);
    
    // Remove article
    context.remove_item(&article);
    assert_eq!(context.items.len(), 0);
}
```

### Integration Tests

**Test Complete User Journey**:
```rust
#[tokio::test]
async fn test_search_to_llm_journey() -> Result<()> {
    // 1. Start app
    let mut app = EguiApp::new().await?;
    
    // 2. Search for articles
    app.search_panel.set_query("async rust".to_string());
    app.search_panel.execute_search().await?;
    assert!(!app.search_panel.results.is_empty());
    
    // 3. Select result
    app.search_panel.select_result(0);
    
    // 4. Add to context
    app.context_panel.add_selected_article();
    assert_eq!(app.context_panel.items.len(), 1);
    
    // 5. Send to LLM
    app.chat_panel.set_message("Explain async/await".to_string());
    app.chat_panel.send().await?;
    
    // 6. Verify response received
    let response = app.chat_panel.wait_for_response().await?;
    assert!(!response.is_empty());
    
    Ok(())
}
```

**Test Role Switching**:
```rust
#[tokio::test]
async fn test_role_switching() -> Result<()> {
    let mut app = EguiApp::new().await?;
    
    // Start with Default role
    assert_eq!(app.current_role.as_str(), "Default");
    
    // Switch to Rust Engineer
    app.role_manager.switch_to("Rust Engineer".into()).await?;
    assert_eq!(app.current_role.as_str(), "Rust Engineer");
    
    // Verify KG loaded
    assert!(app.kg_panel.rolegraph.is_some());
    
    // Switch back
    app.role_manager.switch_to("Default".into()).await?;
    assert_eq!(app.current_role.as_str(), "Default");
    
    Ok(())
}
```

**Test Knowledge Graph Interaction**:
```rust
#[test]
fn test_kg_pan_zoom() {
    let mut kg = KnowledgeGraphPanel::new();
    
    // Test pan
    let initial_center = kg.get_center();
    kg.pan(egui::vec2(10.0, 20.0));
    let new_center = kg.get_center();
    assert_ne!(initial_center, new_center);
    
    // Test zoom
    let initial_zoom = kg.zoom;
    kg.zoom_at(egui::pos2(0.0, 0.0), 1.5);
    assert!(kg.zoom > initial_zoom);
}
```

### End-to-End Tests

**Full Workflow Test**:
```rust
#[tokio::test]
async fn end_to_end_workflow() -> Result<()> {
    // Setup
    let (_temp_dir, app) = setup_test_app().await?;
    
    // 1. Launch app and verify UI
    app.assert_panels_visible(["search", "context", "kg", "chat"]);
    
    // 2. Search workflow
    app.search("rust async programming")
        .await
        .assert_has_results();
    
    app.search_results
        .select(0)
        .add_to_context();
    
    // 3. KG workflow
    app.kg_panel
        .select_nodes([1, 2, 3])
        .add_to_context();
    
    // 4. Chat workflow
    app.chat_panel
        .type_message("What is async in Rust?")
        .send()
        .await
        .assert_streaming_response()
        .assert_response_received();
    
    // 5. Verify context
    app.context_panel
        .assert_item_count(4) // 1 article + 3 KG nodes
        .assert_character_count_lt(32000);
    
    // 6. Role switch
    app.role_manager.switch_to("Rust Engineer");
    app.assert_theme_changed("dark");
    
    // 7. Persist and reload
    app.save_state().await?;
    app.reload_state().await?;
    app.context_panel.assert_item_count(4);
    
    Ok(())
}
```

### Performance Tests

**Autocomplete Benchmarks**:
```rust
fn criterion_benchmark_autocomplete(c: &mut Criterion) {
    let thesaurus = load_test_thesaurus(100_000);
    let index = AutocompleteIndex::new(thesaurus);
    
    c.bench_function("autocomplete_100k", |b| {
        b.iter(|| {
            let results = index.search(black_box("async"), 5);
            assert!(!results.is_empty());
        });
    });
    
    c.bench_function("autocomplete_with_typo", |b| {
        b.iter(|| {
            let results = index.search(black_box("astnc"), 5);
            assert!(!results.is_empty());
        });
    });
}
```

**Search Performance**:
```rust
#[tokio::test]
async fn test_search_performance() {
    let mut service = TerraphimService::new().await;
    let role = RoleName::new("Default");
    
    // Measure search time
    let start = Instant::now();
    let results = service
        .search(&SearchQuery::new("rust tokio async".to_string()), &role)
        .await
        .unwrap();
    let duration = start.elapsed();
    
    assert!(duration < Duration::from_millis(200));
    println!("Search took: {:?}", duration);
}
```

**Memory Usage Tests**:
```rust
#[test]
fn test_memory_usage() {
    let initial_memory = get_memory_usage();
    
    // Load large KG
    let mut app = EguiApp::new();
    app.load_role("Terraphim Engineer").await;
    
    let peak_memory = get_peak_memory_usage();
    let memory_increase = peak_memory - initial_memory;
    
    // Should be under 500MB
    assert!(memory_increase < 500 * 1024 * 1024);
    println!("Memory increase: {} MB", memory_increase / 1024 / 1024);
}
```

### UI Tests

**Snapshots Testing**:
```rust
#[cfg(test)]
mod ui_tests {
    use super::*;
    use insta::assert_snapshot;
    
    #[test]
    fn test_search_panel_layout() {
        let app = EguiApp::new();
        let ui = app.render_search_panel();
        
        assert_snapshot!(ui, "search_panel_default");
    }
    
    #[test]
    fn test_kg_panel_dark_theme() {
        let app = EguiApp::new();
        app.set_theme(ThemeVariant::Dark);
        let ui = app.render_kg_panel();
        
        assert_snapshot!(ui, "kg_panel_dark");
    }
}
```

**Interaction Tests**:
```rust
#[tokio::test]
async fn test_keyboard_shortcuts() {
    let mut app = EguiApp::new().await;
    
    // Test Ctrl+K focus search
    app.send_keyboard_input(ctrl_key('k'));
    assert!(app.search_panel.is_focused());
    
    // Test arrow navigation in autocomplete
    app.search_panel.set_query("as");
    app.send_keyboard_input(Key::ArrowDown);
    assert_eq!(app.search_panel.selected_index, 0);
}
```

### Test Organization

```
tests/
├── unit/
│   ├── autocomplete/
│   │   ├── test_performance.rs
│   │   ├── test_fuzzy_match.rs
│   │   └── test_cache.rs
│   ├── search/
│   │   ├── test_bm25.rs
│   │   ├── test_title_scorer.rs
│   │   └── test_terraphim_graph.rs
│   ├── context/
│   │   ├── test_builder.rs
│   │   └── test_export.rs
│   └── kg/
│       ├── test_layout.rs
│       ├── test_interaction.rs
│       └── test_visualization.rs
├── integration/
│   ├── test_search_workflow.rs
│   ├── test_role_switching.rs
│   └── test_llm_integration.rs
├── e2e/
│   ├── test_complete_journey.rs
│   └── test_performance.rs
├── ui/
│   ├── test_panels.rs
│   ├── test_themes.rs
│   └── test_shortcuts.rs
└── benchmarks/
    ├── autocomplete_bench.rs
    ├── search_bench.rs
    └── kg_bench.rs
```

### Continuous Testing

**GitHub Actions Workflow**:
```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Run unit tests
        run: cargo test --lib
        
      - name: Run integration tests
        run: cargo test --test '*'
        
      - name: Run e2e tests
        run: cargo test --test e2e -- --test-threads=1
        
      - name: Run benchmarks
        run: cargo bench
        
      - name: Check coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out xml
```

### Test Data

**Fixtures**:
```
tests/fixtures/
├── thesauri/
│   ├── small_thesaurus.json (100 terms)
│   ├── medium_thesaurus.json (10k terms)
│   └── large_thesaurus.json (100k terms)
├── knowledge_graphs/
│   ├── simple_kg.json
│   ├── complex_kg.json
│   └── code_kg.json
├── documents/
│   ├── rust_docs/
│   ├── markdown_files/
│   └── code_samples/
└── configs/
    ├── default_config.json
    ├── rust_engineer_config.json
    └── terraphim_engineer_config.json
```

---

## Risk Assessment

### High Risk

#### 1. Knowledge Graph Performance
**Risk**: Large knowledge graphs (>10k nodes) causing frame drops
**Impact**: Unusable UI, poor user experience
**Probability**: High (common scenario)
**Mitigation**:
- Implement level-of-detail (LOD) rendering
- Use view frustum culling
- Async layout computation
- Virtualization for large graphs
- Batch rendering optimizations
- Fallback to simplified view for very large graphs

**Contingency Plan**:
- Add "Simplified View" toggle
- Implement progressive loading
- Add warning for graphs >50k nodes
- Offer to switch to table view

#### 2. WASM Integration Complexity
**Risk**: Autocomplete WASM module fails to load or has bugs
**Impact**: Autocomplete non-functional, degraded UX
**Probability**: Medium
**Mitigation**:
- Fallback to pure Rust implementation
- Comprehensive WASM testing
- Multiple build targets
- Clear error messages
- Version checking

**Contingency Plan**:
- Disable WASM, use Rust fallback
- Switch to WebView + JavaScript implementation
- Notify user of degraded performance

#### 3. LLM Provider Reliability
**Risk**: LLM service unavailable (Ollama down, network issues)
**Impact**: Chat feature non-functional
**Probability**: Medium
**Mitigation**:
- Connection retry logic
- Fallback providers
- Clear error messages
- Offline mode with cached responses
- Provider auto-detection

**Contingency Plan**:
- Disable chat panel
- Use mock responses for demo
- Show helpful error message with setup instructions

### Medium Risk

#### 4. Async/Tokio Complexity
**Risk**: Complex async operations causing deadlocks or race conditions
**Impact**: UI freezes, crashes, data corruption
**Probability**: Medium
**Mitigation**:
- Careful lock ordering
- Use async-aware tools (tokio::sync::RwLock)
- Comprehensive async testing
- Avoid blocking operations in UI
- Use channels for communication

**Contingency Plan**:
- Add detailed logging
- Implement panic recovery
- Add debug mode to track locks

#### 5. Memory Usage
**Risk**: High memory usage with large datasets
**Impact**: System slowdown, OOM crashes
**Probability**: Medium
**Mitigation**:
- Memory profiling from start
- Lazy loading
- Data compression
- Garbage collection hints
- Memory monitoring
- Limits on data size

**Contingency Plan**:
- Add memory limit warnings
- Implement memory cleanup triggers
- Allow user to limit cache size

#### 6. Cross-Platform Compatibility
**Risk**: UI rendering issues on different platforms
**Impact**: Inconsistent experience, crashes
**Probability**: Medium
**Mitigation**:
- Test on all target platforms (Linux, macOS, Windows)
- Use platform-agnostic egui features
- CI/CD with multiple OS builds
- Platform-specific testing
- Font compatibility checks

**Contingency Plan**:
- Platform-specific patches
- Disable problematic features on certain platforms
- Virtual machine-based testing

### Low Risk

#### 7. Configuration Migration
**Risk**: User configuration not compatible
**Impact**: Users lose settings
**Probability**: Low (good config compatibility)
**Mitigation**:
- Same JSON format
- Validation with fallbacks
- Backup creation
- Migration scripts if needed
- Version checking

**Contingency Plan**:
- Default to safe configuration
- Alert users to check settings
- Easy configuration reset

#### 8. Data Persistence
**Risk**: Data loss or corruption
**Impact**: Lost context, chat history
**Probability**: Low (simple storage)
**Mitigation**:
- Incremental saves
- Backup before save
- Atomic operations
- Checksums for integrity
- Error handling

**Contingency Plan**:
- Recovery from backup
- Partial data recovery
- Clear error messages

#### 9. Performance Regression
**Risk**: Slower than expected after optimization
**Impact**: User dissatisfaction
**Probability**: Low (good initial estimates)
**Mitigation**:
- Continuous profiling
- Performance budgets
- Benchmark tracking
- Early performance testing
- Optimization checkpoints

**Contingency Plan**:
- More aggressive optimization
- Feature reduction if needed
- Clear communication about performance

### Risk Monitoring

**Weekly Risk Review**:
- Check performance metrics
- Review crash reports
- Assess user feedback
- Test on all platforms
- Monitor memory usage

**Alert Thresholds**:
- Autocomplete > 50ms
- Frame rate < 30 FPS
- Memory > 1GB
- Crash rate > 1%
- Search > 500ms

**Mitigation Checklist**:
- [ ] Performance monitoring in place
- [ ] Error tracking enabled
- [ ] Fallback mechanisms tested
- [ ] Cross-platform testing scheduled
- [ ] User feedback channels active
- [ ] Documentation updated

### Success Criteria

**Technical Success**:
- [ ] All performance targets met
- [ ] < 1% crash rate
- [ ] Cross-platform compatibility
- [ ] Memory usage < 500MB
- [ ] 80%+ test coverage

**User Success**:
- [ ] < 3 second startup
- [ ] Smooth 60 FPS interaction
- [ ] Intuitive navigation
- [ ] Feature parity with current UI
- [ ] Positive user feedback

**Business Success**:
- [ ] Users can migrate from old UI
- [ ] Reduced support issues
- [ ] Improved user satisfaction
- [ ] Adoption of new features
- [ ] Positive reviews

---

## Conclusion

This specification provides a comprehensive blueprint for migrating Terraphim AI to an egui-based desktop application. The migration will deliver:

1. **Unified Architecture**: Single, native Rust application
2. **Superior Performance**: WASM autocomplete, native rendering
3. **Enhanced UX**: Intuitive, responsive interface
4. **Role-Based Customization**: Tailored experience per role
5. **Complete Feature Parity**: All current features plus new capabilities

The phased implementation approach ensures steady progress with clear milestones. Comprehensive testing and risk mitigation strategies minimize disruption and ensure a successful migration.

**Next Steps**:
1. Review and approve specification
2. Set up development environment
3. Begin Phase 1 implementation
4. Establish CI/CD pipeline
5. Regular progress reviews

**Estimated Timeline**: 20 weeks (5 months)
**Team Size**: 2-3 developers
**Total Effort**: ~3200 developer hours

This migration will position Terraphim AI as a best-in-class, privacy-first AI assistant with a modern, native desktop interface.
