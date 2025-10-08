use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeInput {
    pub session_id: Uuid,
    pub text: String,
    pub context: NarrativeContext,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeContext {
    pub urgency: UrgencyLevel,
    pub stakes: Vec<StakeType>,
    pub audience: AudienceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    High,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StakeType {
    Reputational,
    Legal,
    Financial,
    Operational,
    SocialLicense,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudienceType {
    PublicMedia,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Omission {
    pub id: Uuid,
    pub category: OmissionCategory,
    pub description: String,
    pub severity: f64,
    pub exploitability: f64,
    pub composite_risk: f64,
    pub text_reference: String,
    pub confidence: f64,
    pub suggested_addition: Option<String>,
}

impl Omission {
    pub fn new(
        category: OmissionCategory,
        description: String,
        severity: f64,
        exploitability: f64,
        text_reference: String,
        confidence: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            category,
            description,
            severity,
            exploitability,
            composite_risk: severity * exploitability,
            text_reference,
            confidence,
            suggested_addition: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OmissionCategory {
    MissingEvidence,
    UnstatedAssumption,
    AbsentStakeholder,
    ContextGap,
    UnaddressedCounterargument,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmissionCatalog {
    pub omissions: Vec<Omission>,
    pub prioritized: Vec<Uuid>,
    pub total_risk_score: f64,
}

impl OmissionCatalog {
    pub fn new(mut omissions: Vec<Omission>) -> Self {
        omissions.sort_by(|a, b| {
            b.composite_risk
                .partial_cmp(&a.composite_risk)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_risk_score = omissions.iter().map(|o| o.composite_risk).sum();

        let prioritized = omissions.iter().take(10).map(|o| o.id).collect();

        Self {
            omissions,
            prioritized,
            total_risk_score,
        }
    }

    pub fn get_top_n(&self, n: usize) -> Vec<&Omission> {
        self.omissions.iter().take(n).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateResult {
    pub pass: DebatePass,
    pub supporting_argument: Argument,
    pub opposing_argument: Argument,
    pub evaluation: DebateEvaluation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DebatePass {
    PassOne,
    PassTwo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argument {
    pub agent_name: String,
    pub role: String,
    pub main_argument: String,
    pub supporting_points: Vec<String>,
    pub counterarguments: Vec<String>,
    pub evidence_quality: f64,
    pub argument_strength: f64,
    pub omissions_referenced: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateEvaluation {
    pub scores: DebateScores,
    pub vulnerabilities: Vec<Vulnerability>,
    pub winning_position: Position,
    pub confidence: f64,
    pub key_insights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateScores {
    pub supporting_strength: f64,
    pub opposing_strength: f64,
    pub supporting_evidence: f64,
    pub opposing_evidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Position {
    Supporting,
    Opposing,
    Tie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub description: String,
    pub severity: VulnerabilitySeverity,
    pub exploitability: f64,
    pub recommended_mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Low,
    Moderate,
    High,
    Severe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CumulativeAnalysis {
    pub pass_one_results: DebateResult,
    pub pass_two_results: DebateResult,
    pub vulnerability_delta: VulnerabilityDelta,
    pub point_of_failure: Option<PointOfFailure>,
    pub strategic_risk_level: VulnerabilitySeverity,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityDelta {
    pub supporting_strength_change: f64,
    pub opposing_strength_change: f64,
    pub amplification_factor: f64,
    pub critical_omissions_exploited: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointOfFailure {
    pub narrative_claim: String,
    pub omission_exploited: Uuid,
    pub failure_mechanism: String,
    pub stakeholder_impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseStrategy {
    pub strategy_type: StrategyType,
    pub strategic_rationale: String,
    pub drafts: ResponseDrafts,
    pub risk_assessment: RiskAssessment,
    pub tone_guidance: ToneGuidance,
    pub vulnerabilities_addressed: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StrategyType {
    Reframe,
    CounterArgue,
    Bridge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDrafts {
    pub social_media: String,
    pub press_statement: String,
    pub internal_memo: String,
    pub qa_brief: Vec<QAPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAPair {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub potential_backfire: Vec<String>,
    pub stakeholder_reaction: StakeholderReaction,
    pub media_amplification_risk: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeholderReaction {
    pub supporters: String,
    pub skeptics: String,
    pub media: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToneGuidance {
    Formal,
    Empathetic,
    Assertive,
    Collaborative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasAnalysis {
    pub biases: Vec<BiasPattern>,
    pub overall_bias_score: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasPattern {
    pub bias_type: String,
    pub text: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeMapping {
    pub stakeholders: Vec<Stakeholder>,
    pub scct_classification: SCCTClassification,
    pub attribution: AttributionAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stakeholder {
    pub name: String,
    pub role: String,
    pub frame: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SCCTClassification {
    Victim,
    Accidental,
    Preventable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionAnalysis {
    pub responsibility_level: String,
    pub attribution_type: String,
    pub key_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyLinking {
    pub primary_function: String,
    pub secondary_functions: Vec<String>,
    pub subfunctions: Vec<String>,
    pub lifecycle_stage: String,
    pub recommended_playbooks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthForgeAnalysisResult {
    pub session_id: Uuid,
    pub narrative: NarrativeInput,

    pub bias_analysis: BiasAnalysis,
    pub narrative_mapping: NarrativeMapping,
    pub taxonomy_linking: TaxonomyLinking,
    pub omission_catalog: OmissionCatalog,

    pub pass_one_debate: DebateResult,
    pub pass_two_debate: DebateResult,
    pub cumulative_analysis: CumulativeAnalysis,

    pub response_strategies: Vec<ResponseStrategy>,

    pub executive_summary: String,
    pub processing_time_ms: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
