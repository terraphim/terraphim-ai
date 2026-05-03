# Research Document: ADF Fleet Health Alert 20260503 -- DEGRADED

## 1. Problem Restatement and Scope

The ADF fleet monitoring system (Mneme) has flagged the fleet as **DEGRADED** based on three distinct operational patterns observed over a 24-hour window ending 2026-05-03. This document analyses each pattern, maps the affected system elements, identifies root causes, and proposes de-risking strategies before any design or implementation work begins.

### Pattern P1: Model-Health-Collapse
**Observation:** 417 of 417 agent spawns (100%) fell back from primary providers (openai, anthropic) to the kimi fallback. Zero successful primary routing occurred in 24h. Log evidence shows `skipped_unhealthy=["openai","anthropic"]` with confidence=0.45-0.5.

**Problem Statement:** The provider health probe system is incorrectly or prematurely marking openai and anthropic as unhealthy, causing the knowledge-graph router to bypass them entirely and route all traffic through kimi-for-coding/k2p5.

### Pattern P2: Review-Format-Drift  
**Observation:** 703 PR reviewer comment parsing failures with error "missing Inline Findings section". 96 agents required retries (retry-1: 111, retry-2: 13, retry-3: 5).

**Problem Statement:** The `structural-pr-review` skill template used by the pr-reviewer agent is not consistently emitting the `<h3>Inline Findings</h3>` HTML heading that `pr_review::parse_verdict` requires. When the heading is absent, the verdict is rejected and auto-merge cannot proceed.

### Pattern P3: Auto-Merge-Noise (RESOLVED)
**Observation:** 11 identical auto-merge failure issues created for PR #1151.
**Status:** PR #1151 was closed manually on 2026-05-03. The root cause (protected-branch status checks) is understood. However, the idempotency gap in the auto-merge handler remains a structural issue.

### IN Scope
- Provider health probe accuracy and circuit-breaker tuning
- PR review comment format contract enforcement
- Auto-merge issue deduplication

### OUT of Scope
- Provider API endpoint availability (this is external to ADF)
- Token budget management
- General fleet scaling or resource pressure

---

## 2. User and Business Outcomes

### Visible Impact
1. **Cost:** All traffic routed through kimi subscription instead of using available openai/anthropic quotas, potentially exhausting kimi rate limits
2. **Latency:** Fallback routing adds dispatch overhead; retry storms increase Gitea API load
3. **Reliability:** Auto-merge pipeline is blocked even when reviewer verdicts are substantively PASS, requiring manual intervention
4. **Noise:** Duplicate issues dilute the issue tracker and reduce triage effectiveness

### Success Criteria (for Phase 2)
- >95% of spawns use primary provider when API is healthy
- 0 "missing Inline Findings" parse errors from pr-reviewer
- 0 duplicate auto-merge issues for the same PR within 24h

---

## 3. System Elements and Dependencies

| Element | Location | Role | Dependencies |
|---------|----------|------|--------------|
| `ProviderHealthMap` | `crates/terraphim_orchestrator/src/provider_probe.rs:36` | Tracks per-provider circuit breakers and probe results | `tokio::process::Command`, `terraphim_spawner::health::CircuitBreaker`, KG router action templates |
| `probe_single` | `provider_probe.rs:385` | Executes provider probe via bash -c with test prompt "echo hello" | `config::is_allowed_provider`, KG action templates, CLI tools on PATH |
| `is_allowed_provider` | `config.rs` | C1 subscription allow-list gate | Static consts: `ALLOWED_PROVIDER_PREFIXES`, `BANNED_PROVIDER_PREFIXES`, `CLAUDE_CLI_BARE_MODELS` |
| `CircuitBreaker` | `terraphim_spawner::health` | Tracks failure counts and state (Closed/HalfOpen/Open) | Config: `failure_threshold=2`, `cooldown=60s`, `success_threshold=1` |
| `AgentOrchestrator::spawn_agent` | `lib.rs:~1700` | Dispatches to provider or fallback based on `unhealthy_providers()` | `ProviderHealthMap`, KG router, `terraphim_spawner` |
| `pr_review::parse_verdict` | `pr_review.rs:114` | Parses review comment body into structured verdict | String matching for `<h3>Inline Findings</h3>` or `### Inline Findings` |
| `pr_poller::evaluate_pr_verdict` | `pr_poller.rs` | Fetches PR comments and calls parse_verdict + evaluate | `PrTracker` trait (Gitea API), `parse_verdict`, `AutoMergeCriteria` |
| `AutoMergeExecutor::open_failure_issue` | `pr_poller.rs` | Creates `[ADF] Auto-merge failed` issue on merge error | Gitea API, no deduplication check |
| `structural-pr-review` skill | `~/.config/opencode/skill/disciplined-verification/` | Generates review comments with required sections | Skill template markdown/HTML output |

### Shared State / Cross-Cutting Concerns
- **Provider health cache:** `ProviderHealthMap` is shared across all spawns within one orchestrator instance. A single probe failure affects all subsequent spawns until cooldown expires.
- **Circuit breaker state:** Persisted only in-memory; orchestrator restart resets all breakers to Closed.
- **Review format contract:** Hard-coded in `pr_review.rs` but driven by skill template output which is not version-locked.

---

## 4. Constraints and Their Implications

### Business / Operational
- **C1 subscription model:** Only subscription-based providers are allowed (kimi, openrouter, anthropic via claude CLI). Pay-per-use providers (opencode/*, github-copilot/*) are banned. This means `openai` (ChatGPT OAuth) may be banned unless it uses a subscription endpoint.
- **Implication:** If `openai` models are configured with a banned prefix or missing from `ALLOWED_PROVIDER_PREFIXES`, the probe returns Error immediately, triggering the circuit breaker.

### Performance
- **Probe timeout:** 60 seconds. A slow provider API can cause timeout, which is treated as Error.
- **Probe TTL:** Default 5 minutes (300s). Stale results trigger re-probe.
- **Implication:** During high load or API latency spikes, probes timeout and mark providers unhealthy even when they are merely slow.

### Reliability
- **Circuit breaker config:** `failure_threshold=2`, `cooldown=60s`. Two consecutive failures open the breaker for 60 seconds.
- **Implication:** Transient errors (network blip, API rate limit) can open the breaker quickly, and the 60s cooldown may not be enough for the provider to recover.

### Security
- **Probe uses bash -c:** The action template from KG routing is executed directly via bash. If templates are malformed or contain injections, probes fail.
- **Implication:** Probe failures may indicate template errors, not provider unavailability.

### UX / Triage
- **Issue tracker noise:** Gitea issue count is already high (482 open). Duplicate auto-merge issues make triage harder.
- **Implication:** Any fix must include idempotency to prevent recurrence.

---

## 5. Risks, Unknowns, and Assumptions

### UNKNOWN: Why are openai/anthropic probes actually failing?
- The probe executes `bash -c "<action_template>"` with test prompt "echo hello". We do not have the actual probe result logs showing stderr.
- **De-risk:** Check `/opt/ai-dark-factory/probe_results/` or orchestrator logs for `ProbeResult.error` fields.

### UNKNOWN: What action templates are configured for openai/anthropic in KG routing?
- If the action template references a CLI tool (e.g., `opencode`, `claude`) that is not on PATH, the probe fails with "spawn failed" or "command not found".
- **De-risk:** Dump KG routing rules from the running orchestrator config.

### ASSUMPTION: openai/anthropic APIs are actually healthy
- The alert assumes the providers are healthy but ADF is misrouting. If the APIs are genuinely down, the fallback behaviour is correct.
- **De-risk:** Manually test openai and anthropic endpoints from the bigbox host.

### ASSUMPTION: The `structural-pr-review` skill template has not changed
- If the skill was updated and no longer emits `<h3>Inline Findings</h3>`, the parser will fail consistently.
- **De-risk:** Compare current skill template against the version expected by `pr_review.rs`.

### RISK: Tightening the review parser could break legitimate reviews
- If we make the parser more tolerant (e.g., accept markdown headings), we might accept incomplete reviews.
- **Mitigation:** Add parser variants behind feature flags, or validate against a corpus of known-good reviews.

### RISK: Auto-merge deduplication requires Gitea API search
- Searching for existing issues by title/body is an extra API call that could fail or be rate-limited.
- **Mitigation:** Cache recently created failure issues in memory with a TTL.

---

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Probe executes arbitrary CLI templates:** The probe is only as reliable as the CLI tool and template. A tool failure is indistinguishable from a provider failure.
2. **Circuit breaker has no memory across restarts:** Every orchestrator restart resets health state, causing a burst of probes and potential flapping.
3. **Review parser is rigid:** Single string match for HTML heading; no fallback for markdown or alternative section names.
4. **Auto-merge handler has no deduplication:** Every failure creates a new issue, even for the same PR.

### Simplification Opportunities
1. **S1: Separate tool-health from provider-health probes**
   - Probe the CLI tool first (`which claude`, `claude --version`), then probe the provider API. This distinguishes "tool missing" from "provider down".
   
2. **S2: Add markdown heading fallback to review parser**
   - The parser already accepts `### Inline Findings` as a fallback (line 120). If this is not working, the issue may be whitespace or HTML entity encoding. Add normalised whitespace stripping before matching.
   
3. **S3: In-memory deduplication cache for auto-merge failures**
   - Track `(pr_number, head_sha)` pairs that already have open failure issues. Skip creation if the pair is in the cache.

---

## 7. Questions for Human Reviewer

1. **Probe logs:** Can you provide the latest probe results JSON from `/opt/ai-dark-factory/probe_results/latest.json` or equivalent? We need the actual `error` fields for openai/anthropic probes.

2. **Provider subscription status:** Is `openai` (as a bare provider name or prefix) actually in the `ALLOWED_PROVIDER_PREFIXES` list? The C1 gate may be rejecting it.

3. **KG routing rules:** What are the current `action::` templates for openai and anthropic in the KG router config? Are they using CLI tools that are installed on bigbox?

4. **Review skill version:** Has the `structural-pr-review` skill been modified recently? Can you share the current template output for a sample review?

5. **Circuit breaker tuning:** Is `failure_threshold=2` and `cooldown=60s` intentionally aggressive? Would `failure_threshold=3` and `cooldown=300s` be more appropriate for API latency variability?

6. **Auto-merge retry policy:** Should the auto-merge handler retry the merge API call before creating a failure issue? Currently it seems to fail immediately on the first 405 error.

7. **Probe prompt:** The probe uses test prompt "echo hello". Should it use a more realistic prompt that exercises the actual provider API (e.g., a 1-token completion)?

8. **Provider health dashboard:** Is there an existing dashboard or log aggregation (Quickwit) query that shows provider health over time? This would help distinguish transient vs. persistent failures.

9. **Skill template contract:** Should the review parser be versioned and locked to a specific skill template version to prevent future drift?

10. **Issue deduplication scope:** Should deduplication apply only to auto-merge failures, or to all `[ADF]` automated issues?

---

## Quality Checklist

- [x] Problem clearly distinguished from solutions
- [x] All affected system elements identified with file paths
- [x] Constraints have clear implications
- [x] Every assumption marked as such
- [x] Risks have de-risking suggestions
- [x] Questions are specific and actionable
