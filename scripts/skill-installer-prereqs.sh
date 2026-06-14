#!/bin/bash
# skill-installer-prereqs.sh -- Live-user prerequisite check for Terraphim Skills validation
#
# Verifies the live-user environment has all the required CLIs, directories,
# tsm, adf, and either TSM_TOKEN or ~/.terraphim/token. Does NOT print secrets.
#
# Exit codes:
#   0 = all required prerequisites met
#   1 = one or more required prerequisites missing
#   2 = token missing (user should set TSM_TOKEN or login via tsm)

set -euo pipefail

REQUIRED_DIRS=(
    "$HOME/.claude/skills"     # Claude Code
    "$HOME/.codex/skills"      # Codex
    "$HOME/.pi/agent/skills"   # Pi (badlogic)
    "$HOME/.grok/skills"       # Grok
    "$HOME/.agents/skills"     # OpenCode/pi-rust
)

REQUIRED_BINARIES=(
    "tsm"     # Terraphim Skills Manager
    "claude"  # Claude Code
    "codex"   # Codex
    "pi"      # badlogic Pi
    "pi-rust" # pi-rust
    "grok"    # Grok
)

OPTIONAL_BINARIES=(
    "adf"     # ADF orchestrator (only needed to run agents)
    "adf-ctl" # ADF control CLI
)

PASS=0
FAIL=0
WARN=0
TOKENS_FOUND=0

echo "# Live-user prerequisites check for Terraphim Skills validation"
echo ""

# --- 1. Required binaries -----------------------------------------------------
echo "## Required CLI binaries"
for BIN in "${REQUIRED_BINARIES[@]}"; do
    if command -v "$BIN" >/dev/null 2>&1; then
        VERSION=""
        VERSION=$("$BIN" --version 2>/dev/null | head -1 || echo "version unknown")
        echo "  PASS: $BIN ($VERSION)"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $BIN not on PATH"
        FAIL=$((FAIL + 1))
    fi
done
echo ""

# --- 2. Optional binaries -----------------------------------------------------
echo "## Optional CLI binaries (only needed for ADF agent runs)"
for BIN in "${OPTIONAL_BINARIES[@]}"; do
    if command -v "$BIN" >/dev/null 2>&1; then
        echo "  PASS: $BIN"
        PASS=$((PASS + 1))
    else
        echo "  WARN: $BIN not on PATH (optional)"
        WARN=$((WARN + 1))
    fi
done
echo ""

# --- 3. Required directories --------------------------------------------------
echo "## Required skill directories"
for DIR in "${REQUIRED_DIRS[@]}"; do
    if [ -d "$DIR" ]; then
        COUNT=$(find "$DIR" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | xargs)
        echo "  PASS: $DIR ($COUNT entries)"
        PASS=$((PASS + 1))
    else
        echo "  WARN: $DIR does not exist (will be created on first install)"
        WARN=$((WARN + 1))
    fi
done
echo ""

# --- 4. Token presence (NEVER print token value) ------------------------------
echo "## Token check (never prints the token value)"
if [ -n "${TSM_TOKEN:-}" ]; then
    LEN=${#TSM_TOKEN}
    if [ "$LEN" -gt 0 ]; then
        echo "  PASS: TSM_TOKEN is set (length $LEN, value redacted)"
        TOKENS_FOUND=$((TOKENS_FOUND + 1))
    fi
elif [ -s "$HOME/.terraphim/token" ]; then
    SIZE=$(wc -c < "$HOME/.terraphim/token" | xargs)
    echo "  PASS: ~/.terraphim/token exists ($SIZE bytes, content redacted)"
    TOKENS_FOUND=$((TOKENS_FOUND + 1))
else
    echo "  FAIL: TSM_TOKEN not set and ~/.terraphim/token missing"
    echo "         Run: tsm login  (interactive) or set TSM_TOKEN env var"
    FAIL=$((FAIL + 1))
fi
echo ""

# --- 5. tsm registry reachability -------------------------------------------
echo "## Marketplace reachability"
if command -v tsm >/dev/null 2>&1; then
    TSM_API_URL="${TSM_API_URL:-https://api.terraphim-skills.md}"
    if command -v curl >/dev/null 2>&1; then
        HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -m 5 "$TSM_API_URL/health" 2>/dev/null || echo "0")
        if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "404" ] || [ "$HTTP_CODE" = "401" ]; then
            echo "  PASS: $TSM_API_URL reachable (HTTP $HTTP_CODE)"
            PASS=$((PASS + 1))
        else
            echo "  WARN: $TSM_API_URL not reachable (HTTP $HTTP_CODE)"
            WARN=$((WARN + 1))
        fi
    else
        echo "  WARN: curl not available; cannot test reachability"
        WARN=$((WARN + 1))
    fi
else
    echo "  SKIP: tsm not installed; cannot test marketplace"
fi
echo ""

# --- Summary -----------------------------------------------------------------
echo "## Summary"
echo "  Pass: $PASS"
echo "  Fail: $FAIL"
echo "  Warn: $WARN"
echo "  Token sources found: $TOKENS_FOUND"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "**FAIL:** $FAIL required prerequisite(s) missing"
    if [ "$TOKENS_FOUND" -eq 0 ]; then
        exit 2
    fi
    exit 1
fi

echo "**PASS:** all required prerequisites met"
exit 0
