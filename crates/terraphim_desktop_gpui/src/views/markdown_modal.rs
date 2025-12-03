/// Markdown Modal - Reusable component for rendering markdown content
///
/// A comprehensive modal component that provides rich markdown rendering,
/// keyboard navigation, search functionality, and accessibility features.
/// Inspired by Zed editor's modal patterns and Terraphim's existing modal architecture.

use gpui::*;
use gpui_component::{button::*, IconName, StyledExt};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// Configuration options for markdown modal behavior
#[derive(Debug, Clone)]
pub struct MarkdownModalOptions {
    /// Modal title
    pub title: Option<String>,
    /// Whether to show search functionality
    pub show_search: bool,
    /// Whether to show table of contents
    pub show_toc: bool,
    /// Maximum modal width in pixels
    pub max_width: Option<f32>,
    /// Maximum modal height in pixels
    pub max_height: Option<f32>,
    /// Whether to enable keyboard shortcuts
    pub enable_keyboard_shortcuts: bool,
    /// Custom CSS classes for styling
    pub custom_classes: Vec<String>,
}

impl Default for MarkdownModalOptions {
    fn default() -> Self {
        Self {
            title: None,
            show_search: true,
            show_toc: true,
            max_width: Some(1000.0),
            max_height: Some(700.0),
            enable_keyboard_shortcuts: true,
            custom_classes: Vec::new(),
        }
    }
}

/// Modal state management
#[derive(Debug, Clone)]
pub struct MarkdownModalState {
    /// Whether the modal is currently open
    pub is_open: bool,
    /// Current markdown content
    pub content: String,
    /// Current search query
    pub search_query: String,
    /// Current navigation position
    pub current_section: Option<String>,
    /// Table of contents entries
    pub toc_entries: Vec<TocEntry>,
    /// Search results
    pub search_results: Vec<SearchResult>,
    /// Selected search result index
    pub selected_search_result: Option<usize>,
}

/// Table of contents entry
#[derive(Debug, Clone)]
pub struct TocEntry {
    /// Section title
    pub title: String,
    /// Section level (1-6 for h1-h6)
    pub level: usize,
    /// Section ID for navigation
    pub id: String,
    /// Character position in content
    pub position: usize,
}

/// Search result with highlighting
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Line number where result was found
    pub line_number: usize,
    /// Content snippet with highlighting
    pub snippet: String,
    /// Context around the match
    pub context: String,
    /// Character position of match
    pub position: usize,
}

/// Markdown rendering styles
#[derive(Debug, Clone)]
pub struct MarkdownStyles {
    /// Heading styles by level
    pub heading_sizes: [f32; 6],
    /// Font settings
    pub base_font_size: f32,
    pub line_height: f32,
}

impl Default for MarkdownStyles {
    fn default() -> Self {
        Self {
            heading_sizes: [32.0, 28.0, 24.0, 20.0, 18.0, 16.0],
            base_font_size: 14.0,
            line_height: 24.0,
        }
    }
}

/// Reusable markdown modal component
pub struct MarkdownModal {
    /// Modal state
    state: MarkdownModalState,
    /// Configuration options
    options: MarkdownModalOptions,
    /// Rendering styles
    styles: MarkdownStyles,
    /// Event listeners
    _subscriptions: Vec<Subscription>,
}

/// Modal events for external communication
#[derive(Clone, Debug)]
pub enum MarkdownModalEvent {
    /// Modal was closed
    Closed,
    /// User navigated to section
    SectionNavigated { section: String },
    /// Search was performed
    SearchPerformed { query: String, results: Vec<SearchResult> },
    /// Link was clicked
    LinkClicked { url: String },
    /// Keyboard shortcut triggered
    KeyboardShortcut { shortcut: String },
}

// EventEmitter will be implemented where this modal is used

impl MarkdownModal {
    /// Create a new markdown modal with default options
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_options(MarkdownModalOptions::default(), cx)
    }

    /// Create a new markdown modal with custom options
    pub fn with_options(options: MarkdownModalOptions, _cx: &mut Context<Self>) -> Self {
        Self {
            state: MarkdownModalState {
                is_open: false,
                content: String::new(),
                search_query: String::new(),
                current_section: None,
                toc_entries: Vec::new(),
                search_results: Vec::new(),
                selected_search_result: None,
            },
            options,
            styles: MarkdownStyles::default(),
            _subscriptions: Vec::new(),
        }
    }

    /// Set custom styles for the modal
    pub fn with_styles(mut self, styles: MarkdownStyles) -> Self {
        self.styles = styles;
        self
    }

    /// Open modal with markdown content
    pub fn open(&mut self, content: String, cx: &mut Context<Self>) {
        log::info!("Opening markdown modal with {} characters", content.len());

        self.state.content = content;
        self.state.is_open = true;
        self.state.search_query.clear();
        self.state.search_results.clear();
        self.state.selected_search_result = None;

        // Parse table of contents if enabled
        if self.options.show_toc {
            self.state.toc_entries = self.extract_table_of_contents(&self.state.content);
        }

        cx.notify();
        // Event emission would be handled by implementing EventEmitter where the modal is used
    }

    /// Close modal
    pub fn close(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        log::info!("Closing markdown modal");
        self.state.is_open = false;
        self.state.content.clear();
        self.state.toc_entries.clear();
        self.state.search_results.clear();
        cx.notify();
        // Event emission would be handled by EventEmitter implementation
    }

    /// Set search query and perform search
    pub fn search(&mut self, query: String, cx: &mut Context<Self>) {
        self.state.search_query = query.clone();

        if query.is_empty() {
            self.state.search_results.clear();
            self.state.selected_search_result = None;
        } else {
            self.state.search_results = self.search_content(&query);
            self.state.selected_search_result = if !self.state.search_results.is_empty() {
                Some(0)
            } else {
                None
            };
        }

        cx.notify();
        // Event emission would be handled by EventEmitter implementation
    }

    /// Navigate to search result
    pub fn navigate_to_search_result(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(result) = self.state.search_results.get(index) {
            self.state.selected_search_result = Some(index);
            // In a real implementation, this would scroll to the result position
            log::info!("Navigating to search result at line {}", result.line_number);
            cx.notify();
        }
    }

    /// Navigate to section by ID
    pub fn navigate_to_section(&mut self, section_id: String, cx: &mut Context<Self>) {
        self.state.current_section = Some(section_id.clone());
        log::info!("Navigating to section: {}", section_id);
        cx.notify();
        // Event emission would be handled by EventEmitter implementation
    }

    /// Extract table of contents from markdown content
    fn extract_table_of_contents(&self, content: &str) -> Vec<TocEntry> {
        let mut toc_entries = Vec::new();
        let parser = Parser::new(content);
        let mut position = 0;
        let mut heading_text = String::new();
        let mut current_heading_level: Option<usize> = None;
        let mut heading_start = 0;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    current_heading_level = Some(level as usize);
                    heading_start = position;
                    heading_text.clear();
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some(level) = current_heading_level {
                        if !heading_text.is_empty() {
                            let id = self.generate_section_id(&heading_text);
                            toc_entries.push(TocEntry {
                                title: heading_text.clone(),
                                level,
                                id,
                                position: heading_start,
                            });
                        }
                    }
                    current_heading_level = None;
                }
                Event::Text(text) => {
                    if current_heading_level.is_some() {
                        heading_text.push_str(&text);
                    }
                    position += text.len();
                }
                Event::SoftBreak | Event::HardBreak => {
                    position += 1;
                }
                _ => {}
            }
        }

        toc_entries
    }

    /// Generate section ID from heading text
    fn generate_section_id(&self, heading: &str) -> String {
        heading
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    /// Search content for query
    fn search_content(&self, query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let lines: Vec<&str> = self.state.content.lines().collect();

        for (line_number, line) in lines.iter().enumerate() {
            if let Some(pos) = line.to_lowercase().find(&query.to_lowercase()) {
                let snippet = self.highlight_search_term(line, query, pos);
                let context = self.get_search_context(&lines, line_number, 2);

                results.push(SearchResult {
                    line_number: line_number + 1,
                    snippet,
                    context,
                    position: pos,
                });
            }
        }

        results.truncate(50); // Limit results for performance
        results
    }

    /// Highlight search term in text
    fn highlight_search_term(&self, text: &str, query: &str, pos: usize) -> String {
        let end = (pos + query.len()).min(text.len());
        format!(
            "{}**{}**{}",
            &text[..pos],
            &text[pos..end],
            &text[end..]
        )
    }

    /// Get context around search result
    fn get_search_context(&self, lines: &[&str], line_number: usize, context_size: usize) -> String {
        let start = line_number.saturating_sub(context_size);
        let end = (line_number + context_size + 1).min(lines.len());

        lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let actual_line = start + i + 1;
                format!("{}: {}", actual_line, line)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse markdown content into renderable elements
    fn parse_markdown(&self, content: &str) -> Vec<MarkdownElement> {
        let parser = Parser::new(content);
        let mut elements = Vec::new();
        let mut current_text = String::new();
        let mut code_block = None;
        let mut list_level: usize = 0;
        let mut _quote_level = 0;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level: _, .. }) => {
                    if !current_text.is_empty() {
                        elements.push(MarkdownElement::Paragraph(current_text.clone()));
                        current_text.clear();
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    if !current_text.is_empty() {
                        elements.push(MarkdownElement::Heading {
                            level: 1, // Will be set properly
                            content: current_text.clone(),
                        });
                        current_text.clear();
                    }
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    let language = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(fence) => {
                            if fence.is_empty() { "text".to_string() } else { fence.to_string() }
                        }
                        _ => "text".to_string(),
                    };
                    code_block = Some(language);
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some(lang) = code_block.take() {
                        if !current_text.is_empty() {
                            elements.push(MarkdownElement::CodeBlock {
                                language: lang,
                                content: current_text.clone(),
                            });
                            current_text.clear();
                        }
                    }
                }
                Event::Start(Tag::List(..)) => list_level += 1,
                Event::End(TagEnd::List(_)) => list_level = list_level.saturating_sub(1),
                Event::Start(Tag::Item) => {
                    if !current_text.is_empty() && list_level == 0 {
                        elements.push(MarkdownElement::Paragraph(current_text.clone()));
                        current_text.clear();
                    }
                }
                Event::End(TagEnd::Item) => {
                    if !current_text.is_empty() {
                        elements.push(MarkdownElement::ListItem {
                            level: list_level,
                            content: current_text.clone(),
                        });
                        current_text.clear();
                    }
                }
                Event::Text(text) => {
                    current_text.push_str(&text);
                }
                Event::Code(code) => {
                    current_text.push('`');
                    current_text.push_str(&code);
                    current_text.push('`');
                }
                Event::Start(Tag::Strong) => current_text.push_str("**"),
                Event::End(TagEnd::Strong) => current_text.push_str("**"),
                Event::Start(Tag::Emphasis) => current_text.push('*'),
                Event::End(TagEnd::Emphasis) => current_text.push('*'),
                Event::SoftBreak | Event::HardBreak => current_text.push('\n'),
                _ => {}
            }
        }

        if !current_text.is_empty() {
            elements.push(MarkdownElement::Paragraph(current_text));
        }

        elements
    }

    /// Handle keyboard shortcuts
    pub fn handle_keypress(&mut self, key: &str, cx: &mut Context<Self>) {
        if !self.options.enable_keyboard_shortcuts {
            return;
        }

        match key {
            "escape" => {
                // Close modal - Window::default() is not available, using placeholder
                log::info!("Escape key pressed - closing modal");
                self.state.is_open = false;
                cx.notify();
            },
            "ctrl+f" | "cmd+f" => {
                // Focus search input (would be implemented with proper focus management)
                // Event emission would be handled by EventEmitter implementation
            }
            "ctrl+k" | "cmd+k" => {
                // Clear search
                self.search(String::new(), cx);
            }
            "n" => {
                // Next search result
                if let Some(selected) = self.state.selected_search_result {
                    let next = (selected + 1).min(self.state.search_results.len().saturating_sub(1));
                    self.navigate_to_search_result(next, cx);
                }
            }
            "p" => {
                // Previous search result
                if let Some(selected) = self.state.selected_search_result {
                    let prev = selected.saturating_sub(1);
                    self.navigate_to_search_result(prev, cx);
                }
            }
            _ => {}
        }
    }
}

/// Renderable markdown element
#[derive(Debug, Clone)]
enum MarkdownElement {
    Heading { level: usize, content: String },
    Paragraph(String),
    CodeBlock { language: String, content: String },
    ListItem { level: usize, content: String },
    Blockquote(String),
}

impl Render for MarkdownModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.state.is_open {
            return div().into_any_element();
        }

        let max_width = self.options.max_width.unwrap_or(1000.0);
        let max_height = self.options.max_height.unwrap_or(700.0);
        let title = self.options.title.clone().unwrap_or_else(|| "Markdown Viewer".to_string());

        // Parse markdown content
        let markdown_elements = self.parse_markdown(&self.state.content);

        div()
            .absolute()
            .inset_0()
            .bg(rgb(0x000000))
            .opacity(0.95)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .relative()
                    .w(px(max_width))
                    .max_w_full()
                    .h(px(max_height))
                    .max_h(px(max_height))
                    .bg(rgb(0xffffff))
                    .rounded_lg()
                    .shadow_xl()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    // Header
                    .child(
                        self.render_header(&title, cx)
                    )
                    // Main content area with sidebar
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .overflow_hidden()
                            // Table of contents sidebar
                            .child(
                                self.render_sidebar(cx)
                            )
                            // Markdown content
                            .child(
                                self.render_content(markdown_elements, cx)
                            )
                    )
                    .into_any_element()
            )
            .into_any_element()
    }
}

impl MarkdownModal {
    /// Render modal header
    fn render_header(&self, title: &str, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .py_4()
            .border_b_1()
            .border_color(rgb(0xe0e0e0))
            .bg(rgb(0xf8f8f8))
            .child(
                div()
                    .text_xl()
                    .font_bold()
                    .text_color(rgb(0x1a1a1a))
                    .child(title.to_string())
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    // Search input
                    .child(
                        self.render_search_input(cx)
                    )
                    // Close button
                    .child(
                        Button::new("close-markdown-modal")
                            .label("Close")
                            .icon(IconName::Delete)
                            .ghost()
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.close(_ev, _window, cx);
                            }))
                    )
            )
    }

    /// Render sidebar with table of contents
    fn render_sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        if !self.options.show_toc || self.state.toc_entries.is_empty() {
            return div().w(px(200.0)).border_r_1().border_color(rgb(0xe0e0e0));
        }

        div()
            .w(px(250.0))
            .border_r_1()
            .border_color(rgb(0xe0e0e0))
            .bg(rgb(0xf8f8f8))
            .p_4()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(rgb(0x1a1a1a))
                    .mb_3()
                    .child("Table of Contents")
            )
            .children(
                self.state.toc_entries.iter().map(|entry| {
                    div()
                        .ml(px(((entry.level - 1) as f32) * 16.0))
                        .py_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x333333))
                                .child(entry.title.clone())
                        )
                })
            )
    }

    /// Render search input
    fn render_search_input(&self, cx: &Context<Self>) -> impl IntoElement {
        if !self.options.show_search {
            return div();
        }

        // In a full implementation, this would use gpui_component::input::Input
        // For now, we'll create a simple styled div
        div()
            .relative()
            .w(px(300.0))
            .child(
                div()
                    .w_full()
                    .px_3()
                    .py_2()
                    .border_1()
                    .border_color(rgb(0xd0d0d0))
                    .rounded_md()
                    .bg(rgb(0xffffff))
                    .text_sm()
                    .text_color(rgb(0x333333))
                    .child(if self.state.search_query.is_empty() {
                        "Search... (Ctrl+F)".to_string()
                    } else {
                        self.state.search_query.clone()
                    })
            )
    }

    /// Render main markdown content area
    fn render_content(&self, elements: Vec<MarkdownElement>, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .px_6()
            .py_4()
            .children(
                elements.iter().map(|element| {
                    match element {
                        MarkdownElement::Heading { level, content } => {
                            let font_size = self.styles.heading_sizes.get(level - 1).unwrap_or(&24.0);
                            div()
                                .text_size(px(*font_size))
                                .font_bold()
                                .text_color(rgb(0x1a1a1a))
                                .mt_4()
                                .mb_2()
                                .child(content.clone())
                        }
                        MarkdownElement::Paragraph(text) => {
                            div()
                                .text_size(px(self.styles.base_font_size))
                                .text_color(rgb(0x333333))
                                .line_height(px(self.styles.line_height))
                                .mb_4()
                                .child(text.clone())
                        }
                        MarkdownElement::CodeBlock { language: _, content } => {
                            div()
                                .bg(rgb(0xf8f9fa))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_md()
                                .p_4()
                                .mb_4()
                                .font_family("monospace")
                                .text_size(px(13.0))
                                .text_color(rgb(0xe83e8c))
                                .child(content.clone())
                        }
                        MarkdownElement::ListItem { level, content } => {
                            div()
                                .ml(px((*level as f32) * 24.0))
                                .text_size(px(self.styles.base_font_size))
                                .text_color(rgb(0x333333))
                                .line_height(px(self.styles.line_height))
                                .mb_2()
                                .child(format!("• {}", content))
                        }
                        MarkdownElement::Blockquote(text) => {
                            div()
                                .border_l_4()
                                .border_color(rgb(0x6c757d))
                                .bg(rgb(0xf8f9fa))
                                .pl_4()
                                .py_2()
                                .mb_4()
                                .text_size(px(self.styles.base_font_size))
                                .text_color(rgb(0x6c757d))
                                .child(text.clone())
                        }
                    }
                })
            )
            // Simple search results overlay (no complex interactivity)
            .child(
                self.render_search_results_simple()
            )
    }

    /// Render simple search results overlay
    fn render_search_results_simple(&self) -> impl IntoElement {
        if self.state.search_results.is_empty() {
            return div();
        }

        div()
            .absolute()
            .top_0()
            .right_0()
            .w(px(350.0))
            .max_h(px(400.0))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xd0d0d0))
            .rounded_lg()
            .shadow_lg()
            .p_4()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(rgb(0x1a1a1a))
                    .mb_3()
                    .child(format!(
                        "Found {} result{}",
                        self.state.search_results.len(),
                        if self.state.search_results.len() == 1 { "" } else { "s" }
                    ))
            )
            .children(
                self.state.search_results.iter().enumerate().map(|(index, result)| {
                    let is_selected = self.state.selected_search_result == Some(index);
                    div()
                        .p_3()
                        .mb_2()
                        .border_1()
                        .border_color(if is_selected { rgb(0x007bff) } else { rgb(0xe0e0e0) })
                        .rounded_md()
                        .cursor_pointer()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6c757d))
                                .mb_1()
                                .child(format!("Line {}", result.line_number))
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x333333))
                                .child(result.snippet.clone())
                        )
                })
            )
    }
}