use gpui::*;
use std::sync::Arc;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use terraphim_service::{llm, context::ContextManager as TerraphimContextManager};
use terraphim_types::{
    ChatMessage, ConversationId, RoleName, ChunkType, ContextItem, StreamingChatMessage, RenderChunk, MessageStatus
};
use crate::views::chat::state::StreamingChatState;

/// Stream-to-UI coordination with proper cancellation and error recovery
/// LEVERAGE: Uses existing ConversationService patterns and error handling
pub struct StreamingCoordinator {
    state: StreamingChatState,
    context_manager: Arc<TokioMutex<TerraphimContextManager>>,
    active_streams: Arc<TokioMutex<std::collections::HashMap<ConversationId, StreamHandle>>>,
}

/// Handle to an active stream with cancellation capability
pub struct StreamHandle {
    conversation_id: ConversationId,
    task_handle: tokio::task::JoinHandle<()>,
    cancellation_tx: mpsc::Sender<()>,
    is_active: bool,
}

impl StreamingCoordinator {
    /// Create new streaming coordinator
    pub fn new(
        state: StreamingChatState,
        context_manager: Arc<TokioMutex<TerraphimContextManager>>,
    ) -> Self {
        Self {
            state,
            context_manager,
            active_streams: Arc::new(TokioMutex::new(std::collections::HashMap::new())),
        }
    }

    /// Start streaming response from LLM (LEVERAGE existing LLM streaming from Phase 2.1)
    pub async fn start_llm_stream(
        &mut self,
        conversation_id: ConversationId,
        messages: Vec<serde_json::Value>,
        llm_client: Box<dyn llm::LlmClient>,
    ) -> Result<(), String> {
        log::info!("Starting LLM stream for conversation: {}", conversation_id.as_str());

        // Create base message for assistant response
        let base_message = ChatMessage::assistant(String::new(), Some(llm_client.name().to_string()));

        // Note: In a real implementation, we'd need to update the state differently
        // For now, just log that we're starting
        log::info!("Starting message stream for conversation {}", conversation_id.as_str());
        let conv_id = conversation_id.clone();

        // Set up cancellation channel
        let (cancellation_tx, mut cancellation_rx) = mpsc::channel::<()>(1);

        // Create stream options
        let opts = llm::ChatOptions {
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };

        // Start streaming task
        let conv_id_clone = conv_id.clone();

        let task_handle = tokio::spawn(async move {
            // Get streaming response
            let stream_result = llm_client.chat_completion_stream(messages, opts).await;

            match stream_result {
                Ok(mut stream) => {
                    let mut full_content = String::new();
                    let mut chunk_count = 0;

                    // Process stream with cancellation support
                    loop {
                        tokio::select! {
                            chunk_result = stream.next() => {
                                match chunk_result {
                                    Some(Ok(chunk_content)) => {
                                        chunk_count += 1;
                                        full_content.push_str(&chunk_content);

                                        // Determine chunk type based on content
                                        let chunk_type = Self::detect_chunk_type(&chunk_content);

                                        // Add chunk to state (this would need a way to update the UI)
                                        // For now, just log the chunk
                                        log::debug!("Received chunk {}: {} chars (type: {:?})",
                                                   chunk_count, chunk_content.len(), chunk_type);
                                    }
                                    Some(Err(e)) => {
                                        log::error!("Stream chunk error: {}", e);
                                        // Handle error in state
                                        break;
                                    }
                                    None => {
                                        log::info!("Stream completed for conversation {}", conv_id_clone.as_str());
                                        break;
                                    }
                                }
                            }
                            _ = cancellation_rx.recv() => {
                                log::info!("Stream cancelled for conversation {}", conv_id_clone.as_str());
                                break;
                            }
                        }
                    }

                    // Mark streaming as complete
                    // This would need to be integrated with the UI update cycle
                }
                Err(e) => {
                    log::error!("Failed to start LLM stream: {}", e);
                    // Handle error in state
                }
            }
        });

        // Store stream handle
        let stream_handle = StreamHandle {
            conversation_id: conv_id.clone(),
            task_handle,
            cancellation_tx,
            is_active: true,
        };

        let mut active_streams = self.active_streams.lock().await;
        active_streams.insert(conv_id, stream_handle);

        Ok(())
    }

    /// Cancel active stream for conversation
    pub async fn cancel_stream(&mut self, conversation_id: &ConversationId) -> Result<(), String> {
        let mut active_streams = self.active_streams.lock().await;

        if let Some(stream_handle) = active_streams.remove(conversation_id) {
            log::info!("Cancelling stream for conversation: {}", conversation_id.as_str());

            // Send cancellation signal
            let _ = stream_handle.cancellation_tx.send(()).await;

            // Abort the task
            stream_handle.task_handle.abort();

            // Update state to mark stream as cancelled/paused
            // This would integrate with the UI update cycle

            Ok(())
        } else {
            Err(format!("No active stream for conversation: {}", conversation_id.as_str()))
        }
    }

    /// Cancel all active streams
    pub async fn cancel_all_streams(&mut self) -> usize {
        let mut active_streams = self.active_streams.lock().await;
        let count = active_streams.len();

        log::info!("Cancelling {} active streams", count);

        for (conv_id, stream_handle) in active_streams.drain() {
            let _ = stream_handle.cancellation_tx.send(()).await;
            stream_handle.task_handle.abort();
        }

        count
    }

    /// Get active stream information
    pub async fn get_active_streams(&self) -> Vec<ConversationId> {
        let active_streams = self.active_streams.lock().await;
        active_streams.keys().cloned().collect()
    }

    /// Check if conversation has active stream
    pub async fn has_active_stream(&self, conversation_id: &ConversationId) -> bool {
        let active_streams = self.active_streams.lock().await;
        active_streams.contains_key(conversation_id)
    }

    /// Start a new stream (generic interface)
    pub async fn start_stream(
        &mut self,
        conversation_id: ConversationId,
        messages: Vec<serde_json::Value>,
        role: RoleName,
        context_items: Vec<ContextItem>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut streams = self.active_streams.lock().await;

        // Cancel existing stream for this conversation
        if let Some(existing) = streams.get(&conversation_id) {
            existing.cancellation_tx.send(()).await.ok();
        }

        // Create cancellation channel
        let (cancellation_tx, mut cancellation_rx) = mpsc::channel::<()>(1);

        // Create LLM client based on role
        let llm_client = create_llm_client(&role)?;

        // Create stream handle with task
        let stream_handle = StreamHandle {
            conversation_id: conversation_id.clone(),
            task_handle: tokio::spawn(Self::stream_task(
                conversation_id.clone(),
                messages,
                role,
                context_items,
                llm_client,
                cancellation_rx,
            )),
            cancellation_tx,
            is_active: true,
        };

        streams.insert(conversation_id, stream_handle);
        Ok(())
    }

    /// Async task for streaming LLM responses
    async fn stream_task(
        conversation_id: ConversationId,
        messages: Vec<serde_json::Value>,
        role: RoleName,
        context_items: Vec<ContextItem>,
        llm_client: Box<dyn llm::LlmClient>,
        mut cancellation_rx: mpsc::Receiver<()>,
    ) {
        let llm_client_ref = llm_client.as_ref();
        log::info!("Starting stream task for conversation: {}", conversation_id.as_str());

        // Build messages with context
        let mut full_messages = messages;

        if !context_items.is_empty() {
            let mut context_content = String::from("=== CONTEXT ===\n");
            for (idx, item) in context_items.iter().enumerate() {
                context_content.push_str(&format!(
                    "{}. {}\n{}\n\n",
                    idx + 1,
                    item.title,
                    item.content
                ));
            }
            context_content.push_str("=== END CONTEXT ===\n");

            full_messages.insert(
                0,
                serde_json::json!({
                    "role": "system",
                    "content": context_content
                }),
            );
        }

        // Create stream options
        let opts = llm::ChatOptions {
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };

        // Start streaming
        let stream_result = llm_client.chat_completion_stream(full_messages, opts).await;

        match stream_result {
            Ok(mut stream) => {
                let mut chunk_count = 0;

                // Process stream with cancellation support
                loop {
                    tokio::select! {
                        chunk_result = stream.next() => {
                            match chunk_result {
                                Some(Ok(chunk_content)) => {
                                    chunk_count += 1;

                                    // Determine chunk type
                                    let chunk_type = StreamingCoordinator::detect_chunk_type(&chunk_content);

                                    // Send chunk to UI
                                    log::debug!("Sending chunk {} for conversation {}: {} chars",
                                               chunk_count, conversation_id.as_str(), chunk_content.len());
                                }
                                Some(Err(e)) => {
                                    log::error!("Stream chunk error for {}: {}", conversation_id.as_str(), e);
                                    break;
                                }
                                None => {
                                    log::info!("Stream completed for conversation {}", conversation_id.as_str());
                                    break;
                                }
                            }
                        }
                        _ = cancellation_rx.recv() => {
                            log::info!("Stream cancelled for conversation {}", conversation_id.as_str());
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to start LLM stream for {}: {}", conversation_id.as_str(), e);
            }
        }
    }

    /// Send chunk to UI
    pub async fn send_chunk(
        &self,
        conversation_id: &ConversationId,
        content: String,
        chunk_type: ChunkType,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Sending chunk for {}: {} chars (type: {:?})",
                   conversation_id.as_str(), content.len(), chunk_type);

        // In a real implementation, this would send the chunk to the UI via a channel
        // For now, we just log it
        Ok(())
    }

    /// Check if stream is active
    pub async fn is_stream_active(&self, conversation_id: &ConversationId) -> bool {
        let streams = self.active_streams.lock().await;
        streams.contains_key(conversation_id)
    }

    /// Detect chunk type based on content analysis
    fn detect_chunk_type(content: &str) -> ChunkType {
        let trimmed = content.trim();

        // Code block detection
        if trimmed.starts_with("```") {
            if let Some(lang_end) = trimmed.find('\n') {
                let lang_part = &trimmed[3..lang_end];
                if !lang_part.is_empty() {
                    return ChunkType::CodeBlock {
                        language: lang_part.trim().to_string()
                    };
                }
            }
            return ChunkType::CodeBlock {
                language: "unknown".to_string()
            };
        }

        // Markdown detection (headers, links, emphasis)
        if trimmed.starts_with('#') ||
           trimmed.contains("**") ||
           trimmed.contains("*") ||
           trimmed.contains("[") && trimmed.contains("](") {
            ChunkType::Markdown
        }
        // Metadata/system content
        else if trimmed.starts_with("=== ") ||
                trimmed.starts_with("--- ") ||
                trimmed.contains("Error:") ||
                trimmed.contains("Warning:") {
            ChunkType::Metadata
        }
        // Default to text
        else {
            ChunkType::Text
        }
    }

    /// Process incoming chunk with context integration (LEVERAGE existing search patterns)
    async fn process_chunk_with_context(
        &mut self,
        conversation_id: &ConversationId,
        content: &str,
        chunk_type: ChunkType,
    ) -> Result<(), String> {
        // Note: In a real implementation, we'd need to update the state differently
        // For now, just log chunk processing
        log::debug!("Processing chunk for conversation {}: {} chars (type: {:?})",
                   conversation_id.as_str(), content.len(), chunk_type);

        // If it's a text chunk, look for context opportunities (LEVERAGE search service)
        if matches!(chunk_type, ChunkType::Text) {
            if let Some(query) = self.extract_context_query(content) {
                log::debug!("Potential context query extracted: {}", query);
                // In a real implementation, we'd add context here
            }
        }

        Ok(())
    }

    /// Extract potential context queries from text chunks
    fn extract_context_query(&self, content: &str) -> Option<String> {
        // Simple heuristic to extract potential search terms
        // Look for technical terms, questions, or specific keywords
        let words: Vec<&str> = content
            .split_whitespace()
            .filter(|word| word.len() > 3) // Filter out very short words
            .filter(|word| !self.is_stop_word(word))
            .collect();

        if words.len() >= 2 {
            Some(words.iter().take(5).map(|s| *s).collect::<Vec<&str>>().join(" "))
        } else if words.len() == 1 {
            Some(words[0].to_string())
        } else {
            None
        }
    }

    /// Check if a word is a stop word (basic implementation)
    fn is_stop_word(&self, word: &str) -> bool {
        let stop_words = [
            "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
            "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
            "can", "cannot", "must", "shall", "this", "that", "these", "those", "i", "you",
            "he", "she", "it", "we", "they", "what", "which", "who", "when", "where", "why",
            "how", "all", "any", "both", "each", "few", "more", "most", "other", "some",
            "such", "no", "nor", "not", "only", "own", "same", "so", "than", "too", "very",
            "just", "now", "also", "back", "even", "first", "last", "long", "much", "never",
            "next", "once", "over", "really", "still", "such", "then", "too", "well", "only"
        ];

        stop_words.contains(&word.to_lowercase().as_str())
    }

    /// Get streaming statistics
    pub fn get_streaming_stats(&self) -> StreamingStats {
        let state_stats = self.state.get_performance_stats();

        StreamingStats {
            active_streams: 0, // Would need async call to get this
            total_messages: state_stats.total_messages,
            completed_messages: state_stats.messages_completed,
            average_stream_time: state_stats.avg_stream_duration,
            error_rate: if state_stats.total_messages > 0 {
                state_stats.stream_errors as f64 / state_stats.total_messages as f64
            } else {
                0.0
            },
            cache_hit_rate: state_stats.cache_hit_rate(),
        }
    }
}

/// Streaming statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct StreamingStats {
    pub active_streams: usize,
    pub total_messages: usize,
    pub completed_messages: usize,
    pub average_stream_time: f64,
    pub error_rate: f64,
    pub cache_hit_rate: f64,
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        if self.is_active {
            log::debug!("StreamHandle dropped for conversation {}", self.conversation_id.as_str());
        }
    }
}

/// Create LLM client based on role configuration
fn create_llm_client(role: &RoleName) -> Result<Box<dyn llm::LlmClient>, Box<dyn std::error::Error>> {
    // For now, create a simple client - in a real implementation,
    // this would use the role configuration to determine which LLM to use
    log::debug!("Creating LLM client for role: {}", role.as_str());

    // This is a placeholder - in a real implementation, we'd create the actual client
    // based on role configuration (e.g., OpenAI, Ollama, etc.)
    Err("LLM client creation not yet implemented".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_type_detection() {
        // Code block with language
        let chunk1 = "```rust\nfn main() {}\n```";
        assert!(matches!(StreamingCoordinator::detect_chunk_type(chunk1),
                            ChunkType::CodeBlock { language } if language == "rust"));

        // Markdown header
        let chunk2 = "# This is a header";
        assert!(matches!(StreamingCoordinator::detect_chunk_type(chunk2), ChunkType::Markdown));

        // Plain text
        let chunk3 = "This is plain text";
        assert!(matches!(StreamingCoordinator::detect_chunk_type(chunk3), ChunkType::Text));

        // System metadata
        let chunk4 = "=== SYSTEM INFO ===";
        assert!(matches!(StreamingCoordinator::detect_chunk_type(chunk4), ChunkType::Metadata));
    }

    #[test]
    fn test_context_query_extraction() {
        let coordinator = StreamingCoordinator::new(
            StreamingChatState::default(),
            Arc::new(TokioMutex::new(
                TerraphimContextManager::new(Default::default())
            )),
        );

        // Should extract technical terms
        let text1 = "The user is asking about machine learning algorithms";
        assert!(coordinator.extract_context_query(text1).is_some());

        // Short text should not extract
        let text2 = "Hi there";
        assert!(coordinator.extract_context_query(text2).is_none());

        // Text with only stop words should not extract
        let text3 = "the and or but";
        assert!(coordinator.extract_context_query(text3).is_none());
    }

    #[test]
    fn test_stop_word_detection() {
        let coordinator = StreamingCoordinator::new(
            StreamingChatState::default(),
            Arc::new(TokioMutex::new(
                TerraphimContextManager::new(Default::default())
            )),
        );

        assert!(coordinator.is_stop_word("the"));
        assert!(coordinator.is_stop_word("and"));
        assert!(!coordinator.is_stop_word("algorithm"));
        assert!(!coordinator.is_stop_word("machine"));
    }
}