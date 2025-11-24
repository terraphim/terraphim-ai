use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Element, Length, Task, Theme,
};
use std::sync::Arc;

// Import terraphim modules
use terraphim_config::ConfigState;
use terraphim_settings::DeviceSettings;

fn main() -> iced::Result {
    // Initialize logging
    terraphim_service::logging::init_logging(terraphim_service::logging::detect_logging_config());

    log::info!("Starting Terraphim Desktop (Iced)...");

    iced::application("Terraphim", TerraphimApp::update, TerraphimApp::view)
        .theme(|app: &TerraphimApp| app.theme.clone())
        .run_with(TerraphimApp::new)
}

// Main application state
#[derive(Debug)]
struct TerraphimApp {
    // Current view
    current_view: View,

    // Configuration state
    config_state: Option<Arc<ConfigState>>,
    device_settings: Option<DeviceSettings>,

    // Search state
    search_input: String,
    search_results: Vec<terraphim_types::Document>,

    // Chat state
    chat_input: String,
    chat_messages: Vec<ChatMessage>,

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
    Graph,
    Config,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

// Messages (events) that can be sent to update the application
#[derive(Debug, Clone)]
enum Message {
    // Navigation
    SwitchView(View),

    // Search
    SearchInputChanged(String),
    SearchSubmitted,
    SearchCompleted(Result<Vec<terraphim_types::Document>, String>),

    // Chat
    ChatInputChanged(String),
    ChatMessageSent,
    ChatResponseReceived(Result<String, String>),

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
        let app = TerraphimApp {
            current_view: View::Search,
            config_state: None,
            device_settings: None,
            search_input: String::new(),
            search_results: Vec::new(),
            chat_input: String::new(),
            chat_messages: vec![ChatMessage {
                role: "assistant".to_string(),
                content: "Hi! How can I help you? Ask me anything about your search results or documents.".to_string(),
            }],
            theme: Theme::Light,
            is_loading: false,
            error_message: None,
        };

        // Initialize configuration asynchronously
        let init_task = Task::perform(
            Self::initialize_config(),
            Message::ConfigLoaded,
        );

        (app, init_task)
    }

    fn title(&self) -> String {
        match self.current_view {
            View::Search => "Terraphim - Search".to_string(),
            View::Chat => "Terraphim - Chat".to_string(),
            View::Graph => "Terraphim - Knowledge Graph".to_string(),
            View::Config => "Terraphim - Configuration".to_string(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchView(view) => {
                self.current_view = view;
                Task::none()
            }

            Message::SearchInputChanged(value) => {
                self.search_input = value;
                Task::none()
            }

            Message::SearchSubmitted => {
                if self.search_input.is_empty() {
                    return Task::none();
                }

                self.is_loading = true;
                let query = self.search_input.clone();
                let config_state = self.config_state.clone();

                Task::perform(
                    Self::perform_search(query, config_state),
                    Message::SearchCompleted,
                )
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
                self.chat_input.clear();
                self.is_loading = true;

                Task::perform(
                    Self::send_chat_message(message),
                    Message::ChatResponseReceived,
                )
            }

            Message::ChatResponseReceived(result) => {
                self.is_loading = false;
                match result {
                    Ok(response) => {
                        self.chat_messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: response,
                        });
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Chat error: {}", e));
                    }
                }
                Task::none()
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

            Message::None => Task::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        let content = match self.current_view {
            View::Search => self.view_search(),
            View::Chat => self.view_chat(),
            View::Graph => self.view_graph(),
            View::Config => self.view_config(),
        };

        let main_content = column![
            self.view_header(),
            content,
        ]
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

        // Load device settings
        let _device_settings = DeviceSettings::load_from_env_and_file(None)
            .unwrap_or_else(|_| DeviceSettings::new());

        // Build configuration
        let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
            .build()
            .map_err(|e| format!("Failed to build config: {:?}", e))?;

        // Try to load existing config, fallback to default
        let config = match config.load().await {
            Ok(c) => c,
            Err(_) => {
                ConfigBuilder::new()
                    .build_default_desktop()
                    .build()
                    .map_err(|e| format!("Failed to build default config: {:?}", e))?
            }
        };

        // Create config state
        let mut tmp_config = config.clone();
        let config_state = ConfigState::new(&mut tmp_config)
            .await
            .map_err(|e| format!("Failed to create config state: {:?}", e))?;

        Ok(Arc::new(config_state))
    }

    // Async search
    async fn perform_search(
        query: String,
        config_state: Option<Arc<ConfigState>>,
    ) -> Result<Vec<terraphim_types::Document>, String> {
        use terraphim_types::NormalizedTermValue;

        let config_state = config_state.ok_or("Configuration not loaded")?;

        // Get current role
        let config = config_state.config.lock().await;
        let role_name = config.selected_role.clone();
        drop(config);

        // Build search query with correct structure
        let search_query = terraphim_types::SearchQuery {
            search_term: NormalizedTermValue::new(query),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(10),
            role: Some(role_name),
        };

        // Perform search using service layer
        // Clone the Arc and extract the ConfigState reference
        let config_state_ref = &*config_state;
        let mut search_service = terraphim_service::TerraphimService::new(config_state_ref.clone());
        let results = search_service.search(&search_query)
            .await
            .map_err(|e| format!("Search failed: {:?}", e))?;

        Ok(results)
    }

    // Async chat
    async fn send_chat_message(message: String) -> Result<String, String> {
        // TODO: Implement actual chat API call
        // For now, return a mock response
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        Ok(format!("Echo: {}", message))
    }

    // View: Header with navigation
    fn view_header(&self) -> Element<Message> {
        let title = text("Terraphim")
            .size(24);

        let search_btn = button(text("Search"))
            .on_press(Message::SwitchView(View::Search));

        let chat_btn = button(text("Chat"))
            .on_press(Message::SwitchView(View::Chat));

        let graph_btn = button(text("Graph"))
            .on_press(Message::SwitchView(View::Graph));

        let config_btn = button(text("Config"))
            .on_press(Message::SwitchView(View::Config));

        let theme_btn = button(text("🌓"))
            .on_press(Message::ToggleTheme);

        row![
            title,
            search_btn,
            chat_btn,
            graph_btn,
            config_btn,
            theme_btn,
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }

    // View: Search
    fn view_search(&self) -> Element<Message> {
        let search_input = text_input("Search...", &self.search_input)
            .on_input(Message::SearchInputChanged)
            .on_submit(Message::SearchSubmitted)
            .padding(10)
            .size(20);

        let search_button = button(text("Search"))
            .on_press(Message::SearchSubmitted)
            .padding(10);

        let search_row = row![search_input, search_button]
            .spacing(10)
            .align_y(Alignment::Center);

        let mut content = column![search_row].spacing(20);

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
            content = content.push(text(format!("Found {} results:", self.search_results.len())));

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
                .align_x(Alignment::Center)
            );
        }

        content.into()
    }

    // View: Chat
    fn view_chat(&self) -> Element<Message> {
        let mut messages_col = column![].spacing(10);

        for msg in &self.chat_messages {
            let msg_text = text(&msg.content).size(14);
            let msg_container = container(msg_text)
                .padding(10);
            messages_col = messages_col.push(msg_container);
        }

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

        column![
            scrollable(messages_col)
                .height(Length::FillPortion(8)),
            input_row,
        ]
        .spacing(20)
        .into()
    }

    // View: Graph (placeholder)
    fn view_graph(&self) -> Element<Message> {
        column![
            text("Knowledge Graph Visualization").size(24),
            text("Graph view coming soon...").size(16),
        ]
        .spacing(20)
        .align_x(Alignment::Center)
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
