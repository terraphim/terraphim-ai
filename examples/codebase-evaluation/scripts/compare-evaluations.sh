#!/usr/bin/env bash
# Compare Evaluations and Generate Verdict
# Usage: ./compare-evaluations.sh

set -euo pipefail

BASELINE_DIR="./evaluation-results/baseline"
AFTER_DIR="./evaluation-results/after"
REPORT_FILE="./evaluation-results/verdict.md"

echo "=== Comparing Evaluations ==="

if [ ! -d "$BASELINE_DIR" ] || [ ! -d "$AFTER_DIR" ]; then
    echo "Error: Evaluation results not found."
    echo "Run baseline-evaluation.sh and post-evaluation.sh first."
    exit 1
fi

# Initialize report
cat > "$REPORT_FILE" << 'EOF'
# Codebase Evaluation Verdict

**Generated**: $(date)
**Evaluator**: Terraphim AI

---

## Summary

EOF

# Function to safely read count from file
read_count() {
    local file="$1"
    if [ -f "$file" ]; then
        cat "$file" | tr -d '\n' || echo "0"
    else
        echo "0"
    fi
}

# Compare Clippy warnings
if [ -f "$BASELINE_DIR/clippy-warnings.txt" ] && [ -f "$AFTER_DIR/clippy-warnings.txt" ]; then
    BASELINE_WARNINGS=$(read_count "$BASELINE_DIR/clippy-warnings.txt")
    AFTER_WARNINGS=$(read_count "$AFTER_DIR/clippy-warnings.txt")
    WARNINGS_DELTA=$((AFTER_WARNINGS - BASELINE_WARNINGS))

    echo "### Clippy Warnings" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "| Metric | Baseline | After | Delta |" >> "$REPORT_FILE"
    echo "|--------|----------|-------|-------|" >> "$REPORT_FILE"
    echo "| Warnings | $BASELINE_WARNINGS | $AFTER_WARNINGS | $WARNINGS_DELTA |" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"

    if [ "$WARNINGS_DELTA" -lt 0 ]; then
        echo "✅ **Improvement**: Reduced warnings by ${WARNINGS_DELTA#-}" >> "$REPORT_FILE"
    elif [ "$WARNINGS_DELTA" -gt 0 ]; then
        echo "❌ **Deterioration**: Increased warnings by $WARNINGS_DELTA" >> "$REPORT_FILE"
    else
        echo "➖ **Neutral**: No change in warnings" >> "$REPORT_FILE"
    fi
    echo "" >> "$REPORT_FILE"
fi

# Compare TODOs/FIXMEs
if [ -f "$BASELINE_DIR/todos.txt" ] && [ -f "$AFTER_DIR/todos.txt" ]; then
    BASELINE_TODOS=$(read_count "$BASELINE_DIR/todos.txt")
    AFTER_TODOS=$(read_count "$AFTER_DIR/todos.txt")
    TODOS_DELTA=$((AFTER_TODOS - BASELINE_TODOS))

    echo "### TODOs and FIXMEs" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "| Metric | Baseline | After | Delta |" >> "$REPORT_FILE"
    echo "|--------|----------|-------|-------|" >> "$REPORT_FILE"
    echo "| Count | $BASELINE_TODOS | $AFTER_TODOS | $TODOS_DELTA |" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"

    if [ "$TODOS_DELTA" -lt 0 ]; then
        echo "✅ **Improvement**: Resolved ${TODOS_DELTA#-} TODOs/FIXMEs" >> "$REPORT_FILE"
    elif [ "$TODOS_DELTA" -gt 0 ]; then
        echo "⚠️ **Note**: Added $TODOS_DELTA new TODOs/FIXMEs" >> "$REPORT_FILE"
    else
        echo "➖ **Neutral**: No change in TODOs" >> "$REPORT_FILE"
    fi
    echo "" >> "$REPORT_FILE"
fi

# Compare anti-patterns
if [ -f "$BASELINE_DIR/antipatterns.txt" ] && [ -f "$AFTER_DIR/antipatterns.txt" ]; then
    BASELINE_AP=$(read_count "$BASELINE_DIR/antipatterns.txt")
    AFTER_AP=$(read_count "$AFTER_DIR/antipatterns.txt")
    AP_DELTA=$((AFTER_AP - BASELINE_AP))

    echo "### Anti-Patterns" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "| Metric | Baseline | After | Delta |" >> "$REPORT_FILE"
    echo "|--------|----------|-------|-------|" >> "$REPORT_FILE"
    echo "| Count | $BASELINE_AP | $AFTER_AP | $AP_DELTA |" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"

    if [ "$AP_DELTA" -lt 0 ]; then
        echo "✅ **Improvement**: Removed ${AP_DELTA#-} anti-patterns" >> "$REPORT_FILE"
    elif [ "$AP_DELTA" -gt 0 ]; then
        echo "❌ **Deterioration**: Introduced $AP_DELTA new anti-patterns" >> "$REPORT_FILE"
    else
        echo "➖ **Neutral**: No change in anti-patterns" >> "$REPORT_FILE"
    fi
    echo "" >> "$REPORT_FILE"
fi

# Calculate overall verdict
IMPROVEMENT_COUNT=0
DETERIORATION_COUNT=0
NEUTRAL_COUNT=0

# Lower is better for problems
if [ -n "${WARNINGS_DELTA+x}" ]; then
    if [ "$WARNINGS_DELTA" -lt 0 ]; then
        ((IMPROVEMENT_COUNT++))
    elif [ "$WARNINGS_DELTA" -gt 0 ]; then
        ((DETERIORATION_COUNT++))
    else
        ((NEUTRAL_COUNT++))
    fi
fi

if [ -n "${TODOS_DELTA+x}" ]; then
    if [ "$TODOS_DELTA" -lt 0 ]; then
        ((IMPROVEMENT_COUNT++))
    elif [ "$TODOS_DELTA" -gt 0 ]; then
        # TODOs can be neutral (documenting work)
        ((NEUTRAL_COUNT++))
    else
        ((NEUTRAL_COUNT++))
    fi
fi

if [ -n "${AP_DELTA+x}" ]; then
    if [ "$AP_DELTA" -lt 0 ]; then
        ((IMPROVEMENT_COUNT++))
    elif [ "$AP_DELTA" -gt 0 ]; then
        ((DETERIORATION_COUNT++))
    else
        ((NEUTRAL_COUNT++))
    fi
fi

echo "---" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "## Overall Verdict" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

TOTAL_METRICS=$((IMPROVEMENT_COUNT + DETERIORATION_COUNT + NEUTRAL_COUNT))

if [ "$TOTAL_METRICS" -eq 0 ]; then
    echo "⚠️ **INSUFFICIENT DATA**: No comparable metrics found." >> "$REPORT_FILE"
elif [ "$IMPROVEMENT_COUNT" -gt "$DETERIORATION_COUNT" ]; then
    echo "✅ **IMPROVEMENT**: The AI agent improved the codebase quality." >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "- ✅ Improved metrics: **$IMPROVEMENT_COUNT**" >> "$REPORT_FILE"
    echo "- ❌ Deteriorated metrics: **$DETERIORATION_COUNT**" >> "$REPORT_FILE"
    echo "- ➖ Neutral metrics: **$NEUTRAL_COUNT**" >> "$REPORT_FILE"
elif [ "$DETERIORATION_COUNT" -gt "$IMPROVEMENT_COUNT" ]; then
    echo "❌ **DETERIORATION**: The AI agent worsened the codebase quality." >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "- ✅ Improved metrics: **$IMPROVEMENT_COUNT**" >> "$REPORT_FILE"
    echo "- ❌ Deteriorated metrics: **$DETERIORATION_COUNT**" >> "$REPORT_FILE"
    echo "- ➖ Neutral metrics: **$NEUTRAL_COUNT**" >> "$REPORT_FILE"
else
    echo "➖ **NEUTRAL**: The AI agent had mixed or minimal impact." >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "- ✅ Improved metrics: **$IMPROVEMENT_COUNT**" >> "$REPORT_FILE"
    echo "- ❌ Deteriorated metrics: **$DETERIORATION_COUNT**" >> "$REPORT_FILE"
    echo "- ➖ Neutral metrics: **$NEUTRAL_COUNT**" >> "$REPORT_FILE"
fi

echo "" >> "$REPORT_FILE"
echo "## Recommendations" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if [ -n "${WARNINGS_DELTA+x}" ] && [ "$WARNINGS_DELTA" -gt 0 ]; then
    echo "- 🔧 Fix new clippy warnings introduced by AI changes" >> "$REPORT_FILE"
fi

if [ -n "${AP_DELTA+x}" ] && [ "$AP_DELTA" -gt 0 ]; then
    echo "- 🔧 Refactor new anti-patterns (unwrap, panic, etc.)" >> "$REPORT_FILE"
fi

if [ -n "${TODOS_DELTA+x}" ] && [ "$TODOS_DELTA" -gt 5 ]; then
    echo "- 📝 Review newly added TODOs for completion" >> "$REPORT_FILE"
fi

echo "" >> "$REPORT_FILE"
echo "---" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "*Generated by Terraphim AI Evaluation System*" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "For more details, see:" >> "$REPORT_FILE"
echo "- Baseline: \`$BASELINE_DIR/\`" >> "$REPORT_FILE"
echo "- After: \`$AFTER_DIR/\`" >> "$REPORT_FILE"

cat "$REPORT_FILE"
echo ""
echo "==============================================="
echo "Full report saved to: $REPORT_FILE"
echo "==============================================="

# Exit with error code if deterioration detected
if [ "$DETERIORATION_COUNT" -gt "$IMPROVEMENT_COUNT" ]; then
    exit 1
fi
