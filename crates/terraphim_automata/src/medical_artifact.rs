//! Artifact I/O for ShardedUmlsExtractor
//!
//! Custom format because daachorse shards use raw bytes (not bincode-compatible),
//! while the metadata is bincode-serialized alongside the shard data.
//!
//! Format (after zstd decompression):
//!   [header_len: u64 LE]
//!   [header_bytes: bincode(ArtifactHeader)]
//!   for each shard in header.shard_byte_lengths:
//!     [shard_bytes: raw daachorse bytes]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::umls::UmlsConcept;

/// Serializable pattern metadata
#[derive(Clone, Serialize, Deserialize)]
pub struct PatternMeta {
    pub cui: String,
    pub term: String,
}

/// Artifact header: everything that can be bincode-serialized
#[derive(Serialize, Deserialize)]
pub struct ArtifactHeader {
    /// Per-shard pattern metadata
    pub shard_metadata: Vec<Vec<PatternMeta>>,
    /// Concept lookup by CUI
    pub concept_index: HashMap<String, UmlsConcept>,
    /// Total patterns across all shards
    pub total_patterns: usize,
    /// Raw byte length of each daachorse shard (order matches shard_metadata)
    pub shard_byte_lengths: Vec<usize>,
}

/// Save a UMLS artifact: header (bincode) + shard bytes, compressed with zstd
pub fn save_umls_artifact(
    header: &ArtifactHeader,
    shard_bytes: &[Vec<u8>],
    path: &Path,
) -> anyhow::Result<()> {
    assert_eq!(
        header.shard_byte_lengths.len(),
        shard_bytes.len(),
        "shard_byte_lengths must match shard_bytes count"
    );

    // Encode header with bincode
    let header_encoded = bincode::serialize(header)?;
    let header_len = header_encoded.len() as u64;

    // Build raw payload: [header_len (8 bytes)] + [header_bytes] + [shard bytes...]
    let total_raw_size =
        8 + header_encoded.len() + shard_bytes.iter().map(|s| s.len()).sum::<usize>();
    let mut raw = Vec::with_capacity(total_raw_size);
    raw.write_all(&header_len.to_le_bytes())?;
    raw.write_all(&header_encoded)?;
    for shard in shard_bytes {
        raw.write_all(shard)?;
    }

    // Compress with zstd (level 3 is a good balance of speed vs ratio)
    let compressed = zstd::encode_all(&raw[..], 3)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &compressed)?;

    log::info!(
        "Saved UMLS artifact to {:?}: {} shards, {} total patterns, {} bytes compressed ({} bytes raw)",
        path,
        shard_bytes.len(),
        header.total_patterns,
        compressed.len(),
        raw.len()
    );
    Ok(())
}

/// Load a UMLS artifact: returns (header, shard_bytes_list)
pub fn load_umls_artifact(path: &Path) -> anyhow::Result<(ArtifactHeader, Vec<Vec<u8>>)> {
    let compressed = std::fs::read(path)?;
    let raw = zstd::decode_all(&compressed[..])?;

    if raw.len() < 8 {
        anyhow::bail!("Artifact file too small: {} bytes", raw.len());
    }

    // Read header length
    let header_len = u64::from_le_bytes(raw[..8].try_into().unwrap()) as usize;
    if raw.len() < 8 + header_len {
        anyhow::bail!(
            "Artifact truncated: expected {} header bytes, got {}",
            header_len,
            raw.len() - 8
        );
    }

    // Deserialize header
    let header: ArtifactHeader = bincode::deserialize(&raw[8..8 + header_len])?;

    // Read each shard's raw bytes
    let mut offset = 8 + header_len;
    let mut shard_bytes = Vec::with_capacity(header.shard_byte_lengths.len());
    for (i, &shard_len) in header.shard_byte_lengths.iter().enumerate() {
        if offset + shard_len > raw.len() {
            anyhow::bail!(
                "Shard {} truncated: expected {} bytes at offset {}, have {}",
                i,
                shard_len,
                offset,
                raw.len() - offset
            );
        }
        shard_bytes.push(raw[offset..offset + shard_len].to_vec());
        offset += shard_len;
    }

    log::info!(
        "Loaded UMLS artifact from {:?}: {} shards, {} total patterns, {} bytes compressed",
        path,
        shard_bytes.len(),
        header.total_patterns,
        compressed.len()
    );
    Ok((header, shard_bytes))
}

/// Check if an artifact file exists
pub fn artifact_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_header() -> ArtifactHeader {
        ArtifactHeader {
            shard_metadata: vec![
                vec![
                    PatternMeta {
                        cui: "C0000001".to_string(),
                        term: "lung cancer".to_string(),
                    },
                    PatternMeta {
                        cui: "C0000001".to_string(),
                        term: "nsclc".to_string(),
                    },
                ],
                vec![PatternMeta {
                    cui: "C0000002".to_string(),
                    term: "egfr".to_string(),
                }],
            ],
            concept_index: {
                let mut m = HashMap::new();
                m.insert(
                    "C0000001".to_string(),
                    UmlsConcept {
                        cui: "C0000001".to_string(),
                        terms: vec!["lung cancer".to_string(), "nsclc".to_string()],
                        preferred_term: "lung cancer".to_string(),
                    },
                );
                m
            },
            total_patterns: 3,
            shard_byte_lengths: vec![10, 8],
        }
    }

    #[test]
    fn test_artifact_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("umls.bin.zst");

        let header = make_test_header();
        let shard_bytes = vec![vec![1u8; 10], vec![2u8; 8]];

        save_umls_artifact(&header, &shard_bytes, &path).unwrap();
        assert!(path.exists());

        let (loaded_header, loaded_shards) = load_umls_artifact(&path).unwrap();
        assert_eq!(loaded_header.total_patterns, 3);
        assert_eq!(loaded_header.shard_metadata.len(), 2);
        assert_eq!(loaded_header.shard_byte_lengths, vec![10, 8]);
        assert_eq!(loaded_shards[0], vec![1u8; 10]);
        assert_eq!(loaded_shards[1], vec![2u8; 8]);
        assert!(loaded_header.concept_index.contains_key("C0000001"));
    }

    #[test]
    fn test_artifact_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bin.zst");
        assert!(!artifact_exists(&path));

        let header = make_test_header();
        save_umls_artifact(&header, &[vec![0u8; 10], vec![0u8; 8]], &path).unwrap();
        assert!(artifact_exists(&path));
    }
}
