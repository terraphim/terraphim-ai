use gpui::*;
use std::sync::Arc;
use ulid::Ulid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::components::{ReusableComponent, ComponentConfig, PerformanceTracker, ComponentError, ViewContext, LifecycleEvent, ServiceRegistry};
use terraphim_types::{ContextItem, ContextType};

/// Configuration for context management component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextComponentConfig {
    /// Maximum number of context items to display
    pub max_display_items: usize,
    /// Whether to show item metadata
    pub show_metadata: bool,
    /// Whether to enable item selection
    pub enable_selection: bool,
    /// Whether to show relevance scores
    pub show_relevance: bool,
    /// Whether to enable item editing
    pub enable_editing: bool,
    /// Theme colors
    pub theme: ContextTheme,
    /// Animation settings
    pub animations: AnimationConfig,
}

impl Default for ContextComponentConfig {
    fn default() -> Self {
        Self {
            max_display_items: 50,
            show_metadata: true,
            enable_selection: true,
            show_relevance: true,
            enable_editing: true,
            theme: ContextTheme::default(),
            animations: AnimationConfig::default(),
        }
    }
}

/// Theme configuration for context component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTheme {
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
}

impl Default for ContextTheme {
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
        }
    }
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    pub enabled: bool,
    pub duration: std::time::Duration,
    pub easing: AnimationEasing,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration: std::time::Duration::from_millis(200),
            easing: AnimationEasing::EaseOut,
        }
    }
}

/// Animation easing types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnimationEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// State for context management component
#[derive(Debug, Clone)]
pub struct ContextComponentState {
    /// List of context items
    items: Vec<Arc<ContextItem>>,
    /// Selected item IDs
    selected_items: Vec<String>,
    /// Search query for filtering items
    search_query: String,
    /// Currently active filter
    active_filter: Option<ContextFilter>,
    /// Sort mode
    sort_mode: ContextSortMode,
    /// Editing state
    editing_item: Option<String>, // ID of item being edited
    /// Drag and drop state
    drag_state: Option<DragState>,
    /// UI state
    show_details: bool,
    /// Mount state
    is_mounted: bool,
    /// Performance metrics
    last_update: std::time::Instant,
}

/// Events that can be emitted by the context component
#[derive(Debug, Clone, PartialEq)]
pub enum ContextComponentEvent {
    /// Item was selected
    ItemSelected(String),
    /// Item was deselected
    ItemDeselected(String),
    /// Item was added
    ItemAdded(Arc<ContextItem>),
    /// Item was removed
    ItemRemoved(String),
    /// Item was updated
    ItemUpdated(Arc<ContextItem>),
    /// Selection was cleared
    SelectionCleared,
    /// Filter changed
    FilterChanged(ContextFilter),
    /// Sort mode changed
    SortModeChanged(ContextSortMode),
}

/// Filter options for context items
#[derive(Debug, Clone, PartialEq)]
pub enum ContextFilter {
    ByType(ContextType),
    ByRelevance(f32, f32), // min, max
    ByDateRange(DateTime<Utc>, DateTime<Utc>),
    Selected,
    WithMetadata,
}

/// Sort modes for context items
#[derive(Debug, Clone, PartialEq)]
pub enum ContextSortMode {
    ByDate(bool), // true for newest first
    ByRelevance(bool), // true for highest first
    ByTitle(bool), // true for A-Z
    ByType,
}

/// Drag and drop state
#[derive(Debug, Clone)]
pub struct DragState {
    pub dragged_item_id: String,
    pub start_position: Point<Pixels>,
    pub current_position: Point<Pixels>,
}

/// Main context management component
pub struct ContextComponent {
    config: ContextComponentConfig,
    state: ContextComponentState,
    performance_tracker: PerformanceTracker,
    id: String,
}

impl ContextComponent {
    /// Create a new context component
    pub fn new(config: ContextComponentConfig) -> Self {
        let id = Ulid::new().to_string().to_string();

        Self {
            config,
            state: ContextComponentState {
                items: Vec::new(),
                selected_items: Vec::new(),
                search_query: String::new(),
                active_filter: None,
                sort_mode: ContextSortMode::ByDate(true),
                editing_item: None,
                drag_state: None,
                show_details: false,
                is_mounted: false,
                last_update: std::time::Instant::now(),
            },
            performance_tracker: PerformanceTracker::new(id.clone()),
            id,
        }
    }

    /// Add a context item
    pub fn add_item(&mut self, item: ContextItem) -> Result<(), String> {
        let item_arc = Arc::new(item);
        self.state.items.push(item_arc);
        self.state.last_update = std::time::Instant::now();

        log::info!("Added context item to component");
        Ok(())
    }

    /// Remove a context item
    pub fn remove_item(&mut self, item_id: &str) -> Result<(), String> {
        let initial_len = self.state.items.len();
        self.state.items.retain(|item| item.id.as_str() != item_id);

        if self.state.items.len() == initial_len {
            return Err(format!("Context item with ID '{}' not found", item_id));
        }

        // Also remove from selected
        self.state.selected_items.retain(|id| id != item_id);
        self.state.last_update = std::time::Instant::now();

        log::info!("Removed context item from component");
        Ok(())
    }

    /// Update a context item
    pub fn update_item(&mut self, item_id: &str, new_item: ContextItem) -> Result<(), String> {
        let index = self.state.items
            .iter()
            .position(|item| item.id.as_str() == item_id)
            .ok_or_else(|| format!("Context item with ID '{}' not found", item_id))?;

        self.state.items[index] = Arc::new(new_item);
        self.state.last_update = std::time::Instant::now();

        log::info!("Updated context item in component");
        Ok(())
    }

    /// Get all items
    pub fn get_items(&self) -> &[Arc<ContextItem>] {
        &self.state.items
    }

    /// Get selected items
    pub fn get_selected_items(&self) -> Vec<&Arc<ContextItem>> {
        self.state.items
            .iter()
            .filter(|item| self.state.selected_items.contains(&item.id.to_string()))
            .collect()
    }

    /// Toggle selection of an item
    pub fn toggle_selection(&mut self, item_id: &str) {
        if let Some(pos) = self.state.selected_items.iter().position(|id| id == item_id) {
            self.state.selected_items.remove(pos);
        } else {
            self.state.selected_items.push(item_id.to_string());
        }
        self.state.last_update = std::time::Instant::now();
    }

    /// Select all items
    pub fn select_all(&mut self) {
        self.state.selected_items = self.state.items
            .iter()
            .map(|item| item.id.to_string())
            .collect();
        self.state.last_update = std::time::Instant::now();
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.state.selected_items.clear();
        self.state.last_update = std::time::Instant::now();
    }

    /// Search/filter items
    pub fn set_search_query(&mut self, query: String) {
        self.state.search_query = query;
        self.state.last_update = std::time::Instant::now();
    }

    /// Set active filter
    pub fn set_filter(&mut self, filter: Option<ContextFilter>) {
        self.state.active_filter = filter;
        self.state.last_update = std::time::Instant::now();
    }

    /// Set sort mode
    pub fn set_sort_mode(&mut self, mode: ContextSortMode) {
        self.state.sort_mode = mode;
        self.state.last_update = std::time::Instant::now();
    }

    /// Toggle details view
    pub fn toggle_details(&mut self) {
        self.state.show_details = !self.state.show_details;
        self.state.last_update = std::time::Instant::now();
    }

    /// Get filtered and sorted items
    pub fn get_filtered_items(&self) -> Vec<&Arc<ContextItem>> {
        let mut items: Vec<&Arc<ContextItem>> = self.state.items.iter().collect();

        // Apply search filter
        if !self.state.search_query.is_empty() {
            let query_lower = self.state.search_query.to_lowercase();
            items.retain(|item| {
                item.title.to_lowercase().contains(&query_lower)
                    || item.content.to_lowercase().contains(&query_lower)
                    || item.summary.as_ref().map_or(false, |s| s.to_lowercase().contains(&query_lower))
            });
        }

        // Apply active filter
        if let Some(filter) = &self.state.active_filter {
            match filter {
                ContextFilter::ByType(context_type) => {
                    items.retain(|item| item.context_type == *context_type);
                }
                ContextFilter::ByRelevance(min, max) => {
                    items.retain(|item| {
                        item.relevance_score
                            .map(|score| score >= *min as f64 && score <= *max as f64)
                            .unwrap_or(false)
                    });
                }
                ContextFilter::ByDateRange(start, end) => {
                    items.retain(|item| item.created_at >= *start && item.created_at <= *end);
                }
                ContextFilter::Selected => {
                    items.retain(|item| self.state.selected_items.contains(&item.id.to_string()));
                }
                ContextFilter::WithMetadata => {
                    items.retain(|item| !item.metadata.is_empty());
                }
            }
        }

        // Apply sorting
        match self.state.sort_mode {
            ContextSortMode::ByDate(newest_first) => {
                items.sort_by(|a, b| {
                    if newest_first {
                        b.created_at.cmp(&a.created_at)
                    } else {
                        a.created_at.cmp(&b.created_at)
                    }
                });
            }
            ContextSortMode::ByRelevance(highest_first) => {
                items.sort_by(|a, b| {
                    let score_a = a.relevance_score.unwrap_or(0.0);
                    let score_b = b.relevance_score.unwrap_or(0.0);
                    if highest_first {
                        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                    }
                });
            }
            ContextSortMode::ByTitle(az) => {
                items.sort_by(|a, b| {
                    if az {
                        a.title.cmp(&b.title)
                    } else {
                        b.title.cmp(&a.title)
                    }
                });
            }
            ContextSortMode::ByType => {
                items.sort_by(|a, b| {
                    format!("{:?}", a.context_type).cmp(&format!("{:?}", b.context_type))
                });
            }
        }

        // Apply max display limit
        if items.len() > self.config.max_display_items {
            items.truncate(self.config.max_display_items);
        }

        items
    }

    /// Get statistics
    pub fn get_stats(&self) -> ContextStats {
        ContextStats {
            total_items: self.state.items.len(),
            selected_items: self.state.selected_items.len(),
            filtered_items: self.get_filtered_items().len(),
            by_type: self.state.items.iter().fold(
                std::collections::HashMap::new(),
                |mut acc, item| {
                    let type_str = format!("{:?}", item.context_type);
                    *acc.entry(type_str).or_insert(0) += 1;
                    acc
                }
            ),
            total_relevance: self.state.items.iter()
                .map(|item| item.relevance_score.unwrap_or(0.0))
                .sum(),
        }
    }
}

/// Context statistics
#[derive(Debug, Clone)]
pub struct ContextStats {
    pub total_items: usize,
    pub selected_items: usize,
    pub filtered_items: usize,
    pub by_type: std::collections::HashMap<String, usize>,
    pub total_relevance: f64,
}

impl ContextStats {
    pub fn avg_relevance(&self) -> f64 {
        if self.total_items > 0 {
            self.total_relevance / self.total_items as f64
        } else {
            0.0
        }
    }
}

