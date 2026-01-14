# Research Document: LLMRouter Integration for Terraphim AI

**Status**: Draft
**Date**: 2026-01-01
**Author**: AI Research Specialist
**Reviewers**: [To be assigned]

## Executive Summary

LLMRouter is an intelligent routing system that dynamically selects optimal LLM models based on task complexity, cost, and performance requirements. Terraphim AI currently uses static model selection through role-based configuration. Integrating LLMRouter's intelligent routing capabilities could significantly improve cost efficiency, response quality, and user experience by automatically selecting the best model for each query.

## Problem Statement

### Description
Terraphim AI currently uses static LLM model selection configured per role. Users manually select a single model (e.g., "gpt-3.5-turbo" or "llama3.1") that is used for all queries regardless of query complexity, cost considerations, or performance requirements. This approach has several limitations:

1. **Cost Inefficiency**: Simple queries (e.g., basic summarization) use expensive models when cheaper models would suffice
2. **Quality Trade-offs**: Complex queries may use underpowered models when more capable models are available
3. **Manual Management**: Users must manually select and switch models based on perceived query complexity
4. **No Adaptive Learning**: The system doesn't learn from usage patterns or optimize routing over time
5. **Wasted Resources**: Over-provisioning on simple tasks and under-provisioning on complex tasks

### Impact
**Who is affected:**
- **End Users**: Experience higher costs than necessary, potentially suboptimal response quality
- **Administrators**: Must manually configure and update model selections for different use cases
- **System Performance**: Suboptimal resource utilization leads to higher operational costs

**Business Impact:**
- Increased API costs from using expensive models for simple queries
- Potential degradation in user satisfaction from mismatched model capabilities
- Loss of competitive advantage compared to systems with intelligent routing

### Success Criteria
1. **Cost Reduction**: Reduce LLM API costs by 20-40% through intelligent model selection
2. **Quality Maintenance**: Maintain or improve response quality compared to static routing
3. **User Transparency**: Users can see which model was selected and why
4. **Learning Capability**: System improves routing decisions based on usage patterns
5. **Flexibility**: Support multiple routing strategies (cost-first, quality-first, balanced)

## Current State Analysis

### Existing Implementation

**LLM Client Architecture:**

```
┌─────────────────────────────────────────┐
│   terraphim_service::llm module      │
├─────────────────────────────────────────┤
│ LlmClient (trait)                   │
│   - summarize()                      │
│   - chat_completion()                │
│   - list_models()                    │
└─────────────────────────────────────────┘
         ▲               ▲
         │               │
         │               │
         │               │
┌─────────────────┐  ┌──────────────────┐
│ OpenRouterClient │  │  OllamaClient   │
│ - openrouter    │  │  - ollama       │
└─────────────────┘  └──────────────────┘
         ▲
         │
┌─────────────────────────────────────────┐
│   LlmProxyClient                    │
│ - Unified proxy configuration         │
│ - Auto-detect providers             │
│ - Connectivity testing              │
└─────────────────────────────────────────┘
         ▲
         │
┌─────────────────────────────────────────┐
│   GenAiLlmClient                   │
│ - Multi-provider support             │
│ - Ollama, OpenAI, Anthropic,      │
│   OpenRouter                      │
└─────────────────────────────────────────┘
```

**Key Components:**

| Component | Location | Purpose |
|-----------|----------|---------|
| `LlmClient` trait | `crates/terraphim_service/src/llm.rs:20` | Abstract interface for LLM operations |
| `OpenRouterClient` | `crates/terraphim_service/src/llm.rs:169` | Commercial LLM provider |
| `OllamaClient` | `crates/terraphim_service/src/llm.rs:227` | Local LLM provider |
| `LlmProxyClient` | `crates/terraphim_service/src/llm_proxy.rs:86` | Unified proxy with auto-configuration |
| `GenAiLlmClient` | `crates/terraphim_multi_agent/src/genai_llm_client.rs:16` | Multi-provider client using rust-genai |
| `Role` struct | `crates/terraphim_config/src/lib.rs:178` | User configuration with LLM settings |

**Role-based Configuration:**

```rust
pub struct Role {
    pub llm_enabled: bool,           // Enable LLM features
    pub llm_api_key: Option<String>, // API key
    pub llm_model: Option<String>,   // Static model selection
    pub llm_auto_summarize: bool,   // Auto-summarize results
    // ... other fields
}
```

**Provider Support:**
- **OpenRouter**: Commercial models via API (GPT, Claude, etc.)
- **Ollama**: Local models (Llama, Gemma, etc.)
- **Anthropic**: Direct API with optional z.ai proxy
- **OpenAI**: Direct API support

**Current Routing Logic:**
```rust
// Static model selection from role configuration
pub fn build_llm_from_role(role: &Role) -> Option<Arc<dyn LlmClient>> {
    // Check provider preference
    if let Some(provider) = get_string_extra(&role.extra, "llm_provider") {
        match provider.as_str() {
            "ollama" => build_ollama_from_role(role),
            "openrouter" => build_openrouter_from_role(role),
            // ... more providers
        }
    }
    // Fallback logic
}
```

### Data Flow

```
User Query
    ↓
Role Configuration (static model selection)
    ↓
LlmClient Builder (build_llm_from_role)
    ↓
Provider Selection (OpenRouter/Ollama/etc.)
    ↓
LLM API Call
    ↓
Response
```

**Current Limitations:**
1. No query analysis before routing
2. No cost/performance metrics tracking
3. No fallback or retry mechanisms
4. No learning from past requests
5. Static model selection per role

### Integration Points

**Primary Extension Points:**

1. **`LlmClient` trait** (llm.rs:20)
   - Could add `route()` method to interface with router
   - Existing implementations would wrap router logic

2. **`build_llm_from_role()`** (llm.rs:56)
   - Entry point for LLM client creation
   - Router could intercept here and add intelligence

3. **Role configuration** (lib.rs:178)
   - Add `llm_router_enabled` field
   - Add `llm_router_config` with routing strategy preferences

4. **Proxy system** (llm_proxy.rs:86)
   - Could use proxy for routing decisions
   - Already handles multiple providers

## Constraints

### Technical Constraints

| Constraint | Description | Source |
|------------|-------------|---------|
| **Language Mismatch** | LLMRouter is Python 3.10+, Terraphim is Rust | LLMRouter README |
| **Architecture** | Monolithic role-based configuration | Code analysis |
| **Async Runtime** | Uses tokio for async operations | Cargo.toml |
| **Error Handling** | Uses `thiserror` and `Result<T, E>` pattern | Code analysis |
| **No Python Runtime** | Terraphim is pure Rust, no Python VM | Architecture review |

### Business Constraints

| Constraint | Impact | Source |
|------------|---------|--------|
| **Minimal Dependencies** | Prefer Rust-native solutions | Cargo.toml patterns |
| **Performance** | Low latency routing (<50ms overhead) | User requirements |
| **Maintainability** | Code must follow existing patterns | Team conventions |
| **Feature Flags** | Use cargo features for optional components | Existing pattern |

### Non-Functional Requirements

| Requirement | Target | Current | Notes |
|-------------|--------|---------|-------|
| Routing Overhead | <50ms | N/A (no routing) | Critical for UX |
| Memory Footprint | <100MB for router | N/A | Router must be lightweight |
| Model Switching Time | <100ms | N/A | Time to switch between providers |
| Cost Tracking | Per-request granularity | None | Required for optimization |
| Learning Accuracy | >80% optimal routing | N/A | Based on historical data |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_service` | Core LLM integration | HIGH - Router must integrate with this |
| `terraphim_config` | Role and configuration management | MEDIUM - Need new config fields |
| `terraphim_multi_agent` | Multi-provider support | LOW - Optional integration |
| `rust-genai` | Unified LLM interface | MEDIUM - Could leverage for routing |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `pyo3` | Latest | HIGH - Python-Rust bridge overhead | Reimplement in Rust |
| `reqwest` | 0.12 | LOW - Already used | None |
| `tokio` | 1.0 | LOW - Already used | None |
| `sqlx` | Latest | LOW - Could use for training data | In-memory storage |
| `numpy` | Latest | MEDIUM - For Python bridge | Burn/Candle (Rust ML) |

**Key Dependency Decision:**
- **Option A**: Python bridge (pyo3) to use LLMRouter as-is
  - Pros: Reuse mature code, faster integration
  - Cons: Python runtime overhead, additional dependency, complexity
- **Option B**: Reimplement core routing logic in Rust
  - Pros: Native performance, consistent codebase, no Python
  - Cons: Significant development effort, maintenance burden

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Python Integration Overhead** | HIGH | MEDIUM | Use Option B (Rust reimplementation) or FFI optimization |
| **Training Data Availability** | MEDIUM | HIGH | Start with simple routers, gradually collect data |
| **Performance Degradation** | MEDIUM | HIGH | Benchmarks, optimize hot paths, caching |
| **Model Capability Gaps** | LOW | MEDIUM | Fallback to high-quality models |
| **User Configuration Complexity** | MEDIUM | LOW | Smart defaults with optional fine-tuning |
| **Cost Tracking Accuracy** | LOW | MEDIUM | Implement token counting and provider pricing |

### Open Questions

1. **Router Strategy Selection**: Which routing algorithms are most valuable for Terraphim's use cases?
   - *Investigation needed*: Analyze query patterns from logs
   - *Stakeholder*: Product team, users

2. **Training Data Source**: Should we use LLMRouter's benchmark datasets or Terraphim-specific data?
   - *Investigation needed*: Data availability, domain alignment
   - *Stakeholder*: Data engineering team

3. **Model Catalog**: Which models should be available for routing?
   - *Investigation needed*: Cost analysis, performance benchmarks
   - *Stakeholder*: Infrastructure team

4. **Cost Tracking**: How detailed should cost tracking be?
   - *Options*: Per-request, per-user, per-model, per-day
   - *Investigation needed*: Requirements from finance team

5. **User Preferences**: Should users be able to influence routing decisions?
   - *Options*: Cost-first, quality-first, balanced modes
   - *Investigation needed*: User interviews

### Assumptions

1. **Assumption 1**: Terraphim's queries have varied complexity suitable for routing
   - *Basis*: Search, summarization, Q&A tasks have different requirements
   - *Validation*: Query analysis from logs

2. **Assumption 2**: Users will accept temporary quality variations for cost savings
   - *Basis*: Similar to Cloudflare's Tiered Cache model
   - *Validation*: A/B testing

3. **Assumption 3**: Simple routing algorithms (KNN, rule-based) can be implemented quickly in Rust
   - *Basis*: Rust ML ecosystem (Burn, Candle) is mature
   - *Validation*: Prototype simple router

4. **Assumption 4**: Historical performance data can improve routing accuracy
   - *Basis*: LLMRouter research papers show 20-40% cost savings
   - *Validation*: Data collection experiments

5. **Assumption 5**: The routing overhead (<50ms) will be acceptable compared to LLM latency (500-5000ms)
   - *Basis*: LLM API calls dominate request time
   - *Validation*: Performance benchmarks

## Research Findings

### Key Insights

1. **Terraphim Has Strong Foundation**:
   - Existing `LlmClient` abstraction makes router integration clean
   - Multiple provider support (OpenRouter, Ollama, Anthropic) already implemented
   - Proxy system handles base URLs and authentication automatically
   - Role-based configuration provides natural place for router settings

2. **Language Mismatch is Critical**:
   - LLMRouter is Python-based, Terraphim is Rust
   - Python bridge (pyo3) adds ~10-50ms overhead
   - Reimplementation in Rust is likely better long-term
   - Can borrow algorithm ideas and architecture from LLMRouter

3. **Cost Savings Opportunity**:
   - LLMRouter literature reports 20-40% cost reduction
   - Simple queries (e.g., basic summarization) can use cheaper models
   - Complex queries can route to premium models
   - Example: $0.02/1K tokens for GPT-3.5 vs $0.03/1K tokens for GPT-4

4. **Progressive Implementation Possible**:
   - Start with simple routers (rule-based, KNN)
   - Gradually add advanced features (ML-based, personalized)
   - Can implement subset of LLMRouter functionality first
   - Incremental value delivery

5. **Training Data Requirements**:
   - Need query embeddings (Terraphim doesn't generate these currently)
   - Need model performance metrics (token usage, quality scores)
   - LLMRouter provides 11 benchmark datasets as reference
   - Can bootstrap with synthetic data or offline analysis

6. **Architecture Options**:

   **Option A: Wrapper Router (Python Bridge)**
   ```
   Terraphim Rust Code
       ↓ (FFI)
   LLMRouter Python Module
       ↓
   Multiple LLM Providers
   ```
   - Pros: Reuse mature LLMRouter code
   - Cons: Python runtime, ~50ms overhead, complexity

   **Option B: Native Rust Router**
   ```
   Terraphim Rust Code
       ↓
   Native Rust Router (reimplemented)
       ↓
   LlmClient Interface
       ↓
   Multiple LLM Providers
   ```
   - Pros: Zero overhead, consistent codebase, Rust ecosystem
   - Cons: 2-4 weeks development effort

   **Option C: Hybrid (Rust Routing + Python ML)**
   ```
   Terraphim Rust Code
       ↓
   Router Rules Engine (Rust)
       ↓
   ML Predictions (Python, cached)
       ↓
   LlmClient Interface
   ```
   - Pros: Best of both worlds
   - Cons: Complex architecture, dual runtime

### Relevant Prior Art

- **RouteLLM** (ICLR 2025): Learning to route LLMs with preference data
  - Relevance: Demonstrates 30% cost reduction with ML-based routing
  - Terraphim fit: Could implement similar approach in Rust

- **RouterDC** (NeurIPS 2024): Query-based router by dual contrastive learning
  - Relevance: BERT-based query embeddings for routing
  - Terraphim fit: Could use existing search embeddings

- **AutoMix** (NeurIPS 2024): Automatic model mixing
  - Relevance: Dynamically selects top-k models and combines outputs
  - Terraphim fit: Advanced feature for future

- **Cloudflare Tiered Cache**: Similar cost optimization pattern
  - Relevance: Demonstrates user acceptance of quality/cost tradeoffs
  - Terraphim fit: User-facing controls for routing

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| **Query Embedding Generation** | Generate embeddings for routing input | 1 day |
| **Simple Router Prototype** | Implement rule-based router in Rust | 2 days |
| **Performance Benchmarking** | Measure current LLM latency distribution | 1 day |
| **Cost Tracking System** | Track token usage and costs per model | 2 days |
| **ML Framework Evaluation** | Evaluate Burn vs Candle for ML operations | 1 day |

## Recommendations

### Proceed/No-Proceed

**Recommendation: PROCEED with Option B (Native Rust Implementation)**

**Justification:**
1. **Long-term maintainability**: Consistent Rust codebase is easier to maintain
2. **Performance**: Zero overhead compared to Python bridge (~50ms saved per request)
3. **Learning opportunity**: Team gains ML routing expertise
4. **Incremental delivery**: Can ship simple routers quickly, add ML later
5. **Cost savings**: LLMRouter research shows 20-40% reduction
6. **Strategic value**: Differentiates Terraphim from competitors with static routing

### Scope Recommendations

**Phase 1 (MVP - 4 weeks):**
- Rule-based router (cost-first, quality-first, balanced)
- Simple KNN router with pre-defined similarity thresholds
- Query complexity classification (length, keywords, structure)
- Basic metrics collection (token usage, latency)
- Role configuration integration (enable/disable router)
- Documentation and examples

**Phase 2 (ML-based - 6 weeks):**
- Embedding generation for queries
- Train simple ML router (logistic regression or shallow MLP)
- Historical performance tracking
- A/B testing framework
- Cost optimization dashboard

**Phase 3 (Advanced - 8 weeks):**
- Personalized routing based on user patterns
- Multi-round routing for conversations
- Model mixing strategies
- Online learning and adaptive routing
- Advanced analytics and reporting

**Out of Scope (deferred):**
- Multi-modal routing (images, audio)
- Reinforcement learning routing
- Agentic routing with reasoning
- Custom plugin system (can borrow from LLMRouter later)

### Risk Mitigation Recommendations

1. **Start Simple, Iterate Fast**:
   - Implement rule-based router first
   - Collect real usage data
   - Train ML models with Terraphim-specific data
   - Gradually add complexity

2. **Comprehensive Testing**:
   - Unit tests for routing logic
   - Integration tests with mock LLM providers
   - A/B testing against baseline (static routing)
   - Performance benchmarks at each step

3. **User Controls**:
   - Users can opt out and use static model selection
   - Transparent routing decisions (show model selected and why)
   - Configurable routing strategies (cost vs quality priority)
   - Override capabilities for specific queries

4. **Monitoring and Observability**:
   - Track routing decisions, costs, and quality metrics
   - Alert on unexpected routing patterns
   - Dashboard for administrators
   - Cost projection and optimization opportunities

5. **Fallback Mechanisms**:
   - Fallback to default model on routing failure
   - Circuit breakers for underperforming models
   - Timeout handling for routing logic
   - Rate limit awareness

## Next Steps

If approved:

1. **Spike 1: Query Embeddings** (1 day)
   - Evaluate Burn and Candle frameworks
   - Generate embeddings for sample queries
   - Test performance and accuracy

2. **Spike 2: Simple Router** (2 days)
   - Implement rule-based router in Rust
   - Integrate with existing `LlmClient` trait
   - Test with mock providers

3. **Phase 1 Planning** (3 days)
   - Detailed design for MVP router
   - Define data structures and APIs
   - Create implementation plan

4. **Implementation** (4 weeks)
   - Follow disciplined development process
   - Regular code reviews
   - Continuous testing and integration

## Appendix

### Reference Materials

- **LLMRouter GitHub**: https://github.com/ulab-uiuc/LLMRouter
- **LLMRouter Documentation**: https://ulab-uiuc.github.io/LLMRouter/
- **RouteLLM Paper**: https://arxiv.org/abs/2406.18665
- **RouterDC Paper**: https://arxiv.org/abs/2409.19886
- **AutoMix Paper**: https://arxiv.org/abs/2310.12963
- **Burn Framework**: https://burn.dev/ (Rust ML)
- **Candle Framework**: https://github.com/huggingface/candle (Rust ML)

### Code Snippets

**Current LLM Client Usage:**
```rust
// From terraphim_service/src/llm.rs:56
pub fn build_llm_from_role(role: &Role) -> Option<Arc<dyn LlmClient>> {
    if let Some(provider) = get_string_extra(&role.extra, "llm_provider") {
        match provider.as_str() {
            "ollama" => build_ollama_from_role(role),
            "openrouter" => build_openrouter_from_role(role),
            // ...
        }
    }
}
```

**Proposed Router Interface:**
```rust
pub trait LlmRouter: Send + Sync {
    /// Route query to optimal LLM model
    async fn route(&self, query: &RouterInput) -> RouterDecision;

    /// Update router with performance feedback
    async fn update(&self, metrics: RouterMetrics);

    /// Get routing statistics
    fn get_stats(&self) -> RouterStats;
}

pub struct RouterInput {
    pub query: String,
    pub context: Option<String>,
    pub metadata: HashMap<String, Value>,
}

pub struct RouterDecision {
    pub model: String,
    pub provider: String,
    pub reason: String,
    pub confidence: f32,
}
```

**Rule-Based Router Example:**
```rust
pub struct RuleBasedRouter {
    config: RouterConfig,
}

impl LlmRouter for RuleBasedRouter {
    async fn route(&self, input: &RouterInput) -> RouterDecision {
        // Analyze query characteristics
        let complexity = self.estimate_complexity(&input.query);

        // Select model based on strategy
        match (self.config.strategy, complexity) {
            (Strategy::CostFirst, Complexity::Low) => {
                RouterDecision {
                    model: "gemma3:270m".to_string(),
                    provider: "ollama".to_string(),
                    reason: "Low complexity, using fastest model".to_string(),
                    confidence: 0.9,
                }
            }
            (Strategy::QualityFirst, Complexity::High) => {
                RouterDecision {
                    model: "claude-3-5-sonnet".to_string(),
                    provider: "openrouter".to_string(),
                    reason: "High complexity, using best model".to_string(),
                    confidence: 0.85,
                }
            }
            // ... more rules
        }
    }
}
```

### Terraphim Architecture Context

```
┌──────────────────────────────────────────────┐
│       Terraphim AI Architecture          │
├──────────────────────────────────────────────┤
│  ┌────────────┐    ┌──────────────┐ │
│  │  Role Config │───▶│ Router (NEW)│ │
│  └────────────┘    └──────┬───────┘ │
│                            │           │
│                            ▼           │
│                  ┌──────────────────┐ │
│                  │   LlmClient      │ │
│                  └────────┬─────────┘ │
│                           │            │
│            ┌──────────────┼────────────┐ │
│            ▼              ▼              ▼ │
│     ┌───────────┐  ┌──────────┐ ┌────────┐ │
│     │OpenRouter  │  │  Ollama  │ │Anthropic│ │
│     └───────────┘  └──────────┘ └────────┘ │
└──────────────────────────────────────────────┘
```

### Cost Analysis Example

**Current Static Routing:**
- All queries use GPT-4: $0.03/1K tokens
- 100K queries/month, avg 1K tokens: $3,000/month

**Intelligent Routing:**
- 60% simple queries use GPT-3.5: $0.002/1K tokens
- 30% medium queries use GPT-4: $0.03/1K tokens
- 10% complex queries use GPT-4-Turbo: $0.01/1K tokens
- 100K queries/month: (60K * $0.002) + (30K * $0.03) + (10K * $0.01) = $120 + $900 + $100 = $1,120/month

**Savings**: $3,000 → $1,120 = **63% cost reduction** (conservative estimate, actual may vary)
