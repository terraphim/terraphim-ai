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
//!
//! Integrity: `ArtifactHeader.shard_checksums` holds one SHA-256 digest per
//! shard. `load_umls_artifact` verifies every shard before returning it, so
//! callers of `deserialize_unchecked` can rely on byte provenance.

use sha2::{Digest, Sha256};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::umls::UmlsConcept;

/// Serializable pattern metadata
///
/// A single term may map to multiple CUIs (e.g., "cold" maps to both
/// C0009264 "Common Cold" and C0234192 "Cold Temperature"). All CUIs
/// are preserved to avoid silent data loss during deduplication.
#[derive(Clone, Serialize, Deserialize)]
pub struct PatternMeta {
    pub cuis: Vec<String>,
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
    /// SHA-256 digest of each shard's raw bytes; verified before
    /// `deserialize_unchecked` is called on the bytes.
    pub shard_checksums: Vec<[u8; 32]>,
}

/// Save a UMLS artifact: header (bincode) + shard bytes, compressed with zstd.
///
/// Computes SHA-256 of each shard and stores digests in the header so that
/// `load_umls_artifact` can verify integrity before any unsafe deserialization.
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
    assert_eq!(
        header.shard_checksums.len(),
        shard_bytes.len(),
        "shard_checksums must match shard_bytes count"
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

    // Validate checksum count matches shard count
    if header.shard_checksums.len() != header.shard_byte_lengths.len() {
        anyhow::bail!(
            "Artifact corrupt: {} checksums for {} shards",
            header.shard_checksums.len(),
            header.shard_byte_lengths.len()
        );
    }

    // Read each shard's raw bytes and verify SHA-256 integrity before returning.
    // This establishes the safety precondition for the caller's
    // `deserialize_unchecked`: bytes that pass verification were produced by
    // `serialize()` on the same machine and have not been tampered with.
    let mut offset = 8 + header_len;
    let mut shard_bytes = Vec::with_capacity(header.shard_byte_lengths.len());
    for (i, (&shard_len, expected_checksum)) in header
        .shard_byte_lengths
        .iter()
        .zip(header.shard_checksums.iter())
        .enumerate()
    {
        if offset + shard_len > raw.len() {
            anyhow::bail!(
                "Shard {} truncated: expected {} bytes at offset {}, have {}",
                i,
                shard_len,
                offset,
                raw.len() - offset
            );
        }
        let shard_slice = &raw[offset..offset + shard_len];
        let actual_checksum: [u8; 32] = Sha256::digest(shard_slice).into();
        if &actual_checksum != expected_checksum {
            anyhow::bail!(
                "Shard {} checksum mismatch: artifact may be corrupt or tampered with",
                i
            );
        }
        shard_bytes.push(shard_slice.to_vec());
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

    fn make_test_header(shard_bytes: &[Vec<u8>]) -> ArtifactHeader {
        ArtifactHeader {
            shard_metadata: vec![
                vec![
                    PatternMeta {
                        cuis: vec!["C0000001".to_string()],
                        term: "lung cancer".to_string(),
                    },
                    PatternMeta {
                        cuis: vec!["C0000001".to_string()],
                        term: "nsclc".to_string(),
                    },
                ],
                vec![PatternMeta {
                    cuis: vec!["C0000002".to_string()],
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
            shard_byte_lengths: shard_bytes.iter().map(|b| b.len()).collect(),
            shard_checksums: shard_bytes
                .iter()
                .map(|b| Sha256::digest(b).into())
                .collect(),
        }
    }

    #[test]
    fn test_artifact_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("umls.bin.zst");

        let shard_bytes = vec![vec![1u8; 10], vec![2u8; 8]];
        let header = make_test_header(&shard_bytes);

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
    fn test_artifact_checksum_mismatch_rejected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tampered.bin.zst");

        let shard_bytes = vec![vec![1u8; 10], vec![2u8; 8]];
        let header = make_test_header(&shard_bytes);
        save_umls_artifact(&header, &shard_bytes, &path).unwrap();

        // Load, tamper with shard bytes in the decompressed payload, recompress
        let compressed = std::fs::read(&path).unwrap();
        let mut raw = zstd::decode_all(&compressed[..]).unwrap();
        // Flip one byte in the first shard (after header)
        let header_len = u64::from_le_bytes(raw[..8].try_into().unwrap()) as usize;
        raw[8 + header_len] ^= 0xFF;
        let recompressed = zstd::encode_all(&raw[..], 3).unwrap();
        std::fs::write(&path, recompressed).unwrap();

        let result = load_umls_artifact(&path);
        assert!(result.is_err(), "tampered artifact must be rejected");
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("checksum mismatch"), "error: {}", msg);
    }

    #[test]
    fn test_artifact_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bin.zst");
        assert!(!artifact_exists(&path));

        let shard_bytes = vec![vec![0u8; 10], vec![0u8; 8]];
        let header = make_test_header(&shard_bytes);
        save_umls_artifact(&header, &shard_bytes, &path).unwrap();
        assert!(artifact_exists(&path));
    }
}
