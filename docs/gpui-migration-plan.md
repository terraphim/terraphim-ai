# GPUI Migration Plan - Core User Journey

**Version:** 1.0
**Date:** 2025-11-24
**Status:** Planning
**Scope:** Search, Autocomplete, Markdown Editor, Chat (excludes graph visualization)

---

## Executive Summary

Focused migration plan for Terraphim Desktop's **core user journey** from Tauri/Svelte to GPUI:

1. **Search with Autocomplete** - KG-powered search with real-time suggestions
2. **Markdown Editor** - Novel-style editor with slash commands and MCP integration
3. **Chat with Context** - AI chat with persistent context and session history

**Timeline:** 8-10 weeks
**Risk:** Medium (simplified scope, no complex graph rendering)

---

## 1. Scope Definition

### ✅ In Scope

**Search & Autocomplete**
- Text input with real-time autocomplete
- KG-powered term suggestions from thesaurus
- Search results list with ranking
- Result item display (title, description, URL)
- Term chips for multi-term queries (AND/OR operators)
- Result modals for detail view

**Markdown Editor**
- Rich text editing with markdown support
- Slash commands for MCP tools
- Autocomplete for KG terms in editor
- Code syntax highlighting
- Line numbers and LSP integration

**Chat Interface**
- Message list (user/assistant bubbles)
- Chat input with markdown preview
- Context panel with KG term definitions
- Session history and persistence
- Context management (add/edit/delete)

**Core Infrastructure**
- Theme system (light/dark)
- Keyboard shortcuts
- Configuration management
- Role switching

### ❌ Out of Scope (Future Work)

- Knowledge graph visualization (D3.js)
- Config wizard (multi-step forms)
- Complex data visualization
- CLI interface component
- WebView components

---

## 2. Component Migration Map

### Phase 1: Search Interface

| Svelte Component | GPUI Implementation | Complexity |
|------------------|---------------------|------------|
| `Search/Search.svelte` | Custom `SearchView` + `Input` + `VirtualList` | High |
| `Search/KGSearchInput.svelte` | Custom `AutocompleteInput` + `Popover` | Medium |
| `Search/ResultItem.svelte` | Custom `ResultItemView` | Low |
| `Search/TermChip.svelte` | `Tag` component | Low |
| `Search/ArticleModal.svelte` | `Dialog` + custom content | Medium |
| `Search/KGSearchModal.svelte` | `Dialog` + search UI | Medium |

### Phase 2: Markdown Editor

| Current Component | GPUI Implementation | Complexity |
|-------------------|---------------------|------------|
| `Editor/NovelWrapper.svelte` | gpui-component `Editor` + extensions | High |
| Slash command system | Custom GPUI Actions + completion | Medium |
| MCP autocomplete | Integrate with `Editor` completion API | Medium |

### Phase 3: Chat Interface

| Svelte Component | GPUI Implementation | Complexity |
|------------------|---------------------|------------|
| `Chat/Chat.svelte` | Custom `ChatView` + `ScrollableArea` | High |
| `Chat/SessionList.svelte` | `List` or `VirtualList` | Low |
| `Chat/ContextEditModal.svelte` | `Dialog` + `Form` | Medium |
| `Search/KGContextItem.svelte` | Custom context item renderer | Low |

### Phase 4: Core UI

| Svelte Component | GPUI Implementation | Complexity |
|------------------|---------------------|------------|
| `App.svelte` (routing) | Custom workspace/navigation | Medium |
| `ThemeSwitcher.svelte` | `Switch` + GPUI Theme API | Low |
| `BackButton.svelte` | `Button` with action | Low |

---

## 3. Detailed Implementation Plan

### Phase 1: Foundation & Search (Weeks 1-3)

#### Week 1: Project Setup & Architecture

**Tasks:**
1. Create new GPUI workspace crate
   ```bash
   mkdir crates/terraphim_desktop_gpui
   cd crates/terraphim_desktop_gpui
   cargo init
   ```

2. Add dependencies to `Cargo.toml`:
   ```toml
   [dependencies]
   gpui = "0.1"
   gpui-component = "0.3"

   # Direct integration with terraphim crates
   terraphim_service = { path = "../terraphim_service" }
   terraphim_config = { path = "../terraphim_config" }
   terraphim_middleware = { path = "../terraphim_middleware" }
   terraphim_automata = { path = "../terraphim_automata" }
   terraphim_rolegraph = { path = "../terraphim_rolegraph" }
   terraphim_persistence = { path = "../terraphim_persistence" }

   tokio = { version = "1.36", features = ["full"] }
   anyhow = "1.0"
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   ```

3. Set up GPUI app structure:
   ```rust
   // src/main.rs
   use gpui::*;

   fn main() {
       App::new().run(|cx: &mut AppContext| {
           cx.open_window(WindowOptions::default(), |cx| {
               cx.new_view(|cx| TerraphimApp::new(cx))
           });
       });
   }

   struct TerraphimApp {
       current_view: View,
   }

   enum View {
       Search,
       Chat,
       Editor,
   }
   ```

4. Implement theme system:
   ```rust
   // src/theme.rs
   use gpui::*;
   use gpui_component::theme::*;

   pub struct TerraphimTheme {
       pub light: Theme,
       pub dark: Theme,
   }

   impl TerraphimTheme {
       pub fn new(cx: &mut AppContext) -> Self {
           // Configure Bulma-style colors
           Self {
               light: Theme::default_light(cx),
               dark: Theme::default_dark(cx),
           }
       }
   }
   ```

**Deliverable:** Minimal GPUI app that opens a window

---

#### Week 2: Search UI Implementation

**Task 1: Basic Search Input**
```rust
// src/views/search/input.rs
use gpui::*;
use gpui_component::{Input, Popover};
use terraphim_automata::Autocomplete;

pub struct SearchInput {
    query: SharedString,
    autocomplete: Model<AutocompleteState>,
    suggestions_open: bool,
}

impl SearchInput {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let autocomplete = cx.new_model(|cx| AutocompleteState::new(cx));
        Self {
            query: "".into(),
            autocomplete,
            suggestions_open: false,
        }
    }

    fn handle_input(&mut self, text: String, cx: &mut ViewContext<Self>) {
        self.query = text.clone().into();

        // Trigger autocomplete
        cx.spawn(|this, mut cx| async move {
            let suggestions = fetch_autocomplete(&text).await?;
            this.update(&mut cx, |this, cx| {
                this.update_suggestions(suggestions, cx);
            })
        }).detach();
    }
}

impl Render for SearchInput {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .child(
                Input::new()
                    .placeholder("Search knowledge graph...")
                    .value(self.query.clone())
                    .on_change(cx.listener(|this, text, cx| {
                        this.handle_input(text, cx);
                    }))
            )
            .child(
                // Autocomplete popover
                Popover::new()
                    .visible(self.suggestions_open)
                    .child(self.render_suggestions(cx))
            )
    }
}
```

**Task 2: Autocomplete Integration**
```rust
// src/views/search/autocomplete.rs
use terraphim_automata::{AutocompleteEngine, Suggestion};

pub struct AutocompleteState {
    engine: AutocompleteEngine,
    suggestions: Vec<Suggestion>,
    selected_index: usize,
}

impl AutocompleteState {
    pub fn new(cx: &mut ModelContext<Self>) -> Self {
        // Load thesaurus from config
        let engine = AutocompleteEngine::from_thesaurus(
            &load_thesaurus_for_role()
        ).unwrap();

        Self {
            engine,
            suggestions: vec![],
            selected_index: 0,
        }
    }

    pub async fn fetch_suggestions(&self, query: &str) -> Vec<Suggestion> {
        self.engine.autocomplete(query, 10)
    }

    pub fn handle_key(&mut self, key: &str, cx: &mut ModelContext<Self>) {
        match key {
            "ArrowDown" => self.selected_index =
                (self.selected_index + 1).min(self.suggestions.len() - 1),
            "ArrowUp" => self.selected_index =
                self.selected_index.saturating_sub(1),
            _ => {}
        }
        cx.notify();
    }
}
```

**Task 3: Search Results List**
```rust
// src/views/search/results.rs
use gpui::*;
use gpui_component::VirtualList;
use terraphim_types::Document;

pub struct SearchResults {
    results: Vec<Document>,
    selected_result: Option<usize>,
}

impl Render for SearchResults {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        VirtualList::new()
            .items(self.results.len())
            .render_item(cx.listener(|this, index, cx| {
                this.render_result_item(&this.results[index], cx)
            }))
            .on_select(cx.listener(|this, index, cx| {
                this.open_result_detail(index, cx);
            }))
    }
}

impl SearchResults {
    fn render_result_item(&self, doc: &Document, cx: &mut ViewContext<Self>)
        -> impl IntoElement
    {
        div()
            .class("result-item")
            .child(
                div().class("title").child(doc.title.clone())
            )
            .child(
                div().class("description").child(
                    doc.description.as_ref()
                        .unwrap_or(&"".to_string())
                        .clone()
                )
            )
            .child(
                div().class("url").child(doc.url.clone())
            )
            .on_click(cx.listener(move |this, _, cx| {
                this.open_result_detail_modal(doc.clone(), cx);
            }))
    }
}
```

**Deliverable:** Working search with autocomplete and results list

---

#### Week 3: Search Polish & Term Chips

**Task 1: Term Chips for Complex Queries**
```rust
// src/views/search/term_chips.rs
use gpui::*;
use gpui_component::Tag;

pub struct TermChips {
    terms: Vec<SelectedTerm>,
    operator: Option<LogicalOperator>,
}

#[derive(Clone)]
pub struct SelectedTerm {
    value: String,
    is_from_kg: bool,
}

#[derive(Clone)]
pub enum LogicalOperator {
    And,
    Or,
}

impl Render for TermChips {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("term-chips")
            .children(
                self.terms.iter().enumerate().flat_map(|(i, term)| {
                    let chip = Tag::new()
                        .label(&term.value)
                        .closable(true)
                        .variant(if term.is_from_kg {
                            TagVariant::Primary
                        } else {
                            TagVariant::Default
                        })
                        .on_close(cx.listener(move |this, _, cx| {
                            this.remove_term(i, cx);
                        }));

                    let operator = if i < self.terms.len() - 1 {
                        Some(Tag::new()
                            .label(match self.operator {
                                Some(LogicalOperator::And) => "AND",
                                Some(LogicalOperator::Or) => "OR",
                                None => "AND",
                            })
                            .variant(TagVariant::Light))
                    } else {
                        None
                    };

                    vec![Some(chip), operator].into_iter().flatten()
                })
            )
    }
}
```

**Task 2: Search State Management**
```rust
// src/state/search.rs
use gpui::*;
use terraphim_service::SearchService;

pub struct SearchState {
    service: Arc<SearchService>,
    query: String,
    results: Vec<Document>,
    loading: bool,
}

impl SearchState {
    pub fn new(cx: &mut ModelContext<Self>) -> Self {
        let service = Arc::new(SearchService::new(
            Config::load().unwrap()
        ));

        Self {
            service,
            query: String::new(),
            results: vec![],
            loading: false,
        }
    }

    pub fn search(&mut self, query: String, cx: &mut ModelContext<Self>) {
        self.query = query.clone();
        self.loading = true;
        cx.notify();

        let service = self.service.clone();
        cx.spawn(|this, mut cx| async move {
            let results = service.search(&query).await?;

            this.update(&mut cx, |this, cx| {
                this.results = results;
                this.loading = false;
                cx.notify();
            })
        }).detach();
    }
}
```

**Task 3: Result Detail Modal**
```rust
// src/views/search/article_modal.rs
use gpui::*;
use gpui_component::{Dialog, Button, Scrollable};

pub struct ArticleModal {
    document: Document,
    active: bool,
}

impl Render for ArticleModal {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        Dialog::new()
            .active(self.active)
            .title(&self.document.title)
            .child(
                Scrollable::new()
                    .child(
                        div()
                            .class("article-content")
                            .child(self.render_markdown(&self.document.body))
                    )
            )
            .footer(
                div()
                    .child(
                        Button::new()
                            .label("Open URL")
                            .on_click(cx.listener(|this, _, _| {
                                open::that(&this.document.url).ok();
                            }))
                    )
                    .child(
                        Button::new()
                            .label("Add to Context")
                            .variant(ButtonVariant::Primary)
                            .on_click(cx.listener(|this, _, cx| {
                                this.add_to_chat_context(cx);
                            }))
                    )
            )
            .on_close(cx.listener(|this, _, cx| {
                this.active = false;
                cx.notify();
            }))
    }
}
```

**Deliverable:** Complete search interface with modals and term chips

---

### Phase 2: Markdown Editor (Weeks 4-5)

#### Week 4: Editor Integration

**Task 1: Basic Editor Setup**
```rust
// src/views/editor/mod.rs
use gpui::*;
use gpui_component::Editor;

pub struct MarkdownEditor {
    editor: View<Editor>,
    slash_commands: Model<SlashCommandManager>,
}

impl MarkdownEditor {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let editor = cx.new_view(|cx| {
            Editor::new(cx)
                .language("markdown")
                .line_numbers(true)
                .syntax_highlighting(true)
        });

        let slash_commands = cx.new_model(|cx| {
            SlashCommandManager::new(cx)
        });

        Self {
            editor,
            slash_commands,
        }
    }
}

impl Render for MarkdownEditor {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("markdown-editor-container")
            .child(self.editor.clone())
            .child(
                // Slash command completion popup
                self.render_slash_completion(cx)
            )
    }
}
```

**Task 2: Slash Command System**
```rust
// src/views/editor/slash_commands.rs
use gpui::*;
use terraphim_mcp_server::MCPTool;

pub struct SlashCommandManager {
    commands: Vec<SlashCommand>,
    active_completion: Option<CompletionState>,
}

pub struct SlashCommand {
    name: String,
    description: String,
    handler: Box<dyn Fn(&str) -> String>,
}

impl SlashCommandManager {
    pub fn new(cx: &mut ModelContext<Self>) -> Self {
        let commands = vec![
            SlashCommand {
                name: "search".into(),
                description: "Search knowledge graph".into(),
                handler: Box::new(|query| {
                    // Execute KG search
                    format!("Search results for: {}", query)
                }),
            },
            SlashCommand {
                name: "autocomplete".into(),
                description: "Get term suggestions".into(),
                handler: Box::new(|prefix| {
                    // Get autocomplete suggestions
                    format!("Suggestions for: {}", prefix)
                }),
            },
            SlashCommand {
                name: "mcp".into(),
                description: "Execute MCP tool".into(),
                handler: Box::new(|tool_name| {
                    // Execute MCP tool
                    format!("Executing MCP tool: {}", tool_name)
                }),
            },
        ];

        Self {
            commands,
            active_completion: None,
        }
    }

    pub fn trigger_completion(&mut self, prefix: &str, cx: &mut ModelContext<Self>) {
        let matching = self.commands.iter()
            .filter(|cmd| cmd.name.starts_with(prefix))
            .cloned()
            .collect();

        self.active_completion = Some(CompletionState {
            prefix: prefix.to_string(),
            candidates: matching,
            selected: 0,
        });

        cx.notify();
    }
}
```

**Task 3: KG Autocomplete in Editor**
```rust
// src/views/editor/kg_completion.rs
use gpui::*;
use gpui_component::Popover;
use terraphim_automata::AutocompleteEngine;

pub struct KGCompletionProvider {
    engine: AutocompleteEngine,
}

impl KGCompletionProvider {
    pub fn new(role: &str) -> Self {
        let engine = AutocompleteEngine::from_role(role).unwrap();
        Self { engine }
    }

    pub fn get_completions(&self, prefix: &str, cursor_pos: usize)
        -> Vec<Completion>
    {
        // Extract word at cursor
        let word = extract_word_at_cursor(prefix, cursor_pos);

        // Get KG suggestions
        self.engine.autocomplete(&word, 10)
            .into_iter()
            .map(|suggestion| Completion {
                label: suggestion.term.clone(),
                detail: suggestion.definition.clone(),
                insert_text: suggestion.term.clone(),
                from_kg: true,
            })
            .collect()
    }
}

pub struct Completion {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: String,
    pub from_kg: bool,
}
```

**Deliverable:** Markdown editor with slash commands and KG autocomplete

---

#### Week 5: Editor Polish

**Task 1: Markdown Preview**
```rust
// src/views/editor/preview.rs
use gpui::*;
use pulldown_cmark::{Parser, html};

pub struct MarkdownPreview {
    source: String,
    rendered_html: String,
}

impl MarkdownPreview {
    pub fn new(source: String, cx: &mut ViewContext<Self>) -> Self {
        let rendered_html = Self::render_markdown(&source);
        Self {
            source,
            rendered_html,
        }
    }

    fn render_markdown(source: &str) -> String {
        let parser = Parser::new(source);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    }
}

impl Render for MarkdownPreview {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("markdown-preview")
            // Render HTML (GPUI doesn't have built-in HTML rendering)
            // Would need custom markdown renderer or use WebView component
            .child("Markdown preview: ")
            .child(self.source.clone())
    }
}
```

**Task 2: Code Highlighting**
```rust
// src/views/editor/syntax.rs
use gpui::*;
use tree_sitter_highlight::{Highlighter, HighlightConfiguration};

pub struct SyntaxHighlighter {
    highlighter: Highlighter,
    markdown_config: HighlightConfiguration,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let mut highlighter = Highlighter::new();

        let markdown_config = HighlightConfiguration::new(
            tree_sitter_markdown::language(),
            tree_sitter_markdown::HIGHLIGHT_QUERY,
            "",
            "",
        ).unwrap();

        Self {
            highlighter,
            markdown_config,
        }
    }

    pub fn highlight(&mut self, source: &str) -> Vec<HighlightedRange> {
        let highlights = self.highlighter.highlight(
            &self.markdown_config,
            source.as_bytes(),
            None,
            |_| None,
        ).unwrap();

        highlights.map(|h| h.unwrap()).collect()
    }
}
```

**Task 3: Editor Actions**
```rust
// src/views/editor/actions.rs
use gpui::*;

actions!(
    editor,
    [
        Save,
        InsertSlashCommand,
        TriggerKGAutocomplete,
        FormatMarkdown,
        TogglePreview,
    ]
);

pub fn register_editor_actions(cx: &mut AppContext) {
    cx.bind_keys([
        KeyBinding::new("cmd-s", Save, Some("Editor")),
        KeyBinding::new("/", InsertSlashCommand, Some("Editor")),
        KeyBinding::new("ctrl-space", TriggerKGAutocomplete, Some("Editor")),
        KeyBinding::new("cmd-shift-f", FormatMarkdown, Some("Editor")),
        KeyBinding::new("cmd-shift-p", TogglePreview, Some("Editor")),
    ]);
}
```

**Deliverable:** Full-featured markdown editor with syntax highlighting

---

### Phase 3: Chat Interface (Weeks 6-8)

#### Week 6: Chat UI Foundation

**Task 1: Message List**
```rust
// src/views/chat/messages.rs
use gpui::*;
use gpui_component::Scrollable;
use terraphim_service::ChatMessage;

pub struct ChatMessageList {
    messages: Vec<ChatMessage>,
    auto_scroll: bool,
}

impl Render for ChatMessageList {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        Scrollable::new()
            .class("chat-messages")
            .children(
                self.messages.iter().map(|msg| {
                    self.render_message(msg, cx)
                })
            )
    }
}

impl ChatMessageList {
    fn render_message(&self, msg: &ChatMessage, cx: &ViewContext<Self>)
        -> impl IntoElement
    {
        let is_user = msg.role == "user";

        div()
            .class(if is_user { "message-user" } else { "message-assistant" })
            .child(
                div()
                    .class("message-bubble")
                    .child(
                        if is_user {
                            // Plain text for user
                            div().child(msg.content.clone())
                        } else {
                            // Markdown for assistant
                            self.render_markdown(&msg.content, cx)
                        }
                    )
            )
            .when(msg.role == "assistant", |div| {
                div.child(
                    div()
                        .class("message-actions")
                        .child(
                            Button::new()
                                .icon("copy")
                                .variant(ButtonVariant::Ghost)
                                .on_click(cx.listener(|_, _, _| {
                                    // Copy to clipboard
                                }))
                        )
                )
            })
    }
}
```

**Task 2: Chat Input**
```rust
// src/views/chat/input.rs
use gpui::*;
use gpui_component::Input;

pub struct ChatInput {
    text: String,
    sending: bool,
}

impl Render for ChatInput {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("chat-input-container")
            .child(
                Input::new()
                    .placeholder("Type your message...")
                    .value(self.text.clone())
                    .multiline(true)
                    .rows(3)
                    .disabled(self.sending)
                    .on_change(cx.listener(|this, text, cx| {
                        this.text = text;
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|this, event, cx| {
                        if event.key == "Enter" && !event.shift_key {
                            this.send_message(cx);
                        }
                    }))
            )
            .child(
                Button::new()
                    .icon("send")
                    .variant(ButtonVariant::Primary)
                    .disabled(self.text.is_empty() || self.sending)
                    .on_click(cx.listener(|this, _, cx| {
                        this.send_message(cx);
                    }))
            )
    }
}

impl ChatInput {
    fn send_message(&mut self, cx: &mut ViewContext<Self>) {
        if self.text.trim().is_empty() {
            return;
        }

        let message = self.text.clone();
        self.text.clear();
        self.sending = true;
        cx.notify();

        cx.emit(ChatEvent::SendMessage(message));
    }
}
```

**Task 3: Chat State Management**
```rust
// src/state/chat.rs
use gpui::*;
use terraphim_service::ChatService;

pub struct ChatState {
    service: Arc<ChatService>,
    conversation_id: Option<String>,
    messages: Vec<ChatMessage>,
    context_items: Vec<ContextItem>,
    loading: bool,
}

impl ChatState {
    pub fn new(cx: &mut ModelContext<Self>) -> Self {
        let service = Arc::new(ChatService::new());

        Self {
            service,
            conversation_id: None,
            messages: vec![],
            context_items: vec![],
            loading: false,
        }
    }

    pub fn send_message(&mut self, text: String, cx: &mut ModelContext<Self>) {
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: text.clone(),
        });

        self.loading = true;
        cx.notify();

        let service = self.service.clone();
        let messages = self.messages.clone();

        cx.spawn(|this, mut cx| async move {
            let response = service.chat(messages).await?;

            this.update(&mut cx, |this, cx| {
                this.messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: response.message,
                });
                this.loading = false;
                cx.notify();
            })
        }).detach();
    }
}
```

**Deliverable:** Basic chat interface with message sending

---

#### Week 7: Context Management

**Task 1: Context Panel**
```rust
// src/views/chat/context_panel.rs
use gpui::*;
use gpui_component::{List, Button, Badge};

pub struct ContextPanel {
    context_items: Vec<ContextItem>,
    loading: bool,
}

impl Render for ContextPanel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("context-panel")
            .child(
                div()
                    .class("context-header")
                    .child("Context")
                    .child(
                        Badge::new()
                            .label(&format!("{}", self.context_items.len()))
                            .variant(BadgeVariant::Primary)
                    )
                    .child(
                        Button::new()
                            .icon("refresh")
                            .variant(ButtonVariant::Ghost)
                            .disabled(self.loading)
                            .on_click(cx.listener(|this, _, cx| {
                                this.refresh_context(cx);
                            }))
                    )
            )
            .child(
                List::new()
                    .items(self.context_items.clone())
                    .render_item(cx.listener(|this, item, cx| {
                        this.render_context_item(&item, cx)
                    }))
            )
    }
}

impl ContextPanel {
    fn render_context_item(&self, item: &ContextItem, cx: &ViewContext<Self>)
        -> impl IntoElement
    {
        div()
            .class("context-item")
            .child(
                div()
                    .class("context-item-header")
                    .child(
                        Badge::new()
                            .label(&item.context_type)
                            .variant(match item.context_type.as_str() {
                                "KGTermDefinition" => BadgeVariant::Primary,
                                "Document" => BadgeVariant::Info,
                                _ => BadgeVariant::Default,
                            })
                    )
                    .child(item.title.clone())
            )
            .child(
                div()
                    .class("context-item-content")
                    .child(
                        item.summary.as_ref()
                            .or(item.content.get(..150))
                            .unwrap_or("")
                    )
            )
            .child(
                div()
                    .class("context-item-actions")
                    .child(
                        Button::new()
                            .icon("edit")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Small)
                            .on_click(cx.listener(move |this, _, cx| {
                                this.edit_context_item(item.id.clone(), cx);
                            }))
                    )
                    .child(
                        Button::new()
                            .icon("trash")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Small)
                            .on_click(cx.listener(move |this, _, cx| {
                                this.delete_context_item(item.id.clone(), cx);
                            }))
                    )
            )
    }
}
```

**Task 2: KG Term Context**
```rust
// src/views/chat/kg_context.rs
use gpui::*;
use gpui_component::Dialog;
use terraphim_rolegraph::KGTerm;

pub struct KGContextItem {
    term: KGTerm,
    documents: Vec<Document>,
    expanded: bool,
}

impl Render for KGContextItem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("kg-context-item")
            .child(
                div()
                    .class("kg-term-header")
                    .child(
                        Badge::new()
                            .label("KG Term")
                            .variant(BadgeVariant::Primary)
                    )
                    .child(&self.term.normalized_term)
                    .child(
                        Button::new()
                            .icon(if self.expanded { "chevron-up" } else { "chevron-down" })
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(|this, _, cx| {
                                this.expanded = !this.expanded;
                                cx.notify();
                            }))
                    )
            )
            .when(self.expanded, |div| {
                div.child(
                    div()
                        .class("kg-term-details")
                        .child(
                            self.term.definition.as_ref()
                                .map(|def| div().child(def.clone()))
                        )
                        .child(
                            div()
                                .class("kg-term-synonyms")
                                .children(
                                    self.term.synonyms.iter().map(|syn| {
                                        Badge::new()
                                            .label(syn)
                                            .variant(BadgeVariant::Light)
                                    })
                                )
                        )
                        .child(
                            div()
                                .class("kg-term-documents")
                                .child("Related Documents:")
                                .children(
                                    self.documents.iter().map(|doc| {
                                        div().child(&doc.title)
                                    })
                                )
                        )
                )
            })
    }
}
```

**Task 3: Context Edit Modal**
```rust
// src/views/chat/context_edit.rs
use gpui::*;
use gpui_component::{Dialog, Form, Input, Select};

pub struct ContextEditModal {
    context: ContextItem,
    active: bool,
    mode: ContextEditMode,
}

pub enum ContextEditMode {
    Create,
    Edit,
}

impl Render for ContextEditModal {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        Dialog::new()
            .active(self.active)
            .title(match self.mode {
                ContextEditMode::Create => "Add Context",
                ContextEditMode::Edit => "Edit Context",
            })
            .child(
                Form::new()
                    .child(
                        Select::new()
                            .label("Type")
                            .options(vec![
                                "Document",
                                "SearchResult",
                                "UserInput",
                                "Note",
                            ])
                            .value(&self.context.context_type)
                    )
                    .child(
                        Input::new()
                            .label("Title")
                            .value(&self.context.title)
                            .on_change(cx.listener(|this, text, cx| {
                                this.context.title = text;
                                cx.notify();
                            }))
                    )
                    .child(
                        Input::new()
                            .label("Content")
                            .multiline(true)
                            .rows(6)
                            .value(&self.context.content)
                            .on_change(cx.listener(|this, text, cx| {
                                this.context.content = text;
                                cx.notify();
                            }))
                    )
            )
            .footer(
                div()
                    .child(
                        Button::new()
                            .label("Cancel")
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(|this, _, cx| {
                                this.active = false;
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new()
                            .label("Save")
                            .variant(ButtonVariant::Primary)
                            .on_click(cx.listener(|this, _, cx| {
                                this.save_context(cx);
                            }))
                    )
            )
    }
}
```

**Deliverable:** Full context management system

---

#### Week 8: Session History

**Task 1: Session List**
```rust
// src/views/chat/session_list.rs
use gpui::*;
use gpui_component::{List, Button, Scrollable};

pub struct SessionList {
    sessions: Vec<Conversation>,
    current_session_id: Option<String>,
}

impl Render for SessionList {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("session-list")
            .child(
                div()
                    .class("session-list-header")
                    .child("Conversations")
                    .child(
                        Button::new()
                            .icon("plus")
                            .label("New")
                            .variant(ButtonVariant::Primary)
                            .size(ButtonSize::Small)
                            .on_click(cx.listener(|this, _, cx| {
                                this.create_new_session(cx);
                            }))
                    )
            )
            .child(
                Scrollable::new()
                    .child(
                        List::new()
                            .items(self.sessions.clone())
                            .render_item(cx.listener(|this, session, cx| {
                                this.render_session_item(&session, cx)
                            }))
                    )
            )
    }
}

impl SessionList {
    fn render_session_item(&self, session: &Conversation, cx: &ViewContext<Self>)
        -> impl IntoElement
    {
        let is_current = self.current_session_id.as_ref() == Some(&session.id);

        div()
            .class("session-item")
            .when(is_current, |div| div.class("session-item-active"))
            .child(
                div()
                    .class("session-title")
                    .child(
                        session.title.clone()
                            .unwrap_or_else(|| "Untitled".to_string())
                    )
            )
            .child(
                div()
                    .class("session-meta")
                    .child(format_timestamp(&session.updated_at))
                    .child(
                        Badge::new()
                            .label(&format!("{} msgs", session.messages.len()))
                    )
            )
            .on_click(cx.listener(move |this, _, cx| {
                this.load_session(session.id.clone(), cx);
            }))
    }
}
```

**Task 2: Persistent Storage**
```rust
// src/state/persistence.rs
use gpui::*;
use terraphim_persistence::{PersistenceBackend, Conversation};

pub struct ConversationStore {
    backend: Box<dyn PersistenceBackend>,
}

impl ConversationStore {
    pub fn new(cx: &mut ModelContext<Self>) -> Self {
        let backend = create_persistence_backend();
        Self { backend }
    }

    pub async fn list_conversations(&self) -> Result<Vec<Conversation>> {
        self.backend.list_conversations().await
    }

    pub async fn load_conversation(&self, id: &str) -> Result<Conversation> {
        self.backend.get_conversation(id).await
    }

    pub async fn save_conversation(&mut self, conv: &Conversation) -> Result<()> {
        self.backend.save_conversation(conv).await
    }

    pub async fn delete_conversation(&mut self, id: &str) -> Result<()> {
        self.backend.delete_conversation(id).await
    }
}

fn create_persistence_backend() -> Box<dyn PersistenceBackend> {
    // Use SQLite by default
    Box::new(SqliteBackend::new("~/.terraphim/conversations.db"))
}
```

**Task 3: Auto-save & Sync**
```rust
// src/state/autosave.rs
use gpui::*;
use std::time::Duration;

pub struct AutoSaveManager {
    dirty: bool,
    last_save: Instant,
    save_interval: Duration,
}

impl AutoSaveManager {
    pub fn new(cx: &mut ModelContext<Self>) -> Self {
        let manager = Self {
            dirty: false,
            last_save: Instant::now(),
            save_interval: Duration::from_secs(30),
        };

        // Start autosave timer
        cx.spawn(|this, mut cx| async move {
            loop {
                Timer::after(Duration::from_secs(5)).await;

                this.update(&mut cx, |this, cx| {
                    if this.should_save() {
                        this.save(cx);
                    }
                }).ok();
            }
        }).detach();

        manager
    }

    pub fn mark_dirty(&mut self, cx: &mut ModelContext<Self>) {
        self.dirty = true;
        cx.notify();
    }

    fn should_save(&self) -> bool {
        self.dirty && self.last_save.elapsed() >= self.save_interval
    }

    fn save(&mut self, cx: &mut ModelContext<Self>) {
        // Trigger save
        cx.emit(AutoSaveEvent::Save);
        self.dirty = false;
        self.last_save = Instant::now();
    }
}
```

**Deliverable:** Complete chat interface with persistent sessions

---

### Phase 4: Integration & Polish (Weeks 9-10)

#### Week 9: App Integration

**Task 1: Navigation System**
```rust
// src/app.rs
use gpui::*;

pub struct TerraphimApp {
    current_view: AppView,
    search_view: View<SearchView>,
    chat_view: View<ChatView>,
    editor_view: View<EditorView>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AppView {
    Search,
    Chat,
    Editor,
}

impl TerraphimApp {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let search_view = cx.new_view(|cx| SearchView::new(cx));
        let chat_view = cx.new_view(|cx| ChatView::new(cx));
        let editor_view = cx.new_view(|cx| EditorView::new(cx));

        Self {
            current_view: AppView::Search,
            search_view,
            chat_view,
            editor_view,
        }
    }

    pub fn navigate_to(&mut self, view: AppView, cx: &mut ViewContext<Self>) {
        self.current_view = view;
        cx.notify();
    }
}

impl Render for TerraphimApp {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("app-container")
            .child(self.render_navigation(cx))
            .child(
                match self.current_view {
                    AppView::Search => self.search_view.clone().into_any(),
                    AppView::Chat => self.chat_view.clone().into_any(),
                    AppView::Editor => self.editor_view.clone().into_any(),
                }
            )
    }
}
```

**Task 2: Keyboard Shortcuts**
```rust
// src/actions.rs
use gpui::*;

actions!(
    app,
    [
        NavigateToSearch,
        NavigateToChat,
        NavigateToEditor,
        ToggleTheme,
        OpenSettings,
        NewConversation,
        GlobalSearch,
    ]
);

pub fn register_app_actions(cx: &mut AppContext) {
    cx.bind_keys([
        KeyBinding::new("cmd-1", NavigateToSearch, None),
        KeyBinding::new("cmd-2", NavigateToChat, None),
        KeyBinding::new("cmd-3", NavigateToEditor, None),
        KeyBinding::new("cmd-shift-t", ToggleTheme, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-n", NewConversation, Some("Chat")),
        KeyBinding::new("cmd-k", GlobalSearch, None),
    ]);
}
```

**Task 3: Settings UI**
```rust
// src/views/settings.rs
use gpui::*;
use gpui_component::{Dialog, Form, Select, Switch};

pub struct SettingsModal {
    config: Config,
    active: bool,
}

impl Render for SettingsModal {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        Dialog::new()
            .active(self.active)
            .title("Settings")
            .large()
            .child(
                Form::new()
                    .child(
                        Select::new()
                            .label("Active Role")
                            .options(self.config.roles.keys().collect())
                            .value(&self.config.active_role)
                    )
                    .child(
                        Switch::new()
                            .label("Dark Theme")
                            .checked(self.config.theme == "dark")
                            .on_change(cx.listener(|this, checked, cx| {
                                this.config.theme = if checked { "dark" } else { "light" };
                                cx.notify();
                            }))
                    )
                    .child(
                        Select::new()
                            .label("LLM Provider")
                            .options(vec!["ollama", "openrouter"])
                            .value(&self.config.llm_provider)
                    )
            )
    }
}
```

**Deliverable:** Integrated app with navigation and settings

---

#### Week 10: Testing & Polish

**Task 1: Unit Tests**
```rust
// src/views/search/tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_search_autocomplete(cx: &mut TestAppContext) {
        let view = cx.new_view(|cx| SearchInput::new(cx));

        view.update(cx, |view, cx| {
            view.handle_input("rus".to_string(), cx);
        });

        // Wait for autocomplete
        cx.run_until_parked();

        view.update(cx, |view, cx| {
            assert!(!view.suggestions.is_empty());
            assert!(view.suggestions[0].term.starts_with("rus"));
        });
    }

    #[gpui::test]
    async fn test_search_execution(cx: &mut TestAppContext) {
        let state = cx.new_model(|cx| SearchState::new(cx));

        state.update(cx, |state, cx| {
            state.search("rust async".to_string(), cx);
        });

        cx.run_until_parked();

        state.update(cx, |state, cx| {
            assert!(!state.results.is_empty());
            assert!(!state.loading);
        });
    }
}
```

**Task 2: Integration Tests**
```rust
// tests/integration_test.rs
use terraphim_desktop_gpui::*;
use gpui::TestAppContext;

#[gpui::test]
async fn test_search_to_chat_workflow(cx: &mut TestAppContext) {
    let app = cx.new_view(|cx| TerraphimApp::new(cx));

    // 1. Search for term
    app.update(cx, |app, cx| {
        app.navigate_to(AppView::Search, cx);
    });

    // Perform search
    app.search_view.update(cx, |view, cx| {
        view.search("rust tokio".to_string(), cx);
    });

    cx.run_until_parked();

    // 2. Add result to chat context
    app.search_view.update(cx, |view, cx| {
        let first_result = view.results[0].clone();
        view.add_to_chat_context(first_result, cx);
    });

    // 3. Navigate to chat
    app.update(cx, |app, cx| {
        app.navigate_to(AppView::Chat, cx);
    });

    // 4. Verify context loaded
    app.chat_view.update(cx, |view, cx| {
        assert_eq!(view.context_items.len(), 1);
        assert_eq!(view.context_items[0].title, "rust tokio");
    });
}
```

**Task 3: Performance Profiling**
```rust
// benches/rendering.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use terraphim_desktop_gpui::*;

fn bench_search_results_rendering(c: &mut Criterion) {
    c.bench_function("render_1000_results", |b| {
        let results = generate_mock_results(1000);

        b.iter(|| {
            let view = SearchResults::new(black_box(results.clone()));
            view.render();
        });
    });
}

criterion_group!(benches, bench_search_results_rendering);
criterion_main!(benches);
```

**Deliverable:** Tested, polished application ready for beta

---

## 4. Success Criteria

### Performance Targets
- ✅ **60+ FPS** during search result scrolling (1000+ items)
- ✅ **<50ms** autocomplete latency
- ✅ **<100ms** search response time (local index)
- ✅ **<200ms** chat message rendering

### Feature Completeness
- ✅ Search with KG autocomplete working
- ✅ Multi-term queries with AND/OR operators
- ✅ Markdown editor with slash commands
- ✅ Chat with persistent context
- ✅ Session history saved to disk
- ✅ Theme switching functional
- ✅ Keyboard shortcuts responsive

### Quality Metrics
- ✅ **>80%** code coverage for core logic
- ✅ **Zero** memory leaks detected
- ✅ **<100MB** idle memory usage
- ✅ Cross-platform build success (macOS, Linux)

---

## 5. Risk Assessment

### Low Risk ✅
- Basic UI components (buttons, inputs, lists)
- Search result rendering
- Theme system
- Configuration management

### Medium Risk ⚠️
- Autocomplete performance with large thesaurus
- Markdown rendering (no native HTML in GPUI)
- Slash command system complexity
- Cross-platform keyboard shortcuts

### High Risk 🔴
- GPUI framework breaking changes (pre-1.0)
- gpui-component maturity issues
- Custom markdown editor extensions
- LSP integration with editor

---

## 6. Mitigation Strategies

### For Framework Instability
1. **Pin exact versions** of GPUI and gpui-component
2. **Monitor releases** and test before upgrading
3. **Maintain migration guide** for breaking changes
4. **Build abstractions** to isolate GPUI-specific code

### For Editor Complexity
1. **Start simple** - Plain text first, add features incrementally
2. **Reuse Zed patterns** - Learn from mature GPUI editor
3. **Consider WebView fallback** - For complex markdown preview
4. **Limit LSP scope** - Focus on autocomplete, defer diagnostics

### For Performance Issues
1. **Profile early** - Benchmark each phase
2. **Use VirtualList** - For all long lists (>50 items)
3. **Lazy load** - Don't render off-screen content
4. **Debounce inputs** - Especially autocomplete triggers

---

## 7. Next Steps

### Immediate Actions (This Week)
1. ✅ **Approval** - Get team buy-in on this plan
2. 📋 **Setup** - Create `terraphim_desktop_gpui` crate
3. 🎯 **Prototype** - Build minimal search UI in GPUI
4. 📊 **Benchmark** - Compare Tauri vs GPUI performance

### Phase 1 Kickoff (Next Week)
1. Set up development environment
2. Configure GPUI project structure
3. Integrate terraphim_* crates
4. Implement theme system
5. Build first search input prototype

### Key Decisions Needed
- [ ] Markdown rendering approach (custom vs WebView)
- [ ] Editor component choice (gpui-component vs custom)
- [ ] Persistence backend (SQLite vs RocksDB)
- [ ] Release strategy (alpha/beta timeline)

---

## 8. Resources

### Documentation
- [GPUI Documentation](https://www.gpui.rs/)
- [gpui-component Docs](https://longbridge.github.io/gpui-component/)
- [Zed Editor Source](https://github.com/zed-industries/zed) - Reference GPUI implementation

### Examples
- [Awesome GPUI Projects](https://github.com/zed-industries/awesome-gpui)
- [gpui-component Examples](https://github.com/longbridge/gpui-component/tree/main/examples)

### Tools
- **GPUI DevTools** - Coming soon from Zed team
- **cargo-watch** - For live reload during development
- **criterion** - For performance benchmarking

---

## Appendix: Component Equivalence Table

| Svelte Component | GPUI Implementation | Notes |
|------------------|---------------------|-------|
| `<input>` | `Input::new()` | Direct equivalent |
| `<button>` | `Button::new()` | Direct equivalent |
| `<select>` | `Select::new()` | Direct equivalent |
| `<checkbox>` | `Checkbox::new()` | Direct equivalent |
| `{#each}` | `.children(items.iter().map(...))` | Iterator pattern |
| `{#if cond}` | `.when(cond, |el| el.child(...))` | Conditional rendering |
| `on:click` | `.on_click(cx.listener(...))` | Event handler |
| `bind:value` | Managed via state + `on_change` | Two-way binding manual |
| CSS classes | `.class("name")` | Tailwind-style API |
| Svelte stores | GPUI `Model<T>` | Reactive state |

---

**End of Migration Plan**

*This is a living document. Update as implementation progresses.*
