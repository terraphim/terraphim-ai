use dioxus::prelude::*;
use crate::state::{ConfigState, SearchState};
use crate::services::SearchService;
use terraphim_config::ConfigState as CoreConfigState;

#[component]
pub fn Search() -> Element {
    let mut search_state = use_context::<SearchState>();
    let config_state = use_context::<ConfigState>();

    let input = search_state.input();
    let results = search_state.results();
    let loading = search_state.is_loading();
    let error = search_state.error();

    // Handle search
    let search = move || {
        let input_value = search_state.input();
        if input_value.is_empty() {
            return;
        }

        search_state.set_loading(true);
        search_state.set_error(None);

        spawn(async move {
            tracing::info!("Searching for: {}", input_value);

            // Create backend config state
            match CoreConfigState::from_config(config_state.get_config()).await {
                Ok(core_config) => {
                    let mut service = SearchService::new(core_config);

                    match service.search(&input_value).await {
                        Ok(documents) => {
                            tracing::info!("Found {} documents", documents.len());
                            search_state.set_results(documents);
                            search_state.set_loading(false);
                        }
                        Err(e) => {
                            tracing::error!("Search failed: {:?}", e);
                            search_state.set_error(Some(format!("Search failed: {}", e)));
                            search_state.set_loading(false);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create config state: {:?}", e);
                    search_state.set_error(Some(format!("Configuration error: {}", e)));
                    search_state.set_loading(false);
                }
            }
        });
    };

    rsx! {
        div { class: "search-container",
            div { class: "field",
                div { class: "control has-icons-left",
                    input {
                        class: "input is-large",
                        r#type: "search",
                        placeholder: "Search knowledge graph for {config_state.selected_role()}...",
                        value: "{input}",
                        disabled: loading,
                        oninput: move |evt| {
                            search_state.set_input(evt.value());
                        },
                        onkeydown: move |evt| {
                            if evt.key() == Key::Enter {
                                search();
                            }
                        }
                    }
                    span { class: "icon is-left",
                        i { class: if loading { "fas fa-spinner fa-spin" } else { "fas fa-search" } }
                    }
                }
            }

            // Error display
            if let Some(error_msg) = error {
                div { class: "notification is-danger",
                    button {
                        class: "delete",
                        onclick: move |_| search_state.set_error(None)
                    }
                    "{error_msg}"
                }
            }

            // Results
            div { class: "results mt-4",
                if loading {
                    div { class: "has-text-centered",
                        p { class: "is-size-4",
                            span { class: "icon",
                                i { class: "fas fa-spinner fa-spin" }
                            }
                            " Searching..."
                        }
                    }
                } else if results.is_empty() && !input.is_empty() {
                    div { class: "has-text-centered has-text-grey",
                        p { "No results found. Try a different search term." }
                    }
                } else if results.is_empty() {
                    div { class: "has-text-centered has-text-grey",
                        p { "Enter a search term to get started" }
                    }
                } else {
                    div { class: "columns is-multiline",
                        for result in results {
                            div { class: "column is-12",
                                div { class: "box",
                                    div { class: "content",
                                        p { class: "title is-5",
                                            a {
                                                href: "{result.url}",
                                                target: "_blank",
                                                "{result.title}"
                                            }
                                        }
                                        if let Some(desc) = &result.description {
                                            p { class: "subtitle is-6", "{desc}" }
                                        }
                                        if let Some(rank) = result.rank {
                                            span { class: "tag is-info is-light",
                                                "Rank: {rank}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
