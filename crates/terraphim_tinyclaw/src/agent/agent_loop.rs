//! Tool-calling loop with hybrid LLM routing.

use crate::agent::execution_guard::{ExecutionGuard, GuardDecision};
use crate::agent::proxy_client::{
    Message, ProxyClient, ProxyClientConfig, ProxyResponse, ToolDefinition,
};
use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::commands::CommandRegistry;
use crate::config::{AgentConfig, DirectLlmConfig};
use crate::credentials::{CredentialPool, CredentialSource, EnvVarSource, ProviderId};
use crate::memory::{SharedBackend, jsonl::JsonlBackend};
use crate::session::{ChatMessage, MessageRole, SessionManager};
use crate::tools::agent_memory::{
    AgentMemoryConfig, capture_failed_command, run_agent, should_ignore_command,
};
use crate::tools::{ToolError, ToolRegistry};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Minimum interval between `memory apply` subprocess runs. Prevents a
/// subprocess spawn + full-store scan on every single turn (PR review P2).
const MEMORY_APPLY_COOLDOWN: Duration = Duration::from_secs(30);

/// Configuration for the tool-calling loop.
#[derive(Debug, Clone)]
pub struct ToolCallingConfig {
    /// Maximum tool-calling iterations per message.
    pub max_iterations: usize,
    /// Number of messages to keep after summarization.
    pub keep_last_messages: usize,
}

impl Default for ToolCallingConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
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
    /// Reusable HTTP client for direct LLM calls (connection pooling).
    direct_http: reqwest::Client,
    /// Whether tools are currently available.
    tools_available: AtomicBool,
    /// Optional credential pool. When present and enabled, the router
    /// acquires a live token before each proxy request.
    credential_pool: Option<Arc<CredentialPool>>,
    /// Synchronous source used to resolve TokenRefs.
    credential_source: Arc<dyn CredentialSource>,
    /// Provider class to acquire (e.g. "openrouter"). Mirrors Hermes'
    /// `provider_class` config field.
    credential_class: String,
}

impl HybridLlmRouter {
    /// Create a new hybrid router without credential pooling.
    pub fn new(proxy_config: ProxyClientConfig, direct_config: DirectLlmConfig) -> Self {
        Self::with_credential_pool_inner(
            proxy_config,
            direct_config,
            None,
            Arc::new(EnvVarSource::new()),
            String::new(),
        )
    }

    /// Create a new hybrid router with credential-pool support.
    pub fn with_credential_pool(
        proxy_config: ProxyClientConfig,
        direct_config: DirectLlmConfig,
        pool: Arc<CredentialPool>,
        credential_class: impl Into<String>,
        credential_source: Option<Arc<dyn CredentialSource>>,
    ) -> Self {
        Self::with_credential_pool_inner(
            proxy_config,
            direct_config,
            Some(pool),
            credential_source.unwrap_or_else(|| Arc::new(EnvVarSource::new())),
            credential_class.into(),
        )
    }

    fn with_credential_pool_inner(
        proxy_config: ProxyClientConfig,
        direct_config: DirectLlmConfig,
        credential_pool: Option<Arc<CredentialPool>>,
        credential_source: Arc<dyn CredentialSource>,
        credential_class: String,
    ) -> Self {
        let proxy = ProxyClient::new(proxy_config);
        let direct_http = reqwest::Client::new();

        Self {
            proxy,
            direct_config,
            direct_http,
            tools_available: AtomicBool::new(true),
            credential_pool,
            credential_source,
            credential_class,
        }
    }

    /// Default Ollama base URL.
    const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434";

    /// Resolve the API key to use for the next proxy request.
    ///
    /// If the credential pool is enabled and yields a token, use it and
    /// remember the provider id so we can report success/throttle later.
    /// Otherwise fall back to the static `proxy.api_key`.
    fn acquire_proxy_token(&self) -> (String, Option<ProviderId>) {
        if let Some(pool) = &self.credential_pool
            && !self.credential_class.is_empty()
        {
            match pool.acquire(&self.credential_class, self.credential_source.as_ref()) {
                Ok(cred) => {
                    let provider = cred.provider.clone();
                    return (cred.token, Some(provider));
                }
                Err(e) => {
                    log::warn!(
                        "Credential pool exhausted ({}); falling back to proxy.api_key",
                        e
                    );
                }
            }
        }
        (self.proxy.api_key().to_string(), None)
    }

    /// Call the direct LLM (Ollama) with a prompt.
    /// Returns the response text, or an error if the call fails.
    async fn ollama_generate(&self, prompt: &str) -> Result<String, reqwest::Error> {
        let base_url = self
            .direct_config
            .base_url
            .as_deref()
            .unwrap_or(Self::DEFAULT_OLLAMA_URL);

        let resp = self
            .direct_http
            .post(format!("{}/api/generate", base_url))
            .json(&serde_json::json!({
                "model": &self.direct_config.model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await?;

        resp.json::<serde_json::Value>()
            .await
            .map(|body| body["response"].as_str().unwrap_or("").to_string())
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

        let (token, provider) = self.acquire_proxy_token();

        match self
            .proxy
            .chat_with_tools_and_token(&token, messages, system, tools)
            .await
        {
            Ok(response) => {
                self.tools_available.store(true, Ordering::SeqCst);
                if let Some(p) = provider
                    && let Some(pool) = &self.credential_pool
                {
                    pool.report_success(&p);
                }
                Ok(response)
            }
            Err(e) => {
                self.tools_available.store(false, Ordering::SeqCst);
                if let Some(p) = provider
                    && let Some(pool) = &self.credential_pool
                {
                    pool.report_throttle(&p, None);
                }
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
            let (token, provider) = self.acquire_proxy_token();
            match self
                .proxy
                .chat_with_token(&token, messages.clone(), system.clone())
                .await
            {
                Ok(response) => {
                    if let Some(p) = provider
                        && let Some(pool) = &self.credential_pool
                    {
                        pool.report_success(&p);
                    }
                    return Ok(response.content.unwrap_or_else(|| {
                        "Tools are currently unavailable, answering from knowledge only."
                            .to_string()
                    }));
                }
                Err(e) => {
                    if let Some(p) = provider
                        && let Some(pool) = &self.credential_pool
                    {
                        pool.report_throttle(&p, None);
                    }
                    log::warn!("Proxy unavailable for text response: {}", e);
                }
            }
        }

        // Try direct LLM (Ollama)
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

        match self.ollama_generate(&prompt).await {
            Ok(text) if !text.is_empty() => Ok(text),
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
            let (token, provider) = self.acquire_proxy_token();
            match self
                .proxy
                .chat_with_token(&token, proxy_messages, Some(summarization_system.clone()))
                .await
            {
                Ok(response) => {
                    if let Some(p) = provider
                        && let Some(pool) = &self.credential_pool
                    {
                        pool.report_success(&p);
                    }
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
                    if let Some(p) = provider
                        && let Some(pool) = &self.credential_pool
                    {
                        pool.report_throttle(&p, None);
                    }
                    log::warn!("Proxy unavailable for compression: {}", e);
                }
            }
        }

        // Tier 2: Try direct LLM (Ollama)
        log::info!(
            "Compression fallback to {} ({})",
            self.direct_config.provider,
            self.direct_config.model,
        );

        match self.ollama_generate(&summarization_prompt).await {
            Ok(text) if !text.is_empty() => Ok(text),
            Ok(_) => {
                log::warn!("Direct LLM returned empty response, using fallback summary");
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

/// Build user message content augmented with media URLs.
///
/// When media URLs are present, appends instructions for the LLM to invoke
/// the voice_transcribe tool. Returns content unchanged when no media is present.
fn build_media_augmented_content(content: &str, media: &[String]) -> String {
    if media.is_empty() {
        return content.to_string();
    }

    let mut augmented = content.to_string();
    for url in media {
        augmented.push_str(&format!(
            "\n\nIMPORTANT: The user sent an audio file at URL: {}\n\
             You MUST call the voice_transcribe tool with this URL as the \"audio_url\" parameter. \
             Do NOT say you cannot process audio. After transcription, respond based on the text.",
            url
        ));
    }
    augmented
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
    /// Session memory backend (#3227, T4). All session reads/writes in
    /// the loop go through this trait object so the storage backend is
    /// swappable (jsonl files by default; sqlite via `with_backend`).
    backend: SharedBackend,
    commands: Arc<Mutex<CommandRegistry>>,
    system_prompt: String,
    shutdown: CancellationToken,
    /// Whether agent memory injection is enabled.
    memory_enabled: bool,
    /// Shared config for the agent-memory subprocess bridge.
    memory_config: Option<Arc<AgentMemoryConfig>>,
    /// Agent workspace directory. Failed-command learnings are captured
    /// with this as the subprocess working directory so the learning
    /// lands in `<workspace>/.terraphim/learnings` (#3225).
    workspace: std::path::PathBuf,
    /// Last time `memory apply` ran (cooldown guard — avoids a subprocess
    /// spawn on every single turn).
    memory_last_apply: Arc<Mutex<std::time::Instant>>,
}

impl ToolCallingLoop {
    /// Create a new tool-calling loop.
    ///
    /// The session manager is wrapped in a [`JsonlBackend`] so the loop
    /// persists through the [`crate::memory::MemoryBackend`] trait while
    /// sharing the same manager (mutex + cache + on-disk layout) with the
    /// session tools.
    pub fn new(
        agent_config: &AgentConfig,
        router: HybridLlmRouter,
        tools: Arc<ToolRegistry>,
        sessions: Arc<Mutex<SessionManager>>,
        system_prompt: String,
        memory_config: Option<&crate::config::MemoryConfig>,
    ) -> Self {
        // Initialize command registry with defaults
        let mut commands = CommandRegistry::with_defaults();
        // Load commands from search paths (best effort)
        let _ = commands.load_all();

        Self::with_commands(
            agent_config,
            router,
            tools,
            sessions,
            system_prompt,
            commands,
            memory_config,
        )
    }

    /// Create with a custom command registry.
    pub fn with_commands(
        agent_config: &AgentConfig,
        router: HybridLlmRouter,
        tools: Arc<ToolRegistry>,
        sessions: Arc<Mutex<SessionManager>>,
        system_prompt: String,
        commands: CommandRegistry,
        memory_config: Option<&crate::config::MemoryConfig>,
    ) -> Self {
        Self::with_backend_and_commands(
            agent_config,
            router,
            tools,
            Arc::new(JsonlBackend::from_shared(sessions)),
            system_prompt,
            commands,
            memory_config,
        )
    }

    /// Create with an explicit memory backend and default commands.
    ///
    /// Use this to select a non-default backend (e.g. `SqliteBackend`)
    /// from configuration. The default [`Self::new`] path preserves the
    /// legacy jsonl on-disk layout.
    pub fn with_backend(
        agent_config: &AgentConfig,
        router: HybridLlmRouter,
        tools: Arc<ToolRegistry>,
        backend: SharedBackend,
        system_prompt: String,
        memory_config: Option<&crate::config::MemoryConfig>,
    ) -> Self {
        let mut commands = CommandRegistry::with_defaults();
        let _ = commands.load_all();
        Self::with_backend_and_commands(
            agent_config,
            router,
            tools,
            backend,
            system_prompt,
            commands,
            memory_config,
        )
    }

    /// Create with an explicit memory backend and command registry.
    pub fn with_backend_and_commands(
        agent_config: &AgentConfig,
        router: HybridLlmRouter,
        tools: Arc<ToolRegistry>,
        backend: SharedBackend,
        system_prompt: String,
        commands: CommandRegistry,
        memory_config: Option<&crate::config::MemoryConfig>,
    ) -> Self {
        let (memory_enabled, mem_cfg_arc) = match memory_config {
            Some(cfg) if cfg.enabled => (true, Some(Arc::new(AgentMemoryConfig::from(cfg)))),
            _ => (false, None),
        };

        Self {
            config: ToolCallingConfig {
                max_iterations: agent_config.max_iterations,
                ..Default::default()
            },
            router,
            guard: ExecutionGuard::new(),
            tools,
            backend,
            commands: Arc::new(Mutex::new(commands)),
            system_prompt,
            shutdown: CancellationToken::new(),
            memory_enabled,
            memory_config: mem_cfg_arc,
            workspace: agent_config.workspace.clone(),
            memory_last_apply: Arc::new(Mutex::new(std::time::Instant::now())),
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

            if let Some(msg) = msg
                && let Err(e) = self.process_message(msg, &outbound_tx).await
            {
                log::error!("Error processing message: {}", e);
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
        // Handle /reset command specially - it needs to clear the session
        if msg.content.trim() == "/reset" {
            let session_key = msg.session_key();
            // Get session, clear it, then persist
            let mut session = self.backend.get_or_create(&session_key).await;
            session.clear();
            self.backend.persist(&session).await?;

            let response = OutboundMessage::new(
                &msg.channel,
                &msg.chat_id,
                "Session reset. Your next message will start fresh.".to_string(),
            );
            outbound_tx.send(response).await?;
            return Ok(());
        }

        // Check if this is another slash command
        if let Some(response) = self.handle_slash_command(&msg).await {
            outbound_tx.send(response).await?;
            return Ok(());
        }

        // Get or create session
        let session_key = msg.session_key();
        let mut session = self.backend.get_or_create(&session_key).await;

        // Add user message to session (augmented with media context if present)
        let user_msg = ChatMessage {
            role: MessageRole::User,
            content: build_media_augmented_content(&msg.content, &msg.media),
            sender_id: Some(msg.sender_id.clone()),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        session.add_message(user_msg.clone());

        // Persist session with the new user message
        let message_count = session.messages.len();
        self.backend.persist(&session).await?;

        // Check if we need compression using configured ratio
        let needs_compress = message_count > self.config.keep_last_messages * 2;
        if needs_compress {
            // Keep the last N messages, compress the rest
            let keep_count = self.config.keep_last_messages;

            // Reload the session to read messages for compression
            let messages_to_compress = {
                let session = self.backend.get_or_create(&session_key).await;
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

            // T4 (#3227): write the summary back to the agent-memory
            // bridge so compression stops being lossy-and-lost. Fail-open:
            // the session file remains the authoritative record.
            self.capture_compression_summary(&session_key, &summary)
                .await;

            // Reload the session to record the summary and trim messages
            let mut session = self.backend.get_or_create(&session_key).await;
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
            self.backend.persist(&session).await?;
        }

        // Build proxy messages from CURRENT session state (post-compression)
        let proxy_messages = {
            let session = self.backend.get_or_create(&session_key).await;
            build_proxy_messages(&session.messages, session.summary.as_deref())
        };

        // Memory context injection: prepend retrieved memories to system prompt.
        // Cooldown: `memory apply` spawns a subprocess — run it at most once
        // per MEMORY_APPLY_COOLDOWN to avoid per-turn latency (PR review P2).
        let effective_system_prompt = if self.memory_enabled {
            let mut last = self.memory_last_apply.lock().await;
            let within_cooldown = last.elapsed() < MEMORY_APPLY_COOLDOWN;
            if !within_cooldown {
                if let Some(ref mem_config) = self.memory_config {
                    // Extract the user's latest message as the query.
                    let query = proxy_messages
                        .iter()
                        .rev()
                        .find(|m| m.role == "user")
                        .map(|m| m.content.as_str())
                        .unwrap_or("");

                    let result =
                        run_agent(mem_config, &["memory", "apply", "--prompt", query], None).await;
                    match result {
                        Ok(context) if !context.trim().is_empty() => {
                            // Token-budget guard: truncate to max_context_chars,
                            // walking back to a UTF-8 char boundary so non-ASCII
                            // content (emoji, Cyrillic, CJK) can't panic the slice.
                            let truncated = if context.len() > mem_config.max_context_chars {
                                let mut end = mem_config.max_context_chars;
                                while end > 0 && !context.is_char_boundary(end) {
                                    end -= 1;
                                }
                                &context[..end]
                            } else {
                                &context
                            };
                            *last = std::time::Instant::now();
                            format!("{}\n\n## Memory Context\n{}", self.system_prompt, truncated)
                        }
                        Ok(_) => self.system_prompt.clone(),
                        Err(e) => {
                            log::warn!("Memory apply failed (non-fatal): {}", e);
                            self.system_prompt.clone()
                        }
                    }
                } else {
                    self.system_prompt.clone()
                }
            } else {
                self.system_prompt.clone()
            }
        } else {
            self.system_prompt.clone()
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
            self.run_tool_loop_with_prompt(
                proxy_messages,
                tool_definitions,
                &effective_system_prompt,
            )
            .await?
        } else {
            // Fallback to text-only mode
            self.router
                .text_only(proxy_messages, Some(effective_system_prompt))
                .await?
        };

        // Add assistant response to session
        let mut session = self.backend.get_or_create(&session_key).await;

        let assistant_msg = ChatMessage {
            role: MessageRole::Assistant,
            content: final_response.clone(),
            sender_id: None,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        session.add_message(assistant_msg.clone());

        // Persist session with the assistant response
        self.backend.persist(&session).await?;

        // Send response
        let outbound = OutboundMessage::new(&msg.channel, &msg.chat_id, final_response);
        outbound_tx.send(outbound).await?;

        Ok(())
    }

    /// Write a compression summary back to the agent-memory bridge
    /// (`terraphim-agent memory capture`) with a provenance tag
    /// (`session-compression:<session_key>`), so the knowledge condensed
    /// out of the trimmed messages remains retrievable (#3227, T4).
    ///
    /// Fail-open and gated on `memory.enabled`: any bridge error is
    /// logged and swallowed — the session file stays the authoritative
    /// record. No-op when the memory bridge is disabled.
    async fn capture_compression_summary(&self, session_key: &str, summary: &str) {
        if !self.memory_enabled {
            return;
        }
        let Some(ref mem_config) = self.memory_config else {
            return;
        };

        let tag = format!("session-compression:{session_key}");
        let stdin_json = serde_json::json!({
            "content": summary,
            "item_type": "Experience",
            "importance": "Medium",
        })
        .to_string();

        let mut cli_args = vec!["memory", "capture", "--provenance-tag", tag.as_str()];
        let role_owned;
        if let Some(ref role) = mem_config.role {
            role_owned = role.clone();
            cli_args.push("--role");
            cli_args.push(&role_owned);
        }

        match run_agent(mem_config, &cli_args, Some(&stdin_json)).await {
            Ok(_) => log::info!("Compression summary captured to memory bridge (tag: {tag})"),
            Err(e) => {
                log::warn!("Memory capture of compression summary failed (non-fatal): {e}")
            }
        }
    }

    /// Invariant failure capture (#3225): when an exec-class tool call
    /// fails, capture the failed command as a learning via
    /// `terraphim-agent learn capture` — no model involvement required.
    ///
    /// Semantics mirror terraphim-agent's PostToolUse hook:
    /// - gated on `memory.enabled` (the memory bridge master switch);
    /// - fail-open: a capture failure is a `warn` log, never an error
    ///   surfaced to the turn;
    /// - test-runner commands matching the ignore globs
    ///   (`cargo test*`, `npm test*`, `pytest*`, `yarn test*`) are
    ///   skipped client-side;
    /// - secret redaction is delegated to `terraphim-agent`, which
    ///   redacts before persisting;
    /// - the subprocess timeout and 1 MiB output guard are enforced by
    ///   the shared bridge in `tools::agent_memory`.
    ///
    /// `Blocked` errors are not captured: the command never ran, so
    /// there is no failure to learn from.
    async fn capture_tool_failure(&self, tool_call: &crate::tools::ToolCall, error: &ToolError) {
        if !self.memory_enabled {
            return;
        }
        let Some(ref mem_config) = self.memory_config else {
            return;
        };

        // Exec-class tools only: a failed shell command is the learning
        // signal. Other tools (web, filesystem, …) are out of scope.
        if !matches!(
            tool_call.name.as_str(),
            "shell" | "exec" | "bash" | "sandbox"
        ) {
            return;
        }

        // A blocked command never executed; guard rejections are policy,
        // not command failures.
        if matches!(error, ToolError::Blocked { .. }) {
            return;
        }

        let Some(command) = tool_call.arguments["command"].as_str() else {
            return;
        };

        if should_ignore_command(command) {
            log::debug!("Skipping learning capture for ignored command: {command}");
            return;
        }

        let (exit_code, error_output) = match error {
            ToolError::NonZeroExit {
                exit_code, stderr, ..
            } => (i64::from(*exit_code), stderr.clone()),
            // Conventional timeout exit code (cf. GNU timeout(1)).
            ToolError::Timeout { .. } => (124, error.to_string()),
            other => (1, other.to_string()),
        };

        match capture_failed_command(
            mem_config,
            command,
            &error_output,
            exit_code,
            Some(&self.workspace),
        )
        .await
        {
            Ok(_) => log::info!("Captured failed command as learning: {command}"),
            Err(e) => log::warn!("Learning capture failed (non-fatal): {e}"),
        }
    }

    /// Run the iterative tool-calling loop with explicit system prompt.
    async fn run_tool_loop_with_prompt(
        &self,
        mut messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        system_prompt: &str,
    ) -> anyhow::Result<String> {
        let prompt = system_prompt.to_string();
        for iteration in 0..self.config.max_iterations {
            log::debug!("Tool-calling iteration {}", iteration + 1);

            // Call LLM with tools
            let response = match self
                .router
                .tool_call(messages.clone(), Some(prompt.clone()), tools.clone())
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    log::warn!("Tool call failed: {}. Falling back to text-only.", e);
                    return self.router.text_only(messages, Some(prompt.clone())).await;
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
                            // #3225: failure capture is a loop invariant.
                            self.capture_tool_failure(tool_call, &e).await;
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
                            Err(e) => {
                                // #3225: failure capture is a loop invariant.
                                self.capture_tool_failure(tool_call, &e).await;
                                format!("Tool execution error: {}", e)
                            }
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

    /// Handle slash commands (except /reset which is handled in process_message).
    async fn handle_slash_command(&self, msg: &InboundMessage) -> Option<OutboundMessage> {
        use crate::commands::CommandRegistry;
        let content = msg.content.trim();

        // Built-in commands first (faster path)
        if content.starts_with("/role ") {
            return Some(OutboundMessage::new(
                &msg.channel,
                &msg.chat_id,
                "Role switching not yet implemented (coming in full implementation)".to_string(),
            ));
        }

        if content == "/help" {
            // Get available markdown commands
            let commands_guard: tokio::sync::MutexGuard<'_, CommandRegistry> =
                self.commands.lock().await;
            let mut help_text =
                "Available commands:\n/reset - Clear session\n/help - Show this help".to_string();

            let commands = commands_guard.list();
            if !commands.is_empty() {
                help_text.push_str("\n\nCustom commands:");
                for cmd in commands {
                    help_text.push_str(&format!("\n/{} - {}", cmd.name, cmd.description));
                }
            }
            drop(commands_guard);

            return Some(OutboundMessage::new(&msg.channel, &msg.chat_id, help_text));
        }

        // Check for markdown commands
        let first_word = content.split_whitespace().next()?;
        if let Some(cmd_name) = first_word.strip_prefix('/') {
            let commands_guard: tokio::sync::MutexGuard<'_, CommandRegistry> =
                self.commands.lock().await;
            if let Some(cmd) = commands_guard.get(cmd_name) {
                // Found a markdown command - return info about it
                let _args: Vec<&str> = content.split_whitespace().skip(1).collect();
                let response = format!(
                    "Command: {}\nDescription: {}\nArguments: {}\n\nTo execute, use: /{} {}",
                    cmd.name,
                    cmd.description,
                    if cmd.arguments.is_empty() {
                        "None".to_string()
                    } else {
                        cmd.arguments
                            .iter()
                            .map(|a| {
                                format!(
                                    "{} ({})",
                                    a.name,
                                    if a.required { "required" } else { "optional" }
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    },
                    cmd.name,
                    cmd.arguments
                        .iter()
                        .filter(|a| a.required)
                        .map(|a| format!("{}=<value>", a.name))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                return Some(OutboundMessage::new(&msg.channel, &msg.chat_id, response));
            }
            drop(commands_guard);
        }

        None
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

    #[tokio::test]
    async fn test_slash_command_reset_returns_none() {
        // /reset is now handled in process_message, not handle_slash_command
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
            Arc::new(Mutex::new(sessions)),
            "Test system prompt".to_string(),
            None,
        );

        let msg = InboundMessage::new("cli", "user1", "chat1", "/reset");
        let response = agent.handle_slash_command(&msg).await;

        // handle_slash_command returns None for /reset since it's handled in process_message
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn test_slash_command_help() {
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
            Arc::new(Mutex::new(sessions)),
            "Test".to_string(),
            None,
        );

        let msg = InboundMessage::new("cli", "user1", "chat1", "/help");
        let response = agent.handle_slash_command(&msg).await;

        assert!(response.is_some());
        assert!(response.unwrap().content.contains("Available commands"));
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
    fn test_media_augmented_content_no_media() {
        let result = build_media_augmented_content("Hello", &[]);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_media_augmented_content_with_url() {
        let media = vec!["https://api.telegram.org/file/bot123/voice.ogg".to_string()];
        let result = build_media_augmented_content("[Voice message]", &media);
        assert!(result.contains("[Voice message]"));
        assert!(result.contains("voice_transcribe"));
        assert!(result.contains("https://api.telegram.org/file/bot123/voice.ogg"));
    }

    #[test]
    fn test_media_augmented_content_multiple_urls() {
        let media = vec![
            "https://example.com/a.ogg".to_string(),
            "https://example.com/b.mp3".to_string(),
        ];
        let result = build_media_augmented_content("", &media);
        assert!(result.contains("a.ogg"));
        assert!(result.contains("b.mp3"));
        // Should have two voice_transcribe instructions
        assert_eq!(result.matches("voice_transcribe").count(), 2);
    }

    // -------------------------------------------------------------------------
    // Credential-pool integration tests for HybridLlmRouter.
    // -------------------------------------------------------------------------

    /// Build a minimal proxy config for router tests.
    fn test_proxy_config(api_key: &str) -> ProxyClientConfig {
        ProxyClientConfig {
            base_url: "http://localhost:9999".to_string(),
            api_key: api_key.to_string(),
            timeout_ms: 1000,
            model: Some("test-model".to_string()),
            retry_after_secs: 1,
        }
    }

    fn test_direct_config() -> DirectLlmConfig {
        DirectLlmConfig {
            provider: "ollama".to_string(),
            model: "llama3.2".to_string(),
            base_url: None,
        }
    }

    #[test]
    fn router_without_pool_uses_static_api_key() {
        let router = HybridLlmRouter::new(test_proxy_config("static-key"), test_direct_config());
        let (token, provider) = router.acquire_proxy_token();
        assert_eq!(token, "static-key");
        assert!(provider.is_none());
    }

    #[test]
    fn router_with_pool_uses_resolved_token() {
        // SAFETY: test-only env mutation under the Wave 0 scrubber convention.
        unsafe {
            std::env::set_var("WAVE1_ROUTER_KEY_A", "token-from-pool");
        }

        let pool = Arc::new(CredentialPool::new());
        pool.add(crate::credentials::PoolEntry {
            provider: crate::credentials::ProviderId::from("openrouter-primary"),
            class: crate::credentials::ProviderClass::from("openrouter"),
            token_ref: crate::credentials::TokenRef::EnvVar {
                name: "WAVE1_ROUTER_KEY_A".into(),
            },
        });

        let router = HybridLlmRouter::with_credential_pool(
            test_proxy_config("static-key"),
            test_direct_config(),
            pool.clone(),
            "openrouter",
            None,
        );

        let (token, provider) = router.acquire_proxy_token();
        assert_eq!(token, "token-from-pool");
        assert_eq!(provider.as_deref(), Some("openrouter-primary"));

        unsafe {
            std::env::remove_var("WAVE1_ROUTER_KEY_A");
        }
    }

    #[test]
    fn router_with_pool_falls_back_when_exhausted() {
        // Empty pool for the requested class → fall back to static key.
        let pool = Arc::new(CredentialPool::new());
        let router = HybridLlmRouter::with_credential_pool(
            test_proxy_config("static-key"),
            test_direct_config(),
            pool,
            "openrouter",
            None,
        );

        let (token, provider) = router.acquire_proxy_token();
        assert_eq!(token, "static-key");
        assert!(provider.is_none());
    }

    #[test]
    fn router_pool_success_and_throttle_update_stats() {
        unsafe {
            std::env::set_var("WAVE1_ROUTER_KEY_B", "token-b");
        }

        let pool = Arc::new(CredentialPool::new());
        pool.add(crate::credentials::PoolEntry {
            provider: crate::credentials::ProviderId::from("openrouter-primary"),
            class: crate::credentials::ProviderClass::from("openrouter"),
            token_ref: crate::credentials::TokenRef::EnvVar {
                name: "WAVE1_ROUTER_KEY_B".into(),
            },
        });

        let router = HybridLlmRouter::with_credential_pool(
            test_proxy_config("static-key"),
            test_direct_config(),
            pool.clone(),
            "openrouter",
            None,
        );

        let (_, provider) = router.acquire_proxy_token();
        let provider = provider.expect("acquired a provider");

        pool.report_success(&provider);
        let stats = pool.stats();
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.throttles, 0);

        pool.report_throttle(&provider, None);
        let stats = pool.stats();
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.throttles, 1);

        unsafe {
            std::env::remove_var("WAVE1_ROUTER_KEY_B");
        }
    }
}
