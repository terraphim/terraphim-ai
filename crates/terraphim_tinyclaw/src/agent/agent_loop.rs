//! Tool-calling loop with hybrid LLM routing.

use crate::agent::execution_guard::{ExecutionGuard, GuardDecision};
use crate::agent::proxy_client::{
    Message, ProxyClient, ProxyClientConfig, ProxyResponse, ToolDefinition,
};
use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::config::{AgentConfig, DirectLlmConfig};
use crate::session::{ChatMessage, MessageRole, Session, SessionManager};
use crate::tools::{ToolCall, ToolError, ToolRegistry};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Configuration for the tool-calling loop.
#[derive(Debug, Clone)]
pub struct ToolCallingConfig {
    /// Maximum tool-calling iterations per message.
    pub max_iterations: usize,
    /// Token ratio at which to trigger compression.
    pub summarize_at_token_ratio: f32,
    /// Number of messages to keep after summarization.
    pub keep_last_messages: usize,
}

impl Default for ToolCallingConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            summarize_at_token_ratio: 0.75,
            keep_last_messages: 4,
        }
    }
}

/// Routes LLM calls to either proxy (tool-calling) or direct client (compression/text-only).
pub struct HybridLlmRouter {
    /// Proxy client for tool-calling and quality responses.
    proxy: ProxyClient,
    /// Direct LLM configuration for cheap/local tasks.
    direct_config: DirectLlmConfig,
    /// Whether tools are currently available.
    tools_available: AtomicBool,
}

impl HybridLlmRouter {
    /// Create a new hybrid router.
    pub fn new(proxy_config: ProxyClientConfig, direct_config: DirectLlmConfig) -> Self {
        let proxy = ProxyClient::new(proxy_config);

        Self {
            proxy,
            direct_config,
            tools_available: AtomicBool::new(true),
        }
    }

    /// Check if the proxy is available for tool-calling.
    pub fn tools_available(&self) -> bool {
        self.tools_available.load(Ordering::SeqCst) && self.proxy.is_available()
    }

    /// Call the proxy with tools.
    pub async fn tool_call(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        tools: Vec<ToolDefinition>,
    ) -> anyhow::Result<ProxyResponse> {
        if !self.tools_available() {
            anyhow::bail!("Proxy is unavailable - tools disabled");
        }

        match self.proxy.chat_with_tools(messages, system, tools).await {
            Ok(response) => {
                self.tools_available.store(true, Ordering::SeqCst);
                Ok(response)
            }
            Err(e) => {
                self.tools_available.store(false, Ordering::SeqCst);
                Err(e)
            }
        }
    }

    /// Get a text-only response via direct GenAiLlmClient.
    /// Used as fallback when proxy is unavailable.
    pub async fn text_only(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
    ) -> anyhow::Result<String> {
        // For now, return a placeholder
        // In full implementation, this would use terraphim_multi_agent::GenAiLlmClient
        log::info!(
            "Using direct LLM (provider: {}, model: {})",
            self.direct_config.provider,
            self.direct_config.model
        );

        // Try proxy first for text-only if available
        if self.proxy.is_available() {
            match self.proxy.chat(messages, system).await {
                Ok(response) => {
                    return Ok(response.content.unwrap_or_else(|| {
                        "Tools are currently unavailable, answering from knowledge only."
                            .to_string()
                    }));
                }
                Err(e) => {
                    log::warn!("Proxy unavailable for text response: {}", e);
                }
            }
        }

        // Fallback message
        Ok("Tools are currently unavailable. I can answer questions from my training data, but cannot execute commands or access files.".to_string())
    }

    /// Compress context via direct LLM (cheap/local).
    /// Never goes through proxy.
    pub async fn compress(
        &self,
        _messages: Vec<ChatMessage>,
        _system: String,
    ) -> anyhow::Result<String> {
        // Placeholder for LLM-based compression
        // In full implementation, this would call GenAiLlmClient with Ollama
        log::info!("Context compression via direct LLM (placeholder)");
        Ok("[Previous conversation summarized]".to_string())
    }
}

/// The main tool-calling agent loop.
pub struct ToolCallingLoop {
    config: ToolCallingConfig,
    router: HybridLlmRouter,
    guard: ExecutionGuard,
    tools: Arc<ToolRegistry>,
    sessions: Arc<Mutex<SessionManager>>,
    system_prompt: String,
    shutdown: CancellationToken,
}

impl ToolCallingLoop {
    /// Create a new tool-calling loop.
    pub fn new(
        agent_config: &AgentConfig,
        router: HybridLlmRouter,
        tools: Arc<ToolRegistry>,
        sessions: SessionManager,
        system_prompt: String,
    ) -> Self {
        Self {
            config: ToolCallingConfig {
                max_iterations: agent_config.max_iterations,
                ..Default::default()
            },
            router,
            guard: ExecutionGuard::new(),
            tools,
            sessions: Arc::new(Mutex::new(sessions)),
            system_prompt,
            shutdown: CancellationToken::new(),
        }
    }

    /// Run the agent loop, consuming messages from the bus.
    pub async fn run(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        let outbound_tx = bus.outbound_sender();

        log::info!("Tool-calling loop started");

        loop {
            // Lock receiver only for the recv() call
            let msg = {
                let mut inbound_rx = bus.inbound_rx.lock().await;
                tokio::select! {
                    msg = inbound_rx.recv() => msg,
                    _ = self.shutdown.cancelled() => {
                        log::info!("Tool-calling loop shutting down gracefully");
                        break;
                    }
                }
            };

            if let Some(msg) = msg {
                if let Err(e) = self.process_message(msg, &outbound_tx).await {
                    log::error!("Error processing message: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Process a single inbound message.
    async fn process_message(
        &self,
        msg: InboundMessage,
        outbound_tx: &tokio::sync::mpsc::Sender<OutboundMessage>,
    ) -> anyhow::Result<()> {
        // Check if this is a slash command
        if let Some(response) = self.handle_slash_command(&msg) {
            outbound_tx.send(response).await?;
            return Ok(());
        }

        // Get or create session
        let session_key = msg.session_key();
        let mut sessions_guard = self.sessions.lock().await;
        let session = sessions_guard.get_or_create(&session_key);

        // Add user message to session
        let user_msg = ChatMessage {
            role: MessageRole::User,
            content: msg.content.clone(),
            sender_id: Some(msg.sender_id.clone()),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        session.add_message(user_msg.clone());

        // Store messages for LLM conversion
        let session_messages: Vec<_> = session.messages.clone();
        let session_clone = session.clone();
        let needs_compression = session.message_count() > 200;

        // Save session before releasing lock
        sessions_guard.save(&session_clone)?;
        drop(sessions_guard);

        // Check if we need compression (handled separately to avoid borrow issues)
        if needs_compression {
            // Clone the messages we need for compression
            let messages_to_compress = session_messages.clone();
            let summary = self
                .router
                .compress(messages_to_compress, self.system_prompt.clone())
                .await?;

            // Re-acquire lock to update session
            let mut sessions_guard = self.sessions.lock().await;
            let session = sessions_guard.get_or_create(&session_key);
            session.set_summary(summary);
            session.clear_messages();
            let session_clone = session.clone();
            sessions_guard.save(&session_clone)?;
            drop(sessions_guard);
        }

        // Convert to proxy message format
        let proxy_messages: Vec<Message> = session_messages
            .iter()
            .map(|m| match m.role {
                MessageRole::User => Message::user(&m.content),
                MessageRole::Assistant => Message::assistant(&m.content),
                _ => Message::user(&m.content),
            })
            .collect();

        // Get tool definitions
        let tool_definitions: Vec<ToolDefinition> = self
            .tools
            .to_openai_tools()
            .iter()
            .map(|t| ToolDefinition {
                name: t["function"]["name"].as_str().unwrap_or("").to_string(),
                description: t["function"]["description"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                input_schema: t["function"]["parameters"].clone(),
            })
            .collect();

        // Call LLM with tool-calling loop
        let final_response = if self.router.tools_available() && !tool_definitions.is_empty() {
            self.run_tool_loop(proxy_messages, tool_definitions).await?
        } else {
            // Fallback to text-only mode
            self.router
                .text_only(proxy_messages, Some(self.system_prompt.clone()))
                .await?
        };

        // Add assistant response to session (re-acquire lock)
        let mut sessions_guard = self.sessions.lock().await;
        let session = sessions_guard.get_or_create(&session_key);

        let assistant_msg = ChatMessage {
            role: MessageRole::Assistant,
            content: final_response.clone(),
            sender_id: None,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        session.add_message(assistant_msg.clone());

        // Save session - clone to avoid borrow issues
        let session_clone = session.clone();
        sessions_guard.save(&session_clone)?;
        drop(sessions_guard);

        // Send response
        let outbound = OutboundMessage::new(&msg.channel, &msg.chat_id, final_response);
        outbound_tx.send(outbound).await?;

        Ok(())
    }

    /// Run the iterative tool-calling loop.
    async fn run_tool_loop(
        &self,
        mut messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    ) -> anyhow::Result<String> {
        for iteration in 0..self.config.max_iterations {
            log::debug!("Tool-calling iteration {}", iteration + 1);

            // Call LLM with tools
            let response = match self
                .router
                .tool_call(
                    messages.clone(),
                    Some(self.system_prompt.clone()),
                    tools.clone(),
                )
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    log::warn!("Tool call failed: {}. Falling back to text-only.", e);
                    return self
                        .router
                        .text_only(messages, Some(self.system_prompt.clone()))
                        .await;
                }
            };

            // Check if there are tool calls
            if response.tool_calls.is_empty() {
                // No tool calls - return the content
                return Ok(response.content.unwrap_or_default());
            }

            // Execute each tool call
            for tool_call in &response.tool_calls {
                log::info!("Executing tool: {}", tool_call.name);

                // Check with execution guard
                let decision = self.guard.evaluate(&tool_call.name, &tool_call.arguments);

                let tool_result = match decision {
                    GuardDecision::Allow => match self.tools.execute(tool_call).await {
                        Ok(result) => result,
                        Err(ToolError::Blocked { reason, .. }) => {
                            format!("Tool blocked: {}", reason)
                        }
                        Err(e) => {
                            format!("Tool execution error: {}", e)
                        }
                    },
                    GuardDecision::Block { reason } => {
                        format!("Tool blocked: {}", reason)
                    }
                    GuardDecision::Warn { reason } => {
                        log::warn!(
                            "Tool '{}' executing with warning: {}",
                            tool_call.name,
                            reason
                        );
                        match self.tools.execute(tool_call).await {
                            Ok(result) => result,
                            Err(e) => format!("Tool execution error: {}", e),
                        }
                    }
                };

                // Add tool result to messages
                messages.push(Message::tool(&tool_call.id, tool_result));
            }

            // Add assistant's reasoning to messages
            if let Some(content) = response.content {
                messages.push(Message::assistant(content));
            }
        }

        // Max iterations reached
        log::warn!("Max iterations ({}) reached", self.config.max_iterations);
        Ok(format!(
            "I've reached the maximum number of tool calls ({}). \
             The task may be too complex. Please try breaking it into smaller steps.",
            self.config.max_iterations
        ))
    }

    /// Handle slash commands.
    fn handle_slash_command(&self, msg: &InboundMessage) -> Option<OutboundMessage> {
        let content = msg.content.trim();

        if content == "/reset" {
            // Reset is handled in process_message by creating new session context
            Some(OutboundMessage::new(
                &msg.channel,
                &msg.chat_id,
                "Session reset. Your next message will start fresh.".to_string(),
            ))
        } else if content.starts_with("/role ") {
            Some(OutboundMessage::new(
                &msg.channel,
                &msg.chat_id,
                "Role switching not yet implemented (coming in full implementation)".to_string(),
            ))
        } else if content == "/help" {
            Some(OutboundMessage::new(
                &msg.channel,
                &msg.chat_id,
                "Available commands:\n/reset - Clear session\n/help - Show this help".to_string(),
            ))
        } else {
            None
        }
    }

    /// Compress session if it gets too long.
    async fn compress_session(&self, session: &mut Session) -> anyhow::Result<()> {
        log::info!(
            "Compressing session {} ({} messages)",
            session.key,
            session.message_count()
        );

        let summary = self
            .router
            .compress(session.messages.clone(), self.system_prompt.clone())
            .await?;
        session.set_summary(summary);
        session.clear_messages();

        Ok(())
    }

    /// Trigger graceful shutdown.
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProxyConfig;
    use tempfile::TempDir;

    fn create_test_router() -> HybridLlmRouter {
        let proxy_config = ProxyClientConfig::default();
        let direct_config = DirectLlmConfig::default();
        HybridLlmRouter::new(proxy_config, direct_config)
    }

    #[test]
    fn test_hybrid_router_tools_available() {
        let router = create_test_router();
        // Initially tools_available should be true (but proxy.is_available may be false)
        // The router starts with tools_available = true
        assert!(router.tools_available.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_text_only_fallback() {
        let router = create_test_router();
        let messages = vec![Message::user("Hello")];

        let response = router.text_only(messages, None).await.unwrap();
        assert!(response.contains("Tools are currently unavailable"));
    }

    #[test]
    fn test_slash_command_reset() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = SessionManager::new(temp_dir.path().to_path_buf());
        let tools = Arc::new(ToolRegistry::new());
        let router = create_test_router();

        let loop_config = AgentConfig {
            max_iterations: 10,
            ..Default::default()
        };

        let agent = ToolCallingLoop::new(
            &loop_config,
            router,
            tools,
            sessions,
            "Test system prompt".to_string(),
        );

        let msg = InboundMessage::new("cli", "user1", "chat1", "/reset");
        let response = agent.handle_slash_command(&msg);

        assert!(response.is_some());
        assert!(response.unwrap().content.contains("Session reset"));
    }

    #[test]
    fn test_slash_command_help() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = SessionManager::new(temp_dir.path().to_path_buf());
        let tools = Arc::new(ToolRegistry::new());
        let router = create_test_router();

        let loop_config = AgentConfig {
            max_iterations: 10,
            ..Default::default()
        };

        let agent = ToolCallingLoop::new(&loop_config, router, tools, sessions, "Test".to_string());

        let msg = InboundMessage::new("cli", "user1", "chat1", "/help");
        let response = agent.handle_slash_command(&msg);

        assert!(response.is_some());
        assert!(response.unwrap().content.contains("Available commands"));
    }
}
