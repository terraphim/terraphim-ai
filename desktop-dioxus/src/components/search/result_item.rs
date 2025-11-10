use dioxus::prelude::*;
use terraphim_types::Document;

#[component]
pub fn ResultItem(document: Document) -> Element {
    rsx! {
        div { class: "box",
            p { class: "title is-5", "{document.id}" }
        }
    }
}
