use terraphim_middleware;
use terraphim_middleware::search_haystacks;
use terraphim_pipeline::IndexedDocument;
use terraphim_types::{merge_and_serialize, ConfigState, SearchQuery};

use terraphim_middleware::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let config_state = ConfigState::new().await?;
    let needle = "life cycle framework".to_string();
    // let needle = "trained operators and maintainers".to_string();
    let role_name = "System Operator".to_string();
    println!("{:#?}", role_name);
    println!("Searching articles with query: {needle} {role_name}");
    let search_query = SearchQuery {
        search_term: needle.clone(),
        role: Some(role_name),
        skip: Some(0),
        limit: Some(10),
    };

    // let articles_cached_left = run_ripgrep_service_and_index(config_state.clone(),needle.clone(), haystack).await;
    // println!("articles_cached_left: {:#?}", articles_cached_left.clone());

    let articles_cached = search_haystacks(config_state.clone(), search_query.clone()).await?;
    let docs: Vec<IndexedDocument> = config_state
        .search_articles(search_query)
        .await
        .expect("Failed to search articles");
    let articles = merge_and_serialize(articles_cached, docs);
    println!("Articles: {articles:?}");

    Ok(())
}
