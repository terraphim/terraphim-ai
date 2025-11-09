use dioxus::prelude::*;

#[component]
pub fn ConfigWizard() -> Element {
    rsx! {
        div { class: "notification is-warning",
            p { "Configuration wizard coming soon!" }
        }
    }
}
