use crate::agents::bias_detector::{BiasDetectorAgent, BiasDetectorConfig};
use crate::agents::narrative_mapper::{NarrativeMapperAgent, NarrativeMapperConfig};
use crate::agents::omission_detector::{OmissionDetectorAgent, OmissionDetectorConfig};
use crate::agents::taxonomy_linker::{TaxonomyLinkerAgent, TaxonomyLinkerConfig};
use crate::error::{Result, TruthForgeError};
use crate::types::*;
use std::sync::Arc;
use terraphim_multi_agent::{GenAiLlmClient, LlmMessage, LlmRequest};
use tokio::task::JoinSet;
use tracing::{debug, info, warn};
use uuid::Uuid;

enum PassOneAgentResult {
    OmissionCatalog(OmissionCatalog),
    BiasAnalysis(BiasAnalysis),
    NarrativeMapping(NarrativeMapping),
    TaxonomyLinking(TaxonomyLinking),
}

pub struct PassOneOrchestrator {
    omission_detector: OmissionDetectorAgent,
    llm_client: Option<Arc<GenAiLlmClient>>,
}

impl PassOneOrchestrator {
    pub fn new() -> Self {
        Self {
            omission_detector: OmissionDetectorAgent::new(OmissionDetectorConfig::default()),
            llm_client: None,
        }
    }

    pub fn with_llm_client(mut self, client: Arc<GenAiLlmClient>) -> Self {
        self.llm_client = Some(client.clone());
        self.omission_detector = self.omission_detector.with_llm_client(client);
        self
    }

    pub async fn execute(&self, narrative: &NarrativeInput) -> Result<PassOneResult> {
        info!(
            "Starting Pass 1 Orchestration for session {}",
            narrative.session_id
        );

        let mut join_set = JoinSet::new();

        let narrative_text = narrative.text.clone();
        let narrative_context = narrative.context.clone();
        let session_id = narrative.session_id;
        let use_real_llm = self.llm_client.is_some();
        let llm_client = self.llm_client.clone();

        join_set.spawn(async move {
            debug!(
                "Pass 1: Running Omission Detection for session {} (real LLM: {})",
                session_id, use_real_llm
            );
            let mut detector = OmissionDetectorAgent::new(OmissionDetectorConfig::default());

            let catalog = if let Some(client) = llm_client {
                detector = detector.with_llm_client(client);
                detector
                    .detect_omissions(&narrative_text, &narrative_context)
                    .await?
            } else {
                detector
                    .detect_omissions_mock(&narrative_text, &narrative_context)
                    .await?
            };

            Ok::<PassOneAgentResult, TruthForgeError>(PassOneAgentResult::OmissionCatalog(catalog))
        });

        let narrative_text2 = narrative.text.clone();
        let narrative_context2 = narrative.context.clone();
        let session_id2 = narrative.session_id;
        let llm_client2 = self.llm_client.clone();
        let use_real_llm2 = llm_client2.is_some();

        join_set.spawn(async move {
            debug!(
                "Pass 1: Running Bias Detection for session {} (real LLM: {})",
                session_id2, use_real_llm2
            );
            let mut detector = BiasDetectorAgent::new(BiasDetectorConfig::default());

            let bias = if let Some(client) = llm_client2 {
                detector = detector.with_llm_client(client);
                detector
                    .analyze_bias(&narrative_text2, &narrative_context2)
                    .await?
            } else {
                detector
                    .analyze_bias_mock(&narrative_text2, &narrative_context2)
                    .await?
            };

            Ok::<PassOneAgentResult, TruthForgeError>(PassOneAgentResult::BiasAnalysis(bias))
        });

        let narrative_text3 = narrative.text.clone();
        let narrative_context3 = narrative.context.clone();
        let session_id3 = narrative.session_id;
        let llm_client3 = self.llm_client.clone();
        let use_real_llm3 = llm_client3.is_some();

        join_set.spawn(async move {
            debug!(
                "Pass 1: Running Narrative Mapping for session {} (real LLM: {})",
                session_id3, use_real_llm3
            );
            let mut mapper = NarrativeMapperAgent::new(NarrativeMapperConfig::default());

            let mapping = if let Some(client) = llm_client3 {
                mapper = mapper.with_llm_client(client);
                mapper
                    .map_narrative(&narrative_text3, &narrative_context3)
                    .await?
            } else {
                mapper
                    .map_narrative_mock(&narrative_text3, &narrative_context3)
                    .await?
            };

            Ok::<PassOneAgentResult, TruthForgeError>(PassOneAgentResult::NarrativeMapping(mapping))
        });

        let narrative_text4 = narrative.text.clone();
        let narrative_context4 = narrative.context.clone();
        let session_id4 = narrative.session_id;
        let llm_client4 = self.llm_client.clone();
        let use_real_llm4 = llm_client4.is_some();

        join_set.spawn(async move {
            debug!(
                "Pass 1: Running Taxonomy Linking for session {} (real LLM: {})",
                session_id4, use_real_llm4
            );
            let mut linker = TaxonomyLinkerAgent::new(TaxonomyLinkerConfig::default());

            let linking = if let Some(client) = llm_client4 {
                linker = linker.with_llm_client(client);
                linker
                    .link_taxonomy(&narrative_text4, &narrative_context4)
                    .await?
            } else {
                linker
                    .link_taxonomy_mock(&narrative_text4, &narrative_context4)
                    .await?
            };

            Ok::<PassOneAgentResult, TruthForgeError>(PassOneAgentResult::TaxonomyLinking(linking))
        });

        let mut omission_catalog = None;
        let mut bias_analysis = None;
        let mut narrative_mapping = None;
        let mut taxonomy_linking = None;

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(PassOneAgentResult::OmissionCatalog(cat))) => {
                    omission_catalog = Some(cat);
                }
                Ok(Ok(PassOneAgentResult::BiasAnalysis(ba))) => {
                    bias_analysis = Some(ba);
                }
                Ok(Ok(PassOneAgentResult::NarrativeMapping(nm))) => {
                    narrative_mapping = Some(nm);
                }
                Ok(Ok(PassOneAgentResult::TaxonomyLinking(tl))) => {
                    taxonomy_linking = Some(tl);
                }
                Ok(Err(e)) => {
                    warn!("Pass 1 agent failed: {}", e);
                }
                Err(e) => {
                    warn!("Pass 1 join error: {}", e);
                }
            }
        }

        let omission_catalog =
            omission_catalog.ok_or_else(|| TruthForgeError::WorkflowExecutionFailed {
                phase: "Pass1_OmissionDetection".to_string(),
                reason: "Omission detection failed".to_string(),
            })?;

        let bias_analysis = bias_analysis.unwrap_or_else(|| BiasAnalysis {
            biases: vec![],
            overall_bias_score: 0.0,
            confidence: 0.0,
        });

        let narrative_mapping = narrative_mapping.unwrap_or_else(|| NarrativeMapping {
            stakeholders: vec![],
            scct_classification: SCCTClassification::Accidental,
            attribution: AttributionAnalysis {
                responsibility_level: "Unknown".to_string(),
                attribution_type: "Unknown".to_string(),
                key_factors: vec![],
            },
        });

        let taxonomy_linking = taxonomy_linking.unwrap_or_else(|| TaxonomyLinking {
            primary_function: "issue_crisis_management".to_string(),
            secondary_functions: vec![],
            subfunctions: vec![],
            lifecycle_stage: "unknown".to_string(),
            recommended_playbooks: vec![],
        });

        info!(
            "Pass 1: Analysis complete, {} omissions identified",
            omission_catalog.omissions.len()
        );

        Ok(PassOneResult {
            omission_catalog,
            bias_analysis,
            narrative_mapping,
            taxonomy_linking,
        })
    }
}

impl Default for PassOneOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PassOneResult {
    pub omission_catalog: OmissionCatalog,
    pub bias_analysis: BiasAnalysis,
    pub narrative_mapping: NarrativeMapping,
    pub taxonomy_linking: TaxonomyLinking,
}

pub struct PassTwoOptimizer {
    llm_client: Option<Arc<GenAiLlmClient>>,
}

impl PassTwoOptimizer {
    pub fn new() -> Self {
        Self { llm_client: None }
    }

    pub fn with_llm_client(mut self, client: Arc<GenAiLlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    pub async fn execute(
        &self,
        narrative: &NarrativeInput,
        pass_one_result: &PassOneResult,
        pass_one_debate: &DebateResult,
    ) -> Result<PassTwoResult> {
        info!(
            "Starting Pass 2 Exploitation for session {}",
            narrative.session_id
        );

        let top_vulnerabilities =
            self.extract_top_vulnerabilities(pass_one_result, pass_one_debate);

        info!(
            "Pass 2: Identified {} exploitable vulnerabilities",
            top_vulnerabilities.len()
        );

        let (supporting_argument, opposing_argument, evaluation) = if self.llm_client.is_some() {
            let supporting = self
                .generate_defensive_argument(
                    narrative,
                    pass_one_result,
                    &top_vulnerabilities,
                    pass_one_debate,
                )
                .await?;

            let opposing = self
                .generate_exploitation_argument(
                    narrative,
                    pass_one_result,
                    &top_vulnerabilities,
                    pass_one_debate,
                    &supporting,
                )
                .await?;

            let eval = self
                .evaluate_pass_two_debate(&supporting, &opposing, &top_vulnerabilities)
                .await?;

            (supporting, opposing, eval)
        } else {
            let supporting = self
                .generate_defensive_argument_mock(narrative, &top_vulnerabilities, pass_one_debate)
                .await?;

            let opposing = self
                .generate_exploitation_argument_mock(narrative, &top_vulnerabilities, &supporting)
                .await?;

            let eval = self
                .evaluate_exploitation_debate_mock(&supporting, &opposing, &top_vulnerabilities)
                .await?;

            (supporting, opposing, eval)
        };

        info!("Pass 2: Exploitation debate complete, supporting strength: {:.2}, opposing strength: {:.2}",
            evaluation.scores.supporting_strength,
            evaluation.scores.opposing_strength
        );

        Ok(PassTwoResult {
            debate: DebateResult {
                pass: DebatePass::PassTwo,
                supporting_argument,
                opposing_argument,
                evaluation,
            },
            exploited_vulnerabilities: top_vulnerabilities,
        })
    }

    fn extract_top_vulnerabilities(
        &self,
        pass_one_result: &PassOneResult,
        _pass_one_debate: &DebateResult,
    ) -> Vec<Uuid> {
        let vulnerabilities: Vec<Uuid> = pass_one_result
            .omission_catalog
            .prioritized
            .iter()
            .take(7)
            .copied()
            .collect();

        debug!(
            "Pass 2: Targeting {} vulnerabilities for exploitation",
            vulnerabilities.len()
        );
        vulnerabilities
    }

    async fn generate_defensive_argument_mock(
        &self,
        narrative: &NarrativeInput,
        vulnerabilities: &[Uuid],
        pass_one_debate: &DebateResult,
    ) -> Result<Argument> {
        debug!("Pass 2: Generating defensive argument (mock)");

        Ok(Argument {
            agent_name: "Pass2Defender".to_string(),
            role: "pass2_supporting".to_string(),
            main_argument: format!(
                "While acknowledging {} identified weaknesses from Pass 1, the core narrative remains defensible through contextual explanations and corrective commitments.",
                vulnerabilities.len()
            ),
            supporting_points: vec![
                "Acknowledging gaps builds credibility".to_string(),
                "Context explains constraints that led to omissions".to_string(),
                "Commitments to improvement demonstrate accountability".to_string(),
            ],
            counterarguments: vec![
                "Some omissions are indefensible and must be conceded".to_string(),
            ],
            evidence_quality: 0.6,
            argument_strength: 0.55,
            omissions_referenced: vulnerabilities.to_vec(),
        })
    }

    async fn generate_exploitation_argument_mock(
        &self,
        narrative: &NarrativeInput,
        vulnerabilities: &[Uuid],
        defensive_argument: &Argument,
    ) -> Result<Argument> {
        debug!("Pass 2: Generating exploitation argument (mock)");

        let omission_reference_count = vulnerabilities.len();
        let omission_reference_percentage =
            (omission_reference_count as f64 / vulnerabilities.len() as f64) * 100.0;

        Ok(Argument {
            agent_name: "Pass2Exploiter".to_string(),
            role: "pass2_opposing".to_string(),
            main_argument: format!(
                "The {} omissions identified in Pass 1 are not isolated gaps but reveal systematic failures. Defensive acknowledgments cannot undo original harm.",
                vulnerabilities.len()
            ),
            supporting_points: vec![
                "Pattern of omissions indicates intentional concealment".to_string(),
                "Absent stakeholder voices represent betrayal of trust".to_string(),
                "Promises of future improvement don't address past harm".to_string(),
                "Context and constraints are excuses, not explanations".to_string(),
            ],
            counterarguments: vec![
                "Acknowledging weakness is spin, not accountability".to_string(),
            ],
            evidence_quality: 0.85,
            argument_strength: 0.82,
            omissions_referenced: vulnerabilities.to_vec(),
        })
    }

    async fn evaluate_exploitation_debate_mock(
        &self,
        supporting: &Argument,
        opposing: &Argument,
        vulnerabilities: &[Uuid],
    ) -> Result<DebateEvaluation> {
        debug!("Pass 2: Evaluating exploitation debate (mock)");

        let vulnerability_amplification = 0.25;

        Ok(DebateEvaluation {
            scores: DebateScores {
                supporting_strength: supporting.argument_strength,
                opposing_strength: opposing.argument_strength,
                supporting_evidence: supporting.evidence_quality,
                opposing_evidence: opposing.evidence_quality,
            },
            vulnerabilities: vulnerabilities
                .iter()
                .map(|_id| Vulnerability {
                    description: "Omission exploited in Pass 2".to_string(),
                    severity: VulnerabilitySeverity::High,
                    exploitability: 0.9,
                    recommended_mitigation: "Address with evidence or context".to_string(),
                })
                .collect(),
            winning_position: Position::Opposing,
            confidence: 0.75,
            key_insights: vec![
                "Exploitation arguments successfully weaponized identified omissions".to_string(),
                "Defensive damage control insufficient against targeted attacks".to_string(),
                "Vulnerability amplification demonstrates narrative fragility".to_string(),
            ],
        })
    }

    async fn generate_defensive_argument(
        &self,
        narrative: &NarrativeInput,
        pass_one_result: &PassOneResult,
        vulnerabilities: &[Uuid],
        pass_one_debate: &DebateResult,
    ) -> Result<Argument> {
        debug!("Pass 2: Generating defensive argument (real LLM)");

        let client = self.llm_client.as_ref().ok_or_else(|| {
            TruthForgeError::ConfigError("LLM client not configured for Pass 2".to_string())
        })?;

        let context = self.build_pass_two_context(
            narrative,
            pass_one_result,
            vulnerabilities,
            pass_one_debate,
        );

        let system_prompt =
            include_str!("../../config/roles/pass2_exploitation_supporting_role.json");
        let config: serde_json::Value = serde_json::from_str(system_prompt)?;
        let system_prompt_text = config["extra"]["system_prompt"]
            .as_str()
            .ok_or_else(|| TruthForgeError::ConfigError("Missing system_prompt".to_string()))?;

        let request = LlmRequest::new(vec![
            LlmMessage::system(system_prompt_text.to_string()),
            LlmMessage::user(context),
        ])
        .with_max_tokens(2500)
        .with_temperature(0.4);

        let response = client.generate(request).await.map_err(|e| {
            TruthForgeError::LlmError(format!("Pass2 defensive argument failed: {}", e))
        })?;

        self.parse_pass_two_argument(
            &response.content,
            "Pass2Defender",
            "pass2_supporting",
            vulnerabilities,
        )
    }

    async fn generate_exploitation_argument(
        &self,
        narrative: &NarrativeInput,
        pass_one_result: &PassOneResult,
        vulnerabilities: &[Uuid],
        pass_one_debate: &DebateResult,
        _defensive: &Argument,
    ) -> Result<Argument> {
        debug!("Pass 2: Generating exploitation argument (real LLM)");

        let client = self.llm_client.as_ref().ok_or_else(|| {
            TruthForgeError::ConfigError("LLM client not configured for Pass 2".to_string())
        })?;

        let context = self.build_pass_two_context(
            narrative,
            pass_one_result,
            vulnerabilities,
            pass_one_debate,
        );

        let system_prompt =
            include_str!("../../config/roles/pass2_exploitation_opposing_role.json");
        let config: serde_json::Value = serde_json::from_str(system_prompt)?;
        let system_prompt_text = config["extra"]["system_prompt"]
            .as_str()
            .ok_or_else(|| TruthForgeError::ConfigError("Missing system_prompt".to_string()))?;

        let request = LlmRequest::new(vec![
            LlmMessage::system(system_prompt_text.to_string()),
            LlmMessage::user(context),
        ])
        .with_max_tokens(3000)
        .with_temperature(0.5);

        let response = client.generate(request).await.map_err(|e| {
            TruthForgeError::LlmError(format!("Pass2 exploitation argument failed: {}", e))
        })?;

        self.parse_pass_two_argument(
            &response.content,
            "Pass2Exploiter",
            "pass2_opposing",
            vulnerabilities,
        )
    }

    async fn evaluate_pass_two_debate(
        &self,
        supporting: &Argument,
        opposing: &Argument,
        vulnerabilities: &[Uuid],
    ) -> Result<DebateEvaluation> {
        debug!("Pass 2: Evaluating exploitation debate (real LLM)");

        Ok(DebateEvaluation {
            scores: DebateScores {
                supporting_strength: supporting.argument_strength,
                opposing_strength: opposing.argument_strength,
                supporting_evidence: supporting.evidence_quality,
                opposing_evidence: opposing.evidence_quality,
            },
            vulnerabilities: vulnerabilities
                .iter()
                .map(|_id| Vulnerability {
                    description: "Omission exploited in Pass 2".to_string(),
                    severity: VulnerabilitySeverity::High,
                    exploitability: 0.9,
                    recommended_mitigation: "Address with evidence or context".to_string(),
                })
                .collect(),
            winning_position: if opposing.argument_strength > supporting.argument_strength {
                Position::Opposing
            } else {
                Position::Supporting
            },
            confidence: 0.75,
            key_insights: vec![
                "Exploitation arguments weaponized identified omissions".to_string(),
                "Defensive damage control tested against targeted attacks".to_string(),
            ],
        })
    }

    fn build_pass_two_context(
        &self,
        narrative: &NarrativeInput,
        pass_one_result: &PassOneResult,
        vulnerabilities: &[Uuid],
        pass_one_debate: &DebateResult,
    ) -> String {
        let vulnerability_details = pass_one_result
            .omission_catalog
            .omissions
            .iter()
            .filter(|o| vulnerabilities.contains(&o.id))
            .map(|o| {
                format!(
                    "- {} (severity: {:.2}, exploitability: {:.2})",
                    o.description, o.severity, o.exploitability
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let pass1_evaluator_findings = pass_one_debate.evaluation.key_insights.join("\n- ");

        format!(
            "**Original Narrative**:\n{}\n\n\
            **Pass 1 Debate Outcome**:\n\
            - Supporting Strength: {:.2}\n\
            - Opposing Strength: {:.2}\n\
            - Winner: {:?}\n\n\
            **Pass 1 Evaluator Key Insights**:\n- {}\n\n\
            **Top {} Vulnerabilities to Exploit**:\n{}\n\n\
            **Pass 1 Supporting Argument**:\n{}\n\n\
            **Pass 1 Opposing Argument**:\n{}",
            narrative.text,
            pass_one_debate.evaluation.scores.supporting_strength,
            pass_one_debate.evaluation.scores.opposing_strength,
            pass_one_debate.evaluation.winning_position,
            pass1_evaluator_findings,
            vulnerabilities.len(),
            vulnerability_details,
            pass_one_debate.supporting_argument.main_argument,
            pass_one_debate.opposing_argument.main_argument
        )
    }

    fn parse_pass_two_argument(
        &self,
        content: &str,
        agent_name: &str,
        role: &str,
        vulnerabilities: &[Uuid],
    ) -> Result<Argument> {
        let content = content.trim();
        let json_str = if content.starts_with("```json") {
            content
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim()
        } else if content.starts_with("```") {
            content
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        } else {
            content
        };

        let llm_response: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            TruthForgeError::ParseError(format!("Failed to parse Pass2 argument JSON: {}", e))
        })?;

        let main_argument = llm_response["opening_exploitation"]
            .as_str()
            .or_else(|| llm_response["opening_acknowledgment"].as_str())
            .or_else(|| llm_response["main_argument"].as_str())
            .unwrap_or("No main argument provided")
            .to_string();

        let supporting_points = llm_response["targeted_attacks"]
            .as_array()
            .or_else(|| llm_response["strengthened_defenses"].as_array())
            .or_else(|| llm_response["supporting_points"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        v.as_str().or_else(|| {
                            v.as_object()
                                .and_then(|o| o.get("attack").and_then(|a| a.as_str()))
                        })
                    })
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let counterarguments = llm_response["rejection_of_defenses"]
            .as_array()
            .or_else(|| llm_response["strategic_concessions"].as_array())
            .or_else(|| llm_response["counterarguments"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let evidence_quality = llm_response["evidence_quality"]
            .as_f64()
            .unwrap_or(0.75)
            .clamp(0.0, 1.0);

        let argument_strength = llm_response["exploitation_effectiveness"]
            .as_f64()
            .or_else(|| llm_response["damage_assessment"].as_f64())
            .or_else(|| llm_response["argument_strength"].as_f64())
            .unwrap_or(0.70)
            .clamp(0.0, 1.0);

        Ok(Argument {
            agent_name: agent_name.to_string(),
            role: role.to_string(),
            main_argument,
            supporting_points,
            counterarguments,
            evidence_quality,
            argument_strength,
            omissions_referenced: vulnerabilities.to_vec(),
        })
    }
}

impl Default for PassTwoOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PassTwoResult {
    pub debate: DebateResult,
    pub exploited_vulnerabilities: Vec<Uuid>,
}

pub struct TwoPassDebateWorkflow {
    pass_one: PassOneOrchestrator,
    pass_two: PassTwoOptimizer,
    response_generator: ResponseGenerator,
    llm_client: Option<Arc<GenAiLlmClient>>,
}

impl TwoPassDebateWorkflow {
    pub fn new() -> Self {
        Self {
            pass_one: PassOneOrchestrator::new(),
            pass_two: PassTwoOptimizer::new(),
            response_generator: ResponseGenerator::new(),
            llm_client: None,
        }
    }

    pub fn with_llm_client(mut self, client: Arc<GenAiLlmClient>) -> Self {
        self.pass_one = self.pass_one.with_llm_client(client.clone());
        self.pass_two = self.pass_two.with_llm_client(client.clone());
        self.response_generator = self.response_generator.with_llm_client(client.clone());
        self.llm_client = Some(client);
        self
    }

    pub async fn execute(&self, narrative: &NarrativeInput) -> Result<TruthForgeAnalysisResult> {
        let start_time = std::time::Instant::now();

        info!(
            "Starting TwoPassDebateWorkflow for session {}",
            narrative.session_id
        );

        let pass_one_result = self.pass_one.execute(narrative).await?;

        info!("Pass 1 complete, generating Pass 1 debate");
        let pass_one_debate = if self.llm_client.is_some() {
            self.generate_pass_one_debate(narrative, &pass_one_result)
                .await?
        } else {
            self.generate_pass_one_debate_mock(narrative, &pass_one_result)
                .await?
        };

        info!("Pass 1 debate complete, starting Pass 2 exploitation");
        let pass_two_result = self
            .pass_two
            .execute(narrative, &pass_one_result, &pass_one_debate)
            .await?;

        info!("Pass 2 complete, generating cumulative analysis");

        let omissions_count = pass_one_result.omission_catalog.omissions.len();
        let exploited_count = pass_two_result.exploited_vulnerabilities.len();

        let cumulative_analysis = self
            .generate_cumulative_analysis_mock(
                &pass_one_debate,
                &pass_two_result.debate,
                &pass_two_result.exploited_vulnerabilities,
            )
            .await?;

        let risk_level = cumulative_analysis.strategic_risk_level.clone();

        info!("Cumulative analysis complete, generating response strategies");
        let response_strategies = self
            .response_generator
            .generate_strategies(
                narrative,
                &cumulative_analysis,
                &pass_one_result.omission_catalog,
            )
            .await?;

        info!(
            "Response strategies generated: {} strategies",
            response_strategies.len()
        );

        let result = TruthForgeAnalysisResult {
            session_id: narrative.session_id,
            narrative: narrative.clone(),
            bias_analysis: pass_one_result.bias_analysis,
            narrative_mapping: pass_one_result.narrative_mapping,
            taxonomy_linking: pass_one_result.taxonomy_linking,
            omission_catalog: pass_one_result.omission_catalog,
            pass_one_debate: pass_one_debate.clone(),
            pass_two_debate: pass_two_result.debate.clone(),
            cumulative_analysis,
            response_strategies,
            executive_summary: format!(
                "Pass 1 identified {} omissions. Pass 2 exploited {} vulnerabilities, demonstrating {:?} risk level. Generated 3 response strategies.",
                omissions_count,
                exploited_count,
                risk_level
            ),
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            created_at: chrono::Utc::now(),
        };

        info!(
            "TwoPassDebateWorkflow complete for session {} in {}ms",
            narrative.session_id, result.processing_time_ms
        );

        Ok(result)
    }

    pub async fn generate_pass_one_debate_mock(
        &self,
        _narrative: &NarrativeInput,
        pass_one_result: &PassOneResult,
    ) -> Result<DebateResult> {
        debug!("Generating Pass 1 debate (mock)");

        let supporting_argument = Argument {
            agent_name: "Pass1Debater_Supporting".to_string(),
            role: "pass1_supporting".to_string(),
            main_argument:
                "The narrative demonstrates commitment to transparency and stakeholder value."
                    .to_string(),
            supporting_points: vec![
                "Factual claims are supported by available evidence".to_string(),
                "Actions align with stated organizational values".to_string(),
            ],
            counterarguments: vec![
                "Acknowledges that some context was not included due to space constraints"
                    .to_string(),
            ],
            evidence_quality: 0.7,
            argument_strength: 0.65,
            omissions_referenced: vec![],
        };

        let opposing_argument = Argument {
            agent_name: "Pass1Debater_Opposing".to_string(),
            role: "pass1_opposing".to_string(),
            main_argument: format!(
                "The narrative has {} significant omissions that raise questions about completeness and intent.",
                pass_one_result.omission_catalog.omissions.len()
            ),
            supporting_points: vec![
                "Missing evidence for key claims undermines credibility".to_string(),
                "Absent stakeholder voices suggest one-sided perspective".to_string(),
                "Unstated assumptions reveal biased framing".to_string(),
            ],
            counterarguments: vec![
                "Some factual statements are not disputed".to_string(),
            ],
            evidence_quality: 0.75,
            argument_strength: 0.70,
            omissions_referenced: pass_one_result.omission_catalog.prioritized.iter().take(5).copied().collect(),
        };

        let evaluation = DebateEvaluation {
            scores: DebateScores {
                supporting_strength: supporting_argument.argument_strength,
                opposing_strength: opposing_argument.argument_strength,
                supporting_evidence: supporting_argument.evidence_quality,
                opposing_evidence: opposing_argument.evidence_quality,
            },
            vulnerabilities: pass_one_result
                .omission_catalog
                .prioritized
                .iter()
                .take(5)
                .map(|_id| Vulnerability {
                    description: "Identified weakness in Pass 1".to_string(),
                    severity: VulnerabilitySeverity::Moderate,
                    exploitability: 0.6,
                    recommended_mitigation: "Monitor for exploitation in Pass 2".to_string(),
                })
                .collect(),
            winning_position: Position::Opposing,
            confidence: 0.6,
            key_insights: vec![
                "Omissions create vulnerability to criticism".to_string(),
                "Defensive arguments acknowledge some weaknesses".to_string(),
            ],
        };

        Ok(DebateResult {
            pass: DebatePass::PassOne,
            supporting_argument,
            opposing_argument,
            evaluation,
        })
    }

    pub async fn generate_pass_one_debate(
        &self,
        narrative: &NarrativeInput,
        pass_one_result: &PassOneResult,
    ) -> Result<DebateResult> {
        debug!("Generating Pass 1 debate (real LLM)");

        let client = self.llm_client.as_ref().ok_or_else(|| {
            TruthForgeError::ConfigError("LLM client not configured for debate".to_string())
        })?;

        let context = self.build_debate_context(narrative, pass_one_result);

        let supporting_argument = self.generate_supporting_argument(client, &context).await?;
        let opposing_argument = self
            .generate_opposing_argument(client, &context, &supporting_argument)
            .await?;
        let evaluation = self
            .evaluate_pass_one_debate(client, &context, &supporting_argument, &opposing_argument)
            .await?;

        Ok(DebateResult {
            pass: DebatePass::PassOne,
            supporting_argument,
            opposing_argument,
            evaluation,
        })
    }

    fn build_debate_context(
        &self,
        narrative: &NarrativeInput,
        pass_one_result: &PassOneResult,
    ) -> String {
        let omissions_summary = pass_one_result
            .omission_catalog
            .omissions
            .iter()
            .take(5)
            .map(|o| {
                format!(
                    "- {} (severity: {:.2}, exploitability: {:.2})",
                    o.description, o.severity, o.exploitability
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let bias_summary = format!(
            "Overall bias score: {:.2}, Confidence: {:.2}, Patterns detected: {}",
            pass_one_result.bias_analysis.overall_bias_score,
            pass_one_result.bias_analysis.confidence,
            pass_one_result.bias_analysis.biases.len()
        );

        let stakeholders = pass_one_result
            .narrative_mapping
            .stakeholders
            .iter()
            .map(|s| format!("- {}: {} (frame: {})", s.name, s.role, s.frame))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "**Original Narrative**:\n{}\n\n\
            **Identified Omissions** ({} total):\n{}\n\n\
            **Bias Analysis**:\n{}\n\n\
            **Stakeholders**:\n{}\n\n\
            **SCCT Classification**: {:?}\n\n\
            **Taxonomy Function**: {}",
            narrative.text,
            pass_one_result.omission_catalog.omissions.len(),
            omissions_summary,
            bias_summary,
            stakeholders,
            pass_one_result.narrative_mapping.scct_classification,
            pass_one_result.taxonomy_linking.primary_function
        )
    }

    async fn generate_supporting_argument(
        &self,
        client: &GenAiLlmClient,
        context: &str,
    ) -> Result<Argument> {
        let system_prompt = include_str!("../../config/roles/pass1_debater_supporting_role.json");
        let config: serde_json::Value = serde_json::from_str(system_prompt)?;
        let system_prompt_text = config["extra"]["system_prompt"]
            .as_str()
            .ok_or_else(|| TruthForgeError::ConfigError("Missing system_prompt".to_string()))?;

        let request = LlmRequest::new(vec![
            LlmMessage::system(system_prompt_text.to_string()),
            LlmMessage::user(context.to_string()),
        ])
        .with_max_tokens(2500)
        .with_temperature(0.4);

        let response = client.generate(request).await.map_err(|e| {
            TruthForgeError::LlmError(format!("Supporting argument generation failed: {}", e))
        })?;

        self.parse_debate_argument(
            &response.content,
            "Pass1Debater_Supporting",
            "pass1_supporting",
        )
    }

    async fn generate_opposing_argument(
        &self,
        client: &GenAiLlmClient,
        context: &str,
        _supporting: &Argument,
    ) -> Result<Argument> {
        let system_prompt = include_str!("../../config/roles/pass1_debater_opposing_role.json");
        let config: serde_json::Value = serde_json::from_str(system_prompt)?;
        let system_prompt_text = config["extra"]["system_prompt"]
            .as_str()
            .ok_or_else(|| TruthForgeError::ConfigError("Missing system_prompt".to_string()))?;

        let request = LlmRequest::new(vec![
            LlmMessage::system(system_prompt_text.to_string()),
            LlmMessage::user(context.to_string()),
        ])
        .with_max_tokens(2500)
        .with_temperature(0.4);

        let response = client.generate(request).await.map_err(|e| {
            TruthForgeError::LlmError(format!("Opposing argument generation failed: {}", e))
        })?;

        self.parse_debate_argument(&response.content, "Pass1Debater_Opposing", "pass1_opposing")
    }

    async fn evaluate_pass_one_debate(
        &self,
        client: &GenAiLlmClient,
        context: &str,
        supporting: &Argument,
        opposing: &Argument,
    ) -> Result<DebateEvaluation> {
        let system_prompt = include_str!("../../config/roles/pass1_evaluator_role.json");
        let config: serde_json::Value = serde_json::from_str(system_prompt)?;
        let system_prompt_text = config["extra"]["system_prompt"]
            .as_str()
            .ok_or_else(|| TruthForgeError::ConfigError("Missing system_prompt".to_string()))?;

        let evaluation_context = format!(
            "{}\n\n**Supporting Argument**:\n{}\n\n**Opposing Argument**:\n{}",
            context, supporting.main_argument, opposing.main_argument
        );

        let request = LlmRequest::new(vec![
            LlmMessage::system(system_prompt_text.to_string()),
            LlmMessage::user(evaluation_context),
        ])
        .with_max_tokens(3000)
        .with_temperature(0.3);

        let response = client
            .generate(request)
            .await
            .map_err(|e| TruthForgeError::LlmError(format!("Debate evaluation failed: {}", e)))?;

        self.parse_debate_evaluation(&response.content)
    }

    fn parse_debate_argument(
        &self,
        content: &str,
        agent_name: &str,
        role: &str,
    ) -> Result<Argument> {
        let json_str = self.strip_markdown(content);

        let llm_response: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            TruthForgeError::ParseError(format!("Failed to parse argument JSON: {}", e))
        })?;

        let main_argument = llm_response["opening_statement"]
            .as_str()
            .or_else(|| llm_response["main_argument"].as_str())
            .unwrap_or("No main argument provided")
            .to_string();

        let supporting_points = llm_response["key_claims"]
            .as_array()
            .or_else(|| llm_response["key_challenges"].as_array())
            .or_else(|| llm_response["supporting_points"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        v.as_str().or_else(|| {
                            v.as_object()
                                .and_then(|o| o.get("claim").and_then(|c| c.as_str()))
                        })
                    })
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let counterarguments = llm_response["acknowledged_gaps"]
            .as_array()
            .or_else(|| llm_response["rebuttal_preview"].as_array())
            .or_else(|| llm_response["counterarguments"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        v.as_str().or_else(|| {
                            v.as_object()
                                .and_then(|o| o.get("gap").and_then(|g| g.as_str()))
                        })
                    })
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let evidence_quality = llm_response["evidence_quality"]
            .as_f64()
            .unwrap_or(0.7)
            .clamp(0.0, 1.0);

        let argument_strength = llm_response["overall_strength"]
            .as_f64()
            .or_else(|| llm_response["argument_strength"].as_f64())
            .unwrap_or(0.65)
            .clamp(0.0, 1.0);

        Ok(Argument {
            agent_name: agent_name.to_string(),
            role: role.to_string(),
            main_argument,
            supporting_points,
            counterarguments,
            evidence_quality,
            argument_strength,
            omissions_referenced: vec![],
        })
    }

    fn parse_debate_evaluation(&self, content: &str) -> Result<DebateEvaluation> {
        let json_str = self.strip_markdown(content);

        let llm_response: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            TruthForgeError::ParseError(format!("Failed to parse evaluation JSON: {}", e))
        })?;

        let supporting_strength = llm_response["supporting_score"]
            .as_f64()
            .or_else(|| llm_response["score_breakdown"]["supporting"]["overall"].as_f64())
            .unwrap_or(0.65)
            .clamp(0.0, 1.0);

        let opposing_strength = llm_response["opposing_score"]
            .as_f64()
            .or_else(|| llm_response["score_breakdown"]["opposing"]["overall"].as_f64())
            .unwrap_or(0.70)
            .clamp(0.0, 1.0);

        let scores = DebateScores {
            supporting_strength,
            opposing_strength,
            supporting_evidence: llm_response["score_breakdown"]["supporting"]["evidence"]
                .as_f64()
                .unwrap_or(0.7)
                .clamp(0.0, 1.0),
            opposing_evidence: llm_response["score_breakdown"]["opposing"]["evidence"]
                .as_f64()
                .unwrap_or(0.75)
                .clamp(0.0, 1.0),
        };

        let winner_str = llm_response["winner"].as_str().unwrap_or("opposing");
        let winning_position = match winner_str.to_lowercase().as_str() {
            s if s.contains("support") => Position::Supporting,
            s if s.contains("oppos") => Position::Opposing,
            _ => Position::Tie,
        };

        let vulnerabilities = llm_response["key_vulnerabilities"]
            .as_array()
            .or_else(|| llm_response["pass2_exploitation_targets"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let desc = v
                            .as_str()
                            .or_else(|| v.as_object()?.get("description")?.as_str())?
                            .to_string();
                        let severity_str = v
                            .as_object()
                            .and_then(|o| o.get("severity"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("moderate");
                        let severity = match severity_str.to_lowercase().as_str() {
                            s if s.contains("severe") || s.contains("critical") => {
                                VulnerabilitySeverity::Severe
                            }
                            s if s.contains("high") => VulnerabilitySeverity::High,
                            s if s.contains("low") => VulnerabilitySeverity::Low,
                            _ => VulnerabilitySeverity::Moderate,
                        };
                        Some(Vulnerability {
                            description: desc,
                            severity,
                            exploitability: v
                                .as_object()
                                .and_then(|o| o.get("exploitability"))
                                .and_then(|e| e.as_f64())
                                .unwrap_or(0.6)
                                .clamp(0.0, 1.0),
                            recommended_mitigation: "Monitor for exploitation in Pass 2"
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![Vulnerability {
                    description: "Identified weakness in Pass 1".to_string(),
                    severity: VulnerabilitySeverity::Moderate,
                    exploitability: 0.6,
                    recommended_mitigation: "Monitor for exploitation in Pass 2".to_string(),
                }]
            });

        let key_insights = llm_response["key_insights"]
            .as_array()
            .or_else(|| llm_response["unanswered_questions"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    "Omissions create vulnerability to criticism".to_string(),
                    "Defensive arguments acknowledge some weaknesses".to_string(),
                ]
            });

        let confidence = llm_response["confidence"]
            .as_f64()
            .unwrap_or(0.6)
            .clamp(0.0, 1.0);

        Ok(DebateEvaluation {
            scores,
            vulnerabilities,
            winning_position,
            confidence,
            key_insights,
        })
    }

    fn strip_markdown<'a>(&self, content: &'a str) -> &'a str {
        let content = content.trim();
        if content.starts_with("```json") {
            content
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim()
        } else if content.starts_with("```") {
            content
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        } else {
            content
        }
    }

    pub async fn generate_cumulative_analysis_mock(
        &self,
        pass_one_debate: &DebateResult,
        pass_two_debate: &DebateResult,
        exploited_vulnerabilities: &[Uuid],
    ) -> Result<CumulativeAnalysis> {
        debug!("Generating cumulative analysis (mock)");

        let vulnerability_delta = VulnerabilityDelta {
            supporting_strength_change: pass_two_debate.evaluation.scores.supporting_strength
                - pass_one_debate.evaluation.scores.supporting_strength,
            opposing_strength_change: pass_two_debate.evaluation.scores.opposing_strength
                - pass_one_debate.evaluation.scores.opposing_strength,
            amplification_factor: pass_two_debate.evaluation.scores.opposing_strength
                / pass_one_debate.evaluation.scores.opposing_strength.max(0.1),
            critical_omissions_exploited: exploited_vulnerabilities.len(),
        };

        let delta_magnitude = (pass_one_debate.evaluation.scores.supporting_strength
            - pass_two_debate.evaluation.scores.supporting_strength)
            .abs();

        let strategic_risk_level = if delta_magnitude > 0.40 {
            VulnerabilitySeverity::Severe
        } else if delta_magnitude > 0.25 {
            VulnerabilitySeverity::High
        } else if delta_magnitude > 0.10 {
            VulnerabilitySeverity::Moderate
        } else {
            VulnerabilitySeverity::Low
        };

        let point_of_failure = exploited_vulnerabilities.first().map(|id| PointOfFailure {
            narrative_claim: "Primary defensive position".to_string(),
            omission_exploited: *id,
            failure_mechanism: "Critical omission that collapsed defense in Pass 2".to_string(),
            stakeholder_impact: "Defensive arguments unable to counter targeted exploitation"
                .to_string(),
        });

        Ok(CumulativeAnalysis {
            pass_one_results: pass_one_debate.clone(),
            pass_two_results: pass_two_debate.clone(),
            vulnerability_delta,
            point_of_failure,
            strategic_risk_level,
            recommended_actions: vec![
                "Address top 3 omissions with evidence or context".to_string(),
                "Engage absent stakeholder voices proactively".to_string(),
                "Revise narrative framing to minimize exploitation vectors".to_string(),
            ],
        })
    }
}

pub struct ResponseGenerator {
    llm_client: Option<Arc<GenAiLlmClient>>,
}

impl Default for ResponseGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseGenerator {
    pub fn new() -> Self {
        Self { llm_client: None }
    }

    pub fn with_llm_client(mut self, client: Arc<GenAiLlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    pub async fn generate_strategies(
        &self,
        narrative: &NarrativeInput,
        cumulative_analysis: &CumulativeAnalysis,
        omission_catalog: &OmissionCatalog,
    ) -> Result<Vec<ResponseStrategy>> {
        info!(
            "Generating response strategies for session {}",
            narrative.session_id
        );

        let mut strategies = Vec::new();

        let reframe_strategy = if self.llm_client.is_some() {
            self.generate_reframe_strategy(narrative, cumulative_analysis, omission_catalog)
                .await?
        } else {
            self.generate_reframe_strategy_mock(narrative, cumulative_analysis, omission_catalog)
                .await?
        };
        strategies.push(reframe_strategy);

        let counter_argue_strategy = if self.llm_client.is_some() {
            self.generate_counter_argue_strategy(narrative, cumulative_analysis, omission_catalog)
                .await?
        } else {
            self.generate_counter_argue_strategy_mock(
                narrative,
                cumulative_analysis,
                omission_catalog,
            )
            .await?
        };
        strategies.push(counter_argue_strategy);

        let bridge_strategy = if self.llm_client.is_some() {
            self.generate_bridge_strategy(narrative, cumulative_analysis, omission_catalog)
                .await?
        } else {
            self.generate_bridge_strategy_mock(narrative, cumulative_analysis, omission_catalog)
                .await?
        };
        strategies.push(bridge_strategy);

        info!(
            "Response strategies generated: {} strategies for session {}",
            strategies.len(),
            narrative.session_id
        );

        Ok(strategies)
    }

    async fn generate_reframe_strategy(
        &self,
        narrative: &NarrativeInput,
        cumulative_analysis: &CumulativeAnalysis,
        omission_catalog: &OmissionCatalog,
    ) -> Result<ResponseStrategy> {
        debug!("Generating Reframe strategy (real LLM)");

        let client = self.llm_client.as_ref().ok_or_else(|| {
            TruthForgeError::ConfigError(
                "LLM client not configured for ResponseGenerator".to_string(),
            )
        })?;

        let context = self.build_strategy_context(narrative, cumulative_analysis, omission_catalog);

        let system_prompt = include_str!("../../config/roles/reframe_agent_role.json");
        let config: serde_json::Value = serde_json::from_str(system_prompt)?;
        let system_prompt_text = config["extra"]["system_prompt"].as_str().ok_or_else(|| {
            TruthForgeError::ConfigError(
                "Missing system_prompt in reframe_agent_role.json".to_string(),
            )
        })?;

        let request = LlmRequest::new(vec![
            LlmMessage::system(system_prompt_text.to_string()),
            LlmMessage::user(context),
        ])
        .with_max_tokens(2500)
        .with_temperature(0.4);

        let response = client
            .generate(request)
            .await
            .map_err(|e| TruthForgeError::LlmError(format!("Reframe strategy failed: {}", e)))?;

        self.parse_response_strategy(
            &response.content,
            StrategyType::Reframe,
            ToneGuidance::Empathetic,
            omission_catalog,
        )
    }

    async fn generate_counter_argue_strategy(
        &self,
        narrative: &NarrativeInput,
        cumulative_analysis: &CumulativeAnalysis,
        omission_catalog: &OmissionCatalog,
    ) -> Result<ResponseStrategy> {
        debug!("Generating Counter-Argue strategy (real LLM)");

        let client = self.llm_client.as_ref().ok_or_else(|| {
            TruthForgeError::ConfigError(
                "LLM client not configured for ResponseGenerator".to_string(),
            )
        })?;

        let context = self.build_strategy_context(narrative, cumulative_analysis, omission_catalog);

        let system_prompt = include_str!("../../config/roles/counter_argue_agent_role.json");
        let config: serde_json::Value = serde_json::from_str(system_prompt)?;
        let system_prompt_text = config["extra"]["system_prompt"].as_str().ok_or_else(|| {
            TruthForgeError::ConfigError(
                "Missing system_prompt in counter_argue_agent_role.json".to_string(),
            )
        })?;

        let request = LlmRequest::new(vec![
            LlmMessage::system(system_prompt_text.to_string()),
            LlmMessage::user(context),
        ])
        .with_max_tokens(2500)
        .with_temperature(0.3);

        let response = client.generate(request).await.map_err(|e| {
            TruthForgeError::LlmError(format!("Counter-argue strategy failed: {}", e))
        })?;

        self.parse_response_strategy(
            &response.content,
            StrategyType::CounterArgue,
            ToneGuidance::Assertive,
            omission_catalog,
        )
    }

    async fn generate_bridge_strategy(
        &self,
        narrative: &NarrativeInput,
        cumulative_analysis: &CumulativeAnalysis,
        omission_catalog: &OmissionCatalog,
    ) -> Result<ResponseStrategy> {
        debug!("Generating Bridge strategy (real LLM)");

        let client = self.llm_client.as_ref().ok_or_else(|| {
            TruthForgeError::ConfigError(
                "LLM client not configured for ResponseGenerator".to_string(),
            )
        })?;

        let context = self.build_strategy_context(narrative, cumulative_analysis, omission_catalog);

        let system_prompt = include_str!("../../config/roles/bridge_agent_role.json");
        let config: serde_json::Value = serde_json::from_str(system_prompt)?;
        let system_prompt_text = config["extra"]["system_prompt"].as_str().ok_or_else(|| {
            TruthForgeError::ConfigError(
                "Missing system_prompt in bridge_agent_role.json".to_string(),
            )
        })?;

        let request = LlmRequest::new(vec![
            LlmMessage::system(system_prompt_text.to_string()),
            LlmMessage::user(context),
        ])
        .with_max_tokens(2500)
        .with_temperature(0.4);

        let response = client
            .generate(request)
            .await
            .map_err(|e| TruthForgeError::LlmError(format!("Bridge strategy failed: {}", e)))?;

        self.parse_response_strategy(
            &response.content,
            StrategyType::Bridge,
            ToneGuidance::Collaborative,
            omission_catalog,
        )
    }

    fn build_strategy_context(
        &self,
        narrative: &NarrativeInput,
        cumulative_analysis: &CumulativeAnalysis,
        omission_catalog: &OmissionCatalog,
    ) -> String {
        let top_omissions = omission_catalog
            .omissions
            .iter()
            .take(5)
            .map(|o| {
                format!(
                    "- {} (severity: {:.2}, exploitability: {:.2})",
                    o.description, o.severity, o.exploitability
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let recommended_actions = cumulative_analysis
            .recommended_actions
            .iter()
            .map(|a| format!("- {}", a))
            .collect::<Vec<_>>()
            .join("\n");

        let point_of_failure_text = cumulative_analysis
            .point_of_failure
            .as_ref()
            .map(|pof| {
                format!(
                    "{} (exploited omission: {})",
                    pof.failure_mechanism, pof.omission_exploited
                )
            })
            .unwrap_or_else(|| "No single point of failure identified".to_string());

        format!(
            "**Original Narrative**:\n{}\n\n\
            **Urgency**: {:?}\n\
            **Stakes**: {:?}\n\
            **Audience**: {:?}\n\n\
            **Strategic Risk Level**: {:?}\n\
            **Top 5 Omissions**:\n{}\n\n\
            **Vulnerability Delta**:\n\
            - Supporting Strength Change: {:.2}\n\
            - Opposing Strength Change: {:.2}\n\
            - Amplification Factor: {:.2}\n\
            - Critical Omissions Exploited: {}\n\n\
            **Recommended Actions**:\n{}\n\n\
            **Point of Failure**: {}",
            narrative.text,
            narrative.context.urgency,
            narrative.context.stakes,
            narrative.context.audience,
            cumulative_analysis.strategic_risk_level,
            top_omissions,
            cumulative_analysis
                .vulnerability_delta
                .supporting_strength_change,
            cumulative_analysis
                .vulnerability_delta
                .opposing_strength_change,
            cumulative_analysis.vulnerability_delta.amplification_factor,
            cumulative_analysis
                .vulnerability_delta
                .critical_omissions_exploited,
            recommended_actions,
            point_of_failure_text
        )
    }

    fn parse_response_strategy(
        &self,
        content: &str,
        strategy_type: StrategyType,
        tone_guidance: ToneGuidance,
        omission_catalog: &OmissionCatalog,
    ) -> Result<ResponseStrategy> {
        let content = content.trim();

        // Try to extract JSON from response (Python-style)
        let json_str = if let Some(json_start) = content.find('{') {
            if let Some(json_end) = content.rfind('}') {
                if json_end > json_start {
                    &content[json_start..=json_end]
                } else {
                    content
                }
            } else {
                content
            }
        } else {
            content
        };

        // Try to parse as JSON, fallback to text extraction if it fails
        let llm_response: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(json) => json,
            Err(e) => {
                warn!("Failed to parse response strategy JSON. Content preview: {}",
                      &content[..content.len().min(200)]);
                warn!("Falling back to markdown-based strategy extraction");

                // Parse structured markdown content
                return self.parse_response_strategy_from_markdown(
                    content,
                    strategy_type,
                    tone_guidance,
                    omission_catalog,
                );
            }
        };

        let revised_narrative = llm_response["revised_narrative"]
            .as_str()
            .or_else(|| llm_response["bridge_letter"].as_str())
            .unwrap_or("No revised narrative provided")
            .to_string();

        let empty_array = vec![];
        let key_messages_json = llm_response["key_messages"]
            .as_array()
            .or_else(|| llm_response["key_message_document"].as_array())
            .unwrap_or(&empty_array);
        let _key_messages: Vec<String> = key_messages_json
            .iter()
            .filter_map(|v| {
                v.as_str().or_else(|| {
                    v.as_object()
                        .and_then(|o| o.get("message").and_then(|m| m.as_str()))
                })
            })
            .map(String::from)
            .collect();

        let empty_qa_array = vec![];
        let qa_pairs_json = llm_response["qa_brief"]
            .as_array()
            .unwrap_or(&empty_qa_array);
        let qa_brief: Vec<QAPair> = qa_pairs_json
            .iter()
            .filter_map(|v| {
                v.as_object().and_then(|obj| {
                    let question = obj.get("question")?.as_str()?.to_string();
                    let answer = obj.get("answer")?.as_str()?.to_string();
                    Some(QAPair { question, answer })
                })
            })
            .collect();

        let social_media = llm_response["social_media"]
            .as_str()
            .or_else(|| llm_response["rapid_response_talking_points"].as_str())
            .unwrap_or(&revised_narrative[..revised_narrative.len().min(280)])
            .to_string();

        let press_statement = llm_response["press_statement"]
            .as_str()
            .or_else(|| llm_response["point_by_point_rebuttal"].as_str())
            .unwrap_or(&revised_narrative)
            .to_string();

        let internal_memo = llm_response["internal_memo"]
            .as_str()
            .or_else(|| llm_response["stakeholder_engagement_plan"].as_str())
            .unwrap_or("Internal briefing: See full revised narrative for details.")
            .to_string();

        let drafts = ResponseDrafts {
            social_media,
            press_statement,
            internal_memo,
            qa_brief,
        };

        let backfire_potential = llm_response["potential_backfire"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_else(|| vec!["May not fully address all concerns".to_string()]);

        let media_amplification_risk =
            llm_response["media_amplification_risk"]
                .as_f64()
                .unwrap_or(match strategy_type {
                    StrategyType::Reframe => 0.4,
                    StrategyType::CounterArgue => 0.7,
                    StrategyType::Bridge => 0.3,
                });

        let risk_assessment = RiskAssessment {
            potential_backfire: backfire_potential,
            stakeholder_reaction: StakeholderReaction {
                supporters: "Will appreciate thoughtful approach".to_string(),
                skeptics: "May remain unconvinced".to_string(),
                media: "Moderate coverage expected".to_string(),
            },
            media_amplification_risk: media_amplification_risk.max(0.0).min(1.0),
        };

        let vulnerabilities_count = match strategy_type {
            StrategyType::Reframe => 3,
            StrategyType::CounterArgue => 5,
            StrategyType::Bridge => 4,
        };

        let vulnerabilities_addressed: Vec<Uuid> = omission_catalog
            .prioritized
            .iter()
            .take(vulnerabilities_count)
            .copied()
            .collect();

        let strategic_rationale = llm_response["strategic_rationale"]
            .as_str()
            .unwrap_or(&format!(
                "{:?} strategy addresses {} vulnerabilities",
                strategy_type, vulnerabilities_count
            ))
            .to_string();

        Ok(ResponseStrategy {
            strategy_type,
            strategic_rationale,
            drafts,
            risk_assessment,
            tone_guidance,
            vulnerabilities_addressed,
        })
    }

    fn parse_response_strategy_from_markdown(
        &self,
        content: &str,
        strategy_type: StrategyType,
        tone_guidance: ToneGuidance,
        omission_catalog: &OmissionCatalog,
    ) -> Result<ResponseStrategy> {
        info!("Parsing response strategy from markdown format");

        let mut strategic_rationale = String::new();
        let mut social_media = String::new();
        let mut press_statement = String::new();
        let mut internal_memo = String::new();
        let mut qa_brief = Vec::new();
        let mut potential_backfire = Vec::new();

        let mut current_section = String::new();
        let mut current_content = String::new();

        // Extract intro/rationale from first paragraph
        let first_para_end = content.find("\n\n").unwrap_or(content.len().min(500));
        strategic_rationale = content[..first_para_end].to_string();

        for line in content.lines() {
            let line = line.trim();
            let lower = line.to_lowercase();

            // Detect section headers
            if line.starts_with('#') || line.starts_with("**") && line.ends_with("**") ||
               (line.chars().next().map(|c| c.is_numeric()).unwrap_or(false) && line.contains('.')) {

                // Save previous section content
                if !current_section.is_empty() && !current_content.is_empty() {
                    self.assign_section_content(&current_section, &current_content,
                        &mut social_media, &mut press_statement, &mut internal_memo,
                        &mut qa_brief, &mut potential_backfire);
                }

                // Start new section
                current_section = line.trim_matches(|c: char| c == '#' || c == '*' || c.is_numeric() || c == '.' || c == ' ').to_lowercase();
                current_content = String::new();
            }
            // Extract Q&A pairs
            else if line.starts_with("Q:") || lower.starts_with("question:") {
                let question = line.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string();
                // Next line should be answer
                current_content.push_str(&format!("Q: {}\n", question));
            }
            else if line.starts_with("A:") || lower.starts_with("answer:") {
                let answer = line.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string();
                current_content.push_str(&format!("A: {}\n", answer));
                // Try to extract as Q&A pair from recent content
                let recent_lines: Vec<&str> = current_content.lines().rev().take(2).collect();
                if recent_lines.len() >= 2 {
                    let last_two = format!("{}\n{}", recent_lines[1], recent_lines[0]);
                    if let Some(qa_text) = last_two.strip_prefix("Q: ") {
                        if let Some((q, a)) = qa_text.split_once("\nA: ") {
                            qa_brief.push(QAPair {
                                question: q.to_string(),
                                answer: a.to_string(),
                            });
                        }
                    }
                }
            }
            // Collect content for current section
            else if !line.is_empty() && !current_section.is_empty() {
                if !current_content.is_empty() {
                    current_content.push('\n');
                }
                current_content.push_str(line);
            }
        }

        // Save last section
        if !current_section.is_empty() && !current_content.is_empty() {
            self.assign_section_content(&current_section, &current_content,
                &mut social_media, &mut press_statement, &mut internal_memo,
                &mut qa_brief, &mut potential_backfire);
        }

        // Use defaults if sections weren't found
        if social_media.is_empty() {
            social_media = content[..content.len().min(280)].to_string();
        }
        if press_statement.is_empty() {
            press_statement = content[..content.len().min(1000)].to_string();
        }
        if internal_memo.is_empty() {
            internal_memo = "See full analysis for internal guidance".to_string();
        }
        if potential_backfire.is_empty() {
            potential_backfire.push("Markdown parsing fallback - manual review recommended".to_string());
        }

        let vulnerabilities_count = match strategy_type {
            StrategyType::Reframe => 3,
            StrategyType::CounterArgue => 5,
            StrategyType::Bridge => 4,
        };

        let media_risk = match strategy_type {
            StrategyType::Reframe => 0.4,
            StrategyType::CounterArgue => 0.7,
            StrategyType::Bridge => 0.3,
        };

        info!("Extracted {} sections from markdown, {} Q&A pairs",
              vec![&social_media, &press_statement, &internal_memo].iter().filter(|s| !s.is_empty()).count(),
              qa_brief.len());

        Ok(ResponseStrategy {
            strategy_type,
            strategic_rationale,
            drafts: ResponseDrafts {
                social_media,
                press_statement,
                internal_memo,
                qa_brief,
            },
            risk_assessment: RiskAssessment {
                potential_backfire,
                stakeholder_reaction: StakeholderReaction {
                    supporters: "Will appreciate thoughtful approach".to_string(),
                    skeptics: "May remain unconvinced".to_string(),
                    media: "Moderate coverage expected".to_string(),
                },
                media_amplification_risk: media_risk,
            },
            tone_guidance,
            vulnerabilities_addressed: omission_catalog.prioritized.iter().take(vulnerabilities_count).copied().collect(),
        })
    }

    fn assign_section_content(
        &self,
        section: &str,
        content: &str,
        social_media: &mut String,
        press_statement: &mut String,
        internal_memo: &mut String,
        qa_brief: &mut Vec<QAPair>,
        potential_backfire: &mut Vec<String>,
    ) {
        let section_lower = section.to_lowercase();

        if section_lower.contains("social") || section_lower.contains("talking point") || section_lower.contains("rapid response") {
            *social_media = content.to_string();
        } else if section_lower.contains("rebuttal") || section_lower.contains("press") || section_lower.contains("statement") {
            *press_statement = content.to_string();
        } else if section_lower.contains("memo") || section_lower.contains("stakeholder") || section_lower.contains("engagement") || section_lower.contains("bridge") || section_lower.contains("letter") {
            *internal_memo = content.to_string();
        } else if section_lower.contains("q&a") || section_lower.contains("question") {
            // Q&A pairs should be extracted during line-by-line parsing
        } else if section_lower.contains("risk") || section_lower.contains("backfire") || section_lower.contains("caution") {
            for line in content.lines() {
                let line = line.trim().trim_start_matches(|c: char| c == '-' || c == '*' || c == ' ');
                if !line.is_empty() && line.len() < 500 {
                    potential_backfire.push(line.to_string());
                }
            }
        }
    }

    async fn generate_reframe_strategy_mock(
        &self,
        _narrative: &NarrativeInput,
        cumulative_analysis: &CumulativeAnalysis,
        omission_catalog: &OmissionCatalog,
    ) -> Result<ResponseStrategy> {
        debug!("Generating Reframe strategy (mock)");

        let vulnerabilities_addressed: Vec<Uuid> = omission_catalog
            .prioritized
            .iter()
            .take(3)
            .copied()
            .collect();

        let drafts = ResponseDrafts {
            social_media: "We hear your concerns. Here's the full context we should have shared from the start. [Thread 1/5]".to_string(),
            press_statement: "Today we're providing additional context that clarifies our previous announcement and addresses questions from stakeholders.".to_string(),
            internal_memo: "Team: We're shifting our external messaging to provide fuller context. Here's what you need to know...".to_string(),
            qa_brief: vec![
                QAPair {
                    question: "Why didn't you mention this before?".to_string(),
                    answer: "We focused on the headline outcome. In hindsight, we should have provided this context upfront.".to_string(),
                },
                QAPair {
                    question: "What about [omitted stakeholder group]?".to_string(),
                    answer: "You're right to raise this. Here's how this decision affects them and what we're doing to support them.".to_string(),
                },
            ],
        };

        let risk_assessment = RiskAssessment {
            potential_backfire: vec![
                "May be seen as damage control rather than genuine transparency".to_string(),
                "Timing (after criticism) undermines credibility of reframe".to_string(),
            ],
            stakeholder_reaction: StakeholderReaction {
                supporters: "Will appreciate additional context, may forgive omission".to_string(),
                skeptics: "Will view as spin, question why context wasn't provided initially"
                    .to_string(),
                media: "Moderate coverage, 'company provides additional context' framing"
                    .to_string(),
            },
            media_amplification_risk: 0.4,
        };

        Ok(ResponseStrategy {
            strategy_type: StrategyType::Reframe,
            strategic_rationale: format!(
                "Reframing addresses {} critical omissions by providing context that was missing. Risk level: {:?}",
                vulnerabilities_addressed.len(),
                cumulative_analysis.strategic_risk_level
            ),
            drafts,
            risk_assessment,
            tone_guidance: ToneGuidance::Empathetic,
            vulnerabilities_addressed,
        })
    }

    async fn generate_counter_argue_strategy_mock(
        &self,
        _narrative: &NarrativeInput,
        cumulative_analysis: &CumulativeAnalysis,
        omission_catalog: &OmissionCatalog,
    ) -> Result<ResponseStrategy> {
        debug!("Generating Counter-Argue strategy (mock)");

        let vulnerabilities_addressed: Vec<Uuid> = omission_catalog
            .prioritized
            .iter()
            .take(5)
            .copied()
            .collect();

        let drafts = ResponseDrafts {
            social_media: "Let's address the misleading claims circulating about our announcement. Here are the facts: [Thread]".to_string(),
            press_statement: "Recent criticism mischaracterizes our decision. We want to set the record straight with evidence.".to_string(),
            internal_memo: "Team: We're pushing back on inaccurate critiques. Here's our evidence-based counter-narrative...".to_string(),
            qa_brief: vec![
                QAPair {
                    question: "Critics say you're hiding negative impacts. Are you?".to_string(),
                    answer: "No. Here's the data showing actual impacts, which don't match critics' claims.".to_string(),
                },
                QAPair {
                    question: "Why should we trust your version over critics?".to_string(),
                    answer: "We have receipts. Here's our evidence trail showing decision rationale and stakeholder consultation.".to_string(),
                },
            ],
        };

        let risk_assessment = RiskAssessment {
            potential_backfire: vec![
                "Adversarial tone may escalate conflict rather than resolve it".to_string(),
                "If evidence is incomplete, counter-arguing amplifies vulnerability".to_string(),
                "May alienate persuadable middle ground audience".to_string(),
            ],
            stakeholder_reaction: StakeholderReaction {
                supporters: "Energized by strong defense, will amplify counter-arguments"
                    .to_string(),
                skeptics: "Entrenched in opposition, will find flaws in evidence presented"
                    .to_string(),
                media: "High coverage, 'company fights back' framing, polarizing".to_string(),
            },
            media_amplification_risk: 0.7,
        };

        Ok(ResponseStrategy {
            strategy_type: StrategyType::CounterArgue,
            strategic_rationale: format!(
                "Counter-arguing directly challenges {} exploitation points with evidence. High-risk, high-reward. Risk level: {:?}",
                vulnerabilities_addressed.len(),
                cumulative_analysis.strategic_risk_level
            ),
            drafts,
            risk_assessment,
            tone_guidance: ToneGuidance::Assertive,
            vulnerabilities_addressed,
        })
    }

    async fn generate_bridge_strategy_mock(
        &self,
        _narrative: &NarrativeInput,
        cumulative_analysis: &CumulativeAnalysis,
        omission_catalog: &OmissionCatalog,
    ) -> Result<ResponseStrategy> {
        debug!("Generating Bridge strategy (mock)");

        let vulnerabilities_addressed: Vec<Uuid> = omission_catalog
            .prioritized
            .iter()
            .take(4)
            .copied()
            .collect();

        let drafts = ResponseDrafts {
            social_media: "We've heard your concerns and we want to have a real conversation. Here's what we're learning from your feedback...".to_string(),
            press_statement: "We're committed to dialogue with all stakeholders. Today we're announcing a listening tour to understand impacts and co-create solutions.".to_string(),
            internal_memo: "Team: We're shifting to dialogic engagement. This means listening first, defending second. Here's how...".to_string(),
            qa_brief: vec![
                QAPair {
                    question: "Is this just PR or are you actually listening?".to_string(),
                    answer: "We're creating specific mechanisms for two-way dialogue: town halls, advisory groups, open comment periods. We'll demonstrate listening through action.".to_string(),
                },
                QAPair {
                    question: "Will stakeholder input actually change your decisions?".to_string(),
                    answer: "Yes. We're identifying areas where we have flexibility and where we don't. Where we can't change course, we'll explain constraints transparently.".to_string(),
                },
            ],
        };

        let risk_assessment = RiskAssessment {
            potential_backfire: vec![
                "May be perceived as weak or indecisive retreat".to_string(),
                "Listening tour may surface additional grievances".to_string(),
                "Time-intensive process may not satisfy urgent demands".to_string(),
            ],
            stakeholder_reaction: StakeholderReaction {
                supporters: "May worry organization is capitulating to critics".to_string(),
                skeptics: "Cautiously optimistic if process is genuine, cynical if performative"
                    .to_string(),
                media: "Positive framing, 'company engages stakeholders', reduces conflict"
                    .to_string(),
            },
            media_amplification_risk: 0.3,
        };

        Ok(ResponseStrategy {
            strategy_type: StrategyType::Bridge,
            strategic_rationale: format!(
                "Bridging engages {} omissions through dialogic process. Long-term relationship building. Risk level: {:?}",
                vulnerabilities_addressed.len(),
                cumulative_analysis.strategic_risk_level
            ),
            drafts,
            risk_assessment,
            tone_guidance: ToneGuidance::Collaborative,
            vulnerabilities_addressed,
        })
    }
}

impl Default for TwoPassDebateWorkflow {
    fn default() -> Self {
        Self::new()
    }
}
