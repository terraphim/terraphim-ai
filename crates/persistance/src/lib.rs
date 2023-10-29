pub mod config;
use opendal::Scheme;
use opendal::{Operator, Result};
use std::collections::{HashMap, BTreeMap};
use async_trait::async_trait;
use std::env;
use serde::{Serialize};
use anyhow::anyhow;
use config::Config;
use dirs::config_dir;
use async_once_cell::OnceCell as AsyncOnceCell;

static STATE: AsyncOnceCell<State> = AsyncOnceCell::new();

pub struct State {
    pub ops: Vec<(Operator,u128)>,
    pub fastest_op: Operator,
}

impl State {
    pub async fn instance() -> &'static State {
        STATE
        .get_or_init(async {
            let d = config_dir().ok_or_else(|| anyhow!("unknown config dir")).unwrap();
            let default_config_path = d.join("terraphim/config.toml");
            let cfg = Config::load(default_config_path.as_path()).unwrap();
            println!("cfg: {:?}", cfg);
            let ops = cfg.parse_profiles().await.unwrap();
            let fastest_op = ops
                .iter()
                .min_by_key(|op| op.1)
                .ok_or_else(|| anyhow!("No operators provided")).unwrap().0.clone();
            State {
                ops,
                fastest_op,
            }

        }).await
    }
}
#[async_trait]
pub trait Persistable: Serialize + serde::de::DeserializeOwned {
    fn new()->Self;
    async fn save(&self)->Result<()>;
    async fn load(&mut self, key:&str)->Result<Self> where Self: Sized;
    async fn load_config(&self)->Result<(Vec<(Operator,u128)>,Operator)> {
        let state = State::instance().await;
        Ok((state.ops.clone(), state.fastest_op.clone()))
        // let d = config_dir().ok_or_else(|| anyhow!("unknown config dir")).unwrap();
        // let default_config_path = d.join("terraphim/config.toml");
        // let cfg = config::Config::load(default_config_path.as_path()).unwrap();
        // println!("cfg: {:?}", cfg);
        // let ops = cfg.parse_profiles().await?;
        // let fastest_op = ops
        //     .iter()
        //     .min_by_key(|op| op.1)
        //     .ok_or_else(|| anyhow!("No operators provided")).unwrap().0.clone();
    
        // Ok((ops,fastest_op))
    }
    async fn save_to_operator(&self, op: &Operator) -> Result<()> {
        let key = self.get_key();
        let serde_str=serde_json::to_string(&self).unwrap();
        op.write(key.as_str(), serde_str).await?;
        Ok(())
    }
    
    async fn load_from_operator(key:&str, op: &Operator) -> Result<Self> where Self: Sized {
        let bs = op.read(key).await?;
        let obj = serde_json::from_slice(&bs).unwrap();
        Ok(obj)
    }
    
    fn get_key(&self) -> String;
}

// pub fn init_operator_via_map()->Result<Operator>{

//             let builder = services::Dashmap::default();
//             // Init an operator
//             let op = Operator::new(builder)?
//                 // Init with logging layer enabled.
//                 .layer(LoggingLayer::default())
//                 .finish();
//             debug!("operator: {op:?}");
//             Ok(op)
// }



pub fn init_operator_via_map() -> Result<Operator> {
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