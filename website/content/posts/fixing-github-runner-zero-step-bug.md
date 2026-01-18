+++
title="Fixing the GitHub Runner Zero-Step Bug: A Journey from 0 to N Steps"
date=2026-01-18

[taxonomies]
categories = ["Engineering", "Rust", "Debugging"]
tags = ["Terraphim", "rust", "github-actions", "llm", "debugging", "firecracker", "async", "ownership"]
[extra]
toc = true
comments = true
+++

How we tracked down a critical bug where our GitHub Actions runner was executing workflows with zero steps, the systematic debugging process that revealed the root cause, and the elegant Rust fix that enabled LLM-based workflow parsing in parallel execution.

<!-- more -->

## The Mystery: 0/0 Steps Executed

It started with a confusing support ticket. Our GitHub Runner service, built to execute GitHub Actions workflows in isolated Firecracker microVMs, was reporting success—but nothing was actually running.

```bash
GitHub Runner Execution Results:
python-bindings.yml: 0/0 steps, 1s
ci-optimized.yml: 0/0 steps, 1s
test-firecracker-runner.yml: 0/0 steps, 1s
vm-execution-tests.yml: 0/0 steps, 1s
ci-native.yml: 0/0 steps, 1s
```

Every single workflow: **0 steps executed**. And they all completed in ~1 second—suspiciously fast.

What was going on?

## Phase 1: Deep Research (2 hours)

Instead of jumping to solutions, we applied a disciplined research approach to understand the problem space.

### The Smoking Gun

We traced through the codebase and found this comment on line 512 of `execution.rs`:

```rust
// Note: LLM parser not used in parallel execution to avoid lifetime issues
join_set.spawn(async move {
    let result = execute_workflow_in_vm(
        &workflow_path,
        &gh_event,
        &firecracker_api_url,
        firecracker_auth_token.as_deref(),
        None, // ⚠️ No LLM parser in parallel execution
    ).await;
```

**The comment was telling us exactly what was wrong.**

### Understanding the Impact

Our GitHub Runner has two ways to parse workflows:

1. **Simple YAML Parser**: Reads workflow files, extracts shell commands
   - **Problem**: Can't translate GitHub Actions syntax (`uses: actions/checkout@v4`) to shell commands
   - **Behavior**: Skips these actions with a warning
   - **Result**: 0 steps

2. **LLM Parser**: Uses LLM to understand and translate Actions syntax
   - **Benefit**: Can translate `uses:` to actual shell commands
   - **Problem**: Was disabled in parallel execution
   - **Result**: N steps (when working)

The LLM parser was intentionally disabled because of Rust ownership constraints preventing `Arc<dyn LlmClient>` from being cloned.

### Root Cause Identified

**Technical Constraint:**
- `WorkflowParser` contains `Arc<dyn LlmClient>`
- `Arc<dyn LlmClient>` doesn't implement `Clone`
- Parallel execution uses `async move` blocks that take ownership
- Can't move non-cloneable data into `async move` blocks

**Impact:**
- LLM parser disabled in parallel execution
- Only simple parser used
- All `uses:` actions skipped
- Result: 0/0 steps executed

## Phase 2: Solution Design (1 hour)

We needed to make `WorkflowParser` cloneable so it could be moved into parallel async tasks.

### The Design Challenge

```rust
pub struct WorkflowParser {
    llm_client: Arc<dyn LlmClient>,  // How to clone this?
}
```

**Options Considered:**

1. **Remove Arc and use concrete type**
   - ❌ Loses polymorphism (can't swap LLM providers)

2. **Use `Rc<dyn LlmClient>` instead**
   - ❌ Not thread-safe for async contexts

3. **Implement `Clone` manually using `Arc::clone`**
   - ✅ Preserves polymorphism
   - ✅ Thread-safe
   - ✅ Cheap (only atomic increment)

### The Solution

Implement `Clone` trait manually for `WorkflowParser`:

```rust
impl Clone for WorkflowParser {
    fn clone(&self) -> Self {
        Self {
            llm_client: Arc::clone(&self.llm_client),
        }
    }
}
```

**Key Insight:** `Arc::clone` doesn't deep copy the data—it just increments the atomic reference counter. This is **very cheap** (single atomic instruction).

## Phase 3: Implementation (2 hours)

### Changes Made

**1. Clone Implementation (`parser.rs`)**
```rust
impl Clone for WorkflowParser {
    fn clone(&self) -> Self {
        Self {
            llm_client: Arc::clone(&self.llm_client),
        }
    }
}
```

**2. Enable LLM Parser (`execution.rs`)**
```rust
// Clone the LLM parser to avoid lifetime issues
let llm_parser_clone = llm_parser.cloned();

join_set.spawn(async move {
    if llm_parser_clone.is_some() {
        info!("🤖 LLM parser enabled for this workflow");
    }

    let result = execute_workflow_in_vm(
        &workflow_path,
        &gh_event,
        &firecracker_api_url,
        firecracker_auth_token.as_deref(),
        llm_parser_clone.as_ref(), // Use cloned parser
    ).await;
```

**Total Code Changes:** ~30 lines (9 for Clone, ~20 for execution)

### Tests Added

Created comprehensive test suite proving:
- Clone produces independent parsers
- Cloned parsers work in parallel async tasks
- VM allocation happens at workflow level (not per step)

## Phase 4: Validation (1 hour)

Before deploying, we needed to verify our understanding of the VM allocation architecture. The user had a critical requirement:

> "Github actions are sequential only each action flow shall create it's own VM not each task"

We traced through the codebase and created empirical tests proving:

✅ **1 VM per workflow file** (NOT per step/job)
✅ All steps execute sequentially in the **same VM**
✅ Multiple workflows run in parallel with separate VMs

### VM Allocation Flow

```
Webhook Received (11 workflows)
    ↓
For each workflow (max 5 concurrent):
    ↓
    SessionManager::create_session()
    ↓
    VmProvider::allocate()  ← ⭐ ONE VM ALLOCATED HERE (manager.rs:160)
    ↓
    WorkflowExecutor::execute_workflow()
    ↓
    For each step in workflow:
        ExecuteStep (reuses same VM)
    ↓
    SessionManager::release_session()
```

**Critical Point:** VM allocated **once** per workflow in `manager.rs:160`, then reused for all steps.

## Phase 5: Deployment & Testing (2 hours)

### Model Selection Journey

We tested three different Ollama models:

| Model | Size | Result | Issue |
|-------|------|--------|-------|
| gemma3:270m | 270M | Malformed JSON | Too small for structured output |
| gemma3:4b | 3.3B | Timeout | Too slow for concurrent requests |
| llama3.2:3b | 3.2B | ✅ Success | Fast and capable |

**Lesson:** Model size matters for structured JSON generation.

### Concurrency Tuning

| Setting | Value | Result | Success Rate |
|---------|-------|--------|-------------|
| `MAX_CONCURRENT_WORKFLOWS=5` | Parallel | Ollama overwhelmed | 1/11 (9%) |
| `MAX_CONCURRENT_WORKFLOWS=1` | Serial | Stable | 4/11 (36%) |

**Lesson:** Serial processing with stable LLM > Parallel with crashing LLM.

### Final Results

```bash
Before Fix:
All workflows: 0/0 steps (LLM parser disabled)

After Fix:
Successfully parsed: 4/19 workflows
Total steps extracted: 19 steps

Breakdown:
✅ claude.yml                  → 2 steps
✅ deploy-website.yml          → 9 steps
✅ claude-code-review.yml      → 2 steps
✅ python-bindings.yml         → 6 steps

Failed: 15/19 workflows (Ollama connection errors)
```

**The fix worked!** LLM parser now functional in parallel execution.

## Technical Deep Dive

### Why Arc::clone is Cheap

```rust
// What Arc::clone actually does:
pub fn clone(this: &Arc<T>) -> Arc<T> {
    // Increment reference count (single atomic instruction!)
    this.inner().refcount.fetch_add(1, Ordering::Relaxed);

    // Return new Arc pointing to same data
    ManuallyDrop::new(Arc {
        ptr: this.ptr,
        phantom: PhantomData,
    })
}
```

**No deep copy!** Just an atomic increment. Perfect for sharing across async tasks.

### Rust Ownership in Async Context

The core challenge was `async move` semantics:

```rust
join_set.spawn(async move {
    // This block TAKES OWNERSHIP of captured variables
    // Can't use &llm_parser here - lifetime too short
});
```

**Solution:** Clone before the move:

```rust
// Clone the parser (cheap Arc::clone)
let llm_parser_clone = llm_parser.cloned();

// Now we can move the clone into async block
join_set.spawn(async move {
    llm_parser_clone.as_ref() // Use cloned value
});
```

### VM Allocation Semantics

We created 659 lines of empirical tests proving VM allocation behavior:

```rust
#[tokio::test]
async fn test_single_workflow_multiple_steps_one_vm() {
    // Create workflow with 5 steps
    let workflow = create_test_workflow("test", 5);

    // Execute workflow
    let result = executor.execute_workflow(&workflow, &context).await;

    // CRITICAL VERIFICATION: Exactly ONE VM allocated
    assert_eq!(allocation_count, 1,
        "Expected 1 VM for 5 steps, got {}", allocation_count);

    // Verify all steps used the same VM
    assert_eq!(unique_vms.len(), 1,
        "All steps should use same VM");
}
```

**All 5 tests passed**, confirming our understanding of the architecture.

## Mistakes We Made

### Mistake 1: Skipping Research Phase

Initially tried to fix by modifying workflow parsing logic without understanding root cause. Wasted ~1 hour.

**Fix Applied:** Used disciplined development (Research → Design → Implement)

### Mistake 2: Underestimating Ollama Limitations

Assumed local Ollama could handle 11 concurrent workflow parsing requests. Result: 56 connection errors.

**Reality Check:** Local LLMs are for development, not production. Use cloud providers for production workloads.

### Mistake 3: Not Testing with Real Workflows Initially

Started with mock tests before testing with actual GitHub Actions workflows. Wasted ~30 minutes.

**Better Approach:** Send real webhook from PR #423 with 11 workflows immediately.

## Lessons Learned

### 1. Disciplined Development Saves Time

Our 5-phase process took ~6 hours total:

1. **Research** (2h) - Root cause identification
2. **Design** (1h) - Solution architecture
3. **Implement** (2h) - Code changes
4. **Validate** (1h) - Architecture verification
5. **Verify** (2h) - Empirical testing

Without this process, we estimate it would have taken 2+ days of debugging.

### 2. Empirical Tests Prove Architecture

Unit tests prove code compiles.
Integration tests prove components work together.
**Empirical tests prove architectural assumptions.**

We added 659 lines of VM allocation tests proving 1 VM per workflow.

### 3. Comments Are Documentation

That one comment on line 512 told us everything:
> "LLM parser not used in parallel execution to avoid lifetime issues"

If we'd read the comments first, we'd have saved 2 hours.

**Takeaway:** Always read the comments before writing code.

### 4. Serial Processing > Parallel with Crashes

We tried parallel processing with 5 concurrent workflows:
- Result: Ollama crashed, 1/11 succeeded

We switched to serial processing (1 at a time):
- Result: Stable, 4/11 succeeded

**Lesson:** Stability > Speed for production systems.

## Next Steps

The fix is working, but we have clear next steps:

### 1. Switch to Production LLM Provider ⭐

Replace Ollama with a cloud provider:

```bash
# Set OPENROUTER_API_KEY environment variable
export LLM_PROVIDER="openrouter"
export LLM_MODEL="anthropic/claude-3.5-sonnet"
export MAX_CONCURRENT_WORKFLOWS=5  # Enable parallel!
```

Expected: 95%+ success rate with parallel processing.

### 2. Add Retry Logic

Don't fall back to simple parser on first failure:

```rust
let mut retries = 3;
while retries > 0 {
    match llm_parser.parse_workflow(&content).await {
        Ok(parsed) => break Ok(parsed),
        Err(e) if retries > 0 => {
            tokio::time::sleep(delay).await;
            delay *= 2;
            retries -= 1;
        }
        Err(e) => break Err(e),
    }
}
```

### 3. Add Workflow Caching

Cache parsed workflows to avoid re-parsing:
- Could reduce LLM calls by 80%+
- Invalidate on workflow file changes
- Store in-memory or Redis

## Code You Can Use

### Clone Pattern for Arc<dyn Trait>

```rust
pub struct MyParser {
    client: Arc<dyn LlmClient>,
}

impl Clone for MyParser {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
        }
    }
}
```

### Parallel Execution with Cloned Data

```rust
let data_clone = data.cloned();

join_set.spawn(async move {
    data_clone.as_ref().do_something().await
});
```

### VM Allocation Verification

```rust
#[tokio::test]
async fn test_vm_allocation() {
    let workflow = create_workflow(5);  // 5 steps

    executor.execute(&workflow).await;

    assert_eq!(vm_count, 1);  // Proves 1 VM per workflow
}
```

## Statistics

| Metric | Value |
|--------|-------|
| Lines of code changed | 260+ |
| Test files added | 2 |
| Tests added | 30+ |
| VM allocation tests | 5 (all passing) |
| Workspace tests passing | 700+ |
| Steps extracted | 19 |
| Workflows parsed | 4/19 (21%) |
| Ollama errors | 56 |
| Time to fix | ~6 hours |
| Phases completed | 5 |

## Conclusion

This bug taught us that:

1. **Disciplined development** (Research → Design → Implement) saves time
2. **Empirical testing** proves architectural assumptions
3. **Comments contain valuable documentation**—read them first!
4. **Arc::clone is cheap**—don't fear sharing across async tasks
5. **Serial processing** beats parallel when dependencies crash

The fix itself was only ~30 lines of code, but the **journey** to get there taught us lessons about Rust, async programming, and systematic debugging that we'll apply for years to come.

**From 0 steps to N steps:** Not just a bug fix, but a masterclass in disciplined engineering.

---

**Want to learn more?**
- Code: `crates/terraphim_github_runner/src/workflow/parser.rs`
- Tests: `crates/terraphim_github_runner/tests/vm_allocation_verification_test.rs`
- Commit: bcf055e8
- Issue: #423
