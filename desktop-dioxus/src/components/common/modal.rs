use dioxus::prelude::*;

#[component]
pub fn Modal(is_active: bool, children: Element) -> Element {
    rsx! {
        div { class: if is_active { "modal is-active" } else { "modal" },
            div { class: "modal-background" }
            div { class: "modal-content", {children} }
        }
    }
}
