use terraphim_service::conversation_service::ConversationService;
use terraphim_service::llm_proxy::LlmProxy;
use terraphim_types::{ChatMessage, Conversation, ConversationId, RoleName};
use terraphim_config::Config;
use anyhow::Result;

/// Chat service wrapper for Dioxus frontend
pub struct ChatService {
    conversation_service: ConversationService,
    llm_proxy: Option<LlmProxy>,
}

impl ChatService {
    /// Create a new chat service
    pub fn new() -> Self {
        Self {
            conversation_service: ConversationService::new(),
            llm_proxy: None,
        }
    }

    /// Initialize LLM proxy from config
    pub async fn initialize_llm(&mut self, config: Config) -> Result<()> {
        tracing::info!("Initializing LLM proxy");

        let llm_proxy = LlmProxy::from_config(&config).await?;
        self.llm_proxy = Some(llm_proxy);

        Ok(())
    }

    /// Create a new conversation
    pub async fn create_conversation(
        &self,
        title: String,
        role: RoleName,
    ) -> Result<Conversation> {
        tracing::info!("Creating conversation: {} for role: {}", title, role);

        Ok(self.conversation_service.create_conversation(title, role).await?)
    }

    /// Get conversation by ID
    pub async fn get_conversation(&self, id: &ConversationId) -> Result<Conversation> {
        tracing::info!("Getting conversation: {}", id.as_str());

        Ok(self.conversation_service.get_conversation(id).await?)
    }

    /// Update conversation
    pub async fn update_conversation(&self, conversation: Conversation) -> Result<Conversation> {
        tracing::info!("Updating conversation: {}", conversation.id.as_str());

        Ok(self.conversation_service.update_conversation(conversation).await?)
    }

    /// Send a message and get AI response
    pub async fn send_message(
        &mut self,
        conversation: &mut Conversation,
        user_message: String,
    ) -> Result<ChatMessage> {
        tracing::info!("Sending message in conversation: {}", conversation.id.as_str());

        // Add user message to conversation
        let user_chat_message = ChatMessage::user(user_message.clone());
        conversation.add_message(user_chat_message);

        // Get AI response
        if let Some(llm) = &self.llm_proxy {
            tracing::info!("Getting AI response");

            let messages: Vec<_> = conversation.messages.iter().map(|m| {
                terraphim_types::Message {
                    role: m.role.clone(),
                    content: m.content.clone(),
                }
            }).collect();

            match llm.chat_completion(messages, None).await {
                Ok(response) => {
                    tracing::info!("Got AI response: {} chars", response.len());

                    let assistant_message = ChatMessage::assistant(
                        response,
                        Some("llm".to_string()),
                    );

                    conversation.add_message(assistant_message.clone());

                    // Save updated conversation
                    let _ = self.conversation_service.update_conversation(conversation.clone()).await;

                    Ok(assistant_message)
                }
                Err(e) => {
                    tracing::error!("LLM error: {:?}", e);
                    Err(anyhow::anyhow!("Failed to get AI response: {:?}", e))
                }
            }
        } else {
            Err(anyhow::anyhow!("LLM not initialized"))
        }
    }

    /// List all conversations
    pub async fn list_conversations(&self) -> Result<Vec<Conversation>> {
        tracing::info!("Listing all conversations");

        Ok(self.conversation_service.list_all().await?)
    }

    /// Delete a conversation
    pub async fn delete_conversation(&self, id: &ConversationId) -> Result<()> {
        tracing::info!("Deleting conversation: {}", id.as_str());

        Ok(self.conversation_service.delete_conversation(id).await?)
    }
}
