pub mod error;
pub mod settings;

use async_once_cell::OnceCell as AsyncOnceCell;
use async_trait::async_trait;
use opendal::Operator;
use serde::{de::DeserializeOwned, Serialize};
use terraphim_settings::Settings;

use std::collections::HashMap;

pub use error::{Error, Result};

static DEVICE_STORAGE: AsyncOnceCell<DeviceStorage> = AsyncOnceCell::new();

pub struct DeviceStorage {
    pub ops: HashMap<String, (Operator, u128)>,
    pub fastest_op: Operator,
}

impl DeviceStorage {
    pub async fn instance() -> Result<&'static DeviceStorage> {
        let storage = DEVICE_STORAGE
            .get_or_try_init(async {
                let initialized_storage = init_device_storage().await?;
                Ok::<DeviceStorage, Error>(initialized_storage)
            })
            .await?;
        Ok(storage)
    }
}

async fn init_device_storage() -> Result<DeviceStorage> {
    let settings = Settings::load_from_env_and_file(None)?;
    log::info!("Loaded settings: {:?}", settings);

    let operators = settings::parse_profiles(&settings).await?;
    let mut ops_vec: Vec<(&String, &(Operator, u128))> = operators.iter().collect();
    ops_vec.sort_by_key(|&(_, (_, speed))| speed);

    let ops: HashMap<String, (Operator, u128)> = ops_vec
        .into_iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let fastest_op = match ops.values().next() {
        Some((op, _)) => op.clone(),
        None => return Err(Error::NoOperator),
    };

    Ok(DeviceStorage { ops, fastest_op })
}

#[async_trait]
pub trait Persistable: Serialize + DeserializeOwned {
    /// Create a new instance
    fn new() -> Self;

    /// Save to all profiles
    async fn save(&self) -> Result<()>;

    /// Save to a single profile
    async fn save_to_one(&self, profile_name: &str) -> Result<()>;

    /// Load a key from the fastest operator
    async fn load(&mut self, key: &str) -> Result<Self>
    where
        Self: Sized;

    /// Load the configuration
    async fn load_config(&self) -> Result<(HashMap<String, (Operator, u128)>, Operator)> {
        let state = DeviceStorage::instance().await?;
        Ok((state.ops.clone(), state.fastest_op.clone()))
    }

    /// Save to all profiles
    async fn save_to_all(&self) -> Result<()> {
        let (ops, _fastest_op) = &self.load_config().await?;
        let key = self.get_key();
        let serde_str = serde_json::to_string(&self)?;
        for (op, _time) in ops.values() {
            op.write(&key, serde_str.clone()).await?;
        }
        Ok(())
    }

    /// Save to a single profile
    async fn save_to_profile(&self, profile_name: &str) -> Result<()> {
        let (ops, _fastest_op) = &self.load_config().await?;
        let key = self.get_key();
        let serde_str = serde_json::to_string(&self)?;

        ops.get(profile_name)
            .ok_or_else(|| {
                Error::Profile(format!(
                    "Unknown profile name: {profile_name}. Available profiles: {}",
                    ops.keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<&str>>()
                        .join(", ")
                ))
            })?
            .0
            .write(&key, serde_str.clone())
            .await
            .map_err(Error::OpenDal)?;

        Ok(())
    }

    /// Load from the fastest operator
    async fn load_from_operator(&self, key: &str, _op: &Operator) -> Result<Self>
    where
        Self: Sized,
    {
        let (_ops, fastest_op) = &self.load_config().await?;
        let bs = fastest_op.read(key).await?;
        let obj = serde_json::from_slice(&bs)?;
        Ok(obj)
    }

    fn get_key(&self) -> String;
}
