# Build Ontology vs Existing `action::` Directive

## Current `action::` Usage

Looking at `crates/terraphim_automata/src/markdown_directives.rs`, `action::` is already used for **routing actions**:

```markdown
# Implementation Tier
route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50
```

Here `action::` defines the **CLI invocation template** for a routing decision. It's about "how to invoke the LLM", not "what commands to run".

## Semantic Difference

| Directive | Context | Purpose | Example |
|-----------|---------|---------|---------|
| `action::` | Routing | "How to invoke the LLM for this tier" | `opencode run -m {{ model }}` |
| `build::` | Build | "What build step to execute" | `cargo fmt --all` |

**Key distinction:**
- `action::` is **meta** - it tells the orchestrator how to spawn agents
- `build::` is **operational** - it tells the build what commands to run

## Can We Reuse `action::`?

**Option 1: Reuse `action::` with context**
```markdown
# Build Commands
action:: cargo fmt --all -- --check
type:: build
step:: format
cost:: low

action:: cargo clippy --workspace --all-targets -- -D warnings
type:: build
step:: lint
cost:: medium
```

**Pros:** No new directive needed
**Cons:** Ambiguity - parser can't distinguish routing actions from build actions without the `type::` modifier. Makes the routing KG noisy.

**Option 2: Use `action::` in a separate namespace**
Build commands live in `BUILD.md` (not routing scenarios), so the parser context is different. When parsing `BUILD.md`, `action::` means "shell command". When parsing `routing_scenarios/`, `action::` means "LLM invocation".

**Pros:** Reuses syntax, contextually disambiguated
**Cons:** Confusing for humans, error-prone

**Option 3: Use `build::` (recommended)**
```markdown
# Build Commands
build:: format
action:: cargo fmt --all -- --check
cost:: low

build:: lint
action:: cargo clippy --workspace --all-targets -- -D warnings
cost:: medium
```

**Pros:**
- Clear semantic separation from routing
- Self-documenting - `build::` obviously means "build step"
- Can extend with build-specific metadata (cost, category, toolchain)
- No ambiguity in the knowledge graph

**Cons:**
- New directive to implement (but trivial - same parser pattern as `action::`)

## Recommendation

**Use `build::` as a first-class directive**, but leverage the same parser infrastructure:

```rust
// In terraphim_automata/src/markdown_directives.rs

// Existing directives
pub enum Directive {
    Route(RouteDirective),
    Action(String),       // LLM invocation template
    Trigger(Vec<String>),
    // ... existing directives

    // NEW: Build directive
    Build(BuildDirective),
}

pub struct BuildDirective {
    pub action_type: BuildActionType,  // format, lint, compile, test
    pub command: String,               // cargo fmt --all
    pub cost: BuildCost,
    pub category: BuildCategory,
}
```

The parser already handles `directive:: value` syntax. Adding `build::` is a ~10 line change following the same pattern as `action::`.

## Why Not Overload `action::`?

1. **Knowledge graph pollution**: Routing scenarios and build commands would mix in the same namespace
2. **Parser complexity**: Need `type:: build` modifier everywhere to disambiguate
3. **Human confusion**: `action::` already has well-established meaning in the routing tier
4. **Future extensibility**: `build::` can grow (add `test::`, `deploy::`, etc.) without breaking routing

## Updated Design

```markdown
# BUILD.md - Semantic Build Documentation

## Format
build:: format
action:: cargo fmt --all -- --check
cost:: low
category:: quality-gate
toolchain:: cargo

## Lint
build:: lint
action:: cargo clippy --workspace --all-targets -- -D warnings
cost:: medium
category:: quality-gate
toolchain:: cargo

## Build
build:: compile
action:: cargo build --workspace
cost:: high
category:: compilation
toolchain:: cargo

## Test
build:: test
action:: cargo test --workspace --no-fail-fast
cost:: high
category:: verification
toolchain:: cargo
```

**Note:** `action::` still appears, but now it means "the shell action to execute for this build step" - which is actually semantically consistent! The difference is:
- In routing: `action::` = "how to invoke the agent"
- In build: `action::` = "what shell command to run"

Both are "actions", just in different contexts. The `build::` directive provides the semantic typing.
