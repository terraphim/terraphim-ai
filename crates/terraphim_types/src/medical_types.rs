//! Medical type definitions for clinical and biomedical knowledge graphs.
//!
//! This module provides strongly-typed node and edge types for medical/biomedical
//! knowledge graphs, covering entities and relationships from clinical workflows,
//! molecular biology (PrimeKG), and drug-disease interactions.
//!
//! # Features
//!
//! This module is gated behind the `medical` feature flag.
//!
//! # Examples
//!
//! ```
//! use terraphim_types::medical_types::{MedicalNodeType, MedicalEdgeType, MedicalNodeMetadata};
//!
//! let node_type = MedicalNodeType::Disease;
//! let edge_type = MedicalEdgeType::Treats;
//!
//! assert_eq!(node_type.to_string(), "disease");
//! assert_eq!(edge_type.to_string(), "treats");
//!
//! let metadata = MedicalNodeMetadata::default();
//! assert!(metadata.metadata.is_empty());
//! ```

use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

/// Node types for medical/biomedical knowledge graphs.
///
/// Covers clinical entities (diseases, drugs, symptoms), molecular biology
/// entities (genes, proteins, pathways), and ontology concepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MedicalNodeType {
    /// Generic concept node (default)
    #[default]
    Concept,
    /// A disease or medical condition
    Disease,
    /// A pharmaceutical drug
    Drug,
    /// A gene
    Gene,
    /// A genetic variant
    Variant,
    /// A medical procedure
    Procedure,
    /// A treatment regimen
    Treatment,
    /// A clinical symptom
    Symptom,
    /// A protein
    Protein,
    /// A biological process (Gene Ontology)
    BiologicalProcess,
    /// A molecular function (Gene Ontology)
    MolecularFunction,
    /// A cellular component (Gene Ontology)
    CellularComponent,
    /// A biological pathway
    Pathway,
    /// An anatomical structure
    Anatomy,
    /// A phenotype or observable trait
    Phenotype,
    /// An enzyme
    Enzyme,
    /// A drug transporter
    Transporter,
    /// A drug carrier
    Carrier,
    /// A drug target
    Target,
    /// A metabolite
    Metabolite,
    /// An environmental or chemical exposure
    Exposure,
    /// A chemical compound
    Chemical,
    /// A side effect
    SideEffect,
    /// A tissue type
    Tissue,
    /// A cell type
    CellType,
    /// An organism
    Organism,
    /// A term from a formal ontology
    OntologyTerm,
}

impl Display for MedicalNodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Concept => "concept",
            Self::Disease => "disease",
            Self::Drug => "drug",
            Self::Gene => "gene",
            Self::Variant => "variant",
            Self::Procedure => "procedure",
            Self::Treatment => "treatment",
            Self::Symptom => "symptom",
            Self::Protein => "protein",
            Self::BiologicalProcess => "biological_process",
            Self::MolecularFunction => "molecular_function",
            Self::CellularComponent => "cellular_component",
            Self::Pathway => "pathway",
            Self::Anatomy => "anatomy",
            Self::Phenotype => "phenotype",
            Self::Enzyme => "enzyme",
            Self::Transporter => "transporter",
            Self::Carrier => "carrier",
            Self::Target => "target",
            Self::Metabolite => "metabolite",
            Self::Exposure => "exposure",
            Self::Chemical => "chemical",
            Self::SideEffect => "side_effect",
            Self::Tissue => "tissue",
            Self::CellType => "cell_type",
            Self::Organism => "organism",
            Self::OntologyTerm => "ontology_term",
        };
        write!(f, "{}", s)
    }
}

/// Edge types for medical/biomedical knowledge graphs.
///
/// Organized by category:
/// - Core relationships (IsA, Treats, Causes, etc.)
/// - PrimeKG molecular relationships (AssociatedWith, InteractsWith, etc.)
/// - Clinical workflow temporal relationships (OccursBefore, OccursAfter, etc.)
/// - Severity and probability relationships (Exacerbates, Alleviates, etc.)
/// - Diagnostic relationships (Diagnoses, Confirms, etc.)
/// - Clinical course relationships (ProgressesTo, ResolvesTo, etc.)
/// - Treatment detail relationships (AdministeredVia, CombinedWith, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MedicalEdgeType {
    // -- Core relationships --
    /// Subsumption / taxonomy relationship
    IsA,
    /// Drug treats disease
    Treats,
    /// Entity causes condition
    Causes,
    /// Drug is contraindicated for condition
    Contraindicates,
    /// Drug is metabolized by enzyme
    MetabolizedBy,
    /// Entity has phenotype
    HasPhenotype,
    /// Entity has genetic variant
    HasVariant,
    /// Entity is part of another entity
    PartOf,
    /// Generic relationship (default)
    #[default]
    RelatedTo,

    // -- PrimeKG molecular relationships --
    /// General association
    AssociatedWith,
    /// General linkage
    LinkedTo,
    /// Molecular interaction
    InteractsWith,
    /// Entity has associated gene
    HasGene,
    /// Entity has associated protein
    HasProtein,
    /// Drug has enzyme interaction
    HasEnzyme,
    /// Drug has transporter interaction
    HasTransporter,
    /// Drug has carrier interaction
    HasCarrier,
    /// Drug has target interaction
    HasTarget,
    /// Drug has side effect
    HasSideEffect,
    /// Drug has indication
    HasIndication,
    /// Drug has contraindication
    HasContraindication,
    /// Drug has off-label use
    HasOffLabelUse,
    /// Entity has effect
    HasEffect,
    /// Entity has exposure
    HasExposure,
    /// Entity has symptom
    HasSymptom,
    /// Entity has anatomy association
    HasAnatomy,
    /// Entity has pathway association
    HasPathway,
    /// Entity has molecular function
    HasMolecularFunction,
    /// Entity has biological process
    HasBiologicalProcess,
    /// Entity has cellular component
    HasCellularComponent,
    /// Entity has tissue association
    HasTissue,
    /// Entity has cell type association
    HasCellType,
    /// Gene/protein is expressed in tissue
    Expresses,
    /// Entity regulates another
    Regulates,
    /// Entity participates in process
    ParticipatesIn,
    /// Entity upregulates another
    Upregulates,
    /// Entity downregulates another
    Downregulates,
    /// Molecule binds to target
    BindsTo,
    /// Enzyme catalyzes reaction
    Catalyzes,
    /// Transporter moves substrate
    Transports,
    /// Entity inhibits another
    Inhibits,
    /// Entity activates another
    Activates,
    /// Entity modulates another
    Modulates,
    /// Entities are similar
    SimilarTo,
    /// Entity is derived from another
    DerivedFrom,
    /// Entity is a member of a group
    MemberOf,

    // -- Clinical workflow: Temporal --
    /// Event occurs before another
    OccursBefore,
    /// Event occurs after another
    OccursAfter,
    /// Events are concurrent
    ConcurrentWith,
    /// Event follows another
    Follows,

    // -- Severity / probability --
    /// Entity exacerbates a condition
    Exacerbates,
    /// Entity alleviates a condition
    Alleviates,
    /// Entity predisposes to a condition
    PredisposesTo,
    /// Entity protects against a condition
    ProtectsAgainst,

    // -- Diagnostic --
    /// Test diagnoses condition
    Diagnoses,
    /// Test confirms condition
    Confirms,
    /// Test rules out condition
    RulesOut,
    /// Finding suggests condition
    Suggests,
    /// Finding indicates condition
    Indicates,

    // -- Clinical course --
    /// Condition progresses to another
    ProgressesTo,
    /// Condition resolves to state
    ResolvesTo,
    /// Condition recurs as another
    RecursAs,
    /// Condition complicates another
    Complicates,

    // -- Treatment detail --
    /// Drug administered via route
    AdministeredVia,
    /// Drug combined with another
    CombinedWith,
    /// Drug is an alternative to another
    AlternativesTo,
    /// Drug synergizes with another
    SynergizesWith,
    /// Drug antagonizes another
    Antagonizes,
}

impl Display for MedicalEdgeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            // Core relationships
            Self::IsA => "is_a",
            Self::Treats => "treats",
            Self::Causes => "causes",
            Self::Contraindicates => "contraindicates",
            Self::MetabolizedBy => "metabolized_by",
            Self::HasPhenotype => "has_phenotype",
            Self::HasVariant => "has_variant",
            Self::PartOf => "part_of",
            Self::RelatedTo => "related_to",
            // PrimeKG molecular
            Self::AssociatedWith => "associated_with",
            Self::LinkedTo => "linked_to",
            Self::InteractsWith => "interacts_with",
            Self::HasGene => "has_gene",
            Self::HasProtein => "has_protein",
            Self::HasEnzyme => "has_enzyme",
            Self::HasTransporter => "has_transporter",
            Self::HasCarrier => "has_carrier",
            Self::HasTarget => "has_target",
            Self::HasSideEffect => "has_side_effect",
            Self::HasIndication => "has_indication",
            Self::HasContraindication => "has_contraindication",
            Self::HasOffLabelUse => "has_off_label_use",
            Self::HasEffect => "has_effect",
            Self::HasExposure => "has_exposure",
            Self::HasSymptom => "has_symptom",
            Self::HasAnatomy => "has_anatomy",
            Self::HasPathway => "has_pathway",
            Self::HasMolecularFunction => "has_molecular_function",
            Self::HasBiologicalProcess => "has_biological_process",
            Self::HasCellularComponent => "has_cellular_component",
            Self::HasTissue => "has_tissue",
            Self::HasCellType => "has_cell_type",
            Self::Expresses => "expresses",
            Self::Regulates => "regulates",
            Self::ParticipatesIn => "participates_in",
            Self::Upregulates => "upregulates",
            Self::Downregulates => "downregulates",
            Self::BindsTo => "binds_to",
            Self::Catalyzes => "catalyzes",
            Self::Transports => "transports",
            Self::Inhibits => "inhibits",
            Self::Activates => "activates",
            Self::Modulates => "modulates",
            Self::SimilarTo => "similar_to",
            Self::DerivedFrom => "derived_from",
            Self::MemberOf => "member_of",
            // Clinical workflow: Temporal
            Self::OccursBefore => "occurs_before",
            Self::OccursAfter => "occurs_after",
            Self::ConcurrentWith => "concurrent_with",
            Self::Follows => "follows",
            // Severity / probability
            Self::Exacerbates => "exacerbates",
            Self::Alleviates => "alleviates",
            Self::PredisposesTo => "predisposes_to",
            Self::ProtectsAgainst => "protects_against",
            // Diagnostic
            Self::Diagnoses => "diagnoses",
            Self::Confirms => "confirms",
            Self::RulesOut => "rules_out",
            Self::Suggests => "suggests",
            Self::Indicates => "indicates",
            // Clinical course
            Self::ProgressesTo => "progresses_to",
            Self::ResolvesTo => "resolves_to",
            Self::RecursAs => "recurs_as",
            Self::Complicates => "complicates",
            // Treatment detail
            Self::AdministeredVia => "administered_via",
            Self::CombinedWith => "combined_with",
            Self::AlternativesTo => "alternatives_to",
            Self::SynergizesWith => "synergizes_with",
            Self::Antagonizes => "antagonizes",
        };
        write!(f, "{}", s)
    }
}

/// Metadata container for medical nodes.
///
/// Stores arbitrary key-value metadata associated with a medical knowledge
/// graph node, such as source database identifiers, confidence scores,
/// or provenance information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MedicalNodeMetadata {
    /// Arbitrary key-value metadata
    pub metadata: AHashMap<String, String>,
}

impl MedicalNodeMetadata {
    /// Create a new empty metadata container
    pub fn new() -> Self {
        Self {
            metadata: AHashMap::new(),
        }
    }

    /// Insert a key-value pair into the metadata
    pub fn insert(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Check if the metadata is empty
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    /// Get the number of metadata entries
    pub fn len(&self) -> usize {
        self.metadata.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_medical_node_type_default() {
        let node_type = MedicalNodeType::default();
        assert_eq!(node_type, MedicalNodeType::Concept);
    }

    #[test]
    fn test_medical_edge_type_default() {
        let edge_type = MedicalEdgeType::default();
        assert_eq!(edge_type, MedicalEdgeType::RelatedTo);
    }

    #[test]
    fn test_medical_node_type_display() {
        assert_eq!(MedicalNodeType::Concept.to_string(), "concept");
        assert_eq!(MedicalNodeType::Disease.to_string(), "disease");
        assert_eq!(MedicalNodeType::Drug.to_string(), "drug");
        assert_eq!(MedicalNodeType::Gene.to_string(), "gene");
        assert_eq!(MedicalNodeType::Variant.to_string(), "variant");
        assert_eq!(MedicalNodeType::Procedure.to_string(), "procedure");
        assert_eq!(MedicalNodeType::Treatment.to_string(), "treatment");
        assert_eq!(MedicalNodeType::Symptom.to_string(), "symptom");
        assert_eq!(MedicalNodeType::Protein.to_string(), "protein");
        assert_eq!(
            MedicalNodeType::BiologicalProcess.to_string(),
            "biological_process"
        );
        assert_eq!(
            MedicalNodeType::MolecularFunction.to_string(),
            "molecular_function"
        );
        assert_eq!(
            MedicalNodeType::CellularComponent.to_string(),
            "cellular_component"
        );
        assert_eq!(MedicalNodeType::Pathway.to_string(), "pathway");
        assert_eq!(MedicalNodeType::Anatomy.to_string(), "anatomy");
        assert_eq!(MedicalNodeType::Phenotype.to_string(), "phenotype");
        assert_eq!(MedicalNodeType::Enzyme.to_string(), "enzyme");
        assert_eq!(MedicalNodeType::Transporter.to_string(), "transporter");
        assert_eq!(MedicalNodeType::Carrier.to_string(), "carrier");
        assert_eq!(MedicalNodeType::Target.to_string(), "target");
        assert_eq!(MedicalNodeType::Metabolite.to_string(), "metabolite");
        assert_eq!(MedicalNodeType::Exposure.to_string(), "exposure");
        assert_eq!(MedicalNodeType::Chemical.to_string(), "chemical");
        assert_eq!(MedicalNodeType::SideEffect.to_string(), "side_effect");
        assert_eq!(MedicalNodeType::Tissue.to_string(), "tissue");
        assert_eq!(MedicalNodeType::CellType.to_string(), "cell_type");
        assert_eq!(MedicalNodeType::Organism.to_string(), "organism");
        assert_eq!(MedicalNodeType::OntologyTerm.to_string(), "ontology_term");
    }

    #[test]
    fn test_medical_edge_type_display() {
        // Core relationships
        assert_eq!(MedicalEdgeType::IsA.to_string(), "is_a");
        assert_eq!(MedicalEdgeType::Treats.to_string(), "treats");
        assert_eq!(MedicalEdgeType::Causes.to_string(), "causes");
        assert_eq!(
            MedicalEdgeType::Contraindicates.to_string(),
            "contraindicates"
        );
        assert_eq!(MedicalEdgeType::MetabolizedBy.to_string(), "metabolized_by");
        assert_eq!(MedicalEdgeType::HasPhenotype.to_string(), "has_phenotype");
        assert_eq!(MedicalEdgeType::HasVariant.to_string(), "has_variant");
        assert_eq!(MedicalEdgeType::PartOf.to_string(), "part_of");
        assert_eq!(MedicalEdgeType::RelatedTo.to_string(), "related_to");

        // PrimeKG molecular
        assert_eq!(
            MedicalEdgeType::AssociatedWith.to_string(),
            "associated_with"
        );
        assert_eq!(MedicalEdgeType::InteractsWith.to_string(), "interacts_with");
        assert_eq!(MedicalEdgeType::HasGene.to_string(), "has_gene");
        assert_eq!(MedicalEdgeType::HasProtein.to_string(), "has_protein");
        assert_eq!(MedicalEdgeType::HasEnzyme.to_string(), "has_enzyme");
        assert_eq!(
            MedicalEdgeType::HasTransporter.to_string(),
            "has_transporter"
        );
        assert_eq!(MedicalEdgeType::HasCarrier.to_string(), "has_carrier");
        assert_eq!(MedicalEdgeType::HasTarget.to_string(), "has_target");
        assert_eq!(
            MedicalEdgeType::HasSideEffect.to_string(),
            "has_side_effect"
        );
        assert_eq!(MedicalEdgeType::HasIndication.to_string(), "has_indication");
        assert_eq!(
            MedicalEdgeType::HasContraindication.to_string(),
            "has_contraindication"
        );
        assert_eq!(
            MedicalEdgeType::HasOffLabelUse.to_string(),
            "has_off_label_use"
        );
        assert_eq!(MedicalEdgeType::Expresses.to_string(), "expresses");
        assert_eq!(MedicalEdgeType::Regulates.to_string(), "regulates");
        assert_eq!(
            MedicalEdgeType::ParticipatesIn.to_string(),
            "participates_in"
        );
        assert_eq!(MedicalEdgeType::Upregulates.to_string(), "upregulates");
        assert_eq!(MedicalEdgeType::Downregulates.to_string(), "downregulates");
        assert_eq!(MedicalEdgeType::BindsTo.to_string(), "binds_to");
        assert_eq!(MedicalEdgeType::Catalyzes.to_string(), "catalyzes");
        assert_eq!(MedicalEdgeType::Transports.to_string(), "transports");
        assert_eq!(MedicalEdgeType::Inhibits.to_string(), "inhibits");
        assert_eq!(MedicalEdgeType::Activates.to_string(), "activates");
        assert_eq!(MedicalEdgeType::Modulates.to_string(), "modulates");
        assert_eq!(MedicalEdgeType::SimilarTo.to_string(), "similar_to");
        assert_eq!(MedicalEdgeType::DerivedFrom.to_string(), "derived_from");
        assert_eq!(MedicalEdgeType::MemberOf.to_string(), "member_of");

        // Clinical workflow: Temporal
        assert_eq!(MedicalEdgeType::OccursBefore.to_string(), "occurs_before");
        assert_eq!(MedicalEdgeType::OccursAfter.to_string(), "occurs_after");
        assert_eq!(
            MedicalEdgeType::ConcurrentWith.to_string(),
            "concurrent_with"
        );
        assert_eq!(MedicalEdgeType::Follows.to_string(), "follows");

        // Severity / probability
        assert_eq!(MedicalEdgeType::Exacerbates.to_string(), "exacerbates");
        assert_eq!(MedicalEdgeType::Alleviates.to_string(), "alleviates");
        assert_eq!(MedicalEdgeType::PredisposesTo.to_string(), "predisposes_to");
        assert_eq!(
            MedicalEdgeType::ProtectsAgainst.to_string(),
            "protects_against"
        );

        // Diagnostic
        assert_eq!(MedicalEdgeType::Diagnoses.to_string(), "diagnoses");
        assert_eq!(MedicalEdgeType::Confirms.to_string(), "confirms");
        assert_eq!(MedicalEdgeType::RulesOut.to_string(), "rules_out");
        assert_eq!(MedicalEdgeType::Suggests.to_string(), "suggests");
        assert_eq!(MedicalEdgeType::Indicates.to_string(), "indicates");

        // Clinical course
        assert_eq!(MedicalEdgeType::ProgressesTo.to_string(), "progresses_to");
        assert_eq!(MedicalEdgeType::ResolvesTo.to_string(), "resolves_to");
        assert_eq!(MedicalEdgeType::RecursAs.to_string(), "recurs_as");
        assert_eq!(MedicalEdgeType::Complicates.to_string(), "complicates");

        // Treatment detail
        assert_eq!(
            MedicalEdgeType::AdministeredVia.to_string(),
            "administered_via"
        );
        assert_eq!(MedicalEdgeType::CombinedWith.to_string(), "combined_with");
        assert_eq!(
            MedicalEdgeType::AlternativesTo.to_string(),
            "alternatives_to"
        );
        assert_eq!(
            MedicalEdgeType::SynergizesWith.to_string(),
            "synergizes_with"
        );
        assert_eq!(MedicalEdgeType::Antagonizes.to_string(), "antagonizes");
    }

    #[test]
    fn test_medical_node_type_serde_roundtrip() {
        let variants = vec![
            MedicalNodeType::Concept,
            MedicalNodeType::Disease,
            MedicalNodeType::Drug,
            MedicalNodeType::Gene,
            MedicalNodeType::Variant,
            MedicalNodeType::Procedure,
            MedicalNodeType::Treatment,
            MedicalNodeType::Symptom,
            MedicalNodeType::Protein,
            MedicalNodeType::BiologicalProcess,
            MedicalNodeType::MolecularFunction,
            MedicalNodeType::CellularComponent,
            MedicalNodeType::Pathway,
            MedicalNodeType::Anatomy,
            MedicalNodeType::Phenotype,
            MedicalNodeType::Enzyme,
            MedicalNodeType::Transporter,
            MedicalNodeType::Carrier,
            MedicalNodeType::Target,
            MedicalNodeType::Metabolite,
            MedicalNodeType::Exposure,
            MedicalNodeType::Chemical,
            MedicalNodeType::SideEffect,
            MedicalNodeType::Tissue,
            MedicalNodeType::CellType,
            MedicalNodeType::Organism,
            MedicalNodeType::OntologyTerm,
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: MedicalNodeType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized, "roundtrip failed for {:?}", variant);
        }
    }

    #[test]
    fn test_medical_edge_type_serde_roundtrip() {
        let variants = vec![
            MedicalEdgeType::IsA,
            MedicalEdgeType::Treats,
            MedicalEdgeType::Causes,
            MedicalEdgeType::Contraindicates,
            MedicalEdgeType::MetabolizedBy,
            MedicalEdgeType::HasPhenotype,
            MedicalEdgeType::HasVariant,
            MedicalEdgeType::PartOf,
            MedicalEdgeType::RelatedTo,
            MedicalEdgeType::AssociatedWith,
            MedicalEdgeType::LinkedTo,
            MedicalEdgeType::InteractsWith,
            MedicalEdgeType::HasGene,
            MedicalEdgeType::HasProtein,
            MedicalEdgeType::HasEnzyme,
            MedicalEdgeType::HasTransporter,
            MedicalEdgeType::HasCarrier,
            MedicalEdgeType::HasTarget,
            MedicalEdgeType::HasSideEffect,
            MedicalEdgeType::HasIndication,
            MedicalEdgeType::HasContraindication,
            MedicalEdgeType::HasOffLabelUse,
            MedicalEdgeType::HasEffect,
            MedicalEdgeType::HasExposure,
            MedicalEdgeType::HasSymptom,
            MedicalEdgeType::HasAnatomy,
            MedicalEdgeType::HasPathway,
            MedicalEdgeType::HasMolecularFunction,
            MedicalEdgeType::HasBiologicalProcess,
            MedicalEdgeType::HasCellularComponent,
            MedicalEdgeType::HasTissue,
            MedicalEdgeType::HasCellType,
            MedicalEdgeType::Expresses,
            MedicalEdgeType::Regulates,
            MedicalEdgeType::ParticipatesIn,
            MedicalEdgeType::Upregulates,
            MedicalEdgeType::Downregulates,
            MedicalEdgeType::BindsTo,
            MedicalEdgeType::Catalyzes,
            MedicalEdgeType::Transports,
            MedicalEdgeType::Inhibits,
            MedicalEdgeType::Activates,
            MedicalEdgeType::Modulates,
            MedicalEdgeType::SimilarTo,
            MedicalEdgeType::DerivedFrom,
            MedicalEdgeType::MemberOf,
            MedicalEdgeType::OccursBefore,
            MedicalEdgeType::OccursAfter,
            MedicalEdgeType::ConcurrentWith,
            MedicalEdgeType::Follows,
            MedicalEdgeType::Exacerbates,
            MedicalEdgeType::Alleviates,
            MedicalEdgeType::PredisposesTo,
            MedicalEdgeType::ProtectsAgainst,
            MedicalEdgeType::Diagnoses,
            MedicalEdgeType::Confirms,
            MedicalEdgeType::RulesOut,
            MedicalEdgeType::Suggests,
            MedicalEdgeType::Indicates,
            MedicalEdgeType::ProgressesTo,
            MedicalEdgeType::ResolvesTo,
            MedicalEdgeType::RecursAs,
            MedicalEdgeType::Complicates,
            MedicalEdgeType::AdministeredVia,
            MedicalEdgeType::CombinedWith,
            MedicalEdgeType::AlternativesTo,
            MedicalEdgeType::SynergizesWith,
            MedicalEdgeType::Antagonizes,
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: MedicalEdgeType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized, "roundtrip failed for {:?}", variant);
        }
    }

    #[test]
    fn test_medical_node_type_serde_snake_case() {
        // Verify serde uses snake_case format
        let json = serde_json::to_string(&MedicalNodeType::BiologicalProcess).unwrap();
        assert_eq!(json, "\"biological_process\"");

        let json = serde_json::to_string(&MedicalNodeType::CellType).unwrap();
        assert_eq!(json, "\"cell_type\"");

        let json = serde_json::to_string(&MedicalNodeType::OntologyTerm).unwrap();
        assert_eq!(json, "\"ontology_term\"");

        // Verify deserialization from snake_case
        let node_type: MedicalNodeType = serde_json::from_str("\"biological_process\"").unwrap();
        assert_eq!(node_type, MedicalNodeType::BiologicalProcess);
    }

    #[test]
    fn test_medical_edge_type_serde_snake_case() {
        let json = serde_json::to_string(&MedicalEdgeType::MetabolizedBy).unwrap();
        assert_eq!(json, "\"metabolized_by\"");

        let json = serde_json::to_string(&MedicalEdgeType::HasSideEffect).unwrap();
        assert_eq!(json, "\"has_side_effect\"");

        let json = serde_json::to_string(&MedicalEdgeType::OccursBefore).unwrap();
        assert_eq!(json, "\"occurs_before\"");

        let edge_type: MedicalEdgeType = serde_json::from_str("\"metabolized_by\"").unwrap();
        assert_eq!(edge_type, MedicalEdgeType::MetabolizedBy);
    }

    #[test]
    fn test_medical_node_metadata() {
        let mut metadata = MedicalNodeMetadata::new();
        assert!(metadata.is_empty());
        assert_eq!(metadata.len(), 0);

        metadata.insert("source".to_string(), "PrimeKG".to_string());
        metadata.insert("confidence".to_string(), "0.95".to_string());

        assert!(!metadata.is_empty());
        assert_eq!(metadata.len(), 2);
        assert_eq!(metadata.get("source"), Some(&"PrimeKG".to_string()));
        assert_eq!(metadata.get("confidence"), Some(&"0.95".to_string()));
        assert_eq!(metadata.get("missing"), None);
    }

    #[test]
    fn test_medical_node_metadata_serde_roundtrip() {
        let mut metadata = MedicalNodeMetadata::new();
        metadata.insert("source".to_string(), "SNOMED".to_string());
        metadata.insert("version".to_string(), "2024-01".to_string());

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: MedicalNodeMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized.get("source"), Some(&"SNOMED".to_string()));
        assert_eq!(deserialized.get("version"), Some(&"2024-01".to_string()));
    }

    #[test]
    fn test_medical_node_type_clone_copy() {
        let original = MedicalNodeType::Disease;
        let cloned = original.clone();
        let copied = original; // Copy
        assert_eq!(original, cloned);
        assert_eq!(original, copied);
    }

    #[test]
    fn test_medical_edge_type_clone_copy() {
        let original = MedicalEdgeType::Treats;
        let cloned = original.clone();
        let copied = original; // Copy
        assert_eq!(original, cloned);
        assert_eq!(original, copied);
    }

    #[test]
    fn test_medical_node_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(MedicalNodeType::Disease);
        set.insert(MedicalNodeType::Drug);
        set.insert(MedicalNodeType::Disease); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_medical_edge_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(MedicalEdgeType::Treats);
        set.insert(MedicalEdgeType::Causes);
        set.insert(MedicalEdgeType::Treats); // duplicate
        assert_eq!(set.len(), 2);
    }
}
