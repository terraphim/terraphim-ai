use dioxus::prelude::*;

#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div { class: "has-text-centered",
            span { class: "icon is-large",
                i { class: "fas fa-spinner fa-pulse" }
            }
        }
    }
}
