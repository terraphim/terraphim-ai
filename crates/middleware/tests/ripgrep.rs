#[cfg(test)]
mod tests {
    use terraphim_config::{Config, ConfigState};
    use terraphim_middleware::search_haystacks;
    use terraphim_types::IndexedDocument;
    use terraphim_types::{merge_and_serialize, SearchQuery};

    use terraphim_middleware::Result;

    #[tokio::test]
    async fn test_roundtrip() -> Result<()> {
        let mut config = Config::new();
        let config_state = ConfigState::new(&mut config).await?;

        let role_name = "System Operator".to_string();
        let search_query = SearchQuery {
            search_term: "life cycle framework".to_string(),
            role: Some(role_name.clone()),
            skip: Some(0),
            limit: Some(10),
        };
        println!("Searching articles with query: {search_query:?} {role_name}");

        let cached_articles = search_haystacks(config_state.clone(), search_query.clone()).await?;
        let docs: Vec<IndexedDocument> = config_state.search_articles(&search_query).await;
        let articles = merge_and_serialize(cached_articles, docs);
        log::debug!("Final articles: {articles:?}");

        Ok(())
    }
}
