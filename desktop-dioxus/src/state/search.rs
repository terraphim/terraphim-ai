use dioxus::prelude::*;
use terraphim_types::Document;

#[derive(Clone)]
pub struct SearchState {
    input: Signal<String>,
    results: Signal<Vec<Document>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            input: Signal::new(String::new()),
            results: Signal::new(Vec::new()),
            loading: Signal::new(false),
            error: Signal::new(None),
        }
    }

    pub fn input(&self) -> String {
        self.input.read().clone()
    }

    pub fn set_input(&mut self, value: String) {
        self.input.set(value);
    }

    pub fn results(&self) -> Vec<Document> {
        self.results.read().clone()
    }

    pub fn set_results(&mut self, results: Vec<Document>) {
        self.results.set(results);
    }

    pub fn is_loading(&self) -> bool {
        *self.loading.read()
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading.set(loading);
    }

    pub fn error(&self) -> Option<String> {
        self.error.read().clone()
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error.set(error);
    }

    pub fn clear(&mut self) {
        self.input.set(String::new());
        self.results.set(Vec::new());
        self.loading.set(false);
        self.error.set(None);
    }
}
