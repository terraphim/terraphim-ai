# Build Ontology Spike: Semantic Build Actions

## Concept
Instead of extracting raw shell commands from markdown, define a **build ontology** where:
- Each build action is a typed concept in the knowledge graph
- Commands are mapped to semantic actions (not just strings)
- The LLM extracts **semantic intent**, not just syntax
- terraphim-automata matches intents to known command patterns

## Ontology Structure

```turtle
# Example terraphim build ontology
@prefix build: <https://terraphim.ai/ontology/build/> .
@prefix cmd: <https://terraphim.ai/ontology/command/> .

# Semantic action types
build:Format a build:BuildAction ;
    build:purpose "Format source code" ;
    build:typicalCommand "cargo fmt --all -- --check" ;
    build:category build:QualityGate ;
    build:cost "low" .

build:Lint a build:BuildAction ;
    build:purpose "Static analysis" ;
    build:typicalCommand "cargo clippy --workspace --all-targets -- -D warnings" ;
    build:category build:QualityGate ;
    build:cost "medium" .

build:Compile a build:BuildAction ;
    build:purpose "Build artifacts" ;
    build:typicalCommand "cargo build --workspace" ;
    build:category build:Compilation ;
    build:cost "high" .

build:Test a build:BuildAction ;
    build:purpose "Run test suite" ;
    build:typicalCommand "cargo test --workspace --no-fail-fast" ;
    build:category build:Verification ;
    build:cost "high" .

# Project-specific overrides
build:TerraphimBuildProfile a build:BuildProfile ;
    build:includes
        build:Format,
        build:Lint,
        build:Compile,
        build:Test ;
    build:usesToolchain "rustc 1.85" ;
    build:usesRemoteCompilation "rch" .
```

## Markdown Representation

```markdown
# Build Commands

## Format
build:: format
purpose:: Ensure consistent code formatting
toolchain:: cargo
command:: cargo fmt --all -- --check
cost:: low
category:: quality-gate

## Lint
build:: lint
purpose:: Static analysis and style checking
toolchain:: cargo
command:: cargo clippy --workspace --all-targets -- -D warnings
cost:: medium
category:: quality-gate

## Build
build:: compile
purpose:: Compile all workspace crates
toolchain:: cargo
command:: cargo build --workspace
cost:: high
category:: compilation

## Test
build:: test
purpose:: Run full test suite
toolchain:: cargo
command:: cargo test --workspace --no-fail-fast
cost:: high
category:: verification
```

## Advantages Over Raw Command Extraction

1. **Semantic validation**: The LLM extracts `build:: test` (intent) not `cargo test...` (syntax)
2. **Command resolution**: terraphim-agent resolves intent to actual command based on project context
3. **Learning integration**: Learnings capture semantic actions, not just shell strings
4. **Cross-project reuse**: `build:: format` means the same thing across Rust, Python, JS projects
5. **Cost optimization**: Can skip expensive actions (test, compile) if only formatting changed
6. **Intelligent ordering**: The ontology defines dependencies (format before lint, compile before test)

## terraphim-automata Integration

```rust
// terraphim_automata/src/build_ontology.rs

#[derive(Debug, Clone)]
pub struct BuildAction {
    pub action_type: BuildActionType,  // format, lint, compile, test, etc.
    pub purpose: String,
    pub toolchain: String,             // cargo, npm, make, etc.
    pub command_template: String,      // "cargo fmt --all -- --check"
    pub cost: BuildCost,              // low, medium, high
    pub category: BuildCategory,      // quality-gate, compilation, verification
}

#[derive(Debug, Clone)]
pub enum BuildActionType {
    Format,
    Lint,
    Compile,
    Test,
    Doc,
    Audit,
    Custom(String),
}

impl BuildAction {
    /// Resolve command template to actual command based on project context
    pub fn resolve(&self, ctx: &BuildContext) -> String {
        match self.action_type {
            BuildActionType::Format => {
                if ctx.has_workspace {
                    "cargo fmt --all -- --check".to_string()
                } else {
                    "cargo fmt -- --check".to_string()
                }
            }
            // ... etc
        }
    }
}
```

## LLM Extraction Prompt

```
You are a build ontology extractor. Read the provided BUILD.md and extract
semantic build actions using this ontology:

Valid action types: format, lint, compile, test, doc, audit
Valid categories: quality-gate, compilation, verification, documentation, security
Valid costs: low, medium, high

For each build step found, output:
build:: <action_type>
purpose:: <description>
toolchain:: <cargo|npm|make|...>
command:: <shell command>
cost:: <low|medium|high>
category:: <category>

Rules:
- Only extract build/test/lint commands, not install/setup
- Map similar concepts to standard types (e.g., "check formatting" → format)
- Infer cost based on typical execution time
- Use "custom" action type if no standard type matches
```

## terraphim-agent Learning Integration

```bash
# Capture successful build sequences as semantic learnings
~/.cargo/bin/terraphim-agent learn capture \
  "build:: format, build:: lint, build:: compile, build:: test" \
  --project terraphim-ai \
  --exit-code 0 \
  --metadata '{"sha": "abc123", "duration_secs": 180}'

# Query for known build sequences
~/.cargo/bin/terraphim-agent learn query \
  "build sequence terraphim-ai" \
  --format ontology

# Returns:
# build:: format > build:: lint > build:: compile > build:: test
# Success rate: 98.5% (last 100 runs)
# Average duration: 185s
```

## Cost-Aware Execution

```rust
// Skip expensive actions when only docs changed
if changed_files.iter().all(|f| f.ends_with(".md")) {
    actions.retain(|a| a.cost != BuildCost::High);
}

// Parallelize independent actions
let quality_gate: Vec<_> = actions
    .iter()
    .filter(|a| a.category == BuildCategory::QualityGate)
    .collect();
// Run format + lint in parallel

// Sequential compilation > test (test depends on compile)
let compilation = actions.iter().find(|a| a.category == BuildCategory::Compilation);
let verification = actions.iter().find(|a| a.category == BuildCategory::Verification);
```

## Implementation Plan

1. **Define ontology schema** in terraphim_types
2. **Extend markdown parser** to extract `build::` directives with full metadata
3. **Create command resolver** that maps semantic actions to project-specific commands
4. **Update build-runner-llm** to extract ontology, not just shell strings
5. **Enhance terraphim-agent learning** to capture semantic build sequences

## File Changes

| File | Purpose |
|------|---------|
| `crates/terraphim_types/src/build_ontology.rs` | Ontology types |
| `crates/terraphim_automata/src/build_directives.rs` | Markdown parser extension |
| `crates/terraphim_automata/src/command_resolver.rs` | Semantic → command mapping |
| `scripts/build-runner-llm.sh` | Updated to use ontology extraction |
| `BUILD.md` | Semantic build documentation |

## Example Build Output

```
[build-runner-llm] Extracting build ontology from learnings + BUILD.md...
[build-runner-llm] Found 4 semantic actions:
  1. build:: format [quality-gate, low cost]
  2. build:: lint [quality-gate, medium cost]
  3. build:: compile [compilation, high cost]
  4. build:: test [verification, high cost]
[build-runner-llm] Changed files: 3 .rs files in crates/terraphim_orchestrator
[build-runner-llm] Optimizing: skipping doc generation (no .md changes)
[build-runner-llm] Executing via rch:
  → cargo fmt --all -- --check
  → cargo clippy --workspace --all-targets -- -D warnings
  → cargo build --workspace
  → cargo test --workspace --no-fail-fast
[build-runner-llm] All 4 actions passed (182s total)
[build-runner-llm] Updating learnings with successful sequence
```
