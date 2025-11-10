use dioxus::prelude::*;
use crate::components::search::Search;

#[component]
pub fn SearchRoute() -> Element {
    rsx! {
        div { class: "container",
            div { class: "section",
                Search {}
            }
        }
    }
}
