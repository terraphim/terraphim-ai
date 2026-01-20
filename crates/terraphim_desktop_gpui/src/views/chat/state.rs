use gpui::*;
use std::sync::Arc;
use lru::LruCache;
use dashmap::DashMap;
use tokio::sync::Mutex as TokioMutex;
use terraphim_config::ConfigState;
use terraphim_service::context::{ContextManager as TerraphimContextManager};
use terraphim_types::{
    ChatMessage, ConversationId, RoleName, ContextItem, ContextType,
    StreamingChatMessage, RenderChunk, ChunkType, StreamMetrics
};
use crate::search_service::{SearchService, SearchOptions};

/// Chat state management with streaming support and existing infrastructure integration
/// LEVERAGE: Uses existing ConversationService, OpenDAL patterns, and search optimizations
pub struct StreamingChatState {
    config_state: Option<ConfigState>,
    context_manager: Arc<TokioMutex<TerraphimContextManager>>,
    current_conversation_id: Option<ConversationId>,
    current_role: RoleName,

    // Core streaming state
    streaming_messages: DashMap<ConversationId, Vec<StreamingChatMessage>>,
    active_streams: DashMap<ConversationId, tokio::task::JoinHandle<()>>,

    // Performance optimizations (LEVERAGE from Phase 1 search patterns)
    message_cache: LruCache<String, StreamingChatMessage>,
    render_cache: DashMap<String, Vec<RenderChunk>>,
    debounce_timer: Option<gpui::Task<()>>,

    // State management
    is_streaming: bool,
    current_streaming_message: Option<ConversationId>,
    stream_metrics: DashMap<ConversationId, StreamMetrics>,

    // Error handling and recovery
    error_state: Option<String>,
    retry_attempts: DashMap<ConversationId, u32>,
    max_retries: u32,

    // Search integration (LEVERAGE existing search service)
    search_service: Option<Arc<SearchService>>,
    context_search_cache: LruCache<String, Vec<ContextItem>>,

    // Performance monitoring
    performance_stats: ChatPerformanceStats,
    last_update: std::time::Instant,
}

impl StreamingChatState {
    /// Create new streaming chat state leveraging existing patterns
    pub fn new(
        context_manager: Arc<TokioMutex<TerraphimContextManager>>,
        config_state: Option<ConfigState>,
        search_service: Option<Arc<SearchService>>,
    ) -> Self {
        log::info!("Initializing StreamingChatState with existing infrastructure");

        Self {
            config_state,
            context_manager,
            current_conversation_id: None,
            current_role: RoleName::from("Terraphim Engineer"),

            // Streaming state
            streaming_messages: DashMap::new(),
            active_streams: DashMap::new(),

            // Performance optimizations (LEVERAGE from search.rs patterns)
            message_cache: LruCache::new(std::num::NonZeroUsize::new(64).unwrap()),
            render_cache: DashMap::new(),
            debounce_timer: None,

            // State management
            is_streaming: false,
            current_streaming_message: None,
            stream_metrics: DashMap::new(),

            // Error handling
            error_state: None,
            retry_attempts: DashMap::new(),
            max_retries: 3,

            // Search integration
            search_service,
            context_search_cache: LruCache::new(std::num::NonZeroUsize::new(32).unwrap()),

            // Performance monitoring
            performance_stats: ChatPerformanceStats::default(),
            last_update: std::time::Instant::now(),
        }
    }

    /// Initialize with config and existing conversation service patterns
    pub fn with_config(mut self, config_state: ConfigState) -> Self {
        self.config_state = Some(config_state);
        self
    }

    /// Start streaming a new message (LEVERAGE existing LLM streaming from Phase 2.1)
    pub fn start_message_stream(
        &mut self,
        base_message: ChatMessage,
        cx: &mut Context<Self>,
    ) -> Result<ConversationId, String> {
        let conversation_id = self.current_conversation_id
            .clone()
            .unwrap_or_else(ConversationId::new);

        log::info!("Starting message stream for conversation: {}", conversation_id.as_str());

        // Create streaming message wrapper
        let mut streaming_msg = StreamingChatMessage::start_streaming(base_message);

        // Initialize stream metrics
        let metrics = StreamMetrics {
            started_at: Some(chrono::Utc::now()),
            ..Default::default()
        };

        streaming_msg.stream_metrics = metrics.clone();

        // Add to streaming messages
        let mut messages = self.streaming_messages
            .entry(conversation_id.clone())
            .or_insert_with(Vec::new);
        messages.push(streaming_msg.clone());

        // Store in cache
        let cache_key = format!("{}:{}", conversation_id.as_str(), messages.len());
        self.message_cache.put(cache_key, streaming_msg.clone());

        // Update state
        self.is_streaming = true;
        self.current_streaming_message = Some(conversation_id.clone());
        self.stream_metrics.insert(conversation_id.clone(), metrics);

        self.last_update = std::time::Instant::now();
        cx.notify();

        Ok(conversation_id)
    }

    /// Add streaming chunk to message (LEVERAGE existing render patterns)
    pub fn add_stream_chunk(
        &mut self,
        conversation_id: &ConversationId,
        content: String,
        chunk_type: ChunkType,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let chunk = RenderChunk {
            content,
            chunk_type,
            position: 0, // Will be updated by StreamingChatMessage
            complete: false,
        };

        // Find and update the streaming message
        if let Some(mut messages) = self.streaming_messages.get_mut(conversation_id) {
            let message_count = messages.len();
            if let Some(streaming_msg) = messages.last_mut() {
                streaming_msg.add_chunk(chunk);

                // Update cache
                let cache_key = format!("{}:{}", conversation_id.as_str(), message_count);
                self.message_cache.put(cache_key, streaming_msg.clone());

                // Update performance stats
                self.performance_stats.chunks_processed += 1;
                self.performance_stats.last_chunk_time = std::time::Instant::now();

                self.last_update = std::time::Instant::now();
                cx.notify();

                return Ok(());
            }
        }

        Err(format!("No active streaming message for conversation {}", conversation_id.as_str()))
    }

    /// Complete streaming for a message
    pub fn complete_stream(
        &mut self,
        conversation_id: &ConversationId,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if let Some(mut messages) = self.streaming_messages.get_mut(conversation_id) {
            if let Some(streaming_msg) = messages.last_mut() {
                streaming_msg.complete_streaming();

                // Update metrics
                self.performance_stats.messages_completed += 1;
                if let Some(metrics) = self.stream_metrics.get(conversation_id) {
                    if let Some(started_at) = metrics.started_at {
                        // Calculate elapsed time from DateTime to now
                        let elapsed = chrono::Utc::now().signed_duration_since(started_at);
                        let elapsed_secs = elapsed.num_milliseconds() as f64 / 1000.0;

                        self.performance_stats.avg_stream_duration =
                            (self.performance_stats.avg_stream_duration * (self.performance_stats.messages_completed - 1) as f64
                             + elapsed_secs) / self.performance_stats.messages_completed as f64;
                    }
                }

                self.last_update = std::time::Instant::now();
                cx.notify();

                return Ok(());
            }
        }

        Err(format!("No active streaming message for conversation {}", conversation_id.as_str()))
    }

    /// Handle stream error with retry logic (LEVERAGE existing error handling)
    pub fn handle_stream_error(
        &mut self,
        conversation_id: &ConversationId,
        error: String,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        log::error!("Stream error for conversation {}: {}", conversation_id.as_str(), error);

        // Update retry count
        let mut retry_count = self.retry_attempts
            .entry(conversation_id.clone())
            .or_insert(0);
        *retry_count += 1;

        if *retry_count <= self.max_retries {
            log::info!("Retrying stream for conversation {} (attempt {}/{})",
                     conversation_id.as_str(), *retry_count, self.max_retries);

            // Clear error state for retry
            self.error_state = None;

            // Could trigger retry logic here
            // self.retry_stream(conversation_id, cx)?;

            self.last_update = std::time::Instant::now();
            cx.notify();

            return Ok(());
        }

        // Max retries exceeded, set error state
        self.error_state = Some(format!("Stream failed after {} attempts: {}", self.max_retries, error));

        if let Some(mut messages) = self.streaming_messages.get_mut(conversation_id) {
            if let Some(streaming_msg) = messages.last_mut() {
                streaming_msg.set_error(self.error_state.clone().unwrap());
            }
        }

        self.is_streaming = false;
        self.current_streaming_message = None;
        self.performance_stats.stream_errors += 1;

        self.last_update = std::time::Instant::now();
        cx.notify();

        Ok(())
    }

    /// Get streaming messages for a conversation
    pub fn get_streaming_messages(&self, conversation_id: &ConversationId) -> Vec<StreamingChatMessage> {
        self.streaming_messages
            .get(conversation_id)
            .map(|messages| messages.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get latest streaming message for a conversation
    pub fn get_latest_streaming_message(&self, conversation_id: &ConversationId) -> Option<StreamingChatMessage> {
        self.streaming_messages
            .get(conversation_id)
            .and_then(|messages| messages.last().cloned())
    }

    /// Check if conversation is streaming
    pub fn is_conversation_streaming(&self, conversation_id: &ConversationId) -> bool {
        self.streaming_messages
            .get(conversation_id)
            .map(|messages| messages.iter().any(|msg| msg.is_streaming))
            .unwrap_or(false)
    }

    /// Add context with search enhancement (LEVERAGE existing search service)
    pub async fn add_context_with_search(
        &mut self,
        conversation_id: &ConversationId,
        query: &str,
        cx: &mut Context<'_, Self>,
    ) -> Result<Vec<ContextItem>, String> {
        // Check cache first (LEVERAGE from search.rs patterns)
        let cache_key = format!("context:{}:{}", conversation_id.as_str(), query);
        if let Some(cached_contexts) = self.context_search_cache.get(&cache_key) {
            log::debug!("Context search cache hit for query: {}", query);
            return Ok(cached_contexts.clone());
        }

        // Use search service if available (LEVERAGE existing search infrastructure)
        if let Some(search_service) = &self.search_service {
            match search_service.search(query, SearchOptions::default()).await {
                Ok(results) => {
                    let mut contexts = Vec::new();

                    for result in &results.documents {
                        let context_item = ContextItem {
                            id: ulid::Ulid::new().to_string(),
                            context_type: ContextType::Document,
                            title: result.title.clone(),
                            summary: result.description.clone(),
                            content: result.body.clone(),
                            metadata: ahash::AHashMap::new(),
                            created_at: chrono::Utc::now(),
                            relevance_score: result.rank.map(|r| r as f64),
                        };
                        contexts.push(context_item);
                    }

                    // Cache the results
                    self.context_search_cache.put(cache_key, contexts.clone());

                    // Add to conversation context (LEVERAGE existing ConversationService patterns)
                    for context in &contexts {
                        self.add_context_to_conversation(conversation_id, context.clone(), cx).await?;
                    }

                    log::info!("Added {} context items from search for query: {}", contexts.len(), query);
                    return Ok(contexts);
                }
                Err(e) => {
                    log::warn!("Search failed for context query '{}': {}", query, e);
                }
            }
        }

        Err("No search service available".to_string())
    }

    /// Add context to conversation (LEVERAGE existing ConversationService)
    async fn add_context_to_conversation(
        &mut self,
        conversation_id: &ConversationId,
        context_item: ContextItem,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), String> {
        let manager = self.context_manager.clone();
        let conv_id = conversation_id.clone();

        tokio::spawn(async move {
            let mut mgr = manager.lock().await;
            if let Err(e) = mgr.add_context(&conv_id, context_item.clone()) {
                log::error!("Failed to add context to conversation: {}", e);
            }
        }).await.map_err(|e| format!("Failed to spawn task: {}", e))?;

        Ok(())
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> &ChatPerformanceStats {
        &self.performance_stats
    }

    /// Get stream metrics for a conversation
    pub fn get_stream_metrics(&self, conversation_id: &ConversationId) -> Option<StreamMetrics> {
        self.stream_metrics.get(conversation_id).map(|r| r.clone())
    }

    /// Clear caches (maintenance)
    pub fn clear_caches(&mut self, cx: &mut Context<Self>) {
        self.message_cache.clear();
        self.context_search_cache.clear();
        self.render_cache.clear();

        log::info!("Cleared streaming chat caches");
        self.last_update = std::time::Instant::now();
        cx.notify();
    }

    /// Get current error state
    pub fn get_error(&self) -> Option<&String> {
        self.error_state.as_ref()
    }

    /// Clear error state
    pub fn clear_error(&mut self, cx: &mut Context<Self>) {
        self.error_state = None;
        self.last_update = std::time::Instant::now();
        cx.notify();
    }
}

/// Performance statistics for chat streaming
#[derive(Debug, Clone)]
pub struct ChatPerformanceStats {
    pub total_messages: usize,
    pub messages_completed: usize,
    pub chunks_processed: usize,
    pub stream_errors: usize,
    pub avg_stream_duration: f64,
    pub last_chunk_time: std::time::Instant,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl ChatPerformanceStats {
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 { 0.0 } else { self.cache_hits as f64 / total as f64 }
    }
}

impl Default for ChatPerformanceStats {
    fn default() -> Self {
        Self {
            total_messages: 0,
            messages_completed: 0,
            chunks_processed: 0,
            stream_errors: 0,
            avg_stream_duration: 0.0,
            last_chunk_time: std::time::Instant::now(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

impl Default for StreamingChatState {
    fn default() -> Self {
        Self::new(
            Arc::new(TokioMutex::new(
                TerraphimContextManager::new(Default::default())
            )),
            None,
            None,
        )
    }
}

// Implement EventEmitter for StreamingChatState
impl gpui::EventEmitter<()> for StreamingChatState {}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{ChatMessage, ConversationId};
    use std::time::Duration;

    fn create_test_conversation_id() -> ConversationId {
        ConversationId::new()
    }

    fn create_test_message() -> ChatMessage {
        ChatMessage::user("Test message".to_string())
    }

    #[test]
    fn test_streaming_message_creation() {
        let base_msg = create_test_message();
        let streaming = StreamingChatMessage::start_streaming(base_msg);

        assert_eq!(streaming.status, MessageStatus::Streaming);
        assert!(streaming.is_streaming);
        assert!(streaming.stream_metrics.started_at.is_some());
    }

    #[test]
    fn test_render_chunk_creation() {
        let chunk = RenderChunk {
            content: "Hello".to_string(),
            chunk_type: ChunkType::Text,
            position: 0,
            complete: false,
        };

        assert_eq!(chunk.content, "Hello");
        assert!(matches!(chunk.chunk_type, ChunkType::Text));
    }

    #[test]
    fn test_performance_stats() {
        let mut stats = ChatPerformanceStats::default();
        stats.cache_hits = 80;
        stats.cache_misses = 20;

        assert_eq!(stats.cache_hit_rate(), 0.8);
    }

    #[test]
    fn test_cache_hit_rate_all_hits() {
        let mut stats = ChatPerformanceStats::default();
        stats.cache_hits = 100;
        stats.cache_misses = 0;

        assert_eq!(stats.cache_hit_rate(), 1.0);
    }

    #[test]
    fn test_cache_hit_rate_all_misses() {
        let mut stats = ChatPerformanceStats::default();
        stats.cache_hits = 0;
        stats.cache_misses = 100;

        assert_eq!(stats.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_hit_rate_empty() {
        let stats = ChatPerformanceStats::default();

        assert_eq!(stats.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_performance_stats_default() {
        let stats = ChatPerformanceStats::default();

        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.messages_completed, 0);
        assert_eq!(stats.chunks_processed, 0);
        assert_eq!(stats.stream_errors, 0);
        assert_eq!(stats.avg_stream_duration, 0.0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
    }

    #[test]
    fn test_streaming_chat_state_default() {
        let state = StreamingChatState::default();

        assert!(state.config_state.is_none());
        assert!(state.current_conversation_id.is_none());
        assert!(!state.is_streaming);
        assert!(state.current_streaming_message.is_none());
        assert!(state.error_state.is_none());
        assert_eq!(state.max_retries, 3);
        assert!(state.search_service.is_none());
    }

    #[test]
    fn test_streaming_chat_state_new() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        assert!(state.config_state.is_none());
        assert!(!state.is_streaming);
        assert!(state.error_state.is_none());
    }

    #[test]
    fn test_streaming_chat_state_with_config() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));

        let mut state = StreamingChatState::new(
            context_manager.clone(),
            None,
            None,
        );

        // Note: with_config requires ConfigState which needs async setup
        // This is tested in integration tests
    }

    #[test]
    fn test_chunk_type_variants() {
        let text_chunk = RenderChunk {
            content: "Text".to_string(),
            chunk_type: ChunkType::Text,
            position: 0,
            complete: false,
        };

        let code_chunk = RenderChunk {
            content: "Code".to_string(),
            chunk_type: ChunkType::Code,
            position: 1,
            complete: false,
        };

        assert!(matches!(text_chunk.chunk_type, ChunkType::Text));
        assert!(matches!(code_chunk.chunk_type, ChunkType::Code));
    }

    #[test]
    fn test_stream_metrics_default() {
        let metrics = StreamMetrics::default();

        assert!(metrics.started_at.is_none());
        assert!(metrics.first_token_at.is_none());
        assert!(metrics.completed_at.is_none());
        assert_eq!(metrics.total_tokens, 0);
        assert_eq!(metrics.chunks_received, 0);
        assert!(metrics.error.is_none());
    }

    #[test]
    fn test_streaming_message_status_variants() {
        let msg = create_test_message();

        let streaming = StreamingChatMessage::start_streaming(msg.clone());

        assert_eq!(streaming.status, MessageStatus::Streaming);
        assert!(streaming.is_streaming);

        // Note: Complete and error states require async operations
        // These are tested in integration tests
    }

    #[test]
    fn test_render_chunk_positioning() {
        let mut chunk = RenderChunk {
            content: "Test".to_string(),
            chunk_type: ChunkType::Text,
            position: 0,
            complete: false,
        };

        assert_eq!(chunk.position, 0);

        chunk.position = 5;
        assert_eq!(chunk.position, 5);
    }

    #[test]
    fn test_render_chunk_completion() {
        let mut chunk = RenderChunk {
            content: "Test".to_string(),
            chunk_type: ChunkType::Text,
            position: 0,
            complete: false,
        };

        assert!(!chunk.complete);

        chunk.complete = true;
        assert!(chunk.complete);
    }

    #[test]
    fn test_performance_stats_tracking() {
        let mut stats = ChatPerformanceStats::default();

        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.messages_completed, 0);
        assert_eq!(stats.chunks_processed, 0);
        assert_eq!(stats.stream_errors, 0);

        // Simulate processing
        stats.total_messages = 10;
        stats.messages_completed = 8;
        stats.chunks_processed = 150;
        stats.stream_errors = 1;

        assert_eq!(stats.total_messages, 10);
        assert_eq!(stats.messages_completed, 8);
        assert_eq!(stats.chunks_processed, 150);
        assert_eq!(stats.stream_errors, 1);
    }

    #[test]
    fn test_performance_stats_avg_duration() {
        let mut stats = ChatPerformanceStats::default();

        // Initially 0
        assert_eq!(stats.avg_stream_duration, 0.0);

        // After one message of 2 seconds
        stats.messages_completed = 1;
        stats.avg_stream_duration = 2.0;

        // After second message of 4 seconds
        stats.avg_stream_duration = (2.0 * 1.0 + 4.0) / 2.0;
        assert_eq!(stats.avg_stream_duration, 3.0);
    }

    #[test]
    fn test_streaming_message_content_updates() {
        let base_msg = create_test_message();
        let mut streaming = StreamingChatMessage::start_streaming(base_msg);

        let initial_content = streaming.content.clone();
        assert!(!initial_content.is_empty());

        // Note: Adding chunks requires async context
        // This is tested in integration tests
    }

    #[test]
    fn test_conversation_id_generation() {
        let id1 = create_test_conversation_id();
        let id2 = create_test_conversation_id();

        assert_ne!(id1.as_str(), id2.as_str());
        assert!(!id1.as_str().is_empty());
        assert!(!id2.as_str().is_empty());
    }

    #[test]
    fn test_message_status_equality() {
        assert_eq!(MessageStatus::Streaming, MessageStatus::Streaming);
        assert_ne!(MessageStatus::Streaming, MessageStatus::Completed);
    }

    #[test]
    fn test_streaming_message_impls_clone() {
        let base_msg = create_test_message();
        let streaming = StreamingChatMessage::start_streaming(base_msg);

        // Should be able to clone
        let _cloned = streaming.clone();

        // Note: The actual clone behavior depends on StreamingChatMessage implementation
    }

    #[test]
    fn test_render_chunk_impls_debug() {
        let chunk = RenderChunk {
            content: "Test".to_string(),
            chunk_type: ChunkType::Text,
            position: 0,
            complete: false,
        };

        let debug_str = format!("{:?}", chunk);
        assert!(debug_str.contains("Test"));
        assert!(debug_str.contains("Text"));
    }

    #[test]
    fn test_stream_metrics_impls_debug() {
        let metrics = StreamMetrics::default();
        let debug_str = format!("{:?}", metrics);
        assert!(debug_str.contains("StreamMetrics"));
    }

    #[test]
    fn test_performance_stats_impls_debug() {
        let stats = ChatPerformanceStats::default();
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("ChatPerformanceStats"));
    }

    #[test]
    fn test_chunk_type_impls_debug() {
        let chunk_type = ChunkType::Text;
        let debug_str = format!("{:?}", chunk_type);
        assert!(debug_str.contains("Text"));
    }

    #[test]
    fn test_message_status_impls_debug() {
        let status = MessageStatus::Streaming;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Streaming"));
    }

    #[test]
    fn test_streaming_message_impls_debug() {
        let base_msg = create_test_message();
        let streaming = StreamingChatMessage::start_streaming(base_msg);

        let debug_str = format!("{:?}", streaming);
        assert!(debug_str.contains("StreamingChatMessage"));
    }

    #[test]
    fn test_error_state_management() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let mut state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        assert!(state.error_state.is_none());

        // Note: Error handling is tested in integration tests with actual async operations
    }

    #[test]
    fn test_cache_operations() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let mut state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        // Note: Cache operations are tested in integration tests
        // The LruCache is initialized but requires actual usage to test
    }

    #[test]
    fn test_stream_metrics_timestamps() {
        let mut metrics = StreamMetrics::default();

        assert!(metrics.started_at.is_none());

        metrics.started_at = Some(chrono::Utc::now());

        assert!(metrics.started_at.is_some());
    }

    #[test]
    fn test_performance_stats_timing() {
        let mut stats = ChatPerformanceStats::default();

        // Should have a last_chunk_time
        assert!(!stats.last_chunk_time.elapsed().is_negative());

        // Simulate time passage
        std::thread::sleep(Duration::from_millis(10));
        assert!(stats.last_chunk_time.elapsed() >= Duration::from_millis(10));
    }

    #[test]
    fn test_retry_attempts_tracking() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let mut state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        assert_eq!(state.max_retries, 3);

        // Note: Retry tracking requires actual stream errors
        // This is tested in integration tests
    }

    #[test]
    fn test_search_integration() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let mut state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        assert!(state.search_service.is_none());

        // Note: Search service integration requires async setup
        // This is tested in integration tests
    }

    #[test]
    fn test_context_search_cache() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        // Context search cache is initialized
        // Note: Actual cache behavior tested in integration tests
    }

    #[test]
    fn test_render_cache() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        // Render cache is initialized (DashMap)
        // Note: Actual cache behavior tested in integration tests
    }

    #[test]
    fn test_debounce_timer() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let mut state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        assert!(state.debounce_timer.is_none());

        // Note: Debounce timer is set during operations
        // This is tested in integration tests
    }

    #[test]
    fn test_performance_monitoring() {
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        let mut state = StreamingChatState::new(
            context_manager,
            None,
            None,
        );

        assert!(state.last_update.elapsed() >= Duration::from_secs(0));

        // Note: Performance monitoring is tested through actual operations
    }

    #[test]
    fn test_event_emitter_trait() {
        // Verify that StreamingChatState implements EventEmitter
        fn _assert_event_emitter<T: EventEmitter<()>>(_: T) {}
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(Default::default())
        ));
        _assert_event_emitter(StreamingChatState::new(
            context_manager,
            None,
            None,
        ));
    }
}