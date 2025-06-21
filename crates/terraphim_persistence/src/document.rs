use crate::{Persistable, Result};
use async_trait::async_trait;
use terraphim_types::Document;

/// Provide `Persistable` implementation for Terraphim [`Document`].
///
/// Each document is stored as a single JSON file across all configured
/// [`opendal`] profiles. The filename is derived from the `id` field and
/// normalised to be a safe filesystem key: `document_<id>.json`.
///
/// This allows us to persist individual articles that the user edits in the
/// desktop UI and reload them later on any device/profile.
#[async_trait]
impl Persistable for Document {
    /// Create a new, *empty* document instance using the provided key as the
    /// `id`.  All other fields are initialised with their default values.
    fn new(key: String) -> Self {
        let mut doc = Document::default();
        doc.id = key;
        doc
    }

    /// Save to a single profile.
    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await?;
        Ok(())
    }

    /// Save to *all* profiles.
    async fn save(&self) -> Result<()> {
        // Persist to the fastest operator as well as all other profiles.
        self.save_to_all().await
    }

    /// Load this document (identified by `id`) from the fastest operator.
    async fn load(&mut self) -> Result<Self> {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        let obj = self.load_from_operator(&key, op).await?;
        Ok(obj)
    }

    /// Compute the storage key – `document_<normalized-id>.json`.
    fn get_key(&self) -> String {
        format!("document_{}.json", self.normalize_key(&self.id))
    }
} 