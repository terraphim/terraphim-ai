use anyhow::Result;
use terraphim_types::ConversationId;

pub struct ConversationStorage;

impl ConversationStorage {
    pub fn new() -> Self {
        Self
    }

    pub async fn save_conversation(&self, id: &ConversationId, data: &[u8]) -> Result<()> {
        // TODO: Implement conversation persistence
        Ok(())
    }

    pub async fn load_conversation(&self, id: &ConversationId) -> Result<Option<Vec<u8>>> {
        // TODO: Implement conversation loading
        Ok(None)
    }
}
