use dioxus::prelude::*;

#[component]
pub fn TermChip(term: String, on_remove: EventHandler<String>) -> Element {
    rsx! {
        span { class: "tag is-info", "{term}" }
    }
}
