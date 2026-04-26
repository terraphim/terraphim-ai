use crate::{DeviceStorage, Error, Result};

/// Get the hash key for a role's thesaurus source.
fn hash_key(role_name: &str) -> String {
    let normalized = role_name
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "_");
    format!("thesaurus_{}_source_hash", normalized)
}

/// Load the stored source hash for a role from the fastest operator.
pub async fn load_source_hash(role_name: &str) -> Result<Option<String>> {
    let storage = DeviceStorage::instance().await?;
    let key = hash_key(role_name);

    match storage.fastest_op.read(&key).await {
        Ok(bs) => {
            let hash = String::from_utf8(bs.to_vec())
                .map_err(|e| Error::Serde(format!("Invalid UTF-8 in hash: {}", e)))?;
            Ok(Some(hash))
        }
        Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Save the source hash for a role to all operators.
pub async fn save_source_hash(role_name: &str, hash: &str) -> Result<()> {
    let storage = DeviceStorage::instance().await?;
    let key = hash_key(role_name);

    for (op, _time) in storage.ops.values() {
        op.write(&key, hash.to_string()).await?;
    }
    Ok(())
}

/// Delete the source hash for a role.
pub async fn delete_source_hash(role_name: &str) -> Result<()> {
    let storage = DeviceStorage::instance().await?;
    let key = hash_key(role_name);

    if let Err(e) = storage.fastest_op.delete(&key).await {
        log::debug!("Failed to delete hash key '{}': {}", key, e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[serial_test::serial]
    async fn test_load_save_source_hash_roundtrip() {
        // Use memory-only storage for tests
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        // Save a hash
        save_source_hash("test_role", "abc123").await.unwrap();

        // Load it back
        let loaded = load_source_hash("test_role").await.unwrap();
        assert_eq!(loaded, Some("abc123".to_string()));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_load_source_hash_not_found() {
        // Use memory-only storage for tests
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        // Load a hash that doesn't exist
        let loaded = load_source_hash("nonexistent_role").await.unwrap();
        assert_eq!(loaded, None);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_delete_source_hash() {
        // Use memory-only storage for tests
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        // Save and then delete
        save_source_hash("delete_role", "hash_to_delete")
            .await
            .unwrap();
        delete_source_hash("delete_role").await.unwrap();

        // Should be gone
        let loaded = load_source_hash("delete_role").await.unwrap();
        assert_eq!(loaded, None);
    }
}
