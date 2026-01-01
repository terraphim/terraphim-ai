# Specification Interview: LLM Router Integration

**Date**: 2026-01-01
**Design Document**: [.docs/design-llmrouter-integration.md](.docs/design-llmrouter-integration.md)
**Interviewer**: AI Specification Specialist
**Status**: In Progress

---

## Interview Structure

I'll present questions across 10 critical dimensions. Please provide answers that reflect your requirements and expectations.

---

### Dimension 1: Concurrency & Race Conditions

**Context**: The router will handle multiple simultaneous requests. Need to understand concurrency guarantees.

**Q1.1**: When two users make requests that match the same pattern (e.g., "low cost"), should they both route to the same provider/model, or should there be load balancing across multiple options?

**Options**:
- A) Both route to same provider/model (deterministic)
- B) Load balance across equally good options (non-deterministic)
- C) Round-robin within the matched provider set

**Q1.2**: If a user cancels a request while the router is still making a decision, should the router:
**Options**:
- A) Continue routing and cache the result for future requests
- B) Abort immediately and free resources
- C) Continue routing but don't cache the result

**Q1.3**: How should the router handle rapid-fire requests from the same session? Should routing decisions be cached per-session, or re-evaluated each time?

**Options**:
- A) Cache routing decision for session (TTL based)
- B) Always re-evaluate (current behavior, more expensive)
- C) Hybrid: cache simple patterns, re-evaluate complex scenarios

---

### Dimension 2: Failure Modes & Recovery

**Context**: What happens when routing phases fail or LLM providers are unavailable?

**Q2.1**: If Phase 1 (pattern matching) produces a routing decision, but the selected provider's API returns 429 rate limit, should the router:
**Options**:
- A) Fail immediately with error to user
- B) Fallback to Phase 2 (session-aware) for same query
- C) Fallback directly to default provider from config
- D) Retry with exponential backoff before trying next phase

**Q2.2**: If the proxy routing logic panics (crashes) in the middle of processing a request, should the adapter layer:
**Options**:
- A) Return generic error "routing service unavailable"
- B) Fallback to static LLM client (existing behavior)
- C) Return cached routing decision from previous similar request
- D) Crash the entire application (panic propagation)

**Q2.3**: When Phase 3 (cost optimization) or Phase 4 (performance optimization) is enabled but the pricing database is empty or metrics haven't been collected yet, should the router:
**Options**:
- A) Skip those phases and go to Phase 5 (scenario fallback)
- B) Use default values (average pricing, 50th percentile performance)
- C) Return error requesting configuration of those phases
- D) Disable those phases automatically and log warning

---

### Dimension 3: Edge Cases & Boundaries

**Context**: Boundary conditions and unusual inputs to the routing system.

**Q3.1**: What should happen if a user specifies an explicit provider model format (e.g., `openrouter:anthropic/claude-sonnet-4.5`) but that provider is not configured in the proxy?

**Options**:
- A) Return 400 Bad Request with clear error message
- B) Skip to next routing phase (ignore explicit format)
- C) Use the provider anyway with default model from config
- D) Fallback to default provider from router config

**Q3.2**: When pattern matching returns multiple equal-scoring matches (e.g., "background" and "low cost" both score 0.9), how should the router resolve the tie?

**Options**:
- A) Select the one that appears earlier in the query text
- B) Select the one with higher priority (if configured)
- C) Select the one that's cheaper (cost-first tiebreaker)
- D) Select randomly (non-deterministic but fair)

**Q3.3**: If a user's request is extremely long (e.g., 200K tokens), and all configured providers have context limits below that, should the router:
**Options**:
- A) Select the provider with largest context window even if suboptimal
- B) Return error asking user to reduce request size
- C) Truncate or chunk the request automatically
- D) Select a provider and let the API handle the limit (return 400)

---

### Dimension 4: User Mental Models

**Context**: How users understand and expect the routing feature to work.

**Q4.1**: When a user sees a different model selected than they expected (e.g., they asked for "high quality" but router selected "low cost" provider), should the system:
**Options**:
- A) Provide no explanation (assume users trust the routing)
- B) Log the routing decision only (developer/debug view)
- C) Return routing reason in API response metadata (e.g., `{"model": "deepseek-chat", "routing_reason": "Pattern matched: low_cost_routing (priority: 50)"}`)
- D) Show routing decision in UI with explanation (e.g., "Selected DeepSeek for cost optimization")

**Q4.2**: How should the system handle user feedback on routing quality? (e.g., "this response was poor, use a different model next time")

**Options**:
- A) Ignore feedback (routing is automatic, not learnable)
- B) Temporarily boost provider preference for that user's session
- C) Adjust routing weights/parameters globally based on feedback
- D) Create manual override that bypasses routing for N requests

**Q4.3**: What terminology should be used in documentation and UI for the routing feature? (This affects user mental model)

**Options**:
- A) "Intelligent Routing" - sounds magical
- B) "Automatic Model Selection" - clear and accurate
- C) "Smart Cost Optimization" - emphasizes benefit
- D) "Dynamic Routing" - technical but clear

---

### Dimension 5: Scale & Performance

**Context**: Performance characteristics under load and with many providers.

**Q5.1**: The proxy currently measures 0.21ms routing overhead. What is the maximum acceptable overhead for the integrated library mode (in-process, no network)?

**Options**:
- A) <1ms (must be faster than proxy to be acceptable)
- B) <2ms (slightly slower than proxy, still negligible)
- C) <5ms (acceptable but not ideal)
- D) <10ms (maximum acceptable, routing is optimization overhead)

**Q5.2**: If the routing pattern database grows from 200 patterns to 2000+ patterns (e.g., enterprise taxonomy), should the pattern matching:
**Options**:
- A) Keep using Aho-Corasick (linear time, memory grows with patterns)
- B) Switch to probabilistic model (constant time, slightly less accurate)
- C) Implement pattern hierarchy/categorization to reduce active set
- D) Add caching layer for frequently matched patterns

**Q5.3**: Should the router implement request queuing/batching to optimize for cost? (e.g., combine multiple small requests into one larger request to a more expensive but capable model)

**Options**:
- A) Yes, automatically batch requests within time window (e.g., 100ms)
- B) Yes, but only if user explicitly enables "batching mode"
- C) No, always process immediately (current behavior)
- D) No, but provide API for users to batch their own requests

---

### Dimension 6: Security & Privacy

**Context**: Security implications of intelligent routing and proxy architecture.

**Q6.1**: Should routing decisions be logged for security auditing? (e.g., to detect if routing is being exploited to route to compromised providers)

**Options**:
- A) No logging of routing decisions (privacy by default)
- B) Log only routing phase used, not selected model (minimal)
- C) Log full routing decision including provider, model, and reasoning (security)
- D) Log only anomalous routing decisions (e.g., unusual patterns, rate limits)

**Q6.2**: If a user's role has `llm_router_enabled: true` but they're trying to use an explicit provider model syntax that routes to a provider they don't have API access to, should the router:
**Options**:
- A) Route anyway and let the API fail with 401/403
- B) Check role's API keys for that provider, if missing, skip that routing decision
- C) Check a whitelist of allowed providers per role in router config
- D) Allow routing but return error/warning before making the API call

**Q6.3**: When using external service mode (HTTP proxy on port 3456), should the main application send the user's original API key to the proxy, or should the proxy use its own configured keys?

**Options**:
- A) Proxy uses its own keys only (user keys ignored)
- B) Proxy uses user's original API key (forwards auth)
- C) Proxy uses proxy's keys but falls back to user key if fails
- D) Configurable per proxy deployment (security choice)

---

### Dimension 7: Integration Effects

**Context**: How intelligent routing affects existing Terraphim AI features.

**Q7.1**: The existing system has session management. When routing is enabled, should session state track routing decisions made, or should that be a separate concern?

**Options**:
- A) Existing sessions store last routing decision (minimal change)
- B) New "routing session" concept separate from existing sessions (clean separation)
- C) No session tracking of routing (stateless routing)
- D) Optional - enabled via feature flag or config

**Q7.2**: When intelligent routing is enabled, should existing features like `llm_auto_summarize` and chat history still use the same routed model, or can they bypass routing?

**Options**:
- A) Always go through routing (consistent)
- B) Features can explicitly request specific model (bypass routing)
- C) Routing is advisory only; features can override with provider hint
- D) Configurable per feature (some go through routing, some don't)

**Q7.3**: If a user has multiple roles configured, each with different routing strategies, should routing decisions be:
**Options**:
- A) Cached per-role (different routing contexts)
- B) Global across all roles (shared routing state)
- C) Request-level only (no caching of role-specific patterns)
- D) Session-scoped (each session maintains its own routing cache)

---

### Dimension 8: Migration & Compatibility

**Context**: Transitioning existing users and data to intelligent routing.

**Q8.1**: For existing roles that have `llm_model: "anthropic/claude-sonnet-4.5"` configured but no router config, when router is enabled globally, should those roles:
**Options**:
- A) Continue using static model (router disabled for that role)
- B) Auto-upgrade to intelligent routing with that model as default
- C) Require explicit role configuration update to enable routing
- D) Show warning in UI: "This role could benefit from intelligent routing"

**Q8.2**: If a user is using library mode (in-process routing) and the proxy crashes or needs restart, should existing in-flight requests:
**Options**:
- A) Fail immediately (user experience: error)
- B) Fallback to static LLM client (graceful degradation)
- C) Queue requests and retry when proxy recovers
- D) Use cached routing decisions if available (best effort)

**Q8.3**: What's the rollback strategy if intelligent routing causes problems in production? (e.g., routing to wrong providers, increased costs, poor quality)

**Options**:
- A) Feature flag `llm_router_enabled` can be set to false at runtime
- B) Emergency config setting to disable routing without code deploy
- C) Per-user opt-out (administrators can disable for specific users)
- D) Automatic rollback triggers (e.g., if error rate > 10%, auto-disable)

---

### Dimension 9: Accessibility & Internationalization

**Context**: Making routing features accessible and understandable.

**Q9.1**: Should routing decisions be exposed in accessibility APIs (screen readers) for users who want to understand which model was selected?

**Options**:
- A) No exposure (internal optimization detail)
- B) Expose in API response metadata (developers can read)
- C) Expose in UI with ARIA labels (end-user visible)
- D) Expose in both API and UI with clear language

**Q9.2**: The routing patterns are currently in English (e.g., "low cost", "background"). Should the pattern matching:
**Options**:
- A) Support i18n of pattern files (load different language taxonomies)
- B) Keep patterns in English but translate routing reasons in UI
- C) Add language hint to patterns file (metadata for UI translation)
- D) Language-agnostic patterns (use code/IDs instead of natural language)

---

### Dimension 10: Operational Concerns

**Context**: Monitoring, debugging, and maintenance of intelligent routing.

**Q10.1**: What metrics should be tracked to detect routing issues in production?

**Options**:
- A) Basic only: routing phase used, time spent routing
- B) Standard plus: success rate per provider/model
- C) Comprehensive: A) + cost savings, performance metrics, fallback rates
- D) Full observability: C) + detailed request traces, error categorization

**Q10.2**: If routing starts causing issues (e.g., 90% of requests going to expensive provider unexpectedly), how should operators be alerted?

**Options**:
- A) No automatic alerts (manual monitoring only)
- B) Alert if any phase's usage exceeds X% of baseline
- C) Alert if cost per request increases by Y% over Z hours
- D) Alert if fallback rate (Phase 5 used) exceeds threshold (e.g., >20%)

**Q10.3**: For debugging routing decisions, should the system provide:
**Options**:
- A) Structured logs with all phase results and scores
- B) Debug API endpoint to test routing without making LLM calls
- C) Interactive routing explorer tool (CLI or web UI)
- D) All of the above (comprehensive debugging)

---

## Next Steps

Please review the questions above and provide answers. You can:
1. Answer each question individually (Q1.1, Q1.2, etc.)
2. Answer groups by dimension (Dimension 1: all questions)
3. Provide overall preferences (e.g., "always choose option A for scalability")
4. Skip dimensions that aren't relevant (though I recommend reviewing all)

**When ready**, I will compile your answers into the design document as "Specification Interview Findings" and we can proceed to implementation.

---

**Status**: Waiting for responses...
