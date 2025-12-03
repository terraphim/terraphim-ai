use gpui::*;
use std::sync::Arc;
use ulid::Ulid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::components::{ReusableComponent, ComponentConfig, PerformanceTracker, ComponentError, ViewContext, LifecycleEvent, ServiceRegistry};
use terraphim_types::{ContextItem, ContextType};

/// Configuration for context item component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItemComponentConfig {
    /// Whether to show edit controls
    pub enable_editing: bool,
    /// Whether to show delete controls
    pub enable_deletion: bool,
    /// Whether to show metadata
    pub show_metadata: bool,
    /// Whether to show relevance scores
    pub show_relevance: bool,
    /// Whether to show timestamps
    pub show_timestamps: bool,
    /// Maximum content preview length
    pub max_preview_length: usize,
    /// Theme colors
    pub theme: ContextItemTheme,
}

impl Default for ContextItemComponentConfig {
    fn default() -> Self {
        Self {
            enable_editing: true,
            enable_deletion: true,
            show_metadata: true,
            show_relevance: true,
            show_timestamps: true,
            max_preview_length: 200,
            theme: ContextItemTheme::default(),
        }
    }
}

/// Theme configuration for context item component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItemTheme {
    pub background: gpui::Rgba,
    pub border: gpui::Rgba,
    pub text_primary: gpui::Rgba,
    pub text_secondary: gpui::Rgba,
    pub accent: gpui::Rgba,
    pub hover: gpui::Rgba,
    pub selected: gpui::Rgba,
    pub success: gpui::Rgba,
    pub warning: gpui::Rgba,
    pub error: gpui::Rgba,
    pub edit_mode: gpui::Rgba,
}

impl Default for ContextItemTheme {
    fn default() -> Self {
        Self {
            background: gpui::Rgba::white(),
            border: gpui::Rgba::from_rgb(0.85, 0.85, 0.85),
            text_primary: gpui::Rgba::from_rgb(0.1, 0.1, 0.1),
            text_secondary: gpui::Rgba::from_rgb(0.5, 0.5, 0.5),
            accent: gpui::Rgba::from_rgb(0.2, 0.5, 0.8),
            hover: gpui::Rgba::from_rgb(0.95, 0.95, 0.98),
            selected: gpui::Rgba::from_rgb(0.9, 0.95, 1.0),
            success: gpui::Rgba::from_rgb(0.2, 0.7, 0.2),
            warning: gpui::Rgba::from_rgb(0.8, 0.6, 0.0),
            error: gpui::Rgba::from_rgb(0.8, 0.2, 0.2),
            edit_mode: gpui::Rgba::from_rgb(1.0, 0.98, 0.9),
        }
    }
}

/// Editing modes for context items
#[derive(Debug, Clone, PartialEq)]
pub enum EditingMode {
    View,
    Edit,
    Metadata,
}

/// State for context item component
#[derive(Debug, Clone)]
pub struct ContextItemComponentState {
    /// The context item being managed
    pub item: Option<Arc<ContextItem>>,
    /// Current editing mode
    pub editing_mode: EditingMode,
    /// Editing state - temporary values during editing
    pub edit_title: String,
    pub edit_summary: String,
    pub edit_content: String,
    pub edit_relevance: Option<f64>,
    pub edit_metadata: ahash::AHashMap<String, String>,
    /// UI state
    pub is_expanded: bool,
    pub is_selected: bool,
    pub is_hovered: bool,
    pub show_metadata: bool,
    /// Form validation state
    pub title_error: Option<String>,
    pub content_error: Option<String>,
    /// Mount state
    pub is_mounted: bool,
    /// Performance metrics
    pub last_update: std::time::Instant,
}

/// Events that can be emitted by the context item component
#[derive(Debug, Clone, PartialEq)]
pub enum ContextItemComponentEvent {
    /// Item was selected
    ItemSelected,
    /// Item was deselected
    ItemDeselected,
    /// Item was expanded
    ItemExpanded,
    /// Item was collapsed
    ItemCollapsed,
    /// Edit mode started
    EditModeStarted,
    /// Edit mode cancelled
    EditModeCancelled,
    /// Item was saved
    ItemSaved,
    /// Item was deleted
    ItemDeleted,
}

/// Individual context item component with full CRUD operations
pub struct ContextItemComponent {
    config: ContextItemComponentConfig,
    state: ContextItemComponentState,
    performance_tracker: PerformanceTracker,
    id: String,
}

impl ContextItemComponent {
    /// Create a new context item component
    pub fn new(config: ContextItemComponentConfig) -> Self {
        let id = Ulid::new().to_string().to_string();

        Self {
            config,
            state: ContextItemComponentState {
                item: None,
                editing_mode: EditingMode::View,
                edit_title: String::new(),
                edit_summary: String::new(),
                edit_content: String::new(),
                edit_relevance: None,
                edit_metadata: ahash::AHashMap::new(),
                is_expanded: false,
                is_selected: false,
                is_hovered: false,
                show_metadata: false,
                title_error: None,
                content_error: None,
                is_mounted: false,
                last_update: std::time::Instant::now(),
            },
            performance_tracker: PerformanceTracker::new(id.clone()),
            id,
        }
    }

    /// Set the context item
    pub fn set_item(&mut self, item: Arc<ContextItem>) {
        self.state.item = Some(item.clone());
        self.reset_edit_state();
        self.state.last_update = std::time::Instant::now();
    }

    /// Get the current context item
    pub fn get_item(&self) -> Option<Arc<ContextItem>> {
        self.state.item.clone()
    }

    /// Start editing the item
    pub fn start_editing(&mut self) -> Result<(), String> {
        if let Some(item) = &self.state.item {
            self.state.editing_mode = EditingMode::Edit;
            self.state.edit_title = item.title.clone();
            self.state.edit_summary = item.summary.clone().unwrap_or_default();
            self.state.edit_content = item.content.clone();
            self.state.edit_relevance = item.relevance_score;
            self.state.edit_metadata = item.metadata.clone();
            self.state.last_update = std::time::Instant::now();
            Ok(())
        } else {
            Err("No item to edit".to_string())
        }
    }

    /// Cancel editing
    pub fn cancel_editing(&mut self) {
        self.state.editing_mode = EditingMode::View;
        self.reset_edit_state();
        self.state.title_error = None;
        self.state.content_error = None;
        self.state.last_update = std::time::Instant::now();
    }

    /// Save edits
    pub fn save_edits(&mut self, cx: &mut gpui::Context<Self>) -> Result<(), String> {
        if let Some(item) = &self.state.item {
            // Validate inputs
            if self.state.edit_title.trim().is_empty() {
                self.state.title_error = Some("Title cannot be empty".to_string());
                return Err("Title cannot be empty".to_string());
            }

            if self.state.edit_content.trim().is_empty() {
                self.state.content_error = Some("Content cannot be empty".to_string());
                return Err("Content cannot be empty".to_string());
            }

            // Create updated item
            let new_item = Arc::new(ContextItem {
                id: item.id.clone(),
                title: self.state.edit_title.trim().to_string(),
                summary: if self.state.edit_summary.trim().is_empty() {
                    None
                } else {
                    Some(self.state.edit_summary.trim().to_string())
                },
                content: self.state.edit_content.trim().to_string(),
                context_type: item.context_type.clone(),
                created_at: item.created_at,
                relevance_score: self.state.edit_relevance,
                metadata: self.state.edit_metadata.clone(),
            });

            self.state.item = Some(new_item.clone());
            self.state.editing_mode = EditingMode::View;
            self.reset_edit_state();
            self.state.title_error = None;
            self.state.content_error = None;
            self.state.last_update = std::time::Instant::now();

            Ok(())
        } else {
            Err("No item to save".to_string())
        }
    }

    /// Delete the item
    pub fn delete_item(&mut self, cx: &mut gpui::Context<Self>) -> Result<(), String> {
        if let Some(_item) = self.state.item.clone() {
            self.state.item = None;
            self.state.last_update = std::time::Instant::now();
            Ok(())
        } else {
            Err("No item to delete".to_string())
        }
    }

    /// Toggle selection
    pub fn toggle_selection(&mut self, cx: &mut gpui::Context<Self>) {
        self.state.is_selected = !self.state.is_selected;
        self.state.last_update = std::time::Instant::now();
    }

    /// Toggle expansion
    pub fn toggle_expansion(&mut self, cx: &mut gpui::Context<Self>) {
        self.state.is_expanded = !self.state.is_expanded;
        self.state.last_update = std::time::Instant::now();
    }

    /// Set hover state
    pub fn set_hovered(&mut self, hovered: bool) {
        self.state.is_hovered = hovered;
        self.state.last_update = std::time::Instant::now();
    }

    /// Toggle metadata display
    pub fn toggle_metadata(&mut self) {
        self.state.show_metadata = !self.state.show_metadata;
        self.state.last_update = std::time::Instant::now();
    }

    /// Update edit title
    pub fn set_edit_title(&mut self, title: String) {
        self.state.edit_title = title;
        self.state.title_error = None;
        self.state.last_update = std::time::Instant::now();
    }

    /// Update edit summary
    pub fn set_edit_summary(&mut self, summary: String) {
        self.state.edit_summary = summary;
        self.state.last_update = std::time::Instant::now();
    }

    /// Update edit content
    pub fn set_edit_content(&mut self, content: String) {
        self.state.edit_content = content;
        self.state.content_error = None;
        self.state.last_update = std::time::Instant::now();
    }

    /// Update edit relevance
    pub fn set_edit_relevance(&mut self, relevance: Option<f64>) {
        self.state.edit_relevance = relevance.filter(|r| (0.0..=1.0).contains(r));
        self.state.last_update = std::time::Instant::now();
    }

    /// Update edit metadata
    pub fn set_edit_metadata(&mut self, metadata: ahash::AHashMap<String, String>) {
        self.state.edit_metadata = metadata;
        self.state.last_update = std::time::Instant::now();
    }

    /// Reset editing state to match current item
    fn reset_edit_state(&mut self) {
        if let Some(item) = &self.state.item {
            self.state.edit_title = item.title.clone();
            self.state.edit_summary = item.summary.clone().unwrap_or_default();
            self.state.edit_content = item.content.clone();
            self.state.edit_relevance = item.relevance_score;
            self.state.edit_metadata = item.metadata.clone();
        }
    }

    /// Get truncated content for preview
    fn get_content_preview(&self) -> String {
        if let Some(item) = &self.state.item {
            if item.content.len() > self.config.max_preview_length {
                format!("{}...", &item.content[..self.config.max_preview_length])
            } else {
                item.content.clone()
            }
        } else {
            String::new()
        }
    }
}

