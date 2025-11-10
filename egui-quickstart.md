# Terraphim AI egui - Quickstart Guide

Welcome to the Terraphim AI egui implementation! This guide will help you get started as both a **user** and a **developer**.

## Table of Contents
- [For Users](#for-users)
  - [What is egui?](#what-is-egui)
  - [Getting Started](#getting-started-as-a-user)
  - [Key Features](#key-features)
  - [User Workflows](#user-workflows)
- [For Developers](#for-developers)
  - [Development Setup](#development-setup)
  - [Architecture Overview](#architecture-overview)
  - [Adding New Features](#adding-new-features)
  - [Testing](#testing)
  - [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)

---

## For Users

### What is egui?

egui is a fast, simple, and beautiful immediate mode GUI library written in Rust. The Terraphim AI egui implementation provides a native desktop experience with:
- **Instant response** - UI updates immediately as you interact
- **Low resource usage** - Minimal memory and CPU footprint
- **Cross-platform** - Runs on Windows, macOS, and Linux
- **No web browser** - Native desktop application

### Getting Started as a User

#### Prerequisites
- Rust 1.75+ installed
- Ollama (for LLM features)
  ```bash
  # Install Ollama
  curl -fsSL https://ollama.ai/install.sh | sh

  # Start Ollama
  ollama serve

  # Pull a model (optional, for LLM features)
  ollama pull llama3.2:3b
  ```

#### Building and Running

**Option 1: Using Cargo**

```bash
# Build the egui application
cargo build --package terraphim_egui --release

# Run the application
cargo run --package terraphim_egui --release
```

**Option 2: Using the Desktop Package**

```bash
# Navigate to the desktop directory
cd desktop

# Install dependencies
yarn install

# Run the egui variant
yarn run egui:dev
```

#### First Launch

When you first launch the application, you'll see:
1. **Search Panel** (left) - Main search interface
2. **Chat Panel** (center) - Conversation area
3. **Context Panel** (right) - Context management
4. **Knowledge Graph** (bottom) - Visual relationship display
5. **Role Selector** (top) - Switch between user roles

### Key Features

#### 🔍 Smart Search
- **Real-time autocomplete** as you type
- **Fuzzy matching** for typo tolerance
- **Context-aware results** based on your role
- **Add to context** with one click

#### 💬 Chat with LLM
- **Local LLM support** via Ollama
- **Context-aware conversations** using your search results
- **Markdown rendering** for rich text
- **Conversation history** with persistence

#### 📚 Context Management
- **Add search results** to conversation context
- **Edit context** before sending to LLM
- **Clear context** to start fresh
- **Context preview** with snippets

#### 🎨 Role-Based Interface
- **Multiple roles** (Engineer, Researcher, Writer, etc.)
- **Role-specific search** results
- **Customized UI** per role
- **Quick switching** from the top menu

#### 🕸️ Knowledge Graph
- **Visual relationships** between concepts
- **Interactive exploration** of knowledge
- **Filtered view** by your current role
- **Search integration** with graph nodes

### User Workflows

#### Workflow 1: Search and Explore
1. Open the application
2. Type in the search box (autocomplete will help)
3. Review the search results
4. Click results to add them to context
5. Optionally view the knowledge graph to explore relationships

#### Workflow 2: Research with LLM
1. Search for topics of interest
2. Add relevant results to context (click the + button)
3. Navigate to the Chat panel
4. Ask questions about your context
5. Review the markdown-formatted response
6. Add more context as needed and continue the conversation

#### Workflow 3: Role Switching
1. Click the role selector at the top
2. Choose from available roles
3. Search results and UI adapt to the new role
4. Knowledge graph updates with role-specific data
5. Switch back anytime

#### Workflow 4: Managing Context
1. Add search results using the + button
2. Review context panel (right side)
3. Edit context items if needed
4. Send context to chat
5. Clear context when done with a topic

---

## For Developers

### Development Setup

#### Prerequisites
- Rust 1.75+ with `cargo`
- Git
- Ollama (for testing LLM features)
- A code editor (VS Code with rust-analyzer recommended)

#### Clone and Setup

```bash
# Clone the repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai/terraphim-ai-egui

# Build the workspace
cargo build --workspace

# Run tests
cargo test --package terraphim_egui
```

#### Development Commands

```bash
# Run in development mode (with hot reload for UI)
cargo run --package terraphim_egui --dev

# Run specific test
cargo test --package terraphim_egui --test test_search_functionality

# Run all tests
cargo test --package terraphim_egui

# Check code formatting
cargo fmt --package terraphim_egui

# Run linter
cargo clippy --package terraphim_egui
```

### Architecture Overview

#### Directory Structure

```
crates/terraphim_egui/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── main.rs             # Binary entry point
│   ├── app.rs              # Main application struct
│   ├── state.rs            # Application state management
│   ├── ui/                 # UI components
│   │   ├── mod.rs
│   │   ├── panels.rs       # Panel management
│   │   ├── search/         # Search UI
│   │   │   ├── mod.rs
│   │   │   ├── input.rs    # Search input field
│   │   │   ├── results.rs  # Search results display
│   │   │   ├── autocomplete.rs
│   │   │   └── service.rs  # Search service integration
│   │   ├── chat/           # Chat UI
│   │   │   ├── mod.rs
│   │   │   ├── widget.rs   # Chat widget
│   │   │   └── history.rs  # Chat history
│   │   ├── context/        # Context management
│   │   │   ├── mod.rs
│   │   │   ├── panel.rs    # Context panel
│   │   │   └── manager.rs  # Context manager
│   │   ├── kg/             # Knowledge graph
│   │   │   ├── mod.rs
│   │   │   ├── viewer.rs   # Graph viewer
│   │   │   └── painter.rs  # Graph rendering
│   │   ├── config/         # Configuration UI
│   │   │   ├── mod.rs
│   │   │   ├── role_selector.rs
│   │   │   └── settings.rs
│   │   ├── sessions/       # Session management
│   │   │   ├── mod.rs
│   │   │   ├── panel.rs
│   │   │   └── history.rs
│   │   └── theme.rs        # Theme management
│   └── tests.rs            # Unit tests
└── tests/                  # Integration tests
    ├── mod.rs
    ├── test_*.rs           # Test files
    ├── README.md
    └── TEST_RESULTS.md
```

#### Core Components

**1. Application State (`state.rs`)**
```rust
pub struct AppState {
    pub search_service: Arc<SearchService>,
    pub autocomplete_service: Arc<AutocompleteService>,
    pub llm_client: Arc<Mutex<LLMClient>>,
    pub context_manager: ContextManager,
    pub current_role: String,
    pub conversation_history: Vec<Conversation>,
}
```

**2. Panel System (`ui/panels.rs`)**
```rust
pub enum Panel {
    Search,
    Chat,
    Context,
    KnowledgeGraph,
    Config,
}
```

**3. Services**
- `SearchService` - Handles search operations
- `AutocompleteService` - Provides autocomplete suggestions
- `LLMClient` - Manages LLM communication
- `ContextManager` - Handles context persistence

### Adding New Features

#### Adding a New UI Panel

1. Create the panel module in `src/ui/`:

```rust
// src/ui/my_feature/panel.rs
use egui::CentralPanel;

pub struct MyFeaturePanel {
    // Panel state
    data: Vec<String>,
}

impl MyFeaturePanel {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        CentralPanel::default().show(ui, |ui| {
            ui.heading("My Feature");

            for item in &self.data {
                ui.label(item);
            }

            if ui.button("Add Item").clicked() {
                self.data.push("New Item".to_string());
            }
        });
    }
}
```

2. Register in `ui/mod.rs`:

```rust
mod my_feature;

pub use my_feature::panel::MyFeaturePanel;
```

3. Add to main app:

```rust
// src/app.rs
pub struct TerraphimApp {
    // ... existing fields
    my_feature_panel: MyFeaturePanel,
}

impl TerraphimApp {
    pub fn new() -> Self {
        Self {
            // ... initialize other panels
            my_feature_panel: MyFeaturePanel::new(),
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // ... existing panels

        my_feature_panel.show(ctx);
    }
}
```

#### Adding a New Service

1. Create the service in the appropriate module:

```rust
// src/ui/my_feature/service.rs
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct MyService {
    // Service state
}

impl MyService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn do_something(&self) -> Result<String, String> {
        // Implementation
        Ok("Result".to_string())
    }
}
```

2. Add to application state:

```rust
// src/state.rs
pub struct AppState {
    // ... existing services
    pub my_service: Arc<Mutex<MyService>>,
}
```

3. Initialize in main:

```rust
// src/app.rs
impl TerraphimApp {
    pub fn new() -> Self {
        let state = AppState {
            // ... initialize other services
            my_service: Arc::new(Mutex::new(MyService::new())),
        };

        Self { state }
    }
}
```

### Testing

#### Running Tests

```bash
# Run all tests
cargo test --package terraphim_egui

# Run specific test file
cargo test --package terraphim_egui --test test_search_functionality

# Run tests with output
cargo test --package terraphim_egui --test test_llm_integration -- --nocapture

# Run ignored tests
cargo test --package terraphim_egui -- --ignored
```

#### Writing Tests

**Unit Test Example:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_functionality() {
        let service = SearchService::new();
        let results = service.search("test query", None).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert!(!results.is_empty());
    }
}
```

**Integration Test Example:**

```rust
#[tokio::test]
async fn test_search_to_chat_workflow() {
    let mut state = AppState::new();

    // Simulate user input
    let query = "rust programming".to_string();
    state.search_service.search(&query, None).await.unwrap();

    // Add to context
    let results = state.search_service.get_results();
    assert!(!results.is_empty());

    // Send to LLM
    let response = state.llm_client.lock().await
        .generate_response("What is Rust?", Some(results.clone()))
        .await
        .unwrap();

    assert!(!response.is_empty());
}
```

**Test with Real LLM:**

```rust
#[tokio::test]
async fn test_real_ollama_integration() {
    let client = LLMClient::new("http://localhost:11434".to_string());
    let response = client.generate_response(
        "What is 2+2?",
        None
    ).await;

    assert!(response.is_ok());
    let response_text = response.unwrap();
    assert!(response_text.contains("4"));
}
```

See `tests/README.md` for more test examples.

### Common Patterns

#### Async Operations in UI

```rust
// Use tokio::spawn to run async tasks
let response = ui.ctx().request_repaint_after(Duration::from_millis(100));
tokio::spawn(async move {
    // Long-running operation
    perform_async_work().await;
});
```

#### State Management

```rust
// Use Arc<Mutex<T>> for shared state
pub struct MyComponent {
    state: Arc<Mutex<SharedState>>,
}

// Clone for async contexts
let state = Arc::clone(&self.state);
tokio::spawn(async move {
    let mut state = state.lock().await;
    state.update();
});
```

#### Error Handling

```rust
// Return Result from async operations
async fn my_async_function() -> Result<String, String> {
    match perform_operation().await {
        Ok(data) => Ok(data),
        Err(e) => {
            log::error!("Operation failed: {}", e);
            Err(e.to_string())
        }
    }
}
```

#### UI Patterns

**Modal Dialogs:**

```rust
if self.show_modal {
    egui::Window::new("Modal")
        .open(&mut self.show_modal)
        .show(ui.ctx(), |ui| {
            ui.label("Modal content");

            if ui.button("Close").clicked() {
                self.show_modal = false;
            }
        });
}
```

**Responsive Layouts:**

```rust
// Use egui's layout system
egui::SidePanel::left("left_panel")
    .default_width(200.0)
    .show(ui.ctx(), |ui| {
        // Left panel content
    });

CentralPanel::default().show(ui, |ui| {
    // Main content
});
```

**State Persistence:**

```rust
fn save(&self) -> Result<(), String> {
    let data = serde_json::to_string(&self.state)
        .map_err(|e| e.to_string())?;

    std::fs::write("state.json", data)
        .map_err(|e| e.to_string())
}
```

---

## Troubleshooting

### Common Issues

#### Issue: "Failed to connect to Ollama"
**Solution:**
```bash
# Start Ollama
ollama serve

# Check if it's running
curl http://localhost:11434
```

#### Issue: Tests failing with "Address already in use"
**Solution:**
```bash
# Kill existing processes using the port
lsof -ti:11434 | xargs kill -9

# Or use a different port in tests
OLLAMA_BASE_URL=http://127.0.0.1:11435 cargo test
```

#### Issue: Build errors related to missing dependencies
**Solution:**
```bash
# Update Rust
rustup update

# Clean build
cargo clean
cargo build --package terraphim_egui
```

#### Issue: UI not responding
**Solution:**
- Check that async operations aren't blocking the UI thread
- Use `tokio::spawn` for long-running operations
- Ensure proper error handling to prevent panics

#### Issue: Search results not appearing
**Solution:**
1. Check that `terraphim-service` is running
2. Verify network connection to search backend
3. Check logs for search service errors
4. Ensure role configuration is valid

### Getting Help

1. **Check logs** - The application logs to stderr
2. **Run with debug**:
   ```bash
   RUST_LOG=debug cargo run --package terraphim_egui
   ```
3. **Review test output** - Check `tests/TEST_RESULTS.md`
4. **Read documentation** - See `tests/README.md` for detailed guides
5. **Check existing tests** - Look at `tests/*.rs` for patterns

### Performance Tips

1. **Minimize allocations** in the UI hot path
2. **Use `request_repaint_after`** for time-based updates
3. **Cache expensive operations** in services
4. **Use `Arc<Mutex<T>>`** efficiently to avoid contention
5. **Profile with `cargo bench`** for performance-critical code

---

## Additional Resources

- **egui documentation**: https://docs.rs/egui
- **eframe framework**: https://docs.rs/eframe
- **Rust async book**: https://rust-lang.github.io/async-book
- **Test results**: `tests/TEST_RESULTS.md`
- **Test guide**: `tests/README.md`
- **Architecture docs**: See project README and inline documentation

---

**Happy coding and enjoy building with egui!** 🚀
