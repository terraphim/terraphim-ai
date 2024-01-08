use persistance::Persistable;
use terraphim_config::TerraphimConfig;

use terraphim_config::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
    let config = TerraphimConfig::new();
    let json_str = serde_json::to_string_pretty(&config)?;
    println!("json_str: {:?}", json_str);

    println!("key: {}", config.get_key());
    config.save().await?;
    let profile_name = "s3".to_string();
    config.save_to_one(profile_name).await?;

    println!("saved obj: {:?} to all", config);
    let (_ops, fastest_op) = config.load_config().await?;
    println!("fastest_op: {:?}", fastest_op.info());

    let mut obj1 = TerraphimConfig::new();
    let key = config.get_key();
    // println!("key: {}", key);
    obj1 = obj1.load(&key).await?;
    println!("loaded obj: {:?}", obj1);
    assert_eq!(obj1.get_key(), config.get_key());

    Ok(())
}
