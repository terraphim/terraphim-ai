use std::path::PathBuf;

use ahash::AHashMap;
use persistence::Persistable;
use terraphim_automata::AutomataPath;
use terraphim_config::{
    ConfigBuilder, Haystack, KnowledgeGraph, Result, Role, ServiceType, TerraphimConfigError,
};
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .map_err(TerraphimConfigError::TracingSubscriber);

    let mut config = ConfigBuilder::new()
        .add_role(
            "Engineer",
            Role {
                shortname: Some("Engineer".to_string()),
                name: "Engineer".to_string(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "lumen".to_string(),
                server_url: Url::parse("http://localhost:8000/documents/search").unwrap(),
                kg: KnowledgeGraph {
                    automata_path: AutomataPath::remote_example(),
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from("~/pkm"),
                    public: true,
                    publish: true,
                },
                haystacks: vec![Haystack {
                    path: PathBuf::from("localsearch"),
                    service: ServiceType::Ripgrep,
                }],
                extra: AHashMap::new(),
            },
        )
        .build()?;

    let json_str = serde_json::to_string_pretty(&config)?;
    println!("json_str: {:?}", json_str);

    println!("key: {}", config.get_key());
    config.save().await?;
    config.save_to_one("dash").await?;

    println!("saved obj: {:?} to all", config);
    let (_ops, fastest_op) = config.load_config().await?;
    println!("fastest_op: {:?}", fastest_op.info());

    let key = config.get_key();
    // println!("key: {}", key);
    let loaded_config = config.load(&key).await?;
    println!("loaded obj: {:?}", loaded_config);
    assert_eq!(loaded_config.get_key(), config.get_key());

    Ok(())
}
