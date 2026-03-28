use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of defects that can be present in a plan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefectType {
    /// Missing prerequisite step
    MissingPrerequisite,
    /// Ambiguous acceptance criteria
    AmbiguousAcceptanceCriteria,
    /// Steps in wrong order
    WrongOrdering,
    /// Scope creep beyond stated goals
    ScopeCreep,
    /// Missing rollback or failure path
    MissingRollback,
    /// Contradictory statements
    ContradictoryStatements,
    /// Stale reference to outdated component
    StaleReference,
}

#[allow(dead_code)]
impl DefectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DefectType::MissingPrerequisite => "missing_prerequisite",
            DefectType::AmbiguousAcceptanceCriteria => "ambiguous_acceptance_criteria",
            DefectType::WrongOrdering => "wrong_ordering",
            DefectType::ScopeCreep => "scope_creep",
            DefectType::MissingRollback => "missing_rollback",
            DefectType::ContradictoryStatements => "contradictory_statements",
            DefectType::StaleReference => "stale_reference",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DefectType::MissingPrerequisite => "A required step or dependency is missing",
            DefectType::AmbiguousAcceptanceCriteria => "Success criteria are unclear or subjective",
            DefectType::WrongOrdering => "Steps are in an incorrect sequence",
            DefectType::ScopeCreep => "Tasks exceed the stated scope of the plan",
            DefectType::MissingRollback => "No failure recovery or rollback strategy defined",
            DefectType::ContradictoryStatements => {
                "Plan contains logically conflicting instructions"
            }
            DefectType::StaleReference => "References outdated components, APIs, or documentation",
        }
    }

    pub fn all() -> Vec<DefectType> {
        vec![
            DefectType::MissingPrerequisite,
            DefectType::AmbiguousAcceptanceCriteria,
            DefectType::WrongOrdering,
            DefectType::ScopeCreep,
            DefectType::MissingRollback,
            DefectType::ContradictoryStatements,
            DefectType::StaleReference,
        ]
    }
}

/// A specific defect instance in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defect {
    /// Unique identifier for this defect
    pub id: String,
    /// Type of defect
    pub defect_type: DefectType,
    /// Line number or location in the plan (if applicable)
    pub location: Option<String>,
    /// Description of the defect
    pub description: String,
    /// Is this a synthetic (seeded) defect or organic (naturally occurring)?
    pub is_seeded: bool,
    /// Expected fix or correction
    pub expected_fix: String,
}

/// Ground truth for a single plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGroundTruth {
    /// Plan identifier (matches filename)
    pub plan_id: String,
    /// Plan title
    pub title: String,
    /// Path to the plan file
    pub plan_path: String,
    /// All known defects in the plan (ground truth)
    pub defects: Vec<Defect>,
    /// Human-assigned difficulty score (1-5)
    pub difficulty: u8,
    /// Domain or category of the plan
    pub domain: String,
}

impl PlanGroundTruth {
    /// Get only the seeded (synthetic) defects
    pub fn seeded_defects(&self) -> Vec<&Defect> {
        self.defects.iter().filter(|d| d.is_seeded).collect()
    }

    /// Get only the organic (naturally occurring) defects
    pub fn organic_defects(&self) -> Vec<&Defect> {
        self.defects.iter().filter(|d| !d.is_seeded).collect()
    }

    /// Total defect count
    pub fn total_defects(&self) -> usize {
        self.defects.len()
    }
}

/// The complete ground truth manifest for all plans in the experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthManifest {
    /// Experiment metadata
    pub experiment_id: String,
    /// Created timestamp
    pub created_at: String,
    /// Plans in the corpus
    pub plans: Vec<PlanGroundTruth>,
    /// Defect type distribution summary
    pub defect_summary: HashMap<String, usize>,
}

#[allow(dead_code)]
impl GroundTruthManifest {
    /// Total defects across all plans
    pub fn total_defects(&self) -> usize {
        self.plans.iter().map(|p| p.total_defects()).sum()
    }

    /// Seeded defects across all plans
    pub fn total_seeded_defects(&self) -> usize {
        self.plans.iter().map(|p| p.seeded_defects().len()).sum()
    }

    /// Organic defects across all plans  
    pub fn total_organic_defects(&self) -> usize {
        self.plans.iter().map(|p| p.organic_defects().len()).sum()
    }

    /// Get defect count by type
    pub fn defect_count_by_type(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for plan in &self.plans {
            for defect in &plan.defects {
                *counts
                    .entry(defect.defect_type.as_str().to_string())
                    .or_insert(0) += 1;
            }
        }
        counts
    }
}
