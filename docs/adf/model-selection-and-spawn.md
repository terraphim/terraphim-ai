# ADF model selection and agent spawn — reference

How the AI Dark Factory orchestrator picks a model for each dispatch and
starts the agent subprocess. This walkthrough covers the whole pipeline from
the dispatch queue through to the child process, cross-referencing the code
and the log lines you will see in the journal.

## Stage map

```
tick ──► DispatchContext ──► RoutingDecisionEngine.decide_route
                                      │
             ┌────────────────────────┼────────────────────────┐
             │                        │                        │
         KG match               Keyword match            Static config
             │                        │                        │
             └─────────── merge ──────┴────────── append ──────┘
                                      │
                        C1/C3 allow-list filter
                                      │
                        ProviderBudgetTracker filter
                                      │
                              score + telemetry
                                      │
                                 pick best
                                      │
                                      ▼
                            spawn_with_fallback
                                      │
              try primary ──► model_args ──► tokio::process::Command
                  │
               on fail ──► try fallback (same path)
                                      │
                                      ▼
                                 AgentHandle
                                      │
                                subprocess ──► exit classifier ──► OutputPoster
```

## Stage 1: DispatchContext assembly

The reconcile loop ticks every `tick_interval_secs` (default `30`, set in
`orchestrator.toml`). Each tick drains four sources:

1. Cron-scheduled wakeups from `[[agents]].schedule` cron fields
2. Mention-polled comments (repo-wide `issues/comments` API, cursor-tracked)
3. Webhook-fed `issue_comment` and `pull_request` events
4. The in-memory `DispatchTask` queue (ReviewPr, AutoMerge, PostMergeTestGate)

For each item about to dispatch, the orchestrator builds a `DispatchContext`:

```rust
DispatchContext {
    agent_name:    "security-sentinel",
    cli_tool:      "/home/alex/.local/bin/claude",
    task_keywords: ["review", "verify", ...],   // tokenised from task string
    static_model:  Some("haiku"),               // from agent TOML, if set
    project:       "terraphim",
    fallback_provider, fallback_model, persona, ...
}
```

Plus a `BudgetVerdict` from the agent's per-month USD cost budget
(`ProviderBudgetTracker`). `BudgetPressure` comes out as `NoPressure`,
`NearExhaustion`, or `Exhausted`.

## Stage 2: RoutingDecisionEngine picks the model

`crates/terraphim_orchestrator/src/control_plane/routing.rs` —
`decide_route(&self, ctx: &DispatchContext, budget_verdict: &BudgetVerdict)`.

### 2a. Short-circuit for non-routable CLIs

```rust
if !Self::supports_model_flag(cli_name) {
    return RoutingDecision { source: RouteSource::CliDefault, ... };
}
```

Unknown CLI binaries (anything other than claude, opencode, codex) fall
through here.

### 2b. Collect candidates from three sources

`collect_all_candidates` gathers independent candidates:

| Source | Data | Example route |
|---|---|---|
| Knowledge Graph | Aho-Corasick match of `task_keywords` against `docs/taxonomy/routing_scenarios/adf/{planning,implementation,review}_tier.md` — `trigger::`, `synonyms::`, `route::`, `action::` directives | `concept=review_tier` → `route:: anthropic, haiku` |
| Keyword routing | `KeywordRouter` built from `register_providers()` in `bin/adf.rs`; each `Provider` declares a `keywords` list | "implement/code/generate" → `kimi-for-coding/k2p6` |
| Static config | `model = "..."` on the agent TOML | `security-sentinel` sets no static → empty |

### 2c. Merge candidates

If KG and Keyword both pick the **same** model string, merge into a single
`CombinedKgKeyword` candidate whose confidence is the average of the two.
Otherwise the engine keeps them as distinct candidates for later scoring.
Static config always appends.

### 2d. C1/C3 allow-list filter (defence in depth)

```rust
all_candidates.retain(|cand| {
    let allowed = crate::config::is_allowed_provider(&cand.model);
    if !allowed {
        warn!(..., "routing: dropped banned candidate (C1/C3 gate)");
    }
    allowed
});
```

Allowed prefixes live in `ALLOWED_PROVIDER_PREFIXES` (`config.rs:794`):

- `claude-code`
- `opencode-go`
- `kimi-for-coding`
- `minimax-coding-plan`
- `zai-coding-plan`
- `openai` (subscription-gated via the OpenAI Plus/Pro/Team plan)

Bare names `sonnet`, `opus`, `haiku` are recognised as claude CLI targets and
always allowed. Load-time validation (`validate()`) already enforces the same
rule on the TOML; this filter exists so a malformed KG file or a telemetry
artefact cannot surface a banned target at runtime.

### 2e. ProviderBudgetTracker filter

Each candidate's provider key is checked against the two-window tumbling
tracker (`hour` bucket id `YYYYMMDDHH`, `day` bucket id `YYYYMMDD`):

| Verdict | Action |
|---|---|
| `Exhausted` | drop + `routing: dropped provider-budget-exhausted candidate` |
| `NearExhaustion` | keep; later score ×0.6 |
| `Ok` | keep |

If every candidate got dropped by either filter, the engine returns
`CliDefault` with a descriptive rationale and flags `primary_available=false`
so downstream logging knows this is a degraded fallback.

### 2f. Score candidates

```rust
fn score_candidate(cand: &RouteCandidate, pressure: BudgetPressure) -> f64 {
    let source_weight = match cand.source {
        KnowledgeGraph       => 1.0,
        CombinedKgKeyword    => 1.0,
        KeywordRouting       => 0.8,
        StaticConfig         => 0.6,
        CliDefault           => 0.3,
    };
    let base = source_weight * cand.confidence;
    base * (1.0 - pressure.cost_penalty(&cand.provider.cost_level))
}
```

- `confidence` comes from the match strength (KG rule priority, keyword
  match count, or 1.0 for explicit static config).
- `cost_penalty` is `0.0` under `NoPressure`, rising under `NearExhaustion`
  and `Exhausted` so expensive providers lose out when the monthly budget
  is nearly spent.

Then the engine multiplies by `0.6` for any candidate whose provider is
flagged `NearExhaustion` in the per-provider budget tracker, so a provider
at 80 %+ of its quota is still eligible but loses to a healthy alternative.

### 2g. Telemetry adjustment

If `TelemetryStore` is wired (adf-fleet#15 path), each candidate's historical
`(success_rate, latency_p95)` is fetched and folded into the pressured score.
Low success rate or high latency further deprioritises.

### 2h. Pick the winner

Sort by pressured score descending and take `[0]`. The engine emits:

```
INFO model selected via KG tier routing agent=security-sentinel
     concept=review_tier provider=anthropic model=haiku confidence=0.6
```

When the primary was dropped by unhealthy-providers the orchestrator emits a
separate log from `kg_router`:

```
INFO KG routed to fallback (primary unhealthy)
     agent=merge-coordinator concept=review_tier
     provider=minimax model=minimax-coding-plan/MiniMax-M2.5
     skipped_unhealthy=["anthropic", "openai"]
```

## Stage 3: spawn_with_fallback

`crates/terraphim_spawner/src/lib.rs` —
`spawn_with_fallback(&self, request: &SpawnRequest, ctx: SpawnContext)`.

`SpawnRequest` bundles:

- `primary_provider`, `primary_model` (selected in Stage 2)
- `fallback_provider`, `fallback_model` (from agent TOML)
- `task` (composed prompt: persona + skill chain + task envelope)
- `resource_limits` (RLIMIT_AS, RLIMIT_CPU, ulimit walls)
- `use_stdin` (whether to pipe the prompt in via stdin — true for
  long prompts to avoid ARG_MAX)

```rust
match self.spawn_with_options(&req.primary_provider, &req.task,
                              req.primary_model.as_deref(), ...).await {
    Ok(handle) => Ok(handle),
    Err(primary_err) => {
        warn!("Primary spawn failed, attempting fallback");
        if let Some(fallback) = &req.fallback_provider {
            match self.spawn_with_options(fallback, &req.task,
                                          req.fallback_model.as_deref(), ...).await {
                Ok(handle) => Ok(handle),
                Err(_)     => Err(primary_err),  // both failed — surface original
            }
        } else { Err(primary_err) }
    }
}
```

Fallback selection is **per-agent, not per-routing-decision**. The routing
engine selects the best primary; the TOML-declared fallback is used only if
the primary provider physically fails to start (binary missing, arg error,
permission denied, etc.). For per-candidate fallback across the KG route
list, see `kg_router::route_agent` which iterates the `route::` entries in
order and emits `KG routed to fallback (primary unhealthy)`.

## Stage 4: model_args picks the right CLI flag

`crates/terraphim_spawner/src/config.rs:139`:

```rust
fn model_args(cli_command: &str, model: &str) -> Vec<String> {
    match Self::cli_name(cli_command) {
        "codex"                => vec!["-m", model],
        "claude"|"claude-code" => {
            let normalised = Self::normalise_claude_model(model);
            // `sonnet`/`opus`/`haiku` pass through; `opus-4-6` -> `claude-opus-4-6`
            vec!["--model", &normalised]
        }
        "opencode"             => vec!["-m", model],
        _                      => vec![],   // unknown CLI — no model flag
    }
}
```

The normaliser for claude CLI prepends `claude-` when a version suffix is
present. Bare aliases stay bare so they track the latest Anthropic release.

## Stage 5: subprocess launch

Tokio `Command` builder:

```rust
let mut cmd = tokio::process::Command::new(cli);
cmd.args(model_args)
   .args(other_flags)                 // --allowedTools, --print, ...
   .envs(ctx.env_overrides)           // ADF_PR_NUMBER, GITEA_TOKEN, ...
   .stdin(if use_stdin { Stdio::piped() } else { Stdio::null() })
   .stdout(Stdio::piped())
   .stderr(Stdio::piped())
   .spawn()?;
```

The `ctx` carries per-dispatch env overrides: `GITEA_WORKING_DIR`,
`GITEA_OWNER`, `GITEA_REPO`, `GITEA_TOKEN`, and for ReviewPr dispatches the
`ADF_PR_NUMBER` / `ADF_PR_HEAD_SHA` / `ADF_PR_PROJECT` / `ADF_PR_AUTHOR` /
`ADF_PR_DIFF_LOC` set. The composed prompt (persona + skill chain + task
envelope + mention context) is written to stdin by the spawner.

Trace line:

```
INFO spawning agent agent=security-sentinel layer=Safety
     cli=/home/alex/.local/bin/claude model=Some("haiku")
```

## Stage 6: exit classification feeds back into the next decision

When the child exits, `ExitClassifier` folds together the exit code and the
last 200 lines of stderr against `ProviderErrorSignatures`:

| Verdict | Signal | Effect |
|---|---|---|
| `success` | exit 0 | nothing |
| `throttle` | stderr regex matched from the provider's `throttle` list | trip `CircuitBreaker` for that provider; call `ProviderBudgetTracker::force_exhaust()` so routing drops this provider until the next window |
| `flake` | stderr regex matched from the `flake` list | no breaker trip; next dispatch retries |
| `resource_exhaustion` | `matched_patterns=["oom"]` / `"killed"` / `"signal: 9"` | count as soft failure; raises concern flag for fleet-meta |
| `unknown` | no pattern matched | count as soft failure; escalate pattern to fleet-meta for a human to classify |

Trace line:

```
INFO agent exit classified agent=security-sentinel exit_code=Some(0)
     exit_class=success confidence=1.0 wall_time_secs=90.0
```

## Stage 7: OutputPoster writes back to Gitea

`crates/terraphim_orchestrator/src/output_poster.rs` —
`post_agent_output_for_project(project, agent_name, issue_number, output_lines, exit_code)`.

Resolves the per-project `GiteaTracker`, looks up the per-agent Gitea token
(`agent_tokens.json`) so the comment lands under the agent's own login, and
POSTs to `/api/v1/repos/{owner}/{repo}/issues/{issue_number}/comments`.

### Known bug recently fixed (adf-fleet#44)

Before commit `7cf60d2c` (PR #738), `RepoComment::issue_number` was extracted
only from `issue_url`. For comments on pull requests Gitea returns
`pull_request_url` instead, so every PR comment arrived with
`issue_number = 0` and OutputPoster tried to post to `/issues/0/comments` —
500 from Gitea. The fix reads `pull_request_url` as a fallback. PRs share
the issue numeric namespace so the same trailing-segment extraction works
for both URLs.

## Example: trace from the journal

Security-sentinel got a model at 17:33:08 CEST on 2026-04-21:

```
17:33:08.232  provider probe complete providers_probed=11 healthy=11
17:33:08.244  model selected via KG tier routing agent=security-sentinel
              concept=review_tier provider=anthropic model=haiku confidence=0.6
17:33:08.244  spawning agent agent=security-sentinel layer=Safety
              cli=/home/alex/.local/bin/claude model=Some("haiku")
17:34:38.258  agent exit classified agent=security-sentinel exit_code=Some(0)
              exit_class=success wall_time_secs=90.0
```

Reading the trace:

1. Probe round completed, all 11 providers healthy, including the three
   `openai/*` variants (restored by PR #737 and `kimi-for-coding/k2p6`).
2. Task keywords matched the `review_tier` KG concept (`trigger:: verify,
   validate, ...` among others). The `route::` list for review_tier has
   `anthropic, haiku` as the primary entry.
3. C1 filter: `haiku` is bare, maps to claude CLI, allowed.
4. Budget filter: no near-exhaustion this hour.
5. Score: `source_weight(KnowledgeGraph)=1.0 × confidence(0.6) = 0.6`. No
   telemetry penalty, no pressure penalty.
6. Spawn `claude --model haiku --allowedTools ... --print` with the composed
   prompt on stdin.
7. 90 s later exit 0 → OutputPoster posts the verdict to the
   security-sentinel standing log on Gitea.

## Knobs

| Config | File | Effect |
|---|---|---|
| `tick_interval_secs` | `orchestrator.toml` | reconcile cadence (default 30 s) |
| `probe_ttl_secs` | `orchestrator.toml` | how often `ProviderHealthMap` re-probes (default 1800 s) |
| `[projects.mentions].poll_modulo` | `conf.d/<project>.toml` | poll mentions every N ticks (default 2 → 60 s) |
| `[projects.mentions].max_dispatches_per_tick` | `conf.d/<project>.toml` | cap on dispatches per reconcile (default 3) |
| `[post_merge_gate].max_test_duration_secs` | `orchestrator.toml` | wall-clock budget for `cargo test --workspace` (default 600 s) |
| agent `schedule` | `conf.d/<project>.toml` | per-agent cron expression |
| agent `model` | `conf.d/<project>.toml` | static-config routing candidate |
| agent `fallback_provider` + `fallback_model` | `conf.d/<project>.toml` | `spawn_with_fallback` target when primary fails |
| C1 allow-list | `config.rs:794` `ALLOWED_PROVIDER_PREFIXES` | recompile required |
| KG routing table | `docs/taxonomy/routing_scenarios/adf/*.md` | hot-reloaded each orchestrator start |

## Further reading

- `crates/terraphim_orchestrator/src/control_plane/routing.rs` — the full
  decision engine
- `crates/terraphim_orchestrator/src/kg_router.rs` — KG table parser +
  per-route health fall-through
- `crates/terraphim_orchestrator/src/provider_probe.rs` — probe cadence and
  `ProviderHealthMap`
- `crates/terraphim_orchestrator/src/provider_budget.rs` — tumbling-window
  budgets + `BudgetVerdict`
- `crates/terraphim_orchestrator/src/error_signatures.rs` — per-provider
  stderr classifier
- `crates/terraphim_spawner/src/lib.rs` — spawn + fallback + resource limits
- `crates/terraphim_spawner/src/config.rs` — `model_args`,
  `normalise_claude_model`, API-key inference per CLI
- `crates/terraphim_orchestrator/src/output_poster.rs` — Gitea write-back
- `docs/runbooks/roc-v1-rollout.md` — operator view of the auto-review +
  auto-merge lifecycle these decisions feed into
