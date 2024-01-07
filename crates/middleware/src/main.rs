
use terraphim_types::{ConfigState, SearchQuery};
use terraphim_pipeline::IndexedDocument; 

mod lib;
use lib::run_ripgrep_service_and_index;

#[tokio::main]    
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let needle = "life cycle framework".to_string();
    // let needle = "trained operators and maintainers".to_string();
    let haystack = "../../../INCOSE-Systems-Engineering-Handbook".to_string();
    let mut config_state= ConfigState::new().await.expect("Failed to load config state");
    run_ripgrep_service_and_index(config_state.clone(),needle.clone(), haystack).await;
    let role_name = "System Operator".to_string();
    println!("{:#?}", role_name);
    println!("Searching articles with query: {needle} {role_name}");
    let search_query = SearchQuery {
        search_term: needle,
        role: Some(role_name),
        skip: Some(0),
        limit: Some(10),
    };
    
    let (docs, nodes): (Vec<IndexedDocument>, Vec<u64>) = config_state.search_articles(search_query).await.expect("Failed to search articles");
    
    // let docs: Vec<IndexedDocument> = documents.into_iter().map(|(_id, doc) | doc).collect();
    println!("Found articles: {docs:?}");
    println!("Found nodes: {nodes:?}");
    // send the results to the stream as well (only for testing)
    // for doc in docs.iter() {
    //     println!("Found articles: {:#?}", doc);
    // }

    Ok(())
}
