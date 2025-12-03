use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{button::*, input::InputState, IconName, StyledExt};
use terraphim_types::{ContextItem, ContextType};

use crate::theme::colors::theme;

/// Context edit modal for creating/editing context items
/// Matches functionality from desktop/src/lib/Chat/ContextEditModal.svelte
pub struct ContextEditModal {
    is_open: bool,
    mode: ContextEditMode,
    editing_context: Option<ContextItem>,
    
    // Form fields
    title_state: Option<Entity<InputState>>,
    summary_state: Option<Entity<InputState>>,
    content_state: Option<Entity<InputState>>,
    context_type: ContextType,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContextEditMode {
    Create,
    Edit,
}

/// Events emitted by ContextEditModal
#[derive(Clone, Debug)]
pub enum ContextEditModalEvent {
    Create(ContextItem),
    Update(ContextItem),
    Delete(String),  // context_id
    Close,
}

impl EventEmitter<ContextEditModalEvent> for ContextEditModal {}

impl ContextEditModal {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            is_open: false,
            mode: ContextEditMode::Create,
            editing_context: None,
            title_state: None,
            summary_state: None,
            content_state: None,
            context_type: ContextType::Document,
        }
    }

    /// Open modal for creating a new context item
    pub fn open_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("Opening context edit modal in create mode");
        self.mode = ContextEditMode::Create;
        self.editing_context = None;
        self.context_type = ContextType::Document;
        
        // Initialize form fields
        self.title_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));
        self.summary_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));
        self.content_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));
        
        self.is_open = true;
        cx.notify();
    }

    /// Open modal for editing an existing context item
    pub fn open_edit(&mut self, context_item: ContextItem, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("Opening context edit modal in edit mode for: {}", context_item.title);
        self.mode = ContextEditMode::Edit;
        self.editing_context = Some(context_item.clone());
        self.context_type = context_item.context_type.clone();
        
        // Initialize form fields - values will be set after creation
        self.title_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));
        self.summary_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));
        self.content_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));
        
        // Set values after creation
        if let Some(title_state) = &self.title_state {
            title_state.update(cx, |input, input_cx| {
                input.set_value(
                    gpui::SharedString::from(context_item.title.clone()),
                    window,
                    input_cx
                );
            });
        }
        if let Some(summary_state) = &self.summary_state {
            summary_state.update(cx, |input, input_cx| {
                input.set_value(
                    gpui::SharedString::from(context_item.summary.clone().unwrap_or_default()),
                    window,
                    input_cx
                );
            });
        }
        if let Some(content_state) = &self.content_state {
            content_state.update(cx, |input, input_cx| {
                input.set_value(
                    gpui::SharedString::from(context_item.content.clone()),
                    window,
                    input_cx
                );
            });
        }
        
        self.is_open = true;
        cx.notify();
    }

    /// Open modal with a document (for adding search results to context)
    pub fn open_with_document(&mut self, document: terraphim_types::Document, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("Opening context edit modal with document: {}", document.title);
        
        // Pre-populate from document
        let context_item = ContextItem {
            id: ulid::Ulid::new().to_string(),
            context_type: ContextType::Document,
            title: document.title.clone(),
            summary: document.description.clone(),
            content: document.body.clone(),
            metadata: {
                let mut meta = ahash::AHashMap::new();
                meta.insert("document_id".to_string(), document.id.clone());
                if !document.url.is_empty() {
                    meta.insert("url".to_string(), document.url.clone());
                }
                if let Some(tags) = &document.tags {
                    meta.insert("tags".to_string(), tags.join(", "));
                }
                if let Some(rank) = document.rank {
                    meta.insert("rank".to_string(), rank.to_string());
                }
                meta
            },
            created_at: chrono::Utc::now(),
            relevance_score: document.rank.map(|r| r as f64),
        };
        
        self.open_edit(context_item, window, cx);
    }

    /// Close the modal
    pub fn close(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        log::info!("Closing context edit modal");
        self.is_open = false;
        self.editing_context = None;
        cx.emit(ContextEditModalEvent::Close);
        cx.notify();
    }

    /// Handle save button click
    fn handle_save(&mut self, cx: &mut Context<Self>) {
        let title = self.title_state.as_ref()
            .and_then(|s| Some(s.read(cx).value().to_string()))
            .unwrap_or_default();
        let summary = self.summary_state.as_ref()
            .and_then(|s| {
                let val = s.read(cx).value().to_string();
                if val.trim().is_empty() { None } else { Some(val) }
            });
        let content = self.content_state.as_ref()
            .and_then(|s| Some(s.read(cx).value().to_string()))
            .unwrap_or_default();

        // Validation: title and content are required
        if title.trim().is_empty() || content.trim().is_empty() {
            log::warn!("Cannot save context: title or content is empty");
            return;
        }

        let context_item = if let Some(mut existing) = self.editing_context.clone() {
            // Update existing item
            existing.title = title;
            existing.summary = summary;
            existing.content = content;
            existing.context_type = self.context_type.clone();
            existing
        } else {
            // Create new item
            ContextItem {
                id: ulid::Ulid::new().to_string(),
                context_type: self.context_type.clone(),
                title,
                summary,
                content,
                metadata: ahash::AHashMap::new(),
                created_at: chrono::Utc::now(),
                relevance_score: None,
            }
        };

        match self.mode {
            ContextEditMode::Create => {
                log::info!("Emitting Create event for context item: {}", context_item.title);
                cx.emit(ContextEditModalEvent::Create(context_item));
            }
            ContextEditMode::Edit => {
                log::info!("Emitting Update event for context item: {}", context_item.title);
                cx.emit(ContextEditModalEvent::Update(context_item));
            }
        }

        // Close modal by setting is_open to false and emitting close event
        self.is_open = false;
        self.editing_context = None;
        cx.emit(ContextEditModalEvent::Close);
        cx.notify();
    }

    /// Handle delete button click (only in edit mode)
    fn handle_delete(&mut self, cx: &mut Context<Self>) {
        if let Some(context_item) = &self.editing_context {
            log::info!("Emitting Delete event for context item: {}", context_item.id);
            cx.emit(ContextEditModalEvent::Delete(context_item.id.clone()));
            self.is_open = false;
            self.editing_context = None;
            cx.emit(ContextEditModalEvent::Close);
            cx.notify();
        }
    }

    /// Check if form is valid (title and content are required)
    fn is_valid(&self, cx: &Context<Self>) -> bool {
        let title = self.title_state.as_ref()
            .map(|s| s.read(cx).value().to_string())
            .unwrap_or_default();
        let content = self.content_state.as_ref()
            .map(|s| s.read(cx).value().to_string())
            .unwrap_or_default();
        
        !title.trim().is_empty() && !content.trim().is_empty()
    }
}

impl Render for ContextEditModal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open {
            return div().into_any_element();
        }

        let is_valid = self.is_valid(cx);
        let mode_label = match self.mode {
            ContextEditMode::Create => "Add Context Item",
            ContextEditMode::Edit => "Edit Context Item",
        };

        // Modal overlay
        div()
            .absolute()
            .inset_0()
            .bg(theme::text_primary())
            .opacity(0.95)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .relative()
                    .w(px(800.0))
                    .max_w_full()
                    .max_h(px(700.0))
                    .bg(theme::background())
                    .border_2()
                    .border_color(theme::border())
                    .rounded_lg()
                    .shadow_xl()
                    .flex()
                    .flex_col()
                    .child(
                        // Header
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .py_4()
                            .border_b_1()
                            .border_color(theme::border())
                            .child(
                                div()
                                    .text_xl()
                                    .font_bold()
                                    .text_color(theme::text_primary())
                                    .child(mode_label)
                            )
                            .child(
                                Button::new("close-context-modal")
                                    .icon(IconName::Delete)
                                    .ghost()
                                    .on_click(cx.listener(Self::close))
                            )
                    )
                    .child(
                        // Body with form fields
                        div()
                            .flex_1()
                            .px_6()
                            .py_4()
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_4()
                                    .child(
                                        // Context Type (simplified - just show current type)
                                        div()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .text_color(theme::text_secondary())
                                                    .mb_2()
                                                    .child("Type: Document")
                                            )
                                    )
                                    .child(
                                        // Title field
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .text_color(theme::text_primary())
                                                    .child("Title *")
                                            )
                                            .children(
                                                self.title_state.as_ref().map(|input_state| {
                                                    gpui_component::input::Input::new(input_state)
                                                })
                                            )
                                    )
                                    .child(
                                        // Summary field
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .text_color(theme::text_primary())
                                                    .child("Summary (optional)")
                                            )
                                            .children(
                                                self.summary_state.as_ref().map(|input_state| {
                                                    gpui_component::input::Input::new(input_state)
                                                })
                                            )
                                    )
                                    .child(
                                        // Content field (larger textarea)
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .text_color(theme::text_primary())
                                                    .child("Content *")
                                            )
                                            .children(
                                                self.content_state.as_ref().map(|input_state| {
                                                    gpui_component::input::Input::new(input_state)
                                                })
                                            )
                                    )
                            )
                    )
                    .child(
                        // Footer with buttons
                        div()
                            .px_6()
                            .py_4()
                            .border_t_1()
                            .border_color(theme::border())
                            .bg(theme::surface())
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("save-context")
                                    .label(match self.mode {
                                        ContextEditMode::Create => "Add Context",
                                        ContextEditMode::Edit => "Save Changes",
                                    })
                                    .primary()
                                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                                        if this.is_valid(cx) {
                                            this.handle_save(cx);
                                        }
                                    }))
                            )
                            .child(
                                Button::new("cancel-context")
                                    .label("Cancel")
                                    .outline()
                                    .on_click(cx.listener(Self::close))
                            )
                            .children(
                                if self.mode == ContextEditMode::Edit {
                                    Some(
                                        Button::new("delete-context")
                                            .label("Delete")
                                            .outline()
                                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                                this.handle_delete(cx);
                                            }))
                                    )
                                } else {
                                    None
                                }
                            )
                    )
            )
            .into_any_element()
    }
}
