use dioxus::prelude::*;
use crate::components::chat::Chat;

#[component]
pub fn ChatRoute() -> Element {
    rsx! {
        div { class: "container",
            div { class: "section",
                Chat {}
            }
        }
    }
}
