use terraphim_types::{Document, SearchQuery};

/// Abstraction over a data source that can search and return documents.
pub trait HaystackProvider {
    type Error: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;

    #[allow(async_fn_in_trait)]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::NormalizedTermValue;

    /// A concrete test provider that returns pre-configured documents.
    struct TestProvider {
        documents: Vec<Document>,
    }

    impl TestProvider {
        fn with_docs(documents: Vec<Document>) -> Self {
            Self { documents }
        }

        fn empty() -> Self {
            Self {
                documents: Vec::new(),
            }
        }
    }

    /// Error type for the test provider.
    #[derive(Debug)]
    struct TestProviderError(String);

    impl std::fmt::Display for TestProviderError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestProviderError: {}", self.0)
        }
    }

    impl HaystackProvider for TestProvider {
        type Error = TestProviderError;

        async fn search(&self, _query: &SearchQuery) -> Result<Vec<Document>, Self::Error> {
            Ok(self.documents.clone())
        }
    }

    /// A provider that always returns an error.
    struct FailingProvider;

    impl HaystackProvider for FailingProvider {
        type Error = TestProviderError;

        async fn search(&self, _query: &SearchQuery) -> Result<Vec<Document>, Self::Error> {
            Err(TestProviderError("search failed".to_string()))
        }
    }

    fn make_query(term: &str) -> SearchQuery {
        SearchQuery {
            search_term: NormalizedTermValue::from(term),
            ..Default::default()
        }
    }

    fn make_document(id: &str, title: &str) -> Document {
        Document {
            id: id.to_string(),
            title: title.to_string(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_provider_returns_documents() {
        let provider = TestProvider::with_docs(vec![
            make_document("1", "First Result"),
            make_document("2", "Second Result"),
        ]);
        let results = provider.search(&make_query("test")).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "First Result");
        assert_eq!(results[1].title, "Second Result");
    }

    #[tokio::test]
    async fn test_provider_returns_empty_results() {
        let provider = TestProvider::empty();
        let results = provider.search(&make_query("nothing")).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_provider_error_propagation() {
        let provider = FailingProvider;
        let result = provider.search(&make_query("test")).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("search failed"));
    }

    #[tokio::test]
    async fn test_error_type_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<TestProviderError>();
    }

    #[tokio::test]
    async fn test_provider_with_empty_search_term() {
        let provider = TestProvider::with_docs(vec![make_document("1", "Doc")]);
        let results = provider.search(&make_query("")).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_provider_with_special_characters_in_query() {
        let provider = TestProvider::with_docs(vec![make_document("1", "Doc")]);
        let results = provider
            .search(&make_query("test & <script>alert(1)</script>"))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_searches() {
        let provider =
            std::sync::Arc::new(TestProvider::with_docs(vec![make_document("1", "Result")]));

        let mut handles = Vec::new();
        for _ in 0..10 {
            let p = provider.clone();
            handles.push(tokio::spawn(async move {
                p.search(&make_query("concurrent")).await.unwrap()
            }));
        }

        for handle in handles {
            let results = handle.await.unwrap();
            assert_eq!(results.len(), 1);
        }
    }
}
