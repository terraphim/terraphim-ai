use terraphim_types::NormalizedTermValue;

pub struct AutocompleteService;

impl AutocompleteService {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_suggestions(&self, query: &str, limit: usize) -> Vec<String> {
        // TODO: Integrate with terraphim_automata
        vec![]
    }
}
