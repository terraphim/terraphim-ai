use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::state::{ConfigState, ConversationState, SearchState};
use crate::components::navigation::Navbar;

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
