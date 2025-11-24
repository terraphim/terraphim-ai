use gpui::*;
use terraphim_types::Document;

/// Search state management
pub struct SearchState {
    query: String,
    results: Vec<Document>,
    loading: bool,
}

impl SearchState {
    pub fn new(_cx: &mut ModelContext<Self>) -> Self {
        log::info!("SearchState initialized");

        Self {
            query: String::new(),
            results: vec![],
            loading: false,
        }
    }

    pub fn search(&mut self, query: String, cx: &mut ModelContext<Self>) {
        self.query = query.clone();
        self.loading = true;
        cx.notify();

        log::info!("Search initiated for query: {}", query);

        // TODO: Integrate with terraphim_service for actual search
        // For now, just mark as not loading after a brief delay
        cx.spawn(|this, mut cx| async move {
            // Simulate search delay
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            this.update(&mut cx, |this, cx| {
                this.loading = false;
                this.results = vec![]; // Empty results for now
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
