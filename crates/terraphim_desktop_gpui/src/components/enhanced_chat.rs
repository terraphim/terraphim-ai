use gpui::*;
use std::sync::Arc;
use ulid::Ulid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::components::{ReusableComponent, ComponentConfig, PerformanceTracker, ComponentError, ViewContext, LifecycleEvent, ServiceRegistry};
use crate::components::{ContextComponent, SearchContextBridge};
use terraphim_types::{
    ChatMessage, ConversationId, RoleName, ContextItem
};

/// Streaming chat message for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChatMessage {
    pub message: ChatMessage,
    pub status: MessageStatus,
    pub chunks: Vec<RenderChunk>,
    pub metrics: StreamMetrics,
}

impl StreamingChatMessage {
    pub fn start_streaming(message: ChatMessage) -> Self {
        Self {
            message,
            status: MessageStatus::Streaming,
            chunks: Vec::new(),
            metrics: StreamMetrics::new(),
        }
    }

    pub fn add_chunk(&mut self, chunk: RenderChunk) {
        self.chunks.push(chunk);
        self.metrics.chunks_received += 1;
    }

    pub fn complete(&mut self) {
        self.status = MessageStatus::Completed;
        self.metrics.completed_at = chrono::Utc::now();
    }
}

/// Render chunk for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderChunk {
    pub id: String,
    pub content: String,
    pub chunk_type: ChunkType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Type of chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    Text,
    Code,
    Markdown,
    Context,
}

/// Stream metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetrics {
    pub chunks_received: usize,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub total_bytes: usize,
}

impl StreamMetrics {
    pub fn new() -> Self {
        Self {
            chunks_received: 0,
            started_at: chrono::Utc::now(),
            completed_at: None,
            total_bytes: 0,
        }
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at.map(|completed| completed - self.started_at)
    }
}

/// Message status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending,
    Streaming,
    Completed,
    Failed,
}

/// Configuration for enhanced chat component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedChatConfig {
    /// Maximum number of messages to display
    pub max_messages: usize,
    /// Whether to enable virtual scrolling
    pub enable_virtual_scrolling: bool,
    /// Whether to show context panel
    pub show_context_panel: bool,
    /// Whether to show typing indicators
    pub show_typing_indicators: bool,
    /// Whether to enable message reactions
    pub enable_reactions: bool,
    /// Whether to show message timestamps
    pub show_timestamps: bool,
    /// Streaming configuration
    pub streaming: StreamingConfig,
    /// Message rendering configuration
    pub rendering: MessageRenderingConfig,
    /// Theme colors
    pub theme: EnhancedChatTheme,
}

impl Default for EnhancedChatConfig {
    fn default() -> Self {
        Self {
            max_messages: 1000,
            enable_virtual_scrolling: true,
            show_context_panel: true,
            show_typing_indicators: true,
            enable_reactions: true,
            show_timestamps: true,
            streaming: StreamingConfig::default(),
            rendering: MessageRenderingConfig::default(),
            theme: EnhancedChatTheme::default(),
        }
    }
}

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Chunk size for streaming responses
    pub chunk_size: usize,
    /// Delay between chunks (for demo purposes)
    pub chunk_delay: std::time::Duration,
    /// Maximum stream duration
    pub max_stream_duration: std::time::Duration,
    /// Enable progressive rendering
    pub enable_progressive_rendering: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 100,
            chunk_delay: std::time::Duration::from_millis(50),
            max_stream_duration: std::time::Duration::from_secs(30),
            enable_progressive_rendering: true,
        }
    }
}

/// Message rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRenderingConfig {
    /// Enable markdown rendering
    pub enable_markdown: bool,
    /// Enable syntax highlighting
    pub enable_syntax_highlighting: bool,
    /// Enable code block execution
    pub enable_code_execution: bool,
    /// Maximum message preview length
    pub max_preview_length: usize,
    /// Animation settings
    pub animations: MessageAnimations,
}

impl Default for MessageRenderingConfig {
    fn default() -> Self {
        Self {
            enable_markdown: true,
            enable_syntax_highlighting: true,
            enable_code_execution: false, // Disabled for security
            max_preview_length: 500,
            animations: MessageAnimations::default(),
        }
    }
}

/// Message animation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAnimations {
    pub enabled: bool,
    pub fade_in_duration: std::time::Duration,
    pub typing_animation_duration: std::time::Duration,
    pub chunk_animation_duration: std::time::Duration,
}

impl Default for MessageAnimations {
    fn default() -> Self {
        Self {
            enabled: true,
            fade_in_duration: std::time::Duration::from_millis(300),
            typing_animation_duration: std::time::Duration::from_millis(1000),
            chunk_animation_duration: std::time::Duration::from_millis(100),
        }
    }
}

/// Theme configuration for enhanced chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedChatTheme {
    pub background: gpui::Rgba,
    pub border: gpui::Rgba,
    pub text_primary: gpui::Rgba,
    pub text_secondary: gpui::Rgba,
    pub user_message_bg: gpui::Rgba,
    pub ai_message_bg: gpui::Rgba,
    pub system_message_bg: gpui::Rgba,
    pub accent: gpui::Rgba,
    pub success: gpui::Rgba,
    pub warning: gpui::Rgba,
    pub error: gpui::Rgba,
    pub typing_indicator: gpui::Rgba,
    pub code_bg: gpui::Rgba,
}

impl Default for EnhancedChatTheme {
    fn default() -> Self {
        Self {
            background: gpui::Rgba::from_rgb(0.98, 0.98, 1.0),
            border: gpui::Rgba::from_rgb(0.85, 0.85, 0.85),
            text_primary: gpui::Rgba::from_rgb(0.1, 0.1, 0.1),
            text_secondary: gpui::Rgba::from_rgb(0.5, 0.5, 0.5),
            user_message_bg: gpui::Rgba::from_rgb(0.9, 0.95, 1.0),
            ai_message_bg: gpui::Rgba::from_rgb(1.0, 0.98, 0.95),
            system_message_bg: gpui::Rgba::from_rgb(0.95, 0.95, 0.95),
            accent: gpui::Rgba::from_rgb(0.2, 0.5, 0.8),
            success: gpui::Rgba::from_rgb(0.2, 0.7, 0.2),
            warning: gpui::Rgba::from_rgb(0.8, 0.6, 0.0),
            error: gpui::Rgba::from_rgb(0.8, 0.2, 0.2),
            typing_indicator: gpui::Rgba::from_rgb(0.5, 0.5, 0.5),
            code_bg: gpui::Rgba::from_rgb(0.97, 0.97, 0.97),
        }
    }
}

/// Chat message with enhanced features
#[derive(Debug, Clone)]
pub struct EnhancedChatMessage {
    /// Original chat message
    pub message: ChatMessage,
    /// Streaming state if applicable
    pub streaming_state: Option<StreamingChatMessage>,
    /// Context items that influenced this message
    pub context_items: Vec<Arc<ContextItem>>,
    /// Message reactions
    pub reactions: Vec<MessageReaction>,
    /// Message metadata
    pub metadata: MessageMetadata,
    /// Rendering state
    pub rendering_state: MessageRenderingState,
}

/// Message reaction
#[derive(Debug, Clone)]
pub struct MessageReaction {
    pub emoji: String,
    pub count: usize,
    pub users: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

/// Message metadata
#[derive(Debug, Clone)]
pub struct MessageMetadata {
    pub message_id: String,
    pub conversation_id: ConversationId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub processing_time: Option<std::time::Duration>,
    pub token_count: Option<usize>,
    pub model_used: Option<String>,
    pub confidence_score: Option<f64>,
}

/// Message rendering state
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRenderingState {
    Pending,
    Rendering,
    Rendered,
    Error(String),
}

/// Enhanced chat state
#[derive(Debug, Clone)]
pub struct EnhancedChatState {
    /// Current conversation
    pub conversation_id: Option<ConversationId>,
    pub messages: Vec<EnhancedChatMessage>,
    pub streaming_messages: std::collections::HashMap<ConversationId, Vec<StreamingChatMessage>>,

    /// Context management
    pub context_items: Vec<Arc<ContextItem>>,
    pub context_search_query: String,

    /// UI state
    pub is_typing: bool,
    pub typing_users: Vec<String>,
    pub selected_message: Option<String>,
    pub show_context_panel: bool,
    pub scroll_position: f64,
    /// Mount state
    pub is_mounted: bool,

    /// Performance metrics
    pub performance_metrics: ChatPerformanceMetrics,
    pub last_update: std::time::Instant,

    /// Configuration
    pub current_role: RoleName,
    pub available_models: Vec<String>,
    pub selected_model: Option<String>,
}

/// Chat performance metrics
#[derive(Debug, Clone, Default)]
pub struct ChatPerformanceMetrics {
    pub total_messages: usize,
    pub streaming_messages: usize,
    pub total_processing_time: std::time::Duration,
    pub average_processing_time: std::time::Duration,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub rendering_time: std::time::Duration,
    pub context_hits: usize,
}

/// Enhanced chat component with reusable architecture
pub struct EnhancedChatComponent {
    config: EnhancedChatConfig,
    state: EnhancedChatState,
    performance_tracker: PerformanceTracker,
    id: String,

    // Child components
    context_component: ContextComponent,
    search_context_bridge: SearchContextBridge,

    // Event emitters
    event_emitter: Option<Box<dyn gpui::EventEmitter<EnhancedChatEvent>>>,
}

/// Events emitted by EnhancedChatComponent
#[derive(Debug, Clone)]
pub enum EnhancedChatEvent {
    /// Message was sent
    MessageSent {
        message: EnhancedChatMessage,
        conversation_id: ConversationId,
    },
    /// Streaming started
    StreamingStarted {
        conversation_id: ConversationId,
        message_id: String,
    },
    /// Streaming chunk received
    StreamingChunk {
        conversation_id: ConversationId,
        message_id: String,
        chunk: RenderChunk,
    },
    /// Streaming completed
    StreamingCompleted {
        conversation_id: ConversationId,
        message_id: String,
        total_time: std::time::Duration,
    },
    /// Context items changed
    ContextChanged {
        context_items: Vec<Arc<ContextItem>>,
    },
    /// Reaction added
    ReactionAdded {
        message_id: String,
        reaction: MessageReaction,
    },
    /// Error occurred
    Error {
        error: String,
        context: Option<String>,
    },
}

impl EnhancedChatComponent {
    /// Create a new enhanced chat component
    pub fn new(config: EnhancedChatConfig) -> Self {
        let id = Ulid::new().to_string().to_string();

        Self {
            context_component: ContextComponent::new(crate::components::ContextComponentConfig::default()),
            search_context_bridge: SearchContextBridge::new(crate::components::SearchContextBridgeConfig::default()),
            config,
            state: EnhancedChatState {
                conversation_id: None,
                messages: Vec::new(),
                streaming_messages: std::collections::HashMap::new(),
                context_items: Vec::new(),
                context_search_query: String::new(),
                is_typing: false,
                typing_users: Vec::new(),
                selected_message: None,
                show_context_panel: config.show_context_panel,
                scroll_position: 0.0,
                is_mounted: false,
                performance_metrics: ChatPerformanceMetrics::default(),
                last_update: std::time::Instant::now(),
                current_role: RoleName::from("Terraphim Assistant"),
                available_models: vec![
                    "gpt-4".to_string(),
                    "gpt-3.5-turbo".to_string(),
                    "claude-3".to_string(),
                ],
                selected_model: None,
            },
            performance_tracker: PerformanceTracker::new(id.clone()),
            id,
            event_emitter: None,
        }
    }

    /// Set current conversation
    pub fn set_conversation(&mut self, conversation_id: ConversationId) {
        self.state.conversation_id = Some(conversation_id);
        self.state.last_update = std::time::Instant::now();
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: ChatMessage, cx: &mut gpui::Context<'_, Self>) {
        let enhanced_message = EnhancedChatMessage {
            message: message.clone(),
            streaming_state: None,
            context_items: self.state.context_items.clone(),
            reactions: Vec::new(),
            metadata: MessageMetadata {
                message_id: Ulid::new().to_string().to_string(),
                conversation_id: self.state.conversation_id.clone().unwrap_or_else(ConversationId::new),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                processing_time: None,
                token_count: Some(message.content.len()),
                model_used: self.state.selected_model.clone(),
                confidence_score: None,
            },
            rendering_state: MessageRenderingState::Pending,
        };

        self.state.messages.push(enhanced_message.clone());

        // Limit messages to max_messages
        if self.state.messages.len() > self.config.max_messages {
            self.state.messages.drain(0..self.state.messages.len() - self.config.max_messages);
        }

        // Update performance metrics
        self.state.performance_metrics.total_messages += 1;
        self.state.last_update = std::time::Instant::now();

        // Emit event
        if let Some(ref event_emitter) = self.event_emitter {
            event_emitter.update(cx, |emitter, cx| {
            emitter.emit(EnhancedChatEvent::MessageSent {
                message: enhanced_message,
                conversation_id: self.state.conversation_id.clone().unwrap_or_else(ConversationId::new),
            }, cx);
            });
        }

        cx.notify();
    }

    /// Start streaming a message
    pub fn start_streaming(&mut self, message: ChatMessage, cx: &mut gpui::Context<'_, Self>) -> String {
        let message_id = Ulid::new().to_string().to_string();
        let conversation_id = self.state.conversation_id.clone().unwrap_or_else(ConversationId::new);

        let streaming_message = StreamingChatMessage::start_streaming(message.clone());

        let enhanced_message = EnhancedChatMessage {
            message,
            streaming_state: Some(streaming_message.clone()),
            context_items: self.state.context_items.clone(),
            reactions: Vec::new(),
            metadata: MessageMetadata {
                message_id: message_id.clone(),
                conversation_id: conversation_id.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                processing_time: Some(std::time::Duration::from_millis(0)),
                token_count: Some(0),
                model_used: self.state.selected_model.clone(),
                confidence_score: None,
            },
            rendering_state: MessageRenderingState::Rendering,
        };

        self.state.messages.push(enhanced_message);

        // Add to streaming messages map
        let conversation_streams = self.state.streaming_messages
            .entry(conversation_id.clone())
            .or_insert_with(Vec::new);
        conversation_streams.push(streaming_message);

        self.state.performance_metrics.streaming_messages += 1;
        self.state.last_update = std::time::Instant::now();

        // Emit events
        if let Some(ref event_emitter) = self.event_emitter {
            event_emitter.update(cx, |emitter, cx| {
            emitter.emit(EnhancedChatEvent::StreamingStarted {
                conversation_id,
                message_id: message_id.clone(),
            }, cx);
            });
        }

        cx.notify();
        message_id
    }

    /// Add a streaming chunk
    pub fn add_streaming_chunk(&mut self, message_id: &str, chunk: RenderChunk, cx: &mut gpui::Context<'_, Self>) -> Result<(), String> {
        // Find the message with streaming state
        if let Some(message) = self.state.messages.iter_mut().find(|m| m.metadata.message_id == message_id) {
            if let Some(ref mut streaming_state) = message.streaming_state {
                streaming_state.add_chunk(chunk.clone());
                message.metadata.updated_at = Utc::now();

                // Update token count
                if let Some(content) = streaming_state.get_content() {
                    message.metadata.token_count = Some(content.len());
                }

                self.state.last_update = std::time::Instant::now();

                // Emit event
                if let Some(ref event_emitter) = self.event_emitter {
                    event_emitter.update(cx, |emitter, cx| {
                        emitter.emit(EnhancedChatEvent::StreamingChunk {
                            conversation_id: self.state.conversation_id.clone().unwrap_or_else(ConversationId::new),
                            message_id: message_id.to_string(),
                            chunk,
                        }, cx);
                    });
                }

                cx.notify();
                return Ok(());
            }
        }

        Err(format!("Message {} not found or not streaming", message_id))
    }

    /// Complete streaming for a message
    pub fn complete_streaming(&mut self, message_id: &str, cx: &mut gpui::Context<'_, Self>) -> Result<(), String> {
        let start_time = std::time::Instant::now();

        if let Some(message) = self.state.messages.iter_mut().find(|m| m.metadata.message_id == message_id) {
            if let Some(ref mut streaming_state) = message.streaming_state {
                streaming_state.complete_streaming();
                message.rendering_state = MessageRenderingState::Rendered;

                // Calculate processing time
                if let Some(start) = streaming_state.stream_metrics.started_at {
                    let duration = Utc::now().signed_duration_since(start);
                    message.metadata.processing_time = Some(std::time::Duration::from_millis(duration.num_milliseconds() as u64));

                    // Update performance metrics
                    let processing_time = message.metadata.processing_time.unwrap();
                    self.state.performance_metrics.total_processing_time += processing_time;
                    let total_messages = self.state.performance_metrics.total_messages;
                    self.state.performance_metrics.average_processing_time =
                        self.state.performance_metrics.total_processing_time / total_messages as u32;
                }

                // Update final token count
                message.metadata.token_count = Some(streaming_state.get_content().unwrap_or_default().len());
                message.rendering_state = MessageRenderingState::Rendered;

                self.state.last_update = std::time::Instant::now();

                // Emit completion event
                if let Some(ref event_emitter) = self.event_emitter {
                    event_emitter.update(cx, |emitter, cx| {
                        emitter.emit(EnhancedChatEvent::StreamingCompleted {
                            conversation_id: self.state.conversation_id.clone().unwrap_or_else(ConversationId::new),
                            message_id: message_id.to_string(),
                            total_time: start_time.elapsed(),
                        }, cx);
                    });
                }

                cx.notify();
                return Ok(());
            }
        }

        Err(format!("Message {} not found or not streaming", message_id))
    }

    /// Add context items to the chat
    pub fn add_context_items(&mut self, items: Vec<Arc<ContextItem>>, cx: &mut gpui::Context<'_, Self>) {
        self.state.context_items = items;
        self.state.performance_metrics.context_hits += items.len();
        self.state.last_update = std::time::Instant::now();

        // Emit context change event
        if let Some(ref event_emitter) = self.event_emitter {
            event_emitter.update(cx, |emitter, cx| {
                emitter.emit(EnhancedChatEvent::ContextChanged {
                    context_items: self.state.context_items.clone(),
                }, cx);
            });
        }

        cx.notify();
    }

    /// Toggle typing indicator
    pub fn set_typing(&mut self, is_typing: bool, cx: &mut gpui::Context<'_, Self>) {
        self.state.is_typing = is_typing;
        self.state.last_update = std::time::Instant::now();
        cx.notify();
    }

    /// Add typing user
    pub fn add_typing_user(&mut self, user: String, cx: &mut gpui::Context<'_, Self>) {
        if !self.state.typing_users.contains(&user) {
            self.state.typing_users.push(user);
            self.state.last_update = std::time::Instant::now();
            cx.notify();
        }
    }

    /// Remove typing user
    pub fn remove_typing_user(&mut self, user: &str, cx: &mut gpui::Context<'_, Self>) {
        self.state.typing_users.retain(|u| u != user);
        self.state.last_update = std::time::Instant::now();
        cx.notify();
    }

    /// Toggle context panel
    pub fn toggle_context_panel(&mut self, cx: &mut gpui::Context<'_, Self>) {
        self.state.show_context_panel = !self.state.show_context_panel;
        self.state.last_update = std::time::Instant::now();
        cx.notify();
    }

    /// Set current role
    pub fn set_role(&mut self, role: RoleName, cx: &mut gpui::Context<'_, Self>) {
        self.state.current_role = role;
        self.state.last_update = std::time::Instant::now();
        cx.notify();
    }

    /// Set selected model
    pub fn set_model(&mut self, model: String, cx: &mut gpui::Context<'_, Self>) {
        self.state.selected_model = Some(model);
        self.state.last_update = std::time::Instant::now();
        cx.notify();
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> &ChatPerformanceMetrics {
        &self.state.performance_metrics
    }

    /// Get current context items
    pub fn get_context_items(&self) -> &[Arc<ContextItem>] {
        &self.state.context_items
    }

    /// Get messages for current conversation
    pub fn get_messages(&self) -> &[EnhancedChatMessage] {
        &self.state.messages
    }

    /// Subscribe to events
    pub fn subscribe<F, C>(&self, cx: &mut C, callback: F) -> gpui::Subscription
    where
        C: AppContext,
        F: Fn(&EnhancedChatEvent, &mut C) + 'static,
    {
        cx.subscribe(&self.event_emitter, move |_, event, cx| {
            callback(event, cx);
        })
    }
}

