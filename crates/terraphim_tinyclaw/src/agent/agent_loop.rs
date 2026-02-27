//! Tool-calling loop with hybrid LLM routing.

use crate::agent::execution_guard::{ExecutionGuard, GuardDecision};
use crate::agent::proxy_client::{
    Message, ProxyClient, ProxyClientConfig, ProxyResponse, ToolDefinition,
};
use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::commands::MarkdownCommandRuntime;
use crate::config::{AgentConfig, DirectLlmConfig};
use crate::session::{ChatMessage, MessageRole, SessionManager};
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
    /// Maximum number of session messages before compression.
    pub max_session_messages: usize,
    /// Number of messages to keep after summarization.
    pub keep_last_messages: usize,
}

impl Default for ToolCallingConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            max_session_messages: 200,
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

    /// Get a text-only response via proxy or direct LLM.
    /// Used as fallback when proxy is unavailable for tool-calling.
    pub async fn text_only(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
    ) -> anyhow::Result<String> {
        log::info!(
            "Using text-only mode (provider: {}, model: {})",
            self.direct_config.provider,
            self.direct_config.model
        );

        // Try proxy first for text-only if available
        if self.proxy.is_available() {
            match self.proxy.chat(messages.clone(), system.clone()).await {
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

        // Try direct LLM (Ollama)
        let base_url = self
            .direct_config
            .base_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

        let last_user_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let prompt = if let Some(sys) = &system {
            format!("{}\n\nUser: {}", sys, last_user_msg)
        } else {
            last_user_msg
        };

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/generate", base_url))
            .json(&serde_json::json!({
                "model": &self.direct_config.model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await?;
                Ok(body["response"]
                    .as_str()
                    .unwrap_or("I received your message but could not generate a response.")
                    .to_string())
            }
            _ => Ok(
                "Tools and direct LLM are currently unavailable. Please check your configuration."
                    .to_string(),
            ),
        }
    }

    /// Compress context via LLM summarization.
    /// Tries proxy first (Claude/OpenAI), falls back to direct LLM (Ollama),
    /// then to extractive summary.
    pub async fn compress(
        &self,
        messages: Vec<ChatMessage>,
        _system: String,
    ) -> anyhow::Result<String> {
        // Format conversation for summarization
        let conversation = messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let summarization_prompt = format!(
            "Summarize the following conversation concisely, \
             preserving key facts, decisions, and context:\n\n{}",
            conversation
        );

        let summarization_system = "You are a conversation summarizer. \
             Summarize concisely, preserving key facts, decisions, and context."
            .to_string();

        log::info!("Context compression - {} messages", messages.len());

        // Tier 1: Try proxy (Claude/OpenAI via terraphim-llm-proxy)
        if self.proxy.is_available() {
            let proxy_messages = vec![Message::user(&summarization_prompt)];
            match self
                .proxy
                .chat(proxy_messages, Some(summarization_system.clone()))
                .await
            {
                Ok(response) => {
                    log::info!(
                        "Context compressed via proxy (model: {}, tokens: {}/{})",
                        response.model,
                        response.usage.input_tokens,
                        response.usage.output_tokens
                    );
                    if let Some(content) = response.content {
                        return Ok(content);
                    }
                }
                Err(e) => {
                    log::warn!("Proxy unavailable for compression: {}", e);
                }
            }
        }

        // Tier 2: Try direct LLM (Ollama)
        let base_url = self
            .direct_config
            .base_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
        let model = &self.direct_config.model;

        log::info!(
            "Compression fallback to {} ({})",
            self.direct_config.provider,
            model,
        );

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/generate", base_url))
            .json(&serde_json::json!({
                "model": model,
                "prompt": summarization_prompt,
                "stream": false
            }))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await?;
                Ok(body["response"]
                    .as_str()
                    .unwrap_or("[Previous conversation summarized]")
                    .to_string())
            }
            Ok(resp) => {
                log::warn!(
                    "Direct LLM returned error ({}), using fallback summary",
                    resp.status()
                );
                Ok(Self::fallback_summary(&messages))
            }
            Err(e) => {
                log::warn!("Direct LLM unreachable ({}), using fallback summary", e);
                Ok(Self::fallback_summary(&messages))
            }
        }
    }

    /// Simple extractive fallback when no LLM is available.
    fn fallback_summary(messages: &[ChatMessage]) -> String {
        let total = messages.len();
        let recent: Vec<String> = messages
            .iter()
            .rev()
            .take(4)
            .rev()
            .map(|m| format!("{:?}: {}", m.role, &m.content[..m.content.len().min(100)]))
            .collect();
        format!(
            "[Summary of {} messages, recent context:]\n{}",
            total,
            recent.join("\n")
        )
    }
}

/// Build proxy messages from session messages, optionally prepending a summary.
///
/// If a summary exists, it is injected as a user+assistant pair at the start
/// to maintain the alternating message pattern required by the Anthropic API.
fn build_proxy_messages(messages: &[ChatMessage], summary: Option<&str>) -> Vec<Message> {
    let mut proxy_messages: Vec<Message> = Vec::new();

    // Inject summary as context if it exists
    if let Some(summary) = summary {
        proxy_messages.push(Message::user(format!(
            "[Previous conversation summary]: {}",
            summary
        )));
        proxy_messages.push(Message::assistant(
            "Understood, I have the context from our previous conversation.",
        ));
    }

    // Add current messages
    for m in messages {
        proxy_messages.push(match m.role {
            MessageRole::User => Message::user(&m.content),
            MessageRole::Assistant => Message::assistant(&m.content),
            _ => Message::user(&m.content),
        });
    }

    proxy_messages
}

/// The main tool-calling agent loop.
pub struct ToolCallingLoop {
    config: ToolCallingConfig,
    router: HybridLlmRouter,
    guard: ExecutionGuard,
    tools: Arc<ToolRegistry>,
    markdown_commands: Arc<MarkdownCommandRuntime>,
    sessions: Arc<Mutex<SessionManager>>,
    system_prompt: String,
    shutdown: CancellationToken,
}

impl ToolCallingLoop {
    /// Create a new tool-calling loop.
    #[allow(dead_code)]
    pub fn new(
        agent_config: &AgentConfig,
        router: HybridLlmRouter,
        tools: Arc<ToolRegistry>,
        sessions: SessionManager,
        system_prompt: String,
    ) -> Self {
        Self::new_with_shared_sessions(
            agent_config,
            router,
            tools,
            Arc::new(Mutex::new(sessions)),
            system_prompt,
        )
    }

    /// Create a new tool-calling loop using an externally shared session manager.
    pub fn new_with_shared_sessions(
        agent_config: &AgentConfig,
        router: HybridLlmRouter,
        tools: Arc<ToolRegistry>,
        sessions: Arc<Mutex<SessionManager>>,
        system_prompt: String,
    ) -> Self {
        Self {
            config: ToolCallingConfig {
                max_iterations: agent_config.max_iterations,
                max_session_messages: agent_config.max_session_messages,
                keep_last_messages: ToolCallingConfig::default().keep_last_messages,
            },
            router,
            guard: ExecutionGuard::new(),
            tools,
            markdown_commands: Arc::new(MarkdownCommandRuntime::default()),
            sessions,
            system_prompt,
            shutdown: CancellationToken::new(),
        }
    }

    /// Attach markdown command runtime for slash command dispatch.
    pub fn with_markdown_commands(
        mut self,
        markdown_commands: Arc<MarkdownCommandRuntime>,
    ) -> Self {
        self.markdown_commands = markdown_commands;
        self
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
        // Handle /reset with real state clearing and persistence.
        if msg.content.trim() == "/reset" {
            let session_key = msg.session_key();
            let mut sessions_guard = self.sessions.lock().await;
            sessions_guard.reset_session(&session_key)?;
            drop(sessions_guard);

            outbound_tx
                .send(OutboundMessage::new(
                    &msg.channel,
                    &msg.chat_id,
                    "Session reset. Your next message will start fresh.".to_string(),
                ))
                .await?;
            return Ok(());
        }

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

        // Save session before releasing lock
        let session_clone = session.clone();
        let message_count = session.messages.len();
        sessions_guard.save(&session_clone)?;
        drop(sessions_guard);

        // Check if we need compression using configured session cap.
        let needs_compress = self.should_compress(message_count);
        if needs_compress {
            // Keep the last N messages, compress the rest
            let keep_count = self.config.keep_last_messages;

            // Re-acquire lock to read messages for compression
            let messages_to_compress = {
                let mut sessions_guard = self.sessions.lock().await;
                let session = sessions_guard.get_or_create(&session_key);
                if session.messages.len() > keep_count {
                    session.messages[..session.messages.len() - keep_count].to_vec()
                } else {
                    session.messages.clone()
                }
            };

            let summary = self
                .router
                .compress(messages_to_compress, self.system_prompt.clone())
                .await?;

            // Re-acquire lock to update session
            let mut sessions_guard = self.sessions.lock().await;
            let session = sessions_guard.get_or_create(&session_key);
            session.set_summary(summary);
            // Keep only the recent messages
            let recent: Vec<_> = session
                .messages
                .iter()
                .rev()
                .take(keep_count)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            session.messages = recent;
            let session_clone = session.clone();
            sessions_guard.save(&session_clone)?;
            drop(sessions_guard);
        }

        // Build proxy messages from CURRENT session state (post-compression)
        let proxy_messages = {
            let mut sessions_guard = self.sessions.lock().await;
            let session = sessions_guard.get_or_create(&session_key);
            build_proxy_messages(&session.messages, session.summary.as_deref())
        };

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
            self.run_tool_loop(
                proxy_messages,
                tool_definitions,
                &session_key,
                &msg.channel,
                &msg.chat_id,
            )
            .await?
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

    fn should_compress(&self, message_count: usize) -> bool {
        message_count > self.config.max_session_messages
    }

    /// Run the iterative tool-calling loop.
    async fn run_tool_loop(
        &self,
        mut messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        requester_session_key: &str,
        requester_channel: &str,
        requester_chat_id: &str,
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

            log::debug!(
                "LLM response (model: {}, reason: {}, tokens: {}/{})",
                response.model,
                response.stop_reason,
                response.usage.input_tokens,
                response.usage.output_tokens
            );

            // Check if there are tool calls
            if response.tool_calls.is_empty() {
                // No tool calls - return the content
                return Ok(response.content.unwrap_or_default());
            }

            // Execute each tool call
            for tool_call in &response.tool_calls {
                let tool_call = self.with_session_tool_context(
                    tool_call,
                    requester_session_key,
                    requester_channel,
                    requester_chat_id,
                );
                log::info!("Executing tool: {}", tool_call.name);

                // Check with execution guard
                let decision = self.guard.evaluate(&tool_call.name, &tool_call.arguments);

                let tool_result = match decision {
                    GuardDecision::Allow => match self.tools.execute(&tool_call).await {
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
                        match self.tools.execute(&tool_call).await {
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
            // Reset side effects are handled in process_message.
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
            let mut help =
                "Available commands:\n/reset - Clear session\n/help - Show this help".to_string();
            let custom_commands = self
                .markdown_commands
                .help_lines()
                .into_iter()
                .filter(|line| {
                    let lower = line.to_ascii_lowercase();
                    !lower.starts_with("/help")
                        && !lower.starts_with("/reset")
                        && !lower.starts_with("/role")
                })
                .collect::<Vec<_>>();
            if !custom_commands.is_empty() {
                help.push_str("\n\nCustom commands:\n");
                help.push_str(&custom_commands.join("\n"));
            }

            Some(OutboundMessage::new(&msg.channel, &msg.chat_id, help))
        } else {
            self.markdown_commands
                .dispatch_from_slash_message(content)
                .map(|markdown_content| {
                    OutboundMessage::new(&msg.channel, &msg.chat_id, markdown_content)
                })
        }
    }

    fn with_session_tool_context(
        &self,
        call: &ToolCall,
        requester_session_key: &str,
        requester_channel: &str,
        requester_chat_id: &str,
    ) -> ToolCall {
        if !matches!(
            call.name.as_str(),
            "sessions_list" | "sessions_history" | "sessions_send"
        ) {
            return call.clone();
        }

        let mut arguments = match call.arguments.clone() {
            serde_json::Value::Object(map) => map,
            other => {
                let mut map = serde_json::Map::new();
                map.insert("input".to_string(), other);
                map
            }
        };

        arguments
            .entry("requester_session_key".to_string())
            .or_insert_with(|| serde_json::Value::String(requester_session_key.to_string()));
        arguments
            .entry("requester_channel".to_string())
            .or_insert_with(|| serde_json::Value::String(requester_channel.to_string()));
        arguments
            .entry("requester_chat_id".to_string())
            .or_insert_with(|| serde_json::Value::String(requester_chat_id.to_string()));

        ToolCall {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: serde_json::Value::Object(arguments),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_tools_available_no_auto_reset() {
        let router = create_test_router();
        // Simulate a tool call failure by setting flag to false
        router.tools_available.store(false, Ordering::SeqCst);
        // The getter should NOT auto-reset
        assert!(!router.tools_available());
        // Flag should still be false
        assert!(!router.tools_available.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_text_only_fallback() {
        let proxy_config = ProxyClientConfig {
            base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };
        let direct_config = DirectLlmConfig {
            base_url: Some("http://127.0.0.1:1".to_string()),
            ..Default::default()
        };
        let router = HybridLlmRouter::new(proxy_config, direct_config);
        let messages = vec![Message::user("Hello")];

        let response = router.text_only(messages, None).await.unwrap();
        assert!(
            response.contains("unavailable"),
            "Expected unavailable message when both proxy and direct LLM are unreachable, got: {}",
            response
        );
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

    #[tokio::test]
    async fn test_reset_clears_session_state() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = SessionManager::new(temp_dir.path().to_path_buf());
        let tools = Arc::new(ToolRegistry::new());
        let router = create_test_router();

        let loop_config = AgentConfig {
            max_iterations: 10,
            max_session_messages: 20,
            ..Default::default()
        };

        let agent = ToolCallingLoop::new(
            &loop_config,
            router,
            tools,
            sessions,
            "Test system prompt".to_string(),
        );

        let session_key = "cli:chat1";
        {
            let mut sessions_guard = agent.sessions.lock().await;
            let session = sessions_guard.get_or_create(session_key);
            session.add_message(ChatMessage::user("hello", "user1"));
            session.set_summary("old summary".to_string());
            let snapshot = session.clone();
            sessions_guard.save(&snapshot).unwrap();
        }

        let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::channel(1);
        let reset_msg = InboundMessage::new("cli", "user1", "chat1", "/reset");
        agent
            .process_message(reset_msg, &outbound_tx)
            .await
            .unwrap();

        let response = outbound_rx.recv().await.unwrap();
        assert!(response.content.contains("Session reset"));

        {
            let mut sessions_guard = agent.sessions.lock().await;
            let session = sessions_guard.get_or_create(session_key);
            assert_eq!(session.message_count(), 0);
            assert_eq!(session.summary, None);
        }

        let persisted = std::fs::read_to_string(temp_dir.path().join("cli_chat1.jsonl")).unwrap();
        let last_line = persisted.lines().last().unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(last_line).unwrap();
        assert_eq!(snapshot["messages"].as_array().unwrap().len(), 0);
        assert!(snapshot["summary"].is_null());
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

    #[test]
    fn test_slash_command_dispatches_markdown_command() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join("commands");
        std::fs::create_dir_all(&commands_dir).unwrap();
        std::fs::write(
            commands_dir.join("greet.md"),
            r#"---
name: greet
usage: /greet <name>
parameters:
  - name: name
    required: true
---
Hello {name}
"#,
        )
        .unwrap();

        let markdown_commands =
            Arc::new(MarkdownCommandRuntime::load_from_dir(&commands_dir).unwrap());

        let sessions = SessionManager::new(temp_dir.path().to_path_buf());
        let tools = Arc::new(ToolRegistry::new());
        let router = create_test_router();
        let loop_config = AgentConfig {
            max_iterations: 10,
            ..Default::default()
        };

        let agent = ToolCallingLoop::new(&loop_config, router, tools, sessions, "Test".to_string())
            .with_markdown_commands(markdown_commands);

        let msg = InboundMessage::new("cli", "user1", "chat1", "/greet Alice");
        let response = agent.handle_slash_command(&msg).unwrap();

        assert!(response.content.contains("Hello Alice"));
        assert!(response.content.contains("Usage: /greet <name>"));
    }

    #[test]
    fn test_builtins_take_priority_over_markdown_commands() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join("commands");
        std::fs::create_dir_all(&commands_dir).unwrap();
        std::fs::write(
            commands_dir.join("help.md"),
            r#"---
name: help
usage: /help
---
This custom help should not run.
"#,
        )
        .unwrap();

        let markdown_commands =
            Arc::new(MarkdownCommandRuntime::load_from_dir(&commands_dir).unwrap());

        let sessions = SessionManager::new(temp_dir.path().to_path_buf());
        let tools = Arc::new(ToolRegistry::new());
        let router = create_test_router();
        let loop_config = AgentConfig {
            max_iterations: 10,
            ..Default::default()
        };

        let agent = ToolCallingLoop::new(&loop_config, router, tools, sessions, "Test".to_string())
            .with_markdown_commands(markdown_commands);

        let msg = InboundMessage::new("cli", "user1", "chat1", "/help");
        let response = agent.handle_slash_command(&msg).unwrap();

        assert!(response.content.contains("Available commands"));
        assert!(
            !response
                .content
                .contains("This custom help should not run.")
        );
    }

    #[tokio::test]
    async fn test_compress_fallback_to_extractive() {
        // Both proxy and Ollama unreachable (port 1 is unreachable)
        let proxy_config = ProxyClientConfig {
            base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };
        let direct_config = DirectLlmConfig {
            base_url: Some("http://127.0.0.1:1".to_string()),
            ..Default::default()
        };
        let router = HybridLlmRouter::new(proxy_config, direct_config);

        let messages = vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Hello there".to_string(),
                sender_id: None,
                timestamp: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
            },
            ChatMessage {
                role: MessageRole::Assistant,
                content: "Hi! How can I help?".to_string(),
                sender_id: None,
                timestamp: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
            },
        ];

        let result = router.compress(messages, "system".to_string()).await;
        assert!(
            result.is_ok(),
            "compress should never fail, got: {:?}",
            result
        );
        let summary = result.unwrap();
        assert!(
            summary.contains("2 messages"),
            "Expected extractive summary, got: {}",
            summary
        );
    }

    #[test]
    fn test_compression_trigger_uses_max_session_messages() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = SessionManager::new(temp_dir.path().to_path_buf());
        let tools = Arc::new(ToolRegistry::new());
        let router = create_test_router();

        let loop_config = AgentConfig {
            max_iterations: 10,
            max_session_messages: 6,
            ..Default::default()
        };

        let agent = ToolCallingLoop::new(&loop_config, router, tools, sessions, "Test".to_string());

        assert!(!agent.should_compress(6));
        assert!(agent.should_compress(7));
        assert_eq!(
            agent.config.keep_last_messages,
            ToolCallingConfig::default().keep_last_messages
        );
    }

    #[test]
    fn test_build_proxy_messages_with_summary() {
        let messages = vec![ChatMessage {
            role: MessageRole::User,
            content: "What was the decision?".to_string(),
            sender_id: None,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }];

        let result = build_proxy_messages(&messages, Some("We decided to use Rust."));
        // Summary user message + assistant ack + 1 user message = 3
        assert_eq!(result.len(), 3);
        assert!(result[0].content.contains("We decided to use Rust."));
        assert_eq!(result[0].role, "user");
        assert_eq!(result[1].role, "assistant");
        assert_eq!(result[2].content, "What was the decision?");
    }

    #[test]
    fn test_build_proxy_messages_without_summary() {
        let messages = vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                sender_id: None,
                timestamp: chrono::Utc::now(),
                metadata: HashMap::new(),
            },
            ChatMessage {
                role: MessageRole::Assistant,
                content: "Hi!".to_string(),
                sender_id: None,
                timestamp: chrono::Utc::now(),
                metadata: HashMap::new(),
            },
        ];

        let result = build_proxy_messages(&messages, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "Hello");
        assert_eq!(result[1].content, "Hi!");
    }

    #[test]
    fn test_session_tool_context_injected() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = SessionManager::new(temp_dir.path().to_path_buf());
        let tools = Arc::new(ToolRegistry::new());
        let router = create_test_router();
        let loop_config = AgentConfig::default();

        let agent = ToolCallingLoop::new(&loop_config, router, tools, sessions, "Test".to_string());

        let call = ToolCall {
            id: "call-1".to_string(),
            name: "sessions_history".to_string(),
            arguments: serde_json::json!({"session_key": "cli:target"}),
        };

        let scoped = agent.with_session_tool_context(&call, "cli:source", "cli", "source");
        assert_eq!(scoped.arguments["requester_session_key"], "cli:source");
        assert_eq!(scoped.arguments["requester_channel"], "cli");
        assert_eq!(scoped.arguments["requester_chat_id"], "source");
    }
}
