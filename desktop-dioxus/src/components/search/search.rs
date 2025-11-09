use dioxus::prelude::*;
use crate::state::{ConfigState, SearchState};

#[component]
pub fn Search() -> Element {
    let search_state = use_context::<SearchState>();
    let config_state = use_context::<ConfigState>();

    let input = search_state.input();
    let results = search_state.results();

    rsx! {
        div { class: "search-container",
            div { class: "field",
                div { class: "control has-icons-left",
                    input {
                        class: "input is-large",
                        r#type: "search",
                        placeholder: "Search knowledge graph for {config_state.selected_role()}...",
                        value: "{input}",
                        oninput: move |evt| search_state.set_input(evt.value().clone()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Enter {
                                // TODO: Trigger search
                                tracing::info!("Search triggered for: {}", search_state.input());
                            }
                        }
                    }
                    span { class: "icon is-left",
                        i { class: "fas fa-search" }
                    }
                }
            }

            // Results
            div { class: "results",
                if results.is_empty() {
                    div { class: "has-text-centered has-text-grey",
                        p { "No results yet. Try searching for something!" }
                    }
                } else {
                    for result in results {
                        div { class: "box",
                            p { class: "title is-5", "{result.id}" }
                            if let Some(desc) = &result.description {
                                p { class: "subtitle is-6", "{desc}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
