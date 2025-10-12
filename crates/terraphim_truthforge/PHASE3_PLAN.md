# TruthForge Phase 3: Real LLM Integration

**Status**: Planning
**Duration**: 2-3 days
**Goal**: Replace mock methods with real OpenRouter Claude 3.5 API calls

## Prerequisites ✅

- [x] Phase 1 Foundation complete
- [x] Phase 2 Workflows complete (28/28 tests passing)
- [x] Type system fully defined
- [x] Mock implementations working correctly
- [x] Test infrastructure in place

## Scope

### 1. LLM Client Setup (Day 1)

**Tasks**:
1. Add `rig-core` dependency to Cargo.toml
2. Create `LlmClient` wrapper for OpenRouter integration
3. Implement authentication with API key from environment
4. Add cost tracking infrastructure
5. Implement retry logic with exponential backoff

**Files to Create**:
- `src/llm/mod.rs` - LLM client module
- `src/llm/client.rs` - OpenRouter client wrapper
- `src/llm/cost_tracker.rs` - Cost calculation per model
- `src/llm/error.rs` - LLM-specific error types

**Dependencies to Add**:
```toml
[dependencies]
rig-core = "0.1"  # Check latest version
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1.0", features = ["full"] }
```

**Configuration**:
- Environment variable: `OPENROUTER_API_KEY`
- Model selection per agent:
  - OmissionDetector: `anthropic/claude-3.5-sonnet`
  - BiasDetector: `anthropic/claude-3.5-haiku`
  - NarrativeMapper: `anthropic/claude-3.5-haiku`
  - Pass1/Pass2 Debaters: `anthropic/claude-3.5-sonnet`
  - ResponseGenerator: `anthropic/claude-3.5-sonnet`

### 2. Agent Integration (Day 2)

**Replace Mock Methods**:

#### OmissionDetectorAgent
- Replace `detect_omissions_mock()` with `detect_omissions()`
- Use system prompt from `config/roles/omission_detector.json`
- Parse LLM response as structured JSON (5 omission categories)
- Validate response format and calculate risk scores
- Expected tokens: ~1500 response, ~500 prompt = 2000 total
- Cost: ~$0.015 per call (Sonnet pricing)

#### BiasDetectorAgent
- Replace mock with real LLM call
- System prompt: Detect cognitive biases and framing patterns
- Expected tokens: ~1000 response
- Cost: ~$0.005 per call (Haiku pricing)

#### NarrativeMapperAgent
- Replace mock with real LLM call
- System prompt: SCCT classification and stakeholder mapping
- Expected tokens: ~1200 response
- Cost: ~$0.006 per call (Haiku pricing)

#### Pass1 Debaters
- Replace `generate_pass_one_debate_mock()` with real method
- Two sequential LLM calls (supporting → opposing)
- System prompts from respective role configs
- Expected tokens: ~2000 per debater = 4000 total
- Cost: ~$0.06 per debate (2x Sonnet)

#### Pass2 Debaters
- Replace mock methods in PassTwoOptimizer
- `generate_defensive_argument()` - uses Pass 1 vulnerabilities
- `generate_exploitation_argument()` - targets omissions
- Expected tokens: ~2500 per debater = 5000 total
- Cost: ~$0.075 per exploitation debate

#### ResponseGenerator
- Replace 3 mock strategy methods
- Each generates full ResponseDrafts structure
- System prompts: Reframe/CounterArgue/Bridge agents
- Expected tokens: ~3000 per strategy = 9000 total
- Cost: ~$0.135 for all 3 strategies

**Total Estimated Cost per Analysis**: ~$0.30 per narrative (well under $5 target)

### 3. Prompt Engineering (Day 2-3)

**System Prompts** (from config/roles/*.json):

Each agent needs:
1. Role description
2. Output format specification (JSON schema)
3. Context about previous results (for sequential agents)
4. Examples (few-shot learning)

**Response Parsing**:
- Use `serde_json::from_str()` for structured outputs
- Fallback to regex parsing if JSON fails
- Validate required fields are present
- Handle partial responses gracefully

**Error Handling**:
- API rate limits (429 errors) → retry with backoff
- Token limit exceeded → truncate input or split
- Invalid JSON response → log and retry once
- Network errors → retry up to 3 times

### 4. Streaming Support (Day 3)

**For Long Responses**:
- ResponseGenerator strategies can exceed 2000 tokens
- Implement streaming with `rig-core` stream API
- Update UI to show progressive generation
- Cache completed chunks to avoid re-generation

**Implementation**:
```rust
pub async fn generate_strategy_streaming(
    &self,
    prompt: &str,
) -> Result<impl Stream<Item = Result<String>>> {
    // Use rig-core streaming API
}
```

### 5. Testing Strategy

**Unit Tests** (mock LLM responses):
- Test prompt construction
- Test response parsing
- Test error handling
- Test cost calculation

**Integration Tests** (real API, gated by feature flag):
```rust
#[tokio::test]
#[cfg(feature = "live-llm-tests")]
#[ignore] // Run with --ignored flag
async fn test_omission_detector_with_real_llm() {
    // Requires OPENROUTER_API_KEY
    let detector = OmissionDetectorAgent::new_with_real_llm();
    let result = detector.detect_omissions("narrative").await;
    assert!(result.is_ok());
}
```

**Cost Guards**:
- Add `max_cost_per_analysis` config parameter
- Track cumulative cost during execution
- Abort if exceeding budget
- Log cost breakdown per agent

### 6. Configuration Updates

**Add to terraphim_truthforge/Cargo.toml**:
```toml
[features]
default = []
live-llm-tests = []  # Feature flag for expensive tests

[dependencies]
rig-core = "0.1"
reqwest = { version = "0.11", features = ["json", "stream"] }
futures = "0.3"
```

**Environment Variables**:
```bash
export OPENROUTER_API_KEY="sk-or-v1-..."  # pragma: allowlist secret
export TRUTHFORGE_MAX_COST="5.00"  # $5 max per analysis
export TRUTHFORGE_LLM_TIMEOUT="30"  # 30 seconds per call
```

## Implementation Order

### Day 1: Infrastructure
1. ✅ Add dependencies
2. ✅ Create LLM client module
3. ✅ Implement cost tracker
4. ✅ Add authentication
5. ✅ Implement retry logic
6. ✅ Write unit tests for client

### Day 2: Agent Integration
1. ✅ Replace OmissionDetector mock
2. ✅ Test with real API (single narrative)
3. ✅ Replace Pass1 debate mocks
4. ✅ Replace Pass2 debate mocks
5. ✅ Test complete workflow
6. ✅ Validate cost tracking

### Day 3: Response & Polish
1. ✅ Replace ResponseGenerator mocks
2. ✅ Implement streaming for long responses
3. ✅ Add comprehensive error handling
4. ✅ Write integration tests (with feature flag)
5. ✅ Update documentation
6. ✅ Performance benchmarking

## Success Criteria

- [ ] All mock methods replaced with real LLM calls
- [ ] Cost per analysis <$5 (target: <$0.50)
- [ ] Response time <90 seconds for complete workflow
- [ ] Error rate <5% (with retries)
- [ ] All existing tests still pass
- [ ] 3+ live integration tests passing
- [ ] Documentation updated with API setup instructions

## Risk Mitigation

**Risk 1**: API rate limits
**Mitigation**: Implement exponential backoff, add request queuing

**Risk 2**: Cost overruns
**Mitigation**: Hard cost limits, detailed tracking, abort on budget exceeded

**Risk 3**: Response quality
**Mitigation**: Validate against expected schemas, log failures for review

**Risk 4**: Network failures
**Mitigation**: Retry logic, timeout handling, graceful degradation

**Risk 5**: Token limits
**Mitigation**: Truncate long inputs, split into chunks if needed

## Post-Phase 3 Deliverables

1. **Working LLM Integration**
   - All agents using real Claude 3.5
   - Cost tracking operational
   - Error handling robust

2. **Updated Tests**
   - Unit tests for all LLM interactions
   - Live integration tests (feature-gated)
   - Cost tracking tests

3. **Documentation**
   - API key setup instructions
   - Cost estimation guide
   - Troubleshooting guide
   - Example usage with real LLM

4. **Performance Metrics**
   - Latency per agent
   - Total workflow time
   - Cost breakdown
   - Success/error rates

## Next Phase Preview

**Phase 4: Server Infrastructure** (Weeks 5-6)
- WebSocket API endpoint at `/api/v1/truthforge`
- Real-time progress streaming
- Redis session persistence
- Rate limiting (100 req/hr/user)

**Phase 5: UI Development** (Weeks 7-8)
- Alpha UI deployment
- Beta UI with full PRD features
- Deploy to bigbox.terraphim.cloud
