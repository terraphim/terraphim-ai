# Implementation Plan: Fast/Cheap LLM Build-Runner with Learning

**Status**: Draft
**Research Doc**: `.docs/research-fast-cheap-build-runner.md`
**Author**: opencode (k2p6)
**Date**: 2026-05-11
**Estimated Effort**: 2 days

## Overview

### Summary
Replace the hardcoded bash build-runner with an LLM-driven agent that extracts build commands from markdown documentation (BUILD.md, CONTRIBUTING.md) and terraphim-agent learnings, then executes them via rch with deterministic fallback.

### Approach
Three-tier approach:
1. **Learning tier**: Query terraphim-agent for known successful build commands per project
2. **Documentation tier**: Use cheap LLM (haiku/zai) to extract build commands from markdown
3. **Validation tier**: Cross-reference extracted commands against known-safe patterns before execution

### Scope
**In Scope:**
- New `build-runner-llm` agent template
- `build::` markdown directive parser extension
- terraphim-agent learning integration
- Command validation (whitelist + dry-run)
- Cost tracking and alerting

**Out of Scope:**
- Replacing rch (remote compilation stays)
- Natural language build descriptions (too complex for cheap models)
- Real-time build optimization
- Multi-step LLM reasoning chains

**Avoid At All Cost**:
- Full LLM execution (too risky for CI)
- Removing existing build-runner (must coexist with feature flag)
- Using expensive models (sonnet, k2p5) for simple command extraction

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    build-runner-llm Agent                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   │
│  │   Learning   │   │  Markdown    │   │  Validation  │   │
│  │   Query      │   │  Extractor   │   │   Engine     │   │
│  │  (terraphim) │   │   (haiku)    │   │ (whitelist)  │   │
│  └──────┬───────┘   └──────┬───────┘   └──────┬───────┘   │
│         │                  │                  │            │
│         └──────────────────┼──────────────────┘            │
│                            ▼                               │
│                   ┌─────────────────┐                      │
│                   │  Command Merge  │                      │
│                   │  (deduplicate)  │                      │
│                   └────────┬────────┘                      │
│                            ▼                               │
│                   ┌─────────────────┐                      │
│                   │  rch Executor   │                      │
│                   │ (remote build)  │                      │
│                   └────────┬────────┘                      │
│                            ▼                               │
│                   ┌─────────────────┐                      │
│                   │ Status Reporter │                      │
│                   │ (Gitea API)     │                      │
│                   └─────────────────┘                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                            │
                    ┌───────┴───────┐
                    ▼               ▼
            ┌────────────┐  ┌────────────┐
            │  Success   │  │  Fallback  │
            │ (continue) │  │ (hardcoded)│
            └────────────┘  └────────────┘
```

### Data Flow

```
Push Event → build-runner-llm
    ├── Query terraphim-agent learnings → ["cargo fmt", "cargo clippy", ...]
    ├── Parse BUILD.md/CONTRIBUTING.md → haiku extracts commands
    ├── Validate commands against whitelist
    ├── Merge and deduplicate command list
    ├── Execute via rch exec
    └── POST_STATUS success/failure
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use haiku for extraction | Proven for simple tasks, upstream-synchronizer uses it successfully | zai/glm-5 (slower), sonnet (expensive) |
| terraphim-agent learnings as primary source | Learnings capture actual successful commands, not just documentation | Documentation-only (stale), No learning (repetitive failures) |
| Whitelist validation before execution | Prevents LLM hallucinations from executing dangerous commands | Full sandboxing (overkill), No validation (unsafe) |
| Feature flag coexistence | Must not break existing builds during transition | Full replacement (too risky) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Full LLM execution chain | Too complex for cheap models, increases latency | Build failures, timeout errors |
| Removing deterministic fallback | Breaks builds when LLM fails | CI outages, blocked PRs |
| Using expensive models | Violates $0.01 cost constraint | Budget overrun, unsustainable |
| Parsing all markdown files | Too much noise, slow | False positives, increased latency |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative."

**What if this could be easy?**
- Query learnings → if found, use them
- If no learnings, ask haiku to read BUILD.md and extract commands
- Validate commands against simple whitelist
- Execute via existing rch infrastructure
- Post status via existing POST_STATUS helper

**Senior Engineer Test**: Would a senior engineer call this overcomplicated?
Answer: No. This is a thin wrapper around existing infrastructure (terraphim-agent, haiku, rch) with deterministic fallback.

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_automata/src/build_directives.rs` | Parse `build::` directives from markdown |
| `scripts/build-runner-llm.sh` | New agent task script template |
| `.docs/BUILD.md` | Project build documentation (example for parser) |

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_automata/src/markdown_directives.rs` | Add `build::` directive support |
| `crates/terraphim_automata/src/lib.rs` | Export build directive parser |
| `Cargo.toml` | Add `build-directives` feature flag |
| `crates/terraphim_orchestrator/src/config.rs` | Add `build_runner_llm_enabled` config |
| `docs/taxonomy/routing_scenarios/adf/implementation_tier.md` | Add build-runner-llm route |

### Agent Config Changes
| File | Changes |
|------|---------|
| `/opt/ai-dark-factory/conf.d/terraphim.toml` | Add `build-runner-llm` agent definition |

## API Design

### New Types

```rust
/// Build command extracted from documentation or learnings
#[derive(Debug, Clone)]
pub struct BuildCommand {
    /// The shell command to execute
    pub command: String,
    /// Source of the command (learning, markdown, fallback)
    pub source: CommandSource,
    /// Confidence score (0-1) for LLM-extracted commands
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum CommandSource {
    Learning,
    Markdown(PathBuf),
    Fallback,
}

/// Validation result for extracted commands
#[derive(Debug)]
pub struct ValidationResult {
    /// Validated commands ready for execution
    pub valid_commands: Vec<BuildCommand>,
    /// Rejected commands with reasons
    pub rejected: Vec<(BuildCommand, String)>,
}
```

### Public Functions

```rust
/// Extract build commands from terraphim-agent learnings
///
/// # Arguments
/// * `project_id` - Project identifier for filtering learnings
///
/// # Returns
/// List of build commands from learnings, empty if none found
pub fn extract_from_learnings(project_id: &str) -> Vec<BuildCommand>;

/// Extract build commands from markdown documentation
///
/// # Arguments
/// * `docs` - List of markdown file paths to parse
/// * `llm_client` - Cheap LLM client (haiku) for extraction
///
/// # Returns
/// Validated list of build commands
///
/// # Errors
/// Returns `BuildError::ExtractionFailed` if LLM fails
pub async fn extract_from_markdown(
    docs: &[PathBuf],
    llm_client: &dyn LlmClient,
) -> Result<Vec<BuildCommand>, BuildError>;

/// Validate commands against known-safe patterns
///
/// Whitelist includes: cargo *, make *, npm *, yarn *, pnpm *, bun *
/// Rejects: rm *, sudo *, curl | sh, etc.
pub fn validate_commands(commands: &[BuildCommand]) -> ValidationResult;
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("LLM extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("No build commands found in learnings or documentation")]
    NoCommandsFound,

    #[error("All extracted commands failed validation")]
    ValidationFailed,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_extract_from_learnings` | `build_directives.rs` | Verify learning query returns commands |
| `test_extract_from_markdown` | `build_directives.rs` | Verify haiku extracts commands from BUILD.md |
| `test_validate_whitelist` | `build_directives.rs` | Verify safe commands pass, dangerous ones fail |
| `test_merge_deduplicate` | `build_directives.rs` | Verify learning + markdown commands merge correctly |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_build_runner_llm_e2e` | `tests/build_runner_llm.rs` | Full flow: push event → extract → validate → execute |
| `test_fallback_to_deterministic` | `tests/build_runner_llm.rs` | Verify fallback when LLM fails |
| `test_cost_tracking` | `tests/build_runner_llm.rs` | Verify cost stays under $0.01 |

### Property Tests
```rust
proptest! {
    #[test]
    fn validate_never_allows_dangerous_commands(cmd: String) {
        let result = validate_commands(&[BuildCommand { command: cmd, source: Fallback, confidence: None }]);
        // Commands containing rm -rf, sudo, curl | sh should always be rejected
    }
}
```

## Implementation Steps

### Step 1: Extend terraphim_automata with build directives
**Files:** `crates/terraphim_automata/src/build_directives.rs`, `src/lib.rs`
**Description:** Add `build::` directive parser and command extraction
**Tests:** Unit tests for parsing
**Estimated:** 4 hours

```rust
// Key code to write
pub fn parse_build_directives(content: &str) -> Vec<BuildCommand> {
    // Parse ```bash or ```sh code blocks
    // Parse `build::` directives
}
```

### Step 2: Create build-runner-llm agent template
**Files:** `scripts/build-runner-llm.sh`, update `terraphim.toml`
**Description:** New agent that queries learnings + extracts from markdown
**Tests:** Manual testing with sample BUILD.md
**Dependencies:** Step 1
**Estimated:** 3 hours

```bash
# Key script structure
#!/bin/bash
set -e

# 1. Query learnings
LEARNINGS=$(~/.cargo/bin/terraphim-agent learn query "build commands terraphim-ai")

# 2. Extract from BUILD.md
BUILD_MD="/data/projects/terraphim/terraphim-ai/BUILD.md"
if [ -f "$BUILD_MD" ]; then
    COMMANDS=$(cat "$BUILD_MD" | /home/alex/.local/bin/claude --model haiku -p "Extract shell commands")
fi

# 3. Validate and execute
# ... (whitelist check)
# ... (rch exec)
```

### Step 3: Add cost tracking
**Files:** `crates/terraphim_orchestrator/src/config.rs`
**Description:** Track LLM costs per build, alert if over threshold
**Tests:** Integration tests for cost tracking
**Dependencies:** Step 2
**Estimated:** 2 hours

### Step 4: Feature flag and deployment
**Files:** `crates/terraphim_orchestrator/src/config.rs`, `terraphim.toml`
**Description:** Add `BUILD_RUNNER_LLM_ENABLED` feature flag
**Tests:** Verify fallback when flag is disabled
**Dependencies:** Step 3
**Estimated:** 2 hours

### Step 5: Documentation and handover
**Files:** `.docs/BUILD.md`, `README.md`
**Description:** Document new build process for contributors
**Tests:** None
**Dependencies:** Step 4
**Estimated:** 1 hour

## Rollback Plan

If issues discovered:
1. Disable `BUILD_RUNNER_LLM_ENABLED` flag
2. Existing `build-runner` agent continues working (deterministic)
3. Remove `build-runner-llm` from `agents_on_pr_open` if causing issues

Feature flag: `BUILD_RUNNER_LLM_ENABLED=false`

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Command extraction latency | < 30s | Timer around LLM call |
| Build latency | < 5 min | Total time from push to status |
| Cost per build | < $0.01 | LLM API cost tracking |
| Fallback success rate | > 99% | When LLM fails |

### Benchmarks to Add
```rust
#[bench]
fn bench_extract_commands(b: &mut Bencher) {
    let build_md = include_str!("../../../BUILD.md");
    b.iter(|| extract_from_markdown(build_md));
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| haiku extraction accuracy test | Pending | @alex |
| terraphim-agent learning format verification | Pending | @alex |
| BUILD.md creation for terraphim-ai | Pending | @alex |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
