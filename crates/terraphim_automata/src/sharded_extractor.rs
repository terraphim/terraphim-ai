//! Sharded UMLS extractor for handling very large pattern sets
//!
//! When pattern counts exceed Aho-Corasick limits (~2M patterns),
//! this extractor shards patterns across multiple automatons.
//!
//! Uses daachorse::DoubleArrayAhoCorasick which supports native serialization
//! via `to_bytes()` / `from_bytes()` -- enabling pre-built artifact loading
//! that takes <100ms vs ~842s build time from raw TSV.

use daachorse::DoubleArrayAhoCorasick;
use std::collections::HashMap;

use crate::medical_artifact::{
    ArtifactHeader, PatternMeta, artifact_exists, load_umls_artifact, save_umls_artifact,
};
use crate::umls::{UmlsConcept, UmlsDataset};
use crate::umls_extractor::UmlsMatch;

/// Maximum patterns per automaton shard to avoid state overflow
/// Set conservatively to prevent state ID overflow with complex patterns
const DEFAULT_MAX_PATTERNS_PER_SHARD: usize = 500_000;

/// Sharded UMLS extractor that distributes patterns across multiple automatons
///
/// This allows handling datasets with millions of patterns that would
/// otherwise exceed Aho-Corasick's state identifier limits.
pub struct ShardedUmlsExtractor {
    /// Multiple automatons, each with a subset of patterns
    shards: Vec<DoubleArrayAhoCorasick<u32>>,
    /// Pattern metadata for each shard
    shard_metadata: Vec<Vec<PatternMetadata>>,
    /// Concept lookup by CUI
    concept_index: HashMap<String, UmlsConcept>,
    /// Total patterns across all shards
    total_patterns: usize,
}

/// Metadata for each pattern in a shard
///
/// Stores all CUIs that map to a given term to prevent data loss
/// when multiple concepts share the same surface form.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PatternMetadata {
    cuis: Vec<String>,
    term: String,
}

impl ShardedUmlsExtractor {
    /// Build a sharded extractor from a dataset
    ///
    /// Automatically shards patterns to stay within limits.
    pub fn from_dataset(dataset: &UmlsDataset) -> anyhow::Result<Self> {
        Self::from_dataset_with_shard_size(dataset, DEFAULT_MAX_PATTERNS_PER_SHARD)
    }

    /// Build with custom shard size
    pub fn from_dataset_with_shard_size(
        dataset: &UmlsDataset,
        max_patterns_per_shard: usize,
    ) -> anyhow::Result<Self> {
        let start = std::time::Instant::now();

        // Collect all patterns
        let mut all_patterns: Vec<(String, String)> = Vec::with_capacity(dataset.term_count);
        for (cui, concept) in &dataset.concepts {
            for term in &concept.terms {
                all_patterns.push((term.to_lowercase(), cui.clone()));
            }
        }

        // Sort patterns by term -- daachorse requires sorted, unique input
        all_patterns.sort_by(|a, b| a.0.cmp(&b.0));

        // Merge duplicates: group all CUIs that share the same term.
        // daachorse rejects duplicate patterns, but we must not silently
        // drop CUI mappings when multiple concepts share a surface form
        // (e.g., "cold" = C0009264 "Common Cold" AND C0234192 "Cold Temperature").
        let mut merged: Vec<(String, Vec<String>)> = Vec::new();
        for (term, cui) in all_patterns {
            if let Some(last) = merged.last_mut() {
                if last.0 == term {
                    if !last.1.contains(&cui) {
                        last.1.push(cui);
                    }
                    continue;
                }
            }
            merged.push((term, vec![cui]));
        }

        let multi_cui_count = merged.iter().filter(|(_, cuis)| cuis.len() > 1).count();
        if multi_cui_count > 0 {
            log::warn!(
                "{} terms map to multiple CUIs; all CUIs preserved",
                multi_cui_count
            );
        }

        let total_patterns = merged.len();
        log::info!(
            "Building sharded extractor with {} patterns (max {} per shard)...",
            total_patterns,
            max_patterns_per_shard
        );

        // Shard the patterns
        let num_shards = total_patterns.div_ceil(max_patterns_per_shard);
        let mut shards: Vec<DoubleArrayAhoCorasick<u32>> = Vec::with_capacity(num_shards);
        let mut shard_metadata: Vec<Vec<PatternMetadata>> = Vec::with_capacity(num_shards);

        for shard_idx in 0..num_shards {
            let start_idx = shard_idx * max_patterns_per_shard;
            let end_idx = ((shard_idx + 1) * max_patterns_per_shard).min(total_patterns);

            let shard_patterns: Vec<String> = merged[start_idx..end_idx]
                .iter()
                .map(|(term, _)| term.clone())
                .collect();

            let metadata: Vec<PatternMetadata> = merged[start_idx..end_idx]
                .iter()
                .map(|(term, cuis)| PatternMetadata {
                    term: term.clone(),
                    cuis: cuis.clone(),
                })
                .collect();

            log::debug!(
                "Building shard {}/{} with {} patterns...",
                shard_idx + 1,
                num_shards,
                shard_patterns.len()
            );

            let automaton = DoubleArrayAhoCorasick::<u32>::new(shard_patterns).map_err(|e| {
                anyhow::anyhow!("Failed to build daachorse shard {}: {:?}", shard_idx, e)
            })?;
            shards.push(automaton);
            shard_metadata.push(metadata);
        }

        // Build concept index
        let concept_index: HashMap<String, UmlsConcept> = dataset
            .concepts
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let build_time = start.elapsed();
        log::info!(
            "Sharded extractor built in {}ms ({} shards)",
            build_time.as_millis(),
            num_shards
        );

        Ok(Self {
            shards,
            shard_metadata,
            concept_index,
            total_patterns,
        })
    }

    /// Save this extractor to a compressed artifact file
    ///
    /// The artifact contains the serialized daachorse shard bytes plus concept
    /// metadata, compressed with zstd. Load time from artifact is <100ms vs
    /// ~842s build time from raw TSV.
    pub fn save_to_artifact(&self, path: &std::path::Path) -> anyhow::Result<()> {
        // Serialize each daachorse shard to raw bytes
        let shard_bytes: Vec<Vec<u8>> = self.shards.iter().map(|s| s.serialize()).collect();

        // Convert internal PatternMetadata to public PatternMeta
        let shard_metadata: Vec<Vec<PatternMeta>> = self
            .shard_metadata
            .iter()
            .map(|shard| {
                shard
                    .iter()
                    .map(|m| PatternMeta {
                        cuis: m.cuis.clone(),
                        term: m.term.clone(),
                    })
                    .collect()
            })
            .collect();

        let header = ArtifactHeader {
            shard_metadata,
            concept_index: self.concept_index.clone(),
            total_patterns: self.total_patterns,
            shard_byte_lengths: shard_bytes.iter().map(|b: &Vec<u8>| b.len()).collect(),
        };

        save_umls_artifact(&header, &shard_bytes, path)
    }

    /// Load a pre-built extractor from a compressed artifact file
    ///
    /// This is the fast path: loads in <100ms instead of rebuilding from TSV.
    pub fn load_from_artifact(path: &std::path::Path) -> anyhow::Result<Self> {
        let (header, shard_bytes) = load_umls_artifact(path)?;

        // Reconstruct daachorse shards from raw bytes using unsafe deserialization.
        // Safety: bytes were produced by serialize() in save_to_artifact() on the same machine.
        let shards: Vec<DoubleArrayAhoCorasick<u32>> = shard_bytes
            .iter()
            .map(|bytes| {
                // SAFETY: bytes were produced by DoubleArrayAhoCorasick::serialize()
                let (automaton, _remaining) =
                    unsafe { DoubleArrayAhoCorasick::<u32>::deserialize_unchecked(bytes) };
                automaton
            })
            .collect();

        // Convert public PatternMeta back to internal PatternMetadata
        let shard_metadata: Vec<Vec<PatternMetadata>> = header
            .shard_metadata
            .into_iter()
            .map(|shard| {
                shard
                    .into_iter()
                    .map(|m| PatternMetadata {
                        cuis: m.cuis,
                        term: m.term,
                    })
                    .collect()
            })
            .collect();

        Ok(Self {
            shards,
            shard_metadata,
            concept_index: header.concept_index,
            total_patterns: header.total_patterns,
        })
    }

    /// Check if an artifact exists at the given path
    pub fn artifact_exists(path: &std::path::Path) -> bool {
        artifact_exists(path)
    }

    /// Extract UMLS entities from text
    ///
    /// Searches across all shards and merges results.
    pub fn extract(&self, text: &str) -> Vec<UmlsMatch> {
        let text_lower = text.to_lowercase();
        let mut all_matches: Vec<UmlsMatch> = Vec::new();

        // Search each shard
        for (shard_idx, automaton) in self.shards.iter().enumerate() {
            let metadata = &self.shard_metadata[shard_idx];

            for mat in automaton.find_iter(&text_lower) {
                let pattern_idx = mat.value() as usize;
                let meta = &metadata[pattern_idx];

                let start = mat.start();
                let end = mat.end();

                // Extract original case text
                let matched_original = &text[start..end];

                // Emit one match per CUI for this pattern
                for cui in &meta.cuis {
                    let (canonical, confidence) = if let Some(concept) = self.concept_index.get(cui)
                    {
                        let conf = if concept.preferred_term.to_lowercase() == meta.term {
                            1.0
                        } else {
                            0.9
                        };
                        (concept.preferred_term.clone(), conf)
                    } else {
                        (meta.term.clone(), 0.8)
                    };

                    all_matches.push(UmlsMatch {
                        cui: cui.clone(),
                        matched_term: matched_original.to_string(),
                        canonical_term: canonical,
                        span: (start, end),
                        confidence,
                    });
                }
            }
        }

        // Remove duplicates: same span AND same CUI
        // Different CUIs at the same span are intentional (multi-CUI terms)
        all_matches.sort_by(|a, b| {
            a.span
                .0
                .cmp(&b.span.0)
                .then(a.span.1.cmp(&b.span.1))
                .then(a.cui.cmp(&b.cui))
        });
        all_matches.dedup_by(|a, b| a.span == b.span && a.cui == b.cui);

        all_matches
    }

    /// Get number of shards
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Get total pattern count
    pub fn pattern_count(&self) -> usize {
        self.total_patterns
    }

    /// Get concept count
    pub fn concept_count(&self) -> usize {
        self.concept_index.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::umls::UmlsDataset;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_dataset() -> UmlsDataset {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "non-small cell lung carcinoma\tC0000001").unwrap();
        writeln!(file, "nsclc\tC0000001").unwrap();
        writeln!(file, "lung cancer\tC0000001").unwrap();
        writeln!(file, "egfr\tC0000002").unwrap();
        writeln!(file, "epidermal growth factor receptor\tC0000002").unwrap();
        writeln!(file, "gefitinib\tC0000003").unwrap();

        UmlsDataset::from_tsv(file.path()).unwrap()
    }

    #[test]
    fn test_sharded_extractor() {
        let dataset = create_test_dataset();
        let extractor = ShardedUmlsExtractor::from_dataset_with_shard_size(
            &dataset, 2, // Force multiple shards with small dataset
        )
        .unwrap();

        assert!(extractor.shard_count() >= 2);
        assert_eq!(extractor.pattern_count(), 6);
    }

    #[test]
    fn test_extract_single_entity() {
        let dataset = create_test_dataset();
        let extractor = ShardedUmlsExtractor::from_dataset(&dataset).unwrap();

        let results = extractor.extract("Patient has lung cancer");

        assert!(!results.is_empty());
        assert_eq!(results[0].cui, "C0000001");
    }

    #[test]
    fn test_extract_multiple_entities() {
        let dataset = create_test_dataset();
        let extractor = ShardedUmlsExtractor::from_dataset(&dataset).unwrap();

        let results = extractor.extract("EGFR mutation in NSCLC patient");

        assert!(results.len() >= 2);
        let cuis: Vec<&str> = results.iter().map(|r| r.cui.as_str()).collect();
        assert!(cuis.contains(&"C0000001"));
        assert!(cuis.contains(&"C0000002"));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let dataset = create_test_dataset();
        let extractor = ShardedUmlsExtractor::from_dataset(&dataset).unwrap();

        let results = extractor.extract("Patient has LUNG CANCER");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_artifact_round_trip() {
        let dataset = create_test_dataset();
        let extractor = ShardedUmlsExtractor::from_dataset(&dataset).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let artifact_path = dir.path().join("umls_test.bin.zst");

        // Save artifact
        extractor.save_to_artifact(&artifact_path).unwrap();
        assert!(artifact_path.exists());
        assert!(ShardedUmlsExtractor::artifact_exists(&artifact_path));

        // Load artifact
        let loaded = ShardedUmlsExtractor::load_from_artifact(&artifact_path).unwrap();
        assert_eq!(loaded.pattern_count(), extractor.pattern_count());
        assert_eq!(loaded.shard_count(), extractor.shard_count());
        assert_eq!(loaded.concept_count(), extractor.concept_count());

        // Verify extraction still works after round-trip
        let results = loaded.extract("Patient has lung cancer and EGFR mutation");
        assert!(!results.is_empty());
        let cuis: Vec<&str> = results.iter().map(|r| r.cui.as_str()).collect();
        assert!(cuis.contains(&"C0000001"));
        assert!(cuis.contains(&"C0000002"));
    }

    /// Create a dataset where "cold" maps to two different CUIs
    fn create_multi_cui_dataset() -> UmlsDataset {
        let mut file = NamedTempFile::new().unwrap();
        // "cold" maps to two different concepts
        writeln!(file, "cold\tC0009264").unwrap(); // Common Cold
        writeln!(file, "common cold\tC0009264").unwrap();
        writeln!(file, "cold\tC0234192").unwrap(); // Cold Temperature
        writeln!(file, "cold temperature\tC0234192").unwrap();
        writeln!(file, "fever\tC0015967").unwrap(); // Unique term

        UmlsDataset::from_tsv(file.path()).unwrap()
    }

    #[test]
    fn test_multi_cui_term_preserved() {
        // Regression test: "cold" maps to both C0009264 (Common Cold) and
        // C0234192 (Cold Temperature). The old dedup_by silently dropped one.
        let dataset = create_multi_cui_dataset();
        let extractor = ShardedUmlsExtractor::from_dataset(&dataset).unwrap();

        // "cold" is one unique term in the automaton, so pattern count = 4
        // (cold, cold temperature, common cold, fever)
        assert_eq!(extractor.pattern_count(), 4);

        let results = extractor.extract("Patient has cold and fever");
        let cold_matches: Vec<&str> = results
            .iter()
            .filter(|m| m.matched_term.to_lowercase() == "cold")
            .map(|m| m.cui.as_str())
            .collect();

        // Both CUIs must be present for "cold"
        assert!(
            cold_matches.contains(&"C0009264"),
            "Missing CUI C0009264 (Common Cold) for term 'cold'; got: {:?}",
            cold_matches
        );
        assert!(
            cold_matches.contains(&"C0234192"),
            "Missing CUI C0234192 (Cold Temperature) for term 'cold'; got: {:?}",
            cold_matches
        );

        // Fever should still match normally
        let fever_matches: Vec<&str> = results
            .iter()
            .filter(|m| m.matched_term.to_lowercase() == "fever")
            .map(|m| m.cui.as_str())
            .collect();
        assert_eq!(fever_matches, vec!["C0015967"]);
    }

    #[test]
    fn test_multi_cui_artifact_round_trip() {
        // Verify multi-CUI mappings survive artifact serialization
        let dataset = create_multi_cui_dataset();
        let extractor = ShardedUmlsExtractor::from_dataset(&dataset).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let artifact_path = dir.path().join("multi_cui_test.bin.zst");

        extractor.save_to_artifact(&artifact_path).unwrap();
        let loaded = ShardedUmlsExtractor::load_from_artifact(&artifact_path).unwrap();

        assert_eq!(loaded.pattern_count(), extractor.pattern_count());

        let results = loaded.extract("Patient has cold");
        let cold_cuis: Vec<&str> = results
            .iter()
            .filter(|m| m.matched_term.to_lowercase() == "cold")
            .map(|m| m.cui.as_str())
            .collect();

        assert!(
            cold_cuis.contains(&"C0009264"),
            "After round-trip: missing C0009264; got: {:?}",
            cold_cuis
        );
        assert!(
            cold_cuis.contains(&"C0234192"),
            "After round-trip: missing C0234192; got: {:?}",
            cold_cuis
        );
    }
}
