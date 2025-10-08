use ahash::AHashMap;

use terraphim_automata::AutomataPath;
use terraphim_config::{
    default_haystack_weight, ConfigBuilder, Haystack, KnowledgeGraph, Result, Role, ServiceType,
    TerraphimConfigError,
};
use terraphim_persistence::Persistable;
use terraphim_types::RelevanceFunction;

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
                name: "Engineer".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "lumen".to_string(),
                kg: Some(KnowledgeGraph {
                    automata_path: Some(AutomataPath::local_example()),
                    knowledge_graph_local: None,
                    public: false,
                    publish: false,
                }),
                haystacks: vec![Haystack {
                    location: "localsearch".to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                    weight: default_haystack_weight(),
                    fetch_content: false,
                }],
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                #[cfg(feature = "openrouter")]
                openrouter_auto_summarize: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_chat_system_prompt: None,
                #[cfg(feature = "openrouter")]
                openrouter_chat_model: None,
                llm_system_prompt: None,
                extra: AHashMap::new(),
            },
        )
        .build()?;

    let json_str = serde_json::to_string_pretty(&config)?;
    println!("json_str: {:?}", json_str);

    println!("key: {}", config.get_key());
    config.save().await?;
    config.save_to_one("dashmap").await?;

    println!("saved obj: {:?} to all", config);
    let (_ops, fastest_op) = config.load_config().await?;
    println!("fastest_op: {:?}", fastest_op.info());

    let key = config.get_key();
    println!("key: {}", key);
    let loaded_config = config.load().await?;
    println!("loaded obj: {:?}", loaded_config);
    assert_eq!(loaded_config.get_key(), config.get_key());

    Ok(())
}
