use dioxus::prelude::*;
use terraphim_types::Document;

#[derive(Clone)]
pub struct SearchState {
    input: Signal<String>,
    results: Signal<Vec<Document>>,
    loading: Signal<bool>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            input: Signal::new(String::new()),
            results: Signal::new(Vec::new()),
            loading: Signal::new(false),
        }
    }

    pub fn input(&self) -> String {
        self.input.read().clone()
    }

    pub fn set_input(&self, value: String) {
        self.input.set(value);
    }

    pub fn results(&self) -> Vec<Document> {
        self.results.read().clone()
    }

    pub fn set_results(&self, results: Vec<Document>) {
        self.results.set(results);
    }

    pub fn is_loading(&self) -> bool {
        *self.loading.read()
    }

    pub fn set_loading(&self, loading: bool) {
        self.loading.set(loading);
    }
}
