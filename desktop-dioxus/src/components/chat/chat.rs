use dioxus::prelude::*;
use crate::state::{ConfigState, ConversationState};
use crate::services::{ChatService, render_markdown};
use terraphim_types::{Conversation, ChatMessage, RoleName};

#[component]
pub fn Chat() -> Element {
    let mut conv_state = use_context::<ConversationState>();
    let config_state = use_context::<ConfigState>();

    // Local state
    let mut messages = use_signal(|| Vec::<ChatMessage>::new());
    let mut input = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut conversation = use_signal(|| Option::<Conversation>::None);

    // Initialize conversation on mount
    use_effect(move || {
        spawn(async move {
            let role = RoleName::from(config_state.selected_role());
            let mut service = ChatService::new();

            // Try to initialize LLM
            match service.initialize_llm(config_state.get_config()).await {
                Ok(_) => {
                    tracing::info!("LLM initialized");

                    // Create new conversation
                    match service.create_conversation("New Chat".to_string(), role).await {
                        Ok(conv) => {
                            tracing::info!("Created conversation: {}", conv.id.as_str());
                            conversation.set(Some(conv.clone()));
                            messages.set(conv.messages.clone());
                        }
                        Err(e) => {
                            tracing::error!("Failed to create conversation: {:?}", e);
                            error.set(Some(format!("Failed to create conversation: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to initialize LLM: {:?}", e);
                    error.set(Some("LLM not configured. Check your config.".to_string()));
                }
            }
        });
    });

    // Send message handler
    let send_message = move || {
        let input_value = input().clone();
        if input_value.trim().is_empty() || loading() {
            return;
        }

        loading.set(true);
        error.set(None);

        spawn(async move {
            if let Some(mut conv) = conversation().clone() {
                let mut service = ChatService::new();

                // Re-initialize LLM
                if let Err(e) = service.initialize_llm(config_state.get_config()).await {
                    error.set(Some(format!("LLM initialization failed: {}", e)));
                    loading.set(false);
                    return;
                }

                match service.send_message(&mut conv, input_value).await {
                    Ok(_assistant_msg) => {
                        tracing::info!("Got AI response");
                        conversation.set(Some(conv.clone()));
                        messages.set(conv.messages.clone());
                        input.set(String::new());
                        loading.set(false);
                    }
                    Err(e) => {
                        tracing::error!("Failed to send message: {:?}", e);
                        error.set(Some(format!("Failed to send message: {}", e)));
                        loading.set(false);
                    }
                }
            } else {
                error.set(Some("No conversation active".to_string()));
                loading.set(false);
            }
        });
    };

    rsx! {
        div { class: "chat-container",
            style: "display: flex; flex-direction: column; height: calc(100vh - 200px);",

            // Messages area
            div { 
                class: "messages-area box",
                style: "flex: 1; overflow-y: auto; margin-bottom: 1rem; padding: 1rem;",

                if let Some(err) = error() {
                    div { class: "notification is-danger",
                        button {
                            class: "delete",
                            onclick: move |_| error.set(None)
                        }
                        "{err}"
                    }
                }

                if messages().is_empty() {
                    div { class: "has-text-centered has-text-grey",
                        p { class: "is-size-4",
                            "Start a conversation with AI"
                        }
                        p { class: "mt-2",
                            "Type your message below and press Enter or click Send"
                        }
                    }
                } else {
                    for msg in messages() {
                        div {
                            class: if msg.role == "user" {
                                "message-bubble user-message mb-4"
                            } else {
                                "message-bubble assistant-message mb-4"
                            },
                            style: if msg.role == "user" {
                                "background: #e3f2fd; padding: 1rem; border-radius: 8px; margin-left: 20%;"
                            } else {
                                "background: #f5f5f5; padding: 1rem; border-radius: 8px; margin-right: 20%;"
                            },

                            div { class: "message-header mb-2",
                                strong { 
                                    if msg.role == "user" { "You" } else { "AI Assistant" }
                                }
                                span { 
                                    class: "is-size-7 has-text-grey ml-2",
                                    "{msg.created_at.format(\"%H:%M\")}"
                                }
                            }

                            // Render markdown for assistant messages, plain text for user
                            if msg.role == "assistant" {
                                div { 
                                    class: "message-content markdown-content",
                                    dangerous_inner_html: "{render_markdown(&msg.content)}"
                                }
                            } else {
                                div { class: "message-content",
                                    "{msg.content}"
                                }
                            }
                        }
                    }
                }

                // Loading indicator
                if loading() {
                    div { class: "message-bubble assistant-message mb-4",
                        style: "background: #f5f5f5; padding: 1rem; border-radius: 8px; margin-right: 20%;",
                        span { class: "icon",
                            i { class: "fas fa-spinner fa-spin" }
                        }
                        " AI is thinking..."
                    }
                }
            }

            // Input area
            div { class: "field has-addons",
                div { class: "control is-expanded",
                    textarea {
                        class: "textarea",
                        placeholder: "Type your message... (Shift+Enter for new line)",
                        value: "{input()}",
                        disabled: loading(),
                        rows: 3,
                        oninput: move |evt| input.set(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Enter && !evt.shift_key() && !loading() {
                                send_message();
                                evt.prevent_default();
                            }
                        }
                    }
                }
                div { class: "control",
                    button {
                        class: "button is-primary is-medium",
                        style: "height: 100%;",
                        disabled: loading() || input().trim().is_empty(),
                        onclick: move |_| send_message(),

                        if loading() {
                            span { class: "icon",
                                i { class: "fas fa-spinner fa-spin" }
                            }
                        } else {
                            span { class: "icon",
                                i { class: "fas fa-paper-plane" }
                            }
                        }
                        span { "Send" }
                    }
                }
            }

            // Helpful tip
            div { class: "has-text-grey is-size-7 mt-2",
                "💡 Tip: AI responses support markdown formatting including code blocks, lists, and links"
            }
        }
    }
}
