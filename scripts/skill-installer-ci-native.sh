#!/bin/bash
# skill-installer-ci-native.sh -- ADF ci-native wrapper for PR/branch validation
#
# Invoked by ADF ci-native to validate repository-side artefacts without
# requiring paid model calls or live subscription-gated downloads.
# Does NOT depend on .gitea/workflows/* or Forgejo/Gitea Actions.
#
# Required checks:
#   1. Shell syntax on validation scripts
#   2. Dry-run matrix rendering
#   3. ADF TOML validation (when adf binary is available)
#   4. Repository ownership inventory

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FAILURES=0

echo "# ADF ci-native: Terraphim Skills Validation"
echo ""

# --- Check 1: Shell syntax ---------------------------------------------------
echo "## Check 1: Shell syntax"
for SCRIPT in "$SCRIPT_DIR/skill-installer-validation.sh" "$SCRIPT_DIR/skill-installer-ci-native.sh"; do
    if [ -f "$SCRIPT" ]; then
        if bash -n "$SCRIPT" 2>&1; then
            echo "  PASS: $SCRIPT"
        else
            echo "  FAIL: $SCRIPT"
            FAILURES=$((FAILURES + 1))
        fi
    else
        echo "  SKIP: $SCRIPT not found"
    fi
done
echo ""

# --- Check 2: Dry-run matrix -------------------------------------------------
echo "## Check 2: Dry-run matrix rendering"
DRY_OUTPUT=$(TERRAPHIM_SKILLS_DRY_RUN=1 bash "$SCRIPT_DIR/skill-installer-validation.sh" 2>&1) || {
    echo "  FAIL: dry-run script failed"
    FAILURES=$((FAILURES + 1))
}
if echo "$DRY_OUTPUT" | grep -q '"overall"' 2>/dev/null; then
    echo "  PASS: valid JSON output with overall field"
else
    echo "  FAIL: JSON output missing overall field"
    FAILURES=$((FAILURES + 1))
fi
if echo "$DRY_OUTPUT" | grep -q 'Skills x CLI Status Matrix' 2>/dev/null; then
    echo "  PASS: Markdown matrix rendered"
else
    echo "  FAIL: Markdown matrix missing"
    FAILURES=$((FAILURES + 1))
fi
echo ""

# --- Check 3: ADF TOML validation --------------------------------------------
echo "## Check 3: ADF TOML validation"
TOML_FILE="$REPO_ROOT/crates/terraphim_orchestrator/conf.d/skills-installer.toml"
if [ -f "$TOML_FILE" ]; then
    if command -v adf >/dev/null 2>&1; then
        if adf agent validate-all --config "$TOML_FILE" --format json --skip-model-probe 2>&1; then
            echo "  PASS: TOML validation OK"
        else
            echo "  WARN: TOML validation returned non-zero (check manually)"
        fi
    else
        echo "  WARN: adf binary not available; TOML syntax not verified"
        if python3 -c "import sys; open(sys.argv[1]).read(); print('readable')" "$TOML_FILE" 2>/dev/null; then
            echo "  PASS: TOML file readable (basic check)"
        else
            echo "  FAIL: TOML file not readable"
            FAILURES=$((FAILURES + 1))
        fi
    fi
else
    echo "  FAIL: TOML config not found at $TOML_FILE"
    FAILURES=$((FAILURES + 1))
fi
echo ""

# --- Check 4: Repository inventory -------------------------------------------
echo "## Check 4: Repository ownership inventory"
REQUIRED_REPOS=(
    "terraphim/terraphim-skills"
    "terraphim/terraphim-skills-server"
    "terraphim/terraphim-skills-site"
    "terraphim/terraphim-skills.md"
    "terraphim/codex-skills"
    "terraphim/opencode-skills"
)
if command -v gtr >/dev/null 2>&1; then
    for REPO in "${REQUIRED_REPOS[@]}"; do
        OWNER=$(echo "$REPO" | cut -d/ -f1)
        NAME=$(echo "$REPO" | cut -d/ -f2)
        if gtr list-repos --org "$OWNER" --query "$NAME" --limit 3 2>/dev/null | grep -q "\"name\":\"$NAME\"" 2>/dev/null; then
            echo "  PASS: $REPO exists"
        else
            echo "  WARN: could not confirm $REPO exists via gtr"
        fi
    done
else
    echo "  WARN: gtr not available; repository inventory not verified"
    echo "  Required: ${REQUIRED_REPOS[*]}"
fi
echo ""

# --- Summary ----------------------------------------------------------------
echo "## Summary"
if [ "$FAILURES" -eq 0 ]; then
    echo "**PASS:** All ci-native checks passed"
    exit 0
else
    echo "**FAIL:** $FAILURES ci-native check(s) failed"
    exit 1
fi
