use dioxus::prelude::*;
use crate::state::{ConfigState, SearchState};
use crate::services::SearchService;
use terraphim_config::ConfigState as CoreConfigState;
use terraphim_automata::AutocompleteResult;

#[component]
pub fn Search() -> Element {
    let mut search_state = use_context::<SearchState>();
    let config_state = use_context::<ConfigState>();

    let input = search_state.input();
    let results = search_state.results();
    let loading = search_state.is_loading();
    let error = search_state.error();

    // Autocomplete state
    let mut autocomplete_suggestions = use_signal(|| Vec::<AutocompleteResult>::new());
    let mut show_autocomplete = use_signal(|| false);
    let mut selected_suggestion_index = use_signal(|| 0_usize);

    // Handle search
    let search = move || {
        let input_value = search_state.input();
        if input_value.is_empty() {
            return;
        }

        // Hide autocomplete when searching
        show_autocomplete.set(false);

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

    // Handle autocomplete
    let update_autocomplete = move |prefix: String| {
        if prefix.len() < 2 {
            show_autocomplete.set(false);
            return;
        }

        spawn(async move {
            match CoreConfigState::from_config(config_state.get_config()).await {
                Ok(core_config) => {
                    let mut service = SearchService::new(core_config);

                    // Initialize autocomplete index
                    let role = terraphim_types::RoleName::from(config_state.selected_role());
                    if let Err(e) = service.initialize_autocomplete(&role).await {
                        tracing::warn!("Failed to initialize autocomplete: {:?}", e);
                        return;
                    }

                    let suggestions = service.autocomplete(&prefix);
                    autocomplete_suggestions.set(suggestions.clone());
                    show_autocomplete.set(!suggestions.is_empty());
                    selected_suggestion_index.set(0);
                }
                Err(e) => {
                    tracing::error!("Failed to create config state for autocomplete: {:?}", e);
                }
            }
        });
    };

    // Select suggestion
    let select_suggestion = move |term: String| {
        search_state.set_input(term);
        show_autocomplete.set(false);
        search();
    };

    rsx! {
        div { class: "search-container",
            style: "position: relative;",

            div { class: "field",
                div { class: "control has-icons-left",
                    input {
                        class: "input is-large",
                        r#type: "search",
                        placeholder: "Search knowledge graph for {config_state.selected_role()}...",
                        value: "{input}",
                        disabled: loading,
                        oninput: move |evt| {
                            let value = evt.value();
                            search_state.set_input(value.clone());
                            update_autocomplete(value);
                        },
                        onkeydown: move |evt| {
                            match evt.key() {
                                Key::Enter => {
                                    if show_autocomplete() && !autocomplete_suggestions().is_empty() {
                                        // Select current suggestion
                                        let suggestions = autocomplete_suggestions();
                                        let idx = selected_suggestion_index();
                                        if idx < suggestions.len() {
                                            select_suggestion(suggestions[idx].term.clone());
                                        }
                                    } else {
                                        search();
                                    }
                                }
                                Key::ArrowDown => {
                                    if show_autocomplete() {
                                        let max = autocomplete_suggestions().len();
                                        if max > 0 {
                                            let current = selected_suggestion_index();
                                            selected_suggestion_index.set((current + 1) % max);
                                        }
                                    }
                                }
                                Key::ArrowUp => {
                                    if show_autocomplete() {
                                        let max = autocomplete_suggestions().len();
                                        if max > 0 {
                                            let current = selected_suggestion_index();
                                            selected_suggestion_index.set(if current == 0 { max - 1 } else { current - 1 });
                                        }
                                    }
                                }
                                Key::Escape => {
                                    show_autocomplete.set(false);
                                }
                                _ => {}
                            }
                        },
                        onblur: move |_| {
                            // Delay hiding to allow click events on suggestions
                            spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                                show_autocomplete.set(false);
                            });
                        }
                    }
                    span { class: "icon is-left",
                        i { class: if loading { "fas fa-spinner fa-spin" } else { "fas fa-search" } }
                    }
                }

                // Autocomplete dropdown
                if show_autocomplete() && !autocomplete_suggestions().is_empty() {
                    div {
                        class: "autocomplete-dropdown box",
                        style: "position: absolute; top: 100%; left: 0; right: 0; z-index: 1000; max-height: 300px; overflow-y: auto; margin-top: 0.5rem;",

                        for (idx, suggestion) in autocomplete_suggestions().iter().enumerate() {
                            div {
                                class: if idx == selected_suggestion_index() {
                                    "autocomplete-item p-2 is-active"
                                } else {
                                    "autocomplete-item p-2"
                                },
                                style: if idx == selected_suggestion_index() {
                                    "background: #3273dc; color: white; cursor: pointer;"
                                } else {
                                    "cursor: pointer; border-bottom: 1px solid #f5f5f5;"
                                },
                                onmousedown: {
                                    let term = suggestion.term.clone();
                                    move |_| select_suggestion(term.clone())
                                },
                                onmouseenter: move |_| selected_suggestion_index.set(idx),

                                div { class: "is-flex is-justify-content-space-between",
                                    span { class: "has-text-weight-semibold",
                                        "{suggestion.term}"
                                    }
                                    if let Some(url) = &suggestion.url {
                                        span { class: "is-size-7 has-text-grey",
                                            style: if idx == selected_suggestion_index() { "color: #e8e8e8 !important;" } else { "" },
                                            "📎"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Error display
            if let Some(error_msg) = error {
                div { class: "notification is-danger mt-4",
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
                        p { class: "mt-2 is-size-7",
                            "💡 Tip: Start typing to see autocomplete suggestions"
                        }
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
