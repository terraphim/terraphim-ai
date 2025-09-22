use terraphim_types::{Document, SearchQuery};

pub trait HaystackProvider {
    type Error: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;

    #[allow(async_fn_in_trait)]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>, Self::Error>;
}
