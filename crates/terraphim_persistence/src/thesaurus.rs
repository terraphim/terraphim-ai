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

    /// returns key + .json
    fn get_key(&self) -> String {
        format!("thesaurus_{}.json", self.normalize_key(&self.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
// Test saving and loading a struct to a dashmap profile
#[tokio::test]
#[serial_test::serial]
async fn test_save_and_load() -> Result<()> {

    // Create a test object
    let test_obj = Thesaurus::new("Test Thesaurus".to_string());

    // Save the object
    test_obj.save_to_one("dash").await?;

    // Load the object
    let mut loaded_obj = Thesaurus::new("Test Thesaurus".to_string());
    loaded_obj = loaded_obj.load().await?;

    // Compare the original and loaded objects
    assert_eq!(test_obj, loaded_obj, "Loaded object does not match the original");

    Ok(())
}
/// Test saving and loading a struct to all profiles
#[tokio::test]
#[serial_test::serial]
async fn test_save_and_load_all() -> Result<()> {
    // Create a test object
    let test_obj = Thesaurus::new("Test Thesaurus".to_string());

    // Save the object
    test_obj.save().await?;

    // Load the object
    let mut loaded_obj = Thesaurus::new("Test Thesaurus".to_string());
    loaded_obj = loaded_obj.load().await?;

    // Compare the original and loaded objects
    assert_eq!(test_obj, loaded_obj, "Loaded object does not match the original");

        Ok(())
    }
}
