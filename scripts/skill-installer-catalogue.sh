#!/bin/bash
# skill-installer-catalogue.sh -- Generate canonical full Terraphim Skills catalogue
#
# Output: JSON manifest of all skills expected to be available to a live user.
# Sources (in priority order):
#   1. tsm list (live, requires TSM_TOKEN or ~/.terraphim/token)
#   2. ~/.terraphim/registry/skills/ (local clone, no auth)
#   3. Built-in fallback (named catalogue)
#
# Output format:
#   {
#     "source": "tsm|registry|fallback",
#     "generated_at": "ISO8601",
#     "skill_count": N,
#     "skills": [
#       {
#         "name": "...",
#         "tier": "free|premium",
#         "source_repo": "terraphim-skills|codex-skills|opencode-skills|...",
#         "expected_clis": ["claude-code", "codex", ...],
#         "auth_gated": true|false
#       }
#     ]
#   }
#
# Output goes to stdout. If TERRAPHIM_SKILLS_CATALOGUE_OUT is set, also writes there.

set -euo pipefail

OUT="${TERRAPHIM_SKILLS_CATALOGUE_OUT:-}"
TSM_BIN="${TERRAPHIM_SKILLS_MANAGER_BIN:-tsm}"
REGISTRY_DIR="${TERRAPHIM_SKILLS_REGISTRY_DIR:-$HOME/.terraphim/registry/skills}"

GENERATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
SOURCE="fallback"
SKILLS_JSON=""

# --- Try live tsm list -------------------------------------------------------
if command -v "$TSM_BIN" >/dev/null 2>&1; then
    if [ -n "${TSM_TOKEN:-}" ] || [ -s "$HOME/.terraphim/token" ]; then
        if LIVE_OUT=$("$TSM_BIN" list 2>/dev/null); then
            # Try JSON first, fall back to text parsing
            if JSON_OUT=$("$TSM_BIN" list --json 2>/dev/null); then
                SOURCE="tsm"
                SKILLS_JSON="$JSON_OUT"
            elif echo "$LIVE_OUT" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
                SOURCE="tsm"
                SKILLS_JSON=$(echo "$LIVE_OUT" | python3 -c "import sys,json; print(json.dumps([{\"name\": s} for s in json.load(sys.stdin)]))" 2>/dev/null)
            else
                # Text mode: parse names
                SOURCE="tsm"
                SKILLS_JSON=$(echo "$LIVE_OUT" | grep -E "^[-a-z0-9]+$" | python3 -c "import sys,json; names=[l.strip() for l in sys.stdin if l.strip()]; print(json.dumps([{'name':n} for n in names]))" 2>/dev/null)
            fi
        fi
    fi
fi

# --- Fallback to registry clone ---------------------------------------------
if [ -z "$SKILLS_JSON" ] && [ -d "$REGISTRY_DIR" ]; then
    SOURCE="registry"
    SKILLS_JSON=$(ls "$REGISTRY_DIR" 2>/dev/null | grep -v "\.skill$" | sort -u | python3 -c "import sys,json; names=[l.strip() for l in sys.stdin if l.strip()]; print(json.dumps([{'name':n, 'auth_gated': True} for n in names]))" 2>/dev/null)
fi

# --- Final fallback to known sentinel list ----------------------------------
if [ -z "$SKILLS_JSON" ]; then
    SOURCE="fallback"
    SKILLS_JSON='[{"name":"code-review","auth_gated":true},{"name":"testing","auth_gated":true},{"name":"security-audit","auth_gated":true}]'
fi

# --- Compose manifest -------------------------------------------------------
# Count skills
SKILL_COUNT=$(echo "$SKILLS_JSON" | python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null || echo 0)

# Determine expected CLIs per skill (default to all 5)
ALL_CLIS='["claude-code", "codex", "pi", "pi-rust", "grok"]'

# Build final manifest
MANIFEST=$(cat <<EOF
{
  "source": "$SOURCE",
  "generated_at": "$GENERATED_AT",
  "skill_count": $SKILL_COUNT,
  "skills": $SKILLS_JSON,
  "expected_clis": $ALL_CLIS,
  "notes": "Skills marked auth_gated=true require TSM_TOKEN or ~/.terraphim/token for tsm install. Skills without skill.toml metadata cannot be tsm-verified without prior marketplace install."
}
EOF
)

echo "$MANIFEST"

if [ -n "$OUT" ]; then
    echo "$MANIFEST" > "$OUT"
    echo "" >&2
    echo "Catalogue written to $OUT" >&2
fi
