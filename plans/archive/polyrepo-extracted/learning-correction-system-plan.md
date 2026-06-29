# Learning and Correction System -- Research and Design Plan

## Phase 1: Research Findings

### 1.1 Current State of Implementation

#### learnings/capture.rs (1800 LOC) -- FULLY FUNCTIONAL
- `CapturedLearning` struct with UUID-timestamp IDs, markdown serialization/deserialization
- `CorrectionEvent` struct for typed user corrections (ToolPreference, CodePattern, Naming, WorkflowStep, FactCorrection, StylePreference, Other)
- `LearningEntry` enum unifying Learning + Correction for display
- `ScoredEntry` with keyword-based relevance scoring (dead code, not wired)
- `TranscriptEntry` type for JSONL auto-extraction (dead code, not wired)
- Storage: individual markdown files with YAML frontmatter in `.terraphim/learnings/` or `~/.local/share/terraphim/learnings/`
- Functions: `capture_failed_command`, `capture_correction`, `correct_learning`, `list_all_entries`, `query_all_entries`
- Auto-suggest from KG: TODO comment at line 609, NOT IMPLEMENTED

#### learnings/redaction.rs (180 LOC) -- FULLY WIRED
- SECRET_PATTERNS: AWS keys, OpenAI keys, Slack tokens, GitHub tokens, connection strings (postgresql, mysql, mongodb, redis)
- ENV_VAR_PATTERNS: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, DATABASE_URL, API_KEY, etc.
- `redact_secrets()` is called in `capture_failed_command()` (line 582) and `capture_correction()` (lines 654-656)
- `contains_secrets()` exists but marked `#[allow(dead_code)]` -- not used as a pre-check
- Issue #480 (Gitea) claims this is "partially implemented" -- in fact it IS wired for capture pipeline. What may be missing: hook pipeline does not apply redaction to the raw HookInput before passing through stdout (the hook passes through the original JSON unredacted).

#### learnings/procedure.rs (501 LOC) -- GATED BEHIND #[cfg(test)]
- `ProcedureStore` with JSONL storage, save/load/delete/find_by_title/find_by_id
- `save_with_dedup()` using Aho-Corasick matching on procedure titles via `terraphim_automata::matcher::find_matches`
- `update_confidence()` for recording success/failure
- Types from `terraphim_types::procedure`: `CapturedProcedure`, `ProcedureStep`, `ProcedureConfidence`
- CRITICAL: Line 29-30 of mod.rs has `#[cfg(test)] mod procedure;` -- this module is ONLY compiled in test builds, NOT available at runtime
- All tests pass and cover save, load, dedup, confidence updates, delete

#### learnings/hook.rs (545 LOC) -- FULLY FUNCTIONAL
- `HookInput`, `ToolInput`, `ToolResult` structs for parsing agent hook JSON
- `process_hook_input()` reads stdin, parses, captures if failed Bash, passes through original JSON (fail-open)
- Only captures Bash tool failures (exit_code != 0)
- Does NOT capture success events -- relevant for #693

#### learnings/install.rs (346 LOC) -- FULLY FUNCTIONAL
- Generates shell hook scripts for Claude, Codex, Opencode
- Scripts pipe to `terraphim-agent learn hook --format <agent>`
- Install/uninstall/status checking

#### terraphim_types/src/procedure.rs -- FULLY IMPLEMENTED
- `ProcedureStep`: ordinal, command, precondition, postcondition, working_dir, privileged, tags
- `ProcedureConfidence`: success_count, failure_count, score, is_high_confidence (> 0.8)
- `CapturedProcedure`: id, title, description, steps, confidence, tags, created_at, updated_at, source_session
- Methods: add_step, add_steps, merge_steps, record_success, record_failure, with_source_session, with_tags, with_confidence

#### shared_learning/ -- IMPLEMENTED BUT NOT WIRED TO CLI
- `SharedLearningStore` backed by `terraphim_persistence` with BM25 deduplication
- `SharedLearning` type with trust levels (L1/L2/L3), quality metrics, applicable agents, keywords, verify pattern
- `QualityMetrics` with applied_count, effective_count, agent_count, success_rate, L2 promotion criteria
- `GiteaWikiClient` for syncing L2/L3 learnings to Gitea wiki
- `SharedLearningRecord` implements `Persistable` trait
- NOT referenced in main.rs at all -- no CLI subcommands exist for shared learnings

#### terraphim_agent_evolution/ -- STANDALONE CRATE, NOT INTEGRATED
- `AgentEvolutionSystem` with memory, task, and lesson evolution tracking
- `EvolutionWorkflowManager` with workflow factory and LLM adapter integration
- Lessons, memory snapshots, task lifecycle tracking
- Uses mock LLM adapters in integration -- not wired to real LLM providers
- NOT referenced by terraphim_agent main binary

### 1.2 Dependency Analysis Between Issues

```
Foundation Layer (no dependencies):
  #480 (redaction)     -- partially done, needs hook stdout redaction
  #578 (--robot/--format flags) -- standalone bug fix

Procedural Memory Layer (depends on foundation):
  #693 (success capture) -- depends on un-gating procedure.rs
  #703 (entity annotation) -- depends on terraphim_automata being available at runtime

Replay Layer (depends on procedural memory):
  #694 (procedure replay) -- depends on #693

Monitoring Layer (depends on replay):
  #695 (self-healing procedures) -- depends on #694

Hook Pipeline Layer (independent):
  #599 (multi-hook pipeline) -- depends on hook.rs changes
  #686 (typed tool hooks) -- evaluation only, no implementation dependency

Graduated Guard Layer (depends on hook pipeline):
  #704 (sandbox tier) -- depends on #599

Validation Layer (independent, Gitea issues):
  #515 (PreToolUse validation) -- independent
  #516 (KG command matching) -- depends on #515
  #517 (wire into Claude Code) -- depends on #516
  #451 (LLM hooks unwired) -- related to validation

Agent Evolution Layer (depends on shared_learning):
  #727 (wire into ADF) -- depends on shared_learning being wired
  #728 (haystack ServiceType) -- depends on #727
  #729 (cross-run compounding) -- depends on #728
  #730 (nightwatch stagnation) -- depends on #729
```

### 1.3 Key Findings

1. **procedure.rs is complete code gated behind #[cfg(test)]** -- Removing the gate and adding CLI subcommands would close most of #693 with minimal work.

2. **Redaction IS wired** into capture pipeline -- #480 is mostly done. The gap: when the hook passes through stdout, the original unredacted JSON goes to the agent. Redaction only applies to what gets stored.

3. **shared_learning is a complete module with no CLI integration** -- It has persistence, BM25, trust levels, wiki sync, but zero exposure through the CLI. This is a prerequisite for #727-#730.

4. **Auto-suggest from KG is a TODO** (capture.rs line 609) -- This is the core of #703 (entity annotation).

5. **terraphim_agent_evolution uses mock LLM adapters** -- It needs real adapter wiring before #727 can proceed.

6. **The hook only captures Bash failures** -- #693 requires extending it to also capture successful command sequences. #599 requires multi-hook types (PreToolUse, PostToolUse, UserPromptSubmit).

### 1.4 Vital Few Constraints

- All new code must compile with `cargo check --workspace`
- Tests must use real implementations (no mocks per project rules, though agent_evolution currently uses mock LLM)
- The learning system must remain fail-open (hook failures must not block agent execution)
- Secret redaction must apply to all stored content (already enforced)
- CLI interface must stay backward compatible (existing `learn` subcommands must not change behavior)


## Phase 2: Design Plan

### Implementation Phases

#### Phase A: Foundation Fixes (S complexity, 1-2 days)

**Issues: #480, #578**

1. **#480 -- Complete redaction wiring**
   - Files: `crates/terraphim_agent/src/learnings/hook.rs`
   - Change: In `process_hook_input()`, redact the command and error output in the HookInput before passing through to stdout. Currently only capture-side gets redacted.
   - Add: Call `redact_secrets()` on stdout passthrough for stderr/stdout fields
   - Add: `contains_secrets()` pre-check logging (currently dead code)
   - Test: Unit test with HookInput containing AWS key, verify passthrough is redacted
   - Complexity: S

2. **#578 -- Fix --robot and --format flags for search**
   - Files: `crates/terraphim_agent/src/main.rs` (search command handler)
   - Change: Wire the `--robot` and `--format` CLI flags to the search output formatting
   - Test: Integration test with `--robot` flag
   - Complexity: S

#### Phase B: Procedural Memory (M complexity, 3-5 days)

**Issues: #693**

1. **Un-gate procedure.rs**
   - File: `crates/terraphim_agent/src/learnings/mod.rs` line 29-30
   - Change: Remove `#[cfg(test)]` from `mod procedure;`
   - Add: `pub use procedure::ProcedureStore;` to public exports
   - Complexity: S (trivial change but enables everything downstream)

2. **Extend hook to capture successes**
   - File: `crates/terraphim_agent/src/learnings/hook.rs`
   - Change: Add `should_capture_success()` method that returns true for Bash exit_code==0 with multi-step sequences
   - Add: `capture_success_from_hook()` function that creates CapturedProcedure from successful command sequence
   - Add: Session tracking to group related successful commands into procedures
   - New type needed: `SessionCommandBuffer` to accumulate successful commands within a session
   - Complexity: M

3. **Add CLI subcommands for procedures**
   - File: `crates/terraphim_agent/src/main.rs`
   - Add subcommands under `learn`:
     - `learn procedure list [--recent N]` -- list stored procedures
     - `learn procedure show ID` -- show procedure details
     - `learn procedure record --title TITLE --description DESC` -- start recording
     - `learn procedure success ID` / `learn procedure failure ID` -- update confidence
   - Complexity: M

4. **Wire ProcedureStore into hook pipeline**
   - File: `crates/terraphim_agent/src/learnings/hook.rs`
   - Add: On successful multi-command Bash sequence, save via ProcedureStore
   - Add: Dedup check against existing procedures before saving
   - Complexity: M

**Test strategy**: Unit tests for ProcedureStore (already exist), integration test for CLI procedure subcommands, hook capture test with success scenario.

#### Phase C: Entity Annotation (M complexity, 3-5 days)

**Issues: #703**

1. **Add KG entity annotation to learning captures**
   - File: `crates/terraphim_agent/src/learnings/capture.rs`
   - Change: Implement the TODO at line 609 -- use `terraphim_automata::matcher::find_matches` to annotate captured learnings with KG entities
   - Add: `annotate_with_entities()` function that takes command + error text and returns matched NormalizedTerms
   - Add: `entities` field to `CapturedLearning` struct (Vec<String> of matched term IDs)
   - Add: Entity-based query function `query_by_entity()` alongside existing text search
   - Complexity: M

2. **Enhance query to support semantic search via entities**
   - File: `crates/terraphim_agent/src/learnings/capture.rs`
   - Add: `query_by_entities()` that loads thesaurus, matches query against KG, then filters learnings by matching entities
   - Add: CLI flag `--semantic` to `learn query` subcommand
   - Complexity: M

**Test strategy**: Unit test with known thesaurus, capture command containing known terms, verify entities are annotated. Query test with semantic flag.

**Prerequisite**: A thesaurus must be loadable at capture time. The system already supports loading from JSON files. The config would need a `thesaurus_path` field.

#### Phase D: Procedure Replay Engine (M complexity, 3-5 days)

**Issues: #694**

1. **Add replay command**
   - File: `crates/terraphim_agent/src/main.rs`
   - Add: `learn procedure replay ID [--dry-run]` subcommand
   - Logic: Load procedure by ID, check confidence > 0.8, execute steps in order
   - Each step: check precondition, execute command, verify postcondition
   - Record success/failure after execution
   - `--dry-run` mode: print steps without executing
   - Complexity: M

2. **Add replay engine module**
   - New file: `crates/terraphim_agent/src/learnings/replay.rs`
   - Types: `ReplayResult` (steps executed, steps skipped, errors), `StepOutcome` (Success, Failed, Skipped)
   - Function: `replay_procedure(procedure: &CapturedProcedure, dry_run: bool) -> ReplayResult`
   - Safety: Never execute privileged steps without explicit confirmation
   - Complexity: M

**Test strategy**: Integration test with a simple 2-step procedure (echo commands), verify replay executes and updates confidence.

#### Phase E: Multi-Hook Pipeline and Importance Scoring (L complexity, 5-8 days)

**Issues: #599, #686**

1. **Extend hook types beyond PostToolUse**
   - File: `crates/terraphim_agent/src/learnings/hook.rs`
   - Add: `HookType` enum: PreToolUse, PostToolUse, UserPromptSubmit
   - Add: `--hook-type` flag to `learn hook` subcommand
   - Add: PreToolUse handler that checks against known error patterns before execution
   - Add: UserPromptSubmit handler that captures user corrections inline
   - Complexity: M

2. **Importance scoring**
   - File: `crates/terraphim_agent/src/learnings/capture.rs`
   - Add: `ImportanceScore` struct with factors: error_severity, repetition_count, recency, user_correction_present
   - Add: `calculate_importance()` function
   - Add: `importance` field to CapturedLearning
   - Add: Sort by importance in `list_all_entries()`
   - Complexity: M

3. **#686 -- Typed tool hooks evaluation**
   - Deliverable: Decision document (no code)
   - Evaluate whether terraphim-skills plugin should define typed hook interfaces
   - Compare with current shell-script-based hooks
   - Complexity: S

**Test strategy**: Unit tests for importance scoring, integration test for multi-hook pipeline with different hook types.

#### Phase F: Self-Healing Procedures (M complexity, 3-5 days)

**Issues: #695**

1. **ProcedureHealthReport**
   - File: `crates/terraphim_agent/src/learnings/procedure.rs`
   - Add: `ProcedureHealthReport` struct: rolling_success_rate (last N executions), trend (improving/degrading/stable), auto_disabled flag
   - Add: `health_check()` method on ProcedureStore that returns Vec<ProcedureHealthReport>
   - Add: `auto_disable()` that sets a `disabled` flag when success_rate drops below 0.5 over last 10 executions
   - Complexity: M

2. **CLI health command**
   - File: `crates/terraphim_agent/src/main.rs`
   - Add: `learn procedure health` subcommand showing health reports
   - Add: `learn procedure enable/disable ID` subcommands
   - Complexity: S

**Prerequisite**: Phase D (replay engine) must exist so procedures have execution history.

**Test strategy**: Unit test creating procedure with declining confidence, verify auto-disable triggers.

#### Phase G: Shared Learning CLI Integration (L complexity, 5-8 days)

**Issues: #727 (partial)**

1. **Wire shared_learning into CLI**
   - File: `crates/terraphim_agent/src/main.rs`
   - Add subcommand group: `learn shared`
     - `learn shared list [--trust-level L1|L2|L3]`
     - `learn shared promote ID --to L2|L3`
     - `learn shared sync` (trigger wiki sync)
     - `learn shared import` (import from local learnings to shared store)
   - Complexity: M

2. **Bridge local learnings to shared store**
   - File: `crates/terraphim_agent/src/shared_learning/store.rs`
   - Add: `import_from_local()` function that reads local CapturedLearning/CorrectionEvent files and creates SharedLearning entries at L1
   - Add: BM25 dedup check during import
   - Complexity: M

**Test strategy**: Integration test importing local learning, verifying dedup, promoting to L2.

#### Phase H: Graduated Guard (L complexity, 5-8 days)

**Issues: #704**

1. **Three-tier execution model**
   - New file: `crates/terraphim_agent/src/learnings/guard.rs`
   - Types: `ExecutionTier` (Allow, Sandbox, Deny), `GuardDecision`
   - Function: `evaluate_command(command: &str) -> GuardDecision`
   - Pattern matching: known-safe commands (allow), unknown commands (sandbox), known-dangerous patterns (deny)
   - Integration with learning feedback: commands that previously failed get elevated caution
   - Complexity: L (depends on secure-exec/Firecracker integration)

**Note**: The Firecracker integration (`terraphim_firecracker/`) already exists for sandboxed execution. This phase would wire the guard decision into the Firecracker execution path.

**Test strategy**: Unit test for pattern matching decisions, integration test with Firecracker sandbox for sandboxed tier.

#### Phase I: Agent Evolution Integration (L complexity, 8-12 days)

**Issues: #727, #728, #729, #730**

These are the most complex and depend on Phases B, C, F, G being complete.

1. **#727 -- Wire agent_evolution into ADF orchestrator**
   - Connect `EvolutionWorkflowManager` to real LLM adapters (not mocks)
   - Wire learning capture events to evolution system
   - Complexity: L

2. **#728 -- AgentEvolution haystack ServiceType**
   - Add new ServiceType variant to haystack enum
   - Create indexer for evolution data
   - Complexity: M

3. **#729 -- Cross-run compounding via lesson injection**
   - Use shared_learning store to inject L2/L3 learnings into agent context at startup
   - Validate injected lessons against current codebase state
   - Complexity: L

4. **#730 -- Nightwatch stagnation detection**
   - Monitor agent evolution metrics for stagnation patterns
   - Trigger redirection when an agent is making no progress
   - Complexity: L

#### Phase J: Validation Pipeline (Gitea issues, M complexity)

**Issues: Gitea #515, #516, #517, #451**

1. **#515 -- PreToolUse validation pipeline**
   - Extend terraphim_hooks to validate commands before execution
   - Use KG patterns to detect unsafe or incorrect commands
   - Complexity: M

2. **#516 -- KG command pattern matching**
   - Build Aho-Corasick automata from command patterns in KG
   - Match incoming commands against known patterns
   - Complexity: M

3. **#517 -- Wire into Claude Code /execute pipeline**
   - Integrate validation into the hook scripts generated by install.rs
   - Complexity: S

4. **#451 -- LLM hooks unwired**
   - Wire LLM validation hooks in agent.rs
   - Complexity: M


### Recommended Implementation Order

```
Phase A (S) -----> Phase B (M) -----> Phase C (M)
  #480, #578         #693              #703
                       |
                       v
                  Phase D (M) -----> Phase F (M)
                    #694               #695
                       |
Phase E (L)            |
  #599, #686           |
       |               v
       v          Phase G (L)
  Phase H (L)      #727 partial
    #704               |
                       v
                  Phase I (L)
                    #727-#730

Phase J (M) -- independent track
  Gitea #515-#517, #451
```

### Issues Closable with Minimal Work

| Issue | Work Required | Estimate |
|-------|--------------|----------|
| #480 | Add redaction to hook stdout passthrough + verify `contains_secrets` | 2-3 hours |
| #578 | Wire --robot/--format flags in search handler | 2-3 hours |
| #693 (partial) | Remove `#[cfg(test)]` from procedure.rs, add CLI subcommands | 1 day |
| #686 | Write evaluation document only | 0.5 day |

### Issues Requiring Significant New Code

| Issue | New Code Required | Estimate |
|-------|------------------|----------|
| #693 (full) | Success capture in hook pipeline, session tracking | 3-5 days |
| #694 | Replay engine module, CLI command, safety checks | 3-5 days |
| #695 | Health monitoring, auto-disable, rolling window | 3-5 days |
| #703 | Entity annotation, thesaurus loading at capture time, semantic query | 3-5 days |
| #599 | Multi-hook types, importance scoring | 5-8 days |
| #704 | Guard tiers, Firecracker integration | 5-8 days |
| #727-#730 | Full evolution integration | 8-12 days |

### Eliminated Options (5/25 Rule)

The following were considered but explicitly excluded from the initial plan:

1. **Real-time streaming of learnings** -- Over-engineered for current use case. File-based storage is sufficient.
2. **LLM-based auto-correction suggestions** -- Requires LLM at capture time which adds latency to fail-open hook. Deferred to Phase I.
3. **Web UI for learning management** -- CLI-first approach is consistent with project philosophy.
4. **Cross-machine learning sync via CRDTs** -- Gitea wiki sync (already implemented in shared_learning) is simpler and sufficient.
5. **Custom storage backend for procedures** -- JSONL files are adequate for the expected volume. ProcedureStore already handles this well.

### Risk Assessment

1. **Procedure session tracking (Phase B)**: Grouping related successful commands into procedures requires understanding session boundaries. Risk: Claude Code hooks provide individual tool results, not session context. Mitigation: Use timestamps and working directory to infer session grouping.

2. **Entity annotation performance (Phase C)**: Loading thesaurus on every capture could slow the fail-open hook. Mitigation: Lazy-load and cache the automata, or annotate asynchronously after storage.

3. **Firecracker guard integration (Phase H)**: Requires Firecracker to be available on the execution host. Mitigation: Fallback to allow/deny (skip sandbox tier) when Firecracker is unavailable.

4. **Agent evolution mock LLM (Phase I)**: agent_evolution uses `LlmAdapterFactory::create_mock()`. Wiring real adapters requires choosing between OpenRouter/Ollama and handling credentials. Mitigation: Gate behind feature flag.
