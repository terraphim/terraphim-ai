# Lessons Learned

## 2026-01-13: Configuration Schema Extensions (Issue #55)

### Technical Discoveries

1. **Serde Default vs Rust Default Mismatch**
   - `#[derive(Default)]` uses Rust's `Default::default()` which returns type defaults (empty string, 0, false)
   - `#[serde(default = "fn_name")]` only applies when deserializing from config files
   - **Solution**: Implement `Default` trait manually when you need consistent defaults for both programmatic creation AND deserialization:
   ```rust
   // Wrong: #[derive(Default)] gives storage_backend = ""
   // Right: Manual impl gives storage_backend = "file"
   impl Default for OAuthSettings {
       fn default() -> Self {
           Self {
               storage_backend: default_storage_backend(), // "file"
               // ...
           }
       }
   }
   ```

2. **Serde Rename for Enums**
   - When using `#[serde(rename_all = "snake_case")]` on enums, test TOML must use snake_case
   - `strategy = "RoundRobin"` fails; `strategy = "round_robin"` works
   - Always verify serde rename attributes match test data

3. **Benchmark Files Need Config Updates Too**
   - Pre-commit hooks catch compilation errors in benchmarks
   - When adding fields to config structs, remember to update:
     - `src/` code
     - `tests/` files
     - `examples/` files
     - `benches/` files (easy to forget!)

### Debugging Approaches That Worked

1. **Run targeted tests first**: `cargo test --lib -- config::` isolates config-specific issues
2. **Check test output carefully**: Error messages like "expected 'file', got ''" clearly indicate Default impl issues
3. **Pre-commit hooks are helpful**: They caught the benchmark file oversight before it reached CI

### Pitfalls to Avoid

1. **Don't assume `#[derive(Default)]` works with serde defaults** - they're separate mechanisms
2. **Don't forget to update all files that construct config structs** - grep for struct names to find all occurrences
3. **Don't skip verification/validation phases** - they catch edge cases like Default impl issues

### Best Practices Discovered

1. **V-Model Disciplined Development**
   - Phase 4 (Verification) catches implementation bugs
   - Phase 5 (Validation) confirms requirements are met
   - Defect loop-back ensures fixes go through proper phases

2. **Traceability Matrix**
   - Map each requirement to its test
   - Makes it easy to verify complete coverage
   - Useful for issue closure documentation

3. **Config Extension Pattern**
   - Use `#[serde(default)]` on all new config fields for backwards compatibility
   - Implement manual Default when serde defaults differ from Rust defaults
   - Always include roundtrip tests (serialize -> deserialize -> compare)

### Test Coverage Strategy

For config changes:
1. Test default values explicitly
2. Test TOML/YAML parsing
3. Test backwards compatibility with minimal configs
4. Test roundtrip serialization
5. Run integration tests to verify config flows through system

---

## 2026-01-13: Anthropic Endpoint Test Fixes (Issue #58)

### Technical Discoveries

1. **Anthropic Streaming Returns 200 OK**
   - Per official Anthropic docs: "When receiving a streaming response via SSE, it's possible that an error can occur after returning a 200 response"
   - **Implication**: Tests expecting error status codes (500, 401, 400) for streaming endpoints are incorrect
   - Streaming responses return 200 OK immediately; errors are communicated via SSE events

2. **Proxy-Specific vs Official API Endpoints**
   - `/v1/messages` - Official Anthropic API, must match spec
   - `/v1/chat/completions` - OpenAI-compatible proxy extension
   - `/health`, `/api/sessions`, `/api/metrics` - Proxy-specific, no official spec
   - **Lesson**: Classify endpoints before writing tests; different standards apply

3. **Proxy Error Codes for Backend Failures**
   - 502 BAD_GATEWAY - Backend provider unreachable
   - 504 GATEWAY_TIMEOUT - Backend provider timeout
   - These are correct proxy behavior when backend providers are unavailable (e.g., fake API keys in tests)

4. **Official Anthropic HTTP Error Codes**
   | Code | Type | Description |
   |------|------|-------------|
   | 400 | invalid_request_error | Issue with format or content |
   | 401 | authentication_error | Issue with API key |
   | 403 | permission_error | API key lacks permission |
   | 404 | not_found_error | Resource not found |
   | 413 | request_too_large | Request exceeds 32 MB |
   | 429 | rate_limit_error | Rate limit exceeded |
   | 500 | api_error | Internal server error |
   | 529 | overloaded_error | API temporarily overloaded |

### Debugging Approaches That Worked

1. **Print actual status codes**: When tests fail with unexpected status, add debug print to see what's actually returned
   ```rust
   println!("Actual status: {:?}", response.status());
   ```

2. **Validate against official docs first**: Before fixing tests, check official API documentation
   - Anthropic Messages API: https://platform.claude.com/docs/en/api/messages
   - Anthropic Error Codes: https://platform.claude.com/docs/en/api/errors

3. **Check server implementation**: For proxy-specific endpoints, the server implementation IS the specification

### Pitfalls to Avoid

1. **Don't assume HTTP error codes for streaming** - Streaming establishes connection first (200 OK), then streams errors
2. **Don't mix up response structures** - `checks.providers` vs `providers` at root level matters
3. **Don't forget proxy error codes** - 502/504 are valid when backend unavailable

### Best Practices Discovered

1. **Endpoint Classification Table**
   - Document which endpoints are official API vs proxy-specific
   - Apply different testing standards accordingly

2. **Status Code Assertions for Streaming**
   ```rust
   // Correct: Accept OK for streaming, plus error codes
   assert!(
       response.status() == StatusCode::OK
           || response.status() == StatusCode::INTERNAL_SERVER_ERROR
           || response.status() == StatusCode::BAD_GATEWAY
           // ... other valid codes
   );
   ```

3. **V-Model for Test Fixes**
   - Phase 1: Research the API specification
   - Phase 2: Design fixes with traceability matrix
   - Phase 3: Implement fixes
   - Phase 4: Verify all tests pass
   - Creates audit trail and prevents regression

---

## 2026-01-30: Smart Plan Routing Feature Implementation

### Technical Discoveries

1. **Adding New Config Fields Requires Updates Across Many Files**
   - When adding `plan_implementation` to `RouterSettings`, had to update:
     - `src/router.rs` (2 test configs)
     - `src/server.rs` (1 test config)
     - `src/config.rs` (1 test config)
     - `src/management/` (config_manager.rs, handlers.rs, routes.rs)
     - `tests/` (multiple integration test files)
     - `examples/` (demo files)
   - **Solution**: Use `sed` or search-and-replace across all files when adding fields to widely-used structs
   - **Better Solution**: Consider using a builder pattern or default implementations to reduce boilerplate

2. **Clippy Warnings Become CI Failures**
   - CI uses `-D warnings` flag which converts warnings to errors
   - Common issues after adding new code:
     - Unused imports (remove them)
     - Unused variables (prefix with `_`)
     - Dead code (add `#[allow(dead_code)]` or remove)
     - Collapsible if statements (combine conditions)
   - **Solution**: Run `cargo clippy -- -D warnings` locally before pushing

3. **Nested If Collapsing Requires Care**
   ```rust
   // Before (triggers clippy warning)
   if condition_a {
       if condition_b {
           do_something();
       }
   }
   
   // After (cleaner, but watch for extra closing braces)
   if condition_a && condition_b {
       do_something();
   }
   ```
   - **Pitfall**: When editing nested blocks, it's easy to leave extra `}` characters
   - **Solution**: Always run `cargo build` after refactoring control flow

4. **Test Config Functions Proliferate**
   - `create_test_config()`
   - `create_test_config_with_think()`
   - `create_test_config_with_plan_impl()`
   - **Problem**: Each new scenario requires another helper function
   - **Better Approach**: Use a builder pattern or configuration presets

### Debugging Approaches That Worked

1. **Grep for struct constructors**
   ```bash
   grep -r "RouterSettings {" src/ tests/ examples/
   ```
   - Finds all places that need updating when adding struct fields

2. **Sed for batch fixes**
   ```bash
   sed -i 's/think: None,/think: None,\n                plan_implementation: None,/g' src/**/*.rs
   ```
   - Efficiently updates multiple files

3. **Compile after each refactor**
   - Don't batch multiple changes without compiling
   - Makes it easier to identify which change introduced the error

### Pitfalls to Avoid

1. **Don't add fields to widely-used structs without a migration plan**
   - Every test config, example, and benchmark needs updating
   - Creates significant maintenance burden

2. **Don't ignore clippy until CI fails**
   - Fix warnings incrementally as you code
   - Much easier than batch-fixing dozens of warnings

3. **Don't use complex nested if statements**
   - Combine conditions when possible
   - Easier to read and avoids clippy warnings

### Best Practices Discovered

1. **Config Field Addition Checklist**
   ```markdown
   - [ ] Add field to struct definition
   - [ ] Add `#[serde(default)]` if optional
   - [ ] Update all test configs
   - [ ] Update all example configs  
   - [ ] Update benchmark configs
   - [ ] Run clippy with -D warnings
   - [ ] Run full test suite
   ```

2. **Disciplined Implementation (V-Model)**
   - Phase 1: Research and requirements gathering
   - Phase 2: Design with clear acceptance criteria
   - Phase 3: Implement with tests
   - Phase 4: Verify (run clippy, tests, fmt)
   - Phase 5: Validate against requirements
   - Phase 6: Document and promote

3. **Keyword-Based Routing Pattern**
   - Define clear keywords for each routing scenario
   - Check for overlapping keywords (priority matters)
   - Document in taxonomy files
   - Test with real API calls

### Feature Development Workflow

For adding new routing scenarios:
1. Define scenario in taxonomy documentation
2. Add variant to RoutingScenario enum
3. Add config field to RouterSettings
4. Implement detection logic in determine_scenario()
5. Add routing resolution in get_provider_model_for_scenario()
6. Write unit tests
7. Write integration tests
8. Update documentation
9. Create marketing materials (if user-facing)
10. Validate with live API calls

### Testing Strategy

For routing features:
1. Unit test: `test_*_routing()` - Test detection logic
2. Integration test: Test end-to-end with mock providers
3. Live test: Real API calls (with cost awareness)
4. Edge cases: Overlapping keywords, missing config, fallbacks

---

## 2026-01-30: Marketing Content Creation

### Content Strategy

1. **Target Audience**: AI developers, self-hosters, cost-conscious users
2. **Tone**: Casual, technical, focused on practical benefits
3. **Key Messages**:
   - Save money with smart routing (60-90% cost reduction)
   - Continue working when primary AI hits limits
   - Automatic model selection based on intent

### Content Types Created

1. **Technical Documentation**: `docs/OPENCLAW_BACKUP_GUIDE.md`
   - 250+ lines of comprehensive guide
   - Real examples with expected outputs
   - Cost comparison tables
   - Troubleshooting section

2. **Blog Post**: `blog/2026-01-30-openclaw-smart-backup.md`
   - Hook: The panic moment (running out of tokens)
   - Problem: Cost and workflow interruption
   - Solution: Smart routing with real examples
   - Results: Actual cost savings

3. **Social Media**:
   - Twitter thread: 10 tweets ready to post
   - Reddit post: Formatted for r/selfhosted, r/LocalLLaMA, r/programming

### Best Practices for Technical Marketing

1. **Show, don't just tell**
   - Include actual curl commands
   - Show real API responses
   - Provide cost breakdowns

2. **Lead with the problem**
   - "Running out of tokens mid-project"
   - Creates emotional connection
   - Establishes need

3. **Quantify benefits**
   - "$15 vs $0.25" not just "cheaper"
   - "60x cheaper" not just "saves money"
   - Specific numbers build credibility

4. **Make it actionable**
   - Complete setup instructions
   - Ready-to-use config files
   - One-command demos

### Validation Approach

Before publishing:
1. Test all commands against live proxy
2. Verify actual API responses match documentation
3. Check cost estimates with real provider pricing
4. Run through setup steps from scratch

---

## 2026-02-01: Pattern-Based Intelligent Routing

### Technical Discoveries

1. **Model Aliasing Bypasses Pattern Matching**
   - When a client sends `model: "claude-sonnet-4-5"`, it gets aliased to `openrouter,anthropic/claude-sonnet-4.5`
   - This creates an explicit provider specification (Phase 0 routing)
   - Pattern matching (Phase 1) never runs because explicit provider takes precedence
   - **Solution**: Use `model: "auto"` to trigger pattern-based routing on message content

2. **Provider Must Exist for Pattern Routing to Work**
   - Taxonomy files define routes like `route:: deepseek, deepseek-reasoner`
   - If the proxy config doesn't have a `deepseek` provider, the pattern match is ignored
   - Falls through to default routing silently
   - **Debugging**: Enable debug logging to see "Pattern match found but provider not configured"
   - **Solution**: Ensure all providers referenced in taxonomy files are configured

3. **Aho-Corasick Pattern Matching is Case-Insensitive**
   - User message is lowercased before matching
   - Patterns in taxonomy files are also lowercased during loading
   - "THINK" matches "think" pattern
   - **Benefit**: No need to worry about case in user messages

4. **Pattern Score is Relative to Message Length**
   - Score = (match_length / query_length) * position_factor
   - OpenClaw sends ~11K tokens of system context
   - A 5-word match in 11K tokens scores lower than in a 10-word message
   - **Implication**: Pattern matching works best with focused user messages

5. **1Password Path Specifics**
   - Paths are case-sensitive and exact
   - `deepseek-api-key` vs `deepseek-api-keys` (note the 's') matters
   - Use `op item get` to verify exact field names
   - **Example**: `op://TerraphimPlatform/TruthForge.api-keys/deepseek-api-keys`

### Debugging Approaches That Worked

1. **Enable Debug Logging for Routing Decisions**
   ```bash
   RUST_LOG=debug ./target/release/terraphim-llm-proxy -c config.toml
   ```
   Look for:
   - "Phase 0: Using explicit provider specification"
   - "Phase 1: Priority pattern match"
   - "Pattern match found" / "No pattern match"

2. **Check Taxonomy Loading at Startup**
   ```
   INFO Found 9 taxonomy files
   INFO Built automaton with 118 patterns
   ```
   If patterns not loaded, check taxonomy path exists

3. **Test Pattern Matching Directly**
   ```bash
   curl -X POST http://127.0.0.1:3456/v1/messages \
     -d '{"model": "auto", "messages": [{"role": "user", "content": "think step by step"}]}'
   ```
   Grep logs for "scenario=Pattern"

4. **Verify Provider Exists in Config**
   ```bash
   grep -A5 "name = \"deepseek\"" config.toml
   ```

### Pitfalls to Avoid

1. **Don't forget to add providers for taxonomy routes**
   - Easy to add taxonomy file but forget to add provider
   - Pattern matches silently fail

2. **Don't expect pattern matching with model aliases**
   - `claude-sonnet-4-5` -> explicit provider -> no pattern matching
   - Use `auto` for pattern-based routing

3. **Don't assume large system contexts match well**
   - OpenClaw includes ~11K tokens of system prompts
   - Pattern scores are diluted by large context
   - Consider extracting only user message for matching

4. **Don't mix up 1Password field names**
   - Verify exact field names before using in .env files
   - The proxy just reports "Environment variable not set" without specifics

### Best Practices Discovered

1. **Pattern Routing Testing Protocol**
   ```bash
   # 1. Start proxy with debug logging
   RUST_LOG=debug ./target/release/terraphim-llm-proxy -c config.toml

   # 2. Send test request with model="auto"
   curl -X POST http://127.0.0.1:3456/v1/messages \
     -d '{"model": "auto", "messages": [{"role": "user", "content": "I need to think step by step"}]}'

   # 3. Grep for routing decision
   grep "scenario=Pattern" /tmp/proxy.log
   ```

2. **Taxonomy File Checklist**
   ```markdown
   - [ ] `route::` directive with provider,model
   - [ ] `synonyms::` line with comma-separated keywords
   - [ ] Provider exists in config
   - [ ] Provider has model in models list
   - [ ] Restart proxy to load new patterns
   ```

3. **Multi-Client Configuration Strategy**
   - Each client may send different model names
   - Use model aliases to normalize to `provider,model` format
   - Consider a "pattern-routing" alias that maps to `auto`

4. **Documentation Pattern**
   - Research document: Problem understanding
   - Design document: Implementation plan
   - Multi-client guide: Integration instructions
   - Blog posts: Marketing and examples
   - All linked from README

### Testing Strategy for Pattern Routing

1. **Unit Test**: Pattern matching function with known inputs
2. **Integration Test**: Full routing with mock providers
3. **Live Test**: Real API calls with `model: "auto"`
4. **Regression Test**: Verify existing routes still work

### Key Metrics for Pattern Routing

| Metric | Expected | Actual |
|--------|----------|--------|
| Patterns loaded | >100 | 118 |
| Match time | <1ms | <0.1ms |
| Routing overhead | <1ms | 0.21ms |
| Memory for automaton | <1MB | ~500KB |

---

## 2026-02-02: Licensing and Sponsor-Only Access Model

### Technical Discoveries

1. **37signals O'Saasy vs FSL-1.1-MIT**
   - O'Saasy is simple (MIT + SaaS non-compete) but vague
   - FSL-1.1-MIT has patent/trademark clauses, clearer definitions
   - O'Saasy "primary value" language invites legal disputes
   - FSL-1.1-MIT auto-converts to MIT after 2 years (certainty)

2. **GitHub Sponsors-Only Repos**
   - Repository must be PRIVATE before linking to tier
   - Sponsors get automatic GitHub invitation
   - Only monthly tiers support repo access (not one-time)
   - Repo must be owned by same org/user as sponsors listing

3. **Provider URL Path Differences**
   | Provider | URL Path |
   |----------|----------|
   | Groq | `/openai/v1/chat/completions` |
   | Cerebras | `/v1/chat/completions` |
   | OpenRouter | `/api/v1/chat/completions` |

4. **genai Library URL Construction**
   - genai's OpenAI adapter strips base URL path
   - Adds `/openai/v1/chat/completions` automatically
   - Works for Groq, breaks Cerebras
   - **Solution**: Dedicated clients (CerebrasClient) for non-standard paths

### Debugging Approaches That Worked

1. **Add resolved_endpoint to debug logs** - Reveals actual URL being called
2. **Test with curl first** - Confirms API works before blaming code
3. **Compare working vs broken** - Groq worked, Cerebras didn't = URL difference
4. **GitHub API for sponsors info** - `gh api graphql` to check tier configuration

### Pitfalls to Avoid

1. **Don't assume all OpenAI-compatible APIs use same path** - They don't
2. **Don't use O'Saasy if you need patent protection** - FSL is more robust
3. **Don't try to add repo to one-time sponsor tiers** - Only monthly works
4. **Don't forget FUNDING.yml** - Enables the "Sponsor" button on repo page

### Best Practices Discovered

1. **License Selection Checklist**
   - [ ] Does it have patent grant?
   - [ ] Does it have trademark protection?
   - [ ] Are definitions clear and specific?
   - [ ] Is there a conversion path to full open source?
   - [ ] Is it backed by established legal team?

2. **Sponsor-Only Repo Setup**
   ```
   1. Enable GitHub Sponsors for org/user
   2. Create tier with repo access benefit
   3. Make repo PRIVATE
   4. Link repo to tier in sponsor dashboard
   5. Add FUNDING.yml to repo
   6. Update README with sponsor info
   ```

3. **Dedicated Provider Client Pattern**
   ```rust
   // When provider needs custom URL construction
   pub struct ProviderClient {
       client: reqwest::Client,
   }
   
   impl ProviderClient {
       pub async fn send_request(&self, provider: &Provider, model: &str, request: &ChatRequest) -> Result<Response> {
           // Build URL correctly for this provider
           let endpoint = format!("{}/v1/chat/completions", provider.api_base_url);
           // Direct HTTP call, bypassing genai
       }
   }
   ```

### Key Metrics

| Metric | Value |
|--------|-------|
| Sponsor minimum tier | $3/month |
| License conversion | 2 years -> MIT |
| Cerebras latency | <100ms TTFT |
| Groq latency | <100ms TTFT |

---

## 2026-02-05: Release v0.1.6 and Remotion Video Creation

### Technical Discoveries

1. **Clippy `const_is_empty` Warning on Rust Stable 1.93.0**
   - Rust stable now has a clippy lint `const_is_empty` that warns about `.is_empty()` on const strings
   - The lint considers the result "always known" at compile time
   - **Problem**: Test code like `let default_provider = "";` followed by `if default_provider.is_empty()` triggers warning
   - **Solution**: Convert to runtime String: `let default_provider = "".to_string();`
   - **Alternative**: Use `#[allow(clippy::const_is_empty)]` if const is intentional

2. **Cross-Compilation OpenSSL Issues**
   - `cargo cross` for Linux targets fails with "Could not find directory of OpenSSL installation"
   - Cross images don't include OpenSSL dev headers by default
   - **Solution**: Use Docker directly with `rust:latest` image:
   ```dockerfile
   FROM rust:latest
   RUN apt-get update && apt-get install -y pkg-config libssl-dev
   WORKDIR /app
   COPY . .
   RUN cargo build --release
   ```
   - **Note**: genai crate requires edition 2024, so may need nightly: `rustup default nightly`

3. **Remotion Project Setup Without Interactive CLI**
   - `npx create-video` shows interactive menu that can block automation
   - **Solution**: Manually create project structure:
   ```bash
   mkdir llm-proxy-video && cd llm-proxy-video
   npm init -y
   npm install remotion @remotion/cli @remotion/transitions @remotion/google-fonts
   ```
   - Create `src/index.ts` with `registerRoot(RemotionRoot)`

4. **Remotion TransitionSeries for Scene Sequencing**
   - Use `@remotion/transitions` for smooth scene changes
   - `TransitionSeries.Sequence` wraps each scene
   - `TransitionSeries.Transition` with `fade()` between scenes
   - Calculate total duration accounting for transition overlaps:
   ```typescript
   const TOTAL_SECONDS = scenesDuration - (numTransitions * TRANSITION_DURATION);
   ```

5. **Terminal Fullscreen Styling Pattern**
   - Use conditional sizing based on `fullScreen` prop
   - Key differences for fullscreen:
     | Property | Normal | Fullscreen |
     |----------|--------|------------|
     | fontSize | 14px | 28px |
     | bodyPadding | 16px | 48px |
     | dotSize | 12px | 16px |
     | lineMarginBottom | 4px | 12px |
   - Set `height: '100%'` and `flex: 1` for fullscreen fill

### Debugging Approaches That Worked

1. **Docker Build for Linux Binaries**
   - When cross fails, use native Docker builds
   - Copy binary out with `docker cp`
   - Works around all cross-compilation toolchain issues

2. **Remotion Preview for Rapid Iteration**
   - `npx remotion preview` opens browser with live reload
   - Much faster than full renders for testing animations
   - Frame-by-frame scrubbing helps debug timing issues

3. **Check genai Edition Requirements**
   - Some crates require Rust edition 2024 (nightly only)
   - Error: "feature `edition2024` is required"
   - Quick fix: `rustup default nightly` in build environment

### Pitfalls to Avoid

1. **Don't use `cargo cross` when OpenSSL is required**
   - Cross images lack OpenSSL headers
   - Use Docker with `rust:latest` + `libssl-dev` instead

2. **Don't rely on `npx create-video` in automation**
   - Interactive prompts block CI/scripts
   - Manually create package.json with dependencies

3. **Don't forget transition duration in total time calculation**
   - Transitions overlap scenes
   - Total = sum(scenes) - (numTransitions * transitionDuration)

4. **Don't hardcode sizes for responsive components**
   - Use props like `fullScreen` to conditionally set sizes
   - Makes components reusable across different contexts

### Best Practices Discovered

1. **Docker Build Script for Linux Releases**
   ```bash
   # Build Linux binary using Docker
   docker build -t llm-proxy-builder -f Dockerfile.build .
   docker create --name temp-builder llm-proxy-builder
   docker cp temp-builder:/app/target/release/terraphim-llm-proxy ./releases/
   docker rm temp-builder
   ```

2. **Remotion Color System Pattern**
   ```typescript
   // src/styles/colors.ts
   export const colors = {
     // Background hierarchy
     ink900: '#0a0a0a',   // Primary background
     ink700: '#2d2d2d',   // Secondary (headers)

     // Text hierarchy
     white: '#ffffff',     // Primary text
     silver400: '#9e9e9e', // Secondary text
     silver200: '#d4d4d4', // Muted text

     // Semantic colors
     signalSuccess: '#22c55e',
     dotRed: '#ef4444',
     dotYellow: '#eab308',
     dotGreen: '#22c55e',
   };
   ```

3. **Typewriter Effect Implementation**
   ```typescript
   const charsToShow = Math.min(
     Math.floor(localFrame * charsPerFrame),
     line.text.length
   );
   const displayText = line.text.slice(0, charsToShow);
   const showCursor = charsToShow < line.text.length;
   ```

4. **GitHub Release with Multiple Binaries**
   ```bash
   # Create release and upload binaries
   gh release create v0.1.6 \
     --title "v0.1.6" \
     --notes "Release notes" \
     releases/terraphim-llm-proxy-linux-amd64 \
     releases/terraphim-llm-proxy-darwin-arm64 \
     releases/terraphim-llm-proxy-darwin-amd64
   ```

### Key Metrics

| Metric | Value |
|--------|-------|
| Release binaries | 3 platforms |
| Linux binary size | 9.5 MB |
| macOS binary size | 7.6-7.8 MB |
| Video duration | 24 seconds |
| Video file size | 2 MB |
| Video resolution | 1920x1080 |
| Video FPS | 30 |

---

## 2026-02-02: Z.ai Coding Plan Integration

### Technical Discoveries

1. **Z.ai Has Dual API Endpoints**
   - Anthropic-compatible: `https://api.z.ai/api/anthropic`
   - OpenAI-compatible: `https://api.z.ai/api/coding/paas/v4`
   - **Key insight**: Use OpenAI endpoint for proxy integration (genai uses OpenAI adapter)
   - Anthropic endpoint requires Anthropic-format requests, but genai sends OpenAI format

2. **Z.ai GLM-4.7 "Thinking" Models Use Different Field**
   - Standard OpenAI: Response content in `choices[0].message.content`
   - Z.ai thinking: Response in `choices[0].message.reasoning_content`
   - **Solution**: Fallback logic to check both fields
   ```rust
   let content = message
       .and_then(|msg| msg.get("content"))
       .and_then(|c| c.as_str())
       .filter(|s| !s.is_empty())
       .or_else(|| {
           // Fall back to reasoning_content for thinking models
           message
               .and_then(|msg| msg.get("reasoning_content"))
               .and_then(|c| c.as_str())
       })
       .unwrap_or("");
   ```

3. **Model Mappings Bypass Scenario Routing**
   - Config model mappings transform "auto" -> "zai,glm-4.7"
   - This creates explicit provider specification (Phase 0)
   - Router returns immediately, never reaches think/background detection (Phase 5)
   - **Implication**: Scenario keywords only work for unmapped models
   - **Design decision**: Model mappings take precedence (intentional)

4. **Performance Optimization Also Bypasses Scenario Routing**
   - Phase 4 (performance optimization) returns if performance data available
   - Phase 5 (scenario keywords) only reached as fallback
   - **Routing priority** (by design):
     1. Explicit provider (`provider,model`)
     2. RoleGraph patterns
     3. Cost optimization
     4. Performance optimization
     5. Scenario keywords (think/background)

5. **Dedicated Provider Client Pattern Established**
   | Provider | Client | Reason |
   |----------|--------|--------|
   | Groq | GroqClient | `/openai/v1/` path |
   | Cerebras | CerebrasClient | `/v1/` path |
   | Z.ai | ZaiClient | `/api/coding/paas/v4/` path |
   - All bypass genai library for correct URL construction

### Debugging Approaches That Worked

1. **Check provider logs for routing decisions**
   ```bash
   RUST_LOG=info ./target/release/terraphim-llm-proxy -c config.toml
   # Look for: "Phase N: Using X routing"
   ```

2. **Test direct API access first**
   ```bash
   curl https://api.z.ai/api/coding/paas/v4/chat/completions \
     -H "Authorization: Bearer $ZAI_API_KEY" \
     -d '{"model":"glm-4.7","messages":[...]}'
   ```

3. **Create minimal test config without model mappings**
   - Isolates intelligent routing behavior
   - Shows true scenario detection

4. **Check raw JSON response for thinking models**
   ```bash
   curl ... | jq '.choices[0].message | keys'
   # ["reasoning_content"] vs ["content"]
   ```

### Pitfalls to Avoid

1. **Don't use Z.ai's Anthropic endpoint with genai**
   - genai sends OpenAI format regardless of endpoint
   - Use OpenAI-compatible endpoint instead

2. **Don't expect scenario routing with model mappings**
   - Mappings create explicit provider, bypassing scenario detection
   - Use unmapped model names to test scenario routing

3. **Don't assume all OpenAI responses have `content` field**
   - Thinking/reasoning models may use different field names
   - Always implement fallback extraction

4. **Don't forget to add "zai" case to client.rs**
   - Both streaming and non-streaming handlers need updating
   - Easy to add one and forget the other

### Best Practices Discovered

1. **Provider Client Checklist**
   ```markdown
   When adding new provider:
   - [ ] Check actual API endpoint path
   - [ ] Check response format (especially thinking models)
   - [ ] Create dedicated client if path differs from genai default
   - [ ] Add provider name to client.rs routing switch
   - [ ] Add to both streaming and non-streaming handlers
   - [ ] Test direct API before proxy integration
   ```

2. **Config Testing Strategy**
   - Test with model mappings (production behavior)
   - Test without model mappings (routing behavior)
   - Use explicit provider syntax for direct provider testing
   - Check logs for routing phase decisions

3. **Fallback Content Extraction Pattern**
   ```rust
   // Try standard field first, then provider-specific alternatives
   let content = standard_field
       .or_else(|| alternative_field_1)
       .or_else(|| alternative_field_2)
       .unwrap_or_default();
   ```

### Key Metrics

| Provider | Cost | Status |
|----------|------|--------|
| Z.ai | $3/month unlimited | Working |
| Kimi | Subscription | 404 (expired) |
| MiniMax | Pay-per-use | 404 (unavailable) |

### Provider URL Reference

| Provider | API Base URL | Notes |
|----------|--------------|-------|
| Z.ai (OpenAI) | `https://api.z.ai/api/coding/paas/v4` | Use this |
| Z.ai (Anthropic) | `https://api.z.ai/api/anthropic` | Don't use with genai |
| Groq | `https://api.groq.com/openai/v1` | Works with genai |
| Cerebras | `https://api.cerebras.ai/v1` | Needs CerebrasClient |

---

## 2026-02-08: Claude OAuth API Integration (Bearer + API Key Modes)

### Technical Discoveries

1. **Anthropic API Authentication is Header-Specific**
   - `x-api-key: <key>` -- standard API key auth
   - `Authorization: Bearer <token>` -- OAuth token auth
   - These are NOT interchangeable -- you cannot put a Bearer token in `x-api-key`
   - The genai library's Anthropic adapter hardcoded `x-api-key`
   - **Solution**: Added `AuthData::BearerToken(String)` variant to genai fork
   - This was ~30 lines changed vs the alternative: a 380-line custom `AnthropicOAuthClient`

2. **Non-Exhaustive Enum Matches After Fork Updates**
   - Updating the genai fork added `ChatStreamEvent::ThoughtSignatureChunk`
   - Existing match statements in `server.rs` became non-exhaustive
   - **Solution**: Always add a handler for new variants when updating dependencies with enums
   - Use `_ => {}` cautiously -- explicit arms are better for documenting expected behavior

3. **OAuth Scopes Must Match API Requirements**
   - Default scopes `["openid", "email", "profile"]` were insufficient for Anthropic
   - Correct scopes: `["user:inference", "user:profile"]` for Bearer mode
   - API key mode additionally needs `"org:create_api_key"`
   - **Discovery method**: Reading Anthropic's OAuth documentation and pi-mono/coding-agent reference

4. **Anthropic API Key Creation Endpoint**
   - `POST https://api.anthropic.com/api/oauth/claude_cli/create_api_key`
   - Requires Bearer token from OAuth flow
   - Returns `{ "api_key": "sk-ant-..." }` (permanent, no expiry)
   - Stored in `TokenBundle.metadata["api_key"]` alongside the OAuth token

5. **`create_client_for_provider()` Auth Override Pattern**
   - Adding `auth_override: Option<AuthData>` parameter to the genai client creation function
   - Allows OAuth tokens to override the config's static `provider.api_key`
   - Clean separation: auth resolution happens before client creation
   - Pre-fetch pattern: resolve auth once, use for both streaming and non-streaming

6. **Integration Test Import Differences**
   - `terraphim_llm_proxy::create_server` -- does NOT work (not re-exported at crate root)
   - `terraphim_llm_proxy::server::create_server` -- correct path
   - `terraphim_llm_proxy::webhooks::WebhookSettings` -- not in config module
   - Always check existing integration tests for import patterns before writing new ones

7. **`/oauth/providers` Returns Flat Array**
   - Response is `Json<Vec<ProviderInfo>>`, not `{"providers": [...]}`
   - Test must use `json.as_array()` not `json["providers"].as_array()`
   - Lesson: Check handler return types before writing assertions

### Debugging Approaches That Worked

1. **Match existing test patterns**: The `server_integration_tests.rs` file was the perfect reference for imports, body reading (`BodyExt::collect().to_bytes()`), and config construction.

2. **Compile-first approach**: Fix compilation errors before running tests. `cargo test --no-run` catches type errors without waiting for test execution.

3. **Incremental testing**: Run specific test file first (`--test claude_oauth_integration_tests`) before the full suite. Faster feedback loop.

### Pitfalls to Avoid

1. **Don't assume `RouterSettings` derives `Default`** -- it has a required `default: String` field, so you must construct all fields explicitly in tests.

2. **Don't use `axum::body::to_bytes(body, usize::MAX)`** -- use `http_body_util::BodyExt` with `body.collect().await.unwrap().to_bytes()` instead. The former has type inference issues.

3. **Don't modify genai fork without considering downstream impacts** -- adding enum variants breaks existing match statements in all consumers.

4. **Don't assume `FileTokenStore::new` is sync** -- it's async and takes `PathBuf`, not `&String`. Always check function signatures.

### Best Practices Discovered

1. **Fork Modification Strategy**
   - Make minimal changes (~30 lines vs ~380 lines for alternative)
   - Add unit tests to the fork itself (4 tests added)
   - Create PR, merge, update downstream Cargo.lock
   - Commit downstream fix for breaking changes in the same commit

2. **Builder Pattern for Optional Config**
   ```rust
   let client = LlmClient::new(storage_path)?
       .with_claude_oauth(auth_mode, anthropic_beta);
   ```
   - Keeps constructor simple for common cases
   - Optional features added via chained builders
   - No breaking changes to existing callers

3. **Config Validation at Startup**
   - `validate_claude_oauth()` logs warnings, does not fail
   - Missing `client_id`, wrong scopes, unknown `auth_mode` all produce actionable log messages
   - Users see problems immediately on startup, not when first request fails

4. **Token Fallback Chain**
   - OAuth token/key > config api_key > error
   - Graceful degradation: if OAuth is not configured or has no tokens, fall back silently
   - Only log at debug level for fallback (not warn/error)

### Key Metrics

| Metric | Before | After |
|--------|--------|-------|
| Integration tests | 21 | 26 |
| Config tests | ~15 | ~18 |
| Total tests | 687 | 699 |
| genai fork changes | 0 | ~30 lines (BearerToken) |
| New source files | 0 | 2 (claude_api_key.rs, claude_oauth_integration_tests.rs) |

---

## 2026-02-07: TokenManager with Cross-Process File Locking

### Technical Discoveries

1. **fd-lock Guards Cannot Cross Async Boundaries**
   - `fd_lock::RwLock::try_write()` returns `RwLockWriteGuard` with a lifetime tied to the `RwLock`
   - This guard cannot be held across `.await` points because it borrows the lock
   - `into_inner()` does not exist on `RwLockWriteGuard` (contrary to what docs might suggest)
   - **Solution**: Use atomic lock file creation (`O_EXCL` / `create_new(true)`) instead
   - Lock state is the file's existence, not a held file descriptor
   - Works naturally with async because no borrow is held
   ```rust
   // What doesn't work:
   let lock = fd_lock::RwLock::new(file);
   let guard = lock.try_write()?; // borrows lock
   do_async_work().await; // ERROR: guard borrows lock across await

   // What works:
   OpenOptions::new().write(true).create_new(true).open(lock_path)?;
   // Lock is the file's existence, no borrow needed
   do_async_work().await; // Fine
   std::fs::remove_file(lock_path)?; // Release lock
   ```

2. **O_EXCL Atomic File Creation for Cross-Process Locking**
   - `OpenOptions::new().create_new(true)` maps to `O_CREAT | O_EXCL` on POSIX
   - Atomic on all platforms -- if the file already exists, the operation fails
   - Combined with an RAII `LockGuard` that removes the file on drop
   - Stale lock detection: check file mtime, remove if >30 seconds old
   - Exponential backoff retry: 50ms, 100ms, 200ms, 400ms... up to 2s cap, 30s total timeout

3. **Re-Read-After-Lock Pattern**
   - Critical for multi-instance deployments
   - After acquiring the lock, re-read the token from disk
   - Another instance may have refreshed the token while we waited
   - Avoids redundant refresh calls to the OAuth provider
   ```rust
   // 1. Acquire lock
   // 2. Re-read from store (not from the in-memory copy)
   // 3. If still needs refresh -> do refresh
   // 4. If now valid -> return (another instance refreshed it)
   ```

4. **Proactive Refresh Buffer (5 minutes) vs Reactive Only**
   - Previous code only refreshed when the token was already expired
   - This causes a race: token expires mid-request, 401 returned, retry needed
   - 5-minute proactive buffer: refresh before expiry, never serve an expired token
   - Eliminates the need for 401-retry logic entirely
   - The `expires_within(Duration::seconds(300))` check on `TokenBundle` handles this

5. **Combining Related Issues into Single Implementation Plan**
   - Issues #92 (file locking), #93 (get-or-refresh), #94 (integration tests) were tightly coupled
   - The get-or-refresh function is where file locking belongs
   - Integration tests validate the new behavior
   - Disciplined research phase identified the coupling before implementation began
   - 6-step plan with clear dependencies prevented partial implementations

6. **Path Discrepancy Between Components**
   - `CodexTokenImporter` writes tokens to `~/.terraphim-llm-proxy/auth/`
   - `OpenAiCodexClient` was hardcoded to read from `/var/lib/terraphim-llm-proxy/auth`
   - These are different paths -- tokens imported would never be found by the client
   - **Solution**: Configurable `storage_path` with a sensible default
   - Default matches the importer path; systemd deployments override via config

### Debugging Approaches That Worked

1. **IDE Diagnostics for Cascading Changes**
   - After changing `LlmClient::new()` signature, IDE immediately showed all callers that needed updating
   - Faster than grep for finding every site that constructs the struct
   - Caught test files, examples, and integration tests

2. **Incremental Compilation Checks**
   - `cargo check` after each edit (takes ~4s on this project)
   - Catches issues immediately rather than batching
   - The diagnostic messages from rustc are precise about what's wrong

3. **Test-First for Lock Behavior**
   - Wrote `test_lock_file_created_during_refresh` before implementing
   - Verified lock file is cleaned up after refresh completes
   - `TempDir` provides isolated filesystem for each test

### Pitfalls to Avoid

1. **Don't use fd-lock for locks held across async boundaries**
   - The guard's lifetime is tied to the lock, making it impossible to hold across `.await`
   - Use file existence (O_EXCL) or a tokio Mutex instead
   - fd-lock is fine for synchronous-only locking (e.g., in `spawn_blocking`)

2. **Don't hardcode filesystem paths in library code**
   - `/var/lib/...` works on Linux with systemd but not macOS or containers
   - Always provide a configurable path with a sensible default
   - Use `dirs::home_dir()` for user-space defaults

3. **Don't add 401-retry when you can prevent 401 in the first place**
   - Proactive refresh (5-minute buffer) eliminates most 401s
   - 401 after a fresh token means the token was revoked, not expired
   - Retrying a revoked token wastes time and may trigger rate limits

4. **Don't forget to update all callers when changing function signatures**
   - `LlmClient::new()` -> `LlmClient::new(None)` needed updating in:
     - `src/client.rs` tests
     - `src/server.rs`
     - `src/test_streaming.rs`
     - `test_ollama.rs`
     - `tests/openrouter_streaming_test.rs`
   - Use `replace_all` edits or grep to find all callers

5. **Don't hold `unsafe Default` impls with `block_on`**
   - `OpenAiCodexClient` had a `Default` impl using `tokio::runtime::Handle::current().block_on()`
   - This panics if called outside a tokio runtime
   - Remove `Default` and require explicit async construction

### Best Practices Discovered

1. **RAII Lock Guard Pattern for File Locks**
   ```rust
   struct LockGuard { path: PathBuf }
   impl Drop for LockGuard {
       fn drop(&mut self) {
           let _ = std::fs::remove_file(&self.path);
       }
   }
   ```
   - Lock is always released, even on panic or early return
   - Matches Rust's ownership model naturally

2. **spawn_blocking for Synchronous Lock Operations**
   ```rust
   let guard = tokio::task::spawn_blocking(move || {
       acquire_lock_file(&lock_path)
   }).await??;
   // Now do async work while lock is held via file existence
   ```
   - Keeps async runtime unblocked during lock acquisition retry loop
   - The double `?` handles both JoinError and OAuthError

3. **Disciplined Development Phases**
   - Phase 1 (Research): Mapped current token flow, identified 3 related issues
   - Phase 2 (Design): 6-step plan with function signatures, test strategy, risk mitigation
   - Phase 3 (Implementation): One commit per step, tests at each step
   - Combined phases saved time while maintaining quality

4. **Config Field Addition Checklist (Updated)**
   ```markdown
   - [ ] Add field to struct definition with #[serde(default)]
   - [ ] Update Default impl
   - [ ] Update all test configs (grep for struct name)
   - [ ] Update server_integration_tests.rs
   - [ ] Run cargo check
   - [ ] Run cargo test
   ```

### Key Metrics

| Metric | Before | After |
|--------|--------|-------|
| Integration tests | 17 | 21 |
| TokenManager unit tests | 0 | 10 |
| Total tests | ~672 | 687 |
| OpenAiCodexClient lines | 443 | 371 |
| Inline refresh logic lines | ~70 | 0 (delegated to TokenManager) |
| Token storage paths | 2 (inconsistent) | 1 (configurable) |

---

## 2026-02-08: ChatGPT Backend-API + Knowledge Graph Routing Deployment

### Technical Discoveries

1. **reqwest_eventsource CANNOT be used with chatgpt.com**
   - `chatgpt.com` returns `x-codex-credits-balance:` header with empty value
   - reqwest_eventsource internally validates ALL response headers using `http` crate's strict parser
   - Empty header values cause `"Invalid header value: \"\""` error at HTTP response level
   - NO SSE events are ever received -- the error happens before streaming starts
   - `.http1_ignore_invalid_headers_in_responses(true)` does NOT help (EventSource wraps its own client)
   - **Solution**: Use raw reqwest `.send()` + `bytes_stream()` + `tokio_util::io::StreamReader` + manual SSE line parsing
   - This is a fundamental library incompatibility, not a configuration issue

2. **systemd ProtectHome=true blocks /home/ entirely**
   - The service file had `ProtectHome=true` for hardening
   - Setting `ROLEGRAPH_TAXONOMY_PATH=/home/alex/...` in the env file looks correct but fails silently
   - The error message "ROLEGRAPH_TAXONOMY_PATH set but path does not exist" was misleading -- the path existed on disk but was invisible to the service process
   - **Solution**: Deploy taxonomy files to `/etc/terraphim-llm-proxy/taxonomy/` instead
   - Always check systemd security directives when debugging "file not found" in services

3. **Openclaw uses OpenAI/JS SDK internally**
   - user_agent is `"OpenAI/JS 6.10.0"` -- it uses the official OpenAI JavaScript SDK
   - Requests are standard Chat Completions format with `stream: true`
   - System prompt is ~30K chars (workspace files, tools, skills injected)
   - 24 tool definitions with full JSON schemas are included in every request
   - Token count per request: ~32K tokens (mostly context, not user message)

4. **Openclaw primary model vs fallbacks determine routing**
   - `openai-codex/gpt-5.2-codex` (direct) -- bypasses proxy entirely, uses openclaw's built-in codex client
   - `terraphim/thinking` (via proxy) -- routes through proxy at http://127.0.0.1:3456/v1
   - The provider prefix determines the path, not the model name
   - Fallbacks `terraphim/fastest` and `terraphim/cheapest` also route through proxy

5. **Openclaw binary location on linux-small-box**
   - NOT in PATH by default
   - Installed via bun at `~/.bun/bin/openclaw`
   - `which openclaw` fails, `npx openclaw` fails (npx -> bunx -> not found)
   - Must use full path: `~/.bun/bin/openclaw`

### Debugging Approaches That Worked

1. **3-iteration debugging for SSE**
   - Iteration 1: Added `es.close()` -- wrong hypothesis (reconnection)
   - Iteration 2: Added `http1_only()` + lenient headers -- wrong hypothesis (header parsing in reqwest)
   - Iteration 3: Replaced EventSource entirely -- correct fix (library incompatibility)
   - **Lesson**: When a library wraps HTTP internally, you cannot fix issues with client configuration alone

2. **Debug logging revealed the real issue**
   - `RUST_LOG=debug` showed NO SSE events before the error
   - This proved the problem was at HTTP response level, not SSE streaming
   - Without debug logging, the error looked like a streaming issue

3. **Checking systemd service file for security directives**
   - `cat /etc/systemd/system/terraphim-llm-proxy.service` revealed `ProtectHome=true`
   - Explained why a valid filesystem path was "not found" by the service

4. **Proxy logs with grep patterns**
   ```bash
   sudo journalctl -u terraphim-llm-proxy --since '30 sec ago' | grep -E '(Phase|routing|scenario|RoleGraph)'
   ```
   - Quickly shows routing decisions without noise from metrics

### Pitfalls to Avoid

1. **Don't assume EventSource/SSE libraries handle all servers**
   - Some servers return non-standard headers
   - Empty header values are technically valid HTTP but break some parsers
   - Raw reqwest is more resilient for unofficial/internal APIs

2. **Don't deploy files to /home/ for systemd services with ProtectHome**
   - Use `/etc/` for config, `/var/lib/` for data
   - Check service file security directives before debugging path issues

3. **Don't test openclaw routing by using the direct provider**
   - `openai-codex/gpt-5.2-codex` bypasses the proxy entirely
   - Must use `terraphim/` prefix to route through proxy

4. **Don't forget to check proxy logs when testing client integration**
   - The client may report success but went through a different path
   - Always verify in proxy logs that the request actually arrived

### Best Practices Discovered

1. **Raw SSE Parsing Pattern for Non-Standard Servers**
   ```rust
   let byte_stream = response.bytes_stream().map(|r| {
       r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
   });
   let reader = tokio::io::BufReader::new(
       tokio_util::io::StreamReader::new(byte_stream)
   );
   let mut lines = reader.lines();
   while let Ok(Some(line)) = lines.next_line().await {
       if let Some(data) = line.strip_prefix("data: ") {
           // Process SSE event data
       }
   }
   ```

2. **Taxonomy Deployment Checklist for systemd**
   ```markdown
   - [ ] Copy taxonomy files to /etc/terraphim-llm-proxy/taxonomy/
   - [ ] Set ROLEGRAPH_TAXONOMY_PATH in env file
   - [ ] Verify route:: directives point to configured providers
   - [ ] Restart service: sudo systemctl restart terraphim-llm-proxy
   - [ ] Check logs for "Built automaton with N patterns"
   - [ ] Test with curl containing pattern keywords
   - [ ] Verify routing scenario in logs: scenario=Pattern("...")
   ```

3. **End-to-End Routing Verification Protocol**
   ```bash
   # 1. Send request with pattern keywords
   curl -s -N -X POST http://127.0.0.1:3456/v1/chat/completions \
     -H 'Authorization: Bearer <key>' \
     -d '{"model":"fastest","messages":[{"role":"user","content":"Think step by step: what is 2+2?"}],"stream":true}'

   # 2. Verify routing in logs
   sudo journalctl -u terraphim-llm-proxy --since '10 sec ago' | grep scenario
   # Expected: scenario=Pattern("think_routing") provider=openai-codex

   # 3. Send request without keywords
   curl ... -d '{"model":"fastest","messages":[{"role":"user","content":"What is 2+2?"}],"stream":true}'

   # 4. Verify default routing
   # Expected: scenario=Default provider=groq
   ```

### Key Metrics

| Metric | Value |
|--------|-------|
| Taxonomy files | 9 |
| Aho-Corasick patterns | 120 |
| Pattern match time | <1ms (decision_time_ms=0) |
| Codex streaming latency | ~28s for 32K input + 8 output tokens |
| Openclaw request size | 129KB (~32K tokens) |
| Debugging iterations for SSE fix | 3 |

---

## 2026-02-13: Cerebras Integration Reality Check

### Technical Discoveries

1. **Cerebras is OpenAI-compatible, but not capacity-compatible with our heavy OpenClaw profile**
   - Base URL is correct: `https://api.cerebras.ai/v1`
   - Endpoints are correct: `/v1/chat/completions`, `/v1/models`
   - Integration works for small requests, but long-context traffic still failed with `SSE 400` in real runs.

2. **Model context limits must drive routing decisions**
   - Cerebras docs for `llama3.1-8b` show much lower context than our problematic OpenClaw payloads.
   - Published limits (from docs):
     - Free tier context: `8k`
     - Paid tier context: `32k`
   - We observed failures on requests far above that range (OpenClaw payloads around `~130k` token hints).

3. **`max_completion_tokens` is the canonical parameter in Cerebras docs**
   - Our OpenAI-compatible payloads typically use `max_tokens`.
   - Compatibility may still work in many cases, but we should normalize per-provider fields explicitly to reduce ambiguous 400s.

4. **Some OpenAI fields are explicitly unsupported and produce 400s**
   - Cerebras compatibility docs call out unsupported text-completion fields such as:
     - `frequency_penalty`
     - `logit_bias`
     - `presence_penalty`
   - Provider-specific filtering/validation should happen before dispatch.

5. **Cerebras streaming logs include noisy terminal errors**
   - `SSE error from Cerebras error=Stream ended` appears on normal termination paths.
   - This should not be treated as a provider failure signal.

### Debugging Approaches That Worked

1. **Provider-isolated local config**
   - Running a Cerebras-only local config made failures deterministic and removed routing noise from other providers.

2. **Three-scenario integration probe**
   - Small stream: pass
   - Tool-enabled stream: pass
   - Long-context stream: reproducible `400`
   - This quickly proved partial functionality vs full functionality.

3. **Cross-check API docs against runtime token hints**
   - Comparing documented context limits with our runtime `token_count` immediately explained why long requests fail.

### Pitfalls to Avoid

1. **Do not route long-context requests to Cerebras 8B by default**
   - Even when endpoint/auth are valid, capacity mismatch can still return 400.

2. **Do not map `cheapest` to Cerebras without guardrails**
   - OpenClaw fallback chains can silently move heavy requests into Cerebras and fail in SSE startup.

3. **Do not assume EventSource status-only errors are enough**
   - Lack of response body details slows diagnosis of provider validation failures.

### Best Practices Discovered

1. **Add provider capability gates before routing**
   - Gate by token-count thresholds and request features (tools, JSON mode, reasoning flags).

2. **Separate "cheap" from "high-capacity" aliases**
   - `cheapest` should never be selected for high-context requests.

3. **Normalize request fields per provider**
   - Explicitly map `max_tokens` -> `max_completion_tokens` for Cerebras-bound requests.

4. **Classify terminal stream events correctly**
   - Treat "Stream ended" as a normal close path, not an error metric.

---

## 2026-02-20: linux-small-box Deployment Outage (3 root causes + openclaw fix)

### Technical Discoveries

1. **systemd `Restart=on-failure` does NOT restart on SIGTERM**
   - SIGTERM is a clean exit (code=killed, signal=TERM) and systemd logs "Succeeded"
   - `Restart=on-failure` only triggers on non-zero exit codes or signals like SIGABRT/SIGSEGV
   - After a clean stop, the service stays dead until manually started
   - **Fix**: Manually restart; consider `Restart=always` if the service should never be down

2. **ROLEGRAPH_TAXONOMY_PATH was missing from env file**
   - Taxonomy files existed at `/etc/terraphim-llm-proxy/taxonomy/routing_scenarios/` but the env var was not set
   - Result: "Taxonomy directory not found. RoleGraph pattern matching disabled."
   - Additionally, the path must point to the PARENT dir (`/etc/terraphim-llm-proxy/taxonomy/`)
   - `scan_taxonomy_files()` in `rolegraph_client.rs:193` looks for `routing_scenarios/` subdirectory within the configured path
   - Setting path to `.../taxonomy/routing_scenarios/` causes "Found 0 taxonomy files"
   - **Fix**: Added `ROLEGRAPH_TAXONOMY_PATH=/etc/terraphim-llm-proxy/taxonomy` to env file

3. **Cerebras retired `llama-3.3-70b` without notice**
   - Model returns 404: "Model llama-3.3-70b does not exist or you do not have access to it"
   - Available models changed to: llama3.1-8b, qwen-3-235b-a22b-instruct-2507, gpt-oss-120b, zai-glm-4.7
   - Config referenced the dead model in `default`, `fastest` mapping, and `models` list
   - **Fix**: Updated all references to `llama3.1-8b`

4. **Openclaw API key mismatch caused 401 on all requests**
   - Proxy env file: `PROXY_API_KEY=sk-proxy-local`
   - Openclaw config had stale key: `apiKey: "mDn6WgxFjAihjLcQjUpW0uv2JJPnNuW8VuMkVCdz4Eg"`
   - `env.TERRAPHIM_API_KEY = "$PROXY_API_KEY"` in JSON is a literal string, NOT shell expansion
   - **Fix**: Set both `models.providers.terraphim.apiKey` and `env.TERRAPHIM_API_KEY` to `sk-proxy-local`

5. **Openclaw default model must handle large payloads**
   - Openclaw injects ~15K tokens of system context (workspace files, tools, skills) per request
   - Cerebras llama3.1-8b has 8k context limit -- returns 400 on openclaw payloads
   - Kimi subscription expired -- returns 404
   - **Fix**: Default to `terraphim/zai,glm-5` which handles 128K+ context

### Debugging Approaches That Worked

1. **Check systemd status first** -- `Active: inactive (dead)` with `signal=TERM` immediately showed clean shutdown
2. **grep startup logs for "automaton"** -- "Found 0 taxonomy files" vs "Built automaton with 126 patterns" pinpoints taxonomy issues
3. **Cerebras /v1/models endpoint** -- `curl -H 'Authorization: Bearer $KEY' https://api.cerebras.ai/v1/models` reveals available models
4. **Read openclaw.json directly** -- the apiKey mismatch was immediately visible

### Pitfalls to Avoid

1. **Don't assume the service auto-restarts after SIGTERM** -- `Restart=on-failure` treats SIGTERM as success
2. **Don't set ROLEGRAPH_TAXONOMY_PATH to the routing_scenarios/ directory** -- set it to the parent
3. **Don't forget to add ROLEGRAPH_TAXONOMY_PATH to env file** -- taxonomy files on disk are useless without this
4. **Don't hardcode provider model names** -- providers retire models without warning
5. **Don't assume JSON `$VAR` syntax expands like shell** -- it's a literal string in openclaw config
6. **Don't route openclaw traffic to small-context models** -- 15K+ token payloads need 32K+ context

### Deployment Verification Protocol

```bash
# 1. Restart proxy
sudo systemctl restart terraphim-llm-proxy

# 2. Verify taxonomy loaded
sudo journalctl -u terraphim-llm-proxy --since '10 sec ago' | grep 'automaton'
# Expected: "Built automaton with 126 patterns"

# 3. Verify service healthy
curl -s http://127.0.0.1:3456/health | python3 -c 'import sys,json; print(json.load(sys.stdin)["status"])'
# Expected: "healthy"

# 4. Test proxy with correct API key
curl -s -X POST http://127.0.0.1:3456/v1/chat/completions \
  -H 'Authorization: Bearer sk-proxy-local' \
  -H 'Content-Type: application/json' \
  -d '{"model":"fastest","messages":[{"role":"user","content":"hello"}],"max_tokens":10}'
# Expected: 200 with response

# 5. Restart openclaw gateway
~/.bun/bin/openclaw gateway restart

# 6. Test openclaw end-to-end
~/.bun/bin/openclaw agent -m 'Reply OK' --session-id deploy-test --timeout 30
# Expected: response text
```
