# Technical Specification: TruthForge Terraphim-AI Integration

**Version**: 1.0
**Date**: 2025-10-07
**Status**: Draft
**Owner**: Zestic AI Engineering Team
**Related**: PRD_TwoPassDebateArena.md, ARCHITECTURE_TerraphimPatterns.md

---

## 1. System Architecture Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                            │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  TruthForge Web UI (HTML/JS - Agent-Workflows Pattern)   │  │
│  │  - Narrative input                                        │  │
│  │  - Real-time progress pipeline                           │  │
│  │  - Results visualization                                 │  │
│  │  - WebSocket client (auto-reconnect)                    │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────┬────────────────────────────────────────┘
                         │ WebSocket (wss://) + REST (https://)
┌────────────────────────┴────────────────────────────────────────┐
│                      API Gateway Layer                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  terraphim_truthforge_server (Axum + tokio-tungstenite) │  │
│  │  - WebSocket handler (progress streaming)               │  │
│  │  - REST endpoints (session CRUD)                        │  │
│  │  - Authentication middleware (OAuth2)                    │  │
│  │  - Rate limiting (100 req/hr/user)                      │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────┬────────────────────────────────────────┘
                         │ In-process Rust calls
┌────────────────────────┴────────────────────────────────────────┐
│                    Multi-Agent Orchestration                    │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  terraphim_truthforge (Core Library)                     │  │
│  │  ┌───────────────────────────────────────────────────┐  │  │
│  │  │  TwoPassDebateWorkflow                            │  │  │
│  │  │  - PassOneOrchestrator (Orchestrator-Workers)     │  │  │
│  │  │  - PassTwoOptimizer (Evaluator-Optimizer)         │  │  │
│  │  │  - ResponseGenerator (Parallelization)            │  │  │
│  │  └───────────────────────────────────────────────────┘  │  │
│  │                                                           │  │
│  │  ┌───────────────────────────────────────────────────┐  │  │
│  │  │  TerraphimAgent Pool                              │  │  │
│  │  │  - BiasDetector, NarrativeMapper, TaxonomyLinker │  │  │
│  │  │  - OmissionDetector (NEW)                         │  │  │
│  │  │  - Debaters (Supporting/Opposing)                 │  │  │
│  │  │  - CumulativeEvaluator (NEW)                      │  │  │
│  │  │  - Response Agents (Reframe/Counter/Bridge)       │  │  │
│  │  └───────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────┬────────────────────────────┬───────────────────────────┘
         │                            │
         │ LLM API                    │ Persistence
         │                            │
┌────────┴────────┐          ┌────────┴────────────────────────────┐
│  LLM Providers  │          │  Storage Layer                      │
│  ┌───────────┐ │          │  ┌──────────────┐ ┌───────────────┐│
│  │OpenRouter │ │          │  │Redis Cluster │ │DeviceStorage  ││
│  │(Claude    │ │          │  │- Sessions    │ │(terraphim_    ││
│  │ 3.5       │ │          │  │- Results     │ │persistence)   ││
│  │ Sonnet)   │ │          │  │- Cache       │ │- Knowledge    ││
│  └───────────┘ │          │  └──────────────┘ │  graphs       ││
│  ┌───────────┐ │          │                   │- Agent memory ││
│  │Ollama     │ │          │                   │- Learning     ││
│  │(gemma3:   │ │          │                   │  vault        ││
│  │ 270m)     │ │          │                   └───────────────┘│
│  │TEST ONLY  │ │          └─────────────────────────────────────┘
│  └───────────┘ │
└─────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│              Knowledge Graph & Taxonomy Layer                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  TruthForge RoleGraph (terraphim_rolegraph)              │  │
│  │  - Strategic communication taxonomy                      │  │
│  │  - SCCT crisis classifications                           │  │
│  │  - Workflow pattern mappings                             │  │
│  │                                                           │  │
│  │  TruthForge Automata (terraphim_automata)                │  │
│  │  - PR terminology index                                  │  │
│  │  - Crisis keyword detection                              │  │
│  │  - Narrative pattern matching                            │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Component Responsibilities

#### terraphim_truthforge (Core Library)
- **Purpose**: Multi-agent workflow orchestration for narrative analysis
- **Language**: Rust 2021 edition
- **Key Modules**:
  - `workflows/`: Two-pass debate orchestration
  - `agents/`: Specialized agent implementations
  - `taxonomy/`: RoleGraph migration and SCCT mapping
  - `persistence/`: Redis backend integration

#### terraphim_truthforge_server (WebSocket Server)
- **Purpose**: API gateway and real-time communication
- **Framework**: Axum (web) + tokio-tungstenite (WebSocket)
- **Responsibilities**:
  - Accept narrative submissions
  - Stream agent progress via WebSocket
  - Manage session lifecycle
  - Serve static UI assets

#### Client UI (Agent-Workflows Pattern)
- **Purpose**: User interface for narrative analysis
- **Technology**: Vanilla JS + HTML5 + CSS Grid
- **Pattern**: Matches `examples/agent-workflows/` design
- **Features**:
  - Narrative input with context toggles
  - Real-time progress pipeline visualization
  - Simplified results display (ZES-11)
  - Copy functionality (crash-proof)

---

## 2. Crate Structure & Dependencies

### 2.1 Repository Layout

```
terraphim_truthforge/  (PRIVATE REPOSITORY)
├── Cargo.toml                        # Workspace manifest
├── README.md
├── LICENSE                           # Proprietary - Zestic AI
├── .gitignore
├── .github/
│   └── workflows/
│       ├── ci.yml                    # Test, lint, build
│       └── deploy.yml                # Docker build + k8s deploy
│
├── crates/
│   ├── terraphim_truthforge/         # Core library
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                # Public API
│   │   │   ├── error.rs              # Error types
│   │   │   ├── agents/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── omission_detector.rs
│   │   │   │   ├── exploitation_debater.rs
│   │   │   │   ├── cumulative_evaluator.rs
│   │   │   │   └── debate_orchestrator.rs
│   │   │   ├── workflows/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pass_one.rs       # Orchestrator-Workers
│   │   │   │   ├── pass_two.rs       # Evaluator-Optimizer
│   │   │   │   └── two_pass_debate.rs
│   │   │   ├── taxonomy/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── rolegraph_migration.rs
│   │   │   │   ├── scct_mapping.rs
│   │   │   │   └── playbook_selector.rs
│   │   │   ├── persistence/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── redis_backend.rs
│   │   │   │   └── session_manager.rs
│   │   │   └── types.rs              # Shared data structures
│   │   ├── config/
│   │   │   └── roles/                # JSON role configurations
│   │   │       ├── bias_detector_role.json
│   │   │       ├── narrative_mapper_role.json
│   │   │       ├── taxonomy_linker_role.json
│   │   │       ├── omission_detector_role.json
│   │   │       ├── debater_supporting_role.json
│   │   │       ├── debater_opposing_role.json
│   │   │       ├── pass1_evaluator_role.json
│   │   │       ├── exploitation_debater_supporting_role.json
│   │   │       ├── exploitation_debater_opposing_role.json
│   │   │       ├── cumulative_evaluator_role.json
│   │   │       ├── reframe_agent_role.json
│   │   │       ├── counter_argue_agent_role.json
│   │   │       └── bridge_agent_role.json
│   │   └── tests/
│   │       ├── integration_tests.rs
│   │       ├── workflow_tests.rs
│   │       └── fixtures/
│   │           └── test_narratives.json
│   │
│   └── terraphim_truthforge_server/  # WebSocket server
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs
│       │   ├── config.rs
│       │   ├── websocket.rs          # WebSocket handler
│       │   ├── routes.rs             # REST API
│       │   ├── handlers/
│       │   │   ├── mod.rs
│       │   │   ├── analysis.rs       # POST /analysis
│       │   │   ├── session.rs        # GET /session/:id
│       │   │   └── health.rs         # GET /health
│       │   ├── middleware/
│       │   │   ├── mod.rs
│       │   │   ├── auth.rs           # OAuth2 validation
│       │   │   └── rate_limit.rs     # Token bucket
│       │   └── state.rs              # Shared application state
│       └── static/                   # UI assets
│           ├── index.html
│           ├── css/
│           │   ├── shared-styles.css # From agent-workflows
│           │   └── debate-arena.css  # TruthForge-specific
│           ├── js/
│           │   ├── api-client.js     # WebSocket + REST
│           │   ├── workflow-visualizer.js
│           │   ├── debate-arena.js
│           │   └── utils.js
│           └── assets/
│               ├── logo.svg
│               └── favicon.ico
│
├── taxonomy/
│   ├── truthforge_rolegraph.json     # Migrated from trueforge_taxonomy.json
│   ├── pr_terminology.txt            # For automata index
│   └── scct_mappings.json            # Crisis classification rules
│
├── docs/
│   ├── PRD_TwoPassDebateArena.md
│   ├── SPEC_TerraphimIntegration.md  # This file
│   ├── REQUIREMENTS_AgentRoles.md
│   ├── ROADMAP_Implementation.md
│   ├── ARCHITECTURE_TerraphimPatterns.md
│   └── API.md                        # API documentation
│
└── deploy/
    ├── Dockerfile
    ├── docker-compose.yml
    └── k8s/
        ├── deployment.yaml
        ├── service.yaml
        └── ingress.yaml
```

### 2.2 Cargo.toml (Workspace)

```toml
[workspace]
members = [
    "crates/terraphim_truthforge",
    "crates/terraphim_truthforge_server",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Proprietary"
authors = ["Zestic AI <team@zestic.ai>"]

[workspace.dependencies]
# Terraphim ecosystem (public repo)
terraphim-multi-agent = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
terraphim-config = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
terraphim-rolegraph = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
terraphim-automata = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
terraphim-persistence = { git = "https://github.com/terraphim/terraphim-ai", branch = "main", features = ["redis"] }
terraphim-types = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
terraphim-agent-evolution = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }

# Async runtime
tokio = { version = "1.35", features = ["full"] }
tokio-tungstenite = "0.21"
futures = "0.3"

# Web framework
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "fs", "trace"] }
hyper = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Redis
redis = { version = "0.24", features = ["tokio-comp", "connection-manager", "cluster"] }

# LLM integration
rig-core = "0.1"  # For OpenRouter

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging & tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Crypto & security
sha2 = "0.10"
aes-gcm = "0.10"
uuid = { version = "1.6", features = ["v4", "serde"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Testing
mockall = "0.12"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

### 2.3 terraphim_truthforge/Cargo.toml

```toml
[package]
name = "terraphim-truthforge"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
# Terraphim dependencies
terraphim-multi-agent = { workspace = true }
terraphim-config = { workspace = true }
terraphim-rolegraph = { workspace = true }
terraphim-automata = { workspace = true }
terraphim-persistence = { workspace = true }
terraphim-types = { workspace = true }
terraphim-agent-evolution = { workspace = true }

# Async
tokio = { workspace = true }
futures = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Redis
redis = { workspace = true }

# LLM
rig-core = { workspace = true }

# Error handling
thiserror = { workspace = true }
anyhow = { workspace = true }

# Logging
tracing = { workspace = true }

# Utils
uuid = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
terraphim-multi-agent = { workspace = true, features = ["test-utils"] }
tokio-test = "0.4"
mockall = { workspace = true }
tempfile = "3.8"

[features]
default = ["redis-persistence"]
redis-persistence = []
test-ollama = []  # Enable Ollama for integration tests
```

### 2.4 terraphim_truthforge_server/Cargo.toml

```toml
[package]
name = "terraphim-truthforge-server"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "truthforge-server"
path = "src/main.rs"

[dependencies]
# Internal
terraphim-truthforge = { path = "../terraphim_truthforge" }

# Terraphim dependencies
terraphim-multi-agent = { workspace = true }
terraphim-config = { workspace = true }

# Web framework
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
hyper = { workspace = true }

# WebSocket
tokio-tungstenite = { workspace = true }

# Async
tokio = { workspace = true }
futures = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }

# Redis
redis = { workspace = true }

# Error handling
thiserror = { workspace = true }
anyhow = { workspace = true }

# Logging
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

# Security
sha2 = { workspace = true }
aes-gcm = { workspace = true }

# Utils
uuid = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio-test = "0.4"
```

---

## 3. Data Models & Schemas

### 3.1 Core Data Structures

#### Narrative Input

```rust
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
    High,  // Immediate crisis response
    Low,   // Strategic planning mode
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StakeType {
    Reputational,
    Legal,
    Financial,
    Operational,
    SocialLicense,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudienceType {
    PublicMedia,  // External-facing
    Internal,     // Stakeholder communication
}
```

#### Omission Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Omission {
    pub category: OmissionCategory,
    pub description: String,
    pub severity: f64,           // 0.0-1.0
    pub exploitability: f64,     // 0.0-1.0
    pub composite_risk: f64,     // severity × exploitability
    pub text_reference: String,  // Quote from narrative
    pub confidence: f64,         // 0.0-1.0
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
    pub prioritized: Vec<Uuid>,  // Top 10 by composite_risk
    pub total_risk_score: f64,
}
```

#### Debate Result Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateResult {
    pub pass: DebatePass,
    pub supporting_argument: Argument,
    pub opposing_argument: Argument,
    pub evaluation: DebateEvaluation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebatePass {
    PassOne,   // Initial debate with omission awareness
    PassTwo,   // Exploitation-focused debate
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
    pub omissions_referenced: Vec<Uuid>,  // References to OmissionCatalog
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
```

#### Cumulative Analysis

```rust
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
    pub supporting_strength_change: f64,  // Pass2 - Pass1
    pub opposing_strength_change: f64,
    pub amplification_factor: f64,        // How much worse under attack
    pub critical_omissions_exploited: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointOfFailure {
    pub narrative_claim: String,
    pub omission_exploited: Uuid,
    pub failure_mechanism: String,
    pub stakeholder_impact: String,
}
```

#### Response Strategies

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseStrategy {
    pub strategy_type: StrategyType,
    pub strategic_rationale: String,
    pub drafts: ResponseDrafts,
    pub risk_assessment: RiskAssessment,
    pub tone_guidance: ToneGuidance,
    pub vulnerabilities_addressed: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    Reframe,       // Shift context, reduce polarization
    CounterArgue,  // Direct fact-based rebuttal
    Bridge,        // Pivot to future commitments
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDrafts {
    pub social_media: String,      // ≤280 chars
    pub press_statement: String,   // 2-3 paragraphs
    pub internal_memo: String,     // 1 page
    pub qa_brief: Vec<QAPair>,     // Anticipated Q&A
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToneGuidance {
    Formal,
    Empathetic,
    Assertive,
    Collaborative,
}
```

#### Complete Analysis Result

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthForgeAnalysisResult {
    pub session_id: Uuid,
    pub narrative: NarrativeInput,

    // Phase 2: Analysis
    pub bias_analysis: BiasAnalysis,
    pub narrative_mapping: NarrativeMapping,
    pub taxonomy_linking: TaxonomyLinking,
    pub omission_catalog: OmissionCatalog,

    // Phase 3: Debate
    pub pass_one_debate: DebateResult,
    pub pass_two_debate: DebateResult,
    pub cumulative_analysis: CumulativeAnalysis,

    // Phase 4: Response
    pub response_strategies: Vec<ResponseStrategy>,

    // Metadata
    pub executive_summary: String,
    pub processing_time_ms: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

### 3.2 Redis Key Patterns

#### Session Storage
```
Key:   session:{uuid}
Type:  String (JSON)
Value: NarrativeInput
TTL:   24 hours

Example:
session:550e8400-e29b-41d4-a716-446655440000
```

#### Analysis Results
```
Key:   results:{session_id}
Type:  String (JSON)
Value: TruthForgeAnalysisResult
TTL:   7 days

Example:
results:550e8400-e29b-41d4-a716-446655440000
```

#### Progress Tracking
```
Key:   progress:{session_id}
Type:  Hash
Fields:
  - phase: "analysis" | "debate_pass1" | "debate_pass2" | "response"
  - agent_{name}_progress: 0.0-1.0
  - agent_{name}_status: "pending" | "running" | "complete" | "error"
TTL:   1 hour

Example:
progress:550e8400-e29b-41d4-a716-446655440000
  phase: "analysis"
  agent_bias_detector_progress: "0.75"
  agent_bias_detector_status: "running"
```

#### Learning Vault (Long-term storage)
```
Key:   vault:case:{session_id}
Type:  String (JSON)
Value: RedactedAnalysisResult (PII removed)
TTL:   90 days

Example:
vault:case:550e8400-e29b-41d4-a716-446655440000
```

#### Rate Limiting
```
Key:   ratelimit:{user_id}:{hour}
Type:  String
Value: Request count
TTL:   1 hour

Example:
ratelimit:user_12345:2025-10-07-14
Value: "42"
```

---

## 4. API Specifications

### 4.1 REST Endpoints

#### POST /api/v1/analysis

Submit narrative for analysis

**Request**:
```json
{
  "narrative": {
    "text": "Narrative text here...",
    "context": {
      "urgency": "High",
      "stakes": ["Reputational", "Legal"],
      "audience": "PublicMedia"
    }
  }
}
```

**Response** (202 Accepted):
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "websocket_url": "wss://truthforge.zestic.ai/ws/550e8400-e29b-41d4-a716-446655440000",
  "estimated_duration_ms": 45000
}
```

#### GET /api/v1/session/{session_id}

Retrieve session results (polling alternative to WebSocket)

**Response** (200 OK):
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "progress": 1.0,
  "results": {
    // TruthForgeAnalysisResult
  }
}
```

**Response** (202 Accepted) - Still processing:
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "processing",
  "progress": 0.65,
  "current_phase": "debate_pass2",
  "estimated_remaining_ms": 12000
}
```

#### DELETE /api/v1/session/{session_id}

Delete session (GDPR right to deletion)

**Response** (204 No Content)

#### GET /health

Health check endpoint

**Response** (200 OK):
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "redis": {
    "status": "connected",
    "latency_ms": 2.3
  },
  "agent_pool": {
    "total_agents": 13,
    "available": 10,
    "busy": 3
  }
}
```

### 4.2 WebSocket Protocol

#### Connection

```
Client → Server:
GET /ws/{session_id}
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: ...

Server → Client:
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
```

#### Message Types

##### Client → Server: Start Analysis

```json
{
  "type": "start_analysis",
  "session_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

##### Server → Client: Agent Progress

```json
{
  "type": "agent_progress",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "agent": "BiasDetectorAgent",
  "phase": "analysis",
  "status": "running",
  "progress": 0.75,
  "estimated_remaining_ms": 8000,
  "timestamp": "2025-10-07T14:23:45Z"
}
```

##### Server → Client: Agent Result

```json
{
  "type": "agent_result",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "agent": "OmissionDetectorAgent",
  "phase": "analysis",
  "result": {
    "omissions": [
      {
        "category": "MissingEvidence",
        "description": "Claim about 40% reduction lacks data source",
        "severity": 0.85,
        "exploitability": 0.92,
        "text_reference": "We achieved 40% reduction in incidents"
      }
    ]
  },
  "confidence": 0.87,
  "timestamp": "2025-10-07T14:23:52Z"
}
```

##### Server → Client: Phase Complete

```json
{
  "type": "phase_complete",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "phase": "analysis",
  "results": {
    "bias_analysis": {...},
    "narrative_mapping": {...},
    "taxonomy_linking": {...},
    "omission_catalog": {...}
  },
  "duration_ms": 18234,
  "timestamp": "2025-10-07T14:24:03Z"
}
```

##### Server → Client: Analysis Complete

```json
{
  "type": "analysis_complete",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "results": {
    // Complete TruthForgeAnalysisResult
  },
  "total_duration_ms": 43567,
  "timestamp": "2025-10-07T14:24:28Z"
}
```

##### Server → Client: Error

```json
{
  "type": "error",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "error_code": "AGENT_TIMEOUT",
  "message": "OmissionDetectorAgent timed out after 30 seconds",
  "recoverable": false,
  "timestamp": "2025-10-07T14:24:15Z"
}
```

##### Client → Server: Heartbeat

```json
{
  "type": "ping",
  "timestamp": "2025-10-07T14:24:00Z"
}
```

##### Server → Client: Heartbeat Response

```json
{
  "type": "pong",
  "timestamp": "2025-10-07T14:24:00Z"
}
```

### 4.3 WebSocket Implementation

```rust
// terraphim_truthforge_server/src/websocket.rs

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    StartAnalysis { session_id: Uuid },
    Ping { timestamp: chrono::DateTime<chrono::Utc> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    AgentProgress {
        session_id: Uuid,
        agent: String,
        phase: String,
        status: String,
        progress: f64,
        estimated_remaining_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    AgentResult {
        session_id: Uuid,
        agent: String,
        phase: String,
        result: serde_json::Value,
        confidence: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    PhaseComplete {
        session_id: Uuid,
        phase: String,
        results: serde_json::Value,
        duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    AnalysisComplete {
        session_id: Uuid,
        results: serde_json::Value,
        total_duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Error {
        session_id: Uuid,
        error_code: String,
        message: String,
        recoverable: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Pong {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<Uuid>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, session_id, app_state))
}

async fn handle_socket(socket: WebSocket, session_id: Uuid, app_state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Create progress channel
    let (progress_tx, mut progress_rx) = mpsc::channel::<ServerMessage>(100);

    // Spawn task to send progress updates to client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = progress_rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::StartAnalysis { session_id }) => {
                        // Start analysis workflow
                        let app_state = app_state.clone();
                        let progress_tx = progress_tx.clone();

                        tokio::spawn(async move {
                            if let Err(e) = run_analysis_workflow(
                                session_id,
                                app_state,
                                progress_tx
                            ).await {
                                tracing::error!("Analysis workflow error: {}", e);
                            }
                        });
                    }
                    Ok(ClientMessage::Ping { timestamp }) => {
                        let pong = ServerMessage::Pong {
                            timestamp: chrono::Utc::now()
                        };
                        progress_tx.send(pong).await.ok();
                    }
                    Err(e) => {
                        tracing::warn!("Invalid client message: {}", e);
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
}

async fn run_analysis_workflow(
    session_id: Uuid,
    app_state: Arc<AppState>,
    progress_tx: mpsc::Sender<ServerMessage>,
) -> anyhow::Result<()> {
    use terraphim_truthforge::workflows::TwoPassDebateWorkflow;

    // Load narrative from Redis
    let narrative = app_state.redis_manager
        .get_session(session_id)
        .await?;

    // Create workflow with progress callback
    let workflow = TwoPassDebateWorkflow::new(
        app_state.agent_pool.clone(),
        move |update| {
            let progress_tx = progress_tx.clone();
            tokio::spawn(async move {
                progress_tx.send(update).await.ok();
            });
        }
    );

    // Execute workflow
    let results = workflow.execute(&narrative).await?;

    // Save results to Redis
    app_state.redis_manager
        .save_results(session_id, &results)
        .await?;

    // Send completion message
    progress_tx.send(ServerMessage::AnalysisComplete {
        session_id,
        results: serde_json::to_value(&results)?,
        total_duration_ms: results.processing_time_ms,
        timestamp: chrono::Utc::now(),
    }).await?;

    Ok(())
}
```

---

## 5. LLM Integration

### 5.1 OpenRouter Configuration (Production)

**Provider**: OpenRouter (https://openrouter.ai)
**Primary Model**: `anthropic/claude-3.5-sonnet`
**Fallback Model**: `anthropic/claude-3.5-haiku`

**Cost Structure**:
- Claude 3.5 Sonnet: $3/1M input tokens, $15/1M output tokens
- Claude 3.5 Haiku: $0.25/1M input tokens, $1.25/1M output tokens

**Budget Management**:
- Per-analysis cost cap: $5
- Track token usage per agent
- Auto-switch to Haiku if budget exceeded

#### Role-Specific Model Selection

| Agent Role | Model | Rationale |
|------------|-------|-----------|
| BiasDetectorAgent | Sonnet | Complex pattern recognition |
| NarrativeMapperAgent | Sonnet | Stakeholder analysis requires nuance |
| TaxonomyLinkerAgent | Haiku | Taxonomy matching is straightforward |
| OmissionDetectorAgent | Sonnet | Critical - needs deep analysis |
| Debater (Both) | Sonnet | Argument quality is paramount |
| Pass1Evaluator | Haiku | Scoring is structured task |
| ExploitationDebater | Sonnet | Must maximize exploitation |
| CumulativeEvaluator | Sonnet | Strategic risk assessment critical |
| Response Agents | Haiku | Template-based generation |

#### Rig-Core Integration

```rust
// terraphim_truthforge/src/agents/omission_detector.rs

use rig::providers::openrouter::{OpenRouterClient, CompletionModel};
use terraphim_multi_agent::{TerraphimAgent, AgentConfig};

pub async fn create_omission_detector_agent(
    openrouter_api_key: String,
) -> Result<TerraphimAgent, anyhow::Error> {
    // Initialize OpenRouter client
    let client = OpenRouterClient::new(&openrouter_api_key);
    let model = CompletionModel::Claude3_5Sonnet;

    // Load role configuration
    let role_config = terraphim_config::Role::from_json(
        include_str!("../../config/roles/omission_detector_role.json")
    )?;

    // Create GenAiLlmClient wrapper
    let llm_client = terraphim_multi_agent::GenAiLlmClient::new(
        "openrouter".to_string(),
        model.to_string(),
        openrouter_api_key,
    )?;

    // Create TerraphimAgent
    let agent_config = AgentConfig {
        max_context_tokens: 32000,
        enable_token_tracking: true,
        enable_cost_tracking: true,
        default_timeout_ms: 30000,
        ..Default::default()
    };

    let agent = TerraphimAgent::new(
        role_config,
        Arc::new(llm_client),
        Some(agent_config),
    ).await?;

    Ok(agent)
}
```

### 5.2 Ollama Configuration (Testing)

**Provider**: Ollama (local)
**Model**: `gemma3:270m`
**Purpose**: Fast integration tests, no cost

```rust
// tests/integration_tests.rs

#[cfg(feature = "test-ollama")]
#[tokio::test]
async fn test_omission_detector_with_ollama() {
    use terraphim_multi_agent::test_utils::create_test_agent_simple;

    // Uses Ollama gemma3:270m by default
    let agent = create_test_agent_simple().await.unwrap();

    let narrative = "We reduced costs by 40%. This benefited shareholders.";
    let result = agent.process_narrative(narrative).await.unwrap();

    // Verify omissions detected
    assert!(result.omissions.len() >= 3);
    assert!(result.omissions.iter().any(|o|
        matches!(o.category, OmissionCategory::MissingEvidence)
    ));
}
```

### 5.3 Prompt Engineering

#### System Prompt Template (Omission Detector)

```
You are an expert at identifying gaps, missing context, and unstated assumptions in narratives.

For each narrative, systematically analyze:

1. **Missing Evidence**: Claims without supporting data, statistics, or sources
   - Look for quantitative claims lacking attribution
   - Identify assertions presented as fact without proof
   - Note vague language ("many", "significant") without specifics

2. **Unstated Assumptions**: Implied premises or beliefs not explicitly stated
   - What must be true for this narrative to make sense?
   - What values or priorities are taken for granted?
   - What counterfactual scenarios are ignored?

3. **Absent Stakeholder Voices**: Perspectives or groups not represented
   - Who is affected but not mentioned?
   - Whose interests are served vs. harmed?
   - What stakeholder groups are conspicuously silent?

4. **Context Gaps**: Background information, history, or circumstances omitted
   - What prior events led to this situation?
   - What industry/regulatory context is missing?
   - What comparisons to competitors or benchmarks are absent?

5. **Unaddressed Counterarguments**: Obvious rebuttals or alternative explanations ignored
   - What objections would skeptics raise?
   - What alternative interpretations exist?
   - What inconvenient facts are left out?

For each omission, provide:
- **Category**: (from list above)
- **Description**: What's missing (50-200 words)
- **Severity**: 0.0-1.0 (impact if omission is highlighted by opponents)
- **Exploitability**: 0.0-1.0 (ease with which adversaries can weaponize this gap)
- **Text Reference**: Specific quote from narrative that triggered detection
- **Confidence**: 0.0-1.0 (how certain are you this is truly an omission?)

Prioritize omissions by **composite risk** = severity × exploitability.
Return top 10 omissions in JSON format.
```

#### Dynamic Prompt Construction

```rust
pub fn build_omission_detector_prompt(
    narrative: &str,
    context: &NarrativeContext,
) -> String {
    let urgency_modifier = match context.urgency {
        UrgencyLevel::High => "\nIMPORTANT: This is a high-urgency crisis scenario. Prioritize omissions that opponents could exploit in next 24-48 hours.",
        UrgencyLevel::Low => "\nThis is strategic planning. Focus on systemic omissions that would emerge in sustained scrutiny.",
    };

    let stakes_modifier = if context.stakes.contains(&StakeType::Legal) {
        "\nPay special attention to missing legal context, regulatory compliance information, and unstated legal assumptions."
    } else {
        ""
    };

    format!(
        "{}\n{}{}\n\nNarrative to analyze:\n\n{}",
        SYSTEM_PROMPT,
        urgency_modifier,
        stakes_modifier,
        narrative
    )
}
```

---

## 6. Taxonomy Migration

### 6.1 TruthForge → RoleGraph Schema Mapping

**Source**: `/home/alex/projects/zestic-at/trueforge/truthforge-ai/assets/trueforge_taxonomy.json`
**Target**: `/home/alex/projects/terraphim/terraphim_truthforge/taxonomy/truthforge_rolegraph.json`

#### Migration Strategy

```rust
// terraphim_truthforge/src/taxonomy/rolegraph_migration.rs

use terraphim_rolegraph::{RoleGraph, Node, Edge, NodeType, EdgeType};
use serde_json::Value;

pub async fn migrate_truthforge_taxonomy() -> Result<RoleGraph, anyhow::Error> {
    // Load original taxonomy
    let taxonomy_json = include_str!("../../../taxonomy/source/trueforge_taxonomy.json");
    let taxonomy: Vec<Value> = serde_json::from_str(taxonomy_json)?;

    let mut rolegraph = RoleGraph::new("TruthForge Strategic Communication Taxonomy");

    for function in taxonomy {
        // Create function node
        let function_id = function["id"].as_str().unwrap();
        let function_node = Node::new(
            function_id.to_string(),
            function["name"].as_str().unwrap().to_string(),
            NodeType::Function,
        ).with_attributes(extract_attributes(&function));

        rolegraph.add_node(function_node)?;

        // Create subfunction nodes
        if let Some(subfunctions) = function["subfunctions"].as_array() {
            for subfunction in subfunctions {
                let subfunction_id = format!(
                    "{}_{}",
                    function_id,
                    subfunction["name"].as_str().unwrap()
                );

                let subfunction_node = Node::new(
                    subfunction_id.clone(),
                    subfunction["name"].as_str().unwrap().to_string(),
                    NodeType::Subfunction,
                ).with_attributes(extract_subfunction_attributes(subfunction));

                rolegraph.add_node(subfunction_node)?;

                // Create edge: function → subfunction
                let edge = Edge::new(
                    function_id.to_string(),
                    subfunction_id,
                    EdgeType::HasSubfunction,
                ).with_weight(1.0);

                rolegraph.add_edge(edge)?;
            }
        }

        // Create lifecycle stage nodes
        if let Some(lifecycle) = function["lifecycle"].as_array() {
            for stage in lifecycle {
                let stage_id = format!("{}_{}", function_id, stage.as_str().unwrap());

                let stage_node = Node::new(
                    stage_id.clone(),
                    stage.as_str().unwrap().to_string(),
                    NodeType::LifecycleStage,
                );

                rolegraph.add_node(stage_node)?;

                // Edge: function → lifecycle_stage
                let edge = Edge::new(
                    function_id.to_string(),
                    stage_id,
                    EdgeType::HasLifecycleStage,
                ).with_weight(1.0);

                rolegraph.add_edge(edge)?;
            }
        }

        // SCCT classifications (for issue_crisis_management)
        if function_id == "issue_crisis_management" {
            if let Some(classification) = function["classification"].as_object() {
                create_scct_nodes(&mut rolegraph, function_id, classification)?;
            }
        }
    }

    Ok(rolegraph)
}

fn create_scct_nodes(
    rolegraph: &mut RoleGraph,
    function_id: &str,
    classification: &serde_json::Map<String, Value>,
) -> Result<(), anyhow::Error> {
    // Create SCCT classification nodes
    if let Some(responsibility_types) = classification["responsibilityAttribution"].as_array() {
        for resp_type in responsibility_types {
            let node_id = format!("scct_{}", resp_type.as_str().unwrap());

            let node = Node::new(
                node_id.clone(),
                resp_type.as_str().unwrap().to_string(),
                NodeType::SCCTClassification,
            );

            rolegraph.add_node(node)?;

            // Edge: issue_crisis_management → SCCT type
            let edge = Edge::new(
                function_id.to_string(),
                node_id,
                EdgeType::HasSCCTClassification,
            ).with_weight(1.0);

            rolegraph.add_edge(edge)?;
        }
    }

    Ok(())
}
```

#### Resulting RoleGraph Structure

```json
{
  "name": "TruthForge Strategic Communication Taxonomy",
  "nodes": [
    {
      "id": "relationship_management",
      "label": "Relationship Management",
      "node_type": "function",
      "attributes": {
        "description": "Building and maintaining mutually beneficial organization–public relationships",
        "scope": ["external_stakeholders", "internal_stakeholders", "community"]
      }
    },
    {
      "id": "relationship_management_stakeholder_mapping",
      "label": "Stakeholder Mapping",
      "node_type": "subfunction",
      "attributes": {
        "outputs": ["stakeholder_register", "salience_matrix"]
      }
    },
    {
      "id": "issue_crisis_management",
      "label": "Issue & Crisis Management",
      "node_type": "function",
      "attributes": {
        "description": "Anticipation, preparation, and response to issues and crises"
      }
    },
    {
      "id": "issue_crisis_management_horizon_scanning",
      "label": "Horizon Scanning",
      "node_type": "subfunction",
      "attributes": {
        "outputs": ["issue_register", "weak_signal_reports"]
      }
    },
    {
      "id": "scct_victim",
      "label": "Victim",
      "node_type": "scct_classification",
      "attributes": {
        "responsibility_level": "low",
        "recommended_strategies": ["instructing_information", "bolstering_care"]
      }
    },
    {
      "id": "scct_accidental",
      "label": "Accidental",
      "node_type": "scct_classification",
      "attributes": {
        "responsibility_level": "medium",
        "recommended_strategies": ["apology_if_warranted", "corrective_action"]
      }
    },
    {
      "id": "scct_preventable",
      "label": "Preventable",
      "node_type": "scct_classification",
      "attributes": {
        "responsibility_level": "high",
        "recommended_strategies": ["full_apology", "compensation", "leadership_accountability"]
      }
    }
  ],
  "edges": [
    {
      "source": "relationship_management",
      "target": "relationship_management_stakeholder_mapping",
      "edge_type": "has_subfunction",
      "weight": 1.0
    },
    {
      "source": "issue_crisis_management",
      "target": "scct_victim",
      "edge_type": "has_scct_classification",
      "weight": 1.0
    }
  ]
}
```

### 6.2 Taxonomy Linker Integration

```rust
// terraphim_truthforge/src/agents/taxonomy_linker.rs

use terraphim_rolegraph::RoleGraph;

pub struct TaxonomyLinkerAgent {
    agent: TerraphimAgent,
    rolegraph: Arc<RoleGraph>,
}

impl TaxonomyLinkerAgent {
    pub async fn link_narrative_to_taxonomy(
        &self,
        narrative: &str,
        bias_analysis: &BiasAnalysis,
        narrative_mapping: &NarrativeMapping,
    ) -> Result<TaxonomyLinking, anyhow::Error> {
        // Use SCCT classification to determine primary function
        let primary_function = match narrative_mapping.scct_classification {
            SCCTClassification::Victim |
            SCCTClassification::Accidental |
            SCCTClassification::Preventable => "issue_crisis_management",
            _ => "relationship_management",  // Fallback
        };

        // Query rolegraph for applicable subfunctions
        let subfunctions = self.rolegraph
            .get_subfunctions(primary_function)?
            .into_iter()
            .filter(|sf| self.is_applicable(sf, narrative, bias_analysis))
            .map(|sf| sf.label)
            .collect();

        // Determine lifecycle stage based on context
        let lifecycle_stage = self.infer_lifecycle_stage(narrative_mapping)?;

        // Select recommended playbooks based on SCCT + subfunctions
        let recommended_playbooks = self.select_playbooks(
            primary_function,
            &narrative_mapping.scct_classification,
            &subfunctions,
        )?;

        Ok(TaxonomyLinking {
            primary_function: primary_function.to_string(),
            secondary_functions: vec![],  // Could add based on complexity
            subfunctions,
            lifecycle_stage,
            recommended_playbooks,
        })
    }

    fn select_playbooks(
        &self,
        function: &str,
        scct: &SCCTClassification,
        subfunctions: &[String],
    ) -> Result<Vec<String>, anyhow::Error> {
        let scct_node_id = format!("scct_{}", scct.to_string().to_lowercase());
        let scct_node = self.rolegraph.get_node(&scct_node_id)?;

        // Get recommended strategies from SCCT node attributes
        let strategies = scct_node.attributes
            .get("recommended_strategies")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect())
            .unwrap_or_default();

        // Combine with subfunction-specific playbooks
        let mut playbooks = strategies;
        for subfunction in subfunctions {
            if let Ok(sf_node) = self.rolegraph.get_node(subfunction) {
                if let Some(outputs) = sf_node.attributes.get("outputs") {
                    if let Some(arr) = outputs.as_array() {
                        playbooks.extend(
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        );
                    }
                }
            }
        }

        Ok(playbooks)
    }
}
```

---

## 7. Workflow Implementation

### 7.1 Two-Pass Debate Workflow

```rust
// terraphim_truthforge/src/workflows/two_pass_debate.rs

use terraphim_multi_agent::{
    TerraphimAgent, MultiAgentWorkflow, workflows::*,
};
use tokio::sync::mpsc;
use std::sync::Arc;

pub struct TwoPassDebateWorkflow {
    pass_one: PassOneOrchestrator,
    pass_two: PassTwoOptimizer,
    response_generator: ResponseParallelizer,
    progress_tx: mpsc::Sender<ServerMessage>,
}

impl TwoPassDebateWorkflow {
    pub fn new(
        agent_pool: Arc<AgentPool>,
        progress_callback: impl Fn(ServerMessage) + Send + Sync + 'static,
    ) -> Self {
        let (progress_tx, mut progress_rx) = mpsc::channel(100);

        // Spawn progress forwarder
        tokio::spawn(async move {
            while let Some(msg) = progress_rx.recv().await {
                progress_callback(msg);
            }
        });

        Self {
            pass_one: PassOneOrchestrator::new(agent_pool.clone()),
            pass_two: PassTwoOptimizer::new(agent_pool.clone()),
            response_generator: ResponseParallelizer::new(agent_pool),
            progress_tx,
        }
    }

    pub async fn execute(
        &self,
        narrative: &NarrativeInput,
    ) -> Result<TruthForgeAnalysisResult, anyhow::Error> {
        let start_time = std::time::Instant::now();

        // Phase 1: Pass One (Orchestrator-Workers)
        self.send_progress("analysis", "BiasDetectorAgent", 0.0).await;
        let pass_one_result = self.pass_one.execute(narrative, self.progress_tx.clone()).await?;

        // Phase 2: Pass Two (Evaluator-Optimizer)
        self.send_progress("debate_pass2", "ExploitationDebater", 0.0).await;
        let pass_two_result = self.pass_two.execute(
            narrative,
            &pass_one_result.omission_catalog,
            self.progress_tx.clone(),
        ).await?;

        // Phase 3: Response Generation (Parallelization)
        self.send_progress("response", "ReframeAgent", 0.0).await;
        let response_strategies = self.response_generator.execute(
            narrative,
            &pass_two_result.cumulative_analysis,
            self.progress_tx.clone(),
        ).await?;

        // Generate executive summary
        let executive_summary = self.generate_executive_summary(
            &pass_one_result,
            &pass_two_result,
            &response_strategies,
        );

        Ok(TruthForgeAnalysisResult {
            session_id: narrative.session_id,
            narrative: narrative.clone(),
            bias_analysis: pass_one_result.bias_analysis,
            narrative_mapping: pass_one_result.narrative_mapping,
            taxonomy_linking: pass_one_result.taxonomy_linking,
            omission_catalog: pass_one_result.omission_catalog,
            pass_one_debate: pass_one_result.debate_result,
            pass_two_debate: pass_two_result.debate_result,
            cumulative_analysis: pass_two_result.cumulative_analysis,
            response_strategies,
            executive_summary,
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            created_at: chrono::Utc::now(),
        })
    }

    async fn send_progress(&self, phase: &str, agent: &str, progress: f64) {
        let msg = ServerMessage::AgentProgress {
            session_id: Uuid::new_v4(),  // Set by caller
            agent: agent.to_string(),
            phase: phase.to_string(),
            status: "running".to_string(),
            progress,
            estimated_remaining_ms: 0,  // Calculated dynamically
            timestamp: chrono::Utc::now(),
        };

        self.progress_tx.send(msg).await.ok();
    }
}
```

### 7.2 Pass One Orchestrator

```rust
// terraphim_truthforge/src/workflows/pass_one.rs

pub struct PassOneOrchestrator {
    bias_detector: Arc<TerraphimAgent>,
    narrative_mapper: Arc<TerraphimAgent>,
    taxonomy_linker: Arc<TerraphimAgent>,
    omission_detector: Arc<TerraphimAgent>,
    debater_supporting: Arc<TerraphimAgent>,
    debater_opposing: Arc<TerraphimAgent>,
    evaluator: Arc<TerraphimAgent>,
}

impl PassOneOrchestrator {
    pub async fn execute(
        &self,
        narrative: &NarrativeInput,
        progress_tx: mpsc::Sender<ServerMessage>,
    ) -> Result<PassOneResult, anyhow::Error> {
        // Step 1: Parallel analysis (Orchestrator-Workers pattern)
        let (bias, narrative_map, taxonomy, omissions) = tokio::join!(
            self.bias_detector.analyze(&narrative.text),
            self.narrative_mapper.analyze(&narrative.text),
            self.taxonomy_linker.analyze(&narrative.text),
            self.omission_detector.analyze(&narrative.text),
        );

        let bias_analysis = bias?;
        let narrative_mapping = narrative_map?;
        let taxonomy_linking = taxonomy?;
        let omission_catalog = omissions?;

        // Send progress
        progress_tx.send(ServerMessage::PhaseComplete {
            session_id: narrative.session_id,
            phase: "analysis".to_string(),
            results: serde_json::json!({
                "bias_analysis": bias_analysis,
                "narrative_mapping": narrative_mapping,
                "taxonomy_linking": taxonomy_linking,
                "omission_catalog": omission_catalog,
            }),
            duration_ms: 0,  // Calculate
            timestamp: chrono::Utc::now(),
        }).await?;

        // Step 2: Debate with omission awareness
        let debate_context = DebateContext {
            narrative: narrative.text.clone(),
            bias_analysis: bias_analysis.clone(),
            narrative_mapping: narrative_mapping.clone(),
            omission_catalog: omission_catalog.clone(),
        };

        let (supporting_arg, opposing_arg) = tokio::join!(
            self.debater_supporting.debate(&debate_context),
            self.debater_opposing.debate(&debate_context),
        );

        let supporting_argument = supporting_arg?;
        let opposing_argument = opposing_arg?;

        // Step 3: Evaluate debate
        let evaluation = self.evaluator.evaluate(
            &supporting_argument,
            &opposing_argument,
            &omission_catalog,
        ).await?;

        // Create prioritized omission list for Pass 2
        let prioritized_omissions = omission_catalog.omissions
            .iter()
            .sorted_by(|a, b| b.composite_risk.partial_cmp(&a.composite_risk).unwrap())
            .take(10)
            .map(|o| o.id)
            .collect();

        Ok(PassOneResult {
            bias_analysis,
            narrative_mapping,
            taxonomy_linking,
            omission_catalog: OmissionCatalog {
                prioritized: prioritized_omissions,
                ..omission_catalog
            },
            debate_result: DebateResult {
                pass: DebatePass::PassOne,
                supporting_argument,
                opposing_argument,
                evaluation,
            },
        })
    }
}
```

### 7.3 Pass Two Optimizer

```rust
// terraphim_truthforge/src/workflows/pass_two.rs

pub struct PassTwoOptimizer {
    exploitation_debater_supporting: Arc<TerraphimAgent>,
    exploitation_debater_opposing: Arc<TerraphimAgent>,
    cumulative_evaluator: Arc<TerraphimAgent>,
}

impl PassTwoOptimizer {
    pub async fn execute(
        &self,
        narrative: &NarrativeInput,
        omission_catalog: &OmissionCatalog,
        progress_tx: mpsc::Sender<ServerMessage>,
    ) -> Result<PassTwoResult, anyhow::Error> {
        // Get top 10 prioritized omissions
        let top_omissions: Vec<_> = omission_catalog.omissions
            .iter()
            .filter(|o| omission_catalog.prioritized.contains(&o.id))
            .cloned()
            .collect();

        // Create exploitation context
        let exploitation_context = ExploitationContext {
            narrative: narrative.text.clone(),
            target_omissions: top_omissions,
            directive: "Maximize vulnerability exploitation using identified gaps".to_string(),
        };

        // Run exploitation debate (Evaluator-Optimizer pattern)
        let (supporting_arg, opposing_arg) = tokio::join!(
            self.exploitation_debater_supporting.exploit(&exploitation_context),
            self.exploitation_debater_opposing.exploit(&exploitation_context),
        );

        let supporting_argument = supporting_arg?;
        let opposing_argument = opposing_arg?;

        // Cumulative evaluation (tracks amplification vs Pass 1)
        let cumulative_analysis = self.cumulative_evaluator.evaluate_cumulative(
            &pass_one_debate,  // Need to pass from workflow
            &supporting_argument,
            &opposing_argument,
        ).await?;

        Ok(PassTwoResult {
            debate_result: DebateResult {
                pass: DebatePass::PassTwo,
                supporting_argument,
                opposing_argument,
                evaluation: cumulative_analysis.evaluation.clone(),
            },
            cumulative_analysis,
        })
    }
}
```

---

## 8. Testing Strategy

### 8.1 Unit Tests (Ollama)

```rust
// crates/terraphim_truthforge/tests/workflow_tests.rs

#[cfg(feature = "test-ollama")]
mod ollama_tests {
    use terraphim_truthforge::*;
    use terraphim_multi_agent::test_utils::*;

    #[tokio::test]
    async fn test_omission_detector_finds_gaps() {
        let agent = create_omission_detector_test_agent().await.unwrap();

        let narrative = "We reduced costs by 40%. Shareholders benefited greatly.";
        let result = agent.analyze(narrative).await.unwrap();

        // Should detect missing evidence
        assert!(result.omissions.len() >= 2);

        let has_evidence_gap = result.omissions.iter().any(|o|
            matches!(o.category, OmissionCategory::MissingEvidence)
        );
        assert!(has_evidence_gap, "Should detect missing evidence for '40%' claim");

        // Should have high exploitability
        let max_exploitability = result.omissions.iter()
            .map(|o| o.exploitability)
            .fold(0.0, f64::max);
        assert!(max_exploitability > 0.7, "Cost claim should be highly exploitable");
    }

    #[tokio::test]
    async fn test_pass_two_references_pass_one_omissions() {
        let workflow = create_test_workflow_with_ollama().await.unwrap();

        let narrative = NarrativeInput {
            session_id: Uuid::new_v4(),
            text: "Our company achieved record profits while maintaining our commitment to sustainability.".to_string(),
            context: NarrativeContext {
                urgency: UrgencyLevel::High,
                stakes: vec![StakeType::Reputational],
                audience: AudienceType::PublicMedia,
            },
            submitted_at: chrono::Utc::now(),
        };

        let result = workflow.execute(&narrative).await.unwrap();

        // Pass 1 should identify omissions
        assert!(result.omission_catalog.omissions.len() >= 5);

        // Pass 2 should reference Pass 1 omissions
        let pass2_omission_refs = result.pass_two_debate.opposing_argument.omissions_referenced.len();
        let pass1_top_10 = result.omission_catalog.prioritized.len();

        let reference_percentage = pass2_omission_refs as f64 / pass1_top_10 as f64;
        assert!(reference_percentage >= 0.8,
            "Pass 2 should reference ≥80% of top 10 omissions, got {:.1%}",
            reference_percentage
        );
    }
}
```

### 8.2 Integration Tests (Redis + Ollama)

```rust
// crates/terraphim_truthforge/tests/integration_tests.rs

#[tokio::test]
async fn test_end_to_end_workflow_with_persistence() {
    // Start Redis test container
    let redis_container = start_redis_test_container().await;
    let redis_url = redis_container.url();

    // Create server with test config
    let server = TruthForgeServer::new(TestConfig {
        redis_url,
        llm_provider: "ollama".to_string(),
        ..Default::default()
    }).await.unwrap();

    // Submit narrative via REST API
    let client = reqwest::Client::new();
    let response = client.post("http://localhost:8080/api/v1/analysis")
        .json(&serde_json::json!({
            "narrative": {
                "text": "Test narrative...",
                "context": {
                    "urgency": "High",
                    "stakes": ["Reputational"],
                    "audience": "PublicMedia"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 202);
    let session: SessionResponse = response.json().await.unwrap();

    // Wait for completion (or use WebSocket in real scenario)
    tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;

    // Retrieve results
    let results_response = client.get(format!(
        "http://localhost:8080/api/v1/session/{}",
        session.session_id
    ))
    .send()
    .await
    .unwrap();

    assert_eq!(results_response.status(), 200);
    let results: TruthForgeAnalysisResult = results_response.json().await.unwrap();

    // Verify results persisted in Redis
    let mut redis_conn = redis::Client::open(redis_url).unwrap().get_connection().unwrap();
    let key = format!("results:{}", session.session_id);
    let exists: bool = redis::cmd("EXISTS").arg(&key).query(&mut redis_conn).unwrap();
    assert!(exists, "Results should be persisted in Redis");

    // Cleanup
    redis_container.stop().await;
}
```

### 8.3 Performance Tests

```rust
#[tokio::test]
#[ignore]  // Run with --ignored flag
async fn test_workflow_performance_target() {
    let workflow = create_production_workflow().await.unwrap();  // Uses OpenRouter

    let narrative = create_realistic_test_narrative();

    let start = std::time::Instant::now();
    let result = workflow.execute(&narrative).await.unwrap();
    let duration = start.elapsed();

    // Performance targets
    assert!(duration.as_secs() < 60,
        "Workflow should complete in <60s, took {:?}",
        duration
    );

    assert!(result.processing_time_ms < 45000,
        "Target is <45s, actual: {}ms",
        result.processing_time_ms
    );

    // Check individual phase timings
    println!("Performance breakdown:");
    println!("  Total: {}ms", result.processing_time_ms);
    // Add phase-specific timing checks
}
```

---

## 9. Deployment

### 9.1 Docker Configuration

```dockerfile
# deploy/Dockerfile

# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build release
RUN cargo build --release --bin truthforge-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 truthforge

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/truthforge-server ./
COPY --from=builder /app/crates/terraphim_truthforge_server/static ./static/
COPY --from=builder /app/taxonomy ./taxonomy/

# Set ownership
RUN chown -R truthforge:truthforge /app

USER truthforge

EXPOSE 8080

CMD ["./truthforge-server"]
```

### 9.2 Kubernetes Deployment

```yaml
# deploy/k8s/deployment.yaml

apiVersion: apps/v1
kind: Deployment
metadata:
  name: truthforge-server
  namespace: truthforge
spec:
  replicas: 3
  selector:
    matchLabels:
      app: truthforge-server
  template:
    metadata:
      labels:
        app: truthforge-server
    spec:
      containers:
      - name: server
        image: ghcr.io/zestic-ai/truthforge-server:latest
        ports:
        - containerPort: 8080
          name: http
        env:
        - name: REDIS_URL
          value: "redis://truthforge-redis:6379"
        - name: OPENROUTER_API_KEY
          valueFrom:
            secretKeyRef:
              name: truthforge-secrets
              key: openrouter-api-key
        - name: RUST_LOG
          value: "info,truthforge=debug"
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
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
```

```yaml
# deploy/k8s/service.yaml

apiVersion: v1
kind: Service
metadata:
  name: truthforge-server
  namespace: truthforge
spec:
  type: LoadBalancer
  ports:
  - port: 443
    targetPort: 8080
    protocol: TCP
    name: https
  selector:
    app: truthforge-server
```

---

## 10. Security Considerations

### 10.1 Prompt Injection Prevention

```rust
// Use existing terraphim_multi_agent::prompt_sanitizer

use terraphim_multi_agent::sanitize_system_prompt;

pub fn prepare_narrative_for_analysis(raw_narrative: &str) -> Result<String, anyhow::Error> {
    // Sanitize to prevent prompt injection
    let sanitized = sanitize_system_prompt(raw_narrative);

    if sanitized.was_modified {
        tracing::warn!(
            "Narrative sanitized for security: {:?}",
            sanitized.patterns_detected
        );
    }

    // Additional TruthForge-specific checks
    if sanitized.output.len() > 10_000 {
        return Err(anyhow::anyhow!("Narrative exceeds 10,000 character limit"));
    }

    Ok(sanitized.output)
}
```

### 10.2 PII Redaction

```rust
pub fn redact_pii_before_vault_storage(result: &TruthForgeAnalysisResult) -> RedactedResult {
    // Use regex to detect common PII patterns
    let email_pattern = regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    let phone_pattern = regex::Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap();

    let mut redacted_narrative = result.narrative.text.clone();
    redacted_narrative = email_pattern.replace_all(&redacted_narrative, "[EMAIL_REDACTED]").to_string();
    redacted_narrative = phone_pattern.replace_all(&redacted_narrative, "[PHONE_REDACTED]").to_string();

    RedactedResult {
        narrative: redacted_narrative,
        omissions: result.omission_catalog.clone(),  // Safe - no PII
        scct_classification: result.narrative_mapping.scct_classification.clone(),
        ..Default::default()
    }
}
```

### 10.3 Rate Limiting

```rust
// terraphim_truthforge_server/src/middleware/rate_limit.rs

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, RateLimit>>>,
    requests_per_hour: usize,
}

#[derive(Debug, Clone)]
struct RateLimit {
    count: usize,
    window_start: chrono::DateTime<chrono::Utc>,
}

impl RateLimiter {
    pub fn new(requests_per_hour: usize) -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            requests_per_hour,
        }
    }

    pub async fn check_rate_limit(&self, user_id: &str) -> Result<(), StatusCode> {
        let mut limits = self.limits.write().await;
        let now = chrono::Utc::now();

        let limit = limits.entry(user_id.to_string())
            .or_insert(RateLimit {
                count: 0,
                window_start: now,
            });

        // Reset if window expired
        if (now - limit.window_start).num_hours() >= 1 {
            limit.count = 0;
            limit.window_start = now;
        }

        if limit.count >= self.requests_per_hour {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        limit.count += 1;
        Ok(())
    }
}

pub async fn rate_limit_middleware<B>(
    State(rate_limiter): State<Arc<RateLimiter>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Extract user ID from auth header
    let user_id = extract_user_id(&request)?;

    rate_limiter.check_rate_limit(&user_id).await?;

    Ok(next.run(request).await)
}
```

---

## 11. Monitoring & Observability

### 11.1 Structured Logging

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(narrative))]
pub async fn execute_workflow(narrative: &NarrativeInput) -> Result<TruthForgeAnalysisResult> {
    info!(
        session_id = %narrative.session_id,
        urgency = ?narrative.context.urgency,
        "Starting two-pass debate workflow"
    );

    let start = std::time::Instant::now();

    // ... workflow execution ...

    info!(
        session_id = %narrative.session_id,
        duration_ms = start.elapsed().as_millis(),
        "Workflow completed successfully"
    );

    Ok(result)
}
```

### 11.2 Metrics (Prometheus)

```rust
use prometheus::{IntCounter, Histogram, Registry};

lazy_static! {
    static ref WORKFLOW_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "truthforge_workflow_duration_seconds",
            "Time to complete full workflow"
        )
    ).unwrap();

    static ref OMISSIONS_DETECTED: IntCounter = IntCounter::new(
        "truthforge_omissions_detected_total",
        "Total omissions detected across all analyses"
    ).unwrap();

    static ref PASS2_EXPLOITATION_RATE: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "truthforge_pass2_exploitation_rate",
            "Percentage of Pass 1 omissions exploited in Pass 2"
        )
    ).unwrap();
}

// In workflow:
let timer = WORKFLOW_DURATION.start_timer();
let result = execute_workflow(narrative).await?;
timer.observe_duration();

OMISSIONS_DETECTED.inc_by(result.omission_catalog.omissions.len() as u64);

let exploitation_rate = calculate_exploitation_rate(&result);
PASS2_EXPLOITATION_RATE.observe(exploitation_rate);
```

---

## 12. Appendices

### Appendix A: Environment Configuration

```toml
# .env.example

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
RUST_LOG=info,truthforge=debug

# Redis
REDIS_URL=redis://localhost:6379
REDIS_POOL_SIZE=10
REDIS_TIMEOUT_MS=5000

# LLM Providers
OPENROUTER_API_KEY=sk-or-v1-xxxxx
OLLAMA_BASE_URL=http://localhost:11434

# Security
OAUTH2_CLIENT_ID=xxxxx
OAUTH2_CLIENT_SECRET=xxxxx
JWT_SECRET=xxxxx

# Rate Limiting
RATE_LIMIT_REQUESTS_PER_HOUR=100

# Cost Management
MAX_COST_PER_ANALYSIS_USD=5.00

# Telemetry
PROMETHEUS_ENDPOINT=0.0.0.0:9090
ENABLE_TRACING=true
```

### Appendix B: Role Configuration Template

```json
// config/roles/template_role.json
{
  "name": "Template Agent Role",
  "shortname": "template_agent",
  "relevance_function": "BM25Plus",
  "extra": {
    "llm_provider": "openrouter",
    "llm_model": "anthropic/claude-3.5-sonnet",
    "system_prompt": "You are an expert in...",
    "agent_type": "template_type",
    "quality_criteria": ["criterion1", "criterion2"],
    "taxonomy_mapping": "function.subfunction",
    "max_tokens": 2000,
    "temperature": 0.3
  },
  "haystacks": []
}
```

---

**Document Control**
**Created**: 2025-10-07
**Last Modified**: 2025-10-07
**Version**: 1.0
**Classification**: Internal - Zestic AI Confidential
