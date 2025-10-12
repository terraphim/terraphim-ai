# Architecture Decision Records: TruthForge Terraphim Patterns

**Version**: 1.0
**Date**: 2025-10-07
**Status**: Draft
**Owner**: Zestic AI / K-Partners
**Related Documents**:
- [PRD_TwoPassDebateArena.md](./PRD_TwoPassDebateArena.md)
- [SPEC_TerraphimIntegration.md](./SPEC_TerraphimIntegration.md)
- [REQUIREMENTS_AgentRoles.md](./REQUIREMENTS_AgentRoles.md)
- [ROADMAP_Implementation.md](./ROADMAP_Implementation.md)

---

## Overview

This document contains Architecture Decision Records (ADRs) for the TruthForge Two-Pass Debate Arena implementation using the Terraphim-AI multi-agent framework. Each ADR captures the context, decision, rationale, and consequences of key architectural choices.

---

## ADR-001: Private Repository Strategy

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: Technical Lead, Product Owner
**Context**: TruthForge contains proprietary strategic communication taxonomy and business logic that should not be open-sourced, but relies on the public terraphim-ai framework.

### Decision

Create a **private repository** `terraphim_truthforge` that depends on the public `terraphim-ai` repository as an external dependency.

**Repository Structure**:
```
terraphim_truthforge/ (private)
├── Cargo.toml (workspace)
├── crates/
│   ├── terraphim_truthforge/       # Core library
│   └── terraphim_truthforge_server/ # WebSocket server
├── taxonomy/
│   └── truthforge_rolegraph.json   # Proprietary taxonomy
└── ui/
    └── index.html                   # UI implementation

Dependencies:
[dependencies]
terraphim-multi-agent = { git = "https://github.com/terraphim/terraphim-ai", branch = "main" }
```

### Rationale

**Pros**:
- Protects proprietary strategic communication taxonomy and agent prompts
- Maintains clean separation between open-source framework (terraphim-ai) and closed-source business logic
- Enables independent versioning and release cycles
- Allows integration of terraphim-ai improvements without exposing TruthForge IP
- Supports potential future commercialization

**Cons**:
- Duplicate CI/CD setup (separate from terraphim-ai)
- Dependency management complexity (tracking terraphim-ai changes)
- Cannot contribute TruthForge-specific improvements back to terraphim-ai without abstraction

**Alternatives Considered**:
1. **Fork terraphim-ai** - Rejected: Creates maintenance burden, loses upstream improvements
2. **Private branch in terraphim-ai** - Rejected: Violates terraphim-ai's open-source license intent
3. **Monorepo with private submodule** - Rejected: Git submodule complexity, unclear IP boundaries

### Consequences

**Positive**:
- Clear IP ownership for Zestic AI/K-Partners
- Flexibility to use commercial LLM providers (OpenRouter) without licensing concerns
- Ability to white-label or license TruthForge separately

**Negative**:
- Must manually sync with terraphim-ai API changes
- Cannot easily share TruthForge innovations with terraphim community
- Increased testing burden (must test against terraphim-ai version compatibility)

**Mitigation**:
- Use exact terraphim-ai commit SHAs in Cargo.toml (not `branch = "main"`) for reproducible builds
- Subscribe to terraphim-ai release notifications
- Contribute generic improvements (e.g., WebSocket infrastructure) back to terraphim-ai as separate PRs

---

## ADR-002: Workflow Pattern Combination

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: Technical Lead, AI Engineer
**Context**: TruthForge requires a novel two-pass debate workflow that doesn't match any single terraphim-ai pattern exactly. Need to decide how to combine existing patterns.

### Decision

Implement a **hybrid workflow** combining three terraphim-ai patterns:

1. **Pass 1**: Orchestrator-Workers Pattern (Pattern #4)
   - Orchestrator: Pass1Workflow
   - Workers: BiasDetector, NarrativeMapper, TaxonomyLinker, OmissionDetector
   - Parallel Debate: Pass1DebaterPro, Pass1DebaterCon (Parallelization Pattern #3)
   - Evaluator: Pass1EvaluatorAgent

2. **Pass 2**: Evaluator-Optimizer Pattern (Pattern #5)
   - Generator: Pass2DebaterExploitation (using Pass 1 omissions)
   - Evaluator: Pass2DebaterDefense
   - Optimizer: CumulativeEvaluatorAgent

3. **Response Generation**: Parallelization Pattern (Pattern #3)
   - Parallel execution: ReframeAgent, CounterArgueAgent, BridgeAgent

**Workflow Diagram**:
```
Input Text
    ↓
[Pass 1: Orchestrator-Workers]
    ├─→ BiasDetector ──────┐
    ├─→ NarrativeMapper ───┤
    ├─→ TaxonomyLinker ────┤→ Aggregated Context
    └─→ OmissionDetector ──┘         ↓
         ↓                    [Parallel Debate]
    Omissions List          Pro ←──→ Con
         ↓                           ↓
    Pass1Evaluator → Vulnerability Assessment
         ↓
[Pass 2: Evaluator-Optimizer]
    Exploitation Attack (uses vulnerabilities)
         ↓
    Defense Response (uses SCCT strategy)
         ↓
    CumulativeEvaluator → Residual Risk
         ↓
[Response Generation: Parallelization]
    ├─→ ReframeAgent ──────┐
    ├─→ CounterArgueAgent ─┤→ ResponseSet
    └─→ BridgeAgent ───────┘
```

### Rationale

**Pros**:
- Leverages proven terraphim-ai patterns rather than creating entirely new orchestration
- Orchestrator-Workers provides structured analysis phase
- Evaluator-Optimizer enables iterative improvement (Pass 2 builds on Pass 1)
- Parallelization maximizes throughput for debate and response generation
- Modular design allows independent testing of each pattern

**Cons**:
- Complexity from combining 3 patterns
- No existing terraphim-ai example of this specific combination
- Requires custom orchestration logic to pass data between patterns

**Alternatives Considered**:
1. **Single sequential chain (Prompt Chaining Pattern #1)** - Rejected: Too slow, no parallel debate
2. **Pure Routing Pattern (#2)** - Rejected: Doesn't support iterative Pass 2 exploitation
3. **Custom pattern** - Rejected: Reinventing orchestration logic, harder to maintain

### Consequences

**Positive**:
- Reuses battle-tested terraphim-ai orchestration code
- Clear separation of concerns (analysis → debate → exploitation → response)
- Easy to add new agents within existing pattern structure
- Familiar to terraphim-ai developers

**Negative**:
- Complex inter-pattern data passing (Pass 1 results → Pass 2)
- Debugging requires understanding 3 different patterns
- Performance profiling must account for pattern boundaries

**Mitigation**:
- Create `TwoPassWorkflow` wrapper that encapsulates pattern orchestration
- Comprehensive integration tests for pattern transitions
- Detailed logging at pattern boundaries

---

## ADR-003: Taxonomy Migration Strategy

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: Technical Lead, Backend Engineer
**Context**: TruthForge has existing JSON taxonomy (3 functions, 15 subfunctions) that must be migrated to terraphim-ai's RoleGraph format for knowledge graph integration.

### Decision

Implement a **one-time migration function** that converts `trueforge_taxonomy.json` to `RoleGraph` with:

**Node Structure**:
- **Function nodes**: Top-level strategic communication functions (relationship_management, issue_crisis_management, strategic_management_function)
- **Subfunction nodes**: Specific capabilities (risk_assessment, stakeholder_mapping, etc.)
- **Output nodes**: Deliverables (SCCT_response_matrix, stakeholder_register, etc.)
- **SCCT nodes**: Crisis classifications (victim, accidental, preventable)

**Edge Structure**:
- Function → Subfunction (weight: 1.0)
- Subfunction → Output (weight: 1.0)
- Function → SCCT classification (weight: 1.0)

**Implementation**:
```rust
pub async fn migrate_truthforge_taxonomy(json_path: &Path) -> Result<RoleGraph> {
    let functions: Vec<TaxonomyFunction> = parse_json(json_path)?;
    let mut graph = RoleGraph::new();

    for func in functions {
        let func_node = create_node(&func.id);
        graph.add_node(func_node.clone());

        for subf in func.subfunctions {
            let subf_node = create_node(&format!("{}.{}", func.id, subf.name));
            graph.add_node(subf_node.clone());
            graph.add_edge(&func_node, &subf_node, 1.0)?;

            for output in subf.outputs {
                let output_node = create_node(&output);
                graph.add_node(output_node.clone());
                graph.add_edge(&subf_node, &output_node, 1.0)?;
            }
        }
    }

    Ok(graph)
}
```

### Rationale

**Pros**:
- Preserves existing taxonomy structure and semantics
- Enables TaxonomyLinkerAgent to use graph traversal for classification
- Supports future taxonomy expansion (add nodes/edges without changing structure)
- Integrates with terraphim-ai's automata system for fast text matching
- Validates graph connectivity (all nodes reachable from functions)

**Cons**:
- One-time migration effort required
- JSON and RoleGraph must be kept in sync if taxonomy changes
- Graph may be overkill for only 3 functions (could use simple enum)

**Alternatives Considered**:
1. **Keep JSON, skip RoleGraph** - Rejected: Loses terraphim-ai graph integration, no automata support
2. **Manual RoleGraph creation** - Rejected: Error-prone, not reproducible
3. **Dual format (JSON + RoleGraph)** - Rejected: Synchronization burden

### Consequences

**Positive**:
- TaxonomyLinkerAgent can leverage graph algorithms (shortest path, node similarity)
- Future taxonomy changes are easy to migrate (re-run migration function)
- Graph enables visualization of taxonomy relationships
- Automata support for fast "Does this text relate to crisis management?" queries

**Negative**:
- Migration must be tested thoroughly (validate node count, edge count, connectivity)
- RoleGraph adds memory overhead vs. simple JSON
- Must document migration process for future taxonomy updates

**Mitigation**:
- Comprehensive migration tests (verify all 3 functions, 15 subfunctions migrated)
- Document taxonomy update process (edit JSON → re-run migration → commit RoleGraph)
- Create visualization tool for RoleGraph (aid debugging)

---

## ADR-004: WebSocket vs. REST for Real-Time Updates

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: Backend Engineer, Frontend Engineer
**Context**: TruthForge workflow takes 60-90 seconds. Users need real-time progress updates for good UX. Must decide communication protocol.

### Decision

Use **dual protocol architecture**:

1. **REST API** for control operations:
   - `POST /api/v1/analysis` - Start analysis workflow
   - `GET /api/v1/session/{id}` - Retrieve session status
   - `GET /api/v1/results/{id}` - Retrieve final results

2. **WebSocket** for real-time progress updates:
   - Client subscribes to session via WebSocket connection
   - Server pushes progress events as agents complete
   - 6 message types: AgentProgress, AgentResult, PhaseComplete, AnalysisComplete, Error, Pong

**Architecture**:
```
Client (Browser)
    ├─→ REST POST /analysis → Create session
    ├─→ WebSocket /ws → Subscribe to updates
    └─→ Receive progress events in real-time

Server
    ├─→ REST Handler → Accept request, start workflow, return session_id
    ├─→ Workflow Engine → Execute agents, emit progress events
    └─→ WebSocket Handler → Broadcast events to subscribed clients
```

**WebSocket Message Protocol**:
```typescript
type WsMessage =
  | { type: 'AgentProgress', agent: string, progress: number, message: string }
  | { type: 'AgentResult', agent: string, result: any }
  | { type: 'PhaseComplete', phase: string, results: any }
  | { type: 'AnalysisComplete', final_results: FinalResults }
  | { type: 'Error', error: string }
  | { type: 'Pong' }
```

### Rationale

**Pros**:
- REST is simple for control operations (start/stop/retrieve)
- WebSocket provides true real-time updates without polling
- Separates concerns: REST for commands, WebSocket for events
- WebSocket is efficient (low latency, low bandwidth vs. polling)
- Standard browser API support (WebSocket)

**Cons**:
- Two protocols to implement and maintain
- WebSocket adds complexity (connection management, reconnection)
- Horizontal scaling requires sticky sessions or Redis pub/sub

**Alternatives Considered**:
1. **REST polling** - Rejected: High latency (poll every 1-2s), wasteful (95% of requests get "no update")
2. **Server-Sent Events (SSE)** - Rejected: One-way only, less browser support than WebSocket
3. **gRPC streaming** - Rejected: Poor browser support, requires protobuf
4. **GraphQL subscriptions** - Rejected: Overkill for simple progress updates

### Consequences

**Positive**:
- Excellent UX (progress bar updates in <100ms)
- Efficient resource usage (no polling overhead)
- Supports future features (live collaboration, agent chat)
- Standard protocol (easy client libraries)

**Negative**:
- Must handle WebSocket disconnections (auto-reconnect logic)
- Horizontal scaling requires Redis pub/sub for broadcasting
- Testing is more complex (must test WebSocket lifecycle)

**Mitigation**:
- Client implements auto-reconnect with exponential backoff
- Use Redis pub/sub for multi-server broadcasting (Phase 3)
- Comprehensive WebSocket tests (connection, disconnect, message ordering)
- Heartbeat pings (every 30s) to keep connections alive

---

## ADR-005: Redis Persistence Design

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: Backend Engineer, SRE Engineer
**Context**: TruthForge needs session management, results caching, and learning vault. Must choose persistence strategy balancing speed, cost, and complexity.

### Decision

Use **Redis with tiered TTL strategy**:

**Data Categories**:
1. **Sessions** (`session:{uuid}`) - TTL: 24 hours
   - Active workflow state, progress tracking
   - Expire after 1 day (user unlikely to return)

2. **Results** (`result:{uuid}`) - TTL: 7 days
   - Complete analysis results for retrieval
   - Expire after 1 week (user can download/save locally)

3. **Learning Vault** (`vault:{crisis_type}:{uuid}`) - TTL: 90 days
   - Anonymized case studies for ML training
   - Expire after 90 days (rotate for freshness)

**Redis Features Used**:
- **JSON storage** (redis-json): Store complex nested objects natively
- **Cluster mode**: Horizontal scaling for high availability
- **Persistence**: AOF (Append-Only File) for durability
- **Pub/Sub**: WebSocket message broadcasting across servers

**Key Patterns**:
```
session:{uuid} → { status, current_phase, agents: [...] }
result:{uuid} → { pass1_analysis, pass2_analysis, responses }
vault:{crisis_type}:{uuid} → { omissions, vulnerabilities, outcome }
progress:{session_id} → { agent_name: progress_percent }
```

### Rationale

**Pros**:
- Redis is extremely fast (<1ms operations)
- Native JSON support avoids serialization overhead
- TTL automatically cleans up old data (no manual deletion)
- Pub/Sub enables multi-server WebSocket broadcasting
- Widely supported, battle-tested in production

**Cons**:
- Redis is in-memory (higher cost than disk-based DB)
- Data loss risk if AOF not configured properly
- Limited query capabilities (no complex joins/aggregations)

**Alternatives Considered**:
1. **PostgreSQL** - Rejected: Slower (10-50ms queries), overkill for temporary sessions
2. **MongoDB** - Rejected: More complex, unnecessary for simple key-value lookups
3. **S3 + DynamoDB** - Rejected: Higher latency, more complex architecture
4. **In-memory only (no persistence)** - Rejected: Lose sessions on server restart

### Consequences

**Positive**:
- Sub-millisecond session lookups
- Automatic cleanup (no manual deletion jobs)
- Easy horizontal scaling (Redis Cluster)
- Battle-tested for high-concurrency workloads

**Negative**:
- Must configure AOF persistence correctly (risk of data loss)
- Redis Cluster is complex to manage
- In-memory storage is more expensive than disk

**Mitigation**:
- Enable AOF with `appendfsync everysec` (balance durability/performance)
- Use Redis Sentinel or Cluster for high availability
- Monitor Redis memory usage, set max memory limits
- Regular Redis backups to S3 (daily snapshots)

---

## ADR-006: LLM Provider Strategy (OpenRouter vs. Ollama)

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: AI Engineer, Technical Lead
**Context**: TruthForge needs production-quality LLM responses for accurate analysis but also requires fast, cost-free testing. Must decide LLM provider strategy.

### Decision

Implement **dual-provider strategy**:

**Production: OpenRouter**
- **Models**:
  - **Claude 3.5 Sonnet** (`anthropic/claude-3.5-sonnet:beta`) - Critical agents
    - BiasDetector, OmissionDetector, Pass1/Pass2 Debaters, Response agents
    - Best reasoning quality for complex analysis
  - **Claude 3.5 Haiku** (`anthropic/claude-3.5-haiku:beta`) - Structured tasks
    - TaxonomyLinker, Pass1/Pass2 Evaluators
    - Fast, cheaper, sufficient for classification/scoring

- **Configuration**:
  ```json
  {
    "llm_provider": "openrouter",
    "llm_model": "anthropic/claude-3.5-sonnet:beta",
    "openrouter_api_key_env": "OPENROUTER_API_KEY",
    "temperature": 0.3,
    "max_tokens": 2000
  }
  ```

**Testing: Ollama**
- **Model**: `gemma3:270m` (Google Gemma 3 270M parameters)
- **Use Cases**:
  - Integration tests (fast, deterministic)
  - Development environment (no API costs)
  - CI/CD pipeline tests
- **Trade-offs**: Lower quality responses acceptable for testing

**Cost Management**:
- Estimate: Claude 3.5 Sonnet ~$3/analysis, Haiku ~$0.50/analysis
- Target: <$5 per analysis total
- Monitoring: Track actual costs via OpenRouter dashboard

### Rationale

**Pros (OpenRouter)**:
- Access to best models (Claude 3.5 Sonnet) without Anthropic enterprise contract
- Unified API for multiple providers (easy to swap models)
- Automatic failover between providers
- Pay-as-you-go (no minimum commitment)
- Detailed usage analytics

**Pros (Ollama)**:
- Completely free for testing
- Fast iteration (no API latency)
- Works offline (dev environment)
- Reproducible tests (same model version)

**Cons (OpenRouter)**:
- Cost can be unpredictable (depends on prompt length)
- Rate limits (must implement backoff)
- Dependency on external service

**Cons (Ollama)**:
- Quality too low for production
- Requires local GPU (or slow CPU inference)
- Not suitable for production load

**Alternatives Considered**:
1. **Direct Anthropic API** - Rejected: Requires enterprise contract, higher cost
2. **Azure OpenAI** - Rejected: More complex setup, similar cost
3. **Open-source only (Ollama in prod)** - Rejected: Quality insufficient for business use
4. **OpenAI GPT-4** - Rejected: Worse at nuanced analysis than Claude 3.5 Sonnet

### Consequences

**Positive**:
- Best-in-class analysis quality (Claude 3.5 Sonnet)
- Cost-effective testing (Ollama is free)
- Flexibility to switch models per agent (Sonnet vs. Haiku)
- No vendor lock-in (OpenRouter supports multiple providers)

**Negative**:
- Must implement provider-specific logic (OpenRouter vs. Ollama)
- Cost monitoring required (easy to overspend)
- Quality gap between test (Ollama) and prod (Claude) may hide issues

**Mitigation**:
- Agent abstraction layer (same interface for OpenRouter and Ollama)
- Cost alerts (trigger if daily spend >$500)
- Weekly "production quality" test runs (use OpenRouter in CI once per week)
- Prompt optimization to reduce token usage (shorter prompts, structured outputs)

**Cost Breakdown (per analysis)**:
| Agent | Model | Tokens In | Tokens Out | Cost |
|-------|-------|----------|-----------|------|
| BiasDetector | Sonnet | 1500 | 500 | $0.60 |
| NarrativeMapper | Sonnet | 1500 | 700 | $0.75 |
| TaxonomyLinker | Haiku | 1000 | 300 | $0.08 |
| OmissionDetector | Sonnet | 2000 | 1000 | $1.20 |
| Pass1 Debaters (2) | Sonnet | 2500 × 2 | 600 × 2 | $1.50 |
| Pass1 Evaluator | Haiku | 1500 | 500 | $0.12 |
| Pass2 Debaters (2) | Sonnet | 2000 × 2 | 700 × 2 | $1.40 |
| Pass2 Evaluator | Haiku | 2000 | 600 | $0.16 |
| Response Agents (3) | Sonnet | 1500 × 3 | 400 × 3 | $1.65 |
| **Total** | | | | **~$7.46** |

**Optimization Target**: Reduce to <$5 through prompt compression and selective Haiku usage

---

## ADR-007: Agent Configuration Management

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: AI Engineer, Backend Engineer
**Context**: TruthForge has 13 agent roles with different LLM providers, prompts, quality criteria. Need configuration management strategy.

### Decision

Use **JSON configuration files** per agent role, following terraphim-ai conventions:

**Directory Structure**:
```
taxonomy/
  └── roles/
      ├── bias_detector.json
      ├── narrative_mapper.json
      ├── taxonomy_linker.json
      ├── omission_detector.json
      ├── pass1_debater_pro.json
      ├── pass1_debater_con.json
      ├── pass1_evaluator.json
      ├── pass2_debater_exploit.json
      ├── pass2_debater_defense.json
      ├── cumulative_evaluator.json
      ├── reframe_agent.json
      ├── counterargue_agent.json
      └── bridge_agent.json
```

**Configuration Schema**:
```json
{
  "name": "Agent Display Name",
  "shortname": "agent_identifier",
  "relevance_function": "BM25Plus",
  "extra": {
    "llm_provider": "openrouter|ollama",
    "llm_model": "model_name",
    "system_prompt": "Detailed multi-paragraph prompt...",
    "agent_type": "analyzer|classifier|debater|evaluator|generator",
    "quality_criteria": ["criterion_1", "criterion_2"],
    "taxonomy_mapping": "function.subfunction",
    "temperature": 0.0-1.0,
    "max_tokens": 1000-3000,
    "version": "1.0.0"
  }
}
```

**Loading Mechanism**:
```rust
impl TerraphimAgent {
    pub async fn from_config_file(path: impl AsRef<Path>) -> Result<Self> {
        let json = tokio::fs::read_to_string(path).await?;
        let config: AgentConfig = serde_json::from_str(&json)?;
        Self::from_config(config).await
    }
}
```

### Rationale

**Pros**:
- Human-readable, easy to edit (JSON)
- Version control friendly (text files)
- Supports A/B testing (swap config files)
- Separation of code and prompts (prompt engineers can edit without code changes)
- Compatible with terraphim-ai's existing agent system

**Cons**:
- JSON is verbose (prompts are long strings)
- No schema validation unless manually added
- Environment-specific configs require file duplication (prod vs. test)

**Alternatives Considered**:
1. **Hard-coded in Rust** - Rejected: Can't update prompts without recompiling
2. **YAML** - Rejected: JSON is terraphim-ai standard
3. **TOML** - Rejected: Worse multiline string support than JSON
4. **Database storage** - Rejected: Overkill, loses version control benefits

### Consequences

**Positive**:
- Rapid prompt iteration (edit JSON, restart server)
- A/B testing (create `bias_detector_v2.json`, compare results)
- Non-engineers can improve prompts (product team, domain experts)
- Easy rollback (Git revert)

**Negative**:
- Must validate JSON schema manually (or use JSON Schema)
- Config files can drift from code (e.g., new quality_criteria not used)
- Environment-specific configs require convention (e.g., `*_prod.json`, `*_test.json`)

**Mitigation**:
- Add JSON Schema validation in agent constructor
- Unit tests verify all config files load successfully
- Document config schema in REQUIREMENTS_AgentRoles.md
- Use environment variables for secrets (OPENROUTER_API_KEY_ENV)

---

## ADR-008: Error Handling Strategy

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: Backend Engineer, Technical Lead
**Context**: TruthForge workflow has 13 agents in sequence. Single agent failure could crash entire analysis. Need error handling strategy.

### Decision

Implement **graceful degradation with partial results**:

**Error Categories**:
1. **Retryable Errors**: LLM rate limit, network timeout
   - Action: Retry with exponential backoff (3 attempts)
   - Fallback: Use Ollama if OpenRouter fails repeatedly

2. **Validation Errors**: LLM returns invalid JSON
   - Action: Retry with clarified prompt ("Return only valid JSON")
   - Fallback: Skip optional agents (e.g., NarrativeMapper), continue workflow

3. **Fatal Errors**: Invalid input text, system error
   - Action: Abort workflow, return error to user

**Partial Results Strategy**:
```rust
pub struct AnalysisResult {
    pub status: AnalysisStatus,
    pub pass1_analysis: Option<Pass1Analysis>,
    pub pass2_analysis: Option<Pass2Analysis>,
    pub responses: Option<ResponseSet>,
    pub errors: Vec<AgentError>,
}

pub enum AnalysisStatus {
    Complete,           // All agents succeeded
    PartialSuccess,     // Some agents failed, partial results available
    Failed,             // Fatal error, no results
}
```

**Error Reporting**:
- Log all errors with context (agent name, input, error type)
- Send WebSocket error messages to UI
- Continue workflow when possible, mark status as `PartialSuccess`

### Rationale

**Pros**:
- User gets partial results even if some agents fail
- Network issues don't crash entire analysis
- Graceful degradation improves UX
- Detailed error logging aids debugging

**Cons**:
- Complexity from handling partial results
- User may be confused by partial data
- Retries increase latency

**Alternatives Considered**:
1. **Fail fast** - Rejected: Single agent failure loses all progress
2. **Ignore all errors** - Rejected: Silent failures are worse than no results
3. **Queue failed analyses for later** - Rejected: Adds complexity, user expects real-time

### Consequences

**Positive**:
- Better user experience (partial > nothing)
- Resilient to transient LLM API issues
- Detailed error logs for debugging
- Supports future feature: "Re-run failed agents"

**Negative**:
- UI must handle partial results gracefully
- Testing requires error injection (mock failures)
- Users may expect complete results always

**Mitigation**:
- Clear UI messaging ("3 of 13 agents failed, showing partial results")
- Retry button to re-run failed agents
- Alerting for high failure rates (>10% of agents failing)

---

## ADR-009: Testing Strategy

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: Technical Lead, QA Engineer
**Context**: TruthForge has complex multi-agent workflows with LLM integration. Traditional testing approaches may be insufficient.

### Decision

Implement **four-layer testing pyramid**:

**Layer 1: Unit Tests** (Fast, 80% of tests)
- Test individual functions (omission validation, risk scoring)
- Mock LLM responses (use fixtures)
- Target: 80% code coverage
- Runtime: <10 seconds total

**Layer 2: Integration Tests** (Medium, 15% of tests)
- Test agent workflows (Pass 1, Pass 2, Response generation)
- Use Ollama for real LLM calls (fast, deterministic)
- Test WebSocket + Redis integration
- Runtime: <2 minutes total

**Layer 3: Contract Tests** (Slow, 4% of tests)
- Validate OpenRouter API responses match expectations
- Verify JSON schemas from LLMs
- Test error handling for API failures
- Runtime: <5 minutes total (gated by `--ignored` flag)

**Layer 4: End-to-End Tests** (Very slow, 1% of tests)
- Full workflow with OpenRouter (real production path)
- UI integration tests with Playwright
- Load tests with k6
- Runtime: <15 minutes total (CI nightly only)

**Test Data Strategy**:
- **Synthetic**: Generated crisis communication texts (unit tests)
- **Curated**: Real-world examples from SCCT case studies (integration tests)
- **Production samples**: Anonymized user inputs (E2E tests)

**CI Pipeline**:
```yaml
on: [push, pull_request]
jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - cargo test --lib

  integration-tests:
    runs-on: ubuntu-latest
    services:
      redis:
        image: redis:7
      ollama:
        image: ollama/ollama
    steps:
      - cargo test --test integration_*

  contract-tests:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - cargo test --ignored

  e2e-tests:
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - cargo test --test e2e_*
      - npx playwright test
```

### Rationale

**Pros**:
- Fast feedback (unit tests complete in <10s)
- Comprehensive coverage (all layers tested)
- Cost-effective (use Ollama for most tests)
- CI-friendly (fast tests on every commit)

**Cons**:
- Mock LLM responses may not match real behavior
- Ollama responses differ from OpenRouter (quality gap)
- E2E tests are slow and flaky

**Alternatives Considered**:
1. **Only unit tests** - Rejected: Misses integration issues
2. **Only E2E tests** - Rejected: Too slow, expensive (OpenRouter costs)
3. **Manual testing only** - Rejected: Not scalable, error-prone

### Consequences

**Positive**:
- Fast CI pipeline (unit + integration <3 minutes)
- Catches regressions early
- Production-like testing with Ollama
- Clear test ownership (unit = developers, E2E = QA)

**Negative**:
- Must maintain mock LLM fixtures
- Ollama setup required in CI
- E2E tests may be flaky (network, LLM variance)

**Mitigation**:
- Automate mock fixture generation (record real OpenRouter responses)
- Use Docker for consistent Ollama environment
- Retry flaky E2E tests (up to 3 attempts)
- Weekly "production quality" test runs (OpenRouter in CI)

---

## ADR-010: Security Architecture

**Date**: 2025-10-07
**Status**: Accepted
**Deciders**: Security Engineer, Backend Engineer
**Context**: TruthForge processes sensitive crisis communication text that may contain PII, confidential strategy. Must ensure security.

### Decision

Implement **defense-in-depth security**:

**Layer 1: Input Sanitization**
- Use terraphim-ai's existing `prompt_sanitizer` module
- Detect LLM prompt injection attempts
- Enforce 10,000 character limit on input
- Redact PII (emails, phone numbers, SSNs) before LLM processing

**Layer 2: API Security**
- Rate limiting: 10 requests/minute per IP (DDoS prevention)
- Authentication: API key required for `/api/v1/*` endpoints
- HTTPS only (TLS 1.3)
- CORS policy: whitelist approved domains

**Layer 3: Data Security**
- Redis encryption at rest (AOF encrypted)
- TLS for Redis connections
- PII redaction before storing in learning vault
- Automatic data deletion (TTL enforcement)

**Layer 4: LLM Security**
- OpenRouter API key stored in environment variable (not code)
- Prompt injection detection (terraphim sanitizer)
- Response validation (reject suspicious LLM outputs)
- Audit logging for all LLM calls

**Security Testing**:
- Leverage terraphim-ai's existing security test suite
- Add TruthForge-specific tests (PII redaction, rate limiting)
- Weekly automated security scans (cargo-audit, Dependabot)

**Code Example** (PII Redaction):
```rust
use terraphim_multi_agent::prompt_sanitizer::sanitize_system_prompt;
use regex::Regex;

pub fn sanitize_input(text: &str) -> Result<String> {
    // Step 1: Terraphim prompt injection prevention
    let sanitized = sanitize_system_prompt(text);

    if sanitized.was_modified {
        warn!("Prompt injection detected: {:?}", sanitized.modifications);
    }

    // Step 2: TruthForge PII redaction
    let redacted = redact_pii(&sanitized.sanitized_prompt);

    Ok(redacted)
}

fn redact_pii(text: &str) -> String {
    lazy_static! {
        static ref EMAIL: Regex = Regex::new(r"\b[\w\.-]+@[\w\.-]+\.\w+\b").unwrap();
        static ref PHONE: Regex = Regex::new(r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap();
    }

    let mut result = text.to_string();
    result = EMAIL.replace_all(&result, "[EMAIL]").to_string();
    result = PHONE.replace_all(&result, "[PHONE]").to_string();
    result
}
```

### Rationale

**Pros**:
- Reuses terraphim-ai's battle-tested security modules
- Defense-in-depth (multiple layers)
- Compliance-friendly (PII redaction, data deletion)
- Audit trail for LLM usage

**Cons**:
- PII redaction may reduce analysis accuracy (missing context)
- Rate limiting may frustrate legitimate users
- Security layers add latency

**Alternatives Considered**:
1. **No security** - Rejected: Unacceptable for enterprise
2. **Only input sanitization** - Rejected: Insufficient for production
3. **Custom security module** - Rejected: Reinventing tested code (use terraphim)

### Consequences

**Positive**:
- Enterprise-ready security posture
- GDPR/CCPA compliant (PII redaction, data deletion)
- Prevents prompt injection attacks
- Detailed audit logs for incident response

**Negative**:
- Development overhead (security testing, compliance)
- May reject legitimate edge-case inputs (false positives)
- Latency from sanitization steps (~50ms)

**Mitigation**:
- Tune PII redaction patterns (minimize false positives)
- Implement whitelist for trusted IPs (skip rate limits)
- Cache sanitization results (same input → same output)
- Document security architecture for auditors

---

## Summary: Key Architectural Decisions

| ADR | Decision | Impact |
|-----|----------|--------|
| ADR-001 | Private repository | Protects IP, enables commercialization |
| ADR-002 | Hybrid workflow patterns | Leverages terraphim-ai, supports two-pass innovation |
| ADR-003 | RoleGraph taxonomy migration | Enables graph-based classification |
| ADR-004 | WebSocket + REST | Real-time UX, efficient updates |
| ADR-005 | Redis with tiered TTL | Fast persistence, automatic cleanup |
| ADR-006 | OpenRouter + Ollama | Best quality + cost-effective testing |
| ADR-007 | JSON agent configs | Rapid prompt iteration, version control |
| ADR-008 | Graceful degradation | Resilient to failures, better UX |
| ADR-009 | Four-layer testing | Fast feedback, comprehensive coverage |
| ADR-010 | Defense-in-depth security | Enterprise-ready, compliant |

---

## Future ADRs (Planned)

- **ADR-011**: Horizontal Scaling Strategy (Redis pub/sub, sticky sessions)
- **ADR-012**: Monitoring & Observability (Prometheus, Grafana, alerts)
- **ADR-013**: Deployment Strategy (Kubernetes, Helm, blue-green)
- **ADR-014**: API Versioning & Backwards Compatibility
- **ADR-015**: Machine Learning Integration (Learning vault, model training)

---

**Document Status**: Draft v1.0
**Next Review**: After Phase 1 implementation (Week 2)
**Approval Required**: Technical Lead, Security Engineer, Product Owner
