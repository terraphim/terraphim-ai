# Research & Design: Native PR Gate Producers (#2334)

**Status**: Approved for implementation
**Author**: opencode session
**Date**: 2026-06-09
**Issue**: terraphim-ai#2334
**Parent**: terraphim-ai#2301, PR #2318

## Executive Summary

The previous plan correctly found the immediate execution mismatch, but the proposed fix (`/bin/bash -c <task>`) was the wrong architectural direction. It would return PR gates to shell-owned fetch, prompt assembly, posting, and verdict parsing after PR #2318 deliberately moved gate status ownership into the orchestrator.

The better fix is a native PR gate pipeline: the orchestrator assembles bounded PR context using Terraphim crates, builds deterministic gate prompts, invokes the model runner once per gate, and keeps comment/status handling in native Rust. Producer agents become pure report generators that emit a human report plus one canonical `PrGateResult` block.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | It removes a recurring ADF reliability loop rather than patching another symptom. |
| Leverages strengths? | Yes | Terraphim already has native tracker, workspace, KG matching, grep/search, runner, and orchestration crates. |
| Meets real need? | Yes | Branch-protection gates currently fail closed because producers time out or emit malformed output. |

**Proceed**: Yes. This is a HELL YES because it removes an entire class of bash/native/bash oscillation.

## Problem Statement

### Description

PR gate producers (`pr-reviewer`, `pr-validator`, `pr-verifier`) are not producing useful review evidence. They either emit huge streaming JSON/thinking output, fail to emit `adf:gate-result`, or hit the 300 second PR gate timeout added by PR #2318.

### Impact

- Branch protection no longer wedges because #2318 fails closed, but useful evidence is missing.
- Every PR can receive failure comments caused by producer mechanics rather than actual code quality.
- Operators cannot trust ADF gate comments as validation/verification evidence.

### Success Criteria

- PR gate producers receive complete, bounded native gate prompts.
- Gates do not depend on shell scripts, producer-side `gtr`, or producer-side status posts.
- `pr-reviewer`, `pr-validator`, and `pr-verifier` complete within the 300s safety cap under normal conditions.
- Each gate emits exactly one valid canonical `<!-- adf:gate-result ... -->` block.
- Orchestrator posts useful terminal comments and statuses without fallback envelopes on a synthetic PR webhook.

## Current State Analysis

### Existing Implementation

PR dispatch currently builds a short routing summary in `pr_dispatch::build_review_task()` and passes that summary to the spawner:

```rust
let task_string = pr_dispatch::build_review_task(req);
let mut request = SpawnRequest::new(primary_provider, &task_string);
```

The live TOML task bodies still contain bash scripts that fetch diffs, build prompts, call `claude -p`, post comments with `gtr`, parse verdicts, and emit gate-result blocks. Those scripts are not the right long-term interface.

### Failure Mode

Current path:

```text
PR webhook
  -> handle_review_pr()
  -> build_review_task(req) one-line summary
  -> pi-rust -p --mode json "Structural review of PR #..."
  -> model has no bounded diff, issue context, or gate-specific evidence pack
  -> model tries to discover context/skills dynamically
  -> output grows into 7000+ streaming JSONL lines
  -> 300s timeout kills agent
  -> orchestrator posts fail-closed fallback
```

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| PR fan-out | `crates/terraphim_orchestrator/src/pr_handlers_impl.rs` | Dispatches PR gate agents and attaches `gate_meta`. |
| PR metadata helpers | `crates/terraphim_orchestrator/src/pr_dispatch.rs` | Currently builds only routing summary and env overrides. |
| Gate contract | `crates/terraphim_orchestrator/src/pr_gate_result.rs` | Parses and validates `PrGateResult`. |
| Reconcile/status | `crates/terraphim_orchestrator/src/reconcile_impl.rs` | Reads drain logs, posts comments/statuses, fails closed. |
| Spawner | `crates/terraphim_spawner/src/lib.rs`, `config.rs` | Invokes CLI runners with inferred args. |
| Tracker | `crates/terraphim_tracker/src/lib.rs`, `gitea.rs` | Native Gitea comments/statuses/issues/PRs. |
| Workspace | `crates/terraphim_workspace/src/lib.rs` | Safe per-task workspace management. |
| KG matching | `crates/terraphim_automata/src/lib.rs` | Aho-Corasick term matching and paragraph extraction. |
| KG search | `crates/terraphim_grep/src/lib.rs` | Hybrid KG + ripgrep search and sufficiency judgement. |
| File search | `crates/terraphim_file_search/src/lib.rs` | KG-scored local file search. |
| Native runner patterns | `crates/terraphim_github_runner/src/lib.rs` | Runner/session/workflow execution patterns. |

## Key Research Findings

### 1. The bash fallback is a trap

The spawner already supports bash via:

```rust
"bash" | "sh" => vec!["-c".to_string()]
```

So `cli_tool = "/bin/bash"` plus `SpawnRequest::new(provider, &def.task)` would execute the existing scripts. That would probably make the immediate symptom disappear, but it would move logic back into config shell scripts:

- Diff fetching in shell.
- Prompt assembly in shell.
- Comment posting in shell.
- Verdict parsing in shell.
- Duplicate status/comment ownership against native orchestrator code.

This reintroduces the exact class of problems PR #2318 reduced.

### 2. The orchestrator should own PR evidence packs

The producer should not discover context dynamically. The orchestrator should provide a bounded, deterministic `PrGateEvidencePack` containing:

- PR metadata.
- Head SHA.
- Changed files.
- Bounded diff or per-file excerpts.
- Linked issue body/acceptance criteria when available.
- Matching domain concepts from Terraphim KG.
- Relevant nearby docs/code snippets selected by native matching/search.
- Exact gate-specific instructions and canonical output contract.

### 3. Terraphim crates already cover most primitives

- `terraphim_tracker::GiteaTracker` can fetch tracker data and post comments/statuses natively.
- `terraphim_workspace` can manage safe workspaces and hooks.
- `terraphim_automata` can extract matched KG terms and relevant paragraphs from issue text, PR title, diff, docs, and code.
- `terraphim_grep` and `terraphim_file_search` can retrieve conceptually relevant local context instead of asking the model to roam.
- `terraphim_github_runner` provides native runner/session/workflow patterns that can be reused or mirrored for deterministic execution rather than shell scripts.

### 4. The model should be a bounded report generator

The model does not need tools for the first reliable version. It should receive an evidence pack and return:

- Human-readable report.
- One final canonical `adf:gate-result` block.

No `gtr`, no status post, no shell, no dynamic skill reads.

## Vital Few

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Native ownership | Avoids bash/native/bash loop and duplicate status logic. | #2318 moved status handling into orchestrator. |
| Bounded evidence pack | Prevents runaway thinking/output and keeps within 300s. | Existing logs show 7000+ JSONL lines and timeouts. |
| KG/context matching | Uses Terraphim strengths and reduces blind LLM exploration. | `terraphim_automata`, `terraphim_grep`, `terraphim_file_search` exist. |

## Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Change PR gate agents to `/bin/bash` | Works around symptoms but regresses architecture. |
| Keep producer-side `gtr comment` | Orchestrator should own comment posting and deduplication. |
| Keep producer-side verdict parsing | `PrGateResult` is now the canonical parser. |
| Let model read skills/files dynamically | Causes permission errors and unbounded exploration. |
| Build full Firecracker runner for gate reports now | Too large for #2334; native runner patterns are enough for this fix. |

## Recommended Design

### Overview

Create a native PR gate evidence and prompt pipeline inside `terraphim_orchestrator`.

```text
PR webhook
  -> ReviewPrRequest
  -> Native PrGateEvidencePack builder
       -> Gitea PR metadata/comments/issues via terraphim_tracker
       -> git diff/changed files from controlled workspace
       -> KG concepts via terraphim_automata
       -> relevant docs/code snippets via terraphim_grep/file_search
  -> Gate-specific prompt builder
  -> Spawn model runner with bounded prompt
  -> Reconcile parses PrGateResult
  -> Orchestrator posts comment + status
```

### Component Diagram

```text
dispatcher::ReviewPr
        |
        v
pr_handlers_impl::dispatch_pr_gate_for_pr
        |
        v
pr_gate_context::build_evidence_pack
        |---- terraphim_tracker::GiteaTracker
        |---- controlled git workspace / terraphim_workspace
        |---- terraphim_automata term extraction
        |---- terraphim_grep / terraphim_file_search context retrieval
        v
pr_gate_prompt::build_prompt(kind, evidence)
        |
        v
terraphim_spawner native model CLI runner
        |
        v
reconcile_impl + pr_gate_result
```

## Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Build native evidence pack before spawning | The model should review evidence, not discover it. | Shell scripts; free-roaming tool agents. |
| Keep `task_string` for routing only | Routing needs a compact summary; execution needs full prompt. | Reuse summary as prompt. |
| Use Terraphim matching/search for context selection | Reduces prompt size and aligns with project capabilities. | Dump entire diff/docs into prompt. |
| Orchestrator posts all comments/statuses | Single source of truth; aligns with #2318. | Producer-side `gtr` and curl posts. |
| No tools for initial producer run | Bounded deterministic execution. | Allow Bash/Read/Grep tools and risk loops. |

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/pr_gate_context.rs` | Native evidence-pack construction. |
| `crates/terraphim_orchestrator/src/pr_gate_prompt.rs` | Gate-specific prompt rendering. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Export new modules. |
| `crates/terraphim_orchestrator/src/pr_handlers_impl.rs` | Replace one-line prompt with native gate prompt; keep routing summary as `ADF_TASK_SUMMARY`. |
| `crates/terraphim_orchestrator/src/pr_dispatch.rs` | Add `PrGateKind`/gate entry helpers or keep routing helpers only. |
| `crates/terraphim_orchestrator/Cargo.toml` | Add crate deps only if not already present: `terraphim_automata`, `terraphim_grep`, `terraphim_file_search`, `terraphim_workspace`. |
| `/opt/ai-dark-factory/conf.d/terraphim.toml` | Replace PR gate `task` bash bodies with short labels/descriptions; remove producer-side posting instructions. |

## API Design

### Gate Kind

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrGateKind {
    Review,
    Validation,
    Verification,
}

impl PrGateKind {
    pub fn agent_name(self) -> &'static str;
    pub fn context(self) -> &'static str;
    pub fn title(self) -> &'static str;
}
```

### Evidence Pack

```rust
#[derive(Debug, Clone)]
pub struct PrGateEvidencePack {
    pub pr_number: u64,
    pub project: String,
    pub title: String,
    pub author: String,
    pub head_sha: String,
    pub diff_loc: u32,
    pub changed_files: Vec<String>,
    pub diff_excerpt: String,
    pub linked_issue: Option<LinkedIssueEvidence>,
    pub matched_concepts: Vec<String>,
    pub relevant_context: Vec<RelevantContextChunk>,
}

#[derive(Debug, Clone)]
pub struct LinkedIssueEvidence {
    pub number: u64,
    pub title: String,
    pub body_excerpt: String,
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RelevantContextChunk {
    pub source: String,
    pub reason: String,
    pub text: String,
}
```

### Evidence Builder

```rust
pub async fn build_pr_gate_evidence_pack(
    config: &OrchestratorConfig,
    tracker: Option<&GiteaTracker>,
    req: &ReviewPrRequest,
    limits: PrGateEvidenceLimits,
) -> Result<PrGateEvidencePack, PrGateContextError>;
```

### Prompt Builder

```rust
pub fn build_pr_gate_prompt(
    gate: PrGateKind,
    evidence: &PrGateEvidencePack,
) -> String;
```

### Limits

```rust
#[derive(Debug, Clone)]
pub struct PrGateEvidenceLimits {
    pub max_diff_lines: usize,
    pub max_issue_chars: usize,
    pub max_context_chunks: usize,
    pub max_context_chars: usize,
}
```

Default limits should keep total prompt size bounded, for example:

- `max_diff_lines = 1200`
- `max_issue_chars = 6000`
- `max_context_chunks = 8`
- `max_context_chars = 12000`

## Prompt Contract

Every native gate prompt must include:

```text
You are a bounded PR gate report producer.

Rules:
- Use only the evidence in this prompt.
- Do not call tools.
- Do not post comments or statuses.
- Do not invent files or tests not present in evidence.
- Keep the human report under 1200 words.
- End with exactly one canonical adf:gate-result block.
```

Then gate-specific sections:

- `Review`: structural correctness, security, regressions, maintainability.
- `Validation`: requirements and acceptance criteria satisfaction.
- `Verification`: design/spec conformance and test evidence.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `build_prompt_contains_contract_for_each_gate` | `pr_gate_prompt.rs` | Ensures canonical output block instructions are present. |
| `build_prompt_disallows_tool_and_status_posts` | `pr_gate_prompt.rs` | Prevents regression to producer-side actions. |
| `evidence_pack_bounds_diff_and_context` | `pr_gate_context.rs` | Verifies prompt-size limits. |
| `extract_issue_number_from_pr_title_and_branch` | `pr_gate_context.rs` | Supports validation against linked issue. |
| `concept_matching_extracts_terms_from_diff_and_issue` | `pr_gate_context.rs` | Uses `terraphim_automata` rather than ad hoc string search. |

### Integration Tests

| Test | Purpose |
|------|---------|
| Synthetic PR evidence pack from fixture repo | Verifies changed files, diff excerpt, issue evidence, and KG concepts. |
| Spawn request uses gate prompt not routing summary | Prevents recurrence of current bug. |
| Reconcile accepts native generated gate-result | End-to-end parser/status path. |

### Live Verification

1. Build and deploy ADF from the branch.
2. Trigger synthetic PR webhook for PR #2318 or a small fixture PR.
3. Verify `pr-reviewer`, `pr-validator`, `pr-verifier` finish before 300s.
4. Verify terminal comments are useful human reports, not fallback envelopes.
5. Verify `adf/pr-reviewer`, `adf/validation`, `adf/verification` terminal statuses reflect parsed `PrGateResult`.

## Implementation Steps

### Step 1: Add `PrGateKind` and routing separation

**Files**: `pr_dispatch.rs` or new `pr_gate_prompt.rs`
**Description**: Separate routing summary from execution prompt.
**Tests**: Gate kind maps to expected agent/context.

### Step 2: Build native evidence pack

**Files**: `pr_gate_context.rs`
**Description**: Construct bounded `PrGateEvidencePack` using tracker, workspace/git diff, Terraphim matching/search.
**Tests**: Bounds, issue extraction, changed-file fixtures.

### Step 3: Build gate-specific prompts

**Files**: `pr_gate_prompt.rs`
**Description**: Render review/validation/verification prompts with exact contract.
**Tests**: Prompt snapshots or structural assertions.

### Step 4: Wire PR dispatch to native prompts

**Files**: `pr_handlers_impl.rs`
**Description**: Use routing summary only for route selection and `ADF_TASK_SUMMARY`; use `build_pr_gate_prompt()` for `SpawnRequest::new()`.
**Tests**: Spawn request receives full prompt.

### Step 5: Simplify PR gate config

**Files**: `/opt/ai-dark-factory/conf.d/terraphim.toml` and committed config template if present.
**Description**: Replace bash task bodies with short role labels/descriptions. No producer-side posting instructions. Keep subscription models.
**Tests**: Config load validation.

### Step 6: Deploy and live-verify

**Description**: Build, back up live binary/config, deploy, restart, synthetic webhook proof.
**Tests**: Live statuses and comments.

## Rollback Plan

1. Restore previous ADF binary backup.
2. Restore previous `terraphim.toml` backup.
3. Restart `adf-orchestrator.service`.
4. The #2318 fail-closed fallback remains safe if producer output regresses.

## Approval Gate

This design intentionally rejects the bash workaround. Implementation should proceed only after approval of the native evidence-pack + prompt-builder approach.

## Open Questions

| Question | Default Decision |
|----------|------------------|
| Should model tools be disabled for PR gates? | Yes for #2334; evidence pack should be complete. |
| Should `terraphim_grep` be mandatory or best-effort? | Best-effort initially; diff + issue evidence are mandatory. |
| Should native runner crate be directly reused? | Reuse patterns now; direct dependency only if it reduces code. |
| Should PR gate configs keep `skill_chain`? | Keep metadata for observability, but do not require runtime skill reads. |
