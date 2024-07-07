use crate::Result;
use async_trait::async_trait;
use terraphim_types::Thesaurus;

use crate::Persistable;

#[async_trait]
impl Persistable for Thesaurus {
    fn new(key: String) -> Self {
        Thesaurus::new(key)
    }

    /// Save to a single profile
    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await?;
        Ok(())
    }

    // Saves to all profiles
    async fn save(&self) -> Result<()> {
        let _op = &self.load_config().await?.1;
        let _ = self.save_to_all().await?;
        Ok(())
    }

    /// Load key from the fastest operator
    async fn load(&mut self) -> Result<Self> {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        let obj = self.load_from_operator(&key, op).await?;
        Ok(obj)
    }

    /// returns ulid as key + .json
    fn get_key(&self) -> String {
        format!("thesaurus_{}.json", self.name())
    }
}
