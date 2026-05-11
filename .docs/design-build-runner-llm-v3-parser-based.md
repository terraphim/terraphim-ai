# Build Ontology via Existing Parsers: A Better Approach

## Reconsidering `build::` vs `action::`

**Previous assumption:** We need a new `build::` directive because build commands are semantically different from routing actions.

**New insight:** Build commands ARE actions - they're just actions in a different context. The `action::` directive is already the right abstraction.

```markdown
# Instead of inventing build::
build:: format
action:: cargo fmt --all

# Use action:: with context metadata
action:: cargo fmt --all -- --check
context:: build
step:: format
cost:: low
```

## The Parser Problem

**Previous approach:** Parse markdown directives to extract build commands.

**Problem:** Markdown is human-written, inconsistent, and requires LLM interpretation.

**Better approach:** Projects ALREADY define their builds in machine-readable formats:
- `.github/workflows/*.yml` - GitHub Actions
- `Dockerfile` - Docker builds
- `Earthfile` - Earthly builds
- `package.json` - npm scripts
- `Makefile` - Make targets
- `Cargo.toml` - Rust workspace
- `docker-compose.yml` - Service orchestration

## Using Existing Parsers

### 1. GitHub Actions Parser (Tree-sitter YAML)

```rust
use tree_sitter_yaml;

pub fn extract_from_github_actions(workflow_file: &Path) -> Vec<BuildAction> {
    let content = fs::read_to_string(workflow_file).unwrap();
    let parser = tree_sitter::Parser::new();
    let tree = parser.parse(&content, None).unwrap();

    // Extract jobs and steps
    // job.steps[].run contains the actual commands
    // job.steps[].uses contains reusable actions

    let mut actions = Vec::new();

    // Navigate YAML AST
    let root = tree.root_node();
    for job in find_jobs(root) {
        for step in find_steps(job) {
            if let Some(run_cmd) = step.get_field("run") {
                actions.push(BuildAction {
                    action_type: infer_action_type(&run_cmd),
                    command: run_cmd.to_string(),
                    source: ActionSource::GitHubActions,
                    context: ActionContext::Build,
                });
            }
        }
    }

    actions
}
```

**Example extraction from `.github/workflows/ci-pr.yml`:**
```yaml
jobs:
  build:
    steps:
      - run: cargo fmt --all -- --check        → action:: format
      - run: cargo clippy --workspace ...       → action:: lint
      - run: cargo build --workspace            → action:: compile
      - run: cargo test --workspace --no-fail-fast → action:: test
```

### 2. Earthfile Parser

```rust
use earthfile_parser; // Or regex-based parser

pub fn extract_from_earthfile(earthfile: &Path) -> Vec<BuildAction> {
    let content = fs::read_to_string(earthfile).unwrap();

    // Earthfile targets map to build steps
    // +build → compile
    // +test → test
    // +lint → lint
    // +fmt → format

    let target_regex = regex::Regex::new(r"^([a-zA-Z_-]+):\s*$").unwrap();
    let run_regex = regex::Regex::new(r"^\s*RUN\s+(.+)$").unwrap();

    let mut actions = Vec::new();
    let mut current_target = None;

    for line in content.lines() {
        if let Some(cap) = target_regex.captures(line) {
            current_target = Some(cap[1].to_string());
        }

        if let Some(cap) = run_regex.captures(line) {
            if let Some(target) = &current_target {
                actions.push(BuildAction {
                    action_type: map_earthly_target(target),
                    command: cap[1].to_string(),
                    source: ActionSource::Earthfile,
                    context: ActionContext::Build,
                });
            }
        }
    }

    actions
}
```

### 3. Dockerfile Parser

```rust
use dockerfile_parser;

pub fn extract_from_dockerfile(dockerfile: &Path) -> Vec<BuildAction> {
    let content = fs::read_to_string(dockerfile).unwrap();
    let dockerfile = Dockerfile::parse(&content).unwrap();

    let mut actions = Vec::new();

    for instruction in dockerfile.instructions {
        if let Instruction::Run(cmd) = instruction {
            actions.push(BuildAction {
                action_type: infer_from_command(&cmd),
                command: cmd.to_string(),
                source: ActionSource::Dockerfile,
                context: ActionContext::Build,
            });
        }
    }

    actions
}
```

### 4. Makefile Parser

```rust
pub fn extract_from_makefile(makefile: &Path) -> Vec<BuildAction> {
    let content = fs::read_to_string(makefile).unwrap();

    // Targets become build steps
    // build: → compile
    // test: → test
    // lint: → lint
    // fmt: → format

    let target_regex = regex::Regex::new(r"^([a-zA-Z_-]+):\s*(.*)$").unwrap();

    let mut actions = Vec::new();

    for line in content.lines() {
        if let Some(cap) = target_regex.captures(line) {
            let target = &cap[1];
            let deps = &cap[2];

            // Look up recipe body
            // ... (parse recipe commands)

            actions.push(BuildAction {
                action_type: map_make_target(target),
                command: format!("make {}", target),
                source: ActionSource::Makefile,
                context: ActionContext::Build,
                dependencies: deps.split_whitespace().map(|s| s.to_string()).collect(),
            });
        }
    }

    actions
}
```

### 5. Cargo.toml Parser

```rust
use cargo_toml;

pub fn extract_from_cargo_toml(manifest: &Path) -> Vec<BuildAction> {
    let manifest = Manifest::from_path(manifest).unwrap();

    let mut actions = Vec::new();

    // Workspace members imply build steps
    if manifest.workspace.is_some() {
        actions.push(BuildAction {
            action_type: ActionType::Compile,
            command: "cargo build --workspace".to_string(),
            source: ActionSource::CargoManifest,
            context: ActionContext::Build,
        });

        actions.push(BuildAction {
            action_type: ActionType::Test,
            command: "cargo test --workspace --no-fail-fast".to_string(),
            source: ActionSource::CargoManifest,
            context: ActionContext::Build,
        });
    }

    // Custom targets from [workspace.metadata.build]
    if let Some(metadata) = manifest.workspace.as_ref().and_then(|w| w.metadata.as_ref()) {
        if let Some(build) = metadata.get("build") {
            // Extract custom build steps
        }
    }

    actions
}
```

## Multi-Parser Strategy

```rust
/// Auto-detect build system and extract actions
pub fn auto_extract_build_actions(project_dir: &Path) -> Vec<BuildAction> {
    let mut actions = Vec::new();

    // Try parsers in order of preference
    let parsers: Vec<(&str, Box<dyn Fn(&Path) -> Vec<BuildAction>>> = vec![
        (".github/workflows/ci-pr.yml", Box::new(extract_from_github_actions)),
        (".github/workflows/ci.yml", Box::new(extract_from_github_actions)),
        ("Earthfile", Box::new(extract_from_earthfile)),
        ("Dockerfile", Box::new(extract_from_dockerfile)),
        ("Makefile", Box::new(extract_from_makefile)),
        ("package.json", Box::new(extract_from_package_json)),
        ("Cargo.toml", Box::new(extract_from_cargo_toml)),
    ];

    for (filename, parser) in parsers {
        let path = project_dir.join(filename);
        if path.exists() {
            println!("Found build definition: {}", filename);
            actions.extend(parser(&path));
            break; // Use first match
        }
    }

    if actions.is_empty() {
        // Fall back to LLM extraction from BUILD.md
        println!("No build system detected, falling back to LLM extraction");
        actions = extract_from_build_md(project_dir);
    }

    actions
}
```

## KG Storage Format

```turtle
@prefix action: <https://terraphim.ai/ontology/action/> .
@prefix project: <https://terraphim.ai/projects/> .

# Project with multiple build sources
project:terraphim-ai a action:Project ;
    action:fingerprint "sha256:abc123..." ;
    action:hasBuildSource
        project:terraphim-ai-github-actions,
        project:terraphim-ai-earthfile .

# GitHub Actions source
project:terraphim-ai-github-actions a action:BuildSource ;
    action:sourceType "github-actions" ;
    action:sourceFile ".github/workflows/ci-pr.yml" ;
    action:extractedAt "2026-05-11T14:00:00Z" ;
    action:providesStep
        action:terraphim-ai-step-1,
        action:terraphim-ai-step-2,
        action:terraphim-ai-step-3,
        action:terraphim-ai-step-4 .

# Build steps (unified across all sources)
action:terraphim-ai-step-1 a action:BuildAction ;
    action:actionType action:Format ;
    action:command "cargo fmt --all -- --check" ;
    action:source action:GitHubActions ;
    action:context action:Build ;
    action:cost action:Low ;
    action:estimatedDuration 30 .
```

## Advantages of Parser-Based Approach

1. **No markdown parsing** - Use real build definitions
2. **No LLM on first run** - If GitHub Actions exist, parse them directly
3. **Multiple sources** - Can merge from GitHub Actions + Earthfile + Makefile
4. **Deterministic extraction** - Parsers are deterministic, not LLM-based
5. **Standard formats** - No need to invent BUILD.md format
6. **Existing tooling** - Tree-sitter, dockerfile-parser, etc. are battle-tested

## Updated Design

### BUILD.md is Optional

```markdown
# Build Documentation (Optional)

This file is optional. The build-runner will auto-detect your build system:

1. `.github/workflows/*.yml` - GitHub Actions (preferred)
2. `Earthfile` - Earthly builds
3. `Dockerfile` - Docker builds
4. `Makefile` - Make targets
5. `package.json` - npm scripts
6. `Cargo.toml` - Rust workspace

If none found, falls back to BUILD.md or LLM extraction.

## Custom Steps
action:: ./scripts/custom-check.sh
context:: build
step:: custom-check
```

### terraphim.toml Agent Config

```toml
[[agents]]
name = "build-runner-llm"
layer = "Growth"
cli_tool = "/bin/bash"
model = "haiku"  # Only for LLM fallback
max_cpu_seconds = 1800
event_only = true
project = "terraphim-ai"
capabilities = ["build", "test", "auto-detect-ci"]

# Build auto-detection
[build_detection]
# Priority order for build system detection
detectors = [
    "github-actions",
    "earthfile",
    "dockerfile",
    "makefile",
    "npm",
    "cargo",
]

# LLM fallback only if no build system detected
llm_fallback = true

# terraphim KG caching
kg_role = "DevOps Engineer"
cache_invalidation_hours = 24
```

## terraphim-agent Suggest Integration

### Fuzzy Matching Build Commands

The `terraphim-agent suggest` command provides fuzzy matching that can resolve ambiguous build commands:

```bash
# User types ambiguous command
$ terraphim-agent suggest "fmt" --role "DevOps Engineer" --fuzzy
> format
> fmt-check
> rustfmt

# Match against known build actions
$ terraphim-agent suggest "test" --role "DevOps Engineer" --threshold 0.8
> test
> test-all
> test-integration
> cargo-test
```

### Use Cases in Build-Runner

1. **Resolving ambiguous step names**
   ```yaml
   # GitHub Actions might use:
   - run: cargo check
   # User query: "check" → suggest maps to "lint"
   ```

2. **Cross-project command discovery**
   ```bash
   # Find similar commands across projects
   $ terraphim-agent suggest "build" --role "DevOps Engineer"
   > cargo build --workspace      (terraphim-ai)
   > make all                     (gitea-robot)
   > npm run build                (atomic-server)
   ```

3. **Typo correction in BUILD.md**
   ```markdown
   # User writes:
   action:: cargo formt --all

   # Parser detects typo, suggests:
   > Did you mean: "cargo fmt --all"?
   ```

### Integration with Parsers

```rust
use terraphim_automata::suggest::FuzzyMatcher;

pub fn resolve_build_action(
    raw_command: &str,
    known_actions: &[BuildAction],
) -> Option<BuildAction> {
    // First try exact match
    if let Some(exact) = known_actions.iter().find(|a| a.command == raw_command) {
        return Some(exact.clone());
    }

    // Fall back to fuzzy matching via terraphim-agent
    let matcher = FuzzyMatcher::new()
        .with_threshold(0.8);

    let suggestions: Vec<String> = known_actions
        .iter()
        .map(|a| a.command.clone())
        .collect();

    matcher.suggest(raw_command, &suggestions)
        .first()
        .cloned()
        .and_then(|cmd| known_actions.iter().find(|a| a.command == cmd))
        .cloned()
}
```

## Implementation Plan (Revised)

### Phase 1: Parser Infrastructure (4h)
- Add `tree-sitter-yaml` dependency
- Create `crates/terraphim_automata/src/build_parsers/` module
- Implement GitHub Actions parser
- Implement Earthfile parser
- Implement Cargo.toml parser

### Phase 2: Auto-Detection (2h)
- Create `BuildDetector` that tries parsers in order
- Add `action::` with `context:: build` support to existing markdown parser
- Implement fallback chain: parser → BUILD.md → LLM

### Phase 3: KG Integration (2h)
- Store extracted actions in terraphim KG
- Fingerprint based on build file hashes
- Cache invalidation on file changes

### Phase 4: Build-Runner Agent (2h)
- Update agent script to use auto-detection
- Remove hardcoded cargo commands
- Test with real projects

**Total: 10 hours** (same as v2)

## Files to Change (Revised)

| File | Purpose |
|------|---------|
| `crates/terraphim_automata/src/build_parsers/mod.rs` | Parser infrastructure |
| `crates/terraphim_automata/src/build_parsers/github_actions.rs` | GitHub Actions parser |
| `crates/terraphim_automata/src/build_parsers/earthfile.rs` | Earthfile parser |
| `crates/terraphim_automata/src/build_parsers/cargo.rs` | Cargo.toml parser |
| `crates/terraphim_automata/src/markdown_directives.rs` | Add `context:: build` to action:: |
| `crates/terraphim_types/src/build_ontology.rs` | Unified build action types |
| `scripts/build-runner-llm.sh` | Auto-detection logic |

## Example: terraphim-ai Project

**Build system detected:** `.github/workflows/ci-pr.yml`

**Extracted actions:**
```rust
[
    BuildAction {
        action_type: Format,
        command: "cargo fmt --all -- --check",
        source: GitHubActions,
        context: Build,
        cost: Low,
    },
    BuildAction {
        action_type: Lint,
        command: "cargo clippy --workspace --all-targets -- -D warnings",
        source: GitHubActions,
        context: Build,
        cost: Medium,
    },
    BuildAction {
        action_type: Compile,
        command: "cargo build --workspace",
        source: GitHubActions,
        context: Build,
        cost: High,
    },
    BuildAction {
        action_type: Test,
        command: "cargo test --workspace --no-fail-fast",
        source: GitHubActions,
        context: Build,
        cost: High,
    },
]
```

**No BUILD.md needed!** The build-runner reads the existing CI config.

## Migration Path

```bash
# Current state: Hardcoded commands in build-runner
# → Detect GitHub Actions → Parse automatically → Cache in KG

# Day 1: Deploy parser-based build-runner
# No changes needed to project!
# Reads existing .github/workflows/ci-pr.yml

# Day 7: All projects benefit
# - Projects with GitHub Actions: Automatic parsing
# - Projects with Earthfile: Automatic parsing
# - Projects with Makefile: Automatic parsing
# - Projects with nothing: LLM fallback or manual BUILD.md
```
