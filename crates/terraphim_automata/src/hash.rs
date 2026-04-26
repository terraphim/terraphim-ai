use std::hash::Hasher;
use std::path::Path;
use twox_hash::XxHash64;

/// Compute a combined content hash of all `.md` files in a directory.
///
/// Walks the directory recursively, reads each `.md` file, and combines
/// their individual hashes into a single directory hash. Files are processed
/// in sorted order to ensure deterministic results.
///
/// Returns a hex-encoded string representation of the combined hash.
pub fn hash_kg_dir(path: &Path) -> std::io::Result<String> {
    let mut hasher = XxHash64::default();
    let mut entries: Vec<_> = walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();
    entries.sort();

    for path in &entries {
        let content = std::fs::read(path)?;
        hasher.write(&content);
    }

    Ok(format!("{:016x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_hash_kg_dir_deterministic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let md_path = temp_dir.path().join("test.md");
        let mut file = std::fs::File::create(&md_path).unwrap();
        writeln!(file, "synonyms:: foo, bar").unwrap();
        drop(file);

        let hash1 = hash_kg_dir(temp_dir.path()).unwrap();
        let hash2 = hash_kg_dir(temp_dir.path()).unwrap();
        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_hash_kg_dir_changes_on_edit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let md_path = temp_dir.path().join("test.md");
        let mut file = std::fs::File::create(&md_path).unwrap();
        writeln!(file, "synonyms:: foo, bar").unwrap();
        drop(file);

        let hash1 = hash_kg_dir(temp_dir.path()).unwrap();

        let mut file = std::fs::File::create(&md_path).unwrap();
        writeln!(file, "synonyms:: foo, bar, baz").unwrap();
        drop(file);

        let hash2 = hash_kg_dir(temp_dir.path()).unwrap();
        assert_ne!(hash1, hash2, "Hash should change when file is edited");
    }

    #[test]
    fn test_hash_kg_dir_empty_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let hash1 = hash_kg_dir(temp_dir.path()).unwrap();
        let hash2 = hash_kg_dir(temp_dir.path()).unwrap();
        assert_eq!(hash1, hash2, "Empty directory hash should be deterministic");
    }

    #[test]
    fn test_hash_kg_dir_ignores_non_md() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a .txt file
        let txt_path = temp_dir.path().join("test.txt");
        let mut file = std::fs::File::create(&txt_path).unwrap();
        writeln!(file, "some content").unwrap();
        drop(file);

        let hash1 = hash_kg_dir(temp_dir.path()).unwrap();

        // Modify the .txt file
        let mut file = std::fs::File::create(&txt_path).unwrap();
        writeln!(file, "different content").unwrap();
        drop(file);

        let hash2 = hash_kg_dir(temp_dir.path()).unwrap();
        assert_eq!(hash1, hash2, "Non-.md files should be ignored");
    }
}
