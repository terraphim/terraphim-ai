use anyhow::Result;
use std::sync::Arc;
use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::conversation::OpenDALConversationPersistence;
use terraphim_persistence::Persistable;
use terraphim_service::{context::ContextManager, TerraphimService};
use terraphim_settings::DeviceSettings;
use terraphim_types::{
    ChatMessage, ContextItem, Conversation, ConversationId, ConversationSummary, Document,
    NormalizedTermValue, RoleName, SearchQuery, Thesaurus,
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct TuiService {
    config_state: ConfigState,
    service: Arc<Mutex<TerraphimService>>,
    context_manager: Arc<Mutex<ContextManager>>,
    // Note: OpenDALConversationPersistence doesn't implement Clone, so we wrap in Arc
    conversation_persistence: Arc<OpenDALConversationPersistence>,
}

impl TuiService {
    /// Initialize a new TUI service with embedded configuration
    pub async fn new() -> Result<Self> {
        // Initialize logging
        terraphim_service::logging::init_logging(
            terraphim_service::logging::detect_logging_config(),
        );

        log::info!("Initializing TUI service with embedded configuration");

        // Load device settings
        let device_settings = DeviceSettings::load_from_env_and_file(None)?;
        log::debug!("Device settings: {:?}", device_settings);

        // Try to load existing configuration, fallback to default embedded config
        let mut config = match ConfigBuilder::new_with_id(ConfigId::Embedded).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => {
                    log::info!("Loaded existing embedded configuration");
                    config
                }
                Err(e) => {
                    log::info!("Failed to load config: {:?}, using default embedded", e);
                    ConfigBuilder::new_with_id(ConfigId::Embedded)
                        .build_default_embedded()
                        .build()?
                }
            },
            Err(e) => {
                log::warn!("Failed to build config: {:?}, using default", e);
                ConfigBuilder::new_with_id(ConfigId::Embedded)
                    .build_default_embedded()
                    .build()?
            }
        };

        // Create config state
        let config_state = ConfigState::new(&mut config).await?;

        // Create service
        let service = TerraphimService::new(config_state.clone());
        let service_arc = Arc::new(Mutex::new(service));

        // Pre-build thesauri for roles with knowledge graphs to avoid warnings on first use
        {
            let roles_with_kg: Vec<_> = config
                .roles
                .iter()
                .filter(|(_, role)| role.kg.is_some())
                .map(|(name, _)| name.clone())
                .collect();

            for role_name in roles_with_kg {
                log::info!("Pre-building thesaurus for role: {}", role_name);
                let mut svc = service_arc.lock().await;
                match svc.ensure_thesaurus_loaded(&role_name).await {
                    Ok(_) => log::info!("✅ Thesaurus ready for {}", role_name),
                    Err(e) => log::warn!("Failed to build thesaurus for {}: {}", role_name, e),
                }
            }
        }

        // Create context manager
        let context_manager = Arc::new(Mutex::new(ContextManager::new(
            terraphim_service::context::ContextConfig::default(),
        )));

        // Create conversation persistence
        let conversation_persistence = Arc::new(OpenDALConversationPersistence::new());

        Ok(Self {
            config_state,
            service: service_arc,
            context_manager,
            conversation_persistence,
        })
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> terraphim_config::Config {
        let config = self.config_state.config.lock().await;
        config.clone()
    }

    /// Get the current selected role
    pub async fn get_selected_role(&self) -> RoleName {
        let config = self.config_state.config.lock().await;
        config.selected_role.clone()
    }

    /// Update the selected role
    pub async fn update_selected_role(
        &self,
        role_name: RoleName,
    ) -> Result<terraphim_config::Config> {
        let service = self.service.lock().await;
        Ok(service.update_selected_role(role_name).await?)
    }

    /// List all available roles
    pub async fn list_roles(&self) -> Vec<String> {
        let config = self.config_state.config.lock().await;
        config.roles.keys().map(|r| r.to_string()).collect()
    }

    /// Search documents using the current selected role
    #[allow(dead_code)]
    pub async fn search(&self, search_term: &str, limit: Option<usize>) -> Result<Vec<Document>> {
        let selected_role = self.get_selected_role().await;
        self.search_with_role(search_term, &selected_role, limit)
            .await
    }

    /// Search documents with a specific role
    pub async fn search_with_role(
        &self,
        search_term: &str,
        role: &RoleName,
        limit: Option<usize>,
    ) -> Result<Vec<Document>> {
        let query = SearchQuery {
            search_term: NormalizedTermValue::from(search_term),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit,
            role: Some(role.clone()),
        };

        let mut service = self.service.lock().await;
        Ok(service.search(&query).await?)
    }

    /// Search documents using a complete SearchQuery (supports logical operators)
    pub async fn search_with_query(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        let mut service = self.service.lock().await;
        Ok(service.search(query).await?)
    }

    /// Get thesaurus for a specific role
    pub async fn get_thesaurus(&self, role_name: &RoleName) -> Result<Thesaurus> {
        let mut service = self.service.lock().await;
        Ok(service.ensure_thesaurus_loaded(role_name).await?)
    }

    /// Get the role graph top-k concepts for a specific role
    pub async fn get_role_graph_top_k(
        &self,
        role_name: &RoleName,
        top_k: usize,
    ) -> Result<Vec<String>> {
        // For now, return placeholder data since role graph access needs proper implementation
        // TODO: Implement actual role graph integration
        log::info!("Getting top {} concepts for role {}", top_k, role_name);
        Ok((0..std::cmp::min(top_k, 10))
            .map(|i| format!("concept_{}_for_role_{}", i + 1, role_name))
            .collect())
    }

    /// Generate chat response using LLM
    pub async fn chat(
        &self,
        role_name: &RoleName,
        prompt: &str,
        model: Option<String>,
    ) -> Result<String> {
        // Get role configuration
        let role = {
            let config = self.config_state.config.lock().await;
            config
                .roles
                .get(role_name)
                .ok_or_else(|| anyhow::anyhow!("Role not found"))?
                .clone()
        };

        // Build LLM client from role configuration
        if let Some(llm_client) = terraphim_service::llm::build_llm_from_role(&role) {
            log::info!(
                "Using LLM provider: {} for role: {}",
                llm_client.name(),
                role_name
            );

            // Prepare messages for chat completion
            let messages = vec![serde_json::json!({
                "role": "user",
                "content": prompt
            })];

            // Configure chat options
            let opts = terraphim_service::llm::ChatOptions {
                max_tokens: Some(2048),
                temperature: Some(0.7),
                model,
            };

            // Call LLM
            let response = llm_client
                .chat_completion(messages, opts)
                .await
                .map_err(|e| anyhow::anyhow!("LLM error: {}", e))?;

            Ok(response)
        } else {
            Err(anyhow::anyhow!(
                "No LLM configured for role {}. Add llm_provider to role.extra",
                role_name
            ))
        }
    }

    /// Extract paragraphs from text using thesaurus
    pub async fn extract_paragraphs(
        &self,
        role_name: &RoleName,
        text: &str,
        exclude_term: bool,
    ) -> Result<Vec<(String, String)>> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Use automata to extract paragraphs
        let results = terraphim_automata::matcher::extract_paragraphs_from_automata(
            text,
            thesaurus,
            !exclude_term, // include_term is opposite of exclude_term
        )?;

        // Convert to string tuples
        let string_results = results
            .into_iter()
            .map(|(matched, paragraph)| (matched.normalized_term.value.to_string(), paragraph))
            .collect();

        Ok(string_results)
    }

    /// Perform autocomplete search using thesaurus for a role
    #[allow(dead_code)]
    pub async fn autocomplete(
        &self,
        role_name: &RoleName,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<terraphim_automata::AutocompleteResult>> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Build autocomplete index
        let config = Some(terraphim_automata::AutocompleteConfig {
            max_results: limit.unwrap_or(10),
            min_prefix_length: 1,
            case_sensitive: false,
        });

        let index = terraphim_automata::build_autocomplete_index(thesaurus, config)?;

        // Perform search
        Ok(terraphim_automata::autocomplete_search(
            &index, query, limit,
        )?)
    }

    /// Find matches in text using thesaurus
    #[allow(dead_code)]
    pub async fn find_matches(
        &self,
        role_name: &RoleName,
        text: &str,
    ) -> Result<Vec<terraphim_automata::Matched>> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Find matches
        Ok(terraphim_automata::find_matches(text, thesaurus, true)?)
    }

    /// Replace matches in text with links using thesaurus
    #[allow(dead_code)]
    pub async fn replace_matches(
        &self,
        role_name: &RoleName,
        text: &str,
        link_type: terraphim_automata::LinkType,
    ) -> Result<String> {
        // Get thesaurus for the role
        let thesaurus = self.get_thesaurus(role_name).await?;

        // Replace matches
        let result = terraphim_automata::replace_matches(text, thesaurus, link_type)?;
        Ok(String::from_utf8(result).unwrap_or_else(|_| text.to_string()))
    }

    /// Summarize content using available AI services
    #[allow(dead_code)]
    pub async fn summarize(&self, role_name: &RoleName, content: &str) -> Result<String> {
        // For now, use the chat method with a summarization prompt
        let prompt = format!("Please summarize the following content:\n\n{}", content);
        self.chat(role_name, &prompt, None).await
    }

    /// Save configuration changes
    pub async fn save_config(&self) -> Result<()> {
        let config = self.config_state.config.lock().await;
        config.save().await?;
        Ok(())
    }

    // ==================== Conversation Management (RAG Workflow) ====================

    /// Create a new conversation for chat with context
    pub async fn create_conversation(&self, title: String) -> Result<ConversationId> {
        let role = self.get_selected_role().await;
        let mut context_manager = self.context_manager.lock().await;

        let conv_id = context_manager.create_conversation(title, role).await?;

        // Get the conversation and persist it
        if let Some(conversation) = context_manager.get_conversation(&conv_id) {
            use terraphim_persistence::conversation::ConversationPersistence;
            self.conversation_persistence.save(&conversation).await?;
        }

        Ok(conv_id)
    }

    /// Load an existing conversation from persistence
    pub async fn load_conversation(&self, id: &ConversationId) -> Result<Conversation> {
        use terraphim_persistence::conversation::ConversationPersistence;
        Ok(self.conversation_persistence.load(id).await?)
    }

    /// List all persisted conversations
    pub async fn list_conversations(&self) -> Result<Vec<ConversationSummary>> {
        use terraphim_persistence::conversation::ConversationPersistence;
        Ok(self.conversation_persistence.list_summaries().await?)
    }

    /// Delete a conversation from persistence
    pub async fn delete_conversation(&self, id: &ConversationId) -> Result<()> {
        use terraphim_persistence::conversation::ConversationPersistence;
        Ok(self.conversation_persistence.delete(id).await?)
    }

    /// Get current conversation from context manager (in-memory)
    pub async fn get_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<Option<Conversation>> {
        let context_manager = self.context_manager.lock().await;
        Ok(context_manager
            .get_conversation(conversation_id)
            .map(|c| (*c).clone()))
    }

    // ==================== Context Management ====================

    /// Add a single document to conversation context
    pub async fn add_document_to_context(
        &self,
        conversation_id: &ConversationId,
        document: &Document,
    ) -> Result<()> {
        let mut context_manager = self.context_manager.lock().await;

        // Create context item from document
        let context_item = context_manager.create_document_context(document);

        // Add to conversation
        context_manager.add_context(conversation_id, context_item)?;

        // Persist the updated conversation
        if let Some(conversation) = context_manager.get_conversation(conversation_id) {
            use terraphim_persistence::conversation::ConversationPersistence;
            self.conversation_persistence.save(&conversation).await?;
        }

        Ok(())
    }

    /// Add multiple documents from search results as a single context item
    pub async fn add_search_results_to_context(
        &self,
        conversation_id: &ConversationId,
        query: &str,
        documents: &[Document],
        limit: Option<usize>,
    ) -> Result<()> {
        let mut context_manager = self.context_manager.lock().await;

        // Create context item from search results
        let context_item = context_manager.create_search_context(query, documents, limit);

        // Add to conversation
        context_manager.add_context(conversation_id, context_item)?;

        // Persist the updated conversation
        if let Some(conversation) = context_manager.get_conversation(conversation_id) {
            use terraphim_persistence::conversation::ConversationPersistence;
            self.conversation_persistence.save(&conversation).await?;
        }

        Ok(())
    }

    /// List all context items for a conversation
    pub async fn list_context(&self, conversation_id: &ConversationId) -> Result<Vec<ContextItem>> {
        let context_manager = self.context_manager.lock().await;

        if let Some(conversation) = context_manager.get_conversation(conversation_id) {
            Ok(conversation.global_context.clone())
        } else {
            Ok(Vec::new())
        }
    }

    /// Clear all context from a conversation
    pub async fn clear_context(&self, conversation_id: &ConversationId) -> Result<()> {
        let context_manager = self.context_manager.lock().await;

        if let Some(conversation) = context_manager.get_conversation(conversation_id) {
            // Create updated conversation with cleared context
            let mut updated_conv = (*conversation).clone();
            updated_conv.global_context.clear();
            updated_conv.updated_at = chrono::Utc::now();

            // Save to persistence
            use terraphim_persistence::conversation::ConversationPersistence;
            self.conversation_persistence.save(&updated_conv).await?;
        }

        Ok(())
    }

    /// Remove a specific context item by ID
    pub async fn remove_context_item(
        &self,
        conversation_id: &ConversationId,
        context_id: &str,
    ) -> Result<()> {
        let mut context_manager = self.context_manager.lock().await;

        context_manager.delete_context(conversation_id, context_id)?;

        // Persist changes
        if let Some(conversation) = context_manager.get_conversation(conversation_id) {
            use terraphim_persistence::conversation::ConversationPersistence;
            self.conversation_persistence.save(&conversation).await?;
        }

        Ok(())
    }

    // ==================== Chat with Context (RAG) ====================

    /// Chat with LLM using conversation context (RAG workflow)
    pub async fn chat_with_context(
        &self,
        conversation_id: &ConversationId,
        user_message: String,
        model: Option<String>,
    ) -> Result<String> {
        let role = self.get_selected_role().await;

        // Build prompt with context
        let prompt = {
            let context_manager = self.context_manager.lock().await;

            if let Some(conversation) = context_manager.get_conversation(conversation_id) {
                // Build messages with context for LLM
                let messages = terraphim_service::context::build_llm_messages_with_context(
                    &conversation,
                    true, // include global context
                );

                // Format messages into prompt
                let context_text = messages
                    .iter()
                    .map(|msg| {
                        format!(
                            "{}: {}",
                            msg.get("role")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown"),
                            msg.get("content").and_then(|v| v.as_str()).unwrap_or("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");

                format!("{}\n\nUser: {}", context_text, user_message)
            } else {
                user_message.clone()
            }
        };

        // Call LLM (clone model for later use)
        let model_for_message = model.clone();
        let response = self.chat(&role, &prompt, model).await?;

        // Add messages to conversation
        {
            let mut context_manager = self.context_manager.lock().await;

            context_manager.add_message(conversation_id, ChatMessage::user(user_message))?;

            context_manager.add_message(
                conversation_id,
                ChatMessage::assistant(response.clone(), model_for_message),
            )?;

            // Persist conversation with new messages
            if let Some(conversation) = context_manager.get_conversation(conversation_id) {
                use terraphim_persistence::conversation::ConversationPersistence;
                self.conversation_persistence.save(&conversation).await?;
            }
        }

        Ok(response)
    }
}
