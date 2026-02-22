//! Data loaders that populate MedicalRoleGraph from external data sources.
//!
//! Provides import functions for:
//! - **PrimeKG**: Precision Medicine Knowledge Graph CSV data (100K+ nodes, 4M+ edges)
//! - **SNOMED CT RF2**: SNOMED Clinical Terms in Release Format 2
//!
//! Both loaders populate a [`MedicalRoleGraph`] using its `add_medical_node` and
//! `add_medical_edge` methods rather than creating their own graph structures.
//!
//! This module is gated behind the `medical` feature flag.

use crate::medical::MedicalRoleGraph;
use ahash::AHashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;
use terraphim_types::{MedicalEdgeType, MedicalNodeType};

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// Statistics from a data import operation.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImportStats {
    /// Number of nodes imported.
    pub node_count: usize,
    /// Number of edges imported.
    pub edge_count: usize,
    /// Time taken to import/process (seconds).
    pub import_seconds: f64,
    /// Source of data (e.g. "primekg_csv", "snomed_rf2").
    pub source: String,
    /// Per-type node counts (type string -> count).
    pub node_type_counts: AHashMap<String, usize>,
    /// Per-type edge counts (type string -> count).
    pub edge_type_counts: AHashMap<String, usize>,
}

impl ImportStats {
    /// Print a formatted report to stdout.
    pub fn print_report(&self) {
        println!("\n=== Import Report ===");
        println!("Source:          {}", self.source);
        println!("Nodes imported:  {}", self.node_count);
        println!("Edges imported:  {}", self.edge_count);
        println!("Import time:     {:.2}s", self.import_seconds);
        println!("\nNode types:");
        for (node_type, count) in &self.node_type_counts {
            println!("  {:25} {:>8}", node_type, count);
        }
        println!("\nEdge types:");
        for (edge_type, count) in &self.edge_type_counts {
            println!("  {:25} {:>8}", edge_type, count);
        }
        println!("=====================\n");
    }
}

// ---------------------------------------------------------------------------
// PrimeKG loader
// ---------------------------------------------------------------------------

/// PrimeKG CSV row structure.
#[derive(Debug, serde::Deserialize)]
struct PrimeKgRow {
    #[serde(rename = "x_id")]
    x_id: u64,
    #[serde(rename = "x_type")]
    x_type: String,
    #[serde(rename = "x_name")]
    x_name: String,
    #[serde(rename = "relation")]
    relation: String,
    #[serde(rename = "y_id")]
    y_id: u64,
    #[serde(rename = "y_type")]
    y_type: String,
    #[serde(rename = "y_name")]
    y_name: String,
    #[serde(rename = "display_relation")]
    display_relation: String,
}

/// PrimeKG loader configuration.
#[derive(Debug, Clone)]
pub struct PrimeKgConfig {
    /// Cache directory for binary cache.
    pub cache_dir: std::path::PathBuf,
    /// Skip download if cache exists.
    pub prefer_cache: bool,
}

impl Default for PrimeKgConfig {
    fn default() -> Self {
        Self {
            cache_dir: std::path::PathBuf::from(".cache"),
            prefer_cache: true,
        }
    }
}

/// Import PrimeKG CSV data into a MedicalRoleGraph.
///
/// Parses PrimeKG CSV format and maps node/edge types to
/// [`MedicalNodeType`]/[`MedicalEdgeType`] variants.
pub fn import_primekg_csv(
    graph: &mut MedicalRoleGraph,
    csv_data: &[u8],
) -> anyhow::Result<ImportStats> {
    let start = Instant::now();
    let mut stats = ImportStats::default();
    let mut node_type_counts: AHashMap<String, usize> = AHashMap::new();
    let mut edge_type_counts: AHashMap<String, usize> = AHashMap::new();

    let reader = BufReader::new(csv_data);
    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);

    let mut row_count: usize = 0;

    for result in csv_reader.deserialize::<PrimeKgRow>() {
        match result {
            Ok(row) => {
                row_count += 1;

                // Add source node
                let source_type = parse_node_type(&row.x_type);
                graph.add_medical_node(row.x_id, row.x_name.clone(), source_type, None);

                // Add target node
                let target_type = parse_node_type(&row.y_type);
                graph.add_medical_node(row.y_id, row.y_name.clone(), target_type, None);

                // Add edge
                let edge_type = parse_edge_type(&row.relation, &row.display_relation);
                graph.add_medical_edge(row.x_id, row.y_id, edge_type);

                // Update statistics
                *node_type_counts.entry(row.x_type.clone()).or_insert(0) += 1;
                *node_type_counts.entry(row.y_type.clone()).or_insert(0) += 1;
                *edge_type_counts
                    .entry(row.display_relation.clone())
                    .or_insert(0) += 1;

                if row_count % 100_000 == 0 {
                    log::debug!("Processed {} rows...", row_count);
                }
            }
            Err(e) => {
                log::warn!("Failed to parse row: {}", e);
            }
        }
    }

    stats.node_count = graph.node_count();
    stats.edge_count = graph.medical_edge_count();
    stats.node_type_counts = node_type_counts;
    stats.edge_type_counts = edge_type_counts;
    stats.import_seconds = start.elapsed().as_secs_f64();
    stats.source = "primekg_csv".to_string();

    log::info!(
        "Imported {} rows, {} nodes, {} edges",
        row_count,
        stats.node_count,
        stats.edge_count
    );

    Ok(stats)
}

/// Parse a PrimeKG node type string to [`MedicalNodeType`].
pub fn parse_node_type(type_str: &str) -> MedicalNodeType {
    match type_str.to_lowercase().as_str() {
        "disease" => MedicalNodeType::Disease,
        "drug" => MedicalNodeType::Drug,
        "gene" => MedicalNodeType::Gene,
        "protein" => MedicalNodeType::Protein,
        "pathway" => MedicalNodeType::Pathway,
        "biological_process" => MedicalNodeType::BiologicalProcess,
        "molecular_function" => MedicalNodeType::MolecularFunction,
        "cellular_component" => MedicalNodeType::CellularComponent,
        "anatomy" => MedicalNodeType::Anatomy,
        "symptom" => MedicalNodeType::Symptom,
        "phenotype" => MedicalNodeType::Phenotype,
        "enzyme" => MedicalNodeType::Enzyme,
        "transporter" => MedicalNodeType::Transporter,
        "carrier" => MedicalNodeType::Carrier,
        "target" => MedicalNodeType::Target,
        "metabolite" => MedicalNodeType::Metabolite,
        "exposure" => MedicalNodeType::Exposure,
        "chemical" => MedicalNodeType::Chemical,
        "side_effect" => MedicalNodeType::SideEffect,
        "tissue" => MedicalNodeType::Tissue,
        "cell_type" => MedicalNodeType::CellType,
        "organism" => MedicalNodeType::Organism,
        "variant" => MedicalNodeType::Variant,
        "procedure" => MedicalNodeType::Procedure,
        "treatment" => MedicalNodeType::Treatment,
        _ => {
            log::debug!("Unknown node type: {}, using Concept", type_str);
            MedicalNodeType::Concept
        }
    }
}

/// Parse a PrimeKG edge relation string to [`MedicalEdgeType`].
pub fn parse_edge_type(relation: &str, display_relation: &str) -> MedicalEdgeType {
    // First try the display_relation (more descriptive)
    match display_relation.to_lowercase().as_str() {
        "treats" => MedicalEdgeType::Treats,
        "causes" => MedicalEdgeType::Causes,
        "contraindicates" => MedicalEdgeType::Contraindicates,
        "associated with" => MedicalEdgeType::AssociatedWith,
        "linked to" => MedicalEdgeType::LinkedTo,
        "interacts with" => MedicalEdgeType::InteractsWith,
        "has gene" => MedicalEdgeType::HasGene,
        "has protein" => MedicalEdgeType::HasProtein,
        "has enzyme" => MedicalEdgeType::HasEnzyme,
        "has transporter" => MedicalEdgeType::HasTransporter,
        "has carrier" => MedicalEdgeType::HasCarrier,
        "has target" => MedicalEdgeType::HasTarget,
        "has side effect" => MedicalEdgeType::HasSideEffect,
        "has indication" => MedicalEdgeType::HasIndication,
        "has contraindication" => MedicalEdgeType::HasContraindication,
        "has off-label use" => MedicalEdgeType::HasOffLabelUse,
        "has effect" => MedicalEdgeType::HasEffect,
        "has exposure" => MedicalEdgeType::HasExposure,
        "has symptom" => MedicalEdgeType::HasSymptom,
        "has anatomy" => MedicalEdgeType::HasAnatomy,
        "has pathway" => MedicalEdgeType::HasPathway,
        "has molecular function" => MedicalEdgeType::HasMolecularFunction,
        "has biological process" => MedicalEdgeType::HasBiologicalProcess,
        "has cellular component" => MedicalEdgeType::HasCellularComponent,
        "has tissue" => MedicalEdgeType::HasTissue,
        "has cell type" => MedicalEdgeType::HasCellType,
        "expresses" => MedicalEdgeType::Expresses,
        "regulates" => MedicalEdgeType::Regulates,
        "participates in" => MedicalEdgeType::ParticipatesIn,
        "upregulates" => MedicalEdgeType::Upregulates,
        "downregulates" => MedicalEdgeType::Downregulates,
        "binds to" => MedicalEdgeType::BindsTo,
        "catalyzes" => MedicalEdgeType::Catalyzes,
        "transports" => MedicalEdgeType::Transports,
        "inhibits" => MedicalEdgeType::Inhibits,
        "activates" => MedicalEdgeType::Activates,
        "modulates" => MedicalEdgeType::Modulates,
        "is a" | "isa" => MedicalEdgeType::IsA,
        "similar to" => MedicalEdgeType::SimilarTo,
        "derived from" => MedicalEdgeType::DerivedFrom,
        "member of" | "members" => MedicalEdgeType::MemberOf,
        _ => {
            // Fall back to relation field
            match relation.to_lowercase().as_str() {
                "isa" | "is_a" => MedicalEdgeType::IsA,
                "treats" => MedicalEdgeType::Treats,
                "causes" => MedicalEdgeType::Causes,
                "contraindicates" => MedicalEdgeType::Contraindicates,
                "associated_with" => MedicalEdgeType::AssociatedWith,
                "linked_to" => MedicalEdgeType::LinkedTo,
                "interacts_with" => MedicalEdgeType::InteractsWith,
                "has_phenotype" => MedicalEdgeType::HasPhenotype,
                "has_variant" => MedicalEdgeType::HasVariant,
                "part_of" => MedicalEdgeType::PartOf,
                _ => {
                    log::debug!(
                        "Unknown edge type: {} / {}, using RelatedTo",
                        relation,
                        display_relation
                    );
                    MedicalEdgeType::RelatedTo
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SNOMED CT RF2 loader
// ---------------------------------------------------------------------------

/// SNOMED CT RF2 loader configuration.
#[derive(Debug, Clone, Default)]
pub struct SnomedConfig {
    /// Path to the RF2 Concept file.
    pub concept_path: Option<String>,
    /// Path to the RF2 Description file.
    pub description_path: Option<String>,
    /// Path to the RF2 Relationship file.
    pub relationship_path: Option<String>,
}

/// SNOMED CT concept ID for IS-A relationship type.
const SNOMED_IS_A_TYPE_ID: u64 = 116680003;

/// SNOMED CT description type ID for Fully Specified Name.
const SNOMED_FSN_TYPE_ID: u64 = 900000000000003001;

/// Map a SNOMED CT semantic tag (from FSN parentheses) to a [`MedicalNodeType`].
///
/// SNOMED FSN terms end with a semantic tag in parentheses, e.g.
/// "Appendectomy (procedure)" or "Acetaminophen (substance)".
/// This function maps those tags to the closest `MedicalNodeType` variant.
fn snomed_tag_to_node_type(tag: &str) -> MedicalNodeType {
    match tag.to_lowercase().as_str() {
        "disorder" | "disease" | "finding" | "clinical finding" => MedicalNodeType::Disease,
        "procedure" | "regime/therapy" => MedicalNodeType::Procedure,
        "substance" | "product" | "medicinal product" | "clinical drug" => MedicalNodeType::Drug,
        "body structure" | "cell structure" | "morphologic abnormality" => MedicalNodeType::Anatomy,
        "organism" => MedicalNodeType::Organism,
        "observable entity" | "qualifier value" | "attribute" => MedicalNodeType::Concept,
        "situation" | "event" | "environment" | "geographic location" | "social context" => {
            MedicalNodeType::Concept
        }
        "physical object" | "specimen" | "record artifact" => MedicalNodeType::Concept,
        "physical force" | "occupation" | "person" | "ethnic group" | "religion/philosophy" => {
            MedicalNodeType::Concept
        }
        "linkage concept" | "namespace concept" | "metadata" | "foundation metadata concept" => {
            MedicalNodeType::Concept
        }
        _ => MedicalNodeType::Concept,
    }
}

/// Load SNOMED RF2 data into a MedicalRoleGraph.
///
/// Loads concepts as typed nodes (using FSN semantic tags to determine
/// the [`MedicalNodeType`]), IS-A relationships as edges, and builds
/// the IS-A hierarchy for ancestor/descendant queries.
///
/// The loader processes three RF2 files (all optional):
/// 1. **Concept file**: Identifies active concepts and their IDs
/// 2. **Description file**: Maps concept IDs to human-readable terms and semantic tags
/// 3. **Relationship file**: Loads IS-A hierarchy edges
///
/// Active concepts without a description file get a placeholder term
/// of the form "SNOMED <concept_id>" and default to `MedicalNodeType::Concept`.
pub fn load_snomed_rf2(
    graph: &mut MedicalRoleGraph,
    config: &SnomedConfig,
) -> anyhow::Result<ImportStats> {
    let start = Instant::now();
    let mut stats = ImportStats::default();

    // Step 1: Load active concept IDs
    let active_concepts: AHashMap<u64, bool> = if let Some(ref path) = config.concept_path {
        load_concept_ids(path)?
    } else {
        AHashMap::new()
    };

    // Step 2: Load descriptions (concept_id -> (preferred term, semantic tag))
    let descriptions: AHashMap<u64, (String, Option<String>)> =
        if let Some(ref path) = config.description_path {
            load_descriptions(path)?
        } else {
            AHashMap::new()
        };

    // Step 3: Add nodes for all active concepts
    for (&concept_id, &active) in &active_concepts {
        if !active {
            continue;
        }
        let (term, semantic_tag) = descriptions
            .get(&concept_id)
            .cloned()
            .unwrap_or_else(|| (format!("SNOMED {}", concept_id), None));
        let node_type = semantic_tag
            .as_deref()
            .map(snomed_tag_to_node_type)
            .unwrap_or(MedicalNodeType::Concept);
        graph.add_medical_node(concept_id, term, node_type, Some(concept_id));
        stats.node_count += 1;
        *stats
            .node_type_counts
            .entry(format!("{:?}", node_type).to_lowercase())
            .or_insert(0) += 1;
    }

    // Step 4: Load IS-A relationships
    if let Some(ref path) = config.relationship_path {
        let isa_count = load_isa_relationships(graph, path, &active_concepts)?;
        stats.edge_count = isa_count;
        stats.edge_type_counts.insert("is_a".to_string(), isa_count);
    }

    stats.import_seconds = start.elapsed().as_secs_f64();
    stats.source = "snomed_rf2".to_string();

    log::info!(
        "Loaded {} SNOMED nodes, {} IS-A edges in {:.2}s",
        stats.node_count,
        stats.edge_count,
        stats.import_seconds
    );

    Ok(stats)
}

/// Parse active concept IDs from an RF2 Concept file.
///
/// RF2 format: id\teffectiveTime\tactive\tmoduleId\tdefinitionStatusId
fn load_concept_ids<P: AsRef<Path>>(path: P) -> anyhow::Result<AHashMap<u64, bool>> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::with_capacity(1024 * 1024, file);
    let mut concepts = AHashMap::new();
    let mut line_num: usize = 0;

    for line in reader.lines() {
        line_num += 1;
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log::warn!("Error reading concept line {}: {}", line_num, e);
                continue;
            }
        };

        // Skip header
        if line_num == 1 && line.starts_with("id\t") {
            continue;
        }

        if let Some((concept_id, active)) = parse_concept_line(&line) {
            concepts.insert(concept_id, active);
        }
    }

    Ok(concepts)
}

/// Parse a single RF2 concept line.
///
/// Returns (concept_id, is_active) or None on parse failure.
fn parse_concept_line(line: &str) -> Option<(u64, bool)> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 3 {
        return None;
    }
    let id: u64 = parts[0].parse().ok()?;
    let active = parts[2] == "1";
    Some((id, active))
}

/// Load preferred terms from an RF2 Description file.
///
/// RF2 format: id\teffectiveTime\tactive\tmoduleId\tconceptId\tlanguageCode\ttypeId\tterm\tcaseSignificanceId
///
/// Prefers FSN (type_id = 900000000000003001), falls back to any active description.
/// Returns `(clean_term, Option<semantic_tag>)` for each concept.
fn load_descriptions<P: AsRef<Path>>(
    path: P,
) -> anyhow::Result<AHashMap<u64, (String, Option<String>)>> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::with_capacity(1024 * 1024, file);
    let mut descriptions: AHashMap<u64, (String, Option<String>)> = AHashMap::new();
    // Track whether we have an FSN for each concept (prefer FSN over synonym)
    let mut has_fsn: AHashMap<u64, bool> = AHashMap::new();
    let mut line_num: usize = 0;

    for line in reader.lines() {
        line_num += 1;
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line_num == 1 && line.starts_with("id\t") {
            continue;
        }

        if let Some((concept_id, term, type_id, active)) = parse_description_line(&line) {
            if !active {
                continue;
            }
            let is_fsn = type_id == SNOMED_FSN_TYPE_ID;
            let already_has_fsn = has_fsn.get(&concept_id).copied().unwrap_or(false);

            // Insert if we have no term yet, or if this is an FSN and we only had a synonym
            if !descriptions.contains_key(&concept_id) || (is_fsn && !already_has_fsn) {
                // Extract semantic tag and clean term from FSN:
                // "Term (semantic tag)" -> ("Term", Some("semantic tag"))
                let (clean_term, semantic_tag) = if is_fsn {
                    if let Some(pos) = term.rfind(" (") {
                        let tag = term[pos + 2..term.len() - 1].to_string();
                        (term[..pos].to_string(), Some(tag))
                    } else {
                        (term, None)
                    }
                } else {
                    (term, None)
                };
                descriptions.insert(concept_id, (clean_term, semantic_tag));
                if is_fsn {
                    has_fsn.insert(concept_id, true);
                }
            }
        }
    }

    Ok(descriptions)
}

/// Parse a single RF2 description line.
///
/// Returns (concept_id, term, type_id, is_active) or None on parse failure.
fn parse_description_line(line: &str) -> Option<(u64, String, u64, bool)> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 9 {
        return None;
    }
    let active = parts[2] == "1";
    let concept_id: u64 = parts[4].parse().ok()?;
    let type_id: u64 = parts[6].parse().ok()?;
    let term = parts[7].to_string();
    Some((concept_id, term, type_id, active))
}

/// Load IS-A relationships from an RF2 Relationship file.
///
/// Only loads active IS-A relationships (typeId = 116680003).
/// source IS-A destination means destination is a parent of source.
fn load_isa_relationships<P: AsRef<Path>>(
    graph: &mut MedicalRoleGraph,
    path: P,
    active_concepts: &AHashMap<u64, bool>,
) -> anyhow::Result<usize> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::with_capacity(1024 * 1024, file);
    let mut isa_count: usize = 0;
    let mut line_num: usize = 0;

    for line in reader.lines() {
        line_num += 1;
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line_num == 1 && line.starts_with("id\t") {
            continue;
        }

        if let Some((source_id, dest_id, type_id, active)) = parse_relationship_line(&line) {
            if !active || type_id != SNOMED_IS_A_TYPE_ID {
                continue;
            }

            // Only add if both concepts are known active (or if we loaded no concept file)
            let both_known = active_concepts.is_empty()
                || (active_concepts.get(&source_id).copied().unwrap_or(false)
                    && active_concepts.get(&dest_id).copied().unwrap_or(false));

            if both_known {
                // Ensure both nodes exist in the graph (use Concept for
                // placeholder nodes since we do not know their semantic type)
                if graph.get_node_type(source_id).is_none() {
                    graph.add_medical_node(
                        source_id,
                        format!("SNOMED {}", source_id),
                        MedicalNodeType::Concept,
                        Some(source_id),
                    );
                }
                if graph.get_node_type(dest_id).is_none() {
                    graph.add_medical_node(
                        dest_id,
                        format!("SNOMED {}", dest_id),
                        MedicalNodeType::Concept,
                        Some(dest_id),
                    );
                }

                // source IS-A destination
                graph.add_medical_edge(source_id, dest_id, MedicalEdgeType::IsA);
                isa_count += 1;
            }
        }
    }

    Ok(isa_count)
}

/// Parse a single RF2 relationship line.
///
/// Returns (source_id, destination_id, type_id, is_active) or None on parse failure.
fn parse_relationship_line(line: &str) -> Option<(u64, u64, u64, bool)> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 10 {
        return None;
    }
    let active = parts[2] == "1";
    let source_id: u64 = parts[4].parse().ok()?;
    let dest_id: u64 = parts[5].parse().ok()?;
    let type_id: u64 = parts[7].parse().ok()?;
    Some((source_id, dest_id, type_id, active))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_node_type_known_types() {
        assert_eq!(parse_node_type("disease"), MedicalNodeType::Disease);
        assert_eq!(parse_node_type("drug"), MedicalNodeType::Drug);
        assert_eq!(parse_node_type("gene"), MedicalNodeType::Gene);
        assert_eq!(parse_node_type("protein"), MedicalNodeType::Protein);
        assert_eq!(parse_node_type("pathway"), MedicalNodeType::Pathway);
        assert_eq!(
            parse_node_type("biological_process"),
            MedicalNodeType::BiologicalProcess
        );
        assert_eq!(
            parse_node_type("molecular_function"),
            MedicalNodeType::MolecularFunction
        );
        assert_eq!(
            parse_node_type("cellular_component"),
            MedicalNodeType::CellularComponent
        );
        assert_eq!(parse_node_type("anatomy"), MedicalNodeType::Anatomy);
        assert_eq!(parse_node_type("symptom"), MedicalNodeType::Symptom);
        assert_eq!(parse_node_type("phenotype"), MedicalNodeType::Phenotype);
        assert_eq!(parse_node_type("enzyme"), MedicalNodeType::Enzyme);
        assert_eq!(parse_node_type("transporter"), MedicalNodeType::Transporter);
        assert_eq!(parse_node_type("carrier"), MedicalNodeType::Carrier);
        assert_eq!(parse_node_type("target"), MedicalNodeType::Target);
        assert_eq!(parse_node_type("metabolite"), MedicalNodeType::Metabolite);
        assert_eq!(parse_node_type("exposure"), MedicalNodeType::Exposure);
        assert_eq!(parse_node_type("chemical"), MedicalNodeType::Chemical);
        assert_eq!(parse_node_type("side_effect"), MedicalNodeType::SideEffect);
        assert_eq!(parse_node_type("tissue"), MedicalNodeType::Tissue);
        assert_eq!(parse_node_type("cell_type"), MedicalNodeType::CellType);
        assert_eq!(parse_node_type("organism"), MedicalNodeType::Organism);
        assert_eq!(parse_node_type("variant"), MedicalNodeType::Variant);
        assert_eq!(parse_node_type("procedure"), MedicalNodeType::Procedure);
        assert_eq!(parse_node_type("treatment"), MedicalNodeType::Treatment);
    }

    #[test]
    fn test_parse_node_type_unknown_falls_back_to_concept() {
        assert_eq!(parse_node_type("unknown_type"), MedicalNodeType::Concept);
        assert_eq!(parse_node_type("foo_bar"), MedicalNodeType::Concept);
    }

    #[test]
    fn test_parse_node_type_case_insensitive() {
        assert_eq!(parse_node_type("Disease"), MedicalNodeType::Disease);
        assert_eq!(parse_node_type("DRUG"), MedicalNodeType::Drug);
        assert_eq!(parse_node_type("Gene"), MedicalNodeType::Gene);
    }

    #[test]
    fn test_parse_edge_type_display_relation() {
        assert_eq!(parse_edge_type("any", "treats"), MedicalEdgeType::Treats);
        assert_eq!(parse_edge_type("any", "causes"), MedicalEdgeType::Causes);
        assert_eq!(
            parse_edge_type("any", "contraindicates"),
            MedicalEdgeType::Contraindicates
        );
        assert_eq!(
            parse_edge_type("any", "associated with"),
            MedicalEdgeType::AssociatedWith
        );
        assert_eq!(
            parse_edge_type("any", "interacts with"),
            MedicalEdgeType::InteractsWith
        );
        assert_eq!(parse_edge_type("any", "is a"), MedicalEdgeType::IsA);
    }

    #[test]
    fn test_parse_edge_type_relation_fallback() {
        assert_eq!(
            parse_edge_type("isa", "unknown_display"),
            MedicalEdgeType::IsA
        );
        assert_eq!(
            parse_edge_type("is_a", "unknown_display"),
            MedicalEdgeType::IsA
        );
        assert_eq!(
            parse_edge_type("treats", "unknown_display"),
            MedicalEdgeType::Treats
        );
        assert_eq!(
            parse_edge_type("associated_with", "unknown_display"),
            MedicalEdgeType::AssociatedWith
        );
        assert_eq!(
            parse_edge_type("has_phenotype", "unknown_display"),
            MedicalEdgeType::HasPhenotype
        );
        assert_eq!(
            parse_edge_type("part_of", "unknown_display"),
            MedicalEdgeType::PartOf
        );
    }

    #[test]
    fn test_parse_edge_type_unknown_falls_back_to_related_to() {
        assert_eq!(
            parse_edge_type("unknown", "unknown relation"),
            MedicalEdgeType::RelatedTo
        );
    }

    #[test]
    fn test_import_primekg_csv() {
        let csv_data = r#"x_id,x_type,x_name,relation,y_id,y_type,y_name,display_relation
123456,disease,Test Disease,associated_with,789012,gene,Test Gene,associated with
789012,gene,Test Gene,interacts_with,345678,protein,Test Protein,interacts with
345678,protein,Test Protein,part_of,901234,pathway,Test Pathway,part of
567890,drug,Test Drug,treats,123456,disease,Test Disease,treats
567890,drug,Test Drug,causes,111111,symptom,Nausea,causes
111111,symptom,Nausea,isa,222222,symptom,Symptom Type,is a"#;

        let mut graph = MedicalRoleGraph::new_empty().expect("Failed to create empty graph");
        let stats =
            import_primekg_csv(&mut graph, csv_data.as_bytes()).expect("Failed to import CSV");

        // Should have 7 unique nodes (123456, 789012, 345678, 901234, 567890, 111111, 222222)
        assert_eq!(stats.node_count, 7, "Expected 7 unique nodes");

        // Should have 6 edges
        assert_eq!(stats.edge_count, 6, "Expected 6 edges");

        // Verify node types in stats
        assert!(stats.node_type_counts.contains_key("disease"));
        assert!(stats.node_type_counts.contains_key("gene"));
        assert!(stats.node_type_counts.contains_key("protein"));
        assert!(stats.node_type_counts.contains_key("pathway"));
        assert!(stats.node_type_counts.contains_key("drug"));
        assert!(stats.node_type_counts.contains_key("symptom"));

        // Verify edge types in stats
        assert!(stats.edge_type_counts.contains_key("associated with"));
        assert!(stats.edge_type_counts.contains_key("interacts with"));
        assert!(stats.edge_type_counts.contains_key("part of"));
        assert!(stats.edge_type_counts.contains_key("treats"));
        assert!(stats.edge_type_counts.contains_key("causes"));
        assert!(stats.edge_type_counts.contains_key("is a"));

        // Verify we can look up nodes
        assert_eq!(
            graph.get_node_term(123456),
            Some("Test Disease"),
            "Should find disease node term"
        );
        assert_eq!(
            graph.get_node_type(123456),
            Some(MedicalNodeType::Disease),
            "Should be Disease type"
        );

        // Verify treatments work
        let treatments = graph.get_treatments(123456);
        assert_eq!(treatments.len(), 1, "Should find one treatment");
        assert_eq!(
            graph.get_node_term(treatments[0]),
            Some("Test Drug"),
            "Treatment should be Test Drug"
        );
    }

    #[test]
    fn test_import_primekg_csv_empty() {
        let csv_data = b"x_id,x_type,x_name,relation,y_id,y_type,y_name,display_relation\n";

        let mut graph = MedicalRoleGraph::new_empty().expect("Failed to create empty graph");
        let stats = import_primekg_csv(&mut graph, csv_data).expect("Failed to import empty CSV");

        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
    }

    #[test]
    fn test_new_empty_creates_usable_graph() {
        let mut graph = MedicalRoleGraph::new_empty().expect("Failed to create empty graph");
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.medical_edge_count(), 0);

        // Should be able to add nodes and edges
        graph.add_medical_node(
            1,
            "Diabetes".to_string(),
            MedicalNodeType::Disease,
            Some(73211009),
        );
        graph.add_medical_node(2, "Metformin".to_string(), MedicalNodeType::Drug, None);
        graph.add_medical_edge(2, 1, MedicalEdgeType::Treats);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.medical_edge_count(), 1);
        assert_eq!(graph.get_node_term(1), Some("Diabetes"));
        assert_eq!(graph.snomed_to_node_id(73211009), Some(1));

        let treatments = graph.get_treatments(1);
        assert_eq!(treatments.len(), 1);
        assert!(treatments.contains(&2));
    }

    #[test]
    fn test_import_stats_default() {
        let stats = ImportStats::default();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert_eq!(stats.source, "");
    }

    // -- SNOMED RF2 parsing tests --

    #[test]
    fn test_parse_concept_line_active() {
        let line = "254637007\t20240101\t1\t900000000000207008\t900000000000073002";
        let (id, active) = parse_concept_line(line).expect("Should parse concept line");
        assert_eq!(id, 254637007);
        assert!(active);
    }

    #[test]
    fn test_parse_concept_line_inactive() {
        let line = "254637007\t20240101\t0\t900000000000207008\t900000000000073002";
        let (id, active) = parse_concept_line(line).expect("Should parse concept line");
        assert_eq!(id, 254637007);
        assert!(!active);
    }

    #[test]
    fn test_parse_concept_line_invalid() {
        assert!(parse_concept_line("too\tshort").is_none());
        assert!(parse_concept_line("").is_none());
    }

    #[test]
    fn test_parse_description_line_valid() {
        let line = "123\t20240101\t1\t900000000000207008\t254637007\ten\t900000000000003001\tTest term (disorder)\t900000000000020002";
        let (concept_id, term, type_id, active) =
            parse_description_line(line).expect("Should parse description line");
        assert_eq!(concept_id, 254637007);
        assert_eq!(term, "Test term (disorder)");
        assert_eq!(type_id, SNOMED_FSN_TYPE_ID);
        assert!(active);
    }

    #[test]
    fn test_parse_relationship_line_isa() {
        let line = "456\t20240101\t1\t900000000000207008\t254637007\t363358000\t0\t116680003\t900000000000011006\t900000000000451002";
        let (source_id, dest_id, type_id, active) =
            parse_relationship_line(line).expect("Should parse relationship line");
        assert_eq!(source_id, 254637007);
        assert_eq!(dest_id, 363358000);
        assert_eq!(type_id, SNOMED_IS_A_TYPE_ID);
        assert!(active);
    }

    #[test]
    fn test_parse_relationship_line_inactive() {
        let line = "456\t20240101\t0\t900000000000207008\t254637007\t363358000\t0\t116680003\t900000000000011006\t900000000000451002";
        let (_, _, _, active) =
            parse_relationship_line(line).expect("Should parse relationship line");
        assert!(!active);
    }

    #[test]
    fn test_snomed_tag_to_node_type_disorders() {
        assert_eq!(
            snomed_tag_to_node_type("disorder"),
            MedicalNodeType::Disease
        );
        assert_eq!(snomed_tag_to_node_type("disease"), MedicalNodeType::Disease);
        assert_eq!(snomed_tag_to_node_type("finding"), MedicalNodeType::Disease);
        assert_eq!(
            snomed_tag_to_node_type("clinical finding"),
            MedicalNodeType::Disease
        );
        // Case insensitive
        assert_eq!(
            snomed_tag_to_node_type("Disorder"),
            MedicalNodeType::Disease
        );
        assert_eq!(snomed_tag_to_node_type("FINDING"), MedicalNodeType::Disease);
    }

    #[test]
    fn test_snomed_tag_to_node_type_procedures() {
        assert_eq!(
            snomed_tag_to_node_type("procedure"),
            MedicalNodeType::Procedure
        );
        assert_eq!(
            snomed_tag_to_node_type("regime/therapy"),
            MedicalNodeType::Procedure
        );
    }

    #[test]
    fn test_snomed_tag_to_node_type_drugs() {
        assert_eq!(snomed_tag_to_node_type("substance"), MedicalNodeType::Drug);
        assert_eq!(snomed_tag_to_node_type("product"), MedicalNodeType::Drug);
        assert_eq!(
            snomed_tag_to_node_type("medicinal product"),
            MedicalNodeType::Drug
        );
        assert_eq!(
            snomed_tag_to_node_type("clinical drug"),
            MedicalNodeType::Drug
        );
    }

    #[test]
    fn test_snomed_tag_to_node_type_anatomy() {
        assert_eq!(
            snomed_tag_to_node_type("body structure"),
            MedicalNodeType::Anatomy
        );
        assert_eq!(
            snomed_tag_to_node_type("cell structure"),
            MedicalNodeType::Anatomy
        );
        assert_eq!(
            snomed_tag_to_node_type("morphologic abnormality"),
            MedicalNodeType::Anatomy
        );
    }

    #[test]
    fn test_snomed_tag_to_node_type_organism() {
        assert_eq!(
            snomed_tag_to_node_type("organism"),
            MedicalNodeType::Organism
        );
    }

    #[test]
    fn test_snomed_tag_to_node_type_concept_fallbacks() {
        assert_eq!(
            snomed_tag_to_node_type("observable entity"),
            MedicalNodeType::Concept
        );
        assert_eq!(
            snomed_tag_to_node_type("qualifier value"),
            MedicalNodeType::Concept
        );
        assert_eq!(
            snomed_tag_to_node_type("situation"),
            MedicalNodeType::Concept
        );
        assert_eq!(
            snomed_tag_to_node_type("physical object"),
            MedicalNodeType::Concept
        );
        assert_eq!(
            snomed_tag_to_node_type("linkage concept"),
            MedicalNodeType::Concept
        );
        // Unknown tags fall back to Concept
        assert_eq!(
            snomed_tag_to_node_type("totally unknown"),
            MedicalNodeType::Concept
        );
    }

    #[test]
    fn test_load_snomed_rf2_with_temp_files() {
        // Create temporary RF2 files with diverse semantic tags
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        // Concept file: 5 active concepts, 1 inactive
        let concept_path = temp_dir.path().join("sct2_Concept.txt");
        std::fs::write(
            &concept_path,
            "id\teffectiveTime\tactive\tmoduleId\tdefinitionStatusId\n\
             100001\t20240101\t1\t900000000000207008\t900000000000073002\n\
             100002\t20240101\t1\t900000000000207008\t900000000000073002\n\
             100003\t20240101\t0\t900000000000207008\t900000000000073002\n\
             100004\t20240101\t1\t900000000000207008\t900000000000073002\n\
             100005\t20240101\t1\t900000000000207008\t900000000000073002\n\
             100006\t20240101\t1\t900000000000207008\t900000000000073002\n",
        )
        .expect("Failed to write concept file");

        // Description file with diverse semantic tags in FSN entries
        let description_path = temp_dir.path().join("sct2_Description.txt");
        std::fs::write(
            &description_path,
            "id\teffectiveTime\tactive\tmoduleId\tconceptId\tlanguageCode\ttypeId\tterm\tcaseSignificanceId\n\
             1\t20240101\t1\t900000000000207008\t100001\ten\t900000000000003001\tDiabetes mellitus (disorder)\t900000000000020002\n\
             2\t20240101\t1\t900000000000207008\t100002\ten\t900000000000003001\tAppendectomy (procedure)\t900000000000020002\n\
             3\t20240101\t1\t900000000000207008\t100004\ten\t900000000000003001\tAcetaminophen (substance)\t900000000000020002\n\
             4\t20240101\t1\t900000000000207008\t100005\ten\t900000000000003001\tHeart structure (body structure)\t900000000000020002\n\
             5\t20240101\t1\t900000000000207008\t100006\ten\t900000000000003001\tBlood pressure (observable entity)\t900000000000020002\n",
        )
        .expect("Failed to write description file");

        // Relationship file: Appendectomy IS-A procedure concept (100002 IS-A 100001)
        let relationship_path = temp_dir.path().join("sct2_Relationship.txt");
        std::fs::write(
            &relationship_path,
            "id\teffectiveTime\tactive\tmoduleId\tsourceId\tdestinationId\trelationshipGroup\ttypeId\tcharacteristicTypeId\tmodifierId\n\
             1\t20240101\t1\t900000000000207008\t100002\t100001\t0\t116680003\t900000000000011006\t900000000000451002\n",
        )
        .expect("Failed to write relationship file");

        let config = SnomedConfig {
            concept_path: Some(concept_path.to_string_lossy().to_string()),
            description_path: Some(description_path.to_string_lossy().to_string()),
            relationship_path: Some(relationship_path.to_string_lossy().to_string()),
        };

        let mut graph = MedicalRoleGraph::new_empty().expect("Failed to create empty graph");
        let stats = load_snomed_rf2(&mut graph, &config).expect("Failed to load SNOMED RF2");

        // Should have 5 active concepts (100003 is inactive)
        assert_eq!(stats.node_count, 5, "Expected 5 active concept nodes");

        // Should have 1 IS-A edge (100002 IS-A 100001)
        assert_eq!(stats.edge_count, 1, "Expected 1 IS-A edge");

        // Verify node terms (FSN with semantic tag stripped)
        assert_eq!(graph.get_node_term(100001), Some("Diabetes mellitus"));
        assert_eq!(graph.get_node_term(100002), Some("Appendectomy"));
        assert_eq!(graph.get_node_term(100004), Some("Acetaminophen"));
        assert_eq!(graph.get_node_term(100005), Some("Heart structure"));
        assert_eq!(graph.get_node_term(100006), Some("Blood pressure"));

        // Verify node types derived from semantic tags
        assert_eq!(
            graph.get_node_type(100001),
            Some(MedicalNodeType::Disease),
            "disorder tag should map to Disease"
        );
        assert_eq!(
            graph.get_node_type(100002),
            Some(MedicalNodeType::Procedure),
            "procedure tag should map to Procedure"
        );
        assert_eq!(
            graph.get_node_type(100004),
            Some(MedicalNodeType::Drug),
            "substance tag should map to Drug"
        );
        assert_eq!(
            graph.get_node_type(100005),
            Some(MedicalNodeType::Anatomy),
            "body structure tag should map to Anatomy"
        );
        assert_eq!(
            graph.get_node_type(100006),
            Some(MedicalNodeType::Concept),
            "observable entity tag should map to Concept"
        );

        // Verify node_type_counts reflect actual types
        assert!(
            stats.node_type_counts.contains_key("disease"),
            "Should have disease count"
        );
        assert!(
            stats.node_type_counts.contains_key("procedure"),
            "Should have procedure count"
        );
        assert!(
            stats.node_type_counts.contains_key("drug"),
            "Should have drug count"
        );
        assert!(
            stats.node_type_counts.contains_key("anatomy"),
            "Should have anatomy count"
        );
        assert!(
            stats.node_type_counts.contains_key("concept"),
            "Should have concept count"
        );

        // Verify IS-A hierarchy
        let ancestors = graph.get_ancestors(100002);
        assert!(
            ancestors.contains(&100001),
            "Appendectomy should have Diabetes mellitus as ancestor via IS-A"
        );

        // Verify SNOMED ID mapping
        assert_eq!(graph.snomed_to_node_id(100001), Some(100001));
        assert_eq!(graph.snomed_to_node_id(100002), Some(100002));
    }

    #[test]
    fn test_load_snomed_rf2_no_files() {
        let config = SnomedConfig::default();
        let mut graph = MedicalRoleGraph::new_empty().expect("Failed to create empty graph");
        let stats = load_snomed_rf2(&mut graph, &config).expect("Should handle no files");
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
    }
}
