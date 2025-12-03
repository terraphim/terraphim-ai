use gpui::*;
use crate::views::search::input::SearchInput;
use crate::state::search::{SearchState, AutocompleteSuggestion};
use terraphim_types::RoleName;

/// Test that autocomplete selection updates the input field correctly
#[gpui::test]
async fn test_autocomplete_selection_updates_input(cx: &mut TestAppContext) {
    // Initialize the search state with a test role
    let search_state = cx.new_model(|cx| {
        let mut state = SearchState::new(cx);
        state.current_role = "Terraphim Engineer".to_string();
        state
    });

    // Create the SearchInput view
    let search_input = cx.new_view(|cx| SearchInput::new(cx, search_state.clone()));

    // Simulate user typing "gra"
    search_input.update(cx, |view, cx| {
        view.set_query("gra".to_string(), cx);
    });

    // Wait for autocomplete to process
    cx.run_until_parked();

    // Verify the input value is "gra"
    search_input.update(cx, |view, cx| {
        assert_eq!(view.get_query(cx), "gra");
    });

    // Simulate selecting "graph" from autocomplete
    search_state.update(cx, |state, cx| {
        state.autocomplete_suggestions = vec![
            AutocompleteSuggestion {
                term: "graph".to_string(),
                normalized_term: "graph".to_string(),
                url: Some("https://example.com/graph".to_string()),
                score: 0.95,
            }
        ];
        state.selected_suggestion_index = 0;
        state.show_autocomplete = true;
        cx.notify();
    });

    // Simulate pressing Tab to accept autocomplete
    search_input.update(cx, |view, cx| {
        view.handle_key_down(&KeyDownEvent {
            keystroke: Keystroke {
                key: "tab".to_string(),
                modifiers: Modifiers::default(),
                ime_key: None,
            },
            is_held: false,
        }, cx);
    });

    // Wait for the selection to be processed
    cx.run_until_parked();

    // Verify the input value is now "graph" (not "gra")
    search_input.update(cx, |view, cx| {
        assert_eq!(view.get_query(cx), "graph", "Input should be updated to 'graph', not remain as 'gra'");
    });

    // Verify autocomplete is hidden after selection
    search_state.update(cx, |state, _cx| {
        assert!(!state.show_autocomplete, "Autocomplete should be hidden after selection");
        assert_eq!(state.selected_suggestion_index, 0);
    });
}

/// Test that autocomplete handles race conditions correctly
#[gpui::test]
async fn test_autocomplete_race_condition_prevention(cx: &mut TestAppContext) {
    let search_state = cx.new_model(|cx| {
        let mut state = SearchState::new(cx);
        state.current_role = "Terraphim Engineer".to_string();
        state
    });

    let search_input = cx.new_view(|cx| SearchInput::new(cx, search_state.clone()));

    // Simulate rapid typing
    for char in ["g", "r", "a"] {
        search_input.update(cx, |view, cx| {
            let current = view.get_query(cx);
            view.set_query(format!("{}{}", current, char), cx);
        });
        cx.run_until_parked();
    }

    // Verify final state is "gra" (not corrupted by race conditions)
    search_input.update(cx, |view, cx| {
        assert_eq!(view.get_query(cx), "gra");
    });

    // Now select autocomplete
    search_state.update(cx, |state, cx| {
        state.autocomplete_suggestions = vec![
            AutocompleteSuggestion {
                term: "graph".to_string(),
                normalized_term: "graph".to_string(),
                url: None,
                score: 0.9,
            }
        ];
        state.selected_suggestion_index = 0;
        state.show_autocomplete = true;
        cx.notify();
    });

    search_input.update(cx, |view, cx| {
        view.handle_key_down(&KeyDownEvent {
            keystroke: Keystroke {
                key: "tab".to_string(),
                modifiers: Modifiers::default(),
                ime_key: None,
            },
            is_held: false,
        }, cx);
    });

    cx.run_until_parked();

    // Verify the suppression mechanism worked
    search_input.update(cx, |view, cx| {
        assert_eq!(view.get_query(cx), "graph", "Should be 'graph', not 'gra' with additional input");
    });
}

/// Test mouse click selection for autocomplete
#[gpui::test]
async fn test_autocomplete_mouse_click_selection(cx: &mut TestAppContext) {
    let search_state = cx.new_model(|cx| {
        let mut state = SearchState::new(cx);
        state.current_role = "Terraphim Engineer".to_string();
        state
    });

    let search_input = cx.new_view(|cx| SearchInput::new(cx, search_state.clone()));

    // Type initial query
    search_input.update(cx, |view, cx| {
        view.set_query("app".to_string(), cx);
    });

    cx.run_until_parked();

    // Add autocomplete suggestions
    search_state.update(cx, |state, cx| {
        state.autocomplete_suggestions = vec![
            AutocompleteSuggestion {
                term: "application".to_string(),
                normalized_term: "application".to_string(),
                url: Some("https://example.com/application".to_string()),
                score: 0.92,
            },
            AutocompleteSuggestion {
                term: "api".to_string(),
                normalized_term: "api".to_string(),
                url: Some("https://example.com/api".to_string()),
                score: 0.88,
            }
        ];
        state.selected_suggestion_index = 0;
        state.show_autocomplete = true;
        cx.notify();
    });

    // Simulate clicking on "api" suggestion (index 1)
    search_input.update(cx, |view, cx| {
        view.handle_suggestion_click(1, cx);
    });

    cx.run_until_parked();

    // Verify input updated to "api"
    search_input.update(cx, |view, cx| {
        assert_eq!(view.get_query(cx), "api", "Input should update to clicked suggestion");
    });
}
