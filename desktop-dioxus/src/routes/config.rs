use dioxus::prelude::*;
use crate::components::config::{ConfigWizard, JsonEditor};

#[component]
pub fn ConfigWizardRoute() -> Element {
    rsx! {
        div { class: "container",
            div { class: "section",
                ConfigWizard {}
            }
        }
    }
}

#[component]
pub fn ConfigJsonRoute() -> Element {
    rsx! {
        div { class: "container",
            div { class: "section",
                JsonEditor {}
            }
        }
    }
}
