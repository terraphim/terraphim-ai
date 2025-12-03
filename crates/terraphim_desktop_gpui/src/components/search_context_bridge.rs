use gpui::*;
use std::sync::Arc;
use ulid::Ulid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::components::{ReusableComponent, ComponentConfig, PerformanceTracker, ContextComponent, ComponentError, ViewContext, LifecycleEvent, ServiceRegistry};
use crate::views::search::{AddToContextEvent, OpenArticleEvent};
use terraphim_types::{Document, ContextItem, ContextType};

/// Configuration for search-context bridge component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContextBridgeConfig {
    /// Whether to show add-to-context buttons in search results
    pub show_add_to_context: bool,
    /// Whether to show chat-with-document buttons
    pub show_chat_with_document: bool,
    /// Whether to enable batch operations
    pub enable_batch_operations: bool,
    /// Whether to show context preview
    pub show_context_preview: bool,
    /// Whether to enable context suggestions
    pub enable_suggestions: bool,
    /// Maximum number of items to add at once
    pub max_batch_size: usize,
    /// Theme colors
    pub theme: SearchContextTheme,
}

impl Default for SearchContextBridgeConfig {
    fn default() -> Self {
        Self {
            show_add_to_context: true,
            show_chat_with_document: true,
            enable_batch_operations: true,
            show_context_preview: true,
            enable_suggestions: true,
            max_batch_size: 10,
            theme: SearchContextTheme::default(),
        }
    }
}

/// Theme configuration for search-context bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContextTheme {
    pub add_button: gpui::Rgba,
    pub chat_button: gpui::Rgba,
    pub batch_button: gpui::Rgba,
    pub success: gpui::Rgba,
    pub warning: gpui::Rgba,
    pub error: gpui::Rgba,
    pub preview_bg: gpui::Rgba,
    pub suggestion_bg: gpui::Rgba,
}

impl Default for SearchContextTheme {
    fn default() -> Self {
        Self {
            add_button: gpui::Rgba::from_rgb(0.2, 0.7, 0.2),
            chat_button: gpui::Rgba::from_rgb(0.2, 0.5, 0.8),
            batch_button: gpui::Rgba::from_rgb(0.6, 0.2, 0.8),
            success: gpui::Rgba::from_rgb(0.2, 0.7, 0.2),
            warning: gpui::Rgba::from_rgb(0.8, 0.6, 0.0),
            error: gpui::Rgba::from_rgb(0.8, 0.2, 0.2),
            preview_bg: gpui::Rgba::from_rgb(0.98, 0.98, 1.0),
            suggestion_bg: gpui::Rgba::from_rgb(0.95, 0.97, 1.0),
        }
    }
}

/// Bridge state for search-context integration
#[derive(Debug, Clone)]
pub struct SearchContextBridgeState {
    /// Recently added items for batch operations
    selected_items: Vec<Arc<Document>>,
    /// Context items that have been added from search
    added_contexts: Vec<Arc<ContextItem>>,
    /// Currently showing preview
    preview_item: Option<Arc<ContextItem>>,
    /// UI state
    show_batch_mode: bool,
    show_suggestions: bool,
    last_add_result: Option<String>,
    /// Mount state
    is_mounted: bool,
    /// Performance metrics
    items_added_count: usize,
    last_update: std::time::Instant,
}

/// Events emitted by SearchContextBridge
#[derive(Debug, Clone)]
pub enum SearchContextBridgeEvent {
    /// Document was added to context
    DocumentAdded {
        document: Arc<Document>,
        context_item: Arc<ContextItem>,
    },
    /// Multiple documents were added to context
    BatchAdded {
        documents: Vec<Arc<Document>>,
        context_items: Vec<Arc<ContextItem>>,
    },
    /// User wants to chat with document
    ChatWithDocument {
        document: Arc<Document>,
        context_item: Arc<ContextItem>,
    },
    /// Context operation failed
    OperationFailed {
        error: String,
        document: Option<Arc<Document>>,
    },
}

/// Bridge component that connects search results to context management
pub struct SearchContextBridge {
    config: SearchContextBridgeConfig,
    state: SearchContextBridgeState,
    performance_tracker: PerformanceTracker,
    id: String,
    context_component: ContextComponent,
    event_emitter: Option<Box<dyn gpui::EventEmitter<SearchContextBridgeEvent>>>,
}

impl SearchContextBridge {
    /// Create a new search-context bridge
    pub fn new(config: SearchContextBridgeConfig) -> Self {
        let id = Ulid::new().to_string().to_string();

        Self {
            context_component: ContextComponent::new(crate::components::ContextComponentConfig::default()),
            config,
            state: SearchContextBridgeState {
                selected_items: Vec::new(),
                added_contexts: Vec::new(),
                preview_item: None,
                show_batch_mode: false,
                show_suggestions: false,
                last_add_result: None,
                is_mounted: false,
                items_added_count: 0,
                last_update: std::time::Instant::now(),
            },
            performance_tracker: PerformanceTracker::new(id.clone()),
            id,
            event_emitter: None,
        }
    }

    /// Add a document to context
    pub async fn add_document_to_context(
        &mut self,
        document: Arc<Document>,
        cx: &mut gpui::Context<'_, Self>,
    ) -> Result<Arc<ContextItem>, String> {
        log::info!("Adding document to context: {}", document.title);

        self.performance_tracker.start_tracking("add_to_context").unwrap();

        // Convert document to context item
        let context_item = self.document_to_context_item(document.clone())?;

        // Add to our context component
        self.context_component.add_item((*context_item).clone())?;

        // Update state
        self.state.added_contexts.push(context_item.clone());
        self.state.items_added_count += 1;
        self.state.last_add_result = Some(format!("Added '{}' to context", document.title));
        self.state.last_update = std::time::Instant::now();

        // Emit event
        if let Some(ref event_emitter) = self.event_emitter {
            event_emitter.update(cx, |emitter, cx| {
                emitter.emit(SearchContextBridgeEvent::DocumentAdded {
                    document,
                    context_item: context_item.clone(),
                }, cx);
            });
        }

        self.performance_tracker.end_tracking("add_to_context").unwrap();

        Ok(context_item)
    }

    /// Add multiple documents to context
    pub async fn add_documents_to_context(
        &mut self,
        documents: Vec<Arc<Document>>,
        cx: &mut gpui::Context<'_, Self>,
    ) -> Result<Vec<Arc<ContextItem>>, String> {
        if documents.len() > self.config.max_batch_size {
            return Err(format!("Cannot add more than {} items at once", self.config.max_batch_size));
        }

        log::info!("Adding {} documents to context", documents.len());

        self.performance_tracker.start_tracking("batch_add_to_context").unwrap();

        let mut context_items = Vec::new();
        let mut errors = Vec::new();

        for document in documents {
            match self.document_to_context_item(document.clone()) {
                Ok(context_item) => {
                    if let Err(e) = self.context_component.add_item((*context_item).clone()) {
                        errors.push(format!("Failed to add '{}': {}", document.title, e));
                    } else {
                        context_items.push(context_item.clone());
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to convert '{}': {}", document.title, e));
                }
            }
        }

        if context_items.is_empty() {
            return Err(format!("Failed to add any documents: {}", errors.join("; ")));
        }

        // Update state
        self.state.added_contexts.extend(context_items.clone());
        self.state.items_added_count += context_items.len();
        self.state.last_add_result = Some(format!("Added {} items to context", context_items.len()));
        self.state.last_update = std::time::Instant::now();

        // Emit event
        self.event_emitter.update(cx, |emitter, cx| {
            emitter.emit(SearchContextBridgeEvent::BatchAdded {
                documents: self.state.selected_items.clone(),
                context_items: context_items.clone(),
            }, cx);
        });

        self.performance_tracker.end_tracking("batch_add_to_context").unwrap();

        Ok(context_items)
    }

    /// Chat with document (adds to context and prepares for chat)
    pub async fn chat_with_document(
        &mut self,
        document: Arc<Document>,
        cx: &mut gpui::Context<'_, Self>,
    ) -> Result<Arc<ContextItem>, String> {
        log::info!("Preparing to chat with document: {}", document.title);

        // First add to context
        let context_item = self.add_document_to_context(document.clone(), cx).await?;

        // Emit chat-specific event
        if let Some(ref event_emitter) = self.event_emitter {
            event_emitter.update(cx, |emitter, cx| {
                emitter.emit(SearchContextBridgeEvent::ChatWithDocument {
                    document,
                    context_item: context_item.clone(),
                }, cx);
            });
        }

        Ok(context_item)
    }

    /// Convert document to context item
    fn document_to_context_item(&self, document: Arc<Document>) -> Result<Arc<ContextItem>, String> {
        if document.title.is_empty() {
            return Err("Document title cannot be empty".to_string());
        }

        if document.body.is_empty() {
            return Err("Document content cannot be empty".to_string());
        }

        let context_item = ContextItem {
            id: ulid::Ulid::new().to_string(),
            title: document.title.clone(),
            summary: document.description.clone(),
            content: document.body.clone(),
            context_type: ContextType::SearchResult,
            created_at: Utc::now(),
            relevance_score: document.rank.map(|r| r as f64),
            metadata: {
                let mut metadata = ahash::AHashMap::new();
                metadata.insert("source".to_string(), "search".to_string());
                metadata.insert("url".to_string(), document.url.clone());
                if let Some(tags) = &document.tags {
                    metadata.insert("tags".to_string(), tags.join(", "));
                }
                metadata
            },
        };

        Ok(Arc::new(context_item))
    }

    /// Toggle batch selection mode
    pub fn toggle_batch_mode(&mut self) {
        self.state.show_batch_mode = !self.state.show_batch_mode;
        if !self.state.show_batch_mode {
            self.state.selected_items.clear();
        }
        self.state.last_update = std::time::Instant::now();
    }

    /// Toggle document selection in batch mode
    pub fn toggle_document_selection(&mut self, document: Arc<Document>) {
        if let Some(pos) = self.state.selected_items.iter().position(|d| d.url == document.url) {
            self.state.selected_items.remove(pos);
        } else {
            if self.state.selected_items.len() < self.config.max_batch_size {
                self.state.selected_items.push(document);
            }
        }
        self.state.last_update = std::time::Instant::now();
    }

    /// Check if document is selected in batch mode
    pub fn is_document_selected(&self, document: &Arc<Document>) -> bool {
        self.state.selected_items.iter().any(|d| d.url == document.url)
    }

    /// Get selected documents for batch operations
    pub fn get_selected_documents(&self) -> &[Arc<Document>] {
        &self.state.selected_items
    }

    /// Clear selected documents
    pub fn clear_selection(&mut self) {
        self.state.selected_items.clear();
        self.state.last_update = std::time::Instant::now();
    }

    /// Select all current documents (for batch mode)
    pub fn select_all_documents(&mut self, documents: &[Arc<Document>]) {
        self.state.selected_items = documents
            .iter()
            .take(self.config.max_batch_size)
            .cloned()
            .collect();
        self.state.last_update = std::time::Instant::now();
    }

    /// Show context preview for a document
    pub fn show_context_preview(&mut self, document: Arc<Document>) -> Result<(), String> {
        let context_item = self.document_to_context_item(document)?;
        self.state.preview_item = Some(context_item);
        self.state.last_update = std::time::Instant::now();
        Ok(())
    }

    /// Hide context preview
    pub fn hide_context_preview(&mut self) {
        self.state.preview_item = None;
        self.state.last_update = std::time::Instant::now();
    }

    /// Toggle suggestions display
    pub fn toggle_suggestions(&mut self) {
        self.state.show_suggestions = !self.state.show_suggestions;
        self.state.last_update = std::time::Instant::now();
    }

    /// Get context component for integration
    pub fn get_context_component(&self) -> &ContextComponent {
        &self.context_component
    }

    /// Get mutable context component
    pub fn get_context_component_mut(&mut self) -> &mut ContextComponent {
        &mut self.context_component
    }

    /// Get statistics
    pub fn get_stats(&self) -> SearchContextBridgeStats {
        SearchContextBridgeStats {
            items_added: self.state.items_added_count,
            selected_for_batch: self.state.selected_items.len(),
            max_batch_size: self.config.max_batch_size,
            total_contexts: self.state.added_contexts.len(),
            performance_stats: self.performance_tracker.get_summary(),
        }
    }

    /// Get recent add result
    pub fn get_last_add_result(&self) -> Option<&String> {
        self.state.last_add_result.as_ref()
    }

    /// Clear last add result
    pub fn clear_last_add_result(&mut self) {
        self.state.last_add_result = None;
    }

    /// Subscribe to events
    pub fn subscribe<F, C>(&self, cx: &mut C, callback: F) -> gpui::Subscription
    where
        C: AppContext,
        F: Fn(&SearchContextBridgeEvent, &mut C) + 'static,
    {
        cx.subscribe(&self.event_emitter, move |_, event, cx| {
            callback(event, cx);
        })
    }
}

/// Statistics for the search-context bridge
#[derive(Debug, Clone)]
pub struct SearchContextBridgeStats {
    pub items_added: usize,
    pub selected_for_batch: usize,
    pub max_batch_size: usize,
    pub total_contexts: usize,
    pub performance_stats: crate::components::PerformanceTracker,
}

