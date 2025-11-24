use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Element, Length, Task, Theme,
};
use std::sync::Arc;

// Import terraphim modules
use terraphim_config::ConfigState;
use terraphim_settings::DeviceSettings;
use terraphim_types::{ChatMessage as TerraphimChatMessage, ContextItem as TerraphimContextItem, Conversation, Document, NormalizedTermValue, SearchQuery};
use terraphim_service::conversation_service::ConversationService;

#[cfg(feature = "openrouter")]
use terraphim_service::openrouter::OpenRouterClient;

fn main() -> iced::Result {
    // Initialize logging
    terraphim_service::logging::init_logging(terraphim_service::logging::detect_logging_config());

    log::info!("Starting Terraphim Desktop (Iced)...");

    iced::application("Terraphim", TerraphimApp::update, TerraphimApp::view)
        .theme(|app: &TerraphimApp| app.theme.clone())
        .run_with(TerraphimApp::new)
}

// Main application state
struct TerraphimApp {
    // Current view
    current_view: View,

    // Configuration state
    config_state: Option<Arc<ConfigState>>,
    device_settings: Option<DeviceSettings>,

    // Conversation service (not Debug, so wrapped carefully)
    #[allow(dead_code)]
    conversation_service: Arc<ConversationService>,

    // Search state
    search_input: String,
    search_results: Vec<Document>,
    search_suggestions: Vec<String>,
    show_suggestions: bool,

    // Chat state
    chat_input: String,
    chat_messages: Vec<ChatMessage>,
    conversation: Option<Conversation>,
    context_items: Vec<ContextItem>,
    show_context_form: bool,
    new_context_title: String,
    new_context_content: String,

    // KG Search state
    show_kg_search: bool,
    kg_search_input: String,
    kg_search_results: Vec<KgTerm>,

    // Theme
    theme: Theme,

    // Loading states
    is_loading: bool,
    error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Search,
    Chat,
    Config,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone)]
struct ContextItem {
    id: String,
    title: String,
    content: String,
    context_type: String,
}

impl From<&TerraphimContextItem> for ContextItem {
    fn from(item: &TerraphimContextItem) -> Self {
        Self {
            id: item.id.clone(),
            title: item.title.clone(),
            content: item.content.clone(),
            context_type: format!("{:?}", item.context_type),
        }
    }
}

#[derive(Debug, Clone)]
struct KgTerm {
    term: String,
    definition: Option<String>,
}

// Messages (events) that can be sent to update the application
#[derive(Debug, Clone)]
enum Message {
    // Navigation
    SwitchView(View),

    // Search & Autocomplete
    SearchInputChanged(String),
    SearchSubmitted,
    SearchCompleted(Result<Vec<Document>, String>),
    AutocompleteRequested,
    AutocompleteReceived(Result<Vec<String>, String>),
    SuggestionSelected(String),

    // Chat
    ChatInputChanged(String),
    ChatMessageSent,
    ChatResponseReceived(Result<(String, Conversation), String>),
    ConversationInitialized(Result<Conversation, String>),

    // Context Management
    ToggleContextForm,
    ContextTitleChanged(String),
    ContextContentChanged(String),
    AddContext,
    ContextAdded(Result<(Conversation, TerraphimContextItem), String>),
    RemoveContext(String),
    ContextRemoved(Result<Conversation, String>),
    LoadContext,
    ContextLoaded(Result<Vec<ContextItem>, String>),

    // KG Search
    ToggleKgSearch,
    KgSearchInputChanged(String),
    KgSearchSubmitted,
    KgSearchCompleted(Result<Vec<KgTerm>, String>),
    AddKgTermContext(String),

    // Configuration
    ConfigLoaded(Result<Arc<ConfigState>, String>),
    RoleSelected(String),

    // Theme
    ToggleTheme,

    // System
    None,
}

impl TerraphimApp {
    fn new() -> (Self, Task<Message>) {
        let conversation_service = Arc::new(ConversationService::new());

        let app = TerraphimApp {
            current_view: View::Search,
            config_state: None,
            device_settings: None,
            conversation_service: conversation_service.clone(),
            search_input: String::new(),
            search_results: Vec::new(),
            search_suggestions: Vec::new(),
            show_suggestions: false,
            chat_input: String::new(),
            chat_messages: vec![ChatMessage {
                role: "assistant".to_string(),
                content: "Hi! How can I help you? Ask me anything about your search results or documents.".to_string(),
            }],
            conversation: None,
            context_items: Vec::new(),
            show_context_form: false,
            new_context_title: String::new(),
            new_context_content: String::new(),
            show_kg_search: false,
            kg_search_input: String::new(),
            kg_search_results: Vec::new(),
            theme: Theme::Light,
            is_loading: false,
            error_message: None,
        };

        // Initialize configuration and conversation asynchronously
        let init_task = Task::batch(vec![
            Task::perform(Self::initialize_config(), Message::ConfigLoaded),
            Task::perform(
                Self::initialize_conversation(conversation_service),
                Message::ConversationInitialized,
            ),
        ]);

        (app, init_task)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchView(view) => {
                self.current_view = view;
                Task::none()
            }

            // Search & Autocomplete
            Message::SearchInputChanged(value) => {
                self.search_input = value;
                // Trigger autocomplete if input is long enough
                if self.search_input.len() >= 2 {
                    Task::perform(
                        Self::fetch_autocomplete(
                            self.search_input.clone(),
                            self.config_state.clone(),
                        ),
                        Message::AutocompleteReceived,
                    )
                } else {
                    self.search_suggestions.clear();
                    self.show_suggestions = false;
                    Task::none()
                }
            }

            Message::AutocompleteReceived(result) => {
                match result {
                    Ok(suggestions) => {
                        self.search_suggestions = suggestions;
                        self.show_suggestions = !self.search_suggestions.is_empty();
                    }
                    Err(e) => {
                        log::warn!("Autocomplete error: {}", e);
                        self.search_suggestions.clear();
                        self.show_suggestions = false;
                    }
                }
                Task::none()
            }

            Message::SuggestionSelected(suggestion) => {
                self.search_input = suggestion;
                self.show_suggestions = false;
                self.search_suggestions.clear();
                // Automatically trigger search
                Task::perform(
                    Self::perform_search(self.search_input.clone(), self.config_state.clone()),
                    Message::SearchCompleted,
                )
            }

            Message::SearchSubmitted => {
                if self.search_input.is_empty() {
                    return Task::none();
                }

                self.is_loading = true;
                self.show_suggestions = false;
                let query = self.search_input.clone();
                let config_state = self.config_state.clone();

                Task::perform(Self::perform_search(query, config_state), Message::SearchCompleted)
            }

            Message::SearchCompleted(result) => {
                self.is_loading = false;
                match result {
                    Ok(results) => {
                        self.search_results = results;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Search error: {}", e));
                    }
                }
                Task::none()
            }

            // Chat
            Message::ChatInputChanged(value) => {
                self.chat_input = value;
                Task::none()
            }

            Message::ChatMessageSent => {
                if self.chat_input.is_empty() {
                    return Task::none();
                }

                // Add user message
                self.chat_messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: self.chat_input.clone(),
                });

                let message = self.chat_input.clone();
                let conversation = self.conversation.clone();
                let config_state = self.config_state.clone();
                let conversation_service = self.conversation_service.clone();

                self.chat_input.clear();
                self.is_loading = true;

                Task::perform(
                    Self::send_chat_message(message, conversation, config_state, conversation_service),
                    Message::ChatResponseReceived,
                )
            }

            Message::ChatResponseReceived(result) => {
                self.is_loading = false;
                match result {
                    Ok((response, updated_conv)) => {
                        self.chat_messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: response,
                        });
                        self.conversation = Some(updated_conv);
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Chat error: {}", e));
                    }
                }
                Task::none()
            }

            Message::ConversationInitialized(result) => {
                match result {
                    Ok(conv) => {
                        log::info!("Conversation initialized: {}", conv.id.as_str());
                        self.conversation = Some(conv.clone());
                        // Load context for this conversation
                        let context_items: Vec<ContextItem> = conv
                            .global_context
                            .iter()
                            .map(|item| item.into())
                            .collect();
                        self.context_items = context_items;
                    }
                    Err(e) => {
                        log::warn!("Failed to initialize conversation: {}", e);
                        self.error_message = Some(format!("Failed to initialize conversation: {}", e));
                    }
                }
                Task::none()
            }

            // Context Management
            Message::ToggleContextForm => {
                self.show_context_form = !self.show_context_form;
                if !self.show_context_form {
                    self.new_context_title.clear();
                    self.new_context_content.clear();
                }
                Task::none()
            }

            Message::ContextTitleChanged(value) => {
                self.new_context_title = value;
                Task::none()
            }

            Message::ContextContentChanged(value) => {
                self.new_context_content = value;
                Task::none()
            }

            Message::AddContext => {
                if self.new_context_title.is_empty() || self.new_context_content.is_empty() {
                    return Task::none();
                }

                let title = self.new_context_title.clone();
                let content = self.new_context_content.clone();
                let conversation = self.conversation.clone();
                let conversation_service = self.conversation_service.clone();

                Task::perform(
                    Self::add_context(conversation, conversation_service, title, content),
                    Message::ContextAdded,
                )
            }

            Message::ContextAdded(result) => {
                match result {
                    Ok((updated_conv, new_context)) => {
                        self.new_context_title.clear();
                        self.new_context_content.clear();
                        self.show_context_form = false;
                        self.conversation = Some(updated_conv.clone());
                        // Update context items from conversation
                        self.context_items = updated_conv
                            .global_context
                            .iter()
                            .map(|item| item.into())
                            .collect();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to add context: {}", e));
                    }
                }
                Task::none()
            }

            Message::RemoveContext(context_id) => {
                let conversation = self.conversation.clone();
                let conversation_service = self.conversation_service.clone();

                Task::perform(
                    Self::remove_context(conversation, conversation_service, context_id),
                    Message::ContextRemoved,
                )
            }

            Message::ContextRemoved(result) => {
                match result {
                    Ok(updated_conv) => {
                        self.conversation = Some(updated_conv.clone());
                        // Update context items from conversation
                        self.context_items = updated_conv
                            .global_context
                            .iter()
                            .map(|item| item.into())
                            .collect();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to remove context: {}", e));
                    }
                }
                Task::none()
            }

            Message::LoadContext => {
                if let Some(conv) = &self.conversation {
                    let context_items: Vec<ContextItem> = conv
                        .global_context
                        .iter()
                        .map(|item| item.into())
                        .collect();
                    self.context_items = context_items;
                }
                Task::none()
            }

            Message::ContextLoaded(result) => {
                match result {
                    Ok(items) => {
                        self.context_items = items;
                    }
                    Err(e) => {
                        log::warn!("Failed to load context: {}", e);
                    }
                }
                Task::none()
            }

            // KG Search
            Message::ToggleKgSearch => {
                self.show_kg_search = !self.show_kg_search;
                if !self.show_kg_search {
                    self.kg_search_input.clear();
                    self.kg_search_results.clear();
                }
                Task::none()
            }

            Message::KgSearchInputChanged(value) => {
                self.kg_search_input = value;
                Task::none()
            }

            Message::KgSearchSubmitted => {
                if self.kg_search_input.is_empty() {
                    return Task::none();
                }

                let query = self.kg_search_input.clone();
                let config_state = self.config_state.clone();

                Task::perform(Self::search_kg_terms(query, config_state), Message::KgSearchCompleted)
            }

            Message::KgSearchCompleted(result) => {
                match result {
                    Ok(terms) => {
                        self.kg_search_results = terms;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("KG search error: {}", e));
                    }
                }
                Task::none()
            }

            Message::AddKgTermContext(term) => {
                let conversation = self.conversation.clone();
                let conversation_service = self.conversation_service.clone();

                Task::perform(
                    Self::add_kg_term_context(conversation, conversation_service, term),
                    Message::ContextAdded,
                )
            }

            Message::ConfigLoaded(result) => {
                match result {
                    Ok(config_state) => {
                        self.config_state = Some(config_state);
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Config load error: {}", e));
                    }
                }
                Task::none()
            }

            Message::RoleSelected(_role) => {
                // TODO: Implement role switching
                Task::none()
            }

            Message::ToggleTheme => {
                self.theme = match self.theme {
                    Theme::Light => Theme::Dark,
                    Theme::Dark => Theme::Light,
                    _ => Theme::Light,
                };
                Task::none()
            }

            Message::AutocompleteRequested | Message::None => Task::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        let content = match self.current_view {
            View::Search => self.view_search(),
            View::Chat => self.view_chat(),
            View::Config => self.view_config(),
        };

        let main_content = column![self.view_header(), content,]
            .spacing(20)
            .padding(20);

        container(main_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    // Async initialization
    async fn initialize_config() -> Result<Arc<ConfigState>, String> {
        use terraphim_config::{ConfigBuilder, ConfigId};
        use terraphim_persistence::Persistable;

        let _device_settings = DeviceSettings::load_from_env_and_file(None)
            .unwrap_or_else(|_| DeviceSettings::new());

        let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
            .build()
            .map_err(|e| format!("Failed to build config: {:?}", e))?;

        let config = match config.load().await {
            Ok(c) => c,
            Err(_) => ConfigBuilder::new()
                .build_default_desktop()
                .build()
                .map_err(|e| format!("Failed to build default config: {:?}", e))?,
        };

        let mut tmp_config = config.clone();
        let config_state = ConfigState::new(&mut tmp_config)
            .await
            .map_err(|e| format!("Failed to create config state: {:?}", e))?;

        Ok(Arc::new(config_state))
    }

    // Initialize conversation
    async fn initialize_conversation(
        conversation_service: Arc<ConversationService>,
    ) -> Result<Conversation, String> {
        use terraphim_types::RoleName;

        // Create a new conversation
        let conversation = conversation_service
            .create_conversation(
                "New Chat".to_string(),
                RoleName::new("Default"),
            )
            .await
            .map_err(|e| format!("Failed to create conversation: {:?}", e))?;

        log::info!("Initialized conversation: {}", conversation.id.as_str());
        Ok(conversation)
    }

    // Fetch autocomplete suggestions
    async fn fetch_autocomplete(
        query: String,
        config_state: Option<Arc<ConfigState>>,
    ) -> Result<Vec<String>, String> {
        use terraphim_automata::{autocomplete_search, build_autocomplete_index, fuzzy_autocomplete_search};

        let config_state = config_state.ok_or("Configuration not loaded")?;

        // Get current role
        let config = config_state.config.lock().await;
        let role = config.selected_role.clone();
        drop(config);

        // Get the rolegraph for the specified role
        let rolegraph_sync = config_state.roles.get(&role)
            .ok_or_else(|| format!("Role '{}' not found", role.as_str()))?;

        let rolegraph = rolegraph_sync.lock().await;

        // Build FST autocomplete index from the thesaurus
        let autocomplete_index = build_autocomplete_index(rolegraph.thesaurus.clone(), None)
            .map_err(|e| format!("Failed to build autocomplete index: {}", e))?;

        drop(rolegraph);

        let limit = 8;

        // Get autocomplete suggestions based on query length
        let results = if query.len() >= 3 {
            // For longer queries, try fuzzy search for better UX (0.7 = 70% similarity threshold)
            match fuzzy_autocomplete_search(&autocomplete_index, &query, 0.7, Some(limit)) {
                Ok(results) => results,
                Err(_) => {
                    // Fall back to exact search
                    autocomplete_search(&autocomplete_index, &query, Some(limit))
                        .map_err(|e| format!("Autocomplete failed: {}", e))?
                }
            }
        } else {
            // For shorter queries, use exact prefix match
            autocomplete_search(&autocomplete_index, &query, Some(limit))
                .map_err(|e| format!("Autocomplete failed: {}", e))?
        };

        // Extract just the terms from the results
        let suggestions: Vec<String> = results.into_iter().map(|r| r.term).collect();

        Ok(suggestions)
    }

    // Async search
    async fn perform_search(
        query: String,
        config_state: Option<Arc<ConfigState>>,
    ) -> Result<Vec<Document>, String> {
        let config_state = config_state.ok_or("Configuration not loaded")?;

        let config = config_state.config.lock().await;
        let role_name = config.selected_role.clone();
        drop(config);

        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(query),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(10),
            role: Some(role_name),
        };

        let config_state_ref = &*config_state;
        let mut search_service = terraphim_service::TerraphimService::new(config_state_ref.clone());
        let results = search_service
            .search(&search_query)
            .await
            .map_err(|e| format!("Search failed: {:?}", e))?;

        Ok(results)
    }

    // Send chat message with LLM integration
    async fn send_chat_message(
        message: String,
        conversation: Option<Conversation>,
        config_state: Option<Arc<ConfigState>>,
        conversation_service: Arc<ConversationService>,
    ) -> Result<(String, Conversation), String> {
        let mut conversation = conversation.ok_or("No conversation initialized")?;

        // Add user message to conversation
        let user_message = TerraphimChatMessage::user(message.clone());
        conversation.add_message(user_message.clone());

        // Try to use OpenRouter if available
        #[cfg(feature = "openrouter")]
        {
            if let Some(config_state) = config_state {
                if let Ok(response) = Self::call_openrouter(&conversation, &config_state).await {
                    // Add assistant response to conversation
                    let assistant_message = TerraphimChatMessage::assistant(response.clone(), None);
                    conversation.add_message(assistant_message);

                    // Save conversation
                    let _ = conversation_service.update_conversation(conversation.clone()).await;

                    return Ok((response, conversation));
                }
            }
        }

        // Fallback: Echo response
        let response = format!("Echo: {}", message);
        let assistant_message = TerraphimChatMessage::assistant(response.clone(), None);
        conversation.add_message(assistant_message);

        // Save conversation
        let _ = conversation_service.update_conversation(conversation.clone()).await;

        Ok((response, conversation))
    }

    #[cfg(feature = "openrouter")]
    async fn call_openrouter(
        conversation: &Conversation,
        config_state: &ConfigState,
    ) -> Result<String, String> {
        use std::env;

        // Get API key from environment
        let api_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| "OPENROUTER_API_KEY not set".to_string())?;

        // Create OpenRouter client
        let client = OpenRouterClient::new(api_key, None, None);

        // Convert conversation messages to OpenRouter format
        let messages: Vec<serde_json::Value> = conversation
            .messages
            .iter()
            .map(|msg| {
                let mut message_obj = serde_json::json!({
                    "role": msg.role,
                    "content": msg.content,
                });

                // Add context if available
                if !msg.context_items.is_empty() {
                    let context_text: String = msg
                        .context_items
                        .iter()
                        .map(|c| format!("[{}]\n{}\n", c.title, c.content))
                        .collect::<Vec<_>>()
                        .join("\n");

                    message_obj["content"] = serde_json::json!(format!(
                        "Context:\n{}\n\nUser message: {}",
                        context_text,
                        msg.content
                    ));
                }

                message_obj
            })
            .collect();

        // Add global context to the first message if available
        if !conversation.global_context.is_empty() && !messages.is_empty() {
            let context_text: String = conversation
                .global_context
                .iter()
                .map(|c| format!("[{}]\n{}\n", c.title, c.content))
                .collect::<Vec<_>>()
                .join("\n");

            let mut messages_with_context = vec![serde_json::json!({
                "role": "system",
                "content": format!("Global Context:\n{}", context_text),
            })];
            messages_with_context.extend(messages);

            client
                .chat_completion(messages_with_context, Some(512), Some(0.7))
                .await
                .map_err(|e| format!("OpenRouter error: {:?}", e))
        } else {
            client
                .chat_completion(messages, Some(512), Some(0.7))
                .await
                .map_err(|e| format!("OpenRouter error: {:?}", e))
        }
    }

    // Context management
    async fn add_context(
        conversation: Option<Conversation>,
        conversation_service: Arc<ConversationService>,
        title: String,
        content: String,
    ) -> Result<(Conversation, TerraphimContextItem), String> {
        use terraphim_types::ContextType;

        let mut conversation = conversation.ok_or("No conversation initialized")?;

        // Create context item
        let context_item = TerraphimContextItem {
            id: ulid::Ulid::new().to_string(),
            title,
            summary: None,
            content,
            context_type: ContextType::UserInput,
            metadata: ahash::AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: None,
        };

        // Add to conversation
        conversation.add_global_context(context_item.clone());

        // Save conversation
        conversation_service
            .update_conversation(conversation.clone())
            .await
            .map_err(|e| format!("Failed to save conversation: {:?}", e))?;

        log::info!("Added context: {} to conversation {}", context_item.title, conversation.id.as_str());
        Ok((conversation, context_item))
    }

    async fn remove_context(
        conversation: Option<Conversation>,
        conversation_service: Arc<ConversationService>,
        context_id: String,
    ) -> Result<Conversation, String> {
        let mut conversation = conversation.ok_or("No conversation initialized")?;

        // Remove context by ID
        conversation.global_context.retain(|c| c.id != context_id);

        // Save conversation
        conversation_service
            .update_conversation(conversation.clone())
            .await
            .map_err(|e| format!("Failed to save conversation: {:?}", e))?;

        log::info!("Removed context: {} from conversation {}", context_id, conversation.id.as_str());
        Ok(conversation)
    }

    async fn search_kg_terms(
        query: String,
        config_state: Option<Arc<ConfigState>>,
    ) -> Result<Vec<KgTerm>, String> {
        use terraphim_automata::{build_autocomplete_index, fuzzy_autocomplete_search};

        let config_state = config_state.ok_or("Configuration not loaded")?;

        let config = config_state.config.lock().await;
        let role = config.selected_role.clone();
        drop(config);

        // Get the rolegraph for the specified role
        let rolegraph_sync = config_state.roles.get(&role)
            .ok_or_else(|| format!("Role '{}' not found", role.as_str()))?;

        let rolegraph = rolegraph_sync.lock().await;

        // Build FST autocomplete index from the thesaurus
        let autocomplete_index = build_autocomplete_index(rolegraph.thesaurus.clone(), None)
            .map_err(|e| format!("Failed to build autocomplete index: {}", e))?;

        drop(rolegraph);

        // Use fuzzy search for KG terms
        let results = fuzzy_autocomplete_search(&autocomplete_index, &query, 0.7, Some(10))
            .map_err(|e| format!("KG search failed: {}", e))?;

        // Convert results to KgTerms
        let kg_terms: Vec<KgTerm> = results
            .into_iter()
            .map(|r| KgTerm {
                term: r.term,
                definition: r.url,
            })
            .collect();

        Ok(kg_terms)
    }

    async fn add_kg_term_context(
        conversation: Option<Conversation>,
        conversation_service: Arc<ConversationService>,
        term: String,
    ) -> Result<(Conversation, TerraphimContextItem), String> {
        use terraphim_types::ContextType;

        let mut conversation = conversation.ok_or("No conversation initialized")?;

        // Create context item from KG term
        let context_item = TerraphimContextItem {
            id: ulid::Ulid::new().to_string(),
            title: format!("KG: {}", term),
            summary: Some(format!("Knowledge Graph term: {}", term)),
            content: format!("Knowledge Graph term: {}", term),
            context_type: ContextType::KGTermDefinition,
            metadata: {
                let mut map = ahash::AHashMap::new();
                map.insert("term".to_string(), term.clone());
                map.insert("source_type".to_string(), "kg_term".to_string());
                map
            },
            created_at: chrono::Utc::now(),
            relevance_score: None,
        };

        // Add to conversation
        conversation.add_global_context(context_item.clone());

        // Save conversation
        conversation_service
            .update_conversation(conversation.clone())
            .await
            .map_err(|e| format!("Failed to save conversation: {:?}", e))?;

        log::info!("Added KG term context: {} to conversation {}", context_item.title, conversation.id.as_str());
        Ok((conversation, context_item))
    }

    // View: Header with navigation
    fn view_header(&self) -> Element<Message> {
        let title = text("Terraphim").size(24);

        let search_btn = button(text("Search")).on_press(Message::SwitchView(View::Search));

        let chat_btn = button(text("Chat")).on_press(Message::SwitchView(View::Chat));

        let config_btn = button(text("Config")).on_press(Message::SwitchView(View::Config));

        let theme_btn = button(text("🌓")).on_press(Message::ToggleTheme);

        row![title, search_btn, chat_btn, config_btn, theme_btn,]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
    }

    // View: Search with autocomplete
    fn view_search(&self) -> Element<Message> {
        let search_input = text_input("Search...", &self.search_input)
            .on_input(Message::SearchInputChanged)
            .on_submit(Message::SearchSubmitted)
            .padding(10)
            .size(20);

        let search_button = button(text("Search"))
            .on_press(Message::SearchSubmitted)
            .padding(10);

        let mut search_col = column![row![search_input, search_button]
            .spacing(10)
            .align_y(Alignment::Center)]
        .spacing(5);

        // Show autocomplete suggestions
        if self.show_suggestions && !self.search_suggestions.is_empty() {
            let mut suggestions_col = column![].spacing(2);
            for suggestion in &self.search_suggestions {
                let suggestion_btn = button(text(suggestion))
                    .on_press(Message::SuggestionSelected(suggestion.clone()))
                    .width(Length::Fill);
                suggestions_col = suggestions_col.push(suggestion_btn);
            }
            search_col = search_col.push(
                container(suggestions_col)
                    .padding(5)
                    .width(Length::Fill),
            );
        }

        let mut content = column![search_col].spacing(20);

        // Show error if any
        if let Some(error) = &self.error_message {
            content = content.push(text(error));
        }

        // Show loading indicator
        if self.is_loading {
            content = content.push(text("Searching..."));
        }

        // Show results
        if !self.search_results.is_empty() {
            content = content
                .push(text(format!("Found {} results:", self.search_results.len())));

            let mut results_col = column![].spacing(10);

            for doc in &self.search_results {
                let result_item = column![
                    text(&doc.title).size(18),
                    text(doc.description.as_deref().unwrap_or("No description")).size(14),
                ]
                .spacing(5)
                .padding(10);

                results_col = results_col.push(result_item);
            }

            content = content.push(scrollable(results_col).height(Length::Fill));
        } else if !self.is_loading {
            content = content.push(
                column![
                    text("Welcome to Terraphim").size(24),
                    text("Your privacy-first AI assistant").size(16),
                    text("Enter a search query above to get started").size(14),
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            );
        }

        content.into()
    }

    // View: Chat with context management
    fn view_chat(&self) -> Element<Message> {
        // Left: Chat messages and input
        let mut messages_col = column![].spacing(10);

        for msg in &self.chat_messages {
            let msg_text = text(&msg.content).size(14);
            let msg_container = container(msg_text).padding(10);
            messages_col = messages_col.push(msg_container);
        }

        let chat_scroll = scrollable(messages_col).height(Length::FillPortion(6));

        let chat_input = text_input("Type your message...", &self.chat_input)
            .on_input(Message::ChatInputChanged)
            .on_submit(Message::ChatMessageSent)
            .padding(10)
            .size(16);

        let send_button = button(text("Send"))
            .on_press(Message::ChatMessageSent)
            .padding(10);

        let input_row = row![chat_input, send_button]
            .spacing(10)
            .align_y(Alignment::Center);

        let chat_main = column![chat_scroll, input_row,].spacing(10);

        // Right: Context panel
        let mut context_col = column![
            text("Context").size(20),
            row![
                button(text("Add Context")).on_press(Message::ToggleContextForm),
                button(text("KG Search")).on_press(Message::ToggleKgSearch),
            ]
            .spacing(10),
        ]
        .spacing(10);

        // Context form
        if self.show_context_form {
            context_col = context_col.push(
                column![
                    text("Add Context").size(16),
                    text_input("Title", &self.new_context_title)
                        .on_input(Message::ContextTitleChanged)
                        .padding(5),
                    text_input("Content", &self.new_context_content)
                        .on_input(Message::ContextContentChanged)
                        .padding(5),
                    row![
                        button(text("Add")).on_press(Message::AddContext),
                        button(text("Cancel")).on_press(Message::ToggleContextForm),
                    ]
                    .spacing(10),
                ]
                .spacing(5)
                .padding(10),
            );
        }

        // KG Search modal
        if self.show_kg_search {
            context_col = context_col.push(
                column![
                    text("KG Search").size(16),
                    text_input("Search terms...", &self.kg_search_input)
                        .on_input(Message::KgSearchInputChanged)
                        .on_submit(Message::KgSearchSubmitted)
                        .padding(5),
                    button(text("Search")).on_press(Message::KgSearchSubmitted),
                ]
                .spacing(5)
                .padding(10),
            );

            // Show KG search results
            if !self.kg_search_results.is_empty() {
                let mut kg_results_col = column![].spacing(5);
                for kg_term in &self.kg_search_results {
                    kg_results_col = kg_results_col.push(
                        row![
                            text(&kg_term.term).size(14),
                            button(text("+")).on_press(Message::AddKgTermContext(kg_term.term.clone())),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }
                context_col = context_col.push(scrollable(kg_results_col).height(Length::Fill));
            }

            context_col = context_col.push(button(text("Close")).on_press(Message::ToggleKgSearch));
        }

        // Context items
        if !self.context_items.is_empty() {
            context_col = context_col.push(text(format!("{} items", self.context_items.len())).size(14));

            let mut items_col = column![].spacing(5);
            for item in &self.context_items {
                items_col = items_col.push(
                    row![
                        column![
                            text(&item.title).size(14),
                            text(&item.context_type).size(12),
                        ]
                        .spacing(2),
                        button(text("×")).on_press(Message::RemoveContext(item.id.clone())),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .padding(5),
                );
            }
            context_col = context_col.push(scrollable(items_col).height(Length::Fill));
        }

        let context_panel = container(context_col)
            .padding(10)
            .width(Length::FillPortion(2));

        row![
            container(chat_main)
                .padding(10)
                .width(Length::FillPortion(3)),
            context_panel,
        ]
        .spacing(10)
        .into()
    }

    // View: Config (placeholder)
    fn view_config(&self) -> Element<Message> {
        column![
            text("Configuration").size(24),
            text("Configuration UI coming soon...").size(16),
        ]
        .spacing(20)
        .align_x(Alignment::Center)
        .into()
    }
}
