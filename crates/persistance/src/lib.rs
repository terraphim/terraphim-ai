pub mod settings;
use anyhow::anyhow;
use async_once_cell::OnceCell as AsyncOnceCell;
use async_trait::async_trait;


use opendal::{Operator, Result};
use serde::Serialize;
use std::collections::{HashMap};
use terraphim_settings::Settings;
static DEVICE_STORAGE: AsyncOnceCell<DeviceStorage> = AsyncOnceCell::new();

pub struct DeviceStorage {
    pub ops: HashMap<String, (Operator, u128)>,
    pub fastest_op: Operator,
}

impl DeviceStorage {
    pub async fn instance() -> &'static DeviceStorage {
        DEVICE_STORAGE
            .get_or_init(async {

                let device_settings = Settings::load_from_env_and_file(None);
                println!("cfg: {:?}", device_settings);
                let ops = settings::parse_profiles(&device_settings).await.unwrap();
                let mut ops_vec: Vec<(&String, &(Operator, u128))> = ops.iter().collect();
                ops_vec.sort_by_key(|&(_, (_, speed))| speed);
                let ops: HashMap<String, (Operator, u128)> = ops_vec
                    .into_iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let fastest_op = ops
                    .values()
                    .next()
                    .ok_or_else(|| anyhow!("No operators provided"))
                    .unwrap()
                    .0
                    .clone();
                DeviceStorage { ops, fastest_op }
            })
            .await
    }
}

#[async_trait]
pub trait Persistable: Serialize + serde::de::DeserializeOwned {
    fn new() -> Self;
    async fn save(&self) -> Result<()>;
    async fn save_to_one(&self, profile_name: String) -> Result<()>;
    async fn load(&mut self, key: &str) -> Result<Self>
    where
        Self: Sized;
    async fn load_config(&self) -> Result<(HashMap<String, (Operator, u128)>, Operator)> {
        let state = DeviceStorage::instance().await;
        Ok((state.ops.clone(), state.fastest_op.clone()))
    }
    async fn save_to_all(&self) -> Result<()> {
        let (ops, _fastest_op) = &self.load_config().await?;
        let key = self.get_key();
        let serde_str = serde_json::to_string(&self).unwrap();
        for (_profile, (op, _time)) in ops {
            op.write(&key, serde_str.clone()).await?;
        }
        Ok(())
    }
    async fn save_to_profile(&self, profile_name: String) -> Result<()> {
        let (ops, _fastest_op) = &self.load_config().await?;
        let key = self.get_key();
        let serde_str = serde_json::to_string(&self).unwrap();
        ops.get(&profile_name)
            .ok_or_else(|| anyhow!("unknown profile: {}", profile_name))
            .unwrap()
            .0
            .write(&key, serde_str.clone())
            .await?;
        Ok(())
    }

    async fn load_from_operator(&self, key: &str, _op: &Operator) -> Result<Self>
    where
        Self: Sized,
    {
        let (_ops, fastest_op) = &self.load_config().await?;
        let bs = fastest_op.read(key).await?;
        let obj = serde_json::from_slice(&bs).unwrap();
        Ok(obj)
    }

    fn get_key(&self) -> String;
}
