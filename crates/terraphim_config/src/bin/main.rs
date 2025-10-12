use terraphim_automata::AutomataPath;
use terraphim_config::{
    ConfigBuilder, Haystack, KnowledgeGraph, Result, Role, ServiceType, TerraphimConfigError,
};
use terraphim_persistence::Persistable;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .map_err(TerraphimConfigError::TracingSubscriber);

    let mut config = ConfigBuilder::new()
        .add_role("Engineer", {
            let mut engineer_role = Role::new("Engineer");
            engineer_role.shortname = Some("Engineer".to_string());
            engineer_role.theme = "lumen".to_string();
            engineer_role.kg = Some(KnowledgeGraph {
                automata_path: Some(AutomataPath::local_example()),
                knowledge_graph_local: None,
                public: false,
                publish: false,
            });
            engineer_role.haystacks = vec![Haystack {
                location: "localsearch".to_string(),
                service: ServiceType::Ripgrep,
                read_only: false,
                fetch_content: false,
                atomic_server_secret: None,
                extra_parameters: std::collections::HashMap::new(),
            }];
            engineer_role
        })
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
