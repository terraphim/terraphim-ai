//! HGNC Gene Normalization
//!
//! Specialized gene normalization for HGNC (HUGO Gene Nomenclature Committee) gene symbols.
//! Handles exact matches, aliases, and fuzzy matching for gene names.

use crate::GroundingMetadata;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HGNC gene entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HgncGene {
    /// HGNC gene symbol (e.g., EGFR, TP53)
    pub symbol: String,
    /// HGNC ID (e.g., HGNC:3236)
    pub hgnc_id: String,
    /// Previous/alias symbols
    pub aliases: Vec<String>,
    /// Gene name/description
    pub name: String,
    /// Gene family
    pub gene_family: Option<String>,
}

/// HGNC Gene Normalizer
pub struct HgncNormalizer {
    /// Symbol to gene entry mapping (lowercase for case-insensitive lookup)
    symbol_map: HashMap<String, HgncGene>,
    /// Alias to symbol mapping
    alias_map: HashMap<String, String>,
}

impl HgncNormalizer {
    /// Create a new HGNC normalizer with default oncology genes
    pub fn new() -> Self {
        let mut normalizer = Self {
            symbol_map: HashMap::new(),
            alias_map: HashMap::new(),
        };

        // Add common oncology genes
        normalizer.add_gene(HgncGene {
            symbol: "EGFR".to_string(),
            hgnc_id: "HGNC:3236".to_string(),
            aliases: vec!["ERBB".to_string(), "ERBB1".to_string(), "HER1".to_string()],
            name: "epidermal growth factor receptor".to_string(),
            gene_family: Some("EGFR".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "TP53".to_string(),
            hgnc_id: "HGNC:11998".to_string(),
            aliases: vec!["P53".to_string(), "LFS1".to_string()],
            name: "tumor protein p53".to_string(),
            gene_family: Some("TP53".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "KRAS".to_string(),
            hgnc_id: "HGNC:6407".to_string(),
            aliases: vec!["KI-RAS".to_string(), "KI-RAS2".to_string()],
            name: "KRAS proto-oncogene, GTPase".to_string(),
            gene_family: Some("RAS".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "NRAS".to_string(),
            hgnc_id: "HGNC:7959".to_string(),
            aliases: vec!["N-RAS".to_string()],
            name: "NRAS proto-oncogene, GTPase".to_string(),
            gene_family: Some("RAS".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "BRAF".to_string(),
            hgnc_id: "HGNC:1097".to_string(),
            aliases: vec!["B-RAF".to_string(), "RAF1".to_string()],
            name: "B-Raf proto-oncogene, serine/threonine kinase".to_string(),
            gene_family: Some("RAF".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "ALK".to_string(),
            hgnc_id: "HGNC:427".to_string(),
            aliases: vec!["CD246".to_string()],
            name: "anaplastic lymphoma kinase".to_string(),
            gene_family: Some("ALK".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "EML4".to_string(),
            hgnc_id: "HGNC:13169".to_string(),
            aliases: vec!["ELK4".to_string()],
            name: "EMAP like 4".to_string(),
            gene_family: None,
        });

        normalizer.add_gene(HgncGene {
            symbol: "ALK".to_string(),
            hgnc_id: "HGNC:427".to_string(),
            aliases: vec!["CD246".to_string()],
            name: "anaplastic lymphoma kinase".to_string(),
            gene_family: Some("ALK".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "ROS1".to_string(),
            hgnc_id: "HGNC:10261".to_string(),
            aliases: vec!["c-ROS".to_string()],
            name: "ROS proto-oncogene 1, receptor tyrosine kinase".to_string(),
            gene_family: Some("ROS1".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "MET".to_string(),
            hgnc_id: "HGNC:7029".to_string(),
            aliases: vec!["c-MET".to_string(), "HGFR".to_string()],
            name: "MET proto-oncogene, receptor tyrosine kinase".to_string(),
            gene_family: Some("MET".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "ERBB2".to_string(),
            hgnc_id: "HGNC:3430".to_string(),
            aliases: vec!["HER2".to_string(), "NEU".to_string()],
            name: "erb-b2 receptor tyrosine kinase 2".to_string(),
            gene_family: Some("ERBB".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "ERBB3".to_string(),
            hgnc_id: "HGNC:3431".to_string(),
            aliases: vec!["HER3".to_string()],
            name: "erb-b2 receptor tyrosine kinase 3".to_string(),
            gene_family: Some("ERBB".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "ERBB4".to_string(),
            hgnc_id: "HGNC:3432".to_string(),
            aliases: vec!["HER4".to_string()],
            name: "erb-b2 receptor tyrosine kinase 4".to_string(),
            gene_family: Some("ERBB".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "PIK3CA".to_string(),
            hgnc_id: "HGNC:8979".to_string(),
            aliases: vec!["PI3K".to_string()],
            name: "phosphatidylinositol-4,5-bisphosphate 3-kinase catalytic subunit alpha"
                .to_string(),
            gene_family: Some("PI3K".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "AKT1".to_string(),
            hgnc_id: "HGNC:391".to_string(),
            aliases: vec!["AKT".to_string()],
            name: "AKT serine/threonine kinase 1".to_string(),
            gene_family: Some("AKT".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "BRCA1".to_string(),
            hgnc_id: "HGNC:1100".to_string(),
            aliases: vec!["BRCAI".to_string(), "PS1".to_string()],
            name: "BRCA1 DNA repair associated".to_string(),
            gene_family: Some("BRCA".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "BRCA2".to_string(),
            hgnc_id: "HGNC:1101".to_string(),
            aliases: vec!["BRCAII".to_string(), "FANCD1".to_string()],
            name: "BRCA2 DNA repair associated".to_string(),
            gene_family: Some("BRCA".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "MLH1".to_string(),
            hgnc_id: "HGNC:7127".to_string(),
            aliases: vec!["hMLH1".to_string()],
            name: "mutL homolog 1".to_string(),
            gene_family: Some("MutL".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "MSH2".to_string(),
            hgnc_id: "HGNC:7325".to_string(),
            aliases: vec!["hMSH2".to_string()],
            name: "mutS homolog 2".to_string(),
            gene_family: Some("MutS".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "CDK4".to_string(),
            hgnc_id: "HGNC:1771".to_string(),
            aliases: vec!["Cdk4".to_string()],
            name: "cyclin dependent kinase 4".to_string(),
            gene_family: Some("CDK".to_string()),
        });

        normalizer.add_gene(HgncGene {
            symbol: "CDK6".to_string(),
            hgnc_id: "HGNC:1773".to_string(),
            aliases: vec!["Cdk6".to_string()],
            name: "cyclin dependent kinase 6".to_string(),
            gene_family: Some("CDK".to_string()),
        });

        normalizer
    }

    /// Add a gene to the normalizer
    fn add_gene(&mut self, gene: HgncGene) {
        let symbol_lower = gene.symbol.to_lowercase();
        self.symbol_map.insert(symbol_lower.clone(), gene.clone());

        // Add aliases
        for alias in &gene.aliases {
            self.alias_map
                .insert(alias.to_lowercase(), symbol_lower.clone());
        }
    }

    /// Normalize a gene symbol (exact match)
    pub fn normalize_exact(&self, symbol: &str) -> Option<GroundingMetadata> {
        let symbol_lower = symbol.to_lowercase();

        // Check direct symbol match
        if let Some(gene) = self.symbol_map.get(&symbol_lower) {
            return Some(GroundingMetadata::new(
                format!(
                    "https://www.genenames.org/data/hgnc_data.php?appid={}",
                    gene.hgnc_id.trim_start_matches("HGNC:")
                ),
                gene.symbol.clone(),
                "HGNC".to_string(),
                1.0,
                crate::NormalizationMethod::Exact,
            ));
        }

        // Check alias match
        if let Some(symbol_key) = self.alias_map.get(&symbol_lower) {
            if let Some(gene) = self.symbol_map.get(symbol_key) {
                return Some(GroundingMetadata::new(
                    format!(
                        "https://www.genenames.org/data/hgnc_data.php?appid={}",
                        gene.hgnc_id.trim_start_matches("HGNC:")
                    ),
                    gene.symbol.clone(),
                    "HGNC".to_string(),
                    0.95,
                    crate::NormalizationMethod::Exact,
                ));
            }
        }

        None
    }

    /// Normalize using fuzzy matching (for variants/misspellings)
    pub fn normalize_fuzzy(&self, symbol: &str) -> Option<(HgncGene, f32)> {
        let symbol_lower = symbol.to_lowercase();

        // Try to find closest match using simple substring/prefix matching
        let mut best_match: Option<(HgncGene, f32)> = None;

        for gene in self.symbol_map.values() {
            // Check if symbol starts with gene (e.g., "EGFRvIII" matches "EGFR")
            if symbol_lower.starts_with(&gene.symbol.to_lowercase()) {
                let score = gene.symbol.len() as f32 / symbol.len() as f32;
                match best_match {
                    None => best_match = Some((gene.clone(), score)),
                    Some((_, existing_score)) if score > existing_score => {
                        best_match = Some((gene.clone(), score));
                    }
                    _ => {}
                }
            }

            // Check if gene symbol starts with input (e.g., "EGFR" matches "EGFRvIII")
            if gene.symbol.to_lowercase().starts_with(&symbol_lower) {
                let score = symbol.len() as f32 / gene.symbol.len() as f32;
                match best_match {
                    None => best_match = Some((gene.clone(), score)),
                    Some((_, existing_score)) if score > existing_score => {
                        best_match = Some((gene.clone(), score));
                    }
                    _ => {}
                }
            }

            // Check aliases
            for alias in &gene.aliases {
                if alias.to_lowercase() == symbol_lower {
                    let score = 0.9;
                    match best_match {
                        None => best_match = Some((gene.clone(), score)),
                        Some((_, existing_score)) if score > existing_score => {
                            best_match = Some((gene.clone(), score));
                        }
                        _ => {}
                    }
                }
            }
        }

        best_match
    }

    /// Normalize a gene with both exact and fuzzy matching
    pub fn normalize(&self, symbol: &str) -> Option<GroundingMetadata> {
        // Try exact match first
        if let Some(exact) = self.normalize_exact(symbol) {
            return Some(exact);
        }

        // Fall back to fuzzy matching
        if let Some((gene, score)) = self.normalize_fuzzy(symbol) {
            return Some(GroundingMetadata::new(
                format!(
                    "https://www.genenames.org/data/hgnc_data.php?appid={}",
                    gene.hgnc_id.trim_start_matches("HGNC:")
                ),
                gene.symbol.clone(),
                "HGNC".to_string(),
                score,
                crate::NormalizationMethod::Fuzzy,
            ));
        }

        None
    }

    /// Get all known genes
    pub fn all_genes(&self) -> Vec<&HgncGene> {
        self.symbol_map.values().collect()
    }
}

impl Default for HgncNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_egfr() {
        let normalizer = HgncNormalizer::new();

        let result = normalizer.normalize_exact("EGFR");
        assert!(result.is_some());

        let meta = result.unwrap();
        assert_eq!(meta.normalized_label, Some("EGFR".to_string()));
        assert_eq!(meta.normalized_prov, Some("HGNC".to_string()));
    }

    #[test]
    fn test_alias_match_erbb1() {
        let normalizer = HgncNormalizer::new();

        // ERBB1 is an alias for EGFR
        let result = normalizer.normalize_exact("ERBB1");
        assert!(result.is_some());

        let meta = result.unwrap();
        assert_eq!(meta.normalized_label, Some("EGFR".to_string()));
    }

    #[test]
    fn test_fuzzy_match_egfrv3() {
        let normalizer = HgncNormalizer::new();

        // EGFRvIII is a variant - should fuzzy match to EGFR
        let result = normalizer.normalize("EGFRvIII");
        assert!(result.is_some());

        let meta = result.unwrap();
        assert_eq!(meta.normalized_label, Some("EGFR".to_string()));
    }

    #[test]
    fn test_unknown_gene() {
        let normalizer = HgncNormalizer::new();

        let result = normalizer.normalize("XYZ123");
        assert!(result.is_none());
    }

    #[test]
    fn test_tp53_exact() {
        let normalizer = HgncNormalizer::new();

        let result = normalizer.normalize_exact("TP53");
        assert!(result.is_some());

        let meta = result.unwrap();
        assert_eq!(meta.normalized_label, Some("TP53".to_string()));
        assert!(meta.normalized_score.unwrap() > 0.9);
    }

    #[test]
    fn test_gene_family() {
        let normalizer = HgncNormalizer::new();

        let genes = normalizer.all_genes();
        let egfr = genes.iter().find(|g| g.symbol == "EGFR").unwrap();
        assert_eq!(egfr.gene_family, Some("EGFR".to_string()));
    }
}
