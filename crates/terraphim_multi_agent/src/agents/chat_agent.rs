//! Chat Agent for Agent-Based Conversational AI
//!
//! This agent specializes in maintaining conversational context and providing
//! intelligent responses using the new generic LLM interface.

use crate::{GenAiLlmClient, LlmMessage, LlmRequest, MultiAgentResult, TerraphimAgent};
use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

/// Configuration for chat behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    /// Maximum number of messages to keep in context
    pub max_context_messages: usize,
    /// Default system prompt for the chat
    pub system_prompt: Option<String>,
    /// Temperature for response generation (0.0 = deterministic, 1.0 = creative)
    pub temperature: f32,
    /// Maximum tokens for responses
    pub max_response_tokens: u64,
    /// Enable context summarization when context gets too long
    pub enable_context_summarization: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            max_context_messages: 20,
            system_prompt: None,
            temperature: 0.7,
            max_response_tokens: 500,
            enable_context_summarization: true,
        }
    }
}

/// A single message in the chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub content: String,
    pub role: ChatMessageRole,
    pub timestamp: DateTime<Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Role types for chat messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatMessageRole {
    System,
    User,
    Assistant,
}

impl ChatMessage {
    pub fn user(content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            role: ChatMessageRole::User,
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            role: ChatMessageRole::Assistant,
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn system(content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            role: ChatMessageRole::System,
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Conversation session with context management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: Uuid,
    pub messages: VecDeque<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub title: Option<String>,
}

impl Default for ChatSession {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatSession {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            messages: VecDeque::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            title: None,
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push_back(message);
        self.updated_at = Utc::now();
    }

    pub fn get_recent_messages(&self, count: usize) -> Vec<&ChatMessage> {
        self.messages.iter().rev().take(count).rev().collect()
    }
}

/// Specialized agent for chat conversations
pub struct ChatAgent {
    /// Core Terraphim agent with role-based configuration
    terraphim_agent: TerraphimAgent,
    /// LLM client for generating responses
    llm_client: Arc<GenAiLlmClient>,
    /// Chat configuration
    config: ChatConfig,
    /// Current active session
    current_session: Option<ChatSession>,
    /// Stored sessions for this agent
    sessions: std::collections::HashMap<Uuid, ChatSession>,
}

impl ChatAgent {
    /// Create a new ChatAgent
    pub async fn new(
        terraphim_agent: TerraphimAgent,
        config: Option<ChatConfig>,
    ) -> MultiAgentResult<Self> {
        // Extract LLM configuration from the agent's role
        let role = &terraphim_agent.role_config;

        // Create LLM client based on role configuration
        let llm_client = if let Some(provider) = role.extra.get("llm_provider") {
            let provider_str = provider.as_str().unwrap_or("ollama");
            let model = role
                .extra
                .get("llm_model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            Arc::new(GenAiLlmClient::from_config(provider_str, model)?)
        } else {
            // Default to Ollama with gemma3:270m
            Arc::new(GenAiLlmClient::new_ollama(Some("gemma3:270m".to_string()))?)
        };

        // Use system prompt from role if available
        let mut chat_config = config.unwrap_or_default();
        if chat_config.system_prompt.is_none() {
            if let Some(system_prompt) = role.extra.get("llm_chat_system_prompt") {
                chat_config.system_prompt = system_prompt.as_str().map(|s| s.to_string());
            }
        }

        info!("Created ChatAgent with provider: {}", llm_client.provider());

        Ok(Self {
            terraphim_agent,
            llm_client,
            config: chat_config,
            current_session: None,
            sessions: std::collections::HashMap::new(),
        })
    }

    /// Start a new chat session
    pub fn start_new_session(&mut self) -> Uuid {
        let session = ChatSession::new();
        let session_id = session.id;

        // Add system message if configured
        if let Some(system_prompt) = &self.config.system_prompt {
            let mut session = session;
            session.add_message(ChatMessage::system(system_prompt.clone()));
            self.current_session = Some(session.clone());
            self.sessions.insert(session_id, session);
        } else {
            self.current_session = Some(session.clone());
            self.sessions.insert(session_id, session);
        }

        info!("Started new chat session: {}", session_id);
        session_id
    }

    /// Switch to an existing session
    pub fn switch_to_session(&mut self, session_id: Uuid) -> MultiAgentResult<()> {
        if let Some(session) = self.sessions.get(&session_id) {
            self.current_session = Some(session.clone());
            info!("Switched to session: {}", session_id);
            Ok(())
        } else {
            warn!("Session not found: {}", session_id);
            Err(crate::MultiAgentError::SessionNotFound(session_id))
        }
    }

    /// Send a message and get a response
    pub async fn chat(&mut self, user_message: String) -> MultiAgentResult<String> {
        // Ensure we have an active session
        if self.current_session.is_none() {
            self.start_new_session();
        }

        // Get session ID for later updates
        let session_id = self.current_session.as_ref().unwrap().id;

        // Add user message to session
        let user_msg = ChatMessage::user(user_message);
        if let Some(session) = self.current_session.as_mut() {
            session.add_message(user_msg);
        }

        // Prepare context for LLM (clone current session to avoid borrow conflicts)
        let current_session = self.current_session.clone().unwrap();
        let messages = self.prepare_llm_context(&current_session)?;

        // Use context window from role config, fallback to config default
        let max_tokens = self
            .terraphim_agent
            .role_config
            .llm_context_window
            .map(|cw| (cw / 2).min(4000)) // Use 1/2 of context window, max 4000 for chat responses
            .unwrap_or(self.config.max_response_tokens);

        // Generate response
        let request = LlmRequest::new(messages)
            .with_temperature(self.config.temperature)
            .with_max_tokens(max_tokens);

        debug!("Sending chat request to LLM");
        let response = self.llm_client.generate(request).await?;

        // Add assistant response to session
        let assistant_msg = ChatMessage::assistant(response.content.clone());
        if let Some(session) = self.current_session.as_mut() {
            session.add_message(assistant_msg);
        }

        // Update stored session
        if let Some(current_session) = &self.current_session {
            if let Some(stored_session) = self.sessions.get_mut(&session_id) {
                *stored_session = current_session.clone();
            }
        }

        // Manage context size - extract session temporarily to avoid borrow conflicts
        let session_needs_management = self
            .current_session
            .as_ref()
            .map(|s| s.messages.len() > self.config.max_context_messages * 2)
            .unwrap_or(false);

        if session_needs_management {
            let mut session = self.current_session.take().unwrap();
            self.manage_context_size(&mut session).await?;
            self.current_session = Some(session);
        }

        info!(
            "Generated chat response of {} characters",
            response.content.len()
        );
        Ok(response.content.trim().to_string())
    }

    /// Prepare LLM context from chat session
    fn prepare_llm_context(&self, session: &ChatSession) -> MultiAgentResult<Vec<LlmMessage>> {
        let recent_messages = session.get_recent_messages(self.config.max_context_messages);

        let mut llm_messages = Vec::new();

        for msg in recent_messages {
            let llm_msg = match msg.role {
                ChatMessageRole::System => LlmMessage::system(msg.content.clone()),
                ChatMessageRole::User => LlmMessage::user(msg.content.clone()),
                ChatMessageRole::Assistant => LlmMessage::assistant(msg.content.clone()),
            };
            llm_messages.push(llm_msg);
        }

        Ok(llm_messages)
    }

    /// Manage context size by summarizing old messages if needed
    async fn manage_context_size(&mut self, session: &mut ChatSession) -> MultiAgentResult<()> {
        if !self.config.enable_context_summarization {
            return Ok(());
        }

        if session.messages.len() > self.config.max_context_messages * 2 {
            info!("Context size exceeded, performing summarization");

            // Keep system message and recent messages, summarize the middle
            let system_msgs: Vec<_> = session
                .messages
                .iter()
                .filter(|m| m.role == ChatMessageRole::System)
                .cloned()
                .collect();

            let recent_msgs: Vec<_> = session
                .messages
                .iter()
                .rev()
                .take(self.config.max_context_messages / 2)
                .cloned()
                .collect();

            // Summarize older messages
            let older_msgs: Vec<_> = session
                .messages
                .iter()
                .skip(system_msgs.len())
                .take(session.messages.len() - system_msgs.len() - recent_msgs.len())
                .collect();

            if !older_msgs.is_empty() {
                let summary = self.summarize_conversation(&older_msgs).await?;

                // Rebuild message queue
                let mut new_messages = VecDeque::new();

                // Add system messages
                for msg in system_msgs {
                    new_messages.push_back(msg);
                }

                // Add summary
                new_messages.push_back(ChatMessage::system(format!(
                    "Previous conversation summary: {}",
                    summary
                )));

                // Add recent messages (reverse order since we took them reversed)
                for msg in recent_msgs.into_iter().rev() {
                    new_messages.push_back(msg);
                }

                session.messages = new_messages;
                info!(
                    "Context summarized, new message count: {}",
                    session.messages.len()
                );
            }
        }

        Ok(())
    }

    /// Summarize a portion of the conversation
    async fn summarize_conversation(&self, messages: &[&ChatMessage]) -> MultiAgentResult<String> {
        let conversation_text = messages
            .iter()
            .map(|msg| {
                format!(
                    "{}: {}",
                    match msg.role {
                        ChatMessageRole::User => "User",
                        ChatMessageRole::Assistant => "Assistant",
                        ChatMessageRole::System => "System",
                    },
                    msg.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let summary_prompt = format!(
            "Summarize the following conversation, preserving key information and context:\n\n{}\n\nProvide a concise summary that maintains important details:",
            conversation_text
        );

        let messages = vec![
            LlmMessage::system("You are a conversation summarization expert. Create concise summaries that preserve important context and information.".to_string()),
            LlmMessage::user(summary_prompt),
        ];

        let request = LlmRequest::new(messages)
            .with_temperature(0.3)
            .with_max_tokens(200);

        let response = self.llm_client.generate(request).await?;
        Ok(response.content.trim().to_string())
    }

    /// Get chat history for current session
    pub fn get_chat_history(&self) -> Option<&ChatSession> {
        self.current_session.as_ref()
    }

    /// Get all sessions
    pub fn get_all_sessions(&self) -> &std::collections::HashMap<Uuid, ChatSession> {
        &self.sessions
    }

    /// Update chat configuration
    pub fn update_config(&mut self, config: ChatConfig) {
        self.config = config;
        info!("Updated chat configuration");
    }

    /// Get current configuration
    pub fn get_config(&self) -> &ChatConfig {
        &self.config
    }

    /// Access the underlying Terraphim agent
    pub fn terraphim_agent(&self) -> &TerraphimAgent {
        &self.terraphim_agent
    }

    /// Access the LLM client
    pub fn llm_client(&self) -> &GenAiLlmClient {
        &self.llm_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_agent;

    #[tokio::test]
    async fn test_chat_agent_creation() {
        let agent = create_test_agent().await.unwrap();
        let chat_agent = ChatAgent::new(agent, None).await.unwrap();

        assert_eq!(chat_agent.config.max_context_messages, 20);
        assert_eq!(chat_agent.llm_client.provider(), "ollama");
        assert!(chat_agent.current_session.is_none());
    }

    #[tokio::test]
    async fn test_session_management() {
        let agent = create_test_agent().await.unwrap();
        let mut chat_agent = ChatAgent::new(agent, None).await.unwrap();

        let session_id = chat_agent.start_new_session();
        assert!(chat_agent.current_session.is_some());
        assert!(chat_agent.sessions.contains_key(&session_id));

        let session2_id = chat_agent.start_new_session();
        assert_ne!(session_id, session2_id);

        chat_agent.switch_to_session(session_id).unwrap();
        assert_eq!(chat_agent.current_session.as_ref().unwrap().id, session_id);
    }

    #[test]
    fn test_chat_message_creation() {
        let user_msg = ChatMessage::user("Hello".to_string());
        assert_eq!(user_msg.role, ChatMessageRole::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = ChatMessage::assistant("Hi there!".to_string());
        assert_eq!(assistant_msg.role, ChatMessageRole::Assistant);
        assert_eq!(assistant_msg.content, "Hi there!");
    }
}
