use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::state::{ConfigState, ConversationState, SearchState};
use crate::components::navigation::Navbar;
use crate::system_tray::TrayEvent;

// Define routes
#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    SearchPage {},
    #[route("/chat")]
    ChatPage {},
    #[route("/config/wizard")]
    ConfigWizardPage {},
    #[route("/config/json")]
    ConfigJsonPage {},
}

#[component]
pub fn App() -> Element {
    // Initialize global state contexts
    use_context_provider(|| ConfigState::new());
    use_context_provider(|| ConversationState::new());
    use_context_provider(|| SearchState::new());

    // Handle tray events
    let mut config_state = use_context::<ConfigState>();

    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        if let Some(mut rx) = crate::subscribe_to_tray_events() {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        match event {
                            TrayEvent::RoleChanged(role_name) => {
                                tracing::info!("App received role change: {}", role_name);
                                config_state.select_role(role_name);
                            }
                            TrayEvent::Toggle => {
                                tracing::info!("App received toggle window event");
                                // Window toggle handled via desktop APIs
                                // Dioxus desktop doesn't expose window hide/show yet
                                // For now, we'll just log it
                                // TODO: Implement when Dioxus 0.7 adds window management APIs
                            }
                            TrayEvent::Quit => {
                                // Handled in main.rs
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error receiving tray event: {:?}", e);
                        break;
                    }
                }
            }
        }
    });

    rsx! {
        style { {include_str!("../assets/bulma/bulma.min.css")} }
        style { {include_str!("../assets/styles/custom.css")} }

        div { class: "is-full-height",
            Router::<Route> {}
        }
    }
}

#[component]
pub fn SearchPage() -> Element {
    rsx! {
        Navbar {}
        div { class: "main-area",
            crate::routes::SearchRoute {}
        }
    }
}

#[component]
pub fn ChatPage() -> Element {
    rsx! {
        Navbar {}
        div { class: "main-area",
            crate::routes::ChatRoute {}
        }
    }
}

#[component]
pub fn ConfigWizardPage() -> Element {
    rsx! {
        div { class: "main-area",
            crate::routes::ConfigWizardRoute {}
        }
    }
}

#[component]
pub fn ConfigJsonPage() -> Element {
    rsx! {
        div { class: "main-area",
            crate::routes::ConfigJsonRoute {}
        }
    }
}
