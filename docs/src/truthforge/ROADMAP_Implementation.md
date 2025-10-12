# Implementation Roadmap: TruthForge Two-Pass Debate Arena

**Version**: 1.0
**Date**: 2025-10-07
**Status**: Draft
**Owner**: Zestic AI / K-Partners
**Timeline**: 8 weeks (2025-10-14 to 2025-12-06)
**Related Documents**:
- [PRD_TwoPassDebateArena.md](./PRD_TwoPassDebateArena.md)
- [SPEC_TerraphimIntegration.md](./SPEC_TerraphimIntegration.md)
- [REQUIREMENTS_AgentRoles.md](./REQUIREMENTS_AgentRoles.md)

---

## Executive Summary

This roadmap outlines an 8-week implementation plan to deliver the TruthForge Two-Pass Debate Arena feature. The implementation follows an iterative, test-driven approach with four 2-week phases:

1. **Phase 1 (Weeks 1-2)**: Foundation - Repository setup, taxonomy migration, core agents
2. **Phase 2 (Weeks 3-4)**: Two-Pass Workflow - Debate orchestration and omission detection
3. **Phase 3 (Weeks 5-6)**: Infrastructure - WebSocket server, Redis persistence, UI
4. **Phase 4 (Weeks 7-8)**: Testing, Polish, Deployment - Performance optimization, security, launch

### Key Milestones

- **Week 2**: Pass 1 analysis agents functional (Bias, Narrative, Taxonomy)
- **Week 4**: Two-pass debate workflow complete end-to-end
- **Week 6**: Real-time WebSocket UI and Redis persistence operational
- **Week 8**: Production deployment with <90s total latency, <$5 per analysis

### Success Criteria

- All 13 agent roles implemented and tested
- Two-pass workflow achieves ≥85% omission detection accuracy
- Total workflow completes in <90 seconds (OpenRouter)
- Cost per analysis <$5 (OpenRouter Claude mix)
- UI supports real-time progress tracking via WebSocket
- 100% test coverage for critical paths
- Security review passed (prompt injection prevention, PII redaction)

---

## Phase 1: Foundation (Weeks 1-2)

**Dates**: 2025-10-14 to 2025-10-25
**Goal**: Establish core infrastructure and implement first 3 analysis agents

### Week 1: Repository Setup & Taxonomy Migration

#### Day 1-2: Private Repository Initialization

**Tasks**:
- Create private `terraphim_truthforge` repository
- Set up Cargo workspace with 2 crates:
  - `terraphim_truthforge` (core library)
  - `terraphim_truthforge_server` (WebSocket server)
- Configure GitHub Actions CI/CD
- Add terraphim-ai as git submodule or dependency
- Set up development environment documentation

**Deliverables**:
```
terraphim_truthforge/
├── Cargo.toml (workspace)
├── .github/workflows/ci.yml
├── crates/
│   ├── terraphim_truthforge/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── terraphim_truthforge_server/
│       ├── Cargo.toml
│       └── src/main.rs
├── taxonomy/
│   └── truthforge_rolegraph.json (placeholder)
└── README.md
```

**Dependencies** (`Cargo.toml`):
```toml
[workspace]
members = ["crates/*"]

[workspace.dependencies]
terraphim-multi-agent = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
terraphim-types = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
terraphim-rolegraph = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
rig-core = "0.1"
axum = { version = "0.7", features = ["ws", "macros"] }
tokio = { version = "1.35", features = ["full"] }
tokio-tungstenite = "0.21"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
redis = { version = "0.24", features = ["tokio-comp", "cluster", "json"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

**Testing**:
- CI runs `cargo build --all`
- CI runs `cargo test --all`
- CI runs `cargo clippy -- -D warnings`
- CI runs `cargo fmt -- --check`

**Owner**: DevOps Lead
**Estimated Effort**: 16 hours

---

#### Day 3-4: Taxonomy Migration to RoleGraph

**Tasks**:
- Read `trueforge_taxonomy.json` (3 functions, 15 subfunctions)
- Design RoleGraph node/edge structure
- Implement migration function `migrate_truthforge_taxonomy()`
- Create automata from taxonomy concepts
- Validate graph connectivity and completeness

**Implementation** (`crates/terraphim_truthforge/src/taxonomy.rs`):
```rust
use terraphim_rolegraph::RoleGraph;
use terraphim_types::NormalizedTerm;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TaxonomyFunction {
    id: String,
    name: String,
    description: String,
    subfunctions: Vec<SubFunction>,
    #[serde(default)]
    classification: Option<ScctClassification>,
}

#[derive(Debug, Deserialize)]
struct SubFunction {
    name: String,
    outputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ScctClassification {
    #[serde(rename = "issueTypes")]
    issue_types: Vec<String>,
    #[serde(rename = "responsibilityAttribution")]
    responsibility_attribution: Vec<String>,
}

pub async fn migrate_truthforge_taxonomy(
    json_path: impl AsRef<Path>
) -> anyhow::Result<RoleGraph> {
    let json_str = tokio::fs::read_to_string(json_path).await?;
    let functions: Vec<TaxonomyFunction> = serde_json::from_str(&json_str)?;

    let mut graph = RoleGraph::new();

    for func in functions {
        // Create function node
        let func_node = NormalizedTerm {
            id: graph.next_id(),
            value: func.id.clone().into(),
            url: Some(format!("truthforge://function/{}", func.id)),
        };
        graph.add_node(func_node.clone());

        // Create subfunction nodes
        for subf in func.subfunctions {
            let subf_node = NormalizedTerm {
                id: graph.next_id(),
                value: format!("{}.{}", func.id, subf.name).into(),
                url: Some(format!("truthforge://function/{}/{}", func.id, subf.name)),
            };
            graph.add_node(subf_node.clone());

            // Create edge: function -> subfunction
            graph.add_edge(&func_node, &subf_node, 1.0)?;

            // Create output nodes
            for output in subf.outputs {
                let output_node = NormalizedTerm {
                    id: graph.next_id(),
                    value: output.clone().into(),
                    url: Some(format!("truthforge://output/{}", output)),
                };
                graph.add_node(output_node.clone());

                // Create edge: subfunction -> output
                graph.add_edge(&subf_node, &output_node, 1.0)?;
            }
        }

        // Add SCCT classification nodes if present
        if let Some(classification) = func.classification {
            for attr in classification.responsibility_attribution {
                let scct_node = NormalizedTerm {
                    id: graph.next_id(),
                    value: format!("scct.{}", attr).into(),
                    url: Some(format!("truthforge://scct/{}", attr)),
                };
                graph.add_node(scct_node.clone());
                graph.add_edge(&func_node, &scct_node, 1.0)?;
            }
        }
    }

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_taxonomy_migration() {
        let graph = migrate_truthforge_taxonomy("../../assets/trueforge_taxonomy.json")
            .await
            .expect("Migration failed");

        // Verify 3 core functions
        assert!(graph.has_node("relationship_management"));
        assert!(graph.has_node("issue_crisis_management"));
        assert!(graph.has_node("strategic_management_function"));

        // Verify subfunctions exist
        assert!(graph.has_node("issue_crisis_management.risk_assessment"));
        assert!(graph.has_node("relationship_management.stakeholder_mapping"));

        // Verify SCCT nodes
        assert!(graph.has_node("scct.victim"));
        assert!(graph.has_node("scct.accidental"));
        assert!(graph.has_node("scct.preventable"));

        // Verify edges exist
        let risk_node = graph.get_node("issue_crisis_management").unwrap();
        let subf_node = graph.get_node("issue_crisis_management.risk_assessment").unwrap();
        assert!(graph.has_edge(&risk_node, &subf_node));
    }

    #[tokio::test]
    async fn test_graph_connectivity() {
        let graph = migrate_truthforge_taxonomy("../../assets/trueforge_taxonomy.json")
            .await
            .expect("Migration failed");

        // All nodes should be reachable from at least one function
        let total_nodes = graph.node_count();
        let reachable = graph.nodes_reachable_from_functions();
        assert_eq!(total_nodes, reachable, "Some nodes are unreachable");
    }
}
```

**Deliverables**:
- `taxonomy/truthforge_rolegraph.json` (generated from migration)
- Migration function with 100% test coverage
- Graph contains ~50 nodes, ~75 edges
- Documentation on taxonomy structure

**Testing**:
- Unit tests verify all 3 functions migrated
- Integration test verifies graph connectivity
- Benchmark migration time (<100ms)

**Owner**: Backend Engineer
**Estimated Effort**: 16 hours

---

#### Day 5: Core Data Structures

**Tasks**:
- Implement core types in `crates/terraphim_truthforge/src/types.rs`
- Define all structs from SPEC (Omission, DebateResult, etc.)
- Add serde derives for JSON serialization
- Implement validation logic

**Implementation**:
```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OmissionCategory {
    FactualGap,
    MissingVoice,
    UnstatedAssumption,
    ContextGap,
    UnaddressedCounterargument,
    ProceduralOmission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Omission {
    pub id: Uuid,
    pub category: OmissionCategory,
    pub description: String,
    pub severity: f64,           // 0.0-1.0
    pub exploitability: f64,     // 0.0-1.0
    pub composite_risk: f64,     // severity × exploitability
    pub text_reference: String,
    pub suggested_addition: String,
    pub confidence: f64,         // 0.0-1.0
    pub detected_at: DateTime<Utc>,
}

impl Omission {
    pub fn new(
        category: OmissionCategory,
        description: String,
        severity: f64,
        exploitability: f64,
        text_reference: String,
        suggested_addition: String,
        confidence: f64,
    ) -> anyhow::Result<Self> {
        // Validation
        if !(0.0..=1.0).contains(&severity) {
            anyhow::bail!("Severity must be between 0.0 and 1.0");
        }
        if !(0.0..=1.0).contains(&exploitability) {
            anyhow::bail!("Exploitability must be between 0.0 and 1.0");
        }
        if !(0.0..=1.0).contains(&confidence) {
            anyhow::bail!("Confidence must be between 0.0 and 1.0");
        }

        Ok(Self {
            id: Uuid::new_v4(),
            category,
            description,
            severity,
            exploitability,
            composite_risk: severity * exploitability,
            text_reference,
            suggested_addition,
            confidence,
            detected_at: Utc::now(),
        })
    }

    pub fn is_high_risk(&self) -> bool {
        self.composite_risk >= 0.6
    }
}

// Additional types: DebateResult, Pass1Analysis, Pass2Analysis, etc.
// (Full implementation in SPEC document)
```

**Deliverables**:
- All core types implemented with validation
- Unit tests for validation logic
- Serde serialization/deserialization tests

**Owner**: Backend Engineer
**Estimated Effort**: 8 hours

---

### Week 2: First 3 Analysis Agents

#### Day 6-8: BiasDetectorAgent Implementation

**Tasks**:
- Create agent role configuration JSON
- Implement agent initialization from config
- Integrate with OpenRouter (Claude 3.5 Sonnet)
- Implement mock LLM for testing (Ollama fallback)
- Write comprehensive unit tests

**Implementation** (`crates/terraphim_truthforge/src/agents/bias_detector.rs`):
```rust
use terraphim_multi_agent::TerraphimAgent;
use rig_core::providers::openrouter;
use serde_json::json;

pub struct BiasDetectorAgent {
    agent: TerraphimAgent,
}

impl BiasDetectorAgent {
    pub async fn new(config_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let agent = TerraphimAgent::from_config_file(config_path).await?;
        Ok(Self { agent })
    }

    pub async fn analyze(&self, text: &str) -> anyhow::Result<BiasAnalysis> {
        let prompt = format!(
            "Analyze the following text for cognitive biases, logical fallacies, and rhetorical tactics:\n\n{}",
            text
        );

        let response = self.agent.generate_response(&prompt).await?;
        let analysis: BiasAnalysis = serde_json::from_str(&response)?;

        Ok(analysis)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BiasAnalysis {
    pub biases: Vec<BiasInstance>,
    pub fallacies: Vec<FallacyInstance>,
    pub tactics: Vec<TacticInstance>,
    pub overall_assessment: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bias_detection_confirmation_bias() {
        let agent = BiasDetectorAgent::new("configs/bias_detector.json")
            .await
            .expect("Agent creation failed");

        let text = "We've carefully selected the most reliable research to support our position.";
        let analysis = agent.analyze(text).await.expect("Analysis failed");

        assert!(!analysis.biases.is_empty(), "Should detect confirmation bias");
        assert_eq!(analysis.biases[0].bias_type, "confirmation_bias");
    }

    #[tokio::test]
    async fn test_bias_detection_with_ollama() {
        // Test with Ollama gemma3:270m for fast local testing
        let agent = BiasDetectorAgent::new("configs/bias_detector_ollama.json")
            .await
            .expect("Agent creation failed");

        let text = "My opponent's criticism is invalid because they have financial ties to competitors.";
        let analysis = agent.analyze(text).await.expect("Analysis failed");

        assert!(!analysis.fallacies.is_empty(), "Should detect ad hominem");
    }
}
```

**Agent Configuration** (`taxonomy/roles/bias_detector.json`):
```json
{
  "name": "Bias Detector",
  "shortname": "bias_detector",
  "relevance_function": "BM25Plus",
  "extra": {
    "llm_provider": "openrouter",
    "llm_model": "anthropic/claude-3.5-sonnet:beta",
    "openrouter_api_key_env": "OPENROUTER_API_KEY",
    "system_prompt": "[FULL PROMPT FROM REQUIREMENTS_AgentRoles.md]",
    "agent_type": "analyzer",
    "temperature": 0.3,
    "max_tokens": 2000
  }
}
```

**Testing**:
- 3 unit tests per test scenario from REQUIREMENTS (9 tests total)
- OpenRouter integration test (gated by env var)
- Ollama integration test (fast, always runs)
- Latency test (<5s with OpenRouter, <2s with Ollama)

**Owner**: AI Engineer
**Estimated Effort**: 24 hours

---

#### Day 9-10: NarrativeMapperAgent & TaxonomyLinkerAgent

**Tasks**:
- Implement NarrativeMapperAgent (similar structure to BiasDetectorAgent)
- Implement TaxonomyLinkerAgent with RoleGraph integration
- Write comprehensive tests for both agents
- Integration test: Bias → Narrative → Taxonomy pipeline

**Key Integration** (TaxonomyLinkerAgent uses RoleGraph):
```rust
impl TaxonomyLinkerAgent {
    pub async fn classify(
        &self,
        text: &str,
        narrative_map: &NarrativeMap,
        rolegraph: &RoleGraph,
    ) -> anyhow::Result<TaxonomyClassification> {
        // Use rolegraph to enhance classification with graph connectivity
        let relevant_nodes = rolegraph.find_relevant_nodes(text)?;

        let prompt = format!(
            "Classify the following text using the taxonomy.\n\nRelevant taxonomy nodes: {:?}\n\nText: {}",
            relevant_nodes, text
        );

        let response = self.agent.generate_response(&prompt).await?;
        let classification: TaxonomyClassification = serde_json::from_str(&response)?;

        Ok(classification)
    }
}
```

**Testing**:
- NarrativeMapperAgent: 3 test scenarios (9 tests)
- TaxonomyLinkerAgent: 3 test scenarios (9 tests)
- Integration test: Full Pass 1 analysis pipeline

**Owner**: AI Engineer
**Estimated Effort**: 16 hours

---

### Phase 1 Milestones

**Week 2 End Deliverables**:
- ✅ Private repository operational with CI/CD
- ✅ Taxonomy migrated to RoleGraph (50+ nodes, 75+ edges)
- ✅ Core data structures implemented and validated
- ✅ 3 analysis agents functional (Bias, Narrative, Taxonomy)
- ✅ 27+ unit tests passing
- ✅ Integration test: Bias → Narrative → Taxonomy pipeline
- ✅ Ollama testing infrastructure operational

**Success Metrics**:
- CI green on all commits
- Test coverage ≥80% on core modules
- Agent latency <5s (OpenRouter), <2s (Ollama)
- 100% of test scenarios from REQUIREMENTS passing

**Demo**: Run analysis pipeline on sample crisis communication text, show Bias/Narrative/Taxonomy outputs

---

## Phase 2: Two-Pass Workflow (Weeks 3-4)

**Dates**: 2025-10-28 to 2025-11-08
**Goal**: Implement complete two-pass debate workflow with omission detection

### Week 3: OmissionDetectorAgent & Pass 1 Debate

#### Day 11-13: OmissionDetectorAgent (Critical Innovation)

**Tasks**:
- Implement OmissionDetectorAgent with 6 omission categories
- Integrate with BiasAnalysis and NarrativeMap
- Implement severity/exploitability scoring
- Comprehensive testing on real crisis communications

**Implementation Highlight** (Composite Risk Calculation):
```rust
impl OmissionDetectorAgent {
    async fn detect_omissions(
        &self,
        text: &str,
        bias_analysis: &BiasAnalysis,
        narrative_map: &NarrativeMap,
    ) -> anyhow::Result<Vec<Omission>> {
        let prompt = self.build_detection_prompt(text, bias_analysis, narrative_map);
        let response = self.agent.generate_response(&prompt).await?;
        let raw_omissions: Vec<RawOmission> = serde_json::from_str(&response)?;

        // Convert to validated Omission structs
        let omissions = raw_omissions
            .into_iter()
            .map(|raw| Omission::new(
                raw.category,
                raw.description,
                raw.severity,
                raw.exploitability,
                raw.text_reference,
                raw.suggested_addition,
                raw.confidence,
            ))
            .collect::<Result<Vec<_>, _>>()?;

        // Sort by composite risk
        omissions.sort_by(|a, b| b.composite_risk.partial_cmp(&a.composite_risk).unwrap());

        Ok(omissions)
    }
}
```

**Testing**:
- 4 test scenarios per category (24 tests total)
- Severity calibration test (compare to expert scores)
- Composite risk ranking validation
- Integration with Bias + Narrative agents

**Owner**: AI Engineer
**Estimated Effort**: 24 hours

---

#### Day 14-15: Pass 1 Debate Agents

**Tasks**:
- Implement Pass1DebaterAgent (Pro and Con)
- Implement Pass1EvaluatorAgent
- Orchestrate Pass 1 workflow: Analysis → Debate → Evaluation
- Test end-to-end Pass 1 execution

**Workflow Orchestration** (`crates/terraphim_truthforge/src/workflows/pass1.rs`):
```rust
pub struct Pass1Workflow {
    bias_detector: BiasDetectorAgent,
    narrative_mapper: NarrativeMapperAgent,
    taxonomy_linker: TaxonomyLinkerAgent,
    omission_detector: OmissionDetectorAgent,
    debater_pro: Pass1DebaterAgent,
    debater_con: Pass1DebaterAgent,
    evaluator: Pass1EvaluatorAgent,
}

impl Pass1Workflow {
    pub async fn execute(&self, input_text: &str) -> anyhow::Result<Pass1Analysis> {
        // Sequential analysis
        let bias_analysis = self.bias_detector.analyze(input_text).await?;
        let narrative_map = self.narrative_mapper.analyze(input_text).await?;
        let taxonomy = self.taxonomy_linker.classify(input_text, &narrative_map).await?;
        let omissions = self.omission_detector.detect(input_text, &bias_analysis, &narrative_map).await?;

        // Parallel debate
        let (pro_debate, con_debate) = tokio::join!(
            self.debater_pro.argue(input_text, &bias_analysis, &narrative_map, &omissions),
            self.debater_con.argue(input_text, &bias_analysis, &narrative_map, &omissions)
        );

        let pro_debate = pro_debate?;
        let con_debate = con_debate?;

        // Evaluation
        let evaluation = self.evaluator.evaluate(&pro_debate, &con_debate, &omissions).await?;

        Ok(Pass1Analysis {
            input_text: input_text.to_string(),
            bias_analysis,
            narrative_map,
            taxonomy,
            omissions,
            pro_debate,
            con_debate,
            evaluation,
            timestamp: Utc::now(),
        })
    }
}
```

**Testing**:
- Pass1DebaterAgent: 2 agents × 3 test scenarios (6 tests)
- Pass1EvaluatorAgent: 3 test scenarios (3 tests)
- End-to-end Pass 1 workflow test
- Latency benchmark (<45s target)

**Owner**: AI Engineer
**Estimated Effort**: 24 hours

---

### Week 4: Pass 2 Exploitation & Response Generation

#### Day 16-17: Pass 2 Debate Agents

**Tasks**:
- Implement Pass2DebaterAgent (Exploitation and Defense)
- Implement CumulativeEvaluatorAgent with vulnerability tracking
- Test Pass 2 workflow integration with Pass 1 results

**Cumulative Tracking** (Key Innovation):
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct OmissionResolutionStatus {
    pub omission_id: Uuid,
    pub pass1_risk: f64,
    pub pass2_attack_strength: f64,
    pub pass2_defense_strength: f64,
    pub residual_risk: f64,
    pub status: ResolutionStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResolutionStatus {
    Resolved,      // Defense addressed omission successfully
    Mitigated,     // Partial defense, reduced risk
    Unresolved,    // No effective defense
    Escalated,     // Risk increased due to poor defense
}

impl CumulativeEvaluatorAgent {
    async fn track_omission_resolution(
        &self,
        omission: &Omission,
        exploitation: &ExploitationAttack,
        defense: &DefenseResponse,
    ) -> OmissionResolutionStatus {
        let pass1_risk = omission.composite_risk;

        // Score attack strength (0.0-1.0)
        let attack_strength = self.score_attack_effectiveness(exploitation);

        // Score defense strength (0.0-1.0)
        let defense_strength = self.score_defense_effectiveness(defense, omission);

        // Calculate residual risk
        let residual_risk = pass1_risk * attack_strength * (1.0 - defense_strength);

        // Determine status
        let status = if residual_risk < 0.2 {
            ResolutionStatus::Resolved
        } else if residual_risk < pass1_risk * 0.6 {
            ResolutionStatus::Mitigated
        } else if residual_risk < pass1_risk * 1.2 {
            ResolutionStatus::Unresolved
        } else {
            ResolutionStatus::Escalated
        };

        OmissionResolutionStatus {
            omission_id: omission.id,
            pass1_risk,
            pass2_attack_strength: attack_strength,
            pass2_defense_strength: defense_strength,
            residual_risk,
            status,
        }
    }
}
```

**Testing**:
- Pass2 Exploitation: 3 test scenarios
- Pass2 Defense: 3 test scenarios (with SCCT strategy variation)
- CumulativeEvaluator: Vulnerability tracking accuracy
- Integration: Pass 1 → Pass 2 → Cumulative Evaluation

**Owner**: AI Engineer
**Estimated Effort**: 24 hours

---

#### Day 18-20: Response Generation Agents

**Tasks**:
- Implement ReframeAgent, CounterArgueAgent, BridgeAgent
- Orchestrate parallel response generation
- End-to-end two-pass workflow testing
- Performance optimization

**Parallel Response Generation**:
```rust
pub async fn generate_responses(
    &self,
    cumulative_eval: &CumulativeEvaluation,
) -> anyhow::Result<ResponseSet> {
    let (reframe, counterargue, bridge) = tokio::join!(
        self.reframe_agent.generate(cumulative_eval),
        self.counterargue_agent.generate(cumulative_eval),
        self.bridge_agent.generate(cumulative_eval)
    );

    Ok(ResponseSet {
        reframe: reframe?,
        counterargue: counterargue?,
        bridge: bridge?,
        generated_at: Utc::now(),
    })
}
```

**Testing**:
- Response agent unit tests: 3 agents × 2 scenarios (6 tests)
- Parallel execution test
- **End-to-end workflow test**: Input text → Pass 1 → Pass 2 → Responses
- Cost tracking test (ensure <$5 per analysis)

**Owner**: AI Engineer
**Estimated Effort**: 24 hours

---

### Phase 2 Milestones

**Week 4 End Deliverables**:
- ✅ All 13 agent roles implemented
- ✅ Complete two-pass workflow functional
- ✅ OmissionDetectorAgent achieving ≥85% validity
- ✅ Pass 1 workflow completes in <45s (OpenRouter)
- ✅ Pass 2 workflow completes in <30s (OpenRouter)
- ✅ Response generation completes in <15s (OpenRouter)
- ✅ Total workflow <90s end-to-end
- ✅ Cost per analysis <$5 (verified with actual OpenRouter billing)
- ✅ 50+ unit tests, 10+ integration tests

**Success Metrics**:
- Omission detection accuracy ≥85% (expert validation)
- Pass 2 exploitation targets ≥80% of high-risk omissions
- Cumulative evaluation identifies ≥90% of residual risks
- All 13 agents have ≥80% test coverage

**Demo**: Full two-pass analysis on real crisis communication, show omission detection → exploitation → defense → responses with timing breakdown

---

## Phase 3: Infrastructure (Weeks 5-6)

**Dates**: 2025-10-11 to 2025-10-22
**Goal**: Build WebSocket server, Redis persistence, and real-time UI

### Week 5: WebSocket Server & Redis Persistence

#### Day 21-23: WebSocket Server Implementation

**Tasks**:
- Implement WebSocket server with Axum
- Define WebSocket message protocol (6 message types)
- Implement agent progress streaming
- Handle concurrent client connections

**Implementation** (`crates/terraphim_truthforge_server/src/websocket.rs`):
```rust
use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use tokio::sync::broadcast;

pub struct WebSocketState {
    progress_tx: broadcast::Sender<ProgressMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsMessage {
    AgentProgress {
        session_id: Uuid,
        agent: String,
        status: AgentStatus,
        progress: f32,
        message: String,
    },
    AgentResult {
        session_id: Uuid,
        agent: String,
        result: serde_json::Value,
    },
    PhaseComplete {
        session_id: Uuid,
        phase: String,
        results: serde_json::Value,
    },
    AnalysisComplete {
        session_id: Uuid,
        final_results: FinalResults,
    },
    Error {
        session_id: Uuid,
        error: String,
    },
    Pong,
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebSocketState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<WebSocketState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut progress_rx = state.progress_tx.subscribe();

    // Send progress updates to client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = progress_rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Receive pings from client
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if msg.is_ping() {
                // Handle ping
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
```

**Testing**:
- WebSocket connection test
- Message serialization/deserialization
- Concurrent client test (10 simultaneous connections)
- Progress streaming test with mock workflow

**Owner**: Backend Engineer
**Estimated Effort**: 24 hours

---

#### Day 24-25: Redis Persistence Layer

**Tasks**:
- Implement Redis client with connection pooling
- Create persistence layer for sessions, results, progress
- Implement TTL management (sessions: 24h, results: 7d, vault: 90d)
- Write integration tests with Redis

**Implementation** (`crates/terraphim_truthforge/src/persistence.rs`):
```rust
use redis::{Client, AsyncCommands, JsonAsyncCommands};
use serde::{Serialize, de::DeserializeOwned};

pub struct RedisPersistence {
    client: Client,
}

impl RedisPersistence {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = Client::open(redis_url)?;
        Ok(Self { client })
    }

    pub async fn save_session(
        &self,
        session_id: Uuid,
        data: &SessionData,
    ) -> anyhow::Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("session:{}", session_id);

        conn.json_set(&key, "$", data).await?;
        conn.expire(&key, 86400).await?; // 24 hours

        Ok(())
    }

    pub async fn get_session(
        &self,
        session_id: Uuid,
    ) -> anyhow::Result<Option<SessionData>> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("session:{}", session_id);

        let data: Option<SessionData> = conn.json_get(&key, "$").await?;
        Ok(data)
    }

    pub async fn save_analysis_result(
        &self,
        analysis_id: Uuid,
        result: &FinalResults,
    ) -> anyhow::Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("result:{}", analysis_id);

        conn.json_set(&key, "$", result).await?;
        conn.expire(&key, 604800).await?; // 7 days

        Ok(())
    }

    pub async fn save_to_learning_vault(
        &self,
        crisis_type: &str,
        case_study: &CaseStudy,
    ) -> anyhow::Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("vault:{}:{}", crisis_type, Uuid::new_v4());

        conn.json_set(&key, "$", case_study).await?;
        conn.expire(&key, 7776000).await?; // 90 days

        Ok(())
    }
}
```

**Testing**:
- Session save/retrieve test
- Result persistence test with TTL
- Learning vault test
- Redis connection failure handling

**Owner**: Backend Engineer
**Estimated Effort**: 16 hours

---

### Week 6: UI Implementation

#### Day 26-28: Frontend UI with Agent-Workflows Pattern

**Tasks**:
- Create HTML/JS UI based on agent-workflows examples
- Implement WebSocket client connection
- Build real-time progress visualization
- Create results display with omission highlighting

**Implementation** (`ui/index.html` - key sections):
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>TruthForge - Two-Pass Debate Arena</title>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.5.1/css/all.min.css" />
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <div class="container">
        <h1><i class="fas fa-search"></i> TruthForge - Two-Pass Debate Arena</h1>

        <div class="input-section">
            <i class="fas fa-file-alt"></i>
            <textarea id="input-text" placeholder="Paste your crisis communication text here..."></textarea>
            <button id="analyze-btn" onclick="startAnalysis()">
                <i class="fas fa-play-circle"></i> Analyze
            </button>
        </div>

        <div id="progress-container" style="display: none;">
            <h2><i class="fas fa-tasks"></i> Analysis Progress</h2>
            <div class="phase-tracker">
                <div class="phase" id="phase-1">
                    <h3><i class="fas fa-search"></i> Phase 1: Initial Analysis</h3>
                    <div class="agent-progress" id="bias-detector"></div>
                    <div class="agent-progress" id="narrative-mapper"></div>
                    <div class="agent-progress" id="taxonomy-linker"></div>
                    <div class="agent-progress" id="omission-detector"></div>
                </div>
                <div class="phase" id="phase-2">
                    <h3><i class="fas fa-balance-scale"></i> Phase 2: Debate</h3>
                    <div class="agent-progress" id="debater-pro"></div>
                    <div class="agent-progress" id="debater-con"></div>
                    <div class="agent-progress" id="evaluator"></div>
                </div>
                <div class="phase" id="phase-3">
                    <h3><i class="fas fa-crosshairs"></i> Phase 3: Exploitation</h3>
                    <div class="agent-progress" id="exploitation"></div>
                    <div class="agent-progress" id="defense"></div>
                    <div class="agent-progress" id="cumulative-eval"></div>
                </div>
                <div class="phase" id="phase-4">
                    <h3><i class="fas fa-lightbulb"></i> Phase 4: Response Generation</h3>
                    <div class="agent-progress" id="reframe"></div>
                    <div class="agent-progress" id="counterargue"></div>
                    <div class="agent-progress" id="bridge"></div>
                </div>
            </div>
        </div>

        <div id="results-container" style="display: none;">
            <h2><i class="fas fa-chart-line"></i> Analysis Results</h2>

            <div class="results-section">
                <h3><i class="fas fa-eye-slash"></i> Detected Omissions</h3>
                <div id="omissions-list"></div>
            </div>

            <div class="results-section">
                <h3><i class="fas fa-fire"></i> Pass 2 Vulnerability Assessment</h3>
                <div id="vulnerability-chart"></div>
            </div>

            <div class="results-section">
                <h3><i class="fas fa-reply-all"></i> Recommended Responses</h3>
                <div class="response-tabs">
                    <button class="tab active" onclick="showResponse('reframe')">
                        <i class="fas fa-sync-alt"></i> Reframe
                    </button>
                    <button class="tab" onclick="showResponse('counterargue')">
                        <i class="fas fa-gavel"></i> Counter-Argue
                    </button>
                    <button class="tab" onclick="showResponse('bridge')">
                        <i class="fas fa-handshake"></i> Bridge
                    </button>
                </div>
                <div id="response-content"></div>
            </div>
        </div>
    </div>

    <script src="app.js"></script>
</body>
</html>
```

**WebSocket Client** (`ui/app.js`):
```javascript
let ws;
let sessionId;

function startAnalysis() {
    const inputText = document.getElementById('input-text').value;
    sessionId = generateUUID();

    // Show progress
    document.getElementById('progress-container').style.display = 'block';

    // Connect to WebSocket
    ws = new WebSocket('ws://localhost:3000/ws');

    ws.onopen = () => {
        // Send analysis request via REST API
        fetch('/api/v1/analysis', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                session_id: sessionId,
                input_text: inputText,
            })
        });
    };

    ws.onmessage = (event) => {
        const msg = JSON.parse(event.data);
        handleWebSocketMessage(msg);
    };
}

function handleWebSocketMessage(msg) {
    switch (msg.type) {
        case 'AgentProgress':
            updateAgentProgress(msg.agent, msg.progress, msg.message);
            break;
        case 'PhaseComplete':
            markPhaseComplete(msg.phase);
            break;
        case 'AnalysisComplete':
            displayResults(msg.final_results);
            break;
        case 'Error':
            displayError(msg.error);
            break;
    }
}

function updateAgentProgress(agent, progress, message) {
    const elem = document.getElementById(agent);
    elem.innerHTML = `
        <div class="agent-name">${formatAgentName(agent)}</div>
        <div class="progress-bar">
            <div class="progress-fill" style="width: ${progress * 100}%"></div>
        </div>
        <div class="progress-message">${message}</div>
    `;
}

function displayResults(results) {
    document.getElementById('progress-container').style.display = 'none';
    document.getElementById('results-container').style.display = 'block';

    // Display omissions
    const omissionsList = document.getElementById('omissions-list');
    results.pass1_analysis.omissions
        .sort((a, b) => b.composite_risk - a.composite_risk)
        .forEach(omission => {
            omissionsList.innerHTML += createOmissionCard(omission);
        });

    // Display responses
    displayResponse('reframe', results.responses.reframe);
}

function createOmissionCard(omission) {
    const riskColor = omission.composite_risk >= 0.7 ? 'red' :
                      omission.composite_risk >= 0.5 ? 'orange' : 'yellow';

    const riskIcon = omission.composite_risk >= 0.7 ?
                     '<i class="fas fa-exclamation-circle"></i>' :
                     omission.composite_risk >= 0.5 ?
                     '<i class="fas fa-exclamation-triangle"></i>' :
                     '<i class="fas fa-info-circle"></i>';

    return `
        <div class="omission-card risk-${riskColor}">
            <div class="omission-header">
                <span class="category">
                    <i class="fas fa-tag"></i> ${omission.category}
                </span>
                <span class="risk-score">
                    ${riskIcon} ${(omission.composite_risk * 100).toFixed(0)}%
                </span>
            </div>
            <div class="omission-description">
                <i class="fas fa-comment-dots"></i> ${omission.description}
            </div>
            <div class="omission-reference">
                <i class="fas fa-quote-left"></i> "${omission.text_reference}"
            </div>
            <div class="omission-suggestion">
                <i class="fas fa-plus-circle"></i>
                <strong>Suggested addition:</strong> ${omission.suggested_addition}
            </div>
        </div>
    `;
}
```

**Testing**:
- UI loads correctly in modern browsers
- WebSocket connection established
- Progress updates display in real-time
- Results render correctly with sample data
- Responsive design on mobile/tablet

**Owner**: Frontend Engineer
**Estimated Effort**: 24 hours

---

#### Day 29-30: Integration & Polish

**Tasks**:
- Integrate UI with WebSocket server
- Add error handling and retry logic
- Implement loading states and animations
- Add copy-to-clipboard for responses (fix ZES-11 bug)
- Cross-browser testing

**Key Fix for ZES-11** (Copy Button):
```javascript
function copyToClipboard(text) {
    // Modern Clipboard API
    if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text)
            .then(() => {
                showNotification('Copied to clipboard!', 'success');
            })
            .catch(err => {
                // Fallback to execCommand
                copyToClipboardFallback(text);
            });
    } else {
        copyToClipboardFallback(text);
    }
}

function copyToClipboardFallback(text) {
    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);

    try {
        textarea.select();
        document.execCommand('copy');
        showNotification('Copied to clipboard!', 'success');
    } catch (err) {
        showNotification('Failed to copy. Please select and copy manually.', 'error');
    } finally {
        document.body.removeChild(textarea);
    }
}
```

**Testing**:
- End-to-end UI → Server → Agent workflow test
- Copy button test (multiple browsers)
- Error scenario testing (network failure, invalid input)
- Performance test (large input text >5000 words)

**Owner**: Full Stack Engineer
**Estimated Effort**: 16 hours

---

### Phase 3 Milestones

**Week 6 End Deliverables**:
- ✅ WebSocket server operational with 6 message types
- ✅ Redis persistence layer complete with TTL management
- ✅ Real-time UI with progress tracking
- ✅ Copy-to-clipboard bug fixed (ZES-11)
- ✅ Responsive design on desktop/tablet/mobile
- ✅ Integration tests passing for full stack
- ✅ UI loads in <2s, progress updates <100ms latency

**Success Metrics**:
- WebSocket server handles 50 concurrent sessions
- Redis persistence <50ms per operation
- UI supports IE11, Chrome, Firefox, Safari
- Copy button works reliably across all browsers
- Zero JavaScript errors in console

**Demo**: Live demonstration of real-time analysis with progress tracking, show all 4 phases completing, display results with copy functionality

---

## Phase 4: Testing, Polish, Deployment (Weeks 7-8)

**Dates**: 2025-10-25 to 2025-12-06
**Goal**: Production readiness - performance, security, deployment

### Week 7: Security, Performance, Testing

#### Day 31-32: Security Hardening

**Tasks**:
- Implement prompt injection prevention
- Add PII redaction for sensitive data
- Rate limiting for API endpoints
- Security audit with automated tools

**Prompt Injection Prevention**:
```rust
use terraphim_multi_agent::prompt_sanitizer::{sanitize_system_prompt, validate_system_prompt};

impl SecurityMiddleware {
    pub fn sanitize_user_input(input: &str) -> anyhow::Result<String> {
        // Use existing terraphim sanitizer
        let sanitized = sanitize_system_prompt(input);

        if sanitized.was_modified {
            tracing::warn!(
                "Input sanitized: {} patterns detected",
                sanitized.modifications.len()
            );
        }

        // Additional TruthForge-specific checks
        if input.len() > 10000 {
            anyhow::bail!("Input exceeds maximum length");
        }

        Ok(sanitized.sanitized_prompt)
    }
}
```

**PII Redaction**:
```rust
pub fn redact_pii(text: &str) -> String {
    use regex::Regex;

    lazy_static! {
        static ref EMAIL_REGEX: Regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
        static ref PHONE_REGEX: Regex = Regex::new(r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap();
        static ref SSN_REGEX: Regex = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
    }

    let mut redacted = text.to_string();
    redacted = EMAIL_REGEX.replace_all(&redacted, "[EMAIL REDACTED]").to_string();
    redacted = PHONE_REGEX.replace_all(&redacted, "[PHONE REDACTED]").to_string();
    redacted = SSN_REGEX.replace_all(&redacted, "[SSN REDACTED]").to_string();

    redacted
}
```

**Testing**:
- Prompt injection test suite (leverage terraphim tests)
- PII redaction accuracy test
- Rate limiting test (429 responses after threshold)
- Security scan with cargo-audit

**Owner**: Security Engineer
**Estimated Effort**: 16 hours

---

#### Day 33-35: Performance Optimization

**Tasks**:
- Profile workflow execution (identify bottlenecks)
- Optimize agent prompt sizes (reduce token usage)
- Implement response caching for common inputs
- Database query optimization
- Load testing with k6

**Performance Targets**:
- Total workflow: <90s (OpenRouter), <45s (Ollama)
- Pass 1: <45s, Pass 2: <30s, Response: <15s
- Cost per analysis: <$5 (OpenRouter)
- Support 10 concurrent analyses

**Optimization** (Agent Prompt Caching):
```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AgentCache {
    cache: Arc<RwLock<HashMap<String, CachedResponse>>>,
}

impl AgentCache {
    pub async fn get_or_generate(
        &self,
        input_hash: &str,
        generator: impl Future<Output = anyhow::Result<String>>,
    ) -> anyhow::Result<String> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(input_hash) {
                if cached.is_valid() {
                    return Ok(cached.response.clone());
                }
            }
        }

        // Generate and cache
        let response = generator.await?;
        let mut cache = self.cache.write().await;
        cache.insert(input_hash.to_string(), CachedResponse::new(response.clone()));

        Ok(response)
    }
}
```

**Load Testing** (`tests/load_test.js` with k6):
```javascript
import { check } from 'k6';
import http from 'k6/http';
import ws from 'k6/ws';

export let options = {
    stages: [
        { duration: '1m', target: 5 },   // Ramp up to 5 users
        { duration: '3m', target: 10 },  // Stay at 10 users
        { duration: '1m', target: 0 },   // Ramp down
    ],
    thresholds: {
        'http_req_duration': ['p95<90000'], // 95% under 90s
        'ws_connecting': ['p95<1000'],      // WebSocket connect <1s
    },
};

export default function() {
    const payload = JSON.stringify({
        session_id: `session-${__VU}-${__ITER}`,
        input_text: 'Sample crisis communication text...'
    });

    const res = http.post('http://localhost:3000/api/v1/analysis', payload, {
        headers: { 'Content-Type': 'application/json' },
    });

    check(res, {
        'status is 200': (r) => r.status === 200,
        'response has session_id': (r) => JSON.parse(r.body).session_id !== undefined,
    });
}
```

**Testing**:
- Benchmark individual agents (measure token usage)
- Load test with k6 (10 concurrent users)
- Cost tracking with actual OpenRouter billing
- Memory leak detection (run for 1 hour)

**Owner**: Performance Engineer
**Estimated Effort**: 24 hours

---

### Week 8: Deployment & Launch

#### Day 36-37: Docker & Kubernetes Setup

**Tasks**:
- Create Dockerfile for server
- Create docker-compose for local development
- Write Kubernetes deployment manifests
- Set up Helm chart for production deployment

**Dockerfile** (`Dockerfile`):
```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin terraphim_truthforge_server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/terraphim_truthforge_server /usr/local/bin/

ENV RUST_LOG=info
ENV OPENROUTER_API_KEY=""
ENV REDIS_URL="redis://localhost:6379"

EXPOSE 3000

CMD ["terraphim_truthforge_server"]
```

**Docker Compose** (`docker-compose.yml`):
```yaml
version: '3.8'

services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes

  truthforge-server:
    build: .
    ports:
      - "3000:3000"
    environment:
      - RUST_LOG=info
      - OPENROUTER_API_KEY=${OPENROUTER_API_KEY}
      - REDIS_URL=redis://redis:6379
    depends_on:
      - redis
    restart: unless-stopped

volumes:
  redis-data:
```

**Kubernetes Deployment** (`k8s/deployment.yaml`):
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: truthforge-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: truthforge
  template:
    metadata:
      labels:
        app: truthforge
    spec:
      containers:
      - name: server
        image: truthforge/server:latest
        ports:
        - containerPort: 3000
        env:
        - name: REDIS_URL
          valueFrom:
            configMapKeyRef:
              name: truthforge-config
              key: redis-url
        - name: OPENROUTER_API_KEY
          valueFrom:
            secretKeyRef:
              name: truthforge-secrets
              key: openrouter-api-key
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
```

**Testing**:
- Docker build succeeds
- Docker-compose starts all services
- Kubernetes deployment on staging cluster
- Health checks working
- Horizontal pod autoscaling test

**Owner**: DevOps Engineer
**Estimated Effort**: 16 hours

---

#### Day 38-39: Monitoring & Observability

**Tasks**:
- Set up structured logging (tracing)
- Prometheus metrics export
- Grafana dashboards for key metrics
- Error tracking with Sentry

**Metrics** (`src/metrics.rs`):
```rust
use prometheus::{register_histogram_vec, register_counter_vec, HistogramVec, CounterVec};

lazy_static! {
    pub static ref AGENT_DURATION: HistogramVec = register_histogram_vec!(
        "truthforge_agent_duration_seconds",
        "Time spent in each agent",
        &["agent_name"]
    ).unwrap();

    pub static ref OMISSIONS_DETECTED: CounterVec = register_counter_vec!(
        "truthforge_omissions_detected_total",
        "Total omissions detected by category",
        &["category"]
    ).unwrap();

    pub static ref ANALYSIS_COST: HistogramVec = register_histogram_vec!(
        "truthforge_analysis_cost_usd",
        "Cost per analysis in USD",
        &["llm_provider"]
    ).unwrap();
}
```

**Grafana Dashboard** (key panels):
- Analysis throughput (analyses/minute)
- Agent latency distribution (p50, p95, p99)
- Omission detection rate by category
- Cost per analysis trend
- WebSocket connection count
- Redis operation latency

**Testing**:
- Metrics export endpoint working
- Prometheus scraping correctly
- Grafana dashboards visualize sample data
- Alerts trigger for SLA violations

**Owner**: SRE Engineer
**Estimated Effort**: 16 hours

---

#### Day 40: Final Testing & Launch Prep

**Tasks**:
- Run full regression test suite
- Perform end-to-end production simulation
- Update documentation (API docs, user guide)
- Prepare launch announcement
- Final security review

**Launch Checklist**:
- [ ] All 100+ tests passing
- [ ] Security audit complete (no high/critical issues)
- [ ] Performance targets met (<90s, <$5)
- [ ] Monitoring dashboards operational
- [ ] Kubernetes deployment successful
- [ ] Documentation published
- [ ] Backup and disaster recovery tested
- [ ] On-call rotation scheduled
- [ ] Stakeholder sign-off obtained

**Owner**: Technical Lead + Product Owner
**Estimated Effort**: 8 hours

---

### Phase 4 Milestones

**Week 8 End Deliverables**:
- ✅ Security hardening complete (prompt injection, PII redaction, rate limiting)
- ✅ Performance optimized (meets all latency/cost targets)
- ✅ Load tested (10 concurrent users, 95th percentile <90s)
- ✅ Deployed to production Kubernetes cluster
- ✅ Monitoring and alerting operational
- ✅ Documentation complete (API, user guide, runbook)
- ✅ Launch announcement published

**Success Metrics**:
- Zero high/critical security vulnerabilities
- 100% of tests passing
- Production deployment with 99.9% uptime SLA
- Monitoring captures 100% of critical metrics
- Documentation rated ≥4/5 by early users

**Launch**: TruthForge Two-Pass Debate Arena goes live!

---

## Risk Management

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| OpenRouter API rate limits | Medium | High | Implement exponential backoff, use Ollama fallback for testing |
| LLM response parsing failures | High | Medium | Strict JSON schema validation, retry with clarified prompts |
| WebSocket connection instability | Low | Medium | Auto-reconnect logic, heartbeat pings |
| Redis persistence failures | Low | High | Redis Cluster with replication, automatic failover |
| Omission detection accuracy <85% | Medium | High | Iterative prompt refinement, expert validation in Phase 2 |
| Cost overruns (>$5 per analysis) | Medium | Medium | Token usage monitoring, smaller model fallback (Haiku) |
| Performance degradation under load | Medium | High | Horizontal scaling, caching, load testing in Phase 4 |

### Schedule Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Agent implementation takes longer | High | Medium | Prioritize Pass 1 agents, parallelize work across engineers |
| Integration issues between phases | Medium | Medium | Daily integration testing, continuous deployment |
| Security vulnerabilities discovered late | Low | High | Weekly security reviews, automated scanning |
| Deployment delays | Medium | Low | Staging environment testing, rollback procedures |

---

## Success Criteria Summary

### Phase 1 (Week 2)
- ✅ 3 analysis agents functional
- ✅ Taxonomy migration complete
- ✅ 27+ tests passing
- ✅ Integration pipeline working

### Phase 2 (Week 4)
- ✅ All 13 agents implemented
- ✅ Two-pass workflow <90s
- ✅ Omission detection ≥85% accuracy
- ✅ Cost <$5 per analysis
- ✅ 60+ tests passing

### Phase 3 (Week 6)
- ✅ WebSocket real-time UI
- ✅ Redis persistence operational
- ✅ Copy button bug fixed
- ✅ Responsive design
- ✅ Integration tests passing

### Phase 4 (Week 8)
- ✅ Security audit passed
- ✅ Performance targets met
- ✅ Production deployed
- ✅ Monitoring operational
- ✅ 100+ tests passing

---

## Team Structure

| Role | Responsibilities | Allocation |
|------|-----------------|-----------|
| **AI Engineer** | Agent implementation, prompt engineering, LLM integration | 2 FTE |
| **Backend Engineer** | Workflow orchestration, Redis, core logic | 1 FTE |
| **Frontend Engineer** | UI/UX, WebSocket client, visualization | 1 FTE |
| **DevOps Engineer** | CI/CD, Docker, Kubernetes, deployment | 0.5 FTE |
| **Security Engineer** | Security review, prompt injection prevention, PII redaction | 0.5 FTE |
| **Technical Lead** | Architecture decisions, code review, coordination | 0.5 FTE |
| **Product Owner** | Requirements, testing, stakeholder communication | 0.5 FTE |

**Total**: 6.5 FTE

---

## Cost Estimation

### Development Costs
- **Labor** (8 weeks × 6.5 FTE × $150/hour × 40 hours/week): ~$312,000
- **Infrastructure** (development, staging): ~$2,000
- **Tools & Services** (OpenRouter API credits, monitoring): ~$3,000
- **Total Development**: **~$317,000**

### Operational Costs (Monthly)
- **Infrastructure** (Kubernetes cluster, Redis, load balancer): ~$500/month
- **LLM API** (OpenRouter, assuming 1000 analyses/month @ $5 each): ~$5,000/month
- **Monitoring & Logging** (Prometheus, Grafana, Sentry): ~$200/month
- **Total Monthly**: **~$5,700/month**

---

## Post-Launch Roadmap

### Month 1-2: Monitoring & Iteration
- Collect user feedback
- Monitor omission detection accuracy in production
- Optimize agent prompts based on real usage
- Fix bugs and performance issues

### Month 3-4: Feature Enhancements
- Add support for additional SCCT classifications
- Implement multi-language support
- Create custom agent role builder UI
- Expand taxonomy with industry-specific concepts

### Month 5-6: Scale & Integration
- API for third-party integrations
- Slack/Teams bot for quick analyses
- Export to PowerPoint/PDF for presentations
- Learning vault recommendation engine

---

**Document Status**: Draft v1.0
**Next Review**: Weekly during implementation
**Approval Required**: Technical Lead, Product Owner, Finance
