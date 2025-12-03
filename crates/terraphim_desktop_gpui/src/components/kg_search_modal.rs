/// Enhanced Knowledge Graph Search Modal with ReusableComponent Patterns
///
/// This module provides a reusable knowledge graph search modal built on the
/// ReusableComponent trait foundation, integrating with the KnowledgeGraphComponent
/// while adding standardized lifecycle management, configuration, and performance monitoring.

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{IconName, StyledExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use ulid::Ulid;

use crate::components::{
    ComponentConfig, ComponentError, LifecycleEvent, PerformanceTracker,
    ReusableComponent, ServiceRegistry, ViewContext, KnowledgeGraphComponent,
    KnowledgeGraphConfig, KnowledgeGraphState, KnowledgeGraphEvent,
    KGSearchResult, KGSortStrategy, KGSearchMode, KGUIConfig,
    KGPerformanceAlert, KGAlertSeverity, ComponentMetadata
};
use crate::kg_search::{KGSearchService, KGTerm};
use terraphim_types::{RoleName, Document};

/// Enhanced Knowledge Graph Search Modal Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGSearchModalConfig {
    /// Role name for KG isolation
    pub role: RoleName,

    /// Modal title
    pub title: String,

    /// Input placeholder text
    pub placeholder: String,

    /// Maximum number of suggestions to show
    pub max_suggestions: usize,

    /// Enable keyboard navigation
    pub enable_keyboard_navigation: bool,

    /// Enable result preview
    pub enable_result_preview: bool,

    /// Auto-select first result
    pub auto_select_first: bool,

    /// Show advanced search options
    pub show_advanced_options: bool,

    /// Debounce time for search (milliseconds)
    pub search_debounce_ms: u64,

    /// Animation configuration
    pub animation_config: KGModalAnimationConfig,

    /// Integration with KG component
    pub kg_component_config: KnowledgeGraphConfig,
}

impl Default for KGSearchModalConfig {
    fn default() -> Self {
        Self {
            role: RoleName::from("default"),
            title: "Knowledge Graph Search".to_string(),
            placeholder: "Search knowledge graph terms...".to_string(),
            max_suggestions: 10,
            enable_keyboard_navigation: true,
            enable_result_preview: true,
            auto_select_first: false,
            show_advanced_options: false,
            search_debounce_ms: 300,
            animation_config: KGModalAnimationConfig::default(),
            kg_component_config: KnowledgeGraphConfig::default(),
        }
    }
}

/// Knowledge Graph Modal Animation Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGModalAnimationConfig {
    /// Enable animations
    pub enabled: bool,

    /// Fade in duration (milliseconds)
    pub fade_in_duration_ms: u64,

    /// Slide in duration (milliseconds)
    pub slide_in_duration_ms: u64,

    /// Result highlight duration (milliseconds)
    pub highlight_duration_ms: u64,

    /// Enable result animations
    pub enable_result_animations: bool,
}

impl Default for KGModalAnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fade_in_duration_ms: 200,
            slide_in_duration_ms: 150,
            highlight_duration_ms: 1000,
            enable_result_animations: true,
        }
    }
}

/// Knowledge Graph Search Modal State
#[derive(Debug, Clone)]
pub struct KGSearchModalState {
    /// Current input query
    pub query: String,

    /// Current search results
    pub results: Vec<KGSearchResult>,

    /// Currently selected result index
    pub selected_index: Option<usize>,

    /// Search mode
    pub search_mode: KGSearchMode,

    /// Modal visibility state
    pub is_visible: bool,

    /// Search loading state
    pub is_searching: bool,

    /// Search error state
    pub search_error: Option<String>,

    /// Performance metrics
    pub performance_metrics: KGModalPerformanceMetrics,

    /// UI interaction state
    pub ui_state: KGModalUIState,

    /// Component lifecycle status
    pub is_initialized: bool,

    /// Last event timestamp
    pub last_event_time: Option<Instant>,
}

impl Default for KGSearchModalState {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected_index: None,
            search_mode: KGSearchMode::Standard,
            is_visible: false,
            is_searching: false,
            search_error: None,
            performance_metrics: KGModalPerformanceMetrics::default(),
            ui_state: KGModalUIState::default(),
            is_initialized: false,
            last_event_time: None,
        }
    }
}

/// Knowledge Graph Modal UI State
#[derive(Debug, Clone)]
pub struct KGModalUIState {
    /// Input focus state
    pub input_focused: bool,

    /// Results panel scroll position
    pub scroll_position: f32,

    /// Show advanced search panel
    pub show_advanced_panel: bool,

    /// Current animation state
    pub animation_state: KGModalAnimationState,

    /// Highlighted result index for animations
    pub highlighted_index: Option<usize>,

    /// Keyboard navigation state
    pub keyboard_nav_state: KGModalKeyboardState,
}

impl Default for KGModalUIState {
    fn default() -> Self {
        Self {
            input_focused: false,
            scroll_position: 0.0,
            show_advanced_panel: false,
            animation_state: KGModalAnimationState::default(),
            highlighted_index: None,
            keyboard_nav_state: KGModalKeyboardState::default(),
        }
    }
}

/// Knowledge Graph Modal Animation State
#[derive(Debug, Clone)]
pub struct KGModalAnimationState {
    /// Current animation phase
    pub phase: KGModalAnimationPhase,

    /// Animation progress (0.0-1.0)
    pub progress: f32,

    /// Animation start time
    pub start_time: Option<Instant>,

    /// Target animation duration
    pub target_duration: Duration,
}

impl Default for KGModalAnimationState {
    fn default() -> Self {
        Self {
            phase: KGModalAnimationPhase::Idle,
            progress: 0.0,
            start_time: None,
            target_duration: Duration::from_millis(200),
        }
    }
}

/// Knowledge Graph Modal Animation Phase
#[derive(Debug, Clone, PartialEq)]
pub enum KGModalAnimationPhase {
    /// No animation active
    Idle,
    /// Modal fade in
    FadeIn,
    /// Modal slide in
    SlideIn,
    /// Result highlight
    HighlightResult,
    /// Result selection animation
    SelectResult,
}

/// Knowledge Graph Modal Keyboard Navigation State
#[derive(Debug, Clone)]
pub struct KGModalKeyboardState {
    /// Current modifier keys
    pub modifiers: KGModalModifiers,

    /// Last key press timestamp
    pub last_key_time: Option<Instant>,

    /// Key repeat state
    pub key_repeat_count: u32,

    /// Navigation direction
    pub navigation_direction: Option<KGModalNavigationDirection>,
}

impl Default for KGModalKeyboardState {
    fn default() -> Self {
        Self {
            modifiers: KGModalModifiers::default(),
            last_key_time: None,
            key_repeat_count: 0,
            navigation_direction: None,
        }
    }
}

/// Keyboard modifier keys state
#[derive(Debug, Clone, Default)]
pub struct KGModalModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Navigation direction for keyboard
#[derive(Debug, Clone, PartialEq)]
pub enum KGModalNavigationDirection {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
}

/// Knowledge Graph Modal Performance Metrics
#[derive(Debug, Clone, Default)]
pub struct KGModalPerformanceMetrics {
    /// Total searches performed
    pub total_searches: u64,

    /// Average search latency in milliseconds
    pub avg_search_latency_ms: f64,

    /// Peak search latency in milliseconds
    pub peak_search_latency_ms: u64,

    /// Average render time in milliseconds
    pub avg_render_time_ms: f64,

    /// Number of UI interactions
    pub ui_interactions: u64,

    /// Number of keyboard navigations
    pub keyboard_navigations: u64,

    /// Modal open/close count
    pub modal_toggles: u64,

    /// Performance alerts
    pub performance_alerts: Vec<KGModalPerformanceAlert>,
}

/// Knowledge Graph Modal Performance Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGModalPerformanceAlert {
    /// Alert identifier
    pub id: String,

    /// Alert type
    pub alert_type: KGModalAlertType,

    /// Alert message
    pub message: String,

    /// Alert severity
    pub severity: KGAlertSeverity,

    /// Alert timestamp
    pub timestamp: std::time::SystemTime,

    /// Associated metric value
    pub metric_value: f64,
}

/// Knowledge Graph Modal Alert Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGModalAlertType {
    /// Search latency too high
    HighSearchLatency,
    /// Render time too high
    HighRenderTime,
    /// UI lag detected
    UILag,
    /// Memory usage too high
    HighMemoryUsage,
    /// Modal performance degradation
    ModalPerformanceDegradation,
}

/// Knowledge Graph Search Modal Events
#[derive(Debug, Clone, PartialEq)]
pub enum KGSearchModalEvent {
    /// Modal opened
    ModalOpened,

    /// Modal closed
    ModalClosed,

    /// Search query changed
    QueryChanged { query: String },

    /// Search completed
    SearchCompleted { results: Vec<KGSearchResult> },

    /// Result selected
    ResultSelected { result: KGSearchResult, index: usize },

    /// Keyboard navigation
    KeyboardNavigation { direction: KGModalNavigationDirection, index: usize },

    /// UI interaction
    UIInteraction { interaction_type: KGModalUIInteractionType },

    /// Performance alert
    PerformanceAlert { alert: KGModalPerformanceAlert },

    /// Configuration updated
    ConfigurationUpdated,

    /// Component lifecycle event
    LifecycleEvent { event: LifecycleEvent },
}

/// UI Interaction Types
#[derive(Debug, Clone, PartialEq)]
pub enum KGModalUIInteractionType {
    /// Input focus gained
    InputFocusGained,

    /// Input focus lost
    InputFocusLost,

    /// Result clicked
    ResultClicked,

    /// Result hovered
    ResultHovered,

    /// Scroll action
    ScrollAction,

    /// Advanced panel toggled
    AdvancedPanelToggled,

    /// Keyboard shortcut used
    KeyboardShortcut { shortcut: String },
}

/// Enhanced Reusable Knowledge Graph Search Modal
#[derive(Debug)]
pub struct KGSearchModal {
    /// Component configuration
    config: KGSearchModalConfig,

    /// Component state
    state: KGSearchModalState,

    /// Performance tracker
    performance_tracker: PerformanceTracker,

    /// Knowledge Graph component integration
    kg_component: Arc<RwLock<KnowledgeGraphComponent>>,

    /// Input state entity
    input_state: Entity<InputState>,

    /// Event sender for component events
    event_sender: mpsc::UnboundedSender<KGSearchModalEvent>,

    /// Event receiver for component events
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<KGSearchModalEvent>>>>,

    /// Conversation ID for context
    conversation_id: Option<String>,

    /// Subscription handles
    _subscriptions: Vec<Subscription>,

    /// Component metadata
    metadata: ComponentMetadata,

    /// Search debounce timer
    debounce_timer: Option<Instant>,
}

impl KGSearchModal {
    /// Create a new enhanced KG search modal
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        config: KGSearchModalConfig,
        kg_search_service: Arc<KGSearchService>,
        conversation_id: Option<String>,
    ) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(&config.placeholder)
                .with_icon(IconName::Search)
        });

        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        // Create KG component with integrated configuration
        let kg_component_config = config.kg_component_config.clone();
        let kg_component = Arc::new(RwLock::new(
            // Placeholder initialization - would need proper KG component initialization
            KnowledgeGraphComponent::new(kg_component_config, kg_search_service)
        ));

        let mut modal = Self {
            config: config.clone(),
            state: KGSearchModalState::default(),
            performance_tracker: PerformanceTracker::new("kg-search-modal"),
            kg_component,
            input_state,
            event_sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
            conversation_id,
            _subscriptions: Vec::new(),
            metadata: Self::create_metadata(),
            debounce_timer: None,
        };

        // Set up subscriptions and event handlers
        modal.setup_event_handlers(window, cx);

        modal
    }

    /// Create component metadata
    fn create_metadata() -> ComponentMetadata {
        ComponentMetadata::new(
            "kg-search-modal".to_string(),
            "1.0.0".to_string(),
            "Knowledge Graph Search Modal".to_string(),
            "Reusable modal component for knowledge graph search with advanced UI features".to_string(),
            "Terraphim AI Team".to_string(),
        )
        .with_capability(crate::components::ComponentCapability::Searchable)
        .with_capability(crate::components::ComponentCapability::Filterable)
        .with_capability(crate::components::ComponentCapability::KeyboardNavigable)
        .with_capability(crate::components::ComponentCapability::Accessible)
        .with_capability(crate::components::ComponentCapability::Animated)
    }

    /// Set up event handlers and subscriptions
    fn setup_event_handlers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input_state_clone = self.input_state.clone();
        let config = self.config.clone();

        // Subscribe to input changes
        let input_sub = cx.subscribe_in(&self.input_state, window, move |this, _, ev: &InputEvent, window, cx| {
            match ev {
                InputEvent::Change => {
                    let query = input_state_clone.read(cx).value();
                    this.handle_input_change(query, cx);
                }
                InputEvent::Focus => {
                    this.handle_input_focus(true, cx);
                }
                InputEvent::Blur => {
                    this.handle_input_focus(false, cx);
                }
                InputEvent::KeyDown { event } => {
                    this.handle_key_down(event, cx);
                }
                _ => {}
            }
        });

        self._subscriptions.push(input_sub);
    }

    /// Handle input change with debouncing
    fn handle_input_change(&mut self, query: String, cx: &mut Context<Self>) {
        let now = Instant::now();

        // Update state
        self.state.query = query.clone();
        self.state.last_event_time = Some(now);
        self.state.ui_state.input_focused = true;

        // Check if we should debounce
        if let Some(last_time) = self.debounce_timer {
            if now.duration_since(last_time) < Duration::from_millis(self.config.search_debounce_ms) {
                return; // Skip this change, still in debounce period
            }
        }

        self.debounce_timer = Some(now);

        // Send query changed event
        let _ = self.event_sender.send(KGSearchModalEvent::QueryChanged { query: query.clone() });

        // Clear results and search if query has sufficient length
        if query.trim().len() >= 2 {
            self.perform_search(query, cx);
        } else {
            self.clear_results(cx);
        }
    }

    /// Handle input focus changes
    fn handle_input_focus(&mut self, focused: bool, cx: &mut Context<Self>) {
        self.state.ui_state.input_focused = focused;
        self.state.last_event_time = Some(Instant::now());

        let interaction_type = if focused {
            KGModalUIInteractionType::InputFocusGained
        } else {
            KGModalUIInteractionType::InputFocusLost
        };

        let _ = self.event_sender.send(KGSearchModalEvent::UIInteraction { interaction_type });

        cx.notify();
    }

    /// Handle keyboard events
    fn handle_key_down(&mut self, event: &gpui::Keystroke, cx: &mut Context<Self>) {
        if !self.config.enable_keyboard_navigation {
            return;
        }

        let navigation_direction = match event.key.as_str() {
            "up" => Some(KGModalNavigationDirection::Up),
            "down" => Some(KGModalNavigationDirection::Down),
            "pageup" => Some(KGModalNavigationDirection::PageUp),
            "pagedown" => Some(KGModalNavigationDirection::PageDown),
            "home" => Some(KGModalNavigationDirection::Home),
            "end" => Some(KGModalNavigationDirection::End),
            "enter" => {
                self.handle_selection(cx);
                return;
            }
            "escape" => {
                self.close_modal(cx);
                return;
            }
            _ => None,
        };

        if let Some(direction) = navigation_direction {
            self.handle_keyboard_navigation(direction, cx);
        }

        // Update keyboard state
        self.state.ui_state.keyboard_nav_state.modifiers.ctrl = event.modifiers.control;
        self.state.ui_state.keyboard_nav_state.modifiers.shift = event.modifiers.shift;
        self.state.ui_state.keyboard_nav_state.modifiers.alt = event.modifiers.alt;
        self.state.ui_state.keyboard_nav_state.last_key_time = Some(Instant::now());
    }

    /// Handle keyboard navigation
    fn handle_keyboard_navigation(&mut self, direction: KGModalNavigationDirection, cx: &mut Context<Self>) {
        if self.state.results.is_empty() {
            return;
        }

        let current_index = self.state.selected_index.unwrap_or(0);
        let new_index = match direction {
            KGModalNavigationDirection::Up => {
                if current_index == 0 {
                    self.state.results.len().saturating_sub(1)
                } else {
                    current_index - 1
                }
            }
            KGModalNavigationDirection::Down => {
                if current_index >= self.state.results.len().saturating_sub(1) {
                    0
                } else {
                    current_index + 1
                }
            }
            KGModalNavigationDirection::PageUp => {
                current_index.saturating_sub(5)
            }
            KGModalNavigationDirection::PageDown => {
                (current_index + 5).min(self.state.results.len().saturating_sub(1))
            }
            KGModalNavigationDirection::Home => 0,
            KGModalNavigationDirection::End => self.state.results.len().saturating_sub(1),
            _ => current_index,
        };

        self.state.selected_index = Some(new_index);
        self.state.ui_state.keyboard_nav_state.navigation_direction = Some(direction.clone());
        self.state.ui_state.keyboard_nav_state.keyboard_navigations += 1;

        // Trigger animation for result highlight
        if self.config.animation_config.enable_result_animations {
            self.state.ui_state.highlighted_index = Some(new_index);
            self.start_highlight_animation();
        }

        // Send navigation event
        let _ = self.event_sender.send(KGSearchModalEvent::KeyboardNavigation {
            direction,
            index: new_index,
        });

        let _ = self.event_sender.send(KGSearchModalEvent::UIInteraction {
            interaction_type: KGModalUIInteractionType::KeyboardShortcut {
                shortcut: format!("nav-{:?}", direction),
            },
        });

        cx.notify();
    }

    /// Handle result selection
    fn handle_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.state.selected_index {
            if let Some(result) = self.state.results.get(index) {
                // Send selection event
                let _ = self.event_sender.send(KGSearchModalEvent::ResultSelected {
                    result: result.clone(),
                    index,
                });

                let _ = self.event_sender.send(KGSearchModalEvent::UIInteraction {
                    interaction_type: KGModalUIInteractionType::ResultClicked,
                });

                // Close modal after selection
                self.close_modal(cx);
            }
        }
    }

    /// Perform search using KG component
    fn perform_search(&mut self, query: String, cx: &mut Context<Self>) {
        let search_start = Instant::now();

        // Update search state
        self.state.is_searching = true;
        self.state.search_error = None;
        self.state.ui_state.scroll_position = 0.0;

        cx.notify();

        // Perform async search using KG component
        let kg_component = Arc::clone(&self.kg_component);
        let event_sender = self.event_sender.clone();

        cx.spawn(async move |this, cx| {
            let mut kg_component = kg_component.write().await;

            match kg_component.search(query).await {
                Ok(results) => {
                    let search_duration = search_start.elapsed();

                    // Update modal state with results
                    this.update(cx, |this, cx| {
                        this.state.is_searching = false;
                        this.state.results = results.clone();

                        // Auto-select first result if configured
                        if this.config.auto_select_first && !results.is_empty() {
                            this.state.selected_index = Some(0);
                        } else {
                            this.state.selected_index = None;
                        }

                        // Update performance metrics
                        this.update_search_metrics(search_duration);

                        // Send completion event
                        let _ = event_sender.send(KGSearchModalEvent::SearchCompleted { results });
                    }).ok();
                }
                Err(e) => {
                    let error_msg = format!("Search failed: {}", e);

                    this.update(cx, |this, cx| {
                        this.state.is_searching = false;
                        this.state.search_error = Some(error_msg.clone());
                        this.state.results.clear();
                        this.state.selected_index = None;
                    }).ok();

                    let _ = event_sender.send(KGSearchModalEvent::UIInteraction {
                        interaction_type: KGModalUIInteractionType::KeyboardShortcut {
                            shortcut: "search-error".to_string(),
                        },
                    });
                }
            }
        }).detach();
    }

    /// Clear search results
    fn clear_results(&mut self, cx: &mut Context<Self>) {
        self.state.results.clear();
        self.state.selected_index = None;
        self.state.is_searching = false;
        self.state.search_error = None;
        self.state.ui_state.scroll_position = 0.0;
        self.state.ui_state.highlighted_index = None;

        cx.notify();
    }

    /// Update search performance metrics
    fn update_search_metrics(&mut self, search_duration: Duration) {
        self.state.performance_metrics.total_searches += 1;

        let duration_ms = search_duration.as_millis() as f64;
        let total_searches = self.state.performance_metrics.total_searches as f64;

        // Update average latency
        self.state.performance_metrics.avg_search_latency_ms =
            (self.state.performance_metrics.avg_search_latency_ms * (total_searches - 1.0) + duration_ms) / total_searches;

        // Update peak latency
        self.state.performance_metrics.peak_search_latency_ms =
            self.state.performance_metrics.peak_search_latency_ms.max(duration_ms as u64);

        // Check for performance alerts
        if duration_ms > 1000.0 { // 1 second threshold
            self.generate_modal_alert(
                crate::components::knowledge_graph::KGAlertType::HighSearchLatency,
                "Search latency exceeded threshold".to_string(),
                KGAlertSeverity::Warning,
                duration_ms,
                1000.0,
            );
        }
    }

    /// Generate modal performance alert
    fn generate_modal_alert(
        &mut self,
        alert_type: crate::components::knowledge_graph::KGAlertType,
        message: String,
        severity: KGAlertSeverity,
        metric_value: f64,
        threshold: f64,
    ) {
        let alert = KGModalPerformanceAlert {
            id: Ulid::new().to_string().to_string(),
            alert_type: match alert_type {
                crate::components::knowledge_graph::KGAlertType::HighSearchLatency => KGModalAlertType::HighSearchLatency,
                crate::components::knowledge_graph::KGAlertType::SlowIndexRebuild => KGModalAlertType::ModalPerformanceDegradation,
                crate::components::knowledge_graph::KGAlertType::HighMemoryUsage => KGModalAlertType::HighMemoryUsage,
                _ => KGModalAlertType::HighSearchLatency,
            },
            message,
            severity,
            timestamp: std::time::SystemTime::now(),
            metric_value,
        };

        self.state.performance_metrics.performance_alerts.push(alert.clone());

        // Send alert event
        let _ = self.event_sender.send(KGSearchModalEvent::PerformanceAlert { alert });
    }

    /// Start highlight animation for selected result
    fn start_highlight_animation(&mut self) {
        if !self.config.animation_config.enabled {
            return;
        }

        self.state.ui_state.animation_state.phase = KGModalAnimationPhase::HighlightResult;
        self.state.ui_state.animation_state.start_time = Some(Instant::now());
        self.state.ui_state.animation_state.target_duration =
            Duration::from_millis(self.config.animation_config.highlight_duration_ms);
        self.state.ui_state.animation_state.progress = 0.0;
    }

    /// Open the modal
    pub fn open(&mut self, cx: &mut Context<Self>) {
        self.state.is_visible = true;
        self.state.performance_metrics.modal_toggles += 1;
        self.state.last_event_time = Some(Instant::now());

        // Start fade in animation
        if self.config.animation_config.enabled {
            self.state.ui_state.animation_state.phase = KGModalAnimationPhase::FadeIn;
            self.state.ui_state.animation_state.start_time = Some(Instant::now());
            self.state.ui_state.animation_state.target_duration =
                Duration::from_millis(self.config.animation_config.fade_in_duration_ms);
            self.state.ui_state.animation_state.progress = 0.0;
        }

        // Focus input
        self.input_state.update(cx, |input, cx| {
            input.focus(cx);
        });

        let _ = self.event_sender.send(KGSearchModalEvent::ModalOpened);
        cx.notify();
    }

    /// Close the modal
    pub fn close_modal(&mut self, cx: &mut Context<Self>) {
        self.state.is_visible = false;
        self.state.last_event_time = Some(Instant::now());

        // Clear search state
        self.clear_results(cx);

        let _ = self.event_sender.send(KGSearchModalEvent::ModalClosed);
        cx.notify();
    }

    /// Toggle modal visibility
    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.state.is_visible {
            self.close_modal(cx);
        } else {
            self.open(cx);
        }
    }

    /// Update modal configuration
    pub fn update_config(&mut self, config: KGSearchModalConfig, cx: &mut Context<Self>) {
        self.config = config.clone();

        // Update input placeholder
        self.input_state.update(cx, |input, cx| {
            input.set_placeholder(&config.placeholder);
        });

        let _ = self.event_sender.send(KGSearchModalEvent::ConfigurationUpdated);
        cx.notify();
    }

    /// Get component events
    pub async fn get_events(&mut self) -> Vec<KGSearchModalEvent> {
        let mut receiver = self.event_receiver.write().await;
        if let Some(ref mut rx) = *receiver {
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            events
        } else {
            Vec::new()
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &KGSearchModalConfig {
        &self.config
    }

    /// Get current state
    pub fn state(&self) -> &KGSearchModalState {
        &self.state
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> &KGModalPerformanceMetrics {
        &self.state.performance_metrics
    }

    /// Get conversation ID
    pub fn conversation_id(&self) -> Option<&String> {
        self.conversation_id.as_ref()
    }

    /// Check if modal is visible
    pub fn is_visible(&self) -> bool {
        self.state.is_visible
    }

    /// Get selected result
    pub fn get_selected_result(&self) -> Option<&KGSearchResult> {
        self.state.selected_index.and_then(|index| self.state.results.get(index))
    }

    /// Set search mode
    pub fn set_search_mode(&mut self, mode: KGSearchMode, cx: &mut Context<Self>) {
        self.state.search_mode = mode.clone();

        // Re-perform search with new mode
        if !self.state.query.is_empty() {
            self.perform_search(self.state.query.clone(), cx);
        }
    }

    /// Clear modal state
    pub fn clear_state(&mut self, cx: &mut Context<Self>) {
        self.state = KGSearchModalState::default();
        self.debounce_timer = None;

        // Clear input
        self.input_state.update(cx, |input, cx| {
            input.clear(cx);
        });

        cx.notify();
    }
}

impl Render for KGSearchModal {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        let render_start = Instant::now();

        let modal_content = if self.state.is_visible {
            self.render_modal_content(window, cx)
        } else {
            div().id("kg-search-modal-hidden")
        };

        // Update render metrics
        let render_duration = render_start.elapsed();
        let total_renders = self.state.performance_metrics.ui_interactions + 1;
        self.state.performance_metrics.avg_render_time_ms =
            (self.state.performance_metrics.avg_render_time_ms * (total_renders as f64 - 1.0) + render_duration.as_millis() as f64) / total_renders as f64;
        self.state.performance_metrics.ui_interactions = total_renders;

        modal_content
    }
}

impl KGSearchModal {
    /// Render modal content
    fn render_modal_content(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        let animation_progress = if self.state.ui_state.animation_state.phase != KGModalAnimationPhase::Idle {
            if let Some(start_time) = self.state.ui_state.animation_state.start_time {
                let elapsed = start_time.elapsed();
                let progress = (elapsed.as_millis() as f64 / self.state.ui_state.animation_state.target_duration.as_millis() as f64).min(1.0);
                progress as f32
            } else {
                0.0
            }
        } else {
            1.0
        };

        div()
            .id("kg-search-modal")
            .when(self.state.ui_state.animation_state.phase == KGModalAnimationPhase::FadeIn, |div| {
                div.opacity(animation_progress)
            })
            .size_full()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe0e0e0))
            .rounded_lg()
            .shadow_lg()
            .overflow_hidden()
            .child(
                // Header
                div()
                    .id("kg-modal-header")
                    .bg(rgb(0xf8f9fa))
                    .border_b_1()
                    .border_color(rgb(0xe0e0e0))
                    .px_4()
                    .py_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(rgb(0x1f2937))
                            .child(&self.config.title)
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(self.config.show_advanced_options, |div| {
                                div.child(
                                    button()
                                        .icon(IconName::Settings)
                                        .size(ButtonSize::Small)
                                        .style(ButtonStyle::Ghost)
                                        .on_click(cx.listener(|this, _event, cx| {
                                            this.state.ui_state.show_advanced_panel = !this.state.ui_state.show_advanced_panel;
                                            cx.notify();
                                        }))
                                )
                            })
                            .child(
                                button()
                                    .icon(IconName::X)
                                    .size(ButtonSize::Small)
                                    .style(ButtonStyle::Ghost)
                                    .on_click(cx.listener(|this, _event, cx| {
                                        this.close_modal(cx);
                                    }))
                            )
                    )
            )
            .child(
                // Search input
                div()
                    .id("kg-modal-search")
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0xe0e0e0))
                    .child(
                        div()
                            .relative()
                            .child(self.input_state.clone())
                            .when(self.state.is_searching, |div| {
                                div()
                                    .absolute()
                                    .right_3()
                                    .top_2()
                                    .child(
                                        div()
                                            .size_4()
                                            .border_2()
                                            .border_color(rgb(0x3b82f6))
                                            .border_t-transparent()
                                            .rounded_full()
                                            .animate_spin()
                                    )
                            })
                    )
            )
            .child(
                // Results area
                div()
                    .id("kg-modal-results")
                    .flex_1()
                    .overflow_y_scroll()
                    .when(self.state.results.is_empty() && !self.state.is_searching, |div| {
                        div.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_32()
                                .text_sm()
                                .text_color(rgb(0x6b7280))
                                .child("No results found. Try searching for knowledge graph terms.")
                        )
                    })
                    .when(self.state.is_searching, |div| {
                        div.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_32()
                                .child(
                                    div()
                                        .size_6()
                                        .border_2()
                                        .border_color(rgb(0x3b82f6))
                                        .border_t_transparent()
                                        .rounded_full()
                                        .animate_spin()
                                )
                        )
                    })
                    .when(!self.state.results.is_empty(), |div| {
                        div.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .children(
                                    self.state.results.iter().enumerate().map(|(index, result)| {
                                        self.render_result(result, index, cx)
                                    })
                                )
                        )
                    })
                    .when(self.state.search_error.is_some(), |div| {
                        div.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_32()
                                .text_sm()
                                .text_color(rgb(0xef4444))
                                .child(self.state.search_error.as_ref().unwrap())
                        )
                    })
            )
    }

    /// Render individual search result
    fn render_result(&self, result: &KGSearchResult, index: usize, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let is_selected = self.state.selected_index == Some(index);
        let is_highlighted = self.state.ui_state.highlighted_index == Some(index);

        div()
            .id(format!("kg-result-{}", index))
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(rgb(0xf3f4f6))
            .when(is_selected, |div| {
                div.bg(rgb(0xeff6ff))
            })
            .when(is_highlighted, |div| {
                div.bg(rgb(0xf0f9ff))
            })
            .cursor_pointer()
            .hover(|style| {
                style.bg(rgb(0xf9fafb))
            })
            .on_click(cx.listener(move |this, _event, cx| {
                this.state.selected_index = Some(index);
                this.handle_selection(cx);
            }))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w_8()
                            .h_8()
                            .bg(rgb(0x3b82f6))
                            .text_white()
                            .text_xs()
                            .font_semibold()
                            .rounded_full()
                            .child(format!("{}", index + 1))
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_semibold()
                                    .text_color(rgb(0x1f2937))
                                    .child(&result.term)
                            )
                            .when(!result.related_terms.is_empty(), |div| {
                                div.child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x6b7280))
                                        .mt_1()
                                        .child(format!("{} connections", result.related_terms.len()))
                                )
                            })
                            .when(self.config.ui_config.show_confidence, |div| {
                                div.child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x9ca3af))
                                        .mt_1()
                                        .child(format!("Confidence: {:.1}%", result.confidence * 100.0))
                                )
                            })
                    )
                    .when(result.relevance_score > 0.0, |div| {
                        div()
                            .text_xs()
                            .font_semibold()
                            .px_2()
                            .py_1()
                            .bg(rgb(0x10b981))
                            .text_white()
                            .rounded()
                            .child(format!("{:.1}%", result.relevance_score * 100.0))
                    })
            )
    }
}
