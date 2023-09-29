use log::debug;
use log::info;
use opendal::layers::LoggingLayer;
use opendal::services;
use opendal::Operator;
use opendal::Result;
use opendal::Scheme;
use std::collections::HashMap;

use std::env;
use std::fs;
use serde_json::Value;
use serde::{Serialize, Deserialize};

#[derive(Debug,Serialize, Deserialize)]
pub struct Config {
    global_shortcut: String,
    roles: HashMap<String, Role>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Role {
    shortname: Option<String>,
    name: String,
    relevance_function: String,
    theme: String,
    #[serde(rename = "serverUrl")]
    server_url: String,
    automata_url: Option<String>,
    #[serde(rename = "matcherMapUrl")]
    matcher_map_url: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
    let schemes = [
        Scheme::S3,
        Scheme::Memory,
        Scheme::Dashmap,
        Scheme::Sled,
        Scheme::Redis,
    ];


    let json_str=std::fs::read_to_string("test-data/default_config.json").unwrap();

    // let config: Config = serde_json::from_str(&json_str).unwrap();
    // println!("config: {:?}", config);
    let deserializer = &mut serde_json::Deserializer::from_str(&json_str);

    let result: std::result::Result<Config,_> = serde_path_to_error::deserialize(deserializer);

    match result {
        Ok(_) => {
            for role in result.unwrap().roles.into_values() {
                
                    println!("key: {}, value: {:?}", role.name, role);
                for scheme in schemes.iter() {
                    info!("scheme: {:?}", scheme);
                    read_and_write(*scheme, &role.name.as_str(), serde_json::to_string(&role).unwrap()).await?;
                }
            }
            
        },
        Err(err) => {
            let path = err.path().to_string();
            println!("Error at path: {}", path);
        }
    }

    Ok(())
}

async fn read_and_write(scheme: Scheme, key:&str, value:String) -> Result<()> {
    // Write data into object test and read it back
    let op = match scheme {
        Scheme::S3 => {
            let op = init_operator_via_map()?;
            debug!("operator: {op:?}");
            op
        }
        Scheme::Dashmap => {
            let builder = services::Dashmap::default();
            // Init an operator
            let op = Operator::new(builder)?
                // Init with logging layer enabled.
                .layer(LoggingLayer::default())
                .finish();
            debug!("operator: {op:?}");
            op
        }
        Scheme::Sled => {
            let mut builder = services::Sled::default();
            builder.datadir("/tmp/opendal/sled");
            // Init an operator
            let op = Operator::new(builder)?
                // Init with logging layer enabled.
                .layer(LoggingLayer::default())
                .finish();
            debug!("operator: {op:?}");
            op
        }
        Scheme::Redis => {
            let mut builder = services::Redis::default();
            builder.endpoint("redis://localhost:6379");
            // Init an operator
            let op = Operator::new(builder)?
                // Init with logging layer enabled.
                .layer(LoggingLayer::default())
                .finish();
            debug!("operator: {op:?}");
            op
        }
        _ => {
            let builder = services::Memory::default();
            // Init an operator
            let op = Operator::new(builder)?
                // Init with logging layer enabled.
                .layer(LoggingLayer::default())
                .finish();
            debug!("operator: {op:?}");
            op
        }
    };
    // Write data into object test.
    
    op.write(key, value).await?;

    // Read data from object.
    let bs = op.read(key).await?;
    info!("content: {}", String::from_utf8_lossy(&bs));

    // Get object metadata.
    let meta = op.stat(key).await?;
    info!("meta: {:?}", meta);

    Ok(())
}

fn init_operator_via_map() -> Result<Operator> {
    // setting up the credentials
    let access_key_id =
        env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID is set and a valid String");
    let secret_access_key =
        env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_ACCESS_KEY_ID is set and a valid String");

    let mut map = HashMap::default();
    map.insert("bucket".to_string(), "test".to_string());
    map.insert("region".to_string(), "us-east-1".to_string());
    map.insert("endpoint".to_string(), "http://rpi4node3:8333".to_string());
    map.insert("access_key_id".to_string(), access_key_id);
    map.insert("secret_access_key".to_string(), secret_access_key);

    let op = Operator::via_map(Scheme::S3, map)?;
    Ok(op)
}