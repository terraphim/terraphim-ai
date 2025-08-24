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
        let normalized = self.normalize_key(&self.id);
        let key = format!("document_{}.json", normalized);
        
        log::debug!("Document key generation: id='{}' → key='{}'", self.id, key);
        
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    async fn init_test_persistence() -> Result<()> {
        crate::DeviceStorage::init_memory_only().await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_document_save_and_load() -> Result<()> {
        init_test_persistence().await?;

        // Create a test document
        let mut test_doc = Document::default();
        test_doc.id = "test-document-123".to_string();
        test_doc.title = "Test Document".to_string();
        test_doc.body = "This is a test document for persistence validation.".to_string();
        test_doc.url = "https://example.com/test-document".to_string();
        test_doc.description = Some("Test document description".to_string());
        test_doc.summarization = Some("Test document AI-generated summary".to_string());
        test_doc.tags = Some(vec!["test".to_string(), "persistence".to_string()]);
        test_doc.rank = Some(100);

        // Save the document
        test_doc.save_to_one("memory").await?;

        // Load the document
        let mut loaded_doc = Document::new("test-document-123".to_string());
        loaded_doc = loaded_doc.load().await?;

        // Verify all fields match
        assert_eq!(loaded_doc.id, test_doc.id, "Document IDs should match");
        assert_eq!(loaded_doc.title, test_doc.title, "Document titles should match");
        assert_eq!(loaded_doc.body, test_doc.body, "Document bodies should match");
        assert_eq!(loaded_doc.url, test_doc.url, "Document URLs should match");
        assert_eq!(loaded_doc.description, test_doc.description, "Document descriptions should match");
        assert_eq!(loaded_doc.summarization, test_doc.summarization, "Document summarizations should match");
        assert_eq!(loaded_doc.tags, test_doc.tags, "Document tags should match");
        assert_eq!(loaded_doc.rank, test_doc.rank, "Document ranks should match");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_document_save_and_load_all() -> Result<()> {
        init_test_persistence().await?;

        // Create a test document
        let mut test_doc = Document::default();
        test_doc.id = "test-document-all-backends".to_string();
        test_doc.title = "Test Document All Backends".to_string();
        test_doc.body = "This document tests saving to all backends.".to_string();
        test_doc.url = "https://example.com/all-backends".to_string();
        test_doc.description = Some("Testing all backends".to_string());
        test_doc.summarization = Some("Summary for all backends test".to_string());

        // Save the document to all backends
        test_doc.save().await?;

        // Load the document from fastest backend
        let mut loaded_doc = Document::new("test-document-all-backends".to_string());
        loaded_doc = loaded_doc.load().await?;

        // Verify loaded document matches original
        assert_eq!(loaded_doc.id, test_doc.id, "Document IDs should match");
        assert_eq!(loaded_doc.title, test_doc.title, "Document titles should match");
        assert_eq!(loaded_doc.body, test_doc.body, "Document bodies should match");
        assert_eq!(loaded_doc.url, test_doc.url, "Document URLs should match");
        assert_eq!(loaded_doc.description, test_doc.description, "Document descriptions should match");
        assert_eq!(loaded_doc.summarization, test_doc.summarization, "Document summarizations should match");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_document_with_special_character_ids() -> Result<()> {
        init_test_persistence().await?;

        let test_ids = vec![
            "simple-id",
            "ID_WITH_UNDERSCORES",
            "id with spaces",
            "id-with-special@chars#123",
            "http://example.com/document/456",
            "file:///path/to/document.txt",
            "a33bd45bece9c7cb", // Hash format
            "Document ID (v2.0)",
        ];

        for id in test_ids {
            println!("Testing document persistence for ID: '{}'", id);

            // Create document
            let mut test_doc = Document::default();
            test_doc.id = id.to_string();
            test_doc.title = format!("Test Document {}", id);
            test_doc.body = format!("Body content for document {}", id);
            test_doc.url = format!("https://example.com/{}", id);

            // Save document
            test_doc.save_to_one("memory").await?;

            // Load document with same ID
            let mut loaded_doc = Document::new(id.to_string());
            loaded_doc = loaded_doc.load().await?;

            // Verify
            assert_eq!(loaded_doc.id, id, "Document ID should match for '{}'", id);
            assert_eq!(loaded_doc.title, format!("Test Document {}", id), "Document title should match for '{}'", id);
            assert_eq!(loaded_doc.body, format!("Body content for document {}", id), "Document body should match for '{}'", id);

            println!("  ✅ Successfully persisted document with ID: '{}'", id);
        }

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_document_memory_backend() -> Result<()> {
        init_test_persistence().await?;

        // Create a test document
        let mut test_doc = Document::default();
        test_doc.id = "memory-test-document".to_string();
        test_doc.title = "Memory Backend Test".to_string();
        test_doc.body = "Testing memory backend persistence.".to_string();
        test_doc.url = "memory://test".to_string();

        // Save to memory backend
        test_doc.save_to_one("memory").await?;

        // Load from memory backend
        let mut loaded_doc = Document::new("memory-test-document".to_string());
        loaded_doc = loaded_doc.load().await?;

        // Verify
        assert_eq!(loaded_doc.id, test_doc.id, "Memory backend document ID should match");
        assert_eq!(loaded_doc.title, test_doc.title, "Memory backend document title should match");
        assert_eq!(loaded_doc.body, test_doc.body, "Memory backend document body should match");
        assert_eq!(loaded_doc.url, test_doc.url, "Memory backend document URL should match");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_document_key_normalization() -> Result<()> {
        let test_cases = vec![
            ("simple", "document_simple.json"),
            ("Simple Document", "document_simpledocument.json"),
            ("Document with Spaces", "document_documentwithspaces.json"),
            ("Doc-with-Dashes_123", "document_docwithdashes123.json"),
            ("Doc@Special#Chars!", "document_docspecialchars.json"),
            ("UPPERCASE_DOC", "document_uppercasedoc.json"),
            ("http://example.com/doc", "document_httpexamplecomdoc.json"),
        ];

        for (id, expected_key) in test_cases {
            let mut document = Document::default();
            document.id = id.to_string();
            let actual_key = document.get_key();
            
            assert_eq!(actual_key, expected_key, 
                      "Key normalization failed for ID '{}': got '{}', expected '{}'", 
                      id, actual_key, expected_key);
        }

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_document_persistence() -> Result<()> {
        init_test_persistence().await?;

        // Test saving and loading a minimal document
        let mut empty_doc = Document::default();
        empty_doc.id = "empty-document".to_string();
        // All other fields remain default/empty

        // Save empty document
        empty_doc.save_to_one("memory").await?;

        // Load empty document
        let mut loaded_doc = Document::new("empty-document".to_string());
        loaded_doc = loaded_doc.load().await?;

        // Verify
        assert_eq!(loaded_doc.id, "empty-document", "Empty document ID should match");
        assert_eq!(loaded_doc.title, "", "Empty document title should be empty");
        assert_eq!(loaded_doc.body, "", "Empty document body should be empty");
        assert_eq!(loaded_doc.url, "", "Empty document URL should be empty");
        assert_eq!(loaded_doc.description, None, "Empty document description should be None");
        assert_eq!(loaded_doc.tags, None, "Empty document tags should be None");
        assert_eq!(loaded_doc.rank, None, "Empty document rank should be None");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_document_with_large_content() -> Result<()> {
        init_test_persistence().await?;

        // Create a document with large content
        let large_body = "Lorem ipsum ".repeat(1000); // ~11KB of text
        let mut large_doc = Document::default();
        large_doc.id = "large-document".to_string();
        large_doc.title = "Large Document Test".to_string();
        large_doc.body = large_body.clone();
        large_doc.url = "https://example.com/large-doc".to_string();
        large_doc.description = Some("A document with large body content".to_string());

        // Save large document
        large_doc.save_to_one("memory").await?;

        // Load large document
        let mut loaded_doc = Document::new("large-document".to_string());
        loaded_doc = loaded_doc.load().await?;

        // Verify large content is preserved
        assert_eq!(loaded_doc.id, large_doc.id, "Large document ID should match");
        assert_eq!(loaded_doc.title, large_doc.title, "Large document title should match");
        assert_eq!(loaded_doc.body, large_body, "Large document body should match");
        assert_eq!(loaded_doc.body.len(), large_body.len(), "Large document body length should match");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_document_unicode_content() -> Result<()> {
        init_test_persistence().await?;

        // Create document with unicode content
        let mut unicode_doc = Document::default();
        unicode_doc.id = "unicode-document".to_string();
        unicode_doc.title = "Unicode Test: 🚀 café naïve résumé".to_string();
        unicode_doc.body = "Content with unicode: 中文, العربية, 🎉, математика".to_string();
        unicode_doc.url = "https://example.com/unicode".to_string();
        unicode_doc.description = Some("Testing unicode in documents: ñoño".to_string());

        // Save unicode document
        unicode_doc.save_to_one("memory").await?;

        // Load unicode document
        let mut loaded_doc = Document::new("unicode-document".to_string());
        loaded_doc = loaded_doc.load().await?;

        // Verify unicode content is preserved
        assert_eq!(loaded_doc.id, unicode_doc.id, "Unicode document ID should match");
        assert_eq!(loaded_doc.title, unicode_doc.title, "Unicode document title should match");
        assert_eq!(loaded_doc.body, unicode_doc.body, "Unicode document body should match");
        assert_eq!(loaded_doc.description, unicode_doc.description, "Unicode document description should match");

        Ok(())
    }

    #[cfg(feature = "services-redb")]
    #[tokio::test]
    #[serial]
    async fn test_document_redb_backend() -> Result<()> {
        init_test_persistence().await?;

        // Create a test document
        let mut test_doc = Document::default();
        test_doc.id = "redb-test-document".to_string();
        test_doc.title = "ReDB Backend Test".to_string();
        test_doc.body = "Testing ReDB backend persistence.".to_string();

        // Try to save to ReDB backend - this might not be configured in all environments
        match test_doc.save_to_one("redb").await {
            Ok(()) => {
                // Load from ReDB backend
                let mut loaded_doc = Document::new("redb-test-document".to_string());
                loaded_doc = loaded_doc.load().await?;

                // Verify
                assert_eq!(loaded_doc.id, test_doc.id, "ReDB document ID should match");
                assert_eq!(loaded_doc.title, test_doc.title, "ReDB document title should match");
                assert_eq!(loaded_doc.body, test_doc.body, "ReDB document body should match");
            }
            Err(e) => {
                println!("ReDB backend not available for document (expected in some environments): {:?}", e);
                // This is okay - not all environments may have ReDB configured
            }
        }

        Ok(())
    }

    #[cfg(feature = "services-sqlite")]
    #[tokio::test]
    #[serial]
    async fn test_document_sqlite_backend() -> Result<()> {
        init_test_persistence().await?;

        // Create a test document  
        let mut test_doc = Document::default();
        test_doc.id = "sqlite-test-document".to_string();
        test_doc.title = "SQLite Backend Test".to_string();
        test_doc.body = "Testing SQLite backend persistence.".to_string();

        // Try to save to SQLite backend - this might not be configured in all environments
        match test_doc.save_to_one("sqlite").await {
            Ok(()) => {
                // Load from SQLite backend
                let mut loaded_doc = Document::new("sqlite-test-document".to_string());
                loaded_doc = loaded_doc.load().await?;

                // Verify
                assert_eq!(loaded_doc.id, test_doc.id, "SQLite document ID should match");
                assert_eq!(loaded_doc.title, test_doc.title, "SQLite document title should match");
                assert_eq!(loaded_doc.body, test_doc.body, "SQLite document body should match");
            }
            Err(e) => {
                println!("SQLite backend not available for document (expected in some environments): {:?}", e);
                // This is okay - not all environments may have SQLite configured
            }
        }

        Ok(())
    }
}
