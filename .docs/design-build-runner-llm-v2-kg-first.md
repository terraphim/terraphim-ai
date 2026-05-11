# Build-Runner-LLM v2: Knowledge-Graph-First Architecture

## Insight: Leverage terraphim's Core Competency

Instead of using LLM as a glorified markdown parser on every build, use terraphim's knowledge graph infrastructure:

1. **First run (cold start)**: Cheap LLM (haiku) reads project → creates KG entries
2. **Subsequent runs (hot path)**: terraphim-automata Aho-Corasick matching → deterministic execution
3. **Failure/unknown**: Fall back to LLM → update KG

## Architecture

```
Build Event
    │
    ├─► Check terraphim KG cache for project fingerprint
    │   │
    │   ├─► HIT: Load build sequence from KG
    │   │   ├─► Execute commands deterministically
    │   │   └─► On failure: Update KG with failure reason
    │   │
    │   └─► MISS: First run or cache invalidated
    │       ├─► haiku reads BUILD.md / project structure
    │       ├─► Extracts build sequence
    │       ├─► Stores in terraphim KG
    │       └─► Executes commands
    │
    └─► POST_STATUS
```

## Knowledge Graph Structure

```turtle
@prefix build: <https://terraphim.ai/ontology/build/> .
@prefix project: <https://terraphim.ai/projects/> .

# Project fingerprint (Cargo.lock hash + key file mtimes)
project:terraphim-ai a build:Project ;
    build:fingerprint "sha256:abc123..." ;
    build:buildFile "BUILD.md" ;
    build:extractedAt "2026-05-11T14:00:00Z" .

# Build sequence (ordered list)
project:terraphim-ai-build-sequence a build:BuildSequence ;
    build:project project:terraphim-ai ;
    build:steps (
        build:terraphim-ai-step-1
        build:terraphim-ai-step-2
        build:terraphim-ai-step-3
        build:terraphim-ai-step-4
    ) .

# Individual build steps
build:terraphim-ai-step-1 a build:BuildStep ;
    build:actionType build:Format ;
    build:command "cargo fmt --all -- --check" ;
    build:cost build:Low ;
    build:category build:QualityGate ;
    build:toolchain "cargo" ;
    build:estimatedDuration 30 ;
    build:successRate 0.99 .

build:terraphim-ai-step-2 a build:BuildStep ;
    build:actionType build:Lint ;
    build:command "cargo clippy --workspace --all-targets -- -D warnings" ;
    build:cost build:Medium ;
    build:category build:QualityGate ;
    build:toolchain "cargo" ;
    build:estimatedDuration 120 ;
    build:successRate 0.97 .

build:terraphim-ai-step-3 a build:BuildStep ;
    build:actionType build:Compile ;
    build:command "cargo build --workspace" ;
    build:cost build:High ;
    build:category build:Compilation ;
    build:toolchain "cargo" ;
    build:estimatedDuration 180 ;
    build:successRate 0.95 .

build:terraphim-ai-step-4 a build:BuildStep ;
    build:actionType build:Test ;
    build:command "cargo test --workspace --no-fail-fast" ;
    build:cost build:High ;
    build:category build:Verification ;
    build:toolchain "cargo" ;
    build:estimatedDuration 300 ;
    build:successRate 0.92 .
```

## terraphim-automata Integration

```rust
// crates/terraphim_automata/src/build_matcher.rs

use terraphim_automata::AhoCorasick;

/// Matches project state against known build sequences in KG
pub struct BuildMatcher {
    automaton: AhoCorasick,
    sequences: HashMap<String, BuildSequence>,
}

impl BuildMatcher {
    /// Check if we have a cached build sequence for this project fingerprint
    pub fn find_sequence(&self, fingerprint: &str) -> Option<&BuildSequence> {
        self.sequences.get(fingerprint)
    }

    /// Store a new build sequence in the KG
    pub fn cache_sequence(&mut self, fingerprint: String, sequence: BuildSequence) {
        self.sequences.insert(fingerprint, sequence);
        // Update Aho-Corasick automaton for fast matching
        self.rebuild_automaton();
    }
}
```

## Execution Flow

### Cold Start (First Run)
```bash
# 1. Compute project fingerprint
FINGERPRINT=$(cargo hash-or-fingerprint)  # Or hash of Cargo.toml + Cargo.lock

# 2. Check terraphim KG cache
SEQUENCE=$(terraphim-agent kg query \
  --role "DevOps Engineer" \
  "build sequence for fingerprint:$FINGERPRINT")

if [ -z "$SEQUENCE" ]; then
  # 3. Cache miss - extract with haiku
  echo "No cached build sequence found. Extracting with haiku..."

  COMMANDS=$(cat BUILD.md | haiku -p "Extract build steps as JSON")

  # 4. Store in terraphim KG
  terraphim-agent kg insert \
    --project terraphim-ai \
    --fingerprint "$FINGERPRINT" \
    --sequence "$COMMANDS"

  echo "Cached build sequence for future runs"
fi
```

### Hot Path (Subsequent Runs)
```bash
# 1. Compute fingerprint (same as above)
FINGERPRINT=$(cargo hash-or-fingerprint)

# 2. Fast Aho-Corasick match against KG
SEQUENCE=$(terraphim-agent kg match \
  --fingerprint "$FINGERPRINT" \
  --format commands)

# 3. Execute deterministically (NO LLM CALL)
for cmd in $SEQUENCE; do
  echo "Executing: $cmd"
  rch exec -- $cmd || {
    echo "Command failed: $cmd"
    # Update KG with failure
    terraphim-agent learn capture "$cmd" \
      --error "exit code $?" \
      --project terraphim-ai
    exit 1
  }
done
```

## Project Fingerprint

```rust
/// Compute a fingerprint for the project's build configuration
/// Used as the KG lookup key
pub fn compute_project_fingerprint(project_dir: &Path) -> String {
    let mut hasher = Sha256::new();

    // Hash key build files
    for file in [
        "Cargo.toml",
        "Cargo.lock",
        "BUILD.md",
        "package.json",
        "Makefile",
    ] {
        if let Ok(content) = fs::read_to_string(project_dir.join(file)) {
            hasher.update(content.as_bytes());
        }
    }

    // Hash workspace structure
    if let Ok(entries) = fs::read_dir(project_dir.join("crates")) {
        for entry in entries.flatten() {
            if entry.path().join("Cargo.toml").exists() {
                hasher.update(entry.file_name().as_encoded_bytes());
            }
        }
    }

    format!("sha256:{:x}", hasher.finalize())
}
```

## Invalidation Strategy

```rust
/// When to invalidate the cached build sequence
pub fn should_invalidate_cache(
    project_dir: &Path,
    cached_at: DateTime<Utc>,
) -> bool {
    // Invalidate if BUILD.md changed since caching
    if let Ok(metadata) = fs::metadata(project_dir.join("BUILD.md")) {
        if let Ok(modified) = metadata.modified() {
            if modified > cached_at {
                return true;
            }
        }
    }

    // Invalidate if Cargo.toml changed (new dependencies might need different build steps)
    if let Ok(metadata) = fs::metadata(project_dir.join("Cargo.toml")) {
        if let Ok(modified) = metadata.modified() {
            if modified > cached_at {
                return true;
            }
        }
    }

    // Invalidate on manual trigger
    if env::var("BUILD_RUNNER_INVALIDATE_CACHE").is_ok() {
        return true;
    }

    false
}
```

## Cost Analysis

| Phase | LLM Calls | Cost | Latency |
|-------|-----------|------|---------|
| **Cold start** | 1 (haiku) | ~$0.005 | ~15s |
| **Hot path** | 0 | $0 | ~0.1s (KG lookup) |
| **Failure recovery** | 1 (haiku) | ~$0.005 | ~15s |
| **Average over 100 builds** | 1/50 | ~$0.0001 | ~0.2s |

## Implementation Plan (Updated)

### Phase 1: KG Schema (2h)
- Add `build:` ontology to terraphim_types
- Create `BuildSequence`, `BuildStep`, `ProjectFingerprint` types
- Add terraphim-automata matcher for build sequences

### Phase 2: Fingerprint & Cache (2h)
- Implement `compute_project_fingerprint()`
- Add terraphim-agent CLI for KG insert/query
- Implement cache invalidation logic

### Phase 3: Cold Start Extraction (2h)
- haiku prompt to extract build steps from BUILD.md
- Store extracted sequence in KG
- Link to project fingerprint

### Phase 4: Hot Path Execution (2h)
- Fast KG lookup by fingerprint
- Deterministic command execution
- rch integration

### Phase 5: Failure Learning (2h)
- Capture failed commands in terraphim-agent learnings
- Update KG success rates
- Fallback to LLM on unknown failures

**Total: 10 hours**

## Example terraphim.toml Agent Config

```toml
[[agents]]
name = "build-runner-llm"
layer = "Growth"
cli_tool = "/bin/bash"
model = "haiku"                    # Only used on cold start or failure
max_cpu_seconds = 1800
grace_period_secs = 30
event_only = true
project = "terraphim-ai"
capabilities = ["build", "test", "kg-cached-ci"]

# terraphim KG integration
kg_role = "DevOps Engineer"        # Role for KG lookups
kg_fingerprint_files = [           # Files to hash for fingerprint
    "Cargo.toml",
    "Cargo.lock",
    "BUILD.md",
]
cache_invalidation_hours = 24      # Max cache age

# Fallback to deterministic script if KG fails
fallback_agent = "build-runner"
```

## Advantages Over v1 Design

1. **Cheaper**: After first run, builds cost ~$0 (no LLM calls)
2. **Faster**: Hot path is ~0.1s KG lookup vs ~15s LLM extraction
3. **Self-learning**: Failed commands automatically update KG success rates
4. **Deterministic**: Hot path is fully deterministic, no LLM non-determinism
5. **Leverages existing infrastructure**: Uses terraphim-automata, terraphim-agent learnings
6. **Project-aware**: Different projects can have different build sequences cached

## Files to Change (Updated)

| File | Purpose |
|------|---------|
| `crates/terraphim_types/src/build_ontology.rs` | KG schema for build sequences |
| `crates/terraphim_automata/src/build_matcher.rs` | Aho-Corasick matcher for build fingerprints |
| `crates/terraphim_agent/src/kg_commands.rs` | CLI for KG insert/query |
| `scripts/build-runner-llm.sh` | Updated with KG-first logic |
| `BUILD.md` | Semantic build documentation |

## Migration from Current build-runner

```bash
# Day 1: Deploy with KG caching
# First build: haiku extracts + caches (15s overhead)
# Builds 2-N: KG lookup (0.1s overhead)

# Day 7: Compare metrics
# Current build-runner: $0.00, 3min, 0% adaptability
# build-runner-llm v2: $0.005 total, 3min+0.1s, 100% cached
```
