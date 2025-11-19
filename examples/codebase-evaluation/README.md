# Codebase Evaluation Examples

This directory contains practical examples and scripts for evaluating AI agent improvements to codebases using Terraphim AI.

## Quick Start

### 1. Evaluate Your Own Codebase

```bash
# Run complete evaluation workflow
./scripts/evaluate-ai-agent.sh /path/to/your/codebase

# The script will:
# 1. Create baseline evaluation
# 2. Prompt you to apply AI changes
# 3. Re-evaluate after changes
# 4. Generate verdict report
```

### 2. View Example Evaluation

```bash
# See what a typical evaluation looks like
cat example-outputs/verdict-example.md
```

## Directory Structure

```
examples/codebase-evaluation/
├── README.md                          # This file
├── CODEBASE_EVALUATION_DESIGN.md     # Complete design document
├── scripts/                           # Evaluation scripts
│   ├── evaluate-ai-agent.sh          # Master evaluation script
│   ├── baseline-evaluation.sh        # Baseline metrics
│   ├── post-evaluation.sh            # Post-change metrics
│   └── compare-evaluations.sh        # Comparison and verdict
├── kg-templates/                      # Knowledge graph templates
│   ├── code-quality.md               # Code quality terms
│   ├── bug-patterns.md               # Bug detection terms
│   ├── performance.md                # Performance terms
│   └── security.md                   # Security terms
└── example-outputs/                   # Example evaluation results
    ├── verdict-example.md            # Sample verdict report
    └── baseline/                     # Sample baseline metrics
```

## Scripts Overview

### Master Script

**`evaluate-ai-agent.sh`** - Complete evaluation workflow

```bash
./scripts/evaluate-ai-agent.sh <codebase_path> [ai_agent_name] [role_name]

# Examples:
./scripts/evaluate-ai-agent.sh ./my-project
./scripts/evaluate-ai-agent.sh ./my-project claude-code "Security Auditor"
```

### Individual Scripts

**`baseline-evaluation.sh`** - Run baseline evaluation

```bash
./scripts/baseline-evaluation.sh <codebase_path> [role_name]
```

**`post-evaluation.sh`** - Run post-change evaluation

```bash
./scripts/post-evaluation.sh <codebase_path> [role_name]
```

**`compare-evaluations.sh`** - Generate verdict

```bash
./scripts/compare-evaluations.sh
```

## Metrics Collected

### Terraphim AI Knowledge Graph Metrics

- Semantic matches for code quality issues
- Pattern detection using Aho-Corasick automata
- Concept relationship analysis

### Rust-Specific Metrics (if applicable)

- **Clippy Warnings**: Linting issues count
- **Test Results**: Pass/fail counts
- **Anti-Patterns**: `unwrap()`, `panic!`, `todo!`, `unimplemented!()`
- **TODOs/FIXMEs**: Unfinished work indicators

### General Metrics

- **Lines of Code**: Total LOC via `tokei`
- **Code Complexity**: Cyclomatic complexity (if integrated)
- **Coverage**: Test coverage percentage (if integrated)

## Verdict Logic

The evaluation generates one of three verdicts:

1. **✅ IMPROVEMENT**: More metrics improved than deteriorated
2. **❌ DETERIORATION**: More metrics deteriorated than improved
3. **➖ NEUTRAL**: Equal improvements and deteriorations, or minimal changes

## Example Use Cases

### Use Case 1: Evaluate Claude Code Changes

```bash
# Create baseline
./scripts/baseline-evaluation.sh ./my-rust-project "Code Reviewer"

# Use Claude Code to refactor your code
# (manual step)

# Evaluate changes
./scripts/post-evaluation.sh ./my-rust-project "Code Reviewer"

# Get verdict
./scripts/compare-evaluations.sh
```

### Use Case 2: Evaluate Pull Request from AI Agent

```bash
# Checkout main branch
git checkout main
./scripts/baseline-evaluation.sh . "Security Auditor"

# Checkout PR branch
git checkout ai-agent-pr-123
./scripts/post-evaluation.sh . "Security Auditor"

# Compare
./scripts/compare-evaluations.sh
```

### Use Case 3: Continuous Evaluation in CI/CD

```bash
# In your CI pipeline (e.g., GitHub Actions)
- name: Baseline evaluation
  run: ./scripts/baseline-evaluation.sh ${{ github.workspace }} "Code Reviewer"

- name: Apply AI changes
  run: # Your AI agent step

- name: Post-change evaluation
  run: ./scripts/post-evaluation.sh ${{ github.workspace }} "Code Reviewer"

- name: Generate verdict
  run: ./scripts/compare-evaluations.sh

- name: Fail if deterioration
  run: exit 1  # compare-evaluations.sh already exits with 1 on deterioration
```

## Knowledge Graph Templates

Knowledge graph templates define evaluation perspectives. Located in `kg-templates/`:

### Code Quality (`code-quality.md`)

```markdown
# Code Quality

synonyms:: code smell, technical debt, refactoring opportunity
```

### Bug Patterns (`bug-patterns.md`)

```markdown
# Bug Patterns

synonyms:: null pointer, memory leak, race condition, unhandled exception
```

### Performance (`performance.md`)

```markdown
# Performance Bottleneck

synonyms:: slow code, inefficient algorithm, O(n^2) complexity
```

### Security (`security.md`)

```markdown
# Security Vulnerability

synonyms:: SQL injection, XSS, CSRF, authentication flaw
```

To use custom KG templates:

1. Copy templates to `docs/src/kg/` in your Terraphim installation
2. Rebuild Terraphim indices
3. Run evaluation with appropriate role

## Customization

### Add Custom Evaluation Metrics

Edit scripts to add your own metrics:

```bash
# In baseline-evaluation.sh or post-evaluation.sh

# Example: Check for specific patterns
rg -i "your_pattern" "$CODEBASE_PATH" --count-matches > "$OUTPUT_DIR/custom-metric.txt"
```

### Define Custom Roles

Create role-specific configurations in Terraphim:

```json
{
  "name": "My Custom Role",
  "relevance_function": "terraphim-graph",
  "kg": {
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "docs/src/kg/my-custom-kg"
    }
  }
}
```

### Extend Verdict Logic

Modify `compare-evaluations.sh` to include custom decision criteria:

```bash
# Add your custom metric comparison
if [ -f "$BASELINE_DIR/custom-metric.txt" ] && [ -f "$AFTER_DIR/custom-metric.txt" ]; then
    # Your comparison logic
fi
```

## Troubleshooting

### Script Not Found Errors

Ensure scripts are executable:

```bash
chmod +x scripts/*.sh
```

### Terraphim Binary Not Found

Set `TERRAPHIM_TUI_BIN` environment variable:

```bash
export TERRAPHIM_TUI_BIN=/path/to/terraphim-tui
./scripts/evaluate-ai-agent.sh ./my-project
```

Or build from source:

```bash
cargo build --release -p terraphim_tui --features repl-full
export TERRAPHIM_TUI_BIN=./target/release/terraphim-tui
```

### No Baseline Results

Ensure you have:
1. Built Terraphim TUI
2. Created knowledge graph files in `docs/src/kg/`
3. Valid codebase path

### Exit Code Issues

Compare script exits with code 1 if deterioration detected. This is intentional for CI/CD integration.

## Integration Examples

### GitHub Actions

See `CODEBASE_EVALUATION_DESIGN.md` for complete GitHub Actions workflow example.

### GitLab CI

```yaml
evaluation:
  stage: test
  script:
    - ./scripts/baseline-evaluation.sh . "Code Reviewer"
    # Apply AI changes
    - ./scripts/post-evaluation.sh . "Code Reviewer"
    - ./scripts/compare-evaluations.sh
  artifacts:
    paths:
      - evaluation-results/
    reports:
      junit: evaluation-results/verdict.md
```

## Resources

- [Complete Design Document](./CODEBASE_EVALUATION_DESIGN.md)
- [Terraphim AI Documentation](https://docs.terraphim.ai)
- [Integration Guide](../TERRAPHIM_CLAUDE_INTEGRATION.md)
- [Claude Code Hooks Guide](../claude-code-hooks/README.md)

## Contributing

To contribute evaluation patterns or improvements:

1. Test your changes with real codebases
2. Document new metrics in this README
3. Add example outputs to `example-outputs/`
4. Submit PR with clear description

## License

Follows Terraphim AI licensing (Apache 2.0).

---

*For questions, open an issue at https://github.com/terraphim/terraphim-ai/issues*
