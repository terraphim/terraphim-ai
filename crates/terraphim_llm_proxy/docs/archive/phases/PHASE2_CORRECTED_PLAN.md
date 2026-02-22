# Phase 2 Corrected Plan - Align with Original Terraphim Vision

**Date:** 2025-10-12
**Status:** Planning (corrected after requirements review)
**Duration:** 4 weeks
**Goal:** Implement the **original Terraphim architectural vision** - Knowledge graph-driven intelligent routing

---

## Critical Correction

### What We Realized

After reviewing `llm_proxy_terraphim/` directory and original requirements, we discovered:

**Original Vision:**
- 3-phase routing: Runtime Analysis → Custom Router (WASM) → Pattern Matching (RoleGraph + Automata)
- Knowledge graph-based intelligence
- 52 taxonomy files defining routing concepts
- Aho-Corasick automata for pattern matching
- Deterministic, privacy-first, graph-driven decisions

**What We Built in Phase 1:**
- Phase 1 routing only (runtime analysis)
- No RoleGraph integration
- No pattern matching
- No taxonomy usage
- Basic configuration-driven routing

**Missing:** The core Terraphim differentiator! 🚨

---

## Corrected Phase 2 Objectives

### Primary Goal: Implement Original Terraphim Architecture

**Focus:** Complete the 3-phase routing design as originally specified

**Key Deliverables:**
1. ✅ Phase 1 Routing - Already complete
2. ⏳ Phase 2 Routing - WASM custom router (NEW)
3. ⏳ Phase 3 Routing - RoleGraph + Aho-Corasick (NEW, CRITICAL)
4. ⏳ Advanced transformers (feature parity)
5. ⏳ Operational features (session, hot-reload)

---

## Week 1: RoleGraph Integration & Pattern Matching (CRITICAL)

**Days 1-5: Core Terraphim Intelligence**

### Day 1: RoleGraph Dependency & Client Setup

**Tasks:**
1. Add dependencies to Cargo.toml:
   ```toml
   terraphim_rolegraph = { path = "../terraphim-ai/crates/terraphim_rolegraph" }
   terraphim_automata = { path = "../terraphim-ai/crates/terraphim_automata" }
   aho-corasick = "1.1"
   ```

2. Create `src/rolegraph_client.rs`:
   ```rust
   pub struct RoleGraphClient {
       graph: RoleGraph,
       automaton: AhoCorasick,
   }

   impl RoleGraphClient {
       pub fn new(taxonomy_path: &Path) -> Result<Self> {
           // Load taxonomy from markdown files
           // Build RoleGraph nodes/edges
           // Create Aho-Corasick automaton from synonyms
       }

       pub fn query_routing(&self, query: &str) -> Option<RoutingConcept> {
           // Match patterns in query
           // Query graph for connected providers
           // Return routing decision
       }
   }
   ```

3. Tests: 3 tests (load taxonomy, query, pattern match)

**Files:**
- `src/rolegraph_client.rs` (~250 lines)
- Tests (~100 lines, 3 tests)

### Day 2: Load Taxonomy Files

**Tasks:**
1. Implement taxonomy loading from `../llm_proxy_terraphim/taxonomy/`
2. Parse markdown files (title, description, synonyms)
3. Build RoleGraph nodes:
   - routing_scenarios (6 nodes)
   - providers (12 nodes)
   - transformers (16 nodes)
   - configuration (8 nodes)
   - operations (6 nodes)
   - technical (4 nodes)

4. Build edges (relationships between concepts)
5. Tests: 4 tests (load files, parse, build graph, query)

**Files:**
- `src/taxonomy_loader.rs` (~200 lines)
- Tests (~80 lines, 4 tests)

### Day 3: Aho-Corasick Pattern Matching

**Tasks:**
1. Extract all synonyms from taxonomy
2. Build Aho-Corasick automaton:
   ```rust
   // From basic_setup.json automata_patterns:
   patterns: [
       ("explain|analyze|understand", "default_routing"),
       ("background|index|scan", "background_routing"),
       ("think|reason|plan", "think_routing"),
       ("search|web|current|latest", "web_search_routing"),
   ]
   ```

3. Implement pattern matching on query text
4. Score matches by pattern length and specificity
5. Tests: 5 tests (patterns, scoring, priority, fallback)

**Files:**
- `src/pattern_matcher.rs` (~180 lines)
- Tests (~100 lines, 5 tests)

### Day 4: Integrate Phase 3 into RouterAgent

**Tasks:**
1. Extend RouterAgent.route() with Phase 3:
   ```rust
   async fn route(&self, request: &ChatRequest, hints: &RoutingHints) -> Result<RoutingDecision> {
       // Phase 1: Runtime Analysis (existing)
       if let Some(decision) = self.route_runtime(hints)? {
           return Ok(decision);
       }

       // Phase 2: Custom Router (stub for Week 2)
       if let Some(decision) = self.route_custom(request, hints).await? {
           return Ok(decision);
       }

       // Phase 3: Pattern Matching (NEW)
       let query = self.extract_query(request);
       if let Some(decision) = self.route_with_patterns(&query, hints)? {
           return Ok(decision);
       }

       // Phase 4: Default Fallback (existing)
       Ok(self.get_default())
   }
   ```

2. Implement route_with_patterns()
3. Connect to RoleGraphClient
4. Tests: 4 tests (Phase 3 routing, fallback, priority)

**Files:**
- `src/router.rs` (extend, +150 lines)
- Tests (+80 lines, 4 tests)

### Day 5: Integration Testing & Documentation

**Tasks:**
1. End-to-end test of graph-based routing
2. Test with taxonomy: "explain this code" → default_routing
3. Test with taxonomy: "think about this problem" → think_routing
4. Validate pattern matching works
5. Document RoleGraph integration
6. Update architecture diagrams

**Files:**
- Integration tests (~100 lines, 3 tests)
- Documentation updates

**Week 1 Deliverables:**
- RoleGraph integration complete (~980 lines)
- Pattern matching working (19 tests)
- Phase 3 routing functional
- Taxonomy files integrated
- **Core Terraphim intelligence implemented** ✅

---

## Week 2: WASM Custom Router & Advanced Transformers (Days 6-10)

### Day 6-7: WASM Custom Router (Phase 2 Routing)

**Tasks:**
1. Add wasmtime dependency (~50MB, but secure)
2. Create `src/custom_router/` module:
   - `mod.rs` - WASM runtime setup
   - `interface.rs` - Custom router interface definition
   - `sandbox.rs` - Security sandboxing (resource limits)

3. Interface design:
   ```rust
   // Custom router receives:
   struct CustomRouterInput {
       request: ChatRequest,
       config: ProxyConfig,
       hints: RoutingHints,
   }

   // Returns:
   struct CustomRouterOutput {
       provider: String,
       model: String,
   }
   ```

4. Load WASM module from config.custom_router path
5. Execute with timeout (5 seconds max)
6. Sandbox: No filesystem, no network, memory limit 10MB
7. Tests: 8 tests (load, execute, timeout, sandbox escape attempts)

**Files:**
- `src/custom_router/` (~450 lines)
- Example custom routers in Rust → WASM
- Tests (~150 lines, 8 tests)

### Day 8: Advanced Transformers (Priority 3)

**maxtoken, tooluse, reasoning:**

1. **maxtoken.rs** (~100 lines, 3 tests)
   - Map models to context limits
   - Cap max_tokens to avoid API errors

2. **tooluse.rs** (~80 lines, 2 tests)
   - Set tool_choice to "auto" or "required"
   - Optimize tool calling reliability

3. **reasoning.rs** (~90 lines, 2 tests)
   - Add thinking/reasoning parameters
   - Support DeepSeek Reasoner, etc.

### Day 9: Enhancement Transformers

**sampling, enhancetool, cleancache:**

4. **sampling.rs** (~85 lines, 2 tests)
   - Configure temperature, top_p, top_k

5. **enhancetool.rs** (~70 lines, 2 tests)
   - Add context to tool descriptions

6. **cleancache.rs** (~75 lines, 2 tests)
   - Manage cache_control blocks

### Day 10: Testing & Integration

- Update TransformerChain::from_names() for all 12 transformers
- Integration tests for transformer combinations
- Document all transformers

**Week 2 Deliverables:**
- WASM custom router (~450 lines, 8 tests)
- 6 advanced transformers (~500 lines, 13 tests)
- Full 3-phase routing complete
- Total: ~950 lines, 21 tests

---

## Week 3: Session & Operational Features (Days 11-15)

### Day 11-12: Session Management

**Tasks:**
1. `src/session/store.rs` - LRU cache (~100 lines)
2. Session tracking with metadata.user_id
3. Usage history per session
4. Session-aware routing (prefer long-context if previous was large)
5. Tests: 6 tests

### Day 13: Subagent Routing

**Tasks:**
1. Parse `<CCR-SUBAGENT-MODEL>` XML tags
2. Extract and remove from system prompts
3. Override routing decision
4. Tests: 3 tests

### Day 14: Config Hot-Reload

**Tasks:**
1. File watcher (notify crate)
2. Reload and validate
3. `/api/reload` endpoint
4. Tests: 4 tests

### Day 15: Dynamic Model Switching

**Tasks:**
1. Handle /model command
2. Session-based model preferences
3. Tests: 3 tests

**Week 3 Deliverables:**
- Session management (~150 lines, 6 tests)
- Subagent routing (~100 lines, 3 tests)
- Hot-reload (~120 lines, 4 tests)
- Model switching (~80 lines, 3 tests)
- Total: ~450 lines, 16 tests

---

## Week 4: Security & Production Polish (Days 16-20)

### Day 16-17: Security Implementation

**Tasks:**
1. Complete rate_limiter.rs (~200 lines, 6 tests)
2. Complete ssrf.rs (~250 lines, 8 tests)
3. Security test suite (15 tests from comprehensive guide)

### Day 18: Monitoring & Metrics

**Tasks:**
1. Prometheus metrics (~150 lines)
2. Enhanced logging
3. Status line (optional, ~100 lines)

### Day 19-20: CI/CD & Final Testing

**Tasks:**
1. GitHub Actions workflows
2. Multi-platform testing
3. Performance benchmarks
4. Final documentation
5. Phase 2 completion report

**Week 4 Deliverables:**
- Security complete (~450 lines, 29 tests)
- Monitoring (~250 lines)
- CI/CD operational
- Phase 2 complete

---

## Phase 2 Summary

### Total Deliverables

**Code:**
- RoleGraph integration: ~980 lines
- WASM custom router: ~450 lines
- Advanced transformers: ~500 lines
- Session management: ~450 lines
- Security: ~450 lines
- Monitoring: ~250 lines
- **Total: ~3,080 lines**

**Tests:**
- RoleGraph: 19 tests
- WASM: 8 tests
- Transformers: 13 tests
- Sessions: 16 tests
- Security: 29 tests
- **Total: ~85 new tests**

**Phase 2 Total:** 57 (Phase 1) + 85 (Phase 2) = 142 tests

---

## Success Criteria (Corrected)

### Must Have (Critical for "Terraphim" Identity)

- [ ] RoleGraph integrated with taxonomy
- [ ] Aho-Corasick pattern matching working
- [ ] 3-phase routing complete (runtime → custom → graph)
- [ ] 12+ transformers (full feature parity)
- [ ] WASM custom router support
- [ ] All 52 taxonomy files utilized

### Should Have (Production Features)

- [ ] Session management
- [ ] Subagent routing
- [ ] Config hot-reload
- [ ] Rate limiting
- [ ] SSRF protection
- [ ] Security testing

### Nice to Have (Polish)

- [ ] Monitoring and metrics
- [ ] CI/CD automation
- [ ] Status line
- [ ] Image agent

---

## Immediate Next Steps

**This Session:**
1. ✅ Document gap (REQUIREMENTS_ALIGNMENT.md)
2. ⏳ Update PHASE2_PLAN.md with corrected priorities
3. ⏳ Commit alignment analysis
4. ⏳ Prepare for Week 1 (RoleGraph focus)

**Next Session (Phase 2 Week 1 Day 1):**
1. Add terraphim_rolegraph dependency
2. Create src/rolegraph_client.rs
3. Load taxonomy files
4. Build first pattern matching test

---

**Status:** Requirements gap identified and documented | Corrected plan prioritizes RoleGraph | Ready to begin proper Phase 2
