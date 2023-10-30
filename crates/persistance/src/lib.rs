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
        // TODO: add for each operator save
        // TODO add load from fastest operator
        let state = State::instance().await;
        Ok((state.ops.clone(), state.fastest_op.clone()))

    }
    async fn save_to_operator(&self, op: &Operator) -> Result<()> {
        let (ops, fastest_op) = &self.load_config().await?;
        let key = self.get_key();
        let serde_str=serde_json::to_string(&self).unwrap();
        for (op, _time) in ops {
            op.write(&key, serde_str.clone()).await?;
        }
        Ok(())
    }
    
    async fn load_from_operator(&self, key:&str, op: &Operator) -> Result<Self> where Self: Sized {
        let (ops, fastest_op) = &self.load_config().await?;
        let bs = fastest_op.read(key).await?;
        let obj = serde_json::from_slice(&bs).unwrap();
        Ok(obj)
    }
    
    fn get_key(&self) -> String;
}

