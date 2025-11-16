#!/usr/bin/env bash
# Master Evaluation Script
# Usage: ./evaluate-ai-agent.sh <codebase_path> [ai_agent_name] [role_name]

set -euo pipefail

CODEBASE="${1:?Error: codebase path required}"
AI_AGENT="${2:-claude-code}"
ROLE="${3:-Code Reviewer}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="./evaluation-temp"
BASELINE_CODE="$WORK_DIR/baseline"
AFTER_CODE="$WORK_DIR/after"

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║        Terraphim AI Agent Evaluation System                  ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Configuration:"
echo "  Codebase: $CODEBASE"
echo "  AI Agent: $AI_AGENT"
echo "  Evaluation Role: $ROLE"
echo "  Working Directory: $WORK_DIR"
echo ""

# Clean previous evaluation
if [ -d "$WORK_DIR" ]; then
    echo "Cleaning previous evaluation..."
    rm -rf "$WORK_DIR"
fi

# Create working directories
mkdir -p "$BASELINE_CODE" "$AFTER_CODE"

# Copy baseline
echo "Creating baseline copy..."
cp -r "$CODEBASE/." "$BASELINE_CODE/"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "STEP 1: Baseline Evaluation"
echo "════════════════════════════════════════════════════════════════"
"$SCRIPT_DIR/baseline-evaluation.sh" "$BASELINE_CODE" "$ROLE"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "STEP 2: Apply AI Agent Changes"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "Copy baseline to 'after' directory for modification..."
cp -r "$BASELINE_CODE/." "$AFTER_CODE/"

echo ""
echo "⚠️  MANUAL STEP REQUIRED ⚠️"
echo ""
echo "Apply your AI agent changes to: $AFTER_CODE"
echo ""
echo "Examples:"
echo "  - Run Claude Code on the directory"
echo "  - Apply a pull request"
echo "  - Manually edit files based on AI suggestions"
echo ""
echo "After making changes, press Enter to continue evaluation..."
read -r

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "STEP 3: Post-Change Evaluation"
echo "════════════════════════════════════════════════════════════════"
"$SCRIPT_DIR/post-evaluation.sh" "$AFTER_CODE" "$ROLE"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "STEP 4: Generate Verdict"
echo "════════════════════════════════════════════════════════════════"
"$SCRIPT_DIR/compare-evaluations.sh"

EXIT_CODE=$?

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                   Evaluation Complete                        ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Results:"
echo "  - Baseline: $WORK_DIR/evaluation-results/baseline/"
echo "  - After: $WORK_DIR/evaluation-results/after/"
echo "  - Verdict: $WORK_DIR/evaluation-results/verdict.md"
echo ""

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Overall: IMPROVEMENT or NEUTRAL"
else
    echo "❌ Overall: DETERIORATION detected"
fi

exit $EXIT_CODE
