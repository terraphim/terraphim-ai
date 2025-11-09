use dioxus::prelude::*;
use terraphim_types::ConversationId;

#[derive(Clone)]
pub struct ConversationState {
    current_conversation_id: Signal<Option<ConversationId>>,
    show_session_list: Signal<bool>,
}

impl ConversationState {
    pub fn new() -> Self {
        Self {
            current_conversation_id: Signal::new(None),
            show_session_list: Signal::new(false),
        }
    }

    pub fn current_conversation_id(&self) -> Option<ConversationId> {
        self.current_conversation_id.read().clone()
    }

    pub fn set_current_conversation(&self, id: Option<ConversationId>) {
        self.current_conversation_id.set(id);
    }

    pub fn show_session_list(&self) -> bool {
        *self.show_session_list.read()
    }

    pub fn toggle_session_list(&self) {
        let current = *self.show_session_list.read();
        self.show_session_list.set(!current);
    }
}
