use persistence::Persistable;
use terraphim_config::{Config, Result, TerraphimConfigError};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .map_err(|e| TerraphimConfigError::TracingSubscriber(e));

    let config = Config::new();
    let json_str = serde_json::to_string_pretty(&config)?;
    println!("json_str: {:?}", json_str);

    println!("key: {}", config.get_key());
    config.save().await?;
    config.save_to_one("dash").await?;

    println!("saved obj: {:?} to all", config);
    let (_ops, fastest_op) = config.load_config().await?;
    println!("fastest_op: {:?}", fastest_op.info());

    let mut obj1 = Config::new();
    let key = config.get_key();
    // println!("key: {}", key);
    obj1 = obj1.load(&key).await?;
    println!("loaded obj: {:?}", obj1);
    assert_eq!(obj1.get_key(), config.get_key());

    Ok(())
}
