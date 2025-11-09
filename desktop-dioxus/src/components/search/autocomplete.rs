use dioxus::prelude::*;

#[component]
pub fn AutocompleteDropdown(suggestions: Vec<String>) -> Element {
    rsx! {
        ul { class: "suggestions", "Autocomplete placeholder" }
    }
}
