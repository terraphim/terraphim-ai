use anyhow::Result;
use serde::{Serialize, Deserialize};

pub struct StorageService;

impl StorageService {
    pub fn new() -> Self {
        Self
    }

    pub async fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        // TODO: Implement filesystem-based storage
        Ok(())
    }

    pub async fn load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        // TODO: Implement filesystem-based storage
        Ok(None)
    }
}
