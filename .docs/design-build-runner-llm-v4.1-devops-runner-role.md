# Build-Runner-LLM v4.1: DevOpsRunner Role + Build Ontology

## Correction from v4

While terraphim-agent's `replace` command works for simple tool substitution (npm→bun, pip→uv), a **proper build-runner requires a dedicated role and ontology**:

1. **DevOpsRunner Role**: Specialized role for CI/CD operations with build-specific knowledge graph
2. **Build Ontology**: Structured taxonomy of build steps, not just tool synonyms
3. **Knowledge Graph Population**: Build sequences linked to project fingerprints

## Why Simple Replacement Isn't Enough

```bash
# Simple replacement works for tools:
$ terraphim-agent replace "npm install" --role "Terraphim Engineer"
> bun install

# But build-runner needs SEMANTIC understanding:
$ terraphim-agent replace "cargo test --workspace" --role "Terraphim Engineer"
> cargo test --workspace  (no change - rch not in general KG)

# Need DevOpsRunner role with build ontology:
$ terraphim-agent replace "cargo test --workspace" --role "DevOpsRunner"
> rch exec -- cargo test --workspace  (transformed via build ontology)
```

## DevOpsRunner Role Configuration

```json
{
  "roles": {
    "DevOpsRunner": {
      "shortname": "DevOps",
      "name": "DevOps Runner",
      "relevance_function": "terraphim-graph",
      "terraphim_it": false,
      "kg": {
        "automata_path": null,
        "knowledge_graph_local": {
          "input_type": "markdown",
          "path": "~/.config/terraphim/docs/src/kg/devops"
        }
      },
      "llm_enabled": true,
      "llm_model": "haiku",
      "llm_auto_summarize": false
    }
  }
}
```

## Build Ontology Structure

### Directory Layout

```
~/.config/terraphim/docs/src/kg/devops/
├── build-steps/
│   ├── format.md
│   ├── lint.md
│   ├── compile.md
│   ├── test.md
│   ├── doc.md
│   └── audit.md
├── toolchains/
│   ├── cargo.md
│   ├── npm.md
│   ├── make.md
│   └── docker.md
├── platforms/
│   ├── github-actions.md
│   ├── earthfile.md
│   └── dockerfile.md
└── transformations/
    ├── cargo-to-rch.md
    ├── npm-to-bun.md
    └── pip-to-uv.md
```

### Build Steps Ontology

```markdown
# ~/.config/terraphim/docs/src/kg/devops/build-steps/format.md

# Format

Source code formatting to ensure consistent style across the codebase.

synonyms:: fmt, format, formatting, style-check
action:: format
cost:: low
category:: quality-gate
typical_duration:: 30s
toolchains:: cargo fmt, black, prettier, gofmt
```

```markdown
# ~/.config/terraphim/docs/src/kg/devops/build-steps/lint.md

# Lint

Static analysis to catch bugs, style violations, and anti-patterns.

synonyms:: lint, clippy, eslint, pylint, static-analysis
cost:: medium
category:: quality-gate
typical_duration:: 120s
toolchains:: cargo clippy, eslint, pylint, shellcheck
```

```markdown
# ~/.config/terraphim/docs/src/kg/devops/build-steps/compile.md

# Compile

Build artifacts from source code.

synonyms:: build, compile, compilation, construct
cost:: high
category:: compilation
typical_duration:: 180s
toolchains:: cargo build, make, npm run build, docker build
```

```markdown
# ~/.config/terraphim/docs/src/kg/devops/build-steps/test.md

# Test

Run automated tests to verify correctness.

synonyms:: test, testing, verify, check, validate
cost:: high
category:: verification
typical_duration:: 300s
toolchains:: cargo test, npm test, pytest, go test
```

### Transformations (Command Mapping)

```markdown
# ~/.config/terraphim/docs/src/kg/devops/transformations/cargo-to-rch.md

# Remote Compilation (rch)

When running cargo commands in CI context, use rch for remote compilation.

transformation:: cargo build → rch exec -- cargo build
transformation:: cargo test → rch exec -- cargo test
transformation:: cargo clippy → rch exec -- cargo clippy
transformation:: cargo check → rch exec -- cargo check

context:: ci, build-runner
project_types:: rust, cargo-workspace
```

```markdown
# ~/.config/terraphim/docs/src/kg/devops/transformations/npm-to-bun.md

# Bun Package Manager

Use bun instead of npm/yarn/pnpm for JavaScript projects.

transformation:: npm install → bun install
transformation:: npm run build → bun run build
transformation:: npm test → bun test
transformation:: npx → bunx

context:: ci, build-runner
project_types:: javascript, typescript, node
```

## Populating the Knowledge Graph

### Method 1: Manual Curation

```bash
# Create DevOpsRunner KG directory
mkdir -p ~/.config/terraphim/docs/src/kg/devops/{build-steps,toolchains,platforms,transformations}

# Copy build ontology files
cp -r docs/build-ontology/* ~/.config/terraphim/docs/src/kg/devops/

# Rebuild automata
~/.cargo/bin/terraphim-agent graph --role "DevOpsRunner"
```

### Method 2: LLM-Assisted Extraction (One-Time)

```bash
#!/bin/bash
# populate-kg.sh - One-time population of DevOpsRunner KG

PROJECT_DIR="$1"
ROLE="DevOpsRunner"

echo "Analyzing project: $PROJECT_DIR"

# Extract build system
if [ -f "$PROJECT_DIR/.github/workflows/ci-pr.yml" ]; then
    echo "Found GitHub Actions"

    # Use haiku to extract and categorize build steps
    yq '.jobs[].steps[].run' "$PROJECT_DIR/.github/workflows/ci-pr.yml" | \
    while IFS= read -r cmd; do
        [ -z "$cmd" ] && continue

        # Categorize command using LLM
        CATEGORY=$(echo "$cmd" | ~/.local/bin/claude --model haiku -p "Categorize this CI command as: format, lint, compile, test, doc, audit, or deploy. Reply with just the category.")

        # Create KG entry
        cat >> ~/.config/terraphim/docs/src/kg/devops/build-steps/${CATEGORY}.md <<EOF
# ${CATEGORY}

## Detected Command
action:: ${cmd}
source:: github-actions
project:: $(basename "$PROJECT_DIR")

EOF
    done
fi

# Rebuild graph
~/.cargo/bin/terraphim-agent graph --role "$ROLE"
echo "DevOpsRunner KG populated for $PROJECT_DIR"
```

### Method 3: From Existing CI Config

```bash
#!/bin/bash
# auto-populate-kg.sh - Populate KG from existing CI configuration

# Parse GitHub Actions
parse_github_actions() {
    local file="$1"

    # Extract all 'run' commands
    yq '.jobs[].steps[] | select(.run) | .run' "$file" | \
    while IFS= read -r cmd; do
        [ -z "$cmd" ] || [ "$cmd" = "null" ] && continue

        # Infer step type from command
        local step_type="custom"
        case "$cmd" in
            *"fmt"*|*"format"*) step_type="format" ;;
            *"clippy"*|*"lint"*|*"eslint"*) step_type="lint" ;;
            *"build"*|*"compile"*|*"cargo build"*) step_type="compile" ;;
            *"test"*|*"cargo test"*|*"pytest"*) step_type="test" ;;
            *"doc"*|*"documentation"*) step_type="doc" ;;
            *"audit"*|*"security"*) step_type="audit" ;;
        esac

        # Add to KG
        cat >> ~/.config/terraphim/docs/src/kg/devops/build-steps/${step_type}.md <<EOF
# ${step_type}

detected_command:: ${cmd}
source:: github-actions
file:: ${file}

EOF
    done
}

# Parse Makefile
parse_makefile() {
    local file="$1"

    grep -E "^[a-zA-Z_-]+:" "$file" | while IFS=: read -r target _; do
        case "$target" in
            fmt|format) step_type="format" ;;
            lint|check) step_type="lint" ;;
            build|all) step_type="compile" ;;
            test) step_type="test" ;;
            doc) step_type="doc" ;;
            *) step_type="custom" ;;
        esac

        cat >> ~/.config/terraphim/docs/src/kg/devops/build-steps/${step_type}.md <<EOF
# ${step_type}

detected_command:: make ${target}
source:: makefile
file:: ${file}

EOF
    done
}

# Main
mkdir -p ~/.config/terraphim/docs/src/kg/devops/build-steps

if [ -f ".github/workflows/ci-pr.yml" ]; then
    parse_github_actions ".github/workflows/ci-pr.yml"
fi

if [ -f "Makefile" ]; then
    parse_makefile "Makefile"
fi

# Rebuild graph
~/.cargo/bin/terraphim-agent graph --role "DevOpsRunner"
```

## Updated Build-Runner Script

```bash
#!/bin/bash
# build-runner-llm.sh - Using DevOpsRunner role and KG

set -e

export PATH=$HOME/.cargo/bin:$HOME/.local/bin:$HOME/bin:$HOME/.bun/bin:/usr/local/bin:/usr/bin:/bin:$PATH
export GITEA_URL=https://git.terraphim.cloud

ROLE="DevOpsRunner"
AGENT="$HOME/.cargo/bin/terraphim-agent"

# Status helper
POST_STATUS() {
  local STATE="$1" DESC="$2"
  curl -fsS -X POST \
    -H "Authorization: token $GITEA_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"state\":\"$STATE\",\"context\":\"adf/build\",\"description\":\"$DESC\"}" \
    "$GITEA_URL/api/v1/repos/$GITEA_OWNER/$GITEA_REPO/statuses/$ADF_PUSH_SHA" \
    >/dev/null 2>&1 || true
}

POST_STATUS pending "build started (DevOpsRunner)"

cd "$ADF_WORKING_DIR"

# Compute project fingerprint
FINGERPRINT=$(sha256sum Cargo.toml Cargo.lock .github/workflows/ci-pr.yml 2>/dev/null | sha256sum | cut -d' ' -f1)

# Check if we have cached build sequence for this fingerprint
CACHED_SEQUENCE=$($AGENT search \
  --role "$ROLE" \
  "build sequence fingerprint:$FINGERPRINT" 2>/dev/null || true)

if [ -n "$CACHED_SEQUENCE" ]; then
    echo "Using cached build sequence (fingerprint: ${FINGERPRINT:0:8}...)"

    # Extract commands from cached sequence using terraphim-automata
    COMMANDS=$(echo "$CACHED_SEQUENCE" | $AGENT extract \
        --role "$ROLE" 2>/dev/null || true)
else
    echo "No cached sequence found. Extracting from CI config..."

    if [ -f ".github/workflows/ci-pr.yml" ]; then
        # Parse GitHub Actions
        COMMANDS=$(yq '.jobs[].steps[].run' .github/workflows/ci-pr.yml | grep -v null)

        # Cache for future runs
        echo "$COMMANDS" | $AGENT learn capture \
            "build sequence $FINGERPRINT" \
            --project "$(basename $ADF_WORKING_DIR)" \
            --metadata "{\"fingerprint\":\"$FINGERPRINT\"}" \
            --exit-code 0
    else
        echo "No CI config found, using fallback"
        COMMANDS="cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace --no-fail-fast"
    fi
fi

# Execute commands with DevOpsRunner transformation
SUCCESS=0
TOTAL=0

echo "$COMMANDS" | while IFS= read -r cmd; do
    [ -z "$cmd" ] && continue
    TOTAL=$((TOTAL + 1))

    # Transform command using DevOpsRunner role
    TRANSFORMED=$(echo "$cmd" | $AGENT replace \
        --role "$ROLE" \
        --json 2>/dev/null | jq -r '.result // empty')

    [ -z "$TRANSFORMED" ] && TRANSFORMED="$cmd"

    if [ "$TRANSFORMED" != "$cmd" ]; then
        echo "[$TOTAL] $cmd → $TRANSFORMED"
    else
        echo "[$TOTAL] $cmd"
    fi

    # Execute
    if eval "$TRANSFORMED"; then
        SUCCESS=$((SUCCESS + 1))
    else
        EXIT_CODE=$?
        POST_STATUS failure "step $TOTAL failed: ${TRANSFORMED:0:50}"

        # Capture learning
        echo "$cmd" | $AGENT learn capture \
            "build command failed" \
            --error "exit code $EXIT_CODE" \
            --project "$(basename $ADF_WORKING_DIR)" \
            --exit-code "$EXIT_CODE"

        exit 1
    fi
done

POST_STATUS success "build passed ($SUCCESS/$TOTAL steps)"
```

## KG Files to Create

| File | Content | Purpose |
|------|---------|---------|
| `~/.config/terraphim/config.json` | Add DevOpsRunner role | New role configuration |
| `~/.config/terraphim/docs/src/kg/devops/build-steps/format.md` | Format step ontology | Semantic build step |
| `~/.config/terraphim/docs/src/kg/devops/build-steps/lint.md` | Lint step ontology | Semantic build step |
| `~/.config/terraphim/docs/src/kg/devops/build-steps/compile.md` | Compile step ontology | Semantic build step |
| `~/.config/terraphim/docs/src/kg/devops/build-steps/test.md` | Test step ontology | Semantic build step |
| `~/.config/terraphim/docs/src/kg/devops/transformations/cargo-to-rch.md` | rch transformation | Tool replacement |
| `~/.config/terraphim/docs/src/kg/devops/transformations/npm-to-bun.md` | bun transformation | Tool replacement |

## Implementation Plan (Final)

### Phase 1: Create DevOpsRunner Role (30 min)
- Update `~/.config/terraphim/config.json`
- Create `~/.config/terraphim/docs/src/kg/devops/` directory structure

### Phase 2: Populate Build Ontology (1 hour)
- Create build step markdown files (format, lint, compile, test)
- Create transformation files (cargo-to-rch, npm-to-bun)
- Build knowledge graph: `terraphim-agent graph --role DevOpsRunner`

### Phase 3: Build-Runner Script (30 min)
- Create `scripts/build-runner-llm.sh`
- Use DevOpsRunner role for transformations
- Add fingerprint-based caching

### Phase 4: Testing (30 min)
- Test with terraphim-ai project
- Verify cargo→rch transformation works
- Verify build sequence caching works

**Total: 2.5 hours**

## Verification Commands

```bash
# Test DevOpsRunner role
~/.cargo/bin/terraphim-agent replace "cargo build --workspace" --role "DevOpsRunner" --json

# Expected output:
# {"result":"rch exec -- cargo build --workspace", "changed":true}

# Test build step extraction
~/.cargo/bin/terraphim-agent extract "format the code and run tests" --role "DevOpsRunner"

# Expected output:
# format
# test
```
