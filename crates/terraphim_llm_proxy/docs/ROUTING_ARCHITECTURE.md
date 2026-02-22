# Routing Architecture

**Terraphim LLM Proxy Multi-Phase Intelligent Routing System**

Version: 3.0 (Model Mappings)
Last Updated: 2026-01-12
Status: Production-Ready ✅

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture Philosophy](#architecture-philosophy)
3. [Model Mappings](#model-mappings) ⭐ NEW
4. [Routing Phases](#routing-phases)
5. [Pattern Matching (Terraphim AI-Driven)](#pattern-matching-terraphim-ai-driven)
6. [Cost & Performance Optimization](#cost--performance-optimization)
7. [Configuration](#configuration)
8. [Request Flow](#request-flow)
9. [Testing](#testing)
10. [Performance](#performance)

---

## Overview

The Terraphim LLM Proxy implements a sophisticated **multi-phase routing system** that combines:

- **Taxonomy-driven pattern matching** (Terraphim AI configuration)
- **Algorithmic optimization** (cost and performance metrics)
- **Scenario-based fallback** (hint analysis)
- **Explicit provider control** (user specification)

This hybrid architecture enables both **AI-driven configuration** through taxonomy files and **runtime optimization** through metrics and algorithms.

### Key Features

✅ **Pattern-first routing** - Taxonomy patterns take precedence
✅ **Cost optimization** - Budget-aware model selection
✅ **Performance optimization** - Metrics-based provider ranking
✅ **Graceful fallback** - Multi-level degradation
✅ **Session-aware** - Context-based routing preferences
✅ **Explicit control** - User can override with `provider:model` syntax

---

## Architecture Philosophy

### Hybrid Approach

The routing system uses a **hybrid architecture** that combines:

1. **Static Configuration** (Pattern Matching)
   - User intent patterns stored in taxonomy markdown files
   - Hot-reloadable without code changes
   - Terraphim AI-driven configuration
   - Expresses "what users want" through synonyms

2. **Dynamic Optimization** (Algorithmic)
   - Runtime cost calculations and budget enforcement
   - Performance metrics and scoring
   - Real-time provider health monitoring
   - Implements "how to achieve it" through algorithms

### Why This Approach?

**Pattern matching alone cannot:**
- Calculate real-time costs based on token usage
- Track and enforce budget constraints
- Measure and compare provider performance
- Make multi-factor optimization decisions

**Algorithmic optimization alone cannot:**
- Easily capture user intent variations
- Be reconfigured without code changes
- Leverage knowledge graph relationships
- Express routing rules in human-readable form

**Together they provide:**
- Flexibility through taxonomy configuration
- Intelligence through runtime optimization
- Maintainability through separation of concerns
- Performance through efficient pattern matching

---

## Model Mappings

Model mappings allow you to configure aliases for model names without code changes. This is essential for integrating with clients like Claude Code that send specific model identifiers.

### How Model Mappings Work

Model mappings are processed **before any routing decision** (Phase 0). When a request arrives:

1. The model name is checked against configured mappings
2. If a match is found, the model name is replaced with the target
3. The target format `provider,model` triggers explicit provider routing
4. If no match, the original model name proceeds through normal routing

### Configuration

Add model mappings to your config file:

```toml
[router]
default = "openrouter,anthropic/claude-sonnet-4.5"
think = "openrouter,deepseek/deepseek-v3.1-terminus"

# Model name mappings (glob patterns supported)
[[router.model_mappings]]
from = "claude-opus-4-5-*"
to = "openrouter,anthropic/claude-opus-4.5"

[[router.model_mappings]]
from = "claude-sonnet-4-5-*"
to = "openrouter,anthropic/claude-sonnet-4.5"

[[router.model_mappings]]
from = "claude-3-5-sonnet-*"
to = "openrouter,anthropic/claude-3.5-sonnet"

[[router.model_mappings]]
from = "my-custom-alias"
to = "deepseek,deepseek-chat"
bidirectional = true  # Also remap response model name back
```

### Mapping Fields

| Field | Required | Description |
|-------|----------|-------------|
| `from` | Yes | Pattern to match (glob `*` supported, case-insensitive) |
| `to` | Yes | Target in `provider,model` format |
| `bidirectional` | No | If true, response model name is remapped back (default: false) |

### Glob Pattern Support

The `from` field supports glob patterns:

| Pattern | Matches | Example |
|---------|---------|---------|
| `claude-opus-4-5-*` | Any string starting with prefix | `claude-opus-4-5-20251101` |
| `*-fast` | Any string ending with suffix | `my-model-fast` |
| `claude-*-sonnet` | Prefix and suffix with any middle | `claude-3.5-sonnet` |

### Routing Flow with Mappings

```
Request: model="claude-opus-4-5-20251101"
    ↓
Model Mapping Check
    ↓ Match found: "claude-opus-4-5-*" → "openrouter,anthropic/claude-opus-4.5"
    ↓
model="openrouter,anthropic/claude-opus-4.5"
    ↓
Phase 0: Explicit Provider Specification
    ↓ Detects comma separator, extracts provider="openrouter", model="anthropic/claude-opus-4.5"
    ↓
Route to OpenRouter with model="anthropic/claude-opus-4.5"
    ↓
Response: model="anthropic/claude-opus-4.5"
```

### Interaction with Semantic Routing

**Key principle:** Model mappings **override** semantic routing.

| Request Model | Prompt | Routing Result |
|---------------|--------|----------------|
| `claude-opus-4-5-20251101` | "Help me plan" | → `anthropic/claude-opus-4.5` (mapping wins) |
| `auto` | "Help me plan" | → `deepseek-v3.1-terminus` (semantic: "plan" triggers think route) |
| `gpt-4` | "Hello" | → Default route (no mapping, no semantic match) |

### When to Use Model Mappings vs Semantic Routing

| Use Case | Solution |
|----------|----------|
| Client sends specific model IDs (Claude Code) | Model Mappings |
| Route based on prompt content | Semantic Routing (Taxonomy) |
| Force specific provider for testing | Explicit `provider:model` syntax |
| Budget-aware routing | Cost Optimization (Phase 3) |

### Example: Claude Code Integration

Claude Code sends model names like `claude-opus-4-5-20251101`. Configure mappings to route these to your preferred providers:

```toml
[[router.model_mappings]]
from = "claude-opus-4-5-*"
to = "openrouter,anthropic/claude-opus-4.5"

[[router.model_mappings]]
from = "claude-sonnet-4-5-*"
to = "openrouter,anthropic/claude-sonnet-4.5"

[[router.model_mappings]]
from = "claude-haiku-4-5-*"
to = "openrouter,anthropic/claude-3.5-haiku"
```

### Testing Model Mappings

```bash
# Test mapped model
curl -X POST http://localhost:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: YOUR_API_KEY" \
  -d '{
    "model": "claude-opus-4-5-20251101",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 10
  }'

# Response should show: "model": "anthropic/claude-opus-4.5"
```

### Performance

- **Lookup time:** < 0.1ms (linear scan with glob matching)
- **Memory overhead:** < 1KB for typical configurations
- **First-match semantics:** Order matters - more specific patterns should come first

---

## Routing Phases

The router executes up to **6 phases** in order, returning immediately when a match is found.

### Phase 0: Explicit Provider Specification

**Purpose:** Allow users to explicitly choose a provider and model

**Trigger:** Model name contains colon (`:`) or comma (`,`) separator

**Supported Formats:**
- `openrouter:anthropic/claude-sonnet-4.5` (colon - user explicit)
- `openrouter,anthropic/claude-sonnet-4.5` (comma - from model mappings)

**Behavior:**
```rust
// Supports both : and , separators
let separator_pos = model.find(':').or_else(|| model.find(','));

if let Some(pos) = separator_pos {
    let provider = &model[..pos];
    let model_name = &model[pos + 1..];
    return route_to(provider, model_name)
}
```

**Use cases:**
- Testing specific providers (colon syntax)
- Model mapping targets (comma syntax)
- Debugging routing decisions
- Forcing a particular model
- Bypassing semantic routing

**Examples:**
```bash
# User explicit (colon)
curl -X POST http://localhost:3456/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "openrouter:anthropic/claude-sonnet-4.5",
    "messages": [{"role": "user", "content": "Hello"}]
  }'

# From model mapping (comma - internal)
# Request: model="claude-opus-4-5-20251101"
# After mapping: model="openrouter,anthropic/claude-opus-4.5"
# Phase 0 detects comma, routes to openrouter
```

---

### Phase 1: Pattern-Based Routing (RoleGraph)

**Purpose:** Detect user intent from query text using Terraphim taxonomy

**Trigger:** Query text matches patterns in taxonomy files

**Algorithm:**
1. Extract last user message as query
2. Run Aho-Corasick pattern matching against query (lowercase)
3. Score matches based on length and position
4. Use highest-scoring match to determine routing

**Pattern Matching Engine:**
- **Technology:** Aho-Corasick automaton (200+ patterns)
- **Performance:** <1ms per query
- **Scoring:** `length_score * position_score`
  - Longer matches score higher
  - Earlier matches slightly preferred

**Taxonomy Integration:**

Taxonomy files define routing through two methods:

1. **Direct routing directives:**
```markdown
# Low Cost Routing

route:: deepseek, deepseek-chat

synonyms:: low cost, budget mode, cost cap, cheapest routing
```

2. **Fallback heuristics:** (if no directive present)
```rust
match concept {
    s if s.contains("background") => ("ollama", "qwen2.5-coder:latest"),
    s if s.contains("think") => ("deepseek", "deepseek-reasoner"),
    s if s.contains("search") => ("openrouter", "perplexity/..."),
    // ...
}
```

**Available Patterns:**

From `docs/taxonomy/routing_scenarios/`:
- `background_routing.md` → Local ollama models for batch tasks
- `think_routing.md` → Reasoning models for complex problems
- `low_cost_routing.md` → Budget-optimized providers
- `high_throughput_routing.md` → Low-latency, high-speed models
- `long_context_routing.md` → Large context window models
- `web_search_routing.md` → Search-capable models
- `image_routing.md` → Multimodal vision models
- `default_routing.md` → Standard routing

**Example Matching:**

Query: `"use cheapest option for this task"`
- Matches: `low_cost_routing` (synonym: "cheapest")
- Routes to: `deepseek/deepseek-chat`
- Scenario: `Pattern("low_cost_routing")`

**Code Location:** `src/router.rs:183-225`, `src/rolegraph_client.rs`

---

### Phase 2: Session-Aware Pattern Routing

**Purpose:** Use session history to refine pattern-based routing

**Trigger:**
- Session manager is available
- Session ID present in hints
- Pattern match found

**Behavior:**
1. Get or create session for session_id
2. Match patterns in query (same as Phase 1)
3. Use session provider preferences to select provider
4. Provider with highest preference score for concept wins

**Session Preferences:**
```rust
session.provider_preferences: HashMap<String, f64>
// Example: {"deepseek": 0.8, "openrouter": 0.6, "ollama": 0.3}
```

**Selection Logic:**
```rust
for (provider, score) in session.provider_preferences {
    if provider_matches_concept(provider, concept_key) && score > best_score {
        best_provider = provider
    }
}
```

**Use cases:**
- User consistently prefers certain providers
- Previous requests performed better on specific providers
- Learning from user feedback over time

**Code Location:** `src/router.rs:227-260`

---

### Phase 3: Cost Optimization

**Purpose:** Select the cheapest provider/model that meets requirements

**Trigger:**
- Cost optimization features enabled (pricing_database, cost_calculator, budget_manager)
- No pattern match found, or pattern matching disabled

**Algorithm:**
1. Determine scenario from hints (see Phase 5)
2. Get available providers for scenario
3. Estimate token usage from request
4. Calculate cost estimate for each provider/model
5. Check budget constraints
6. Select cheapest option that fits budget

**Cost Calculation:**
```rust
input_cost = input_tokens * pricing.input_cost_per_1k_tokens / 1000.0
output_cost = estimated_output_tokens * pricing.output_cost_per_1k_tokens / 1000.0
total_cost = input_cost + output_cost
```

**Budget Management:**
- Per-scenario budgets (e.g., `default-Background`)
- Budget periods: Daily, Weekly, Monthly
- Auto-creates default $100 daily budgets if missing
- Rejects requests exceeding budget limits

**Token Estimation:**
```rust
// Rough heuristic: ~4 characters per token
input_tokens = (message_chars / 4.0).ceil()
output_tokens = (input_tokens * 0.75).ceil()  // Estimate 75% of input
```

**Use cases:**
- Budget-constrained operations
- Cost-sensitive batch processing
- Development/testing with spending limits
- Multi-tenant cost tracking

**Code Location:** `src/router.rs:265-293`, `src/cost/`

---

### Phase 4: Performance Optimization

**Purpose:** Select the best-performing provider/model based on metrics

**Trigger:**
- Performance features enabled (performance_database, performance_tester)
- No pattern match or cost optimization found

**Algorithm:**
1. Determine scenario from hints
2. Get available providers for scenario
3. Retrieve performance metrics for each provider/model
4. Calculate performance score using weighted metrics
5. Filter by minimum thresholds
6. Select highest-scoring option

**Performance Metrics:**
```rust
struct PerformanceMetrics {
    latency_p50: f64,      // Median response time
    latency_p95: f64,      // 95th percentile latency
    throughput: f64,       // Tokens per second
    success_rate: f64,     // Successful requests ratio
    sample_count: usize,   // Number of measurements
}
```

**Scoring Formula:**
```rust
score = (latency_weight * latency_score) +
        (throughput_weight * throughput_score) +
        (success_rate_weight * success_rate)
```

**Default Weights:**
```rust
PerformanceWeights {
    latency: 0.4,        // 40% weight on latency
    throughput: 0.3,     // 30% weight on throughput
    success_rate: 0.3,   // 30% weight on success rate
}
```

**Thresholds:**
```rust
PerformanceThresholds {
    min_success_rate: 0.95,    // 95% minimum
    max_p95_latency_ms: 5000,  // 5s maximum
    min_sample_count: 10,      // Need at least 10 samples
}
```

**Use cases:**
- Latency-critical applications
- High-throughput batch processing
- Quality-of-service requirements
- A/B testing providers

**Code Location:** `src/router.rs:295-317`, `src/performance/`

---

### Phase 5: Scenario-Based Routing (Hints)

**Purpose:** Route based on request characteristics detected by analyzer

**Trigger:** All previous phases failed or were bypassed

**Scenario Detection Priority (highest to lowest):**

1. **Image** - `hints.has_images == true`
   - Images detected in content blocks
   - Routes to multimodal model

2. **WebSearch** - `hints.has_web_search == true`
   - Web search tool detected in tools array
   - Routes to search-capable model

3. **LongContext** - `hints.token_count >= long_context_threshold`
   - Token count exceeds threshold (default: 60,000)
   - Routes to large context window model

4. **Think** - `hints.has_thinking == true`
   - `thinking` field present in request
   - Routes to reasoning model

5. **Background** - `hints.is_background == true`
   - Model name contains "haiku"
   - Routes to background processing model

6. **Default** - Always available fallback

**Scenario Configuration:**
```toml
[router]
default = "openrouter,anthropic/claude-sonnet-4.5"
background = "ollama,qwen2.5-coder:latest"
think = "deepseek,deepseek-reasoner"
long_context = "openrouter,google/gemini-2.5-flash-preview"
long_context_threshold = 60000
web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"
image = "openrouter,anthropic/claude-sonnet-4.5"
```

**Use cases:**
- No pattern match found
- Pattern matching disabled
- Fallback for unsupported queries
- Basic routing without taxonomy

**Code Location:** `src/router.rs:319-321`, `src/analyzer.rs`

---

## Pattern Matching (Terraphim AI-Driven)

### Taxonomy Structure

Taxonomy files live in `docs/taxonomy/routing_scenarios/`:

```
docs/taxonomy/
└── routing_scenarios/
    ├── background_routing.md
    ├── think_routing.md
    ├── low_cost_routing.md
    ├── high_throughput_routing.md
    ├── long_context_routing.md
    ├── web_search_routing.md
    ├── image_routing.md
    └── default_routing.md
```

### Taxonomy File Format

```markdown
# Concept Name

Brief description of the routing scenario.

route:: provider_name, model_name

Detailed explanation of when this routing is triggered...

Characteristics:
- Bullet points describing behavior

Use cases:
- Example use cases

synonyms:: synonym1, synonym2, keyword phrase, another pattern
```

### Example: Low Cost Routing

```markdown
# Low Cost Routing

Low cost routing minimizes per-token spend while keeping
acceptable quality for support and automation tasks.

route:: deepseek, deepseek-chat

Low cost routing is triggered when:
- Requests mention "cost cap", "budget mode", or "cheapest option"
- Operations are tagged as back-office or non-critical
- Long-running batch jobs where cumulative spend dominates

Characteristics:
- Uses DeepSeek Chat as primary low-cost model
- Falls back to local Ollama when APIs unavailable
- Keeps streaming enabled to reduce idle billing

Use cases:
- Background knowledge-base updates
- Batch content transformations
- Cost-sensitive analytics and reporting

synonyms:: low cost, budget mode, cost cap, cheapest routing,
economy tier, deepseek cost saver
```

### Pattern Matching Algorithm

**Implementation:** `src/rolegraph_client.rs`

```rust
// 1. Load taxonomy files
pub fn load_taxonomy(&mut self) -> Result<()> {
    let taxonomy_files = self.scan_taxonomy_files()?;

    for file_path in taxonomy_files {
        let (concept, synonyms) = self.parse_taxonomy_file(&file_path)?;
        let (provider, model) = Self::parse_routing_directives(&file_path)?;

        self.routing_map.insert(concept.clone(), (provider, model));

        // Add concept and all synonyms as patterns
        patterns.push(concept.clone());
        for synonym in synonyms {
            patterns.push(synonym);
        }
    }

    // Build Aho-Corasick automaton
    self.automaton = Some(AhoCorasick::new(patterns)?);
}

// 2. Match patterns in query
pub fn query_routing(&self, query: &str) -> Option<PatternMatch> {
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();

    for mat in self.automaton.find_iter(&query_lower) {
        let pattern_id = mat.pattern().as_usize();
        let concept = self.pattern_map.get(&pattern_id)?;
        let score = self.calculate_match_score(mat.start(), mat.end(), query.len());

        if let Some((provider, model)) = self.routing_map.get(concept) {
            matches.push(PatternMatch {
                concept: concept.clone(),
                provider: provider.clone(),
                model: model.clone(),
                score,
            });
        }
    }

    // Sort by score (descending) and return best
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    matches.into_iter().next()
}

// 3. Calculate match score
fn calculate_match_score(&self, start: usize, end: usize, query_len: usize) -> f64 {
    let length = end - start;

    // Longer matches are better
    let length_score = length as f64 / query_len as f64;

    // Earlier matches slightly preferred
    let position_score = 1.0 - (start as f64 / query_len as f64) * 0.1;

    length_score * position_score
}
```

### Adding New Patterns

To add a new routing pattern:

1. **Create taxonomy file:** `docs/taxonomy/routing_scenarios/your_pattern.md`

2. **Define routing directive:**
```markdown
# Your Pattern Name

Description...

route:: provider_name, model_name

synonyms:: pattern1, pattern2, keyword phrase
```

3. **Reload taxonomy:** Restart proxy or implement hot-reload

4. **Test pattern:**
```bash
curl -X POST http://localhost:3456/v1/messages \
  -d '{"model": "claude-3-5-sonnet",
       "messages": [{"role": "user", "content": "pattern1 test"}]}'
```

5. **Verify routing:** Check logs for `Pattern matched, using RoleGraph routing`

---

## Cost & Performance Optimization

### Cost Optimization Architecture

**Components:**

1. **PricingDatabase** (`src/cost/database.rs`)
   - Stores pricing info for provider/model pairs
   - Supports input/output token pricing
   - Optional volume discounts

2. **CostCalculator** (`src/cost/calculator.rs`)
   - Estimates costs based on token usage
   - Applies discounts and markup
   - Returns CostEstimate with breakdown

3. **BudgetManager** (`src/cost/manager.rs`)
   - Creates and tracks budgets
   - Enforces spending limits
   - Supports Daily/Weekly/Monthly periods

**Workflow:**

```
Request arrives
    ↓
Estimate token usage (input + output)
    ↓
For each candidate provider/model:
    Calculate cost = (input_tokens * input_price) +
                    (output_tokens * output_price)
    Check budget: spent + cost <= limit?
    If yes, add to candidates
    ↓
Sort candidates by cost (ascending)
    ↓
Select cheapest
```

**Configuration:**

```toml
# Pricing data (example)
[[pricing]]
provider = "deepseek"
model = "deepseek-chat"
input_cost_per_1k_tokens = 0.00014
output_cost_per_1k_tokens = 0.00028
currency = "USD"

[[pricing]]
provider = "openrouter"
model = "anthropic/claude-sonnet-4.5"
input_cost_per_1k_tokens = 0.003
output_cost_per_1k_tokens = 0.015
currency = "USD"
```

**Code Location:** `src/cost/`

---

### Performance Optimization Architecture

**Components:**

1. **PerformanceDatabase** (`src/performance/database.rs`)
   - Stores performance metrics per provider/model
   - Tracks latency percentiles, throughput, success rate
   - Time-based metric expiry

2. **PerformanceTester** (`src/performance/tester.rs`)
   - Runs periodic performance tests
   - Measures latency, throughput, reliability
   - Updates performance database

3. **PerformanceConfig** (`src/performance/config.rs`)
   - Weights for scoring (latency, throughput, success_rate)
   - Thresholds for filtering (min success rate, max latency)
   - Test intervals and retention periods

**Workflow:**

```
Request arrives
    ↓
For each candidate provider/model:
    Fetch performance metrics from database
    If no recent metrics, use default score
    Calculate score = weighted sum of metrics
    Check thresholds: meets minimums?
    If yes, add to candidates
    ↓
Sort candidates by score (descending)
    ↓
Select highest-scoring
```

**Metrics Tracked:**

```rust
pub struct PerformanceMetrics {
    pub provider: String,
    pub model: String,
    pub latency_p50: f64,        // Median latency (ms)
    pub latency_p95: f64,        // 95th percentile (ms)
    pub throughput: f64,         // Tokens per second
    pub success_rate: f64,       // 0.0 to 1.0
    pub sample_count: usize,     // Number of measurements
    pub last_updated: SystemTime,
}
```

**Code Location:** `src/performance/`

---

## Configuration

### Full Configuration Example

```toml
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "your-secret-key"
timeout_ms = 60000

[router]
# Scenario routing (Phase 5 fallback)
default = "openrouter,anthropic/claude-sonnet-4.5"
background = "ollama,qwen2.5-coder:latest"
think = "deepseek,deepseek-reasoner"
long_context = "openrouter,google/gemini-2.5-flash-preview-09-2025"
long_context_threshold = 60000
web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"
image = "openrouter,anthropic/claude-sonnet-4.5"

# Pattern matching (Phase 1)
taxonomy_path = "docs/taxonomy"  # Path to taxonomy files

# Session management (Phase 2)
enable_sessions = true
session_ttl_seconds = 3600

# Cost optimization (Phase 3)
enable_cost_optimization = false  # Requires pricing data
default_budget_daily_usd = 100.0

# Performance optimization (Phase 4)
enable_performance_optimization = false  # Requires metrics
performance_test_interval_seconds = 300

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com"
api_key = "${DEEPSEEK_API_KEY}"
models = ["deepseek-chat", "deepseek-reasoner"]
transformers = ["deepseek"]

[[providers]]
name = "openrouter"
api_base_url = "https://openrouter.ai/api/v1"
api_key = "${OPENROUTER_API_KEY}"
models = [
    "anthropic/claude-sonnet-4.5",
    "google/gemini-2.5-flash-preview-09-2025",
    "perplexity/llama-3.1-sonar-large-128k-online"
]
transformers = ["openrouter"]

[[providers]]
name = "ollama"
api_base_url = "http://localhost:11434"
api_key = "ollama"
models = ["qwen2.5-coder:latest"]
transformers = ["ollama"]

[security]
enable_api_key_auth = true
allowed_origins = ["*"]
rate_limit_requests_per_minute = 60
```

### Environment Variables

```bash
# Required
export ANTHROPIC_API_KEY=sk-ant-...
export OPENROUTER_API_KEY=sk-or-...
export DEEPSEEK_API_KEY=sk-...

# Optional
export PROXY_API_KEY=your-secret-key
export RUST_LOG=info
```

---

## Request Flow

### Complete Request Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Client sends request                                      │
│    POST /v1/messages                                         │
│    Headers: x-api-key, content-type                         │
│    Body: {model, messages, ...}                             │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Authentication (16μs)                                     │
│    Validate API key from header                             │
│    Check against config.proxy.api_key                       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Token Counting (124μs)                                    │
│    Count tokens using tiktoken-rs                           │
│    - Messages (all roles)                                   │
│    - System prompt                                          │
│    - Tools definitions                                      │
│    - Images (1,275 tokens each)                            │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. Request Analysis (50μs)                                   │
│    Generate routing hints:                                  │
│    - is_background (haiku model?)                           │
│    - has_thinking (thinking field?)                         │
│    - has_web_search (web_search tool?)                     │
│    - has_images (image blocks?)                            │
│    - token_count                                           │
│    - session_id                                            │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. Multi-Phase Routing (5μs)                                │
│                                                             │
│    Phase 0: Explicit (model: "provider:model")             │
│         ↓ No match                                          │
│    Phase 1: Pattern Matching (RoleGraph taxonomy)          │
│         ↓ No match                                          │
│    Phase 2: Session-Aware Patterns                         │
│         ↓ No match                                          │
│    Phase 3: Cost Optimization                              │
│         ↓ Disabled                                          │
│    Phase 4: Performance Optimization                       │
│         ↓ Disabled                                          │
│    Phase 5: Scenario Hints (fallback)                      │
│         → RoutingDecision                                   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 6. Transformer Chain (16μs)                                 │
│    Apply provider-specific transformers:                    │
│    - OpenRouter: Add HTTP-Referer, X-Title                 │
│    - DeepSeek: Convert thinking to system prompt           │
│    - Ollama: Remove unsupported fields                     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 7. LLM Client (genai 0.4)                                   │
│    Create ChatRequest for selected provider/model          │
│    Set endpoint via ServiceTargetResolver                   │
│    Execute streaming or non-streaming request              │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 8. SSE Streaming Response                                   │
│    Stream events in Claude API format:                      │
│    - event: message_start                                  │
│    - event: content_block_start                            │
│    - event: content_block_delta                            │
│    - event: content_block_stop                             │
│    - event: message_delta                                  │
│    - event: message_stop                                   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 9. Client receives response                                 │
│    SSE stream or JSON response                              │
└─────────────────────────────────────────────────────────────┘

Total Proxy Overhead: ~211μs (0.21ms)
```

### Routing Decision Example

**Request:**
```json
{
  "model": "claude-3-5-sonnet",
  "messages": [{
    "role": "user",
    "content": "Please optimize for lowest cost when processing this batch"
  }]
}
```

**Routing Flow:**

1. **Phase 0:** Model != "provider:model" → Skip
2. **Phase 1:** Pattern matching
   - Query: "please optimize for lowest cost when processing this batch"
   - Match: `low_cost_routing` (synonym: "lowest cost")
   - Score: 0.68 (good match)
   - **Decision: Use pattern routing**
   - Provider: `deepseek`
   - Model: `deepseek-chat`
   - Scenario: `Pattern("low_cost_routing")`

3. Phases 2-5: Skipped (Phase 1 succeeded)

**Log Output:**
```
INFO Phase 1: Pattern matched, using RoleGraph routing
    concept=low_cost_routing
    score=0.68
    provider=deepseek
    model=deepseek-chat
```

---

## Testing

### Test Coverage

**Total: 186 tests passing**

#### Unit Tests (158 tests)
- `src/router.rs` - Routing logic (42 tests)
- `src/analyzer.rs` - Hint generation (15 tests)
- `src/token_counter.rs` - Token counting (25 tests)
- `src/rolegraph_client.rs` - Pattern matching (8 tests)
- `src/cost/` - Cost optimization (28 tests)
- `src/performance/` - Performance optimization (20 tests)
- Other modules (20 tests)

#### Integration Tests (28 tests)
- `tests/integration_test.rs` - Basic E2E (6 tests)
- `tests/rolegraph_routing_integration_tests.rs` - Routing (10 tests)
- `tests/session_management_e2e_tests.rs` - Sessions (12 tests)

### Key Tests

**Pattern Matching Tests:**
```rust
#[tokio::test]
async fn test_rolegraph_background_routing() {
    // Query contains "background"
    // Expects: ollama/qwen2.5-coder:latest
}

#[tokio::test]
async fn test_rolegraph_low_cost_routing() {
    // Query contains "budget mode"
    // Expects: deepseek/deepseek-chat
}

#[tokio::test]
async fn test_rolegraph_high_throughput_routing() {
    // Query contains "low latency"
    // Expects: groq/llama-3.1-8b-instant
}
```

**Routing Priority Tests:**
```rust
#[tokio::test]
async fn test_pattern_matching_routing() {
    // Pattern match should return Pattern scenario
    // Not Default scenario from hints
}

#[tokio::test]
async fn test_scenario_priority() {
    // Image should take priority over web_search
    // In scenario-based routing (Phase 5)
}
```

### Running Tests

```bash
# All tests
cargo test

# Specific test file
cargo test --test rolegraph_routing_integration_tests

# Specific test
cargo test test_rolegraph_low_cost_routing

# With output
cargo test -- --nocapture

# With logging
RUST_LOG=debug cargo test
```

---

## Performance

### Measured Latencies

From production logs (Claude Code E2E test):

| Component | Latency | Notes |
|-----------|---------|-------|
| Authentication | 16μs | API key validation |
| Token counting | 124μs | 17,245 tokens processed |
| Request analysis | 50μs | Hint generation |
| Routing (all phases) | 5μs | Pattern matching + fallback |
| Transformer chain | 16μs | Provider-specific transforms |
| **Total overhead** | **211μs** | **0.21ms** |

LLM call latency: ~1,500ms (not included, varies by provider)

### Pattern Matching Performance

**Aho-Corasick automaton:**
- **Patterns:** 200+ loaded
- **Query matching:** <1ms per query
- **Memory overhead:** <2MB
- **Build time:** <10ms at startup

**Benchmark results:**
```
test rolegraph_client::bench_pattern_matching ... bench: 285 ns/iter
test rolegraph_client::bench_query_routing ... bench: 412 ns/iter
```

### Throughput Capacity

- **Request throughput:** >4,000 req/sec (CPU-bound)
- **Token counting:** 2.8M tokens/sec
- **Pattern matching:** >2,000 queries/sec
- **Memory per request:** <500 bytes

### Optimization Overhead

**Cost optimization:**
- Token estimation: ~10μs
- Price lookup: ~5μs per provider
- Budget check: ~8μs
- Total: ~50μs for 3 providers

**Performance optimization:**
- Metrics lookup: ~12μs per provider
- Score calculation: ~5μs per provider
- Total: ~60μs for 3 providers

**Combined overhead:** ~110μs (acceptable for optimization benefits)

---

## Advanced Topics

### Custom Taxonomy Patterns

You can extend the taxonomy with custom patterns:

1. **Create custom directory:** `docs/taxonomy/custom/`

2. **Add custom pattern file:**
```markdown
# GPU Accelerated Routing

For tasks requiring GPU acceleration like image generation.

route:: runpod, stable-diffusion-xl

synonyms:: gpu, cuda, generate image, stable diffusion,
image generation, render, graphics
```

3. **Update taxonomy scanner:** Modify `src/rolegraph_client.rs:122-141`
```rust
for subdir in &["routing_scenarios", "custom"] {
    // ...
}
```

### Implementing Custom Optimization

To add a new optimization phase:

1. **Add phase in router:** `src/router.rs`
```rust
// Phase N: Custom Optimization
if let Some(custom_optimizer) = &self.custom_optimizer {
    match custom_optimizer.route(&scenario, request).await {
        Ok(decision) => return Ok(decision),
        Err(e) => {
            debug!("Custom optimization failed: {}", e);
        }
    }
}
```

2. **Implement optimizer:** `src/custom_optimizer.rs`
```rust
pub struct CustomOptimizer {
    // Your state
}

impl CustomOptimizer {
    pub async fn route(
        &self,
        scenario: &RoutingScenario,
        request: &ChatRequest,
    ) -> Result<RoutingDecision> {
        // Your optimization logic
    }
}
```

3. **Update constructor:** Add to `RouterAgent::with_all_features()`

### Monitoring and Observability

**Structured logging:**
```rust
info!(
    concept = %pattern_match.concept,
    score = pattern_match.score,
    provider = %provider_name,
    model = %model_name,
    "Phase 1: Pattern matched, using RoleGraph routing"
);
```

**Metrics to track:**
- Phase usage distribution (which phase wins most often?)
- Pattern match success rate
- Cost optimization savings
- Performance optimization improvements
- Fallback frequency

**Recommended tools:**
- **Logging:** tracing + tracing-subscriber
- **Metrics:** prometheus + grafana
- **Tracing:** OpenTelemetry
- **Alerting:** Alert on high fallback rates

---

## Troubleshooting

### Pattern Matching Not Working

**Symptom:** Requests not matching expected patterns

**Debug steps:**
1. Check taxonomy files loaded:
```bash
RUST_LOG=debug cargo run
# Look for: "Loading taxonomy from..."
# Should see: "Found N taxonomy files"
```

2. Verify pattern in taxonomy:
```bash
grep -r "your pattern" docs/taxonomy/
```

3. Test query lowercasing:
```rust
// Patterns are matched case-insensitively
query.to_lowercase()
```

4. Check pattern scoring:
```rust
// Longer matches score higher
// Earlier matches slightly preferred
```

### Cost Optimization Not Running

**Symptom:** Always falls back to scenario routing

**Debug steps:**
1. Verify cost features enabled:
```rust
self.pricing_database.is_some() &&
self.cost_calculator.is_some() &&
self.budget_manager.is_some()
```

2. Check pricing data loaded:
```bash
# Should have pricing entries in database
```

3. Verify budget exists:
```bash
# Default budget created if missing
# Check logs for: "create_budget"
```

### Routing Always Uses Fallback

**Symptom:** Phase 5 (scenario hints) used every time

**Possible causes:**
1. RoleGraph not initialized (`self.rolegraph.is_none()`)
2. No pattern matches in query
3. Pattern matches but provider not configured
4. Cost/performance optimization failing

**Fix:**
```rust
// Enable debug logging
RUST_LOG=debug cargo run

// Check each phase:
// - "Phase 0: Using explicit provider" (if explicit)
// - "Phase 1: Pattern matched" (if RoleGraph match)
// - "Phase 2: Using session-aware" (if session match)
// - "Phase 3: Using cost-optimized" (if cost enabled)
// - "Phase 4: Using performance-optimized" (if perf enabled)
// - "Phase 5: Using scenario-based" (fallback)
```

---

## Future Enhancements

### Planned Features

1. **Hot-reload taxonomy files** (without restart)
2. **Machine learning ranking** (learn from user feedback)
3. **Multi-factor optimization** (cost + performance combined)
4. **Provider health monitoring** (circuit breakers)
5. **A/B testing framework** (compare providers)
6. **Caching layer** (pattern match results, routing decisions)
7. **WASM custom routers** (user-defined routing logic)

### Phase 2 Roadmap

- **Week 2:** WASM custom router support
- **Week 3:** Session management and preferences
- **Week 4:** Advanced transformers and streaming

---

## References

### Code Locations

- **Router:** `src/router.rs`
- **RoleGraph:** `src/rolegraph_client.rs`
- **Analyzer:** `src/analyzer.rs`
- **Cost:** `src/cost/`
- **Performance:** `src/performance/`
- **Taxonomy:** `docs/taxonomy/`

### Related Documentation

- [README.md](../README.md) - Project overview
- [integration_design.md](integration_design.md) - System design
- [cost_based_prioritization_spec.md](cost_based_prioritization_spec.md) - Cost optimization
- [latency_throughput_testing_spec.md](latency_throughput_testing_spec.md) - Performance testing

### External Resources

- [Aho-Corasick Algorithm](https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm)
- [genai Rust Library](https://github.com/jeremychone/rust-genai)
- [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs)

---

**Last Updated:** 2025-10-14
**Version:** 2.0 (Post-Priority Fix)
**Maintainer:** Terraphim Team
**License:** MIT OR Apache-2.0
