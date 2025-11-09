use dioxus::prelude::*;
use crate::state::ConversationState;

#[component]
pub fn Chat() -> Element {
    let conv_state = use_context::<ConversationState>();

    rsx! {
        div { class: "chat-container",
            div { class: "notification is-info",
                p { "Chat feature coming soon!" }
                p { "Current conversation: {conv_state.current_conversation_id():?}" }
            }
        }
    }
}
