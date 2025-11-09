use dioxus::prelude::*;

#[component]
pub fn JsonEditor() -> Element {
    rsx! {
        div { class: "notification is-warning",
            p { "JSON editor coming soon!" }
        }
    }
}
