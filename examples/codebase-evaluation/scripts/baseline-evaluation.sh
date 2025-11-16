#!/usr/bin/env bash
# Baseline Evaluation Script
# Usage: ./baseline-evaluation.sh <codebase_path> [role_name]

set -euo pipefail

CODEBASE_PATH="${1:?Error: codebase path required}"
ROLE="${2:-Code Reviewer}"
OUTPUT_DIR="./evaluation-results/baseline"
TERRAPHIM_TUI="${TERRAPHIM_TUI_BIN:-terraphim-tui}"

# Find terraphim-tui binary
if ! command -v "$TERRAPHIM_TUI" &> /dev/null; then
    if [ -f "$(git rev-parse --show-toplevel 2>/dev/null)/target/release/terraphim-tui" ]; then
        TERRAPHIM_TUI="$(git rev-parse --show-toplevel)/target/release/terraphim-tui"
    else
        echo "Error: terraphim-tui not found. Build with: cargo build --release -p terraphim_tui"
        exit 1
    fi
fi

mkdir -p "$OUTPUT_DIR"

echo "=== Baseline Evaluation ==="
echo "Codebase: $CODEBASE_PATH"
echo "Role: $ROLE"
echo "Output: $OUTPUT_DIR"
echo ""

# Run evaluation queries using terraphim-tui replace functionality
echo "Running evaluation queries..."

# Code quality checks
echo "Checking for code smells..."
"$TERRAPHIM_TUI" replace "code smell technical debt refactoring" 2>/dev/null > "$OUTPUT_DIR/code-smells.txt" || true

echo "Checking for bug patterns..."
"$TERRAPHIM_TUI" replace "null pointer memory leak race condition" 2>/dev/null > "$OUTPUT_DIR/bug-patterns.txt" || true

echo "Checking for duplication..."
"$TERRAPHIM_TUI" replace "duplicate code copy paste DRY violation" 2>/dev/null > "$OUTPUT_DIR/duplication.txt" || true

# Count matches in codebase
if command -v rg &> /dev/null; then
    echo "Scanning codebase for issues..."

    # Count TODO/FIXME
    rg -i "TODO|FIXME" "$CODEBASE_PATH" --count-matches > "$OUTPUT_DIR/todos.txt" 2>/dev/null || echo "0" > "$OUTPUT_DIR/todos.txt"

    # Count common anti-patterns
    rg -i "unwrap\(\)|panic!|todo!|unimplemented!" "$CODEBASE_PATH" --count-matches > "$OUTPUT_DIR/antipatterns.txt" 2>/dev/null || echo "0" > "$OUTPUT_DIR/antipatterns.txt"
fi

# Run Rust-specific checks if applicable
if [ -f "$CODEBASE_PATH/Cargo.toml" ]; then
    echo "Running Rust quality checks..."
    cd "$CODEBASE_PATH"

    # Clippy
    if command -v cargo &> /dev/null; then
        cargo clippy --all-targets -- -D warnings 2>&1 | tee "$OUTPUT_DIR/../../clippy-baseline.log" || true

        # Count warnings
        grep -c "warning:" "$OUTPUT_DIR/../../clippy-baseline.log" > "$OUTPUT_DIR/clippy-warnings.txt" 2>/dev/null || echo "0" > "$OUTPUT_DIR/clippy-warnings.txt"
    fi

    # Tests
    cargo test --no-fail-fast 2>&1 | tee "$OUTPUT_DIR/../../test-baseline.log" || true

    cd - > /dev/null
fi

# Count lines of code
if command -v tokei &> /dev/null; then
    tokei "$CODEBASE_PATH" > "$OUTPUT_DIR/tokei.txt"
fi

echo ""
echo "Baseline evaluation complete!"
echo "Results saved to: $OUTPUT_DIR"
echo ""
echo "Summary:"
[ -f "$OUTPUT_DIR/clippy-warnings.txt" ] && echo "  Clippy warnings: $(cat $OUTPUT_DIR/clippy-warnings.txt)"
[ -f "$OUTPUT_DIR/todos.txt" ] && echo "  TODOs/FIXMEs: $(cat $OUTPUT_DIR/todos.txt)"
[ -f "$OUTPUT_DIR/antipatterns.txt" ] && echo "  Anti-patterns: $(cat $OUTPUT_DIR/antipatterns.txt)"
