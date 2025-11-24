use gpui::*;
use terraphim_automata::Autocomplete;

/// Autocomplete state management
pub struct AutocompleteState {
    suggestions: Vec<Suggestion>,
    selected_index: usize,
}

#[derive(Clone, Debug)]
pub struct Suggestion {
    pub term: String,
    pub definition: Option<String>,
    pub from_kg: bool,
}

impl AutocompleteState {
    pub fn new(_cx: &mut ModelContext<Self>) -> Self {
        Self {
            suggestions: vec![],
            selected_index: 0,
        }
    }

    pub async fn fetch_suggestions(&mut self, _query: &str) -> Vec<Suggestion> {
        // TODO: Integrate with terraphim_automata
        // For now, return empty suggestions
        vec![]
    }

    pub fn select_next(&mut self, cx: &mut ModelContext<Self>) {
        if !self.suggestions.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.suggestions.len() - 1);
            cx.notify();
        }
    }

    pub fn select_previous(&mut self, cx: &mut ModelContext<Self>) {
        self.selected_index = self.selected_index.saturating_sub(1);
        cx.notify();
    }
}
