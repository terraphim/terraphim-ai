# Codebase Evaluation Check Using Terraphim AI

## Overview

This document describes how to use Terraphim AI's deterministic search and knowledge graph capabilities to evaluate whether an AI agent (e.g., Claude Code, GitHub Copilot, or autonomous coding agents) improves or deteriorates a codebase.

Terraphim is ideally suited for this evaluation because:
- **Deterministic**: Aho-Corasick automata provide consistent, repeatable scoring
- **Local & Private**: No external API dependencies for evaluation
- **Knowledge Graph-Based**: Captures semantic relationships in code
- **Role-Specific**: Customizable evaluation perspectives (security, performance, quality)
- **Quantifiable**: Provides numeric scores for objective comparison

## Core Concept

The check compares **before** and **after** states of a codebase by:
1. Indexing the codebase as a haystack
2. Building knowledge graphs under custom evaluation roles
3. Running standardized queries to measure code quality
4. Comparing quantitative metrics (scores, graph density, error counts)
5. Generating a verdict: Improvement, Deterioration, or Neutral

## Prerequisites

### Required
- Target codebase (Git repository)
- Terraphim AI installation (see Installation section)
- Predefined evaluation queries tailored to your domain
- AI agent that proposes changes (pull requests, patches, etc.)

### Optional
- Metrics tools: `cargo clippy`, `cargo test`, code coverage tools
- CI/CD integration (GitHub Actions, GitLab CI)
- Cloud storage (AWS S3, R2) for cross-run persistence

## Installation

### Quick Install (Docker)

```bash
# Docker-based installation (easiest)
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/docker-run.sh | bash
```

### Binary Installation

```bash
# Direct binary installation
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash
```

### Build from Source

```bash
# Clone repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build backend server
cargo build --release

# Build TUI with full REPL features
cargo build -p terraphim_tui --features repl-full --release

# Run server
cargo run --release

# Run TUI (in separate terminal)
./target/release/terraphim-tui
```

### Environment Configuration

```bash
# Logging level
export LOG_LEVEL=debug
export RUST_LOG=debug

# Data persistence path
export TERRAPHIM_DATA_PATH=./evaluation-data

# Optional: Cloud storage for cross-run persistence
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_REGION=us-east-1
export S3_BUCKET=terraphim-evaluations
```

## Evaluation Roles

Terraphim uses **roles** to define evaluation perspectives. Each role has:
- **Name**: Role identifier (e.g., "Code Reviewer")
- **Knowledge Graph**: Terms and synonyms relevant to the evaluation
- **Haystack**: Data sources to search (local codebase, documentation)
- **Relevance Function**: Scoring algorithm (TerraphimGraph, BM25, TitleScorer)

### Predefined Evaluation Roles

#### 1. Code Reviewer Role

**Focus**: Code quality, maintainability, best practices

**Knowledge Graph Terms** (`docs/src/kg/`):

Create `code-quality.md`:
```markdown
# Code Quality

Code quality encompasses maintainability, readability, and adherence to best practices.

synonyms:: code smell, technical debt, maintainability issue, refactoring opportunity, bad practice
```

Create `bug-patterns.md`:
```markdown
# Bug Patterns

Common programming errors and anti-patterns that lead to bugs.

synonyms:: null pointer, memory leak, race condition, off-by-one error, unhandled exception, edge case
```

Create `duplication.md`:
```markdown
# Code Duplication

Repeated code that should be refactored into reusable components.

synonyms:: duplicate code, repeated logic, copy-paste code, DRY violation, code clone
```

#### 2. Performance Analyst Role

**Focus**: Efficiency, optimization, resource usage

Create `performance-bottleneck.md`:
```markdown
# Performance Bottleneck

Code sections that cause performance degradation.

synonyms:: slow code, inefficient algorithm, O(n^2) complexity, blocking operation, performance issue, bottleneck
```

Create `optimization-opportunity.md`:
```markdown
# Optimization Opportunity

Areas where performance can be improved.

synonyms:: can be optimized, performance improvement, efficiency gain, faster alternative, reduce allocations
```

#### 3. Security Auditor Role

**Focus**: Security vulnerabilities, attack vectors

Create `security-vulnerability.md`:
```markdown
# Security Vulnerability

Security flaws that could be exploited.

synonyms:: SQL injection, XSS, CSRF, authentication flaw, authorization bypass, insecure deserialization, command injection
```

Create `input-validation.md`:
```markdown
# Input Validation

Issues related to unvalidated or improperly sanitized input.

synonyms:: unsanitized input, missing validation, user input, untrusted data, injection vulnerability
```

#### 4. Documentation Quality Role

**Focus**: Code comments, documentation completeness

Create `missing-documentation.md`:
```markdown
# Missing Documentation

Functions, modules, or APIs lacking adequate documentation.

synonyms:: undocumented, no comments, missing docstring, unclear API, needs documentation
```

### Role Configuration

Create role configuration file `evaluation_roles.json`:

```json
{
  "roles": [
    {
      "name": "Code Reviewer",
      "relevance_function": "terraphim-graph",
      "kg": {
        "knowledge_graph_local": {
          "input_type": "markdown",
          "path": "docs/src/kg/code-quality"
        }
      },
      "haystacks": [
        {
          "name": "Target Codebase",
          "service": "Ripgrep",
          "extra_parameters": {
            "path": "./target-codebase"
          }
        }
      ]
    },
    {
      "name": "Performance Analyst",
      "relevance_function": "terraphim-graph",
      "kg": {
        "knowledge_graph_local": {
          "input_type": "markdown",
          "path": "docs/src/kg/performance"
        }
      },
      "haystacks": [
        {
          "name": "Target Codebase",
          "service": "Ripgrep",
          "extra_parameters": {
            "path": "./target-codebase"
          }
        }
      ]
    },
    {
      "name": "Security Auditor",
      "relevance_function": "terraphim-graph",
      "kg": {
        "knowledge_graph_local": {
          "input_type": "markdown",
          "path": "docs/src/kg/security"
        }
      },
      "haystacks": [
        {
          "name": "Target Codebase",
          "service": "Ripgrep",
          "extra_parameters": {
            "path": "./target-codebase"
          }
        }
      ]
    }
  ]
}
```

## Evaluation Procedure

### Step 1: Baseline Evaluation (Before AI Changes)

```bash
#!/bin/bash
# baseline-evaluation.sh

set -euo pipefail

CODEBASE_PATH="$1"
ROLE="${2:-Code Reviewer}"
OUTPUT_DIR="./evaluation-results/baseline"

mkdir -p "$OUTPUT_DIR"

echo "=== Baseline Evaluation for Role: $ROLE ==="

# 1. Index the codebase
echo "Indexing codebase at $CODEBASE_PATH..."
terraphim-tui index --role "$ROLE" --path "$CODEBASE_PATH"

# 2. Run evaluation queries
echo "Running evaluation queries..."

# Code quality queries
terraphim-tui search "code smell" --role "$ROLE" > "$OUTPUT_DIR/code-smells.json"
terraphim-tui search "bug patterns" --role "$ROLE" > "$OUTPUT_DIR/bug-patterns.json"
terraphim-tui search "code duplication" --role "$ROLE" > "$OUTPUT_DIR/duplication.json"

# 3. Extract metrics
echo "Extracting knowledge graph metrics..."
terraphim-tui graph-stats --role "$ROLE" > "$OUTPUT_DIR/graph-stats.json"

# 4. Run supplementary tools
echo "Running supplementary quality checks..."

# For Rust codebases
if [ -f "$CODEBASE_PATH/Cargo.toml" ]; then
    cd "$CODEBASE_PATH"
    cargo clippy --all-targets -- -D warnings 2>&1 | tee "$OUTPUT_DIR/clippy.log"
    cargo test 2>&1 | tee "$OUTPUT_DIR/test.log"
    cd -
fi

# Count lines of code
tokei "$CODEBASE_PATH" --output json > "$OUTPUT_DIR/tokei.json"

echo "Baseline evaluation complete. Results in $OUTPUT_DIR"
```

### Step 2: Apply AI Agent Changes

```bash
#!/bin/bash
# apply-ai-changes.sh

set -euo pipefail

BEFORE_PATH="$1"
AFTER_PATH="$2"
AI_AGENT="${3:-claude-code}"

echo "=== Applying AI Agent Changes ==="
echo "Before: $BEFORE_PATH"
echo "After: $AFTER_PATH"
echo "Agent: $AI_AGENT"

# Copy codebase for modification
cp -r "$BEFORE_PATH" "$AFTER_PATH"

# Apply AI agent changes
# This could be:
# - Running Claude Code with specific prompts
# - Applying a PR from GitHub Copilot
# - Executing autonomous agent tasks

case "$AI_AGENT" in
    claude-code)
        echo "Apply Claude Code changes to $AFTER_PATH"
        # Example: Use Claude API or manual intervention
        ;;
    copilot)
        echo "Apply GitHub Copilot suggestions to $AFTER_PATH"
        ;;
    custom)
        echo "Apply custom agent changes to $AFTER_PATH"
        ;;
    *)
        echo "Unknown agent: $AI_AGENT"
        exit 1
        ;;
esac

echo "AI changes applied. Review $AFTER_PATH before evaluation."
```

### Step 3: Post-Change Evaluation

```bash
#!/bin/bash
# post-evaluation.sh

set -euo pipefail

CODEBASE_PATH="$1"
ROLE="${2:-Code Reviewer}"
OUTPUT_DIR="./evaluation-results/after"

mkdir -p "$OUTPUT_DIR"

echo "=== Post-Change Evaluation for Role: $ROLE ==="

# Re-index the modified codebase
echo "Re-indexing codebase at $CODEBASE_PATH..."
terraphim-tui index --role "$ROLE" --path "$CODEBASE_PATH" --rebuild

# Run the same queries
echo "Running evaluation queries..."

terraphim-tui search "code smell" --role "$ROLE" > "$OUTPUT_DIR/code-smells.json"
terraphim-tui search "bug patterns" --role "$ROLE" > "$OUTPUT_DIR/bug-patterns.json"
terraphim-tui search "code duplication" --role "$ROLE" > "$OUTPUT_DIR/duplication.json"

# Extract metrics
echo "Extracting knowledge graph metrics..."
terraphim-tui graph-stats --role "$ROLE" > "$OUTPUT_DIR/graph-stats.json"

# Run supplementary tools
echo "Running supplementary quality checks..."

if [ -f "$CODEBASE_PATH/Cargo.toml" ]; then
    cd "$CODEBASE_PATH"
    cargo clippy --all-targets -- -D warnings 2>&1 | tee "$OUTPUT_DIR/clippy.log"
    cargo test 2>&1 | tee "$OUTPUT_DIR/test.log"
    cd -
fi

tokei "$CODEBASE_PATH" --output json > "$OUTPUT_DIR/tokei.json"

echo "Post-change evaluation complete. Results in $OUTPUT_DIR"
```

### Step 4: Comparison and Verdict

```bash
#!/bin/bash
# compare-evaluations.sh

set -euo pipefail

BASELINE_DIR="./evaluation-results/baseline"
AFTER_DIR="./evaluation-results/after"
REPORT_FILE="./evaluation-results/verdict.md"

echo "=== Comparing Evaluations ==="

# Function to extract score from JSON
extract_score() {
    local file="$1"
    jq -r '.score // 0' "$file"
}

# Function to count results
count_results() {
    local file="$1"
    jq -r '.results | length' "$file"
}

# Initialize report
cat > "$REPORT_FILE" << 'EOF'
# Codebase Evaluation Verdict

## Summary

EOF

# Compare code smells
BASELINE_SMELLS=$(count_results "$BASELINE_DIR/code-smells.json")
AFTER_SMELLS=$(count_results "$AFTER_DIR/code-smells.json")
SMELLS_DELTA=$((AFTER_SMELLS - BASELINE_SMELLS))

echo "### Code Smells" >> "$REPORT_FILE"
echo "- Baseline: $BASELINE_SMELLS" >> "$REPORT_FILE"
echo "- After: $AFTER_SMELLS" >> "$REPORT_FILE"
echo "- Delta: $SMELLS_DELTA" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Compare bug patterns
BASELINE_BUGS=$(count_results "$BASELINE_DIR/bug-patterns.json")
AFTER_BUGS=$(count_results "$AFTER_DIR/bug-patterns.json")
BUGS_DELTA=$((AFTER_BUGS - BASELINE_BUGS))

echo "### Bug Patterns" >> "$REPORT_FILE"
echo "- Baseline: $BASELINE_BUGS" >> "$REPORT_FILE"
echo "- After: $AFTER_BUGS" >> "$REPORT_FILE"
echo "- Delta: $BUGS_DELTA" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Compare duplication
BASELINE_DUP=$(count_results "$BASELINE_DIR/duplication.json")
AFTER_DUP=$(count_results "$AFTER_DIR/duplication.json")
DUP_DELTA=$((AFTER_DUP - BASELINE_DUP))

echo "### Code Duplication" >> "$REPORT_FILE"
echo "- Baseline: $BASELINE_DUP" >> "$REPORT_FILE"
echo "- After: $AFTER_DUP" >> "$REPORT_FILE"
echo "- Delta: $DUP_DELTA" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Compare clippy warnings (if available)
if [ -f "$BASELINE_DIR/clippy.log" ] && [ -f "$AFTER_DIR/clippy.log" ]; then
    BASELINE_WARNINGS=$(grep -c "warning:" "$BASELINE_DIR/clippy.log" || echo 0)
    AFTER_WARNINGS=$(grep -c "warning:" "$AFTER_DIR/clippy.log" || echo 0)
    WARNINGS_DELTA=$((AFTER_WARNINGS - BASELINE_WARNINGS))

    echo "### Clippy Warnings" >> "$REPORT_FILE"
    echo "- Baseline: $BASELINE_WARNINGS" >> "$REPORT_FILE"
    echo "- After: $AFTER_WARNINGS" >> "$REPORT_FILE"
    echo "- Delta: $WARNINGS_DELTA" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

# Compare test results
if [ -f "$BASELINE_DIR/test.log" ] && [ -f "$AFTER_DIR/test.log" ]; then
    BASELINE_PASSES=$(grep -c "test result: ok" "$BASELINE_DIR/test.log" || echo 0)
    AFTER_PASSES=$(grep -c "test result: ok" "$AFTER_DIR/test.log" || echo 0)

    echo "### Test Results" >> "$REPORT_FILE"
    echo "- Baseline: $BASELINE_PASSES passing" >> "$REPORT_FILE"
    echo "- After: $AFTER_PASSES passing" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

# Calculate overall verdict
IMPROVEMENT_COUNT=0
DETERIORATION_COUNT=0

# Lower is better for problems
[ "$SMELLS_DELTA" -lt 0 ] && ((IMPROVEMENT_COUNT++)) || ((DETERIORATION_COUNT++))
[ "$BUGS_DELTA" -lt 0 ] && ((IMPROVEMENT_COUNT++)) || ((DETERIORATION_COUNT++))
[ "$DUP_DELTA" -lt 0 ] && ((IMPROVEMENT_COUNT++)) || ((DETERIORATION_COUNT++))

if [ -n "${WARNINGS_DELTA+x}" ]; then
    [ "$WARNINGS_DELTA" -lt 0 ] && ((IMPROVEMENT_COUNT++)) || ((DETERIORATION_COUNT++))
fi

echo "## Verdict" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if [ "$IMPROVEMENT_COUNT" -gt "$DETERIORATION_COUNT" ]; then
    echo "✅ **IMPROVEMENT**: The AI agent improved the codebase quality." >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "- Improved metrics: $IMPROVEMENT_COUNT" >> "$REPORT_FILE"
    echo "- Deteriorated metrics: $DETERIORATION_COUNT" >> "$REPORT_FILE"
elif [ "$DETERIORATION_COUNT" -gt "$IMPROVEMENT_COUNT" ]; then
    echo "❌ **DETERIORATION**: The AI agent worsened the codebase quality." >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "- Improved metrics: $IMPROVEMENT_COUNT" >> "$REPORT_FILE"
    echo "- Deteriorated metrics: $DETERIORATION_COUNT" >> "$REPORT_FILE"
else
    echo "➖ **NEUTRAL**: The AI agent had mixed or minimal impact." >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "- Improved metrics: $IMPROVEMENT_COUNT" >> "$REPORT_FILE"
    echo "- Deteriorated metrics: $DETERIORATION_COUNT" >> "$REPORT_FILE"
fi

echo "" >> "$REPORT_FILE"
echo "## Recommendations" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if [ "$SMELLS_DELTA" -gt 0 ]; then
    echo "- Review new code smells introduced by AI changes" >> "$REPORT_FILE"
fi

if [ "$BUGS_DELTA" -gt 0 ]; then
    echo "- Address new bug patterns introduced by AI changes" >> "$REPORT_FILE"
fi

if [ "$DUP_DELTA" -gt 0 ]; then
    echo "- Refactor new code duplication" >> "$REPORT_FILE"
fi

if [ -n "${WARNINGS_DELTA+x}" ] && [ "$WARNINGS_DELTA" -gt 0 ]; then
    echo "- Fix new clippy warnings" >> "$REPORT_FILE"
fi

echo "" >> "$REPORT_FILE"
echo "---" >> "$REPORT_FILE"
echo "*Generated by Terraphim AI Evaluation System*" >> "$REPORT_FILE"

cat "$REPORT_FILE"
echo ""
echo "Full report saved to: $REPORT_FILE"
```

## Complete Evaluation Workflow

Combine all steps into a single master script:

```bash
#!/bin/bash
# evaluate-ai-agent.sh

set -euo pipefail

CODEBASE="$1"
AI_AGENT="${2:-claude-code}"
ROLE="${3:-Code Reviewer}"

# Create working directories
BASELINE_CODE="./evaluation-temp/baseline"
AFTER_CODE="./evaluation-temp/after"

mkdir -p "$BASELINE_CODE" "$AFTER_CODE"

# Copy baseline
cp -r "$CODEBASE" "$BASELINE_CODE"

echo "=== Step 1: Baseline Evaluation ==="
./baseline-evaluation.sh "$BASELINE_CODE" "$ROLE"

echo ""
echo "=== Step 2: Apply AI Agent Changes ==="
./apply-ai-changes.sh "$BASELINE_CODE" "$AFTER_CODE" "$AI_AGENT"

echo ""
echo "=== Step 3: Post-Change Evaluation ==="
./post-evaluation.sh "$AFTER_CODE" "$ROLE"

echo ""
echo "=== Step 4: Generate Verdict ==="
./compare-evaluations.sh

echo ""
echo "Evaluation complete!"
```

## Metrics Reference

### Knowledge Graph Metrics

**Extracted from Terraphim**:
- **Nodes**: Number of concepts in the knowledge graph
- **Edges**: Number of relationships between concepts
- **Graph Density**: `edges / (nodes * (nodes - 1) / 2)`
- **Search Scores**: Relevance scores from Aho-Corasick matching

**Interpretation**:
- Higher scores = Better semantic match to quality/problem patterns
- More nodes after = Richer concept space (can be good or bad depending on context)
- Higher density = More interconnected concepts (generally better)

### Code Quality Metrics

**From External Tools**:
- **Clippy Warnings**: Rust linting issues
- **Test Pass Rate**: Percentage of passing tests
- **Code Coverage**: Percentage of code tested
- **Cyclomatic Complexity**: Measure of code complexity
- **Lines of Code**: Total LOC, comment ratio

**Interpretation**:
- Fewer warnings = Improvement
- Higher test pass rate = Improvement
- Higher coverage = Improvement (if tests are meaningful)
- Lower complexity = Improvement (simpler is better)

## Integration with CI/CD

### GitHub Actions Example

Create `.github/workflows/ai-evaluation.yml`:

```yaml
name: AI Agent Evaluation

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  evaluate:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout baseline (main branch)
        uses: actions/checkout@v3
        with:
          ref: main
          path: baseline

      - name: Checkout PR changes
        uses: actions/checkout@v3
        with:
          path: pr-changes

      - name: Install Terraphim
        run: |
          curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash
          echo "$HOME/.terraphim/bin" >> $GITHUB_PATH

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: clippy

      - name: Run baseline evaluation
        run: |
          ./scripts/baseline-evaluation.sh baseline "Code Reviewer"

      - name: Run post-change evaluation
        run: |
          ./scripts/post-evaluation.sh pr-changes "Code Reviewer"

      - name: Generate verdict
        id: verdict
        run: |
          ./scripts/compare-evaluations.sh
          echo "report_path=./evaluation-results/verdict.md" >> $GITHUB_OUTPUT

      - name: Post verdict as comment
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const verdict = fs.readFileSync('${{ steps.verdict.outputs.report_path }}', 'utf8');

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: verdict
            });

      - name: Fail if deterioration detected
        run: |
          if grep -q "❌ \*\*DETERIORATION\*\*" ./evaluation-results/verdict.md; then
            echo "AI changes deteriorated codebase quality"
            exit 1
          fi
```

## Advanced Features

### 1. Historical Trend Analysis

Track evaluations over time:

```bash
#!/bin/bash
# track-trends.sh

EVAL_DATE=$(date +%Y%m%d)
HISTORY_DIR="./evaluation-history/$EVAL_DATE"

mkdir -p "$HISTORY_DIR"

# Copy results
cp -r ./evaluation-results/* "$HISTORY_DIR/"

# Generate trend report
python3 << 'EOF'
import json
import glob
from pathlib import Path

history_dirs = sorted(glob.glob("./evaluation-history/*"))

print("# Evaluation Trends\n")
print("| Date | Code Smells | Bugs | Warnings |")
print("|------|-------------|------|----------|")

for dir_path in history_dirs:
    date = Path(dir_path).name
    try:
        with open(f"{dir_path}/after/code-smells.json") as f:
            smells = len(json.load(f).get("results", []))
        with open(f"{dir_path}/after/bug-patterns.json") as f:
            bugs = len(json.load(f).get("results", []))

        warnings = 0
        if Path(f"{dir_path}/after/clippy.log").exists():
            with open(f"{dir_path}/after/clippy.log") as f:
                warnings = f.read().count("warning:")

        print(f"| {date} | {smells} | {bugs} | {warnings} |")
    except:
        pass
EOF
```

### 2. Multi-Role Evaluation

Run multiple evaluation perspectives simultaneously:

```bash
#!/bin/bash
# multi-role-evaluation.sh

CODEBASE="$1"
ROLES=("Code Reviewer" "Performance Analyst" "Security Auditor" "Documentation Quality")

for role in "${ROLES[@]}"; do
    echo "=== Evaluating with role: $role ==="
    ./baseline-evaluation.sh "$CODEBASE" "$role"

    # Store results in role-specific directory
    ROLE_DIR="./evaluation-results/$(echo $role | tr ' ' '-' | tr '[:upper:]' '[:lower:]')"
    mkdir -p "$ROLE_DIR"
    mv ./evaluation-results/baseline "$ROLE_DIR/"
done

echo "Multi-role evaluation complete"
```

### 3. Firecracker VM Integration

For isolated, secure evaluation:

```bash
#!/bin/bash
# secure-evaluation.sh

CODEBASE="$1"

# Launch Firecracker VM with terraphim
terraphim-tui /vm launch eval-vm

# Run evaluation in VM
terraphim-tui /vm exec eval-vm "cd $CODEBASE && cargo clippy"
terraphim-tui /vm exec eval-vm "cd $CODEBASE && cargo test"

# Retrieve results
terraphim-tui /vm download eval-vm /tmp/results ./evaluation-results/

# Cleanup
terraphim-tui /vm terminate eval-vm
```

## Best Practices

### 1. Define Clear Evaluation Criteria

Before running evaluations:
- Document what constitutes "improvement" vs "deterioration"
- Set threshold values for metrics (e.g., "no increase in warnings")
- Align criteria with project goals

### 2. Version Control Knowledge Graphs

Track evolution of evaluation criteria:
```bash
git add docs/src/kg/
git commit -m "Update evaluation KG with new security patterns"
```

### 3. Automate Regular Evaluations

Run evaluations on every PR:
- Use CI/CD integration (GitHub Actions, GitLab CI)
- Block merges if quality deteriorates
- Track trends over time

### 4. Combine Quantitative and Qualitative Analysis

Don't rely solely on scores:
- Review actual code changes manually
- Use Terraphim chat for semantic analysis:
  ```bash
  terraphim-tui /chat "Analyze the security implications of this change"
  ```

### 5. Calibrate for Your Domain

Customize knowledge graphs for your specific:
- Programming language(s)
- Framework conventions
- Team coding standards
- Domain-specific concerns

## Troubleshooting

### Low Scores Despite Good Code

**Cause**: Knowledge graph may not cover positive patterns

**Solution**: Add KG entries for good practices:
```markdown
# Best Practice Implementation

Well-implemented code following best practices.

synonyms:: clean code, well-structured, idiomatic, proper error handling, good abstraction
```

### False Positives in Bug Detection

**Cause**: Overly broad synonyms in bug KG files

**Solution**: Make synonyms more specific:
```markdown
# Null Pointer Dereference

synonyms:: null pointer dereference, NPE, null reference exception
# NOT: null, pointer (too broad)
```

### Inconsistent Results Across Runs

**Cause**: Non-deterministic factors (file order, timestamps)

**Solution**: Terraphim's Aho-Corasick is deterministic, but ensure:
- Same role configuration
- Same KG files
- Clean rebuild of indices between runs

## Limitations and Future Work

### Current Limitations

1. **Filename-Based Concepts**: KG uses filenames as terms (underscores for spaces)
2. **Manual Query Definition**: Requires upfront definition of evaluation queries
3. **Text-Based Analysis**: Works best with textual code and comments
4. **Binary/Compiled Code**: Limited support for non-text formats

### Future Enhancements

1. **Machine Learning Integration**: Train models on evaluation outcomes
2. **Natural Language Verdicts**: Generate human-readable explanations
3. **Real-Time Evaluation**: Stream results as AI agent makes changes
4. **Cross-Language Support**: Multi-language KG entries and analysis
5. **Visualization Dashboard**: Web UI for exploring evaluation results

## References

- [Terraphim AI Documentation](https://docs.terraphim.ai)
- [Aho-Corasick Algorithm](https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm)
- [Knowledge Graph Construction](https://github.com/terraphim/terraphim-ai/blob/main/docs/knowledge-graph.md)
- [TUI Commands Reference](https://github.com/terraphim/terraphim-ai/blob/main/crates/terraphim_tui/README.md)

## Contributing

To contribute evaluation patterns:

1. Create new KG files in `docs/src/kg/`
2. Test with your codebase
3. Submit PR with examples and documentation
4. Share evaluation scripts in `examples/codebase-evaluation/`

## License

This evaluation framework follows Terraphim AI's licensing (Apache 2.0).

---

*For questions or support, open an issue at https://github.com/terraphim/terraphim-ai/issues*
