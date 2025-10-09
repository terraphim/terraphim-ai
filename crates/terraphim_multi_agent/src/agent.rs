//! Core TerraphimAgent implementation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use terraphim_agent_evolution::{VersionedLessons, VersionedMemory, VersionedTaskList};
use terraphim_automata::AutocompleteIndex; // Use actual type from automata
use terraphim_config::Role;
use terraphim_persistence::{DeviceStorage, Persistable};
use terraphim_rolegraph::RoleGraph;

use crate::{
    AgentContext, AgentId, CommandHistory, CommandInput, CommandOutput, CommandRecord, CommandType,
    ContextItem, ContextItemType, CostTracker, GenAiLlmClient, LlmMessage, LlmRequest,
    MultiAgentError, MultiAgentResult, TokenUsageTracker,
};

/// Goals for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentGoals {
    /// Global goal shared across all agents
    pub global_goal: String,
    /// Individual goals specific to this agent
    pub individual_goals: Vec<String>,
    /// Goal alignment score (0.0 - 1.0)
    pub alignment_score: f64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl AgentGoals {
    pub fn new(global_goal: String, individual_goals: Vec<String>) -> Self {
        Self {
            global_goal,
            individual_goals,
            alignment_score: 0.5, // Start neutral
            last_updated: Utc::now(),
        }
    }

    pub fn update_alignment_score(&mut self, score: f64) {
        self.alignment_score = score.clamp(0.0, 1.0);
        self.last_updated = Utc::now();
    }

    pub fn add_individual_goal(&mut self, goal: String) {
        self.individual_goals.push(goal);
        self.last_updated = Utc::now();
    }
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    /// Agent is initializing
    Initializing,
    /// Agent is ready to receive commands
    Ready,
    /// Agent is processing a command
    Busy,
    /// Agent is temporarily paused
    Paused,
    /// Agent encountered an error
    Error(String),
    /// Agent is being shut down
    Terminating,
    /// Agent is offline
    Offline,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum context tokens
    pub max_context_tokens: u64,
    /// Maximum context items
    pub max_context_items: usize,
    /// Command history limit
    pub max_command_history: usize,
    /// Token usage tracking enabled
    pub enable_token_tracking: bool,
    /// Cost tracking enabled
    pub enable_cost_tracking: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval_seconds: u64,
    /// Default LLM timeout in milliseconds
    pub default_timeout_ms: u64,
    /// Quality threshold for learning
    pub quality_threshold: f64,
    /// VM execution configuration
    pub vm_execution: Option<crate::vm_execution::VmExecutionConfig>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 32000,
            max_context_items: 100,
            max_command_history: 1000,
            enable_token_tracking: true,
            enable_cost_tracking: true,
            auto_save_interval_seconds: 300, // 5 minutes
            default_timeout_ms: 30000,       // 30 seconds
            quality_threshold: 0.7,
            vm_execution: None, // Disabled by default
        }
    }
}

/// Core Terraphim Agent that wraps a Role configuration with Rig integration
#[derive(Debug)]
pub struct TerraphimAgent {
    // Core identity
    pub agent_id: AgentId,
    pub role_config: Role,
    pub config: AgentConfig,
    pub status: Arc<RwLock<AgentStatus>>,

    // Knowledge graph context
    pub rolegraph: Arc<RoleGraph>,
    pub automata: Arc<AutocompleteIndex>,

    // Individual evolution tracking
    pub memory: Arc<RwLock<VersionedMemory>>,
    pub tasks: Arc<RwLock<VersionedTaskList>>,
    pub lessons: Arc<RwLock<VersionedLessons>>,

    // Goals and alignment
    pub goals: AgentGoals,

    // Context and history
    pub context: Arc<RwLock<AgentContext>>,
    pub command_history: Arc<RwLock<CommandHistory>>,

    // Tracking
    pub token_tracker: Arc<RwLock<TokenUsageTracker>>,
    pub cost_tracker: Arc<RwLock<CostTracker>>,

    // Persistence
    pub persistence: Arc<DeviceStorage>,

    // LLM Client
    pub llm_client: Arc<GenAiLlmClient>,

    // VM Execution Client (optional)
    pub vm_execution_client: Option<Arc<crate::vm_execution::VmExecutionClient>>,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub last_active: Arc<RwLock<DateTime<Utc>>>,
}

impl Clone for TerraphimAgent {
    fn clone(&self) -> Self {
        Self {
            agent_id: self.agent_id,
            role_config: self.role_config.clone(),
            config: self.config.clone(),
            status: self.status.clone(),
            rolegraph: self.rolegraph.clone(),
            automata: self.automata.clone(),
            memory: self.memory.clone(),
            tasks: self.tasks.clone(),
            lessons: self.lessons.clone(),
            goals: self.goals.clone(),
            context: self.context.clone(),
            command_history: self.command_history.clone(),
            token_tracker: self.token_tracker.clone(),
            cost_tracker: self.cost_tracker.clone(),
            persistence: self.persistence.clone(),
            llm_client: self.llm_client.clone(),
            vm_execution_client: self.vm_execution_client.clone(),
            created_at: self.created_at,
            last_active: self.last_active.clone(),
        }
    }
}

impl TerraphimAgent {
    /// Create a new TerraphimAgent from a Role configuration
    pub async fn new(
        role_config: Role,
        persistence: Arc<DeviceStorage>,
        config: Option<AgentConfig>,
    ) -> MultiAgentResult<Self> {
        let agent_id = AgentId::new_v4();
        let config =
            crate::vm_execution::create_agent_config_with_vm_execution(&role_config, config);

        // Initialize knowledge graph components
        let rolegraph = Arc::new(Self::load_rolegraph(&role_config).await?);
        let automata = Arc::new(Self::load_automata(&role_config).await?);

        // Initialize evolution components
        let memory = Arc::new(RwLock::new(VersionedMemory::new(format!(
            "agent_{}/memory/current",
            agent_id
        ))));
        let tasks = Arc::new(RwLock::new(VersionedTaskList::new(format!(
            "agent_{}/tasks/current",
            agent_id
        ))));
        let lessons = Arc::new(RwLock::new(VersionedLessons::new(format!(
            "agent_{}/lessons/current",
            agent_id
        ))));

        // Initialize goals
        let goals = AgentGoals::new(
            "Build reliable, helpful AI systems".to_string(), // Default global goal
            Self::extract_individual_goals(&role_config),
        );

        // Initialize context and history
        let context = Arc::new(RwLock::new(AgentContext::new(
            agent_id,
            config.max_context_tokens,
            config.max_context_items,
        )));

        let command_history = Arc::new(RwLock::new(CommandHistory::new(
            agent_id,
            config.max_command_history,
        )));

        // Initialize tracking
        let token_tracker = Arc::new(RwLock::new(TokenUsageTracker::new(agent_id)));
        let cost_tracker = Arc::new(RwLock::new(CostTracker::new()));

        // Initialize LLM client using role configuration
        let provider = role_config
            .extra
            .get("llm_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("ollama");
        let model = role_config
            .extra
            .get("llm_model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let base_url = role_config
            .extra
            .get("llm_base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        log::debug!(
            "🤖 TerraphimAgent::new - LLM config: provider={}, model={:?}, base_url={:?}",
            provider,
            model,
            base_url
        );

        let llm_client = Arc::new(GenAiLlmClient::from_config_with_url(
            provider, model, base_url,
        )?);

        // Initialize VM execution client if enabled
        let vm_execution_client = if let Some(vm_config) = &config.vm_execution {
            log::debug!("Initializing VM execution client for agent {}", agent_id);
            Some(Arc::new(crate::vm_execution::VmExecutionClient::new(
                vm_config,
            )))
        } else {
            None
        };

        let now = Utc::now();

        Ok(Self {
            agent_id,
            role_config,
            config,
            status: Arc::new(RwLock::new(AgentStatus::Initializing)),
            rolegraph,
            automata,
            memory,
            tasks,
            lessons,
            goals,
            context,
            command_history,
            token_tracker,
            cost_tracker,
            persistence,
            llm_client,
            vm_execution_client,
            created_at: now,
            last_active: Arc::new(RwLock::new(now)),
        })
    }

    /// Initialize the agent and load any persisted state
    pub async fn initialize(&self) -> MultiAgentResult<()> {
        // Try to load existing state from persistence
        // TODO: Implement proper state loading with interior mutability
        // self.load_state().await?;

        // Set up system context
        self.setup_system_context().await?;

        *self.status.write().await = AgentStatus::Ready;
        *self.last_active.write().await = Utc::now();

        log::info!(
            "Agent {} ({}) initialized successfully",
            self.agent_id,
            self.role_config.name
        );

        Ok(())
    }

    /// Process a command using Rig framework
    pub async fn process_command(&self, input: CommandInput) -> MultiAgentResult<CommandOutput> {
        {
            let status = self.status.read().await;
            if *status != AgentStatus::Ready {
                return Err(MultiAgentError::AgentNotAvailable(self.agent_id));
            }
        }

        *self.status.write().await = AgentStatus::Busy;
        *self.last_active.write().await = Utc::now();

        let start_time = Utc::now();
        let mut command_record = CommandRecord::new(self.agent_id, input.clone());

        // Capture context snapshot
        let context_snapshot = {
            let context = self.context.read().await;
            crate::history::HistoryContextSnapshot::from_context(&context)
        };
        command_record = command_record.with_context_snapshot(context_snapshot);

        let result = match input.command_type {
            CommandType::Generate => self.handle_generate_command(&input).await,
            CommandType::Answer => self.handle_answer_command(&input).await,
            CommandType::Search => self.handle_search_command(&input).await,
            CommandType::Analyze => self.handle_analyze_command(&input).await,
            CommandType::Execute => self.handle_execute_command(&input).await,
            CommandType::Create => self.handle_create_command(&input).await,
            CommandType::Edit => self.handle_edit_command(&input).await,
            CommandType::Review => self.handle_review_command(&input).await,
            CommandType::Plan => self.handle_plan_command(&input).await,
            CommandType::System => self.handle_system_command(&input).await,
            CommandType::Custom(ref cmd_type) => self.handle_custom_command(&input, cmd_type).await,
        };

        let duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;

        match result {
            Ok(output) => {
                command_record = command_record.complete(output.clone(), duration_ms);

                // Update context with the interaction
                self.update_context_with_interaction(&input, &output)
                    .await?;

                // Learn from successful interaction
                self.learn_from_interaction(&command_record).await?;

                *self.status.write().await = AgentStatus::Ready;

                // Add to command history
                {
                    let mut history = self.command_history.write().await;
                    history.add_record(command_record);
                }

                Ok(output)
            }
            Err(error) => {
                let cmd_error = crate::history::CommandError::new(
                    crate::history::ErrorType::Internal,
                    error.to_string(),
                );
                command_record = command_record.with_error(cmd_error);

                *self.status.write().await = AgentStatus::Error(error.to_string());

                // Add failed command to history
                {
                    let mut history = self.command_history.write().await;
                    history.add_record(command_record);
                }

                Err(error)
            }
        }
    }

    /// Get agent capabilities based on role configuration
    pub fn get_capabilities(&self) -> Vec<String> {
        let mut capabilities = Vec::new();

        // Extract capabilities from role configuration
        if !self.role_config.extra.is_empty() {
            if let Some(caps) = self.role_config.extra.get("capabilities") {
                if let Ok(cap_list) = serde_json::from_value::<Vec<String>>(caps.clone()) {
                    capabilities.extend(cap_list);
                }
            }
        }

        // Add default capabilities based on role name and haystacks
        capabilities.push(format!("role_{}", self.role_config.name.as_lowercase()));

        for haystack in &self.role_config.haystacks {
            capabilities.push(format!("haystack_{}", haystack.location.to_lowercase()));
        }

        capabilities
    }

    /// Save current agent state to persistence
    pub async fn save_state(&self) -> MultiAgentResult<()> {
        let state = AgentState {
            agent_id: self.agent_id,
            role_config: self.role_config.clone(),
            config: self.config.clone(),
            goals: self.goals.clone(),
            status: self.status.read().await.clone(),
            created_at: self.created_at,
            last_active: *self.last_active.read().await,
            memory_snapshot: {
                let memory = self.memory.read().await;
                memory.state.clone()
            },
            tasks_snapshot: {
                let tasks = self.tasks.read().await;
                tasks.state.clone()
            },
            lessons_snapshot: {
                let lessons = self.lessons.read().await;
                lessons.state.clone()
            },
        };

        let key = format!("agent_state:{}", self.agent_id);
        let serialized = serde_json::to_vec(&state).map_err(MultiAgentError::SerializationError)?;

        // Use DeviceStorage write method
        self.persistence
            .fastest_op
            .write(&key, serialized)
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        log::debug!("Saved state for agent {}", self.agent_id);
        Ok(())
    }

    /// Load agent state from persistence
    #[allow(dead_code)]
    async fn load_state(&self) -> MultiAgentResult<()> {
        let key = format!("agent_state:{}", self.agent_id);

        match self.persistence.fastest_op.read(&key).await {
            Ok(data) => {
                let state: AgentState = serde_json::from_slice(&data.to_vec())
                    .map_err(MultiAgentError::SerializationError)?;

                // Restore state
                // TODO: Implement proper state loading with interior mutability
                // self.goals = state.goals;
                *self.status.write().await = state.status;
                // self.created_at = state.created_at;
                *self.last_active.write().await = state.last_active;

                // Restore evolution components
                {
                    let _memory = self.memory.write().await;
                    // TODO: Implement state restoration
                    // *memory = VersionedMemory::from_snapshot(state.memory_snapshot);
                }
                {
                    let _tasks = self.tasks.write().await;
                    // TODO: Implement state restoration
                    // *tasks = VersionedTaskList::from_state(state.tasks_snapshot);
                }
                {
                    let _lessons = self.lessons.write().await;
                    // TODO: Implement state restoration
                    // *lessons = VersionedLessons::from_lessons(state.lessons_snapshot);
                }

                log::info!("Loaded existing state for agent {}", self.agent_id);
            }
            Err(ref e) => {
                log::info!(
                    "No existing state found for agent {} ({})",
                    self.agent_id,
                    e
                );
            }
        }

        Ok(())
    }

    /// Set up initial system context
    async fn setup_system_context(&self) -> MultiAgentResult<()> {
        let mut context = self.context.write().await;

        // Add system prompt - use configured prompt if available, otherwise use generic
        let system_prompt =
            if let Some(configured_prompt) = self.role_config.extra.get("llm_system_prompt") {
                configured_prompt.as_str().unwrap_or("").to_string()
            } else {
                format!(
                    "You are {}, a specialized AI agent with the following capabilities: {}. \
                 Your global goal is: {}. Your individual goals are: {}.",
                    self.role_config.name,
                    self.get_capabilities().join(", "),
                    self.goals.global_goal,
                    self.goals.individual_goals.join(", ")
                )
            };

        log::debug!(
            "🎯 Agent {} using system prompt: {}",
            self.role_config.name,
            if system_prompt.len() > 100 {
                format!("{}...", &system_prompt[..100])
            } else {
                system_prompt.clone()
            }
        );

        let mut system_item = ContextItem::new(
            ContextItemType::System,
            system_prompt,
            100, // Estimated tokens
            1.0, // Always relevant
        );
        system_item.metadata.pinned = true; // Don't remove system prompt

        context.add_item(system_item)?;

        Ok(())
    }

    /// Update context with a command interaction
    async fn update_context_with_interaction(
        &self,
        input: &CommandInput,
        output: &CommandOutput,
    ) -> MultiAgentResult<()> {
        let mut context = self.context.write().await;

        // Add user input
        let user_item = ContextItem::new(
            ContextItemType::User,
            input.text.clone(),
            input.text.len() as u64 / 4, // Rough token estimate
            0.8,
        );
        context.add_item(user_item)?;

        // Add assistant output
        let assistant_item = ContextItem::new(
            ContextItemType::Assistant,
            output.text.clone(),
            output.text.len() as u64 / 4, // Rough token estimate
            0.8,
        );
        context.add_item(assistant_item)?;

        Ok(())
    }

    /// Learn from a successful interaction
    async fn learn_from_interaction(&self, record: &CommandRecord) -> MultiAgentResult<()> {
        if let Some(quality_score) = record.quality_score {
            if quality_score >= self.config.quality_threshold {
                // Extract lessons from high-quality interactions
                let _lesson = format!(
                    "Successful {:?} command: {} -> {} (quality: {:.2})",
                    record.input.command_type,
                    record.input.text.chars().take(50).collect::<String>(),
                    record.output.text.chars().take(50).collect::<String>(),
                    quality_score
                );

                let _lessons = self.lessons.write().await;
                // TODO: Implement lesson learning - access through state field
                // lessons.state.add_lesson(lesson, quality_score);
            }
        }

        Ok(())
    }

    // Command handlers (placeholders for now - will be implemented with Rig)
    async fn handle_generate_command(
        &self,
        input: &CommandInput,
    ) -> MultiAgentResult<CommandOutput> {
        let context_items = self.get_enriched_context_for_query(&input.text).await?;

        let system_prompt = if let Some(configured_prompt) =
            self.role_config.extra.get("llm_system_prompt")
        {
            let raw_prompt = configured_prompt.as_str().unwrap_or("");
            let sanitized = crate::prompt_sanitizer::sanitize_system_prompt(raw_prompt);

            if sanitized.was_modified {
                tracing::warn!(
                    "System prompt was sanitized for agent {}. Warnings: {:?}",
                    self.agent_id,
                    sanitized.warnings
                );
            }

            sanitized.content
        } else {
            format!(
                "You are {}, a specialized AI agent with expertise in software development, architecture, and technical implementation. \
                Your role is to provide detailed, actionable, and technically accurate responses. \
                When generating code, ensure it's complete and functional - write actual working code, not placeholders or TODO comments. \
                When creating plans, provide specific numbered steps. \
                When writing documentation, be comprehensive and clear. \
                Focus on practical implementation and avoid generic responses.",
                self.role_config.name
            )
        };

        let messages = vec![
            LlmMessage::system(system_prompt),
            LlmMessage::user(format!(
                "Context: {}\n\nRequest: {}",
                context_items, input.text
            )),
        ];

        let request = LlmRequest::new(messages)
            .with_temperature(0.7)
            .with_metadata("command_type".to_string(), "generate".to_string())
            .with_metadata("agent_id".to_string(), self.agent_id.to_string());

        let response = self.llm_client.generate(request).await?;
        Ok(CommandOutput::new(response.content))
    }

    async fn handle_answer_command(&self, input: &CommandInput) -> MultiAgentResult<CommandOutput> {
        let context_items = self.get_enriched_context_for_query(&input.text).await?;

        let messages = vec![
            LlmMessage::system(format!(
                "You are {}, a knowledgeable AI agent. Provide accurate, helpful answers to questions. \
                 Use the provided context when relevant.",
                self.role_config.name
            )),
            LlmMessage::user(format!(
                "Context: {}\n\nQuestion: {}",
                context_items,
                input.text
            ))
        ];

        let request = LlmRequest::new(messages)
            .with_metadata("command_type".to_string(), "answer".to_string())
            .with_metadata("agent_id".to_string(), self.agent_id.to_string());

        let response = self.llm_client.generate(request).await?;
        Ok(CommandOutput::new(response.content))
    }

    async fn handle_search_command(
        &self,
        _input: &CommandInput,
    ) -> MultiAgentResult<CommandOutput> {
        // TODO: Implement using haystacks
        Ok(CommandOutput::new("Search results placeholder".to_string()))
    }

    async fn handle_analyze_command(
        &self,
        input: &CommandInput,
    ) -> MultiAgentResult<CommandOutput> {
        let context_items = self.get_enriched_context_for_query(&input.text).await?;

        let messages = vec![
            LlmMessage::system(format!(
                "You are {}, an analytical AI agent. Provide thorough, structured analysis of the given content. \
                 Break down complex topics, identify key patterns, and offer insights.",
                self.role_config.name
            )),
            LlmMessage::user(format!(
                "Context: {}\n\nAnalyze: {}",
                context_items,
                input.text
            ))
        ];

        let request = LlmRequest::new(messages)
            .with_temperature(0.3) // Lower temperature for more focused analysis
            .with_metadata("command_type".to_string(), "analyze".to_string())
            .with_metadata("agent_id".to_string(), self.agent_id.to_string());

        let response = self.llm_client.generate(request).await?;
        Ok(CommandOutput::new(response.content))
    }

    async fn handle_execute_command(
        &self,
        input: &CommandInput,
    ) -> MultiAgentResult<CommandOutput> {
        // Check if VM execution is enabled
        if let Some(vm_client) = &self.vm_execution_client {
            // Try to extract and execute code from the input
            log::info!("VM execution enabled, extracting code blocks from input");
            log::info!(
                "Input text length: {}, content: {:?}",
                input.text.len(),
                &input.text[..input.text.len().min(200)]
            );

            let code_extractor = crate::vm_execution::CodeBlockExtractor::new();
            let code_blocks = code_extractor.extract_code_blocks(&input.text);

            log::info!("Extracted {} code blocks", code_blocks.len());
            for (i, block) in code_blocks.iter().enumerate() {
                log::info!(
                    "Block {}: language={}, confidence={}, code_len={}",
                    i,
                    block.language,
                    block.execution_confidence,
                    block.code.len()
                );
            }

            if code_blocks.is_empty() {
                // No code blocks found, check execution intent
                let intent = code_extractor.detect_execution_intent(&input.text);
                if intent.confidence < 0.3 {
                    // Low confidence, treat as regular command
                    return Ok(CommandOutput::new(
                        "No executable code found in input".to_string(),
                    ));
                }
            }

            // Execute code blocks with sufficient confidence
            let mut results = Vec::new();
            for code_block in code_blocks {
                if code_block.execution_confidence > 0.5 {
                    // Validate code before execution
                    if let Err(validation_error) = code_extractor.validate_code(&code_block) {
                        results.push(format!(
                            "Validation failed for {} code: {}",
                            code_block.language, validation_error
                        ));
                        continue;
                    }

                    log::info!(
                        "Executing {} code block with confidence {}",
                        code_block.language,
                        code_block.execution_confidence
                    );

                    // Execute the code
                    let execute_request = crate::vm_execution::VmExecuteRequest {
                        agent_id: self.agent_id.to_string(),
                        language: code_block.language.clone(),
                        code: code_block.code.clone(),
                        vm_id: None, // Auto-provision
                        requirements: vec![],
                        timeout_seconds: Some(30),
                        working_dir: None,
                        metadata: None,
                    };

                    match vm_client.execute_code(execute_request).await {
                        Ok(response) => {
                            let result = format!(
                                "Executed {} code (exit code: {}):\n{}\n{}",
                                code_block.language,
                                response.exit_code,
                                if !response.stdout.is_empty() {
                                    &response.stdout
                                } else {
                                    "(no output)"
                                },
                                if !response.stderr.is_empty() {
                                    format!("Errors: {}", response.stderr)
                                } else {
                                    String::new()
                                }
                            );
                            results.push(result);
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Failed to execute {} code: {}", code_block.language, e);
                            log::error!("{}", error_msg);
                            results.push(error_msg);
                        }
                    }
                } else {
                    results.push(format!(
                        "Skipped {} code block (low confidence: {})",
                        code_block.language, code_block.execution_confidence
                    ));
                }
            }

            if results.is_empty() {
                Ok(CommandOutput::new("No code was executed".to_string()))
            } else {
                Ok(CommandOutput::new(results.join("\n\n")))
            }
        } else {
            // VM execution not enabled
            Ok(CommandOutput::new(
                "VM execution is not enabled for this agent".to_string(),
            ))
        }
    }

    async fn handle_create_command(&self, input: &CommandInput) -> MultiAgentResult<CommandOutput> {
        let context_items = self.get_enriched_context_for_query(&input.text).await?;

        let messages = vec![
            LlmMessage::system(format!(
                "You are {}, a creative AI agent. Create new content, structures, or solutions based on the request. \
                 Be innovative while following best practices.",
                self.role_config.name
            )),
            LlmMessage::user(format!(
                "Context: {}\n\nCreate: {}",
                context_items,
                input.text
            ))
        ];

        let request = LlmRequest::new(messages)
            .with_temperature(0.8) // Higher temperature for creativity
            .with_metadata("command_type".to_string(), "create".to_string())
            .with_metadata("agent_id".to_string(), self.agent_id.to_string());

        let response = self.llm_client.generate(request).await?;
        Ok(CommandOutput::new(response.content))
    }

    async fn handle_edit_command(&self, _input: &CommandInput) -> MultiAgentResult<CommandOutput> {
        // TODO: Implement with Rig framework
        Ok(CommandOutput::new("Edit placeholder".to_string()))
    }

    async fn handle_review_command(&self, input: &CommandInput) -> MultiAgentResult<CommandOutput> {
        let context_items = self.get_enriched_context_for_query(&input.text).await?;

        let messages = vec![
            LlmMessage::system(format!(
                "You are {}, a meticulous review agent. Provide detailed, constructive reviews. \
                 Identify strengths, weaknesses, and specific improvement recommendations.",
                self.role_config.name
            )),
            LlmMessage::user(format!(
                "Context: {}\n\nReview: {}",
                context_items, input.text
            )),
        ];

        let request = LlmRequest::new(messages)
            .with_temperature(0.4) // Moderate temperature for balanced critique
            .with_metadata("command_type".to_string(), "review".to_string())
            .with_metadata("agent_id".to_string(), self.agent_id.to_string());

        let response = self.llm_client.generate(request).await?;
        Ok(CommandOutput::new(response.content))
    }

    async fn handle_plan_command(&self, _input: &CommandInput) -> MultiAgentResult<CommandOutput> {
        // TODO: Implement with Rig framework
        Ok(CommandOutput::new("Plan placeholder".to_string()))
    }

    async fn handle_system_command(
        &self,
        _input: &CommandInput,
    ) -> MultiAgentResult<CommandOutput> {
        // TODO: Implement system commands
        Ok(CommandOutput::new("System command placeholder".to_string()))
    }

    async fn handle_custom_command(
        &self,
        _input: &CommandInput,
        _cmd_type: &str,
    ) -> MultiAgentResult<CommandOutput> {
        // TODO: Implement custom commands
        Ok(CommandOutput::new("Custom command placeholder".to_string()))
    }

    // Helper methods
    async fn load_rolegraph(role_config: &Role) -> MultiAgentResult<RoleGraph> {
        // TODO: Load from role configuration
        // TODO: Load actual rolegraph from role config
        use terraphim_types::{RoleName, Thesaurus};
        let role_name = RoleName::from(role_config.name.as_str());
        let thesaurus = Thesaurus::new("default".to_string());
        RoleGraph::new(role_name, thesaurus)
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))
    }

    async fn load_automata(_role_config: &Role) -> MultiAgentResult<AutocompleteIndex> {
        // TODO: Load from role configuration
        // TODO: Load actual automata from role config
        // TODO: Load actual automata from role config
        use terraphim_automata::{build_autocomplete_index, AutocompleteConfig};
        use terraphim_types::Thesaurus;

        let thesaurus = Thesaurus::new("default".to_string());
        build_autocomplete_index(thesaurus, Some(AutocompleteConfig::default()))
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))
    }

    fn extract_individual_goals(role_config: &Role) -> Vec<String> {
        let mut goals = Vec::new();

        // Extract goals from role configuration
        if !role_config.extra.is_empty() {
            if let Some(role_goals) = role_config.extra.get("goals") {
                if let Ok(goal_list) = serde_json::from_value::<Vec<String>>(role_goals.clone()) {
                    goals.extend(goal_list);
                }
            }
        }

        // Default goals based on role name
        match role_config.name.as_lowercase() {
            name if name.contains("engineer") => {
                goals.extend(vec![
                    "Write clean, efficient code".to_string(),
                    "Ensure system reliability".to_string(),
                    "Optimize performance".to_string(),
                ]);
            }
            name if name.contains("research") => {
                goals.extend(vec![
                    "Find accurate information".to_string(),
                    "Provide comprehensive analysis".to_string(),
                    "Cite reliable sources".to_string(),
                ]);
            }
            name if name.contains("documentation") => {
                goals.extend(vec![
                    "Create clear documentation".to_string(),
                    "Maintain consistency".to_string(),
                    "Improve accessibility".to_string(),
                ]);
            }
            _ => {
                goals.push("Provide helpful assistance".to_string());
            }
        }

        goals
    }

    /// Get relevant context for LLM requests with knowledge graph enrichment
    async fn get_relevant_context(&self) -> MultiAgentResult<String> {
        let context = self.context.read().await;

        // Get the most relevant context items from agent memory
        let relevant_items = context.get_items_by_relevance(0.5, Some(3));

        let mut context_summary = String::new();

        // Add existing agent memory context
        if !relevant_items.is_empty() {
            context_summary.push_str("=== Agent Memory Context ===\n");
            for (i, item) in relevant_items.iter().enumerate() {
                context_summary.push_str(&format!(
                    "{}. [{}] {}\n",
                    i + 1,
                    match item.item_type {
                        ContextItemType::System => "System",
                        ContextItemType::User => "User",
                        ContextItemType::Assistant => "Assistant",
                        ContextItemType::Memory => "Memory",
                        ContextItemType::Task => "Task",
                        ContextItemType::Concept => "Concept",
                        ContextItemType::Tool => "Tool",
                        ContextItemType::Document => "Document",
                        ContextItemType::Lesson => "Lesson",
                    },
                    item.content.chars().take(200).collect::<String>()
                ));
            }
            context_summary.push('\n');
        }

        // Always return some context, even if empty
        if context_summary.is_empty() {
            Ok("No relevant context available.".to_string())
        } else {
            Ok(context_summary)
        }
    }

    /// Enhanced context enrichment using rolegraph and haystack search
    async fn get_enriched_context_for_query(&self, query: &str) -> MultiAgentResult<String> {
        let mut enriched_context = String::new();

        // 1. Get knowledge graph node IDs that match the query
        let node_ids = self.rolegraph.find_matching_node_ids(query);
        if !node_ids.is_empty() {
            enriched_context.push_str("=== Knowledge Graph Matches ===\n");
            for (i, node_id) in node_ids.iter().take(3).enumerate() {
                enriched_context.push_str(&format!(
                    "{}. Graph Node ID: {} (related to query)\n",
                    i + 1,
                    node_id
                ));
            }
            enriched_context.push('\n');
        }

        // 2. Check for connected concepts in knowledge graph
        // Use the original query text to check for connections
        if self.rolegraph.is_all_terms_connected_by_path(query) {
            enriched_context.push_str("=== Knowledge Graph Connections ===\n");
            enriched_context.push_str(&format!(
                "Knowledge graph shows strong semantic connections for: '{}'\n\n",
                query
            ));
        }

        // 3. Query the graph for related concepts
        if let Ok(graph_results) = self.rolegraph.query_graph(query, Some(3), None) {
            if !graph_results.is_empty() {
                enriched_context.push_str("=== Related Graph Concepts ===\n");
                for (i, (term, _doc)) in graph_results.iter().take(3).enumerate() {
                    enriched_context.push_str(&format!(
                        "{}. Related Concept: {}\n",
                        i + 1,
                        term.chars().take(100).collect::<String>()
                    ));
                }
                enriched_context.push('\n');
            }
        }

        // 4. Add haystack search context information
        if !self.role_config.haystacks.is_empty() {
            enriched_context.push_str("=== Available Knowledge Sources ===\n");
            for (i, haystack) in self.role_config.haystacks.iter().enumerate() {
                enriched_context.push_str(&format!(
                    "{}. {:?}: {} - Ready for search queries\n",
                    i + 1,
                    haystack.service,
                    haystack.location
                ));
            }
            enriched_context.push('\n');
        }

        // 5. Get existing agent memory context
        let memory_context = self.get_relevant_context().await?;
        if memory_context != "No relevant context available." {
            enriched_context.push_str(&memory_context);
        }

        // 6. Add role-specific context enrichment
        enriched_context.push_str("=== Role Context ===\n");
        enriched_context.push_str(&format!("Acting as: {}\n", self.role_config.name));
        enriched_context.push_str(&format!(
            "Relevance Function: {:?}\n",
            self.role_config.relevance_function
        ));
        if let Some(kg) = &self.role_config.kg {
            enriched_context.push_str(&format!("Knowledge Graph Available: {:?}\n", kg));
        }

        if enriched_context.is_empty() {
            Ok("No enriched context available.".to_string())
        } else {
            Ok(enriched_context)
        }
    }
}

/// Serializable agent state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentState {
    pub agent_id: AgentId,
    pub role_config: Role,
    pub config: AgentConfig,
    pub goals: AgentGoals,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub memory_snapshot: terraphim_agent_evolution::MemoryState,
    pub tasks_snapshot: terraphim_agent_evolution::TasksState,
    pub lessons_snapshot: terraphim_agent_evolution::LessonsState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_config::{Role, ServiceType};
    use terraphim_persistence::DeviceStorage;

    #[tokio::test]
    async fn test_agent_creation() {
        let mut role = Role::new("Test Agent");
        role.shortname = Some("test".to_string());

        DeviceStorage::init_memory_only().await.unwrap();
        // Use test utility function which handles storage correctly
        let persistence = DeviceStorage::arc_memory_only().await.unwrap();
        let agent = TerraphimAgent::new(role, persistence, None).await.unwrap();

        assert_eq!(agent.role_config.name, "Test Agent".into());
        assert_eq!(*agent.status.read().await, AgentStatus::Initializing);
    }

    #[tokio::test]
    async fn test_agent_capabilities() {
        let mut role = Role::new("Engineering Agent");
        role.shortname = Some("eng".to_string());
        role.haystacks = vec![terraphim_config::Haystack {
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            location: "./src".to_string(),
            service: ServiceType::Ripgrep,
        }];
        role.extra.insert(
            "capabilities".to_string(),
            serde_json::json!(["code_review", "architecture"]),
        );

        DeviceStorage::init_memory_only().await.unwrap();
        // Use test utility function which handles storage correctly
        let persistence = DeviceStorage::arc_memory_only().await.unwrap();
        let agent = TerraphimAgent::new(role, persistence, None).await.unwrap();

        let capabilities = agent.get_capabilities();
        assert!(capabilities.contains(&"code_review".to_string()));
        assert!(capabilities.contains(&"architecture".to_string()));
        assert!(capabilities.contains(&"role_engineering agent".to_string()));
        assert!(capabilities.contains(&"haystack_code".to_string()));
    }

    #[tokio::test]
    async fn test_agent_goals() {
        let mut goals = AgentGoals::new(
            "Global goal".to_string(),
            vec!["Goal 1".to_string(), "Goal 2".to_string()],
        );

        assert_eq!(goals.global_goal, "Global goal");
        assert_eq!(goals.individual_goals.len(), 2);
        assert_eq!(goals.alignment_score, 0.5);

        goals.update_alignment_score(0.8);
        assert_eq!(goals.alignment_score, 0.8);

        goals.add_individual_goal("Goal 3".to_string());
        assert_eq!(goals.individual_goals.len(), 3);
    }
}
